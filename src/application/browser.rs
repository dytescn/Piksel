use winit::window::Window;
use wry::{WebView, WebViewBuilder};

pub fn create_webview(window: Option<&Window>) -> Option<WebView> {
    let window = window?;

    let builder = WebViewBuilder::new()
        .with_initialization_script("window.Piksel_DESKTOP = true;")
        .with_devtools(true);

    let url = url::Url::parse("http://127.0.0.1:44944").ok()?;
    let builder = builder.with_url(url);

    builder.build(window).ok()
}
