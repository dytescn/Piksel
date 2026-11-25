#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod core;
mod platform;

use core::app::{App, UserEvent};
use core::pid::{self, PidManager};
use winit::event_loop::EventLoop;

fn main() {
    // 平台级初始化：Linux 下注册 X11 错误钩子，必须在任何 X11 / GLX 初始化之前。
    platform::init();

    if let Err(e) = run() {
        eprintln!("Piksel 启动失败: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut pid_mgr = PidManager::new();

    // 原子地获取单实例锁；失败说明已有实例在运行。
    if !pid_mgr.try_acquire()? {
        eprintln!("Error: Program is already running!");
        return Ok(());
    }

    // panic 会跳过下面的清理逻辑，用钩子兜底终止 cyctron、删除 PID 文件。
    pid::install_panic_hook();
    println!("Program started, pid: {}", std::process::id());

    // 事件循环内的 panic（例如 winit X11 输入法 focus 失败）会由 panic hook
    // 先清理一次；这里再捕获一层，保证 main 的清理路径一定执行，并返回明确的
    // 非零退出码，而不是让 panic 直接终止进程。
    let result: Result<(), Box<dyn std::error::Error>> =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_event_loop(&mut pid_mgr)
        }))
        .unwrap_or_else(|_| Err("Piksel 事件循环异常退出".into()));

    // 无论是正常退出还是初始化失败，都回收 cyctron 并清理 PID 文件。
    pid_mgr.stop_cyctron();
    pid_mgr.clean_pid();
    result
}

fn run_event_loop(pid_mgr: &mut PidManager) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        println!("Initializing ...");
        gtk::init().map_err(|e| format!("Failed to init GTK: {e}"))?;
    }

    // 全局 dytes 缺失时提示安装；必须在 GTK 初始化之后调用（Linux 提示框依赖 GTK）。
    core::dytes::ensure_installed();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    // SIGINT / SIGTERM / Windows 控制台事件通过 proxy 投递到事件循环，
    // 让 Ctrl+C、kill 都能走正常的退出清理路径。
    pid::setup_exit_handler(event_loop.create_proxy());

    let mut app = App::new();

    // 启动 cyctron 并托管；失败只记录日志，窗口仍会打开以便显示错误。
    if let Err(e) = pid_mgr.start_cyctron() {
        eprintln!("Failed to start cyctron: {}", e);
    }

    event_loop.run_app(&mut app)?;

    println!("Cleanup completed");
    Ok(())
}
