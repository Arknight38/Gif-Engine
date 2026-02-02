use std::sync::{Arc, Mutex, OnceLock};

use crate::app::process::ProcessStore;
use crate::app::store::{HotkeyConfig, Store};

#[cfg(target_os = "windows")]
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
        MOD_SHIFT, MOD_WIN,
    },
    WindowsAndMessaging::{
        DispatchMessageW, FindWindowW, GetMessageW, IsWindowVisible, SetForegroundWindow, ShowWindow,
        TranslateMessage, MSG, SW_HIDE, SW_RESTORE, SW_SHOW, WM_HOTKEY,
    },
};

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;

#[cfg(target_os = "windows")]
struct HotkeyContext {
    store: Arc<Mutex<Store>>,
    process_store: Arc<Mutex<ProcessStore>>,
}

#[cfg(target_os = "windows")]
static HOTKEY_CTX: OnceLock<HotkeyContext> = OnceLock::new();

pub fn init_global_hotkeys(store: Arc<Mutex<Store>>, process_store: Arc<Mutex<ProcessStore>>) {
    #[cfg(target_os = "windows")]
    {
        if HOTKEY_CTX.get().is_some() {
            #[cfg(debug_assertions)]
            eprintln!("[hotkeys] init_global_hotkeys: already initialized");
            return;
        }

        let _ = HOTKEY_CTX.set(HotkeyContext {
            store,
            process_store,
        });

        #[cfg(debug_assertions)]
        eprintln!("[hotkeys] init_global_hotkeys: starting hook thread");

        std::thread::spawn(|| {
            #[cfg(debug_assertions)]
            eprintln!("[hotkeys] RegisterHotKey thread started");

            // Register and message loop
            let mut msg = MSG::default();

            // Track current registered combos to allow updates if user changes settings
            let mut registered_toggle: Option<(u32, u32)> = None; // (mods, vk)
            let mut registered_stop: Option<(u32, u32)> = None;

            // Hotkey IDs in this thread
            const ID_TOGGLE: i32 = 1;
            const ID_STOP_ALL: i32 = 2;

            loop {
                // Refresh registration from store every loop iteration that wakes
                // (also after any WM_HOTKEY since GetMessageW wakes frequently)
                let (toggle_cfg, stop_cfg) = {
                    let Some(ctx) = HOTKEY_CTX.get() else {
                        break;
                    };
                    let store = ctx.store.lock().unwrap_or_else(|e| e.into_inner());
                    (store.settings.hotkeys.toggle_manager.clone(), store.settings.hotkeys.stop_all.clone())
                };

                unsafe {
                    update_registration(ID_TOGGLE, &toggle_cfg, &mut registered_toggle);
                    update_registration(ID_STOP_ALL, &stop_cfg, &mut registered_stop);
                }

                // Block waiting for next message (WM_HOTKEY will wake us)
                let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if r.0 <= 0 {
                    break;
                }

                if msg.message == WM_HOTKEY {
                    let id = msg.wParam.0 as i32;
                    #[cfg(debug_assertions)]
                    eprintln!("[hotkeys] WM_HOTKEY received id={}", id);

                    match id {
                        ID_TOGGLE => action_toggle_manager_window(),
                        ID_STOP_ALL => action_stop_all(),
                        _ => {}
                    }
                }

                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });
    }
}

/// Validate key string (e.g. "A", "1", "F5")
pub fn is_key_valid(key: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        parse_key_to_vk(key).is_some()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = key;
        false
    }
}

#[cfg(target_os = "windows")]
fn parse_key_to_vk(key: &str) -> Option<u32> {
    let k = key.trim().to_uppercase();
    if k.len() == 1 {
        let ch = k.chars().next().unwrap();
        if ('A'..='Z').contains(&ch) {
            return Some(ch as u32);
        }
        if ('0'..='9').contains(&ch) {
            return Some(ch as u32);
        }
    }

    if let Some(rest) = k.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u32>() {
            if (1..=24).contains(&n) {
                // Virtual key codes: F1=0x70
                return Some(0x70 + (n - 1));
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn cfg_to_mods_and_vk(cfg: &HotkeyConfig) -> Option<(u32, u32)> {
    if !cfg.enabled {
        return None;
    }
    let vk = parse_key_to_vk(&cfg.key)?;
    let mut mods: HOT_KEY_MODIFIERS = MOD_NOREPEAT;
    if cfg.ctrl { mods |= MOD_CONTROL; }
    if cfg.alt { mods |= MOD_ALT; }
    if cfg.shift { mods |= MOD_SHIFT; }
    if cfg.win { mods |= MOD_WIN; }
    Some((mods.0 as u32, vk))
}

#[cfg(target_os = "windows")]
fn action_toggle_manager_window() {
    #[cfg(debug_assertions)]
    eprintln!("[hotkeys] action: toggle manager window");
    unsafe {
        let window_title = windows::core::w!("Gif-Engine Manager");
        let hwnd = FindWindowW(PCWSTR::null(), window_title);
        if hwnd.0 == 0 {
            #[cfg(debug_assertions)]
            eprintln!("[hotkeys] toggle: manager window not found");
            return;
        }
        if IsWindowVisible(hwnd).as_bool() {
            ShowWindow(hwnd, SW_HIDE);
        } else {
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
fn action_stop_all() {
    #[cfg(debug_assertions)]
    eprintln!("[hotkeys] action: stop all animations");
    let Some(ctx) = HOTKEY_CTX.get() else { return };
    let mut ps = ctx.process_store.lock().unwrap_or_else(|e| e.into_inner());
    let pids: Vec<u32> = ps.processes.keys().cloned().collect();
    for pid in pids {
        let _ = ps.kill_process(pid);
    }
}

#[cfg(target_os = "windows")]
unsafe fn update_registration(id: i32, cfg: &HotkeyConfig, registered: &mut Option<(u32, u32)>) {
    let desired = cfg_to_mods_and_vk(cfg);

    if *registered == desired {
        return;
    }

    // Unregister old
    if registered.is_some() {
        let _ = unsafe { UnregisterHotKey(None, id) };
        #[cfg(debug_assertions)]
        eprintln!("[hotkeys] UnregisterHotKey id={}", id);
    }

    *registered = desired;

    // Register new
    if let Some((mods, vk)) = desired {
        match unsafe { RegisterHotKey(None, id, HOT_KEY_MODIFIERS(mods), vk) } {
            Ok(_) => {
                #[cfg(debug_assertions)]
                eprintln!("[hotkeys] RegisterHotKey id={} mods=0x{:x} vk={}", id, mods, vk);
            }
            Err(e) => {
                eprintln!("[hotkeys] RegisterHotKey failed id={} mods=0x{:x} vk={} err={:?}", id, mods, vk, e);
            }
        }
    }
}


