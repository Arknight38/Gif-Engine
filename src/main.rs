#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod decoder;
mod renderer;
mod playback;
mod platform;
mod app;
mod gui;
pub mod types;

fn main() {
    // Always run the GUI; CLI/TUI entry points have been removed
    if let Err(e) = gui::run_gui() {
        eprintln!("GUI Error: {}", e);
    }
}
