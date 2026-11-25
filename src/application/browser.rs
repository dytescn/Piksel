use winit::window::Window;
use wry::{WebView, WebViewBuilder};

pub fn create_webview(window: &Window) -> Result<WebView, String> {
    // 发布版不启用 devtools；页面开发与调试请在独立的开发应用中进行。
    let builder = WebViewBuilder::new()
        .with_initialization_script("window.Piksel_DESKTOP = true;");

    let url = url::Url::parse("http://127.0.0.1:44944")
        .map_err(|e| format!("无效的 WebView URL: {e}"))?;

    builder
        .with_url(url)
        .build(window)
        .map_err(|e| format!("WebView 构建失败: {e}"))
}
