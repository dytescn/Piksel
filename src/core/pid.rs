use std::fs;
use std::io;
use std::process::Child;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use windows::core::BOOL;

const PID_FILE: &str = "./run.pid";

#[derive(Default)]
pub struct PidManager {
    child: Option<Child>,
}

impl PidManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_running(&self) -> bool {
        if !std::path::Path::new(PID_FILE).exists() { return false; }
        if let Ok(content) = fs::read_to_string(PID_FILE) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                return is_process_alive(pid);
            }
        }
        false
    }

    pub fn create_pid(&self) -> io::Result<()> {
        fs::write(PID_FILE, std::process::id().to_string())?;
        Ok(())
    }

    pub fn clean_pid(&self) {
        let _ = fs::remove_file(PID_FILE);
    }

    pub fn start_cyctron(&mut self) -> std::io::Result<()> {
        // Piksel 启动时先替换 update.new.exe（update.exe 此时未运行）
        self.replace_update();
        // 先运行更新
        self.run_update()?;
        
        let exe = "./cyctron";
        println!("Trying to start: {}", exe);
        println!("Exists: {}", std::path::Path::new(exe).exists());

        let mut cmd = std::process::Command::new(exe);
        cmd.arg("start");
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let child = cmd.spawn()?;
        self.child = Some(child);
        Ok(())
    }

    /// 用 update.new.exe 替换 update.exe（无 bootstrap，由 Piksel 主程序完成）
    fn replace_update(&self) {
        let new_path = std::path::Path::new("update.new.exe");
        if !new_path.exists() {
            return;
        }
        let old_path = std::path::Path::new("update.exe");
        if old_path.exists() {
            let _ = fs::remove_file(old_path);
        }
        match fs::rename(new_path, old_path) {
            Ok(_) => println!("✅ update.exe replaced"),
            Err(e) => eprintln!("Failed to replace update.exe: {e}"),
        }
    }

    pub fn stop_cyctron(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("✅ cyctron stopped");
        }
    }

    pub fn run_update(&mut self) -> std::io::Result<()> {
        #[cfg(windows)] {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let exe = "update.exe";
            println!("Running: {}", exe);
            let mut cmd = std::process::Command::new(exe);
            cmd.args(["--auto", "--startup"]);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let mut child = cmd.spawn()?;
            let _ = child.wait();
        }
        #[cfg(any(unix, target_os = "macos"))] {
            let exe = "update";
            println!("Running: {}", exe);
            let mut child = std::process::Command::new(exe)
                .args(["--auto", "--startup"])
                .spawn()?;
            let _ = child.wait();
        }
        Ok(())
    }
}

static EXIT_FLAG: AtomicBool = AtomicBool::new(false);

pub fn setup_exit_handler() {
    unsafe {
        #[cfg(unix)]
        {
            libc::signal(libc::SIGINT, exit_handler as *const () as libc::sighandler_t);
            libc::signal(libc::SIGTERM, exit_handler as *const () as libc::sighandler_t);
        }
        #[cfg(windows)]
        {
            use windows::Win32::System::Console::SetConsoleCtrlHandler;
            let _ = SetConsoleCtrlHandler(Some(win_exit_handler), true);
        }
    }

    extern "C" fn exit_handler(_: i32) {
        EXIT_FLAG.store(true, Ordering::Release);
    }

    #[cfg(windows)]
    unsafe extern "system" fn win_exit_handler(_: u32) -> BOOL {
        EXIT_FLAG.store(true, Ordering::Release);
        BOOL(1)
    }
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe { libc::kill(pid as i32, 0) == 0 }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => { let _ = CloseHandle(handle); true }
            Err(_) => false,
        }
    }
}