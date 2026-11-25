#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod core;
mod platform;

use core::app::{App, UserEvent};
use core::pid::{self, PidManager};
use winit::event_loop::EventLoop;

fn main() {
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

    let result = run_event_loop(&mut pid_mgr);

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
