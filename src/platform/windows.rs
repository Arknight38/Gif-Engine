use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWLP_HWNDPARENT, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_APPWINDOW, WS_EX_TRANSPARENT, SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
    CreatePopupMenu, AppendMenuW, TrackPopupMenu, DestroyMenu,
    TPM_RETURNCMD, TPM_NONOTIFY, TPM_RIGHTBUTTON, MF_STRING, MF_CHECKED, MF_UNCHECKED, MF_SEPARATOR,
    FindWindowW, SetForegroundWindow, IsIconic, ShowWindow, SW_RESTORE, GetWindowRect,
    EnumWindows, GetWindowTextW, GetWindowTextLengthW, IsWindowVisible,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU, VK_CONTROL};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
use windows::core::PCWSTR;

pub const ID_PAUSE: usize = 101;
pub const ID_STOP: usize = 102;
pub const ID_SETTINGS: usize = 103;
pub const ID_TOGGLE_CLICKTHROUGH: usize = 104;

pub fn focus_window(title: &str) -> bool {
    unsafe {
        let title_w = windows::core::HSTRING::from(title);
        let hwnd = FindWindowW(None, PCWSTR(title_w.as_ptr()));
        if hwnd.0 != 0 {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
            return true;
        }
    }
    false
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
    // Check visibility first (unsafe call inside unsafe fn requires unsafe block in 2024 edition if not careful, 
    // but actually the fn itself is unsafe so we can call unsafe functions... 
    // Wait, Rust 2024 requires unsafe blocks even inside unsafe functions for unsafe ops.
    // Let's just wrap the body in unsafe or individual calls.
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                if GetWindowTextW(hwnd, &mut buf) > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    let list = &mut *(lparam.0 as *mut Vec<String>);
                    if !title.trim().is_empty() {
                        list.push(title);
                    }
                }
            }
        }
    }
    windows::Win32::Foundation::BOOL(1)
}

pub fn get_open_window_titles() -> Vec<String> {
    let mut titles = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_window_proc), windows::Win32::Foundation::LPARAM(&mut titles as *mut _ as isize));
    }
    titles.sort();
    titles.dedup();
    titles
}

struct FindResult {
    search: String,
    found_hwnd: Option<HWND>,
}

unsafe extern "system" fn find_partial_proc_struct(hwnd: HWND, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
    unsafe {
        let result = &mut *(lparam.0 as *mut FindResult);
        
        if IsWindowVisible(hwnd).as_bool() {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                if GetWindowTextW(hwnd, &mut buf) > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    if title.to_lowercase().contains(&result.search) {
                        result.found_hwnd = Some(hwnd);
                        return windows::Win32::Foundation::BOOL(0); // Stop
                    }
                }
            }
        }
    }
    windows::Win32::Foundation::BOOL(1)
}

pub fn get_window_rect_by_title(title: &str) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        // First try exact match (fast)
        let title_w = windows::core::HSTRING::from(title);
        let hwnd = FindWindowW(None, PCWSTR(title_w.as_ptr()));
        
        let target_hwnd = if hwnd.0 != 0 {
            Some(hwnd)
        } else {
            // Fallback to partial match
            let mut result = FindResult {
                search: title.to_lowercase(),
                found_hwnd: None,
            };
            let _ = EnumWindows(Some(find_partial_proc_struct), windows::Win32::Foundation::LPARAM(&mut result as *mut _ as isize));
            result.found_hwnd
        };

        if let Some(hwnd) = target_hwnd {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                return Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
            }
        }
    }
    None
}

pub fn get_hwnd_by_title(title: &str) -> Option<isize> {
    unsafe {
        // First try exact match
        let title_w = windows::core::HSTRING::from(title);
        let hwnd = FindWindowW(None, PCWSTR(title_w.as_ptr()));
        
        if hwnd.0 != 0 {
            return Some(hwnd.0 as isize);
        }

        // Fallback to partial match
        let mut result = FindResult {
            search: title.to_lowercase(),
            found_hwnd: None,
        };
        let _ = EnumWindows(Some(find_partial_proc_struct), windows::Win32::Foundation::LPARAM(&mut result as *mut _ as isize));
        
        result.found_hwnd.map(|h| h.0 as isize)
    }
}

pub fn get_window_rect_by_hwnd(hwnd_val: isize) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let hwnd = HWND(hwnd_val);
        if IsWindowVisible(hwnd).as_bool() {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                return Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
            }
        }
    }
    None
}

pub fn set_window_owner(window: &Window, owner_hwnd: isize) {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner_hwnd as isize);
            }
        }
    }
}

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

pub fn set_topmost(window: &Window, enable: bool) {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);

            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let mut new_style = ex_style;
                
                let hwnd_insert_after = if enable {
                    new_style |= WS_EX_TOPMOST.0 as isize;
                    HWND_TOPMOST
                } else {
                    new_style &= !(WS_EX_TOPMOST.0 as isize);
                    HWND_NOTOPMOST
                };
                
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

                let _ = SetWindowPos(
                    hwnd,
                    hwnd_insert_after,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                );
            }
        }
    }
}

pub fn is_ctrl_pressed() -> bool {
    unsafe {
        // VK_CONTROL = 0x11
        let state = GetAsyncKeyState(VK_CONTROL.0 as i32);
        (state as u16 & 0x8000) != 0
    }
}

pub fn is_alt_pressed() -> bool {
    unsafe {
        // VK_MENU = 0x12 (Alt key)
        let state = GetAsyncKeyState(VK_MENU.0 as i32);
        (state as u16 & 0x8000) != 0
    }
}

pub fn show_context_menu(window: &Window, x: i32, y: i32, paused: bool, click_through: bool) -> Option<usize> {
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = HWND(handle.hwnd.get() as _);
            
            unsafe {
                let menu = CreatePopupMenu().ok()?;
                
                let pause_text = if paused { "Resume" } else { "Pause" };
                let pause_w = windows::core::HSTRING::from(pause_text);
                let _ = AppendMenuW(menu, MF_STRING, ID_PAUSE, PCWSTR(pause_w.as_ptr()));
                
                // Click Through Toggle
                let ct_flags = if click_through { MF_CHECKED } else { MF_UNCHECKED };
                let ct_w = windows::core::HSTRING::from("Click Through");
                let _ = AppendMenuW(menu, ct_flags, ID_TOGGLE_CLICKTHROUGH, PCWSTR(ct_w.as_ptr()));

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                let settings_w = windows::core::HSTRING::from("Settings");
                let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS, PCWSTR(settings_w.as_ptr()));

                let stop_w = windows::core::HSTRING::from("Stop");
                let _ = AppendMenuW(menu, MF_STRING, ID_STOP, PCWSTR(stop_w.as_ptr()));
                
                // TrackPopupMenu returns the selected command ID because we used TPM_RETURNCMD
                let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON;
                let result = TrackPopupMenu(menu, flags, x, y, 0, hwnd, None);
                
                let _ = DestroyMenu(menu);
                
                if result.0 != 0 {
                    return Some(result.0 as usize);
                }
            }
        }
    }
    None
}
