use crate::application::browser;
use crate::application::vindu;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use wry::{Rect, WebView};

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
        let window = self.window.as_ref().unwrap();
        let webview = self.webview.as_ref().unwrap();
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = vindu::init_vindu(event_loop);
        self.webview = browser::create_webview(self.window.as_ref());
        #[cfg(target_os = "linux")]
        {
            // Linux GTK 下 WebView 初始化必须 pump GTK 事件一次，避免闪烁
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
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

            WindowEvent::RedrawRequested => {}

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
    }
}
