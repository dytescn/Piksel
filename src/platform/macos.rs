use std::process::Command;

use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// 平台级初始化。macOS 无需额外处理。
pub fn init() {}

pub fn create_window(event_loop: &ActiveEventLoop) -> Result<Window, winit::error::OsError> {
    let attrs = Window::default_attributes()
        .with_title("Piksel")
        .with_inner_size(LogicalSize::new(1280.0, 800.0));
    event_loop.create_window(attrs)
}

/// 原生「是 / 否」对话框，返回用户是否选择「是」。
pub fn prompt_yes_no(message: &str) -> bool {
    let script = format!(
        "display dialog \"{}\" buttons {{\"取消\", \"安装\"}} default button \"安装\" with title \"Piksel\"",
        escape(message)
    );
    Command::new("osascript")
        .args(["-e", &script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// 原生信息提示框。
pub fn show_message(message: &str) {
    let script = format!(
        "display alert \"Piksel\" message \"{}\" buttons {{\"好\"}} default button \"好\"",
        escape(message)
    );
    let _ = Command::new("osascript").args(["-e", &script]).status();
}

/// 转义 AppleScript 字符串字面量中的反斜杠与双引号。
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
