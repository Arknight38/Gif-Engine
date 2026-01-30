use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_APPWINDOW, SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub fn set_overlay_mode(window: &Window) {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);

            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

                // Add WS_EX_LAYERED, WS_EX_TOPMOST, WS_EX_TOOLWINDOW
                // REMOVE WS_EX_APPWINDOW to prevent taskbar appearance
                let mut new_style = ex_style | (WS_EX_LAYERED.0 as isize) | (WS_EX_TOPMOST.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
                new_style &= !(WS_EX_APPWINDOW.0 as isize);
                
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
