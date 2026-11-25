//! 全局 `dytes` 命令行的定位、更新调用与安装提示。
//!
//! dytes 不再作为 Piksel 的安装依赖（`dytes.json` 的 `dependencies-core` 中已移除），
//! 启动更新直接调用 PATH 中的全局 `dytes` 命令。未安装时在启动阶段弹出提示，
//! 用户确认后执行官方安装脚本。

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::core::paths;
use crate::platform;

/// 官方安装命令，用于提示与失败后的手动指引。
#[cfg(windows)]
const INSTALL_COMMAND: &str = "irm https://www.dytes.cn/install.ps1 | iex";
#[cfg(not(windows))]
const INSTALL_COMMAND: &str = "curl -fsSL https://www.dytes.cn/install.sh | sh";

/// 全局安装目录中可能出现的命令文件名。
#[cfg(windows)]
const CANDIDATES: &[&str] = &["dytes.exe", "dytes.cmd", "dytes.bat", "dytes"];
#[cfg(not(windows))]
const CANDIDATES: &[&str] = &["dytes"];

/// 查找全局 dytes 命令的绝对路径。
///
/// 先按 PATH 查找；安装脚本刚写入 PATH 时当前进程可能还看不到，因此再回退到
/// 安装脚本的默认目录（Windows `%LOCALAPPDATA%\dytes\bin`，Unix `~/.local/bin`）。
pub fn find_global() -> Option<PathBuf> {
    find_in_path().or_else(find_in_install_dir)
}

fn find_in_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        // 空目录项（如 `:/bin`）会解析为当前目录，忽略以免误匹配。
        .filter(|dir| !dir.as_os_str().is_empty())
        .find_map(|dir| first_existing(&dir))
}

fn find_in_install_dir() -> Option<PathBuf> {
    install_dir().and_then(|dir| first_existing(&dir))
}

/// 安装脚本的默认目录，可用 `DYTES_INSTALL_DIR` 覆盖。
fn install_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("DYTES_INSTALL_DIR") {
        return Some(PathBuf::from(dir));
    }

    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("dytes").join("bin"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(|dir| PathBuf::from(dir).join(".local").join("bin"))
    }
}

fn first_existing(dir: &Path) -> Option<PathBuf> {
    CANDIDATES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// 启动阶段确保全局 dytes 可用：缺失时提示用户下载安装。
///
/// 必须在 GTK 初始化之后、创建窗口之前调用（Linux 的提示框依赖 GTK）。
pub fn ensure_installed() {
    if find_global().is_some() {
        return;
    }

    let message = format!(
        "Piksel 使用全局 dytes 命令检查并安装更新，但当前未检测到 dytes。\n\n\
         是否现在下载并安装 dytes？\n\n\
         安装方式：{INSTALL_COMMAND}"
    );
    if !platform::prompt_yes_no(&message) {
        eprintln!("未检测到全局 dytes，已跳过启动更新。");
        return;
    }

    match install() {
        Ok(()) if find_global().is_some() => {
            platform::show_message("dytes 安装完成。");
        }
        Ok(()) => {
            platform::show_message(
                "dytes 已安装，但当前 PATH 中仍找不到该命令。\n\n\
                 请重新登录系统，或把 dytes 安装目录加入 PATH 后重新启动 Piksel。",
            );
        }
        Err(e) => {
            platform::show_message(&format!(
                "dytes 安装失败：{e}\n\n可手动执行：\n{INSTALL_COMMAND}"
            ));
        }
    }
}

/// 执行官方安装脚本。Windows 下弹出独立控制台窗口，便于查看下载进度。
fn install() -> io::Result<()> {
    #[cfg(windows)]
    {
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                INSTALL_COMMAND,
            ])
            .creation_flags(CREATE_NEW_CONSOLE)
            .status()?;
        check_status(status)
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("sh").args(["-c", INSTALL_COMMAND]).status()?;
        check_status(status)
    }
}

fn check_status(status: std::process::ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("安装脚本退出码 {status}")))
    }
}

/// 后台打开 `dytes update --gui` 图形更新窗口，不阻塞 Piksel 启动。
///
/// 更新失败、dytes 缺失或超时都只记录日志，窗口与 cyctron 已经可用。
pub fn spawn_background_update() {
    std::thread::spawn(|| {
        let Some(exe) = find_global() else {
            eprintln!("未检测到全局 dytes，跳过启动更新");
            return;
        };

        println!("Running: {}", exe.display());
        let mut cmd = Command::new(&exe);
        cmd.args(["update", "--gui"]);
        // dytes 按当前目录下的 dytes.json 识别被更新的应用。
        cmd.current_dir(paths::app_dir());
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match cmd.status() {
            Ok(status) => println!("后台更新完成: {status}"),
            Err(e) => eprintln!("后台更新失败: {e}"),
        }
    });
}
