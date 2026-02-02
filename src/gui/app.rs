use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use crate::app::store::Store;
use crate::app::process::ProcessStore;
use crate::types::{Frame, AnimationInfo};
use crate::gui::tray::{TrayCommand, get_auto_launch};
use crate::gui::preview::PreviewState;

#[derive(PartialEq)]
pub enum ViewMode {
    Library,
    Active,
    Settings,
}

pub struct AnimeApp {
    pub store: Arc<Mutex<Store>>,
    pub process_store: Arc<Mutex<ProcessStore>>,
    pub view: ViewMode,
    
    // Selection & Preview
    pub selected_name: Option<String>,
    pub preview: Option<PreviewState>,
    pub load_rx: Option<mpsc::Receiver<Result<(AnimationInfo, Vec<Frame>), String>>>,
    pub is_loading: bool,
    pub load_error: Option<String>, // Store error message for UI display
    
    // UI State
    pub refresh_timer: std::time::Instant,
    pub input_path: String,
    pub search_query: String, // Search query for library filtering
    
    // Tray
    pub _tray_icon: Option<tray_icon::TrayIcon>,
    pub _tray_menu: Option<Box<tray_icon::menu::Menu>>, // Keep menu alive for tray events to work
    #[allow(dead_code)] // Used on non-Windows platforms
    pub quit_item: Option<tray_icon::menu::MenuItem>,
    #[allow(dead_code)] // Used on non-Windows platforms
    pub show_item: Option<tray_icon::menu::MenuItem>,
    pub should_exit: bool,
    pub tray_cmd_rx: mpsc::Receiver<TrayCommand>, // Receive commands from background tray thread
    pub pending_close_canceled: bool, // Track if we've canceled a pending close request
    
    // Global Settings
    pub startup_enabled: bool,
}

impl AnimeApp {
    // Helper function to handle mutex poisoning gracefully
    pub fn lock_store(store: &Arc<Mutex<Store>>) -> std::sync::MutexGuard<'_, Store> {
        store.lock().unwrap_or_else(|e| {
            eprintln!("Warning: Store mutex poisoned, recovering");
            e.into_inner()
        })
    }
    
    pub fn lock_process_store(store: &Arc<Mutex<ProcessStore>>) -> std::sync::MutexGuard<'_, ProcessStore> {
        store.lock().unwrap_or_else(|e| {
            eprintln!("Warning: ProcessStore mutex poisoned, recovering");
            e.into_inner()
        })
    }
    
    pub fn new(tray_icon: Option<tray_icon::TrayIcon>, tray_menu: Option<Box<tray_icon::menu::Menu>>, quit_item: Option<tray_icon::menu::MenuItem>, show_item: Option<tray_icon::menu::MenuItem>, tray_cmd_rx: mpsc::Receiver<TrayCommand>) -> Self {
        let startup_enabled = if let Some(auto) = get_auto_launch() {
            auto.is_enabled().unwrap_or(false)
        } else {
            false
        };

        Self {
            store: Arc::new(Mutex::new(Store::load())),
            process_store: Arc::new(Mutex::new(ProcessStore::load())),
            view: ViewMode::Library,
            selected_name: None,
            preview: None,
            load_rx: None,
            is_loading: false,
            load_error: None,
            refresh_timer: std::time::Instant::now(),
            input_path: String::new(),
            search_query: String::new(),
            _tray_icon: tray_icon,
            _tray_menu: tray_menu,
            quit_item,
            show_item,
            should_exit: false,
            tray_cmd_rx,
            pending_close_canceled: false,
            startup_enabled,
        }
    }
}

impl Default for AnimeApp {
    fn default() -> Self {
        let (tray_cmd_tx, tray_cmd_rx) = mpsc::channel();
        let (tray_icon, tray_menu, quit_item, show_item) = crate::gui::tray::setup_tray(tray_cmd_tx);
        Self::new(tray_icon, tray_menu, quit_item, show_item, tray_cmd_rx)
    }
}

impl eframe::App for AnimeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle commands from the background tray thread
        // The tray thread polls MenuEvent::receiver() and sends commands via channel
        // This works even when the window is hidden because the thread runs independently
        // Use blocking receive with timeout to ensure we process quit commands even when hidden
        loop {
            match self.tray_cmd_rx.try_recv() {
                Ok(cmd) => {
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] GUI: Received command from tray thread: {:?}", cmd);
                    
                    match cmd {
                        TrayCommand::ShowWindow => {
                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] GUI: Showing window");
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                            // Reset close cancel flag when showing window
                            self.pending_close_canceled = false;
                        }
                        TrayCommand::QuitApplication => {
                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] GUI: Quit requested from tray - killing child animations");

                            // Kill all running animation processes before exiting
                            {
                                let mut ps = Self::lock_process_store(&self.process_store);
                                let pids: Vec<u32> = ps.processes.keys().cloned().collect();
                                for pid in pids {
                                    let _ = ps.kill_process(pid);
                                }
                            }

                            // Force exit immediately - don't wait for window close
                            std::process::exit(0);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // On Windows, the tray thread handles commands directly via Windows APIs
                    // and doesn't use the channel, so disconnection is expected
                    // Only log on non-Windows platforms where the channel is actually used
                    #[cfg(all(debug_assertions, not(target_os = "windows")))]
                    eprintln!("[DEBUG] GUI: Tray thread channel disconnected");
                    break;
                }
            }
        }

        // Handle Window Close Request
        // X button: Hide window to tray (app keeps running) OR Quit based on setting
        // Tray "Quit": should_exit=true, then window closes and app exits
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        
        if close_requested {
            if self.should_exit {
                // Quit was requested from tray menu - allow close and exit
                #[cfg(debug_assertions)]
                eprintln!("[DEBUG] Window close requested and should_exit=true - allowing close and exit");
                // The window will close and the process will exit (handled by tray thread)
                self.pending_close_canceled = false; // Reset flag when actually closing
            } else if !self.pending_close_canceled {
                // User clicked X button
                
                // Check setting: Minimize to tray or Quit?
                let minimize_to_tray = {
                    let store = Self::lock_store(&self.store);
                    store.settings.minimize_to_tray
                };

                if minimize_to_tray {
                    // Hide to tray, keep app running
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Window close requested (X button) - hiding to tray, app stays running");
                    
                    // Cancel the close request first
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    
                    // Hide window using Windows APIs (more reliable than eframe commands)
                    #[cfg(target_os = "windows")]
                    {
                        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE};
                        use windows::core::PCWSTR;
                        
                        let window_title = windows::core::w!("Gif-Engine Manager");
                        unsafe {
                            let hwnd = FindWindowW(PCWSTR::null(), window_title);
                            if hwnd.0 != 0 {
                                ShowWindow(hwnd, SW_HIDE);
                                #[cfg(debug_assertions)]
                                eprintln!("[DEBUG] Window hidden using Windows API");
                            }
                        }
                    }
                    
                    // Also try eframe command as fallback
                    #[cfg(not(target_os = "windows"))]
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    
                    self.pending_close_canceled = true; // Mark that we've canceled
                } else {
                    // Quit application
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Window close requested (X button) - quitting application (minimize_to_tray=false)");
                    self.should_exit = true;
                    // Don't cancel close, let it propagate.
                    // But we should also make sure the process actually exits, 
                    // because eframe might just close the window and wait if run_and_return=false
                    // So we spawn a thread to force exit after a moment, just like tray quit
                     std::thread::spawn(|| {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        std::process::exit(0);
                    });
                }
            }
        } else {
            // No close request - reset the flag so we can handle the next one
            self.pending_close_canceled = false;
        }

        // Set custom visuals
        let theme = {
            let store = Self::lock_store(&self.store);
            store.settings.theme.clone()
        };

        let mut visuals = if theme == "light" {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        if theme != "light" {
            visuals.window_fill = egui::Color32::from_rgb(20, 20, 25);
            visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 45);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 70);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 90);
        }
        ctx.set_visuals(visuals);

        // Auto-refresh process list every second
        if self.refresh_timer.elapsed().as_secs() >= 1 {
            if let Ok(mut ps) = self.process_store.lock() {
                ps.cleanup_dead_processes();
            }
            self.refresh_timer = std::time::Instant::now();
        }

        // Handle async loading
        if let Some(rx) = &self.load_rx {
            if let Ok(result) = rx.try_recv() {
                self.is_loading = false;
                self.load_rx = None;
                match result {
                    Ok((info, frames)) => {
                        self.load_error = None; // Clear any previous errors
                        self.preview = Some(PreviewState::new(frames, info));
                    }
                    Err(e) => {
                        // Store error for UI display instead of just printing
                        self.load_error = Some(e.clone());
                        eprintln!("Failed to load preview: {}", e);
                    }
                }
            }
        }

        // Update Preview Animation
        if let Some(preview) = &mut self.preview {
            preview.update(ctx);
        }

        // SIDEBAR
        egui::SidePanel::left("library_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                // Wrap entire sidebar in scroll area
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            self.show_library(ui);
                        });
                    });
                
                // Bottom Navigation Buttons (outside scroll area so they stay visible)
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui.selectable_label(self.view == ViewMode::Settings, "⚙ Settings").clicked() {
                        self.view = ViewMode::Settings;
                        self.selected_name = None; // Deselect animation when going to settings
                    }
                    ui.separator();
                    if ui.selectable_label(self.view == ViewMode::Active, "▶ Active").clicked() {
                        self.view = ViewMode::Active;
                        self.selected_name = None; // Deselect animation when going to active
                    }
                    ui.separator();
                });
            });

        // MAIN CONTENT
        egui::CentralPanel::default().show(ctx, |ui| {
            // Wrap content in scroll area to handle overflow
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    match self.view {
                        ViewMode::Library => {
                            if let Some(name) = &self.selected_name {
                                self.show_selected_animation(ui, name.clone());
                            } else {
                                ui.centered_and_justified(|ui| {
                                    ui.label("Select an animation from the library to edit.");
                                });
                            }
                        },
                        ViewMode::Active => {
                            self.show_active_animations(ui);
                        },
                        ViewMode::Settings => {
                            self.show_settings_panel(ui);
                        }
                    }
                });
        });
        
        // CRITICAL: Always request a repaint to keep update() being called
        // This ensures tray commands are processed even when window is hidden
        // We use a short delay (50ms) to keep the event loop active
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

