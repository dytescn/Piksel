use std::ffi::{c_int, c_uchar, c_ulong, c_void};

use gtk::prelude::*;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::platform::x11::register_xlib_error_hook;
use winit::window::Window;

/// GLXBadWindow 的 X11 错误码。
const GLX_BAD_WINDOW: c_uchar = 170;

/// Xlib 的 `XErrorEvent`，与 libX11 的 C 布局一致。
#[repr(C)]
#[allow(dead_code)] // 字段只用于对齐布局，实际只读取 error_code
struct XErrorEvent {
    type_: c_int,
    display: *mut c_void,
    resourceid: c_ulong,
    serial: c_ulong,
    error_code: c_uchar,
    request_code: c_uchar,
    minor_code: c_uchar,
}

/// 平台级初始化（Linux）：吞掉 webkit 与 winit 共用 GLX 时产生的 GLXBadWindow。
///
/// 这类错误多为异步产生（窗口销毁 / 焦点切换时），会污染 winit 的下一次同步
/// 请求，表现为 `Failed to focus input context: XError { GLXBadWindow }` 并
/// 直接 panic（见 issue #1）。`register_xlib_error_hook` 是 winit 提供的公开
/// 接口，wry 官方 winit 示例同样用它忽略 170。
///
/// 必须在任何 X11 / GLX 初始化（GTK、EventLoop、WebView）之前调用。
pub fn init() {
    register_xlib_error_hook(Box::new(|_display, event| {
        if event.is_null() {
            return false;
        }
        // 返回 true 表示错误已处理，winit 不再把它交给 IME focus 的 expect。
        unsafe { (*(event as *const XErrorEvent)).error_code == GLX_BAD_WINDOW }
    }));
}

pub fn create_window(event_loop: &ActiveEventLoop) -> Result<Window, winit::error::OsError> {
    let attrs = Window::default_attributes()
        .with_title("Piksel")
        .with_inner_size(LogicalSize::new(800.0, 600.0));
    event_loop.create_window(attrs)
}

/// 原生「是 / 否」对话框，返回用户是否选择「是」。
///
/// 调用前必须已经 `gtk::init()`（见 `main.rs` 启动流程）。
pub fn prompt_yes_no(message: &str) -> bool {
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Question,
        gtk::ButtonsType::YesNo,
        message,
    );
    dialog.set_title("Piksel");
    let response = dialog.run();
    dialog.close();
    response == gtk::ResponseType::Yes
}

/// 原生信息提示框。
pub fn show_message(message: &str) {
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::Ok,
        message,
    );
    dialog.set_title("Piksel");
    dialog.run();
    dialog.close();
}
