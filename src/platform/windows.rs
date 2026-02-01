use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_APPWINDOW, WS_EX_TRANSPARENT, SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub fn set_overlay_mode(window: &Window, click_through: bool) {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);

            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

                // Add WS_EX_LAYERED, WS_EX_TOPMOST, WS_EX_TOOLWINDOW
                // REMOVE WS_EX_APPWINDOW to prevent taskbar appearance
                let mut new_style = ex_style | (WS_EX_LAYERED.0 as isize) | (WS_EX_TOPMOST.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
                new_style &= !(WS_EX_APPWINDOW.0 as isize);
                
                // Add WS_EX_TRANSPARENT for click-through if enabled
                if click_through {
                    new_style |= WS_EX_TRANSPARENT.0 as isize;
                }
                
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

                // Force window to top using SetWindowPos
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                );
            }
        }
    }
}

pub fn set_click_through(window: &Window, enabled: bool) {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);

            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let mut new_style = ex_style;
                
                if enabled {
                    new_style |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    new_style &= !(WS_EX_TRANSPARENT.0 as isize);
                }
                
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
            }
        }
    }
}

pub fn is_ctrl_pressed() -> bool {
    unsafe {
        // VK_CONTROL = 0x11
        // GetAsyncKeyState returns a SHORT where bit 15 indicates if key is currently pressed
        let state = GetAsyncKeyState(0x11);
        (state as u16 & 0x8000) != 0
    }
}
