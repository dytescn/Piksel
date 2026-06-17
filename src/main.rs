#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod core;
mod platform;

use core::app::App;
use core::pid::PidManager;
use winit::event_loop::EventLoop;

fn main() {
    // ==============================
    // 🔥 全部改用 PidManager 管理
    // ==============================
    let mut pid_mgr = PidManager::new();

    // 防多开
    if pid_mgr.is_running() {
        eprintln!("Error: Program is already running!");
        return;
    }

    // 创建 PID + run.flag
    if let Err(e) = pid_mgr.create_pid() {
        eprintln!("Failed to create pid: {e}");
        return;
    }

    println!("Program started, pid: {}", std::process::id());
    // 注册全局退出监听
    core::pid::setup_exit_handler();

    #[cfg(target_os = "linux")]
    {
        println!("Initializing ...");
        gtk::init().expect("Failed to init GTK");
    }

    let event_loop = EventLoop::new().expect("Failed to create EventLoop");
    let mut app = App::new();

    // ==============================
    // 🔥 启动 cyctron 并托管
    // ==============================
    if let Err(e) = pid_mgr.start_cyctron() {
        eprintln!("Failed to start cyctron: {}", e);
    }

    // 运行主循环
    let _ = event_loop.run_app(&mut app);

    // ==============================
    // 🔥 主进程退出：自动清理
    // ==============================
    pid_mgr.stop_cyctron();
    pid_mgr.clean_pid();

    println!("Cleanup completed");
}
