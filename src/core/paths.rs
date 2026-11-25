//! 运行时路径解析。
//!
//! Piksel 会从桌面快捷方式、开机自启或任意工作目录启动，因此所有与安装目录
//! 相关的路径都不能依赖当前工作目录（cwd）。这里统一以可执行文件所在目录为
//! 基准解析，找不到时再回退到当前目录（便于开发时从 target/debug 运行）。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static APP_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 可执行文件所在目录；无法获取时回退到当前目录。
pub fn app_dir() -> &'static Path {
    APP_DIR.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    })
}

/// 单实例锁 / PID 文件，固定放在可执行文件目录，与启动时的 cwd 无关。
pub fn pid_file() -> PathBuf {
    app_dir().join("run.pid")
}

/// 按候选名查找同级可执行文件，返回绝对路径。
///
/// 先查可执行文件目录，再查当前目录，兼容开发时从 `target/debug` 直接运行。
pub fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    let roots = [Some(app_dir().to_path_buf()), std::env::current_dir().ok()];

    for root in roots.into_iter().flatten() {
        for name in candidates {
            let path = root.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}
