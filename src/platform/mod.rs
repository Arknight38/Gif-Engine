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

pub fn set_window_owner(window: &Window, owner_hwnd: isize) {
    #[cfg(target_os = "windows")]
    windows::set_window_owner(window, owner_hwnd);
}

pub fn set_topmost(window: &Window, enable: bool) {
    #[cfg(target_os = "windows")]
    windows::set_topmost(window, enable);
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

pub fn is_alt_pressed() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::is_alt_pressed()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub const ID_PAUSE: usize = 101;
pub const ID_STOP: usize = 102;
pub const ID_SETTINGS: usize = 103;
pub const ID_TOGGLE_CLICKTHROUGH: usize = 104;

pub fn show_context_menu(window: &Window, x: i32, y: i32, paused: bool, click_through: bool) -> Option<usize> {
    #[cfg(target_os = "windows")]
    {
        windows::show_context_menu(window, x, y, paused, click_through)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn get_hwnd_by_title(title: &str) -> Option<isize> {
    #[cfg(target_os = "windows")]
    {
        windows::get_hwnd_by_title(title)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn get_window_rect_by_hwnd(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        windows::get_window_rect_by_hwnd(hwnd)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn focus_window(title: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::focus_window(title)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn get_window_rect_by_title(title: &str) -> Option<(i32, i32, i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        windows::get_window_rect_by_title(title)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
