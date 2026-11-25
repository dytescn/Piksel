use crate::platform;
use winit::{event_loop::ActiveEventLoop, window::Window};
pub fn init_vindu(event_loop: &ActiveEventLoop) -> Result<Window, winit::error::OsError> {
    platform::create_window(event_loop)
}
