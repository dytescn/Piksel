#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
pub use windows::{create_window, init, prompt_yes_no, show_message};

#[cfg(target_os = "macos")]
pub use macos::{create_window, init, prompt_yes_no, show_message};

#[cfg(target_os = "linux")]
pub use linux::{create_window, init, prompt_yes_no, show_message};
