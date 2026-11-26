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

    roots
        .into_iter()
        .flatten()
        .find_map(|root| find_in_dir(&root, candidates))
}

/// 在单个目录中按 `candidates` 的顺序取第一个存在的文件。
fn find_in_dir(dir: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "piksel-paths-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn picks_first_existing_candidate() {
        let dir = temp_dir("first");
        std::fs::write(dir.join("b.bin"), b"").unwrap();
        std::fs::write(dir.join("c.exe"), b"").unwrap();

        assert_eq!(
            find_in_dir(&dir, &["a", "b.bin", "c.exe"]),
            Some(dir.join("b.bin"))
        );
        assert_eq!(find_in_dir(&dir, &["a"]), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 安装目录里同时存在 .bin 与 .exe 时，非 Windows 平台必须选中本平台产物。
    #[cfg(not(windows))]
    #[test]
    fn platform_candidates_skip_other_platforms() {
        use crate::core::pid::CYCTRON_CANDIDATES;

        assert!(!CYCTRON_CANDIDATES.contains(&"cyctron.exe"));

        let dir = temp_dir("platform");
        std::fs::write(dir.join("cyctron.bin"), b"").unwrap();
        std::fs::write(dir.join("cyctron.app"), b"").unwrap();
        std::fs::write(dir.join("cyctron.exe"), b"").unwrap();

        let picked = find_in_dir(&dir, CYCTRON_CANDIDATES).unwrap();
        assert_ne!(picked.extension().and_then(|e| e.to_str()), Some("exe"));

        // 无后缀命令入口始终优先于发布产物名。
        std::fs::write(dir.join("cyctron"), b"").unwrap();
        assert_eq!(find_in_dir(&dir, CYCTRON_CANDIDATES), Some(dir.join("cyctron")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
