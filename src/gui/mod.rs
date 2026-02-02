mod tray;
mod preview;
mod app;
mod views;
mod hotkeys;

pub use app::AnimeApp;
pub use tray::TrayCommand;

pub fn run_gui() -> Result<(), eframe::Error> {
    use std::sync::Arc;
    use std::sync::mpsc;
    use tray::load_icon_data;
    use tray::setup_tray;
    use tray::spawn_tray_thread;
    use app::AnimeApp;

    let icon_data = load_icon_data();
    
    // Create channel for tray thread to send commands to GUI
    let (tray_cmd_tx, tray_cmd_rx) = mpsc::channel();
    
    // Setup tray icon and spawn background thread to poll for events
    let (tray_icon, tray_menu, quit_item, show_item) = setup_tray(tray_cmd_tx.clone());
    
    // Spawn background thread to poll for tray events and handle them DIRECTLY
    // This bypasses the need for update() to be called - we use Windows APIs directly
    let quit_id = quit_item.as_ref().map(|q| q.id().clone());
    let show_id = show_item.as_ref().map(|s| s.id().clone());
    spawn_tray_thread(quit_id, show_id, tray_cmd_tx);
    
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
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
