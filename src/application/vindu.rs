use winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};
#[allow(dead_code)]
pub fn init_vindu(
    event: &ActiveEventLoop,
    debug: bool,
) -> Option<Window> {

    let attrs = if debug {
        WindowAttributes::default()
            .with_title("Piksel Debug")
            .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0))
    } else {
        WindowAttributes::default()
            .with_title("Piksel")
            .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0))
    };

    event.create_window(attrs).ok()
}
