use winit::window::Window;
use wry::{WebView, WebViewBuilder};

pub fn create_webview(
    window: Option<&Window>,
    _debug: bool,
) -> Option<WebView> {
    let window = window?;
    let mut builder = WebViewBuilder::new()
        .with_initialization_script("window.IS_DESKTOP = true;");
    builder = builder
            .with_url("http://127.0.0.1:44944");
    builder.build(window).ok()
}
