use winit::window::Window;
use wry::{WebView, WebViewBuilder};

pub fn create_webview(window: &Window) -> Result<WebView, String> {
    let builder = WebViewBuilder::new()
        .with_initialization_script("window.Piksel_DESKTOP = true;")
        .with_devtools(true);

    let url = url::Url::parse("http://127.0.0.1:44944")
        .map_err(|e| format!("无效的 WebView URL: {e}"))?;

    builder
        .with_url(url)
        .build(window)
        .map_err(|e| format!("WebView 构建失败: {e}"))
}
