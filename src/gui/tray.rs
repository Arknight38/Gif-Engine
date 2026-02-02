use std::sync::mpsc;
use std::thread;
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};
use tray_icon::Icon;
use image::GenericImageView;
use auto_launch::AutoLaunch;
use crate::app::process::ProcessStore;

// Commands that the background tray thread can send to the GUI
// NOTE: On Windows, tray commands are handled directly in the tray thread using Windows APIs
// This enum is kept for non-Windows platforms or as a fallback
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used on non-Windows platforms
pub enum TrayCommand {
    ShowWindow,
    QuitApplication,
}

pub fn setup_tray(_tray_cmd_tx: mpsc::Sender<TrayCommand>) -> (Option<tray_icon::TrayIcon>, Option<Box<Menu>>, Option<MenuItem>, Option<MenuItem>) {
    let icon_data = load_icon_data();
    let icon = match Icon::from_rgba(icon_data.rgba, icon_data.width, icon_data.height) {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("Warning: Failed to create tray icon: {:?}. Tray icon may not appear.", e);
            // Return None for tray icon if creation fails
            return (None, None, None, None);
        }
    };

    // Create tray icon in the main thread - no separate event loop needed
    // Tray events will be checked in the GUI's update loop using MenuEvent::receiver()
    let tray_menu = Menu::new();
    let show_item = MenuItem::new("Show Manager", true, None);
    let quit_item = MenuItem::new("Quit Gif-Engine", true, None);
    
    #[cfg(debug_assertions)]
    {
        eprintln!("[DEBUG] Tray menu setup:");
        eprintln!("[DEBUG]   Show item ID: {:?}", show_item.id());
        eprintln!("[DEBUG]   Quit item ID: {:?}", quit_item.id());
    }
    
    let _ = tray_menu.append(&show_item);
    let _ = tray_menu.append(&quit_item);
    
    let menu_box = Box::new(tray_menu);
    let tray_icon = TrayIconBuilder::new()
        .with_menu(menu_box.clone())
        .with_tooltip("Gif-Engine Manager")
        .with_icon(icon)
        .build()
        .ok();
    
    #[cfg(debug_assertions)]
    if tray_icon.is_some() {
        eprintln!("[DEBUG] Tray icon created successfully");
    } else {
        eprintln!("[DEBUG] Failed to create tray icon");
    }
    
    (tray_icon, Some(menu_box), Some(quit_item), Some(show_item))
}

pub fn load_icon_data() -> eframe::egui::IconData {
    // Embed the icon at compile time
    let icon_bytes = include_bytes!("../icon.png");
    
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let (width, height) = img.dimensions();
        let rgba = img.into_rgba8().into_raw();
        return eframe::egui::IconData { rgba, width, height };
    }

    // Fallback: Create a simple 32x32 colored box (e.g., Purple)
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for _ in 0..width {
            rgba.push(100); // R
            rgba.push(50);  // G
            rgba.push(200); // B
            rgba.push(255); // A
        }
    }
    eframe::egui::IconData { rgba, width, height }
}

pub fn get_auto_launch() -> Option<AutoLaunch> {
    let app_name = "Gif-Engine";
    let app_path = std::env::current_exe().ok()?;
    let path_str = app_path.to_str()?;
    Some(AutoLaunch::new(app_name, path_str, &[] as &[&str]))
}

pub fn spawn_tray_thread(quit_id: Option<tray_icon::menu::MenuId>, show_id: Option<tray_icon::menu::MenuId>, tray_cmd_tx: mpsc::Sender<TrayCommand>) {
    thread::spawn(move || {
        #[cfg(debug_assertions)]
        eprintln!("[DEBUG] Tray thread: Started polling for tray events");
        
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                FindWindowW, ShowWindow, SW_SHOW, SW_RESTORE,
                SetForegroundWindow, IsIconic,
            };
            use windows::core::PCWSTR;
            
            loop {
                // Poll for tray menu events - this doesn't require a winit event loop
                if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Tray thread: Menu event received: event_id={:?}", menu_event.id);
                    
                    // Handle quit directly - kill all processes and exit
                    // This works even when the window is hidden and update() isn't being called
                    if let Some(quit_id_val) = &quit_id {
                        if menu_event.id == *quit_id_val {
                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] Tray thread: Quit command - killing processes and exiting");
                            
                            // Kill all running animation processes before exiting
                            let mut ps = ProcessStore::load();
                            let pids: Vec<u32> = ps.processes.keys().cloned().collect();
                            for pid in pids {
                                let _ = ps.kill_process(pid);
                            }
                            
                            // Exit immediately
                            std::process::exit(0);
                        }
                    }
                    
                    // Handle show directly using Windows APIs
                    if let Some(show_id_val) = &show_id {
                        if menu_event.id == *show_id_val {
                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] Tray thread: Show command - showing window directly");
                            
                            // Find window by title
                            let window_title = windows::core::w!("Gif-Engine Manager");
                            unsafe {
                                let hwnd = FindWindowW(PCWSTR::null(), window_title);
                                // Check if hwnd is valid (not null)
                                if hwnd.0 != 0 {
                                    // Check if window is minimized
                                    let is_minimized = IsIconic(hwnd);
                                    
                                    if is_minimized.as_bool() {
                                        ShowWindow(hwnd, SW_RESTORE);
                                    } else {
                                        ShowWindow(hwnd, SW_SHOW);
                                    }
                                    
                                    // Bring to foreground
                                    let _ = SetForegroundWindow(hwnd);
                                    
                                    #[cfg(debug_assertions)]
                                    eprintln!("[DEBUG] Tray thread: Window shown successfully");
                                } else {
                                    #[cfg(debug_assertions)]
                                    eprintln!("[DEBUG] Tray thread: Could not find window");
                                }
                            }
                        }
                    }
                }
                
                // Small sleep to avoid busy-waiting
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // Non-Windows: fall back to channel-based approach
            let cmd_tx = tray_cmd_tx.clone();
            loop {
                if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
                    if let Some(quit_id_val) = &quit_id {
                        if menu_event.id == *quit_id_val {
                            let _ = cmd_tx.send(TrayCommand::QuitApplication);
                            continue;
                        }
                    }
                    if let Some(show_id_val) = &show_id {
                        if menu_event.id == *show_id_val {
                            let _ = cmd_tx.send(TrayCommand::ShowWindow);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    });
}

