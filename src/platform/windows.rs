use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use windows::core::HSTRING;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO,
};

/// 平台级初始化。Windows 无需额外处理。
pub fn init() {}

pub fn create_window(event_loop: &ActiveEventLoop) -> Result<Window, winit::error::OsError> {
    let attrs = Window::default_attributes()
        .with_title("Piksel")
        .with_inner_size(LogicalSize::new(800.0, 600.0));
    event_loop.create_window(attrs)
}

/// 原生「是 / 否」对话框，返回用户是否选择「是」。
pub fn prompt_yes_no(message: &str) -> bool {
    let text = HSTRING::from(message);
    let title = HSTRING::from("Piksel");
    unsafe { MessageBoxW(None, &text, &title, MB_YESNO | MB_ICONQUESTION) == IDYES }
}

/// 原生信息提示框。
pub fn show_message(message: &str) {
    let text = HSTRING::from(message);
    let title = HSTRING::from("Piksel");
    unsafe {
        MessageBoxW(None, &text, &title, MB_OK | MB_ICONINFORMATION);
    }
}
