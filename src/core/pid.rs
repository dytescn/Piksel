use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use winit::event_loop::EventLoopProxy;

use crate::core::app::UserEvent;
use crate::core::dytes;
use crate::core::paths;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
use windows::core::BOOL;

/// `cyctron` 的候选名，按当前平台的优先级排列。
///
/// 安装目录里可能同时存在多个平台的产物（例如 exFAT 移动硬盘上 `cyctron.bin`
/// 与 `cyctron.exe` 并存）。如果候选顺序不区分平台，Linux 上会优先选中
/// `cyctron.exe`，启动时得到 `Exec format error (os error 8)`，窗口无法加载页面。
#[cfg(windows)]
pub(crate) const CYCTRON_CANDIDATES: &[&str] = &["cyctron.exe", "cyctron"];
/// macOS 的发布产物是 bundle 目录，`Command` 无法直接执行，优先无后缀入口。
#[cfg(target_os = "macos")]
pub(crate) const CYCTRON_CANDIDATES: &[&str] = &["cyctron", "cyctron.app"];
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) const CYCTRON_CANDIDATES: &[&str] = &["cyctron", "cyctron.bin"];

/// 持有单实例锁的句柄：进程存活期间一直持有，进程退出时由操作系统释放。
#[derive(Default)]
pub struct PidManager {
    child: Option<Child>,
    lock: Option<File>,
}

impl PidManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 原子地获取单实例锁并写入当前 PID。
    ///
    /// 返回 `Ok(false)` 表示已有实例在运行。锁文件位于可执行文件目录，
    /// 不随启动时的 cwd 变化。
    pub fn try_acquire(&mut self) -> io::Result<bool> {
        let path = paths::pid_file();

        match try_lock(&path)? {
            LockOutcome::Busy => Ok(false),
            LockOutcome::Acquired(file) => {
                // 兼容未使用文件锁的旧版本：锁空闲但 PID 仍存活时仍视为已运行。
                if stale_pid_is_alive(&path) {
                    eprintln!(
                        "检测到仍在运行的 Piksel（PID {}），如需强制启动请删除 {}",
                        read_pid(&path).unwrap_or_default(),
                        path.display()
                    );
                    return Ok(false);
                }
                write_pid_to(&file)?;
                self.lock = Some(file);
                Ok(true)
            }
            LockOutcome::Unsupported => {
                // exFAT / 网络文件系统可能不支持文件锁：回退到 PID 存活检测。
                if stale_pid_is_alive(&path) {
                    return Ok(false);
                }
                write_pid_at(&path)?;
                Ok(true)
            }
        }
    }

    pub fn clean_pid(&mut self) {
        // Windows 上必须先释放锁句柄才能删除文件。
        self.lock = None;
        let _ = std::fs::remove_file(paths::pid_file());
    }

    pub fn start_cyctron(&mut self) -> io::Result<()> {
        // 发布产物按平台命名（cyctron.bin / cyctron.app），安装器会额外生成
        // 无后缀的 `cyctron` 命令入口；候选名按平台排序，避免跨平台误选。
        let exe = paths::find_executable(CYCTRON_CANDIDATES).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("未找到 cyctron（{}）", CYCTRON_CANDIDATES.join(" / ")),
            )
        })?;
        println!("Trying to start: {}", exe.display());

        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("start");
        // cyctron 通过 `./libary` 加载库，必须在它自己的目录下运行。
        if let Some(dir) = exe.parent() {
            cmd.current_dir(dir);
        }
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let child = cmd.spawn()?;
        CYCTRON_PID.store(child.id(), Ordering::Release);
        self.child = Some(child);

        // 更新不阻塞窗口创建：放到后台线程，失败只记录日志。
        dytes::spawn_background_update();
        Ok(())
    }

    pub fn stop_cyctron(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("✅ cyctron stopped");
        }
        CYCTRON_PID.store(0, Ordering::Release);
    }
}

enum LockOutcome {
    Acquired(File),
    Busy,
    /// 仅 Unix 文件系统可能返回；Windows 的独占共享模式总是可用。
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported,
}

#[cfg(unix)]
fn try_lock(path: &Path) -> io::Result<LockOutcome> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(LockOutcome::Acquired(file));
    }

    let err = io::Error::last_os_error();
    match err.raw_os_error() {
        Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN => Ok(LockOutcome::Busy),
        Some(code) if code == libc::ENOSYS || code == libc::EOPNOTSUPP || code == libc::ENOTSUP => {
            Ok(LockOutcome::Unsupported)
        }
        _ => Err(err),
    }
}

#[cfg(windows)]
fn try_lock(path: &Path) -> io::Result<LockOutcome> {
    const ERROR_SHARING_VIOLATION: i32 = 32;

    // 以 share_mode(0) 打开：只要另一个进程持有该文件，就会得到
    // ERROR_SHARING_VIOLATION，从而实现原子单实例。
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(path)
    {
        Ok(file) => Ok(LockOutcome::Acquired(file)),
        Err(err) if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(LockOutcome::Busy),
        Err(err) => Err(err),
    }
}

/// 向已持有锁的文件写入当前 PID。
fn write_pid_to(file: &File) -> io::Result<()> {
    file.set_len(0)?;
    let mut file = file;
    write!(file, "{}", std::process::id())?;
    file.sync_all()
}

/// 无文件锁（Unsupported）时直接写 PID 文件。
fn write_pid_at(path: &Path) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    write!(file, "{}", std::process::id())?;
    file.sync_all()
}

fn read_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn stale_pid_is_alive(path: &Path) -> bool {
    match read_pid(path) {
        Some(pid) if pid != std::process::id() => is_process_alive(pid),
        _ => false,
    }
}

static EXIT_FLAG: AtomicBool = AtomicBool::new(false);
static EXIT_PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub fn setup_exit_handler(proxy: EventLoopProxy<UserEvent>) {
    let _ = EXIT_PROXY.set(proxy);

    unsafe {
        #[cfg(unix)]
        {
            libc::signal(
                libc::SIGINT,
                exit_handler as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGTERM,
                exit_handler as *const () as libc::sighandler_t,
            );
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

    // 信号处理器只设置原子标志；由独立线程把退出请求投递到事件循环，
    // 这样在事件循环空闲等待时也能立即退出。
    std::thread::spawn(|| loop {
        if EXIT_FLAG.swap(false, Ordering::AcqRel) {
            match EXIT_PROXY.get() {
                Some(proxy) => {
                    if proxy.send_event(UserEvent::Exit).is_err() {
                        break;
                    }
                }
                None => break,
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    });
}

/// 当前 cyctron 子进程 PID（0 表示未运行），供 panic 兜底清理使用。
static CYCTRON_PID: AtomicU32 = AtomicU32::new(0);

/// panic 会跳过 `main` 中事件循环之后的清理逻辑：安装钩子确保 cyctron
/// 被终止、PID 文件被删除，避免留下孤儿进程和端口占用。
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let pid = CYCTRON_PID.swap(0, Ordering::AcqRel);
        if pid != 0 {
            terminate_process(pid);
        }
        let _ = std::fs::remove_file(paths::pid_file());
        default_hook(info);
    }));
}

fn terminate_process(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
            let _ = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let _ = CloseHandle(handle);
                true
            }
            Err(_) => false,
        }
    }
}
