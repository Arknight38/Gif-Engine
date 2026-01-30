use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use crate::app::store::{Store, GifConfig};
use crate::app::process::ProcessStore;
use crate::types::{Frame, AnimationInfo};
use std::process::Command;
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};
use tray_icon::Icon;
use image::GenericImageView;
use auto_launch::AutoLaunch;

// Commands that the background tray thread can send to the GUI
// NOTE: On Windows, tray commands are handled directly in the tray thread using Windows APIs
// This enum is kept for non-Windows platforms or as a fallback
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used on non-Windows platforms
enum TrayCommand {
    ShowWindow,
    QuitApplication,
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let icon_data = load_icon_data();
    
    // Create channel for tray thread to send commands to GUI
    let (tray_cmd_tx, tray_cmd_rx) = mpsc::channel();
    
    // Setup tray icon and spawn background thread to poll for events
    let (tray_icon, tray_menu, quit_item, show_item) = setup_tray(tray_cmd_tx.clone());
    
    // Spawn background thread to poll for tray events and handle them DIRECTLY
    // This bypasses the need for update() to be called - we use Windows APIs directly
    let _tray_thread = {
        let quit_id = quit_item.as_ref().map(|q| q.id().clone());
        let show_id = show_item.as_ref().map(|s| s.id().clone());
        
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
                        
                        // Handle quit directly - no need to go through GUI
                        if let Some(quit_id_val) = &quit_id {
                            if menu_event.id == *quit_id_val {
                                #[cfg(debug_assertions)]
                                eprintln!("[DEBUG] Tray thread: Quit command - exiting directly");
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
        })
    };
    
    // No wake-up thread needed - tray thread handles commands directly using Windows APIs
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0]) // Increased size for split view
            .with_icon(Arc::new(icon_data)),
        // Keep the app running even when window is closed
        // This is important for tray icon functionality
        run_and_return: false,
        ..Default::default()
    };
    
    eframe::run_native(
        "Gif-Engine Manager",
        options,
        Box::new(move |_cc| Box::new(AnimeApp::new(tray_icon, tray_menu, quit_item, show_item, tray_cmd_rx))),
    )
}

struct PreviewState {
    frames: Vec<Frame>,
    info: AnimationInfo,
    current_frame: usize,
    last_update: std::time::Instant,
    texture: Option<egui::TextureHandle>,
    override_delay: Option<std::time::Duration>,
}

#[derive(PartialEq)]
enum ViewMode {
    Library,
    Settings,
}

struct AnimeApp {
    store: Arc<Mutex<Store>>,
    process_store: Arc<Mutex<ProcessStore>>,
    view: ViewMode,
    
    // Selection & Preview
    selected_name: Option<String>,
    preview: Option<PreviewState>,
    load_rx: Option<mpsc::Receiver<Result<(AnimationInfo, Vec<Frame>), String>>>,
    is_loading: bool,
    load_error: Option<String>, // Store error message for UI display
    
    // UI State
    refresh_timer: std::time::Instant,
    input_path: String,
    
    // Tray
    _tray_icon: Option<tray_icon::TrayIcon>,
    _tray_menu: Option<Box<Menu>>, // Keep menu alive for tray events to work
    #[allow(dead_code)] // Used on non-Windows platforms
    quit_item: Option<MenuItem>,
    #[allow(dead_code)] // Used on non-Windows platforms
    show_item: Option<MenuItem>,
    should_exit: bool,
    tray_cmd_rx: mpsc::Receiver<TrayCommand>, // Receive commands from background tray thread
    pending_close_canceled: bool, // Track if we've canceled a pending close request
    
    // Global Settings
    startup_enabled: bool,
}

impl AnimeApp {
    // Helper function to handle mutex poisoning gracefully
    fn lock_store(store: &Arc<Mutex<Store>>) -> std::sync::MutexGuard<'_, Store> {
        store.lock().unwrap_or_else(|e| {
            eprintln!("Warning: Store mutex poisoned, recovering");
            e.into_inner()
        })
    }
    
    fn lock_process_store(store: &Arc<Mutex<ProcessStore>>) -> std::sync::MutexGuard<'_, ProcessStore> {
        store.lock().unwrap_or_else(|e| {
            eprintln!("Warning: ProcessStore mutex poisoned, recovering");
            e.into_inner()
        })
    }
    
    fn new(tray_icon: Option<tray_icon::TrayIcon>, tray_menu: Option<Box<Menu>>, quit_item: Option<MenuItem>, show_item: Option<MenuItem>, tray_cmd_rx: mpsc::Receiver<TrayCommand>) -> Self {
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
        let (tray_icon, tray_menu, quit_item, show_item) = setup_tray(tray_cmd_tx);
        Self::new(tray_icon, tray_menu, quit_item, show_item, tray_cmd_rx)
    }
}

fn setup_tray(_tray_cmd_tx: mpsc::Sender<TrayCommand>) -> (Option<tray_icon::TrayIcon>, Option<Box<Menu>>, Option<MenuItem>, Option<MenuItem>) {
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

fn load_icon_data() -> egui::IconData {
    // Embed the icon at compile time
    let icon_bytes = include_bytes!("../icon.png");
    
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let (width, height) = img.dimensions();
        let rgba = img.into_rgba8().into_raw();
        return egui::IconData { rgba, width, height };
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
    egui::IconData { rgba, width, height }
}

fn get_auto_launch() -> Option<AutoLaunch> {
    let app_name = "Gif-Engine";
    let app_path = std::env::current_exe().ok()?;
    let path_str = app_path.to_str()?;
    Some(AutoLaunch::new(app_name, path_str, &[] as &[&str]))
}

impl eframe::App for AnimeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle commands from the background tray thread
        // The tray thread polls MenuEvent::receiver() and sends commands via channel
        // This works even when the window is hidden because the thread runs independently
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
                            eprintln!("[DEBUG] GUI: Quit requested from tray");
                            self.should_exit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            std::thread::spawn(|| {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                std::process::exit(0);
                            });
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    #[cfg(debug_assertions)]
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
                        self.preview = Some(PreviewState {
                            frames,
                            info,
                            current_frame: 0,
                            last_update: std::time::Instant::now(),
                            texture: None,
                            override_delay: None,
                        });
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
            if !preview.frames.is_empty() {
                let delay = preview.override_delay.unwrap_or(preview.frames[preview.current_frame].delay);

                if preview.last_update.elapsed() >= delay {
                    preview.current_frame = (preview.current_frame + 1) % preview.frames.len();
                    
                    // Accumulate time to prevent drift
                    preview.last_update += delay;
                    
                    // If we've drifted too far (e.g. system sleep or heavy lag), reset
                    if preview.last_update.elapsed() > delay {
                        preview.last_update = std::time::Instant::now();
                    }
                    
                    // Update texture
                    let frame = &preview.frames[preview.current_frame];
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [frame.width as usize, frame.height as usize],
                        &frame.buffer,
                    );
                    
                    if let Some(texture) = &mut preview.texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        preview.texture = Some(ctx.load_texture(
                            "preview_tex",
                            image,
                            egui::TextureOptions::LINEAR
                        ));
                    }
                    ctx.request_repaint(); // Ensure continuous animation
                } else {
                     // Request repaint for smooth timing even if we didn't update frame yet
                     // but use delay to avoid busy loop
                     let remaining = delay.saturating_sub(preview.last_update.elapsed());
                     ctx.request_repaint_after(remaining);
                }
            }
        }

        // SIDEBAR
        egui::SidePanel::left("library_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.show_library(ui);
                });
                
                // Bottom Settings Button
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui.selectable_label(self.view == ViewMode::Settings, "⚙ Settings").clicked() {
                        self.view = ViewMode::Settings;
                        self.selected_name = None; // Deselect animation when going to settings
                    }
                    ui.separator();
                });
            });

        // MAIN CONTENT
        egui::CentralPanel::default().show(ctx, |ui| {
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
                 ViewMode::Settings => {
                     self.show_settings_panel(ui);
                 }
             }
        });
        
        // CRITICAL: Always request a repaint to keep update() being called
        // This ensures tray commands are processed even when window is hidden
        // We use a short delay (50ms) to keep the event loop active
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

impl AnimeApp {
    fn show_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Settings");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                 if ui.button("✖ Close").clicked() {
                     self.view = ViewMode::Library;
                 }
            });
        });
        ui.separator();
        
        // General Settings
        ui.collapsing("General", |ui| {
            // Startup
            let mut enabled = self.startup_enabled;
             if ui.checkbox(&mut enabled, "Run on Startup").changed() {
                 if let Some(auto) = get_auto_launch() {
                    if enabled {
                        let _ = auto.enable();
                    } else {
                        let _ = auto.disable();
                    }
                    self.startup_enabled = enabled;
                }
             }
             
             // Theme (Basic Toggle for now, though Store supports it, we need to apply it)
             let mut store = Self::lock_store(&self.store);
             let mut theme = store.settings.theme.clone();
             let mut minimize = store.settings.minimize_to_tray;
             
             ui.horizontal(|ui| {
                 ui.label("Theme:");
                 if ui.selectable_value(&mut theme, "dark".to_string(), "Dark").clicked() 
                 || ui.selectable_value(&mut theme, "light".to_string(), "Light").clicked() {
                     store.settings.theme = theme;
                     let _ = store.save();
                 }
             });
             
             if ui.checkbox(&mut minimize, "Minimize to Tray on Close").changed() {
                 store.settings.minimize_to_tray = minimize;
                 let _ = store.save();
             }
        });
        
        ui.add_space(20.0);
        
        // Data Management
        ui.collapsing("Data Management", |ui| {
            if ui.button("Clean Dead Processes").clicked() {
                if let Ok(mut ps) = self.process_store.lock() {
                    ps.cleanup_dead_processes();
                }
            }
            
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Danger Zone").color(egui::Color32::RED));
            if ui.button("Reset Library (Deletes all entries)").clicked() {
                // Confirmation could be added here, but for now simple
                let mut store = Self::lock_store(&self.store);
                store.gifs.clear();
                let _ = store.save();
                self.selected_name = None;
                self.preview = None;
            }
        });

        ui.add_space(20.0);
        
        // About
        ui.collapsing("About", |ui| {
            ui.label("Desktop Anime Manager v0.1.0");
            ui.label("A lightweight engine for playing GIFs/WebPs on your desktop.");
            ui.add_space(10.0);
            ui.hyperlink("https://github.com/your-repo/gif-engine");
        });
    }

    fn show_selected_animation(&mut self, ui: &mut egui::Ui, name: String) {
        let mut store = Self::lock_store(&self.store);
        let mut to_launch = None;
        let mut to_delete = None;
        let mut should_save = false;

        if let Some(config) = store.gifs.get_mut(&name) {
            // Update preview override
            if let Some(preview) = &mut self.preview {
                preview.override_delay = config.fps.map(|f| std::time::Duration::from_secs_f64(1.0 / f as f64));
            }
            
            ui.heading(&name);
            ui.label(egui::RichText::new(format!("Path: {:?}", config.path)).italics().weak());
            ui.separator();

            // Preview Area
            ui.vertical_centered(|ui| {
                if self.is_loading {
                    ui.spinner();
                    ui.label("Loading preview...");
                } else if let Some(error) = &self.load_error {
                    // Display error message in UI
                    ui.label(egui::RichText::new("❌ Failed to load preview").color(egui::Color32::RED));
                    ui.label(egui::RichText::new(error).small().color(egui::Color32::GRAY));
                    ui.add_space(10.0);
                    if ui.button("🔄 Retry").clicked() {
                        // Retry loading
                        self.load_error = None;
                        if let Some(config) = Self::lock_store(&self.store).gifs.get(&name) {
                            let path = config.path.clone();
                            let (tx, rx) = mpsc::channel();
                            self.load_rx = Some(rx);
                            self.is_loading = true;
                            self.preview = None;
                            
                            thread::spawn(move || {
                                let res = crate::decoder::load_animation(path);
                                let _ = tx.send(res.map_err(|e| e.to_string()));
                            });
                        }
                    }
                } else if let Some(preview) = &self.preview {
                    if let Some(texture) = &preview.texture {
                         ui.image((texture.id(), texture.size_vec2()));
                    }
                    ui.label(format!("Original: {}x{} @ {:.2} FPS", 
                        preview.info.width, 
                        preview.info.height,
                        preview.info.frame_count as f64 / preview.info.duration.as_secs_f64()
                    ));
                } else {
                    ui.label("Preview unavailable");
                }
            });
            ui.separator();

            // Controls
            ui.horizontal(|ui| {
                let is_running = {
                    let ps = Self::lock_process_store(&self.process_store);
                    ps.processes.values().any(|info| info.name == name)
                };

                if is_running {
                    if ui.button("🔄 Restart").clicked() {
                        // Kill existing
                         let pid_opt = {
                            let ps = Self::lock_process_store(&self.process_store);
                            ps.processes.iter().find(|(_, info)| info.name == name).map(|(pid, _)| *pid)
                        };
                        if let Some(pid) = pid_opt {
                            if let Ok(mut ps) = self.process_store.lock() { ps.kill_process(pid); }
                        }
                        to_launch = Some(name.clone());
                    }
                    if ui.button("⏹ Stop").clicked() {
                        let pid_opt = {
                            let ps = Self::lock_process_store(&self.process_store);
                            ps.processes.iter().find(|(_, info)| info.name == name).map(|(pid, _)| *pid)
                        };
                        if let Some(pid) = pid_opt {
                            if let Ok(mut ps) = self.process_store.lock() { ps.kill_process(pid); }
                        }
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        to_launch = Some(name.clone());
                    }
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑 Delete").clicked() {
                        to_delete = Some(name.clone());
                    }
                });
            });
            ui.separator();

            // Settings
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([40.0, 10.0])
                .striped(true)
                .show(ui, |ui| {
                    // Overlay
                    ui.label("Overlay:");
                    if ui.checkbox(&mut config.overlay, "Always on top").changed() {
                        should_save = true;
                    }
                    ui.end_row();

                    // FPS
                    let original_fps = if let Some(p) = &self.preview {
                         (p.info.frame_count as f64 / p.info.duration.as_secs_f64()) as u32
                    } else { 60 };
                    let max_fps = if original_fps == 0 { 60 } else { original_fps * 2 };
                    
                    let mut fps_val = config.fps.unwrap_or(original_fps);
                    
                    ui.label("Target FPS:");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut fps_val).speed(1).clamp_range(0..=max_fps).suffix(" fps")).changed() {
                            config.fps = if fps_val > 0 { Some(fps_val) } else { None };
                            should_save = true;
                        }
                        if ui.add(egui::Slider::new(&mut fps_val, 0..=max_fps).show_value(false)).changed() {
                             config.fps = if fps_val > 0 { Some(fps_val) } else { None };
                             should_save = true;
                        }
                    });
                    ui.end_row();

                    // Scale
                    let mut scale_val = config.scale.unwrap_or(1.0);
                    ui.label("Scale:");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut scale_val).speed(0.01).clamp_range(0.1..=2.0)).changed() {
                            config.scale = if (scale_val - 1.0).abs() > f32::EPSILON { Some(scale_val) } else { None };
                            should_save = true;
                        }
                        if ui.add(egui::Slider::new(&mut scale_val, 0.1..=2.0).show_value(false)).changed() {
                             config.scale = if (scale_val - 1.0).abs() > f32::EPSILON { Some(scale_val) } else { None };
                             should_save = true;
                        }
                    });
                    ui.end_row();

                    // Alignment
                    ui.label("Alignment:");
                    let mut changed_align = false;
                    egui::ComboBox::from_id_source("align_combo")
                        .selected_text(&config.align)
                        .show_ui(ui, |ui| {
                            changed_align |= ui.selectable_value(&mut config.align, "top-left".to_string(), "Top-Left").clicked();
                            changed_align |= ui.selectable_value(&mut config.align, "top-right".to_string(), "Top-Right").clicked();
                            changed_align |= ui.selectable_value(&mut config.align, "bottom-left".to_string(), "Bottom-Left").clicked();
                            changed_align |= ui.selectable_value(&mut config.align, "bottom-right".to_string(), "Bottom-Right").clicked();
                            changed_align |= ui.selectable_value(&mut config.align, "center".to_string(), "Center").clicked();
                            changed_align |= ui.selectable_value(&mut config.align, "custom".to_string(), "Custom").clicked();
                        });
                    if changed_align { should_save = true; }
                    ui.end_row();

                    // Position
                    if config.align == "custom" {
                        ui.label("Position:");
                        let mut use_pos = config.position.is_some();
                        if ui.checkbox(&mut use_pos, "Custom Coordinates").changed() {
                            if use_pos {
                                config.position = Some((100, 100));
                            } else {
                                config.position = None;
                            }
                            should_save = true;
                        }
                        ui.end_row();

                        if let Some((x, y)) = &mut config.position {
                            ui.label("");
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                if ui.add(egui::DragValue::new(x)).changed() { should_save = true; }
                                ui.add_space(10.0);
                                ui.label("Y:");
                                if ui.add(egui::DragValue::new(y)).changed() { should_save = true; }
                            });
                            ui.end_row();
                        }
                    } else {
                        ui.label("Position:");
                        ui.label(egui::RichText::new("Controlled by Alignment").weak());
                        ui.end_row();
                    }
                });

        }

        if should_save {
            let _ = store.save();
        }
        
        if let Some(n) = to_delete {
            if self.selected_name.as_ref() == Some(&n) {
                self.selected_name = None;
                self.preview = None;
            }
             store.gifs.remove(&n);
             let _ = store.save();
        }

        let launch_config = if let Some(n) = to_launch {
            store.gifs.get(&n).cloned()
        } else {
            None
        };
        
        drop(store);
        
        if let Some(c) = launch_config {
            self.launch_animation(&c);
        }
    }

    fn launch_animation(&mut self, config: &GifConfig) {
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                eprintln!("Error: Failed to get executable path: {}", e);
                return;
            }
        };
        let mut cmd = Command::new(exe);
        cmd.arg("play").arg(&config.path);
           
        if let Some(fps) = config.fps {
            cmd.arg("--fps").arg(fps.to_string());
        }
        if let Some(scale) = config.scale {
            cmd.arg("--scale").arg(scale.to_string());
        }
        if let Some((x, y)) = config.position {
            cmd.arg("--x").arg(x.to_string());
            cmd.arg("--y").arg(y.to_string());
        }
        
        cmd.arg("--align").arg(&config.align);
        cmd.arg("--monitor").arg(config.monitor.to_string());

        if config.overlay {
            cmd.arg("--overlay");
        }

        match cmd.spawn() {
            Ok(child) => {
                let mut ps = Self::lock_process_store(&self.process_store);
                ps.add_process(child.id(), config.name.clone());
            },
            Err(e) => eprintln!("Failed to start: {}", e),
        }
    }
    
    fn show_library(&mut self, ui: &mut egui::Ui) {
        ui.heading("Library");
        ui.add_space(10.0);
        
        // Add Section
        ui.horizontal(|ui| {
            if ui.button("📂").on_hover_text("Browse file").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.input_path = path.to_string_lossy().to_string();
                }
            }

            if ui.button("📁+").on_hover_text("Scan Folder").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    if let Ok(entries) = std::fs::read_dir(path) {
                        let mut store = Self::lock_store(&self.store);
                        let mut count = 0;
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                     let ext = ext.to_lowercase();
                                     if ext == "gif" || ext == "apng" {
                                         let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                         if !name.is_empty() && !store.gifs.contains_key(&name) {
                                             if store.add_gif(name, path).is_ok() {
                                                 count += 1;
                                             }
                                         }
                                     }
                                }
                            }
                        }
                        if count > 0 {
                            let _ = store.save();
                        }
                    }
                }
            }

            let re = ui.add(egui::TextEdit::singleline(&mut self.input_path).hint_text("Path..."));
            if ui.button("Add").clicked() || (re.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                 let mut store = Self::lock_store(&self.store);
                 let trimmed = self.input_path.trim();
                 if !trimmed.is_empty() {
                     let path = std::path::PathBuf::from(trimmed);
                     if path.exists() {
                         let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                         if !name.is_empty() {
                             if store.add_gif(name, path).is_ok() {
                                 let _ = store.save();
                                 self.input_path.clear();
                             }
                         }
                     }
                 }
            }
        });

        ui.separator();

        let store = Self::lock_store(&self.store);
        let mut keys: Vec<String> = store.gifs.keys().cloned().collect();
        keys.sort();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 5.0);
            for name in keys {
                ui.push_id(&name, |ui| {
                    let is_selected = self.selected_name.as_ref() == Some(&name);
                    
                    // Check running status
                    let is_running = {
                         let ps = Self::lock_process_store(&self.process_store);
                         ps.processes.values().any(|info| info.name == name)
                    };

                    let label = if is_running {
                        format!("▶ {}", name)
                    } else {
                        name.clone()
                    };

                    if ui.selectable_label(is_selected, label).clicked() {
                        self.view = ViewMode::Library; // Ensure we switch back to library view
                        if !is_selected {
                            self.selected_name = Some(name.clone());
                            // Trigger load
                            if let Some(config) = store.gifs.get(&name) {
                                let path = config.path.clone();
                                let (tx, rx) = mpsc::channel();
                                self.load_rx = Some(rx);
                                self.is_loading = true;
                                self.preview = None;
                                
                                thread::spawn(move || {
                                    let res = crate::decoder::load_animation(path);
                                    let _ = tx.send(res.map_err(|e| e.to_string()));
                                });
                            }
                        }
                    }
                });
            }
        });
    }
}
