#[cfg(target_os = "windows")]
pub mod windows;

use winit::window::Window;

pub fn set_overlay(window: &Window, click_through: bool) {
    #[cfg(target_os = "windows")]
    windows::set_overlay_mode(window, click_through);
}

pub fn set_click_through(window: &Window, enabled: bool) {
    #[cfg(target_os = "windows")]
    windows::set_click_through(window, enabled);
}

pub fn is_ctrl_pressed() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::is_ctrl_pressed()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
