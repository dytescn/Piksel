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
        // 先运行更新；dytes 是安装 Piksel 的必要组件
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

    pub fn stop_cyctron(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("✅ cyctron stopped");
        }
    }

    pub fn run_update(&mut self) -> std::io::Result<()> {
        let exe = if cfg!(windows) { "dytes.exe" } else { "dytes" };
        if !std::path::Path::new(exe).exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("未找到 {exe}，请先安装 dytes"),
            ));
        }

        println!("Running: {}", exe);
        let mut cmd = std::process::Command::new(exe);
        // dytes 统一入口：`dytes update --auto --startup`
        cmd.args(["update", "--auto", "--startup"]);
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn()?;
        let _ = child.wait();
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

    #[cfg(unix)]
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