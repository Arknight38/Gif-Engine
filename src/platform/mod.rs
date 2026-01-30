#[cfg(target_os = "windows")]
pub mod windows;

use winit::window::Window;

pub fn set_overlay(window: &Window) {
    #[cfg(target_os = "windows")]
    windows::set_overlay_mode(window);
}
