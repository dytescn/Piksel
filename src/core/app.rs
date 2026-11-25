use crate::application::browser;
use crate::application::vindu;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use wry::{Rect, WebView};

#[cfg(target_os = "linux")]
use winit::event_loop::ControlFlow;

/// 由其他线程（信号处理器轮询线程等）投递到事件循环的事件。
#[derive(Debug, Clone, Copy)]
pub enum UserEvent {
    Exit,
}

pub struct App {
    window: Option<Window>,
    webview: Option<WebView>,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            webview: None,
        }
    }

    fn render(&mut self, size: PhysicalSize<u32>) {
        let (Some(window), Some(webview)) = (self.window.as_ref(), self.webview.as_ref()) else {
            return;
        };
        let size = size.to_logical::<u32>(window.scale_factor());
        let new_bounds = Rect {
            position: LogicalPosition::new(0, 0).into(),
            size: LogicalSize::new(size.width, size.height).into(),
        };
        match webview.bounds() {
            Ok(current_bounds) => {
                if current_bounds != new_bounds {
                    let _ = webview.set_bounds(new_bounds);
                }
            }
            Err(_) => {
                let _ = webview.set_bounds(new_bounds);
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match vindu::init_vindu(event_loop) {
            Ok(window) => window,
            Err(e) => {
                eprintln!("创建窗口失败: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);

        let Some(window) = self.window.as_ref() else {
            return;
        };
        match browser::create_webview(window) {
            Ok(webview) => {
                self.webview = Some(webview);

                #[cfg(target_os = "linux")]
                {
                    // Linux GTK 下 WebView 初始化必须 pump GTK 事件一次，避免闪烁
                    while gtk::events_pending() {
                        gtk::main_iteration_do(false);
                    }
                }
            }
            Err(e) => {
                eprintln!("创建 WebView 失败: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if Some(id) != self.window.as_ref().map(|w| w.id()) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                self.render(size);
            }

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        if matches!(event, UserEvent::Exit) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        {
            // GTK / WebKit 的事件需要定期泵送。用定时器驱动，而不是无条件
            // request_redraw：后者会让 RedrawRequested 始终为空的事件循环
            // 持续自旋，占满 CPU。
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
            _event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(16),
            ));
        }
    }
}
