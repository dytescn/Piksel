use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub fn create_window(event_loop: &ActiveEventLoop) -> Option<Window> {
    let mut attrs = Window::default_attributes()
        .with_title("Piksel")
        .with_inner_size(LogicalSize::new(1280.0, 800.0));
    event_loop.create_window(attrs).ok()
}
