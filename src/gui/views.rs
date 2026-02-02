use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::process::Command;
use crate::app::store::{Store, GifConfig};
use crate::app::process::ProcessStore;
use crate::gui::app::{AnimeApp, ViewMode};
use crate::gui::preview::PreviewState;

impl AnimeApp {
    pub fn show_settings_panel(&mut self, ui: &mut egui::Ui) {
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
        ui.vertical(|ui| {
            // Startup
            let mut enabled = self.startup_enabled;
             if ui.checkbox(&mut enabled, "Run on Startup").changed() {
                 if let Some(auto) = crate::gui::tray::get_auto_launch() {
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
             
             // Click-through setting
             let mut click_through = store.settings.click_through;
             ui.add_space(10.0);
             if ui.checkbox(&mut click_through, "Enable Click-Through for Animations").changed() {
                 store.settings.click_through = click_through;
                 let _ = store.save();
             }
             ui.label(egui::RichText::new("When enabled, animations won't block mouse clicks. Hold Ctrl and drag to move them.").small().weak());
        });
        
        ui.add_space(20.0);
        
        // Data Management
        ui.vertical(|ui| {
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
        ui.vertical(|ui| {
            ui.label(format!("Desktop Anime Manager v{}", env!("CARGO_PKG_VERSION")));
            ui.label("A lightweight engine for playing GIFs/WebPs on your desktop.");
            ui.add_space(10.0);
            ui.hyperlink("https://github.com/Arknight38/Gif-Engine");
        });
    }

    pub fn show_selected_animation(&mut self, ui: &mut egui::Ui, name: String) {
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

    pub fn launch_animation(&mut self, config: &GifConfig) {
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
        
        // Get click-through setting from store
        let click_through = {
            let store = Self::lock_store(&self.store);
            store.settings.click_through
        };
        if click_through {
            cmd.arg("--click-through");
        }

        match cmd.spawn() {
            Ok(child) => {
                let mut ps = Self::lock_process_store(&self.process_store);
                ps.add_process(child.id(), config.name.clone());
            },
            Err(e) => eprintln!("Failed to start: {}", e),
        }
    }
    
    pub fn show_library(&mut self, ui: &mut egui::Ui) {
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

