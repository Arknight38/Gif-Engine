use eframe::egui;
use std::sync::mpsc;
use std::thread;
use std::process::Command;
use crate::app::store::GifConfig;
use crate::gui::app::{AnimeApp, ViewMode};
use crate::gui::hotkeys::is_key_valid;
use crate::gui::community::{fetch_manifest_async, install_pack_async};

impl AnimeApp {
    pub fn show_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Settings");
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

             ui.add_space(15.0);
             ui.label(egui::RichText::new("Global Hotkeys (Windows)").size(16.0));
             ui.label(egui::RichText::new("Supported keys: A-Z, 0-9, F1-F24. Hotkeys require exact modifier match.").small().weak());

             let mut hotkeys_changed = false;

             // Toggle Manager Window hotkey
             {
                 let hk = &mut store.settings.hotkeys.toggle_manager;
                 ui.group(|ui| {
                     ui.horizontal(|ui| {
                         hotkeys_changed |= ui.checkbox(&mut hk.enabled, "Enable").changed();
                         ui.label("Toggle Manager");
                     });
                     ui.horizontal(|ui| {
                         ui.label("Modifiers:");
                         hotkeys_changed |= ui.checkbox(&mut hk.ctrl, "Ctrl").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.alt, "Alt").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.shift, "Shift").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.win, "Win").changed();
                     });
                     ui.horizontal(|ui| {
                         ui.label("Key:");
                         hotkeys_changed |= ui.text_edit_singleline(&mut hk.key).changed();
                         if !hk.key.trim().is_empty() && !is_key_valid(&hk.key) {
                             ui.label(egui::RichText::new("Invalid key").color(egui::Color32::RED));
                         }
                     });
                 });
             }

             // Stop All Animations hotkey
             {
                 let hk = &mut store.settings.hotkeys.stop_all;
                 ui.group(|ui| {
                     ui.horizontal(|ui| {
                         hotkeys_changed |= ui.checkbox(&mut hk.enabled, "Enable").changed();
                         ui.label("Stop All Animations");
                     });
                     ui.horizontal(|ui| {
                         ui.label("Modifiers:");
                         hotkeys_changed |= ui.checkbox(&mut hk.ctrl, "Ctrl").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.alt, "Alt").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.shift, "Shift").changed();
                         hotkeys_changed |= ui.checkbox(&mut hk.win, "Win").changed();
                     });
                     ui.horizontal(|ui| {
                         ui.label("Key:");
                         hotkeys_changed |= ui.text_edit_singleline(&mut hk.key).changed();
                         if !hk.key.trim().is_empty() && !is_key_valid(&hk.key) {
                             ui.label(egui::RichText::new("Invalid key").color(egui::Color32::RED));
                         }
                     });
                 });
             }

             if hotkeys_changed {
                 let _ = store.save();
             }
        });
        
        ui.add_space(20.0);
        
        // Data Management
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Export / Import").size(16.0));
            ui.horizontal(|ui| {
                if ui.button("📤 Export Library & Settings").clicked() {
                    let store = Self::lock_store(&self.store);
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("ZIP Archive", &["zip"])
                        .set_file_name("gif-engine-backup.zip")
                        .save_file() {
                        if let Err(e) = store.export_zip(&path) {
                            eprintln!("Failed to export: {}", e);
                        }
                    }
                }
                
                if ui.button("📥 Import Library & Settings").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("ZIP Archive", &["zip"])
                        .pick_file() {
                        let mut store = Self::lock_store(&self.store);
                        // Merge by default (adds/updates animations, updates settings)
                        if let Err(e) = store.import_zip(&path, true) {
                            eprintln!("Failed to import: {}", e);
                        } else {
                            let _ = store.save();
                            // Refresh UI
                            self.selected_name = None;
                            self.preview = None;
                        }
                    }
                }
            });
            ui.label(egui::RichText::new("Export creates a ZIP file with all animations and settings. Import merges with existing data.").small().weak());
            
            ui.add_space(10.0);
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

    pub fn show_community_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Community Packs");
        ui.label(egui::RichText::new("Source: https://arknight38.github.io/Gif-Engine-Library/").small().weak());
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() && !self.community_loading {
                let (tx, rx) = mpsc::channel();
                self.community_rx = Some(rx);
                self.community_loading = true;
                self.community_error = None;
                self.community_packs.clear();
                fetch_manifest_async(tx);
            }
            if ui.button("🌐 Open in Browser").clicked() {
                let _ = webbrowser::open("https://arknight38.github.io/Gif-Engine-Library/");
            }
        });
        ui.add_space(10.0);

        if self.community_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading packs from GitHub...");
            });
        } else if let Some(err) = &self.community_error {
            ui.colored_label(egui::Color32::RED, format!("Failed to load packs: {}", err));
        } else if self.community_packs.is_empty() {
            ui.label(egui::RichText::new("No packs loaded. Click \"Refresh\" to fetch from GitHub.").small().weak());
        } else {
            let available_height = ui.available_height();
            egui::ScrollArea::vertical()
                .max_height(available_height)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for pack in &self.community_packs {
                            // Card Setup
                            let card_width = 240.0;
                            let card_height = 350.0;
                            
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(card_width, card_height), 
                                egui::Sense::hover()
                            );
                            
                            // Draw Card Background
                            ui.painter().rect(
                                rect, 
                                8.0, 
                                ui.visuals().widgets.noninteractive.bg_fill, 
                                egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
                            );
                            
                            let content_rect = rect.shrink(10.0);
                            let mut content_ui = ui.child_ui(content_rect, *ui.layout());
                            
                            content_ui.vertical(|ui| {
                                // Thumbnail
                                let thumb_id = pack.preview_url.clone();
                                let thumb_height = 140.0;
                                
                                ui.allocate_ui(egui::vec2(ui.available_width(), thumb_height), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        if !thumb_id.is_empty() {
                                            if let Some(texture) = self.thumbnail_cache.get(&thumb_id) {
                                                 ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(ui.available_width(), thumb_height)));
                                            } else {
                                                 use crate::gui::app::ThumbnailState;
                                                 if !self.thumbnail_states.contains_key(&thumb_id) {
                                                     let tx = self.thumbnail_tx.clone();
                                                     let url = thumb_id.clone();
                                                     self.thumbnail_states.insert(thumb_id.clone(), ThumbnailState::Loading);
                                                     
                                                     std::thread::spawn(move || {
                                                         match reqwest::blocking::get(&url) {
                                                             Ok(resp) => {
                                                                 if let Ok(bytes) = resp.bytes() {
                                                                     if let Ok(img) = image::load_from_memory(&bytes) {
                                                                         let resized = img.resize(300, 300, image::imageops::FilterType::Lanczos3);
                                                                         let w = resized.width() as usize;
                                                                         let h = resized.height() as usize;
                                                                         let rgba = resized.to_rgba8().into_raw();
                                                                         let _ = tx.send((url, Some((w, h, rgba))));
                                                                         return;
                                                                     }
                                                                 }
                                                             }
                                                             Err(_) => {}
                                                         }
                                                         let _ = tx.send((url, None));
                                                     });
                                                 }
                                                 ui.spinner();
                                            }
                                        } else {
                                            ui.label("No Preview");
                                        }
                                    });
                                });

                                ui.add_space(5.0);

                                // Header
                                ui.horizontal(|ui| {
                                    ui.heading(egui::RichText::new(&pack.name).size(16.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("v{}", pack.version)).weak().small());
                                    });
                                });
                                
                                ui.label(egui::RichText::new(format!("by {}", pack.author)).small().weak());
                                ui.add_space(2.0);
                                
                                // Description (Truncated)
                                ui.label(egui::RichText::new(&pack.description).small()).on_hover_text(&pack.description);
                                
                                ui.add_space(5.0);

                                // Tags
                                if !pack.tags.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for tag in pack.tags.iter().take(3) {
                                            ui.label(egui::RichText::new(format!("🏷 {}", tag)).small().weak());
                                        }
                                        if pack.tags.len() > 3 {
                                            ui.label(egui::RichText::new("...").small().weak());
                                        }
                                    });
                                }

                                // Buttons at bottom
                                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("⬇ Install").clicked() {
                                            install_pack_async(pack.clone(), self.store.clone());
                                        }
                                        if ui.button("🔗 Json").clicked() {
                                            let _ = webbrowser::open(&pack.pack_url);
                                        }
                                    });
                                    ui.add_space(5.0);
                                });
                            });
                        }
                    });
                });
        }
    }

    pub fn show_selected_animation(&mut self, ui: &mut egui::Ui, name: String) {
        let mut store = Self::lock_store(&self.store);
        let mut to_launch = None;
        let mut to_delete = None;
        let mut should_save = false;
        let mut export_config: Option<GifConfig> = None;

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
                        // Constrain image to fit available width without expanding window
                        let available_width = ui.available_width();
                        let texture_size = texture.size_vec2();
                        let aspect_ratio = texture_size.y / texture_size.x;
                        
                        // Calculate size that fits within available width
                        let max_width = available_width;
                        let display_width = max_width.min(texture_size.x);
                        let display_height = display_width * aspect_ratio;
                        
                        // Show image with constrained size - use fit_to_exact_size to ensure it doesn't expand
                        ui.add(egui::Image::new(texture)
                            .fit_to_exact_size(egui::vec2(display_width, display_height)));
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
            let is_running = {
                let ps = Self::lock_process_store(&self.process_store);
                ps.processes.values().any(|info| info.name == name)
            };

            ui.horizontal(|ui| {
                if is_running {
                    if ui.button("🔄 Restart").clicked() {
                        // Kill existing
                        let pid_opt = {
                            let ps = Self::lock_process_store(&self.process_store);
                            ps.processes.iter().find(|(_, info)| info.name == name).map(|(pid, _)| *pid)
                        };
                        if let Some(pid) = pid_opt {
                            if let Ok(mut ps) = self.process_store.lock() { 
                                ps.kill_process(pid); 
                            }
                        }
                        to_launch = Some(name.clone());
                    }
                    if ui.button("⏹ Stop").clicked() {
                        let pid_opt = {
                            let ps = Self::lock_process_store(&self.process_store);
                            ps.processes.iter().find(|(_, info)| info.name == name).map(|(pid, _)| *pid)
                        };
                        if let Some(pid) = pid_opt {
                            if let Ok(mut ps) = self.process_store.lock() { 
                                ps.kill_process(pid); 
                            }
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
                    if ui.button("📦 Export Pack").clicked() {
                        export_config = Some(config.clone());
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

                    // Anchor
                    ui.label("Anchor Window:");
                    ui.vertical(|ui| {
                        let mut anchor_text = config.anchor.clone().unwrap_or_default();
                        
                        ui.horizontal(|ui| {
                            if ui.text_edit_singleline(&mut anchor_text).on_hover_text("Enter window title (partial match supported)").changed() {
                                config.anchor = if anchor_text.trim().is_empty() { None } else { Some(anchor_text.trim().to_string()) };
                                should_save = true;
                            }
                            
                            // Window Picker Popup
                            ui.menu_button("📍 Pick", |ui| {
                                ui.set_min_width(200.0);
                                if ui.button("🔄 Refresh List").clicked() {
                                    self.open_windows = crate::platform::windows::get_open_window_titles();
                                }
                                ui.separator();
                                
                                if self.open_windows.is_empty() {
                                    ui.label("Click refresh to see windows");
                                    // Auto-refresh if empty
                                    if ui.ui_contains_pointer() { // Poor man's on-open check, but safe enough
                                         // Don't spam refresh, just let user click. 
                                         // Or we can just call it once here if we want:
                                         // self.open_windows = crate::platform::windows::get_open_window_titles();
                                    }
                                }
                                
                                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                    for win_title in &self.open_windows {
                                        if ui.button(win_title).clicked() {
                                            config.anchor = Some(win_title.clone());
                                            should_save = true;
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                        });
                        
                        if let Some(anchor) = &config.anchor {
                             ui.label(egui::RichText::new(format!("Anchored to: \"{}\"", anchor)).small().weak());
                        }
                    });
                    ui.end_row();

                    // Behavior
                    ui.label("Behavior:");
                    egui::ComboBox::from_id_source("behavior_combo")
                        .selected_text(match config.behavior {
                            crate::app::store::Behavior::None => "None",
                            crate::app::store::Behavior::Bounce => "Bounce",
                            crate::app::store::Behavior::Wander => "Wander",
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut config.behavior, crate::app::store::Behavior::None, "None").clicked() { should_save = true; }
                            if ui.selectable_value(&mut config.behavior, crate::app::store::Behavior::Bounce, "Bounce").clicked() { should_save = true; }
                            if ui.selectable_value(&mut config.behavior, crate::app::store::Behavior::Wander, "Wander").clicked() { should_save = true; }
                        });
                    ui.end_row();

                    // FPS
                    let original_fps = if let Some(p) = &self.preview {
                        let duration = p.info.duration.as_secs_f64();
                        if duration > 0.0 {
                            (p.info.frame_count as f64 / duration) as u32
                        } else {
                            0
                        }
                    } else { 60 };
                    let max_fps = if original_fps == 0 { 60 } else { original_fps.saturating_mul(2) };
                    
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
                    
                    // Tags
                    ui.label("Tags:");
                    ui.horizontal_wrapped(|ui| {
                        let mut tags_to_remove = Vec::new();
                        for (idx, tag) in config.tags.iter().enumerate() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(tag);
                                    if ui.small_button("✖").clicked() {
                                        tags_to_remove.push(idx);
                                        should_save = true;
                                    }
                                });
                            });
                        }
                        // Remove tags in reverse order to maintain indices
                        for idx in tags_to_remove.iter().rev() {
                            config.tags.remove(*idx);
                        }
                        
                        // Add new tag input
                        let response = ui.text_edit_singleline(&mut self.new_tag_input);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let trimmed = self.new_tag_input.trim().to_string();
                            if !trimmed.is_empty() && !config.tags.contains(&trimmed) {
                                config.tags.push(trimmed.clone());
                                should_save = true;
                                self.new_tag_input.clear();
                            }
                        }
                        if ui.button("+").clicked() {
                            let trimmed = self.new_tag_input.trim().to_string();
                            if !trimmed.is_empty() && !config.tags.contains(&trimmed) {
                                config.tags.push(trimmed.clone());
                                should_save = true;
                                self.new_tag_input.clear();
                            }
                        }
                    });
                    ui.end_row();
                });

        }

        let deleted_name = if let Some(ref n) = to_delete {
            // Kill running instance of this animation
            let pid_opt = {
                let ps = Self::lock_process_store(&self.process_store);
                ps.processes.iter()
                    .find(|(_, info)| info.name == *n)
                    .map(|(pid, _)| *pid)
            };
            if let Some(pid) = pid_opt {
                if let Ok(mut ps) = self.process_store.lock() {
                    let _ = ps.kill_process(pid);
                }
            }
            
            if self.selected_name.as_ref() == Some(n) {
                self.selected_name = None;
                self.preview = None;
            }
             store.gifs.remove(n);
            Some(n.clone())
        } else {
            None
        };

        if should_save || deleted_name.is_some() {
            let _ = store.save();
        }

        let launch_config = if let Some(n) = to_launch {
            store.gifs.get(&n).cloned()
        } else {
            None
        };
        
        drop(store);

        if let Some(cfg) = export_config {
            self.export_pack(&cfg);
        }

        if let Some(c) = launch_config {
            self.launch_animation(&c);
        }
    }

    pub fn launch_animation(&mut self, config: &GifConfig) {
        // Check if animation is already running
        let is_running = {
            let ps = Self::lock_process_store(&self.process_store);
            ps.processes.values().any(|info| info.name == config.name)
        };
        
        if is_running {
            // Animation is already running, don't launch another instance
            return;
        }
        
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

        if let Some(anchor) = &config.anchor {
            if !anchor.is_empty() {
                cmd.arg("--anchor").arg(anchor);
            }
        }

        match config.behavior {
            crate::app::store::Behavior::Bounce => { cmd.arg("--behavior").arg("bounce"); },
            crate::app::store::Behavior::Wander => { cmd.arg("--behavior").arg("wander"); },
            _ => {}
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
        
        // Search & Add Bar
        ui.horizontal_wrapped(|ui| {
            // Search
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Search...").desired_width(200.0));
            if !self.search_query.is_empty() {
                if ui.button("✖").clicked() {
                    self.search_query.clear();
                }
            }
            
            ui.add_space(20.0);
            
            if ui.button("🎲 Random").clicked() {
                 let store = Self::lock_store(&self.store);
                 let keys: Vec<String> = store.gifs.keys().cloned().collect();
                 if !keys.is_empty() {
                     use rand::seq::SliceRandom;
                     let mut rng = rand::thread_rng();
                     if let Some(picked) = keys.choose(&mut rng) {
                         drop(store); // Unlock before calling show_selected_animation logic
                         
                         self.view = ViewMode::Library;
                         self.selected_name = Some(picked.clone());
                         self.new_tag_input.clear();
                         
                         // Trigger load
                         if let Some(config) = Self::lock_store(&self.store).gifs.get(picked) {
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
            }
            
            if ui.button("📂 Add File").on_hover_text("Browse file").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.input_path = path.to_string_lossy().to_string();
                    // Auto-add if picked
                     let mut store = Self::lock_store(&self.store);
                     let path_buf = std::path::PathBuf::from(&self.input_path);
                     let name = path_buf.file_stem().unwrap_or_default().to_string_lossy().to_string();
                     if !name.is_empty() && !store.gifs.contains_key(&name) {
                         if store.add_gif(name, path_buf).is_ok() {
                             let _ = store.save();
                             self.input_path.clear();
                         }
                     }
                }
            }

            if ui.button("📁 Scan Folder").on_hover_text("Scan Folder for GIFs").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    if let Ok(entries) = std::fs::read_dir(path) {
                        let mut store = Self::lock_store(&self.store);
                        let mut count = 0;
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                     let ext = ext.to_lowercase();
                                     match ext.as_str() {
                                         "gif" | "apng" | "png" | "webp" | "jpg" | "jpeg" | "bmp" | "ico" | "tiff" => {
                                             let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                             if !name.is_empty() && !store.gifs.contains_key(&name) {
                                                 if store.add_gif(name, path).is_ok() {
                                                     count += 1;
                                                 }
                                             }
                                         },
                                         _ => {}
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
        });

        ui.separator();

        let store = Self::lock_store(&self.store);
        let mut keys: Vec<String> = store.gifs.keys().cloned().collect();
        keys.sort();
        
        // Filter by search query (name or tags)
        let search_lower = self.search_query.to_lowercase();
        let filtered_keys: Vec<String> = if search_lower.is_empty() {
            keys
        } else {
            keys.into_iter().filter(|name| {
                // Match name
                if name.to_lowercase().contains(&search_lower) {
                    return true;
                }
                // Match tags
                if let Some(config) = store.gifs.get(name) {
                    for tag in &config.tags {
                        if tag.to_lowercase().contains(&search_lower) {
                            return true;
                        }
                    }
                }
                false
            }).collect()
        };

        if filtered_keys.is_empty() {
            ui.centered_and_justified(|ui| {
                if self.search_query.is_empty() {
                    ui.label("Library is empty. Add some GIFs!");
                } else {
                    ui.label("No matches found.");
                }
            });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Use a wrapped horizontal layout to create a grid of cards
                    ui.horizontal_wrapped(|ui| {
                        for name in filtered_keys {
                            let is_selected = self.selected_name.as_ref() == Some(&name);
                            
                            // Card Styling
                            let card_width = 200.0;
                            let card_height = 240.0;
                            
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(card_width, card_height), 
                                egui::Sense::click()
                            );

                            // Handle selection click
                            if response.clicked() {
                                self.view = ViewMode::Library;
                                if !is_selected {
                                    self.selected_name = Some(name.clone());
                                    self.new_tag_input.clear();
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

                            // Custom Card Rendering
                            let visuals = ui.style().interact(&response);
                            
                            // Determine background color
                            let bg_color = if is_selected {
                                ui.visuals().selection.bg_fill
                            } else if response.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().widgets.noninteractive.bg_fill
                            };

                            // Draw rounded rect
                            ui.painter().rect(
                                rect, 
                                8.0, // increased rounding
                                bg_color, 
                                if is_selected { ui.visuals().selection.stroke } else { egui::Stroke::NONE }
                            );
                            
                            // Draw Content inside the card
                            let content_rect = rect.shrink(10.0);
                            let mut content_ui = ui.child_ui(content_rect, *ui.layout());
                            
                            content_ui.vertical_centered(|ui| {
                                // Thumbnail Logic
                                let thumb_id = name.clone();
                                if let Some(texture) = self.thumbnail_cache.get(&thumb_id) {
                                     ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(ui.available_width(), 150.0)));
                                } else {
                                     use crate::gui::app::ThumbnailState;
                                     // Check state
                                     let _state = self.thumbnail_states.get(&thumb_id).copied().unwrap_or(ThumbnailState::Loading);
                                     
                                     if !self.thumbnail_states.contains_key(&thumb_id) {
                                         // Trigger load
                                         if let Some(config) = store.gifs.get(&name) {
                                             let path = config.path.clone();
                                             let tx = self.thumbnail_tx.clone();
                                             let id = thumb_id.clone();
                                             self.thumbnail_states.insert(id.clone(), ThumbnailState::Loading);
                                             
                                             std::thread::spawn(move || {
                                                  match crate::decoder::load_thumbnail(&path) {
                                                      Ok((w, h, rgba)) => { let _ = tx.send((id, Some((w, h, rgba)))); },
                                                      Err(e) => { 
                                                          eprintln!("Failed to load thumbnail for {}: {}", path.display(), e);
                                                          let _ = tx.send((id, None)); 
                                                      }
                                                  }
                                             });
                                         }
                                     }
                                     
                                     // Placeholder
                                     let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::hover());
                                     ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(50));
                                     ui.allocate_ui_at_rect(rect, |ui| {
                                         ui.centered_and_justified(|ui| {
                                             ui.label(egui::RichText::new("📷 No Preview").weak());
                                         });
                                     });
                                }
                                
                                ui.add_space(5.0);
                                ui.add(egui::Label::new(egui::RichText::new(&name).strong().size(16.0)).truncate(true));
                                
                                // Check running status
                                let is_running = {
                                     let ps = Self::lock_process_store(&self.process_store);
                                     ps.processes.values().any(|info| info.name == name)
                                };
                                
                                ui.add_space(5.0);
                                if is_running {
                                    ui.label(egui::RichText::new("▶ Running").color(egui::Color32::GREEN).small());
                                } else {
                                    ui.label(egui::RichText::new("⏹ Stopped").weak().small());
                                }

                                ui.add_space(5.0);

                                // Tags (truncate to 2-3)
                                if let Some(config) = store.gifs.get(&name) {
                                    if !config.tags.is_empty() {
                                        ui.horizontal(|ui| {
                                            for tag in config.tags.iter().take(2) {
                                                ui.label(egui::RichText::new(format!("🏷 {}", tag)).small().weak());
                                            }
                                            if config.tags.len() > 2 {
                                                ui.label(egui::RichText::new("...").small().weak());
                                            }
                                        });
                                    }
                                }
                            });
                        }
                    });
            });
        }
    }

    pub fn show_active_animations(&mut self, ui: &mut egui::Ui) {
        ui.heading("Active Animations");
        ui.label(egui::RichText::new("Manage currently running desktop animations.").small().weak());
        ui.separator();
        
        let mut processes: Vec<(u32, String, u64)> = {
            let ps = Self::lock_process_store(&self.process_store);
            ps.processes.iter()
                .map(|(pid, proc)| (*pid, proc.name.clone(), proc.start_time))
                .collect()
        };
        
        // Sort by name, then by start time
        processes.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2))
        });
        
        if processes.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No active animations.").size(18.0).weak());
            });
            return;
        }
        
        // Group by name to show counts
        use std::collections::HashMap;
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for (_, name, _) in &processes {
            *name_counts.entry(name.clone()).or_insert(0) += 1;
        }
        
        ui.horizontal(|ui| {
             ui.label(egui::RichText::new(format!("Total: {} animation{} running", 
                processes.len(),
                if processes.len() == 1 { "" } else { "s" }
            )).strong());
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⏹ Stop All").clicked() {
                    if let Ok(mut ps) = self.process_store.lock() {
                        let pids: Vec<u32> = ps.processes.keys().cloned().collect();
                        for pid in pids {
                            let _ = ps.kill_process(pid);
                        }
                    }
                }
            });
        });

        ui.separator();
        
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut current_name: Option<String> = None;
                
                for (pid, name, start_time) in &processes {
                    // Show group header when name changes
                    if current_name.as_ref() != Some(name) {
                        if current_name.is_some() {
                            ui.add_space(10.0);
                        }
                        current_name = Some(name.clone());
                        let count = name_counts.get(name).unwrap_or(&1);
                        
                        ui.horizontal(|ui| {
                             ui.heading(format!("{}", name));
                             ui.label(egui::RichText::new(format!("({} instances)", count)).weak());
                        });
                        ui.separator();
                    }
                    
                    // Card for each process
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                             // Calculate runtime
                            let runtime_secs = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .saturating_sub(*start_time);
                            let runtime_str = if runtime_secs < 60 {
                                format!("{}s", runtime_secs)
                            } else if runtime_secs < 3600 {
                                format!("{}m {}s", runtime_secs / 60, runtime_secs % 60)
                            } else {
                                format!("{}h {}m", runtime_secs / 3600, (runtime_secs % 3600) / 60)
                            };

                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(format!("PID: {}", pid)).monospace().small());
                                ui.label(egui::RichText::new(format!("Runtime: {}", runtime_str)).small());
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("⏹ Stop").clicked() {
                                    if let Ok(mut ps) = self.process_store.lock() {
                                        let _ = ps.kill_process(*pid);
                                    }
                                }
                            });
                        });
                    });
                }
            });
    }

    pub fn export_pack(&mut self, config: &GifConfig) {
        // Choose destination directory for this pack
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose export folder for Gif-Engine pack")
            .pick_folder()
        {
            let pack_id = config
                .name
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();

            let pack_root = folder.join(&pack_id);
            let animations_dir = pack_root.join("animations");

            if let Err(e) = std::fs::create_dir_all(&animations_dir) {
                eprintln!("Failed to create pack directories: {}", e);
                return;
            }

            // Copy animation file
            let src = &config.path;
            let filename = src
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("animation.gif");
            let dest = animations_dir.join(filename);

            if let Err(e) = std::fs::copy(src, &dest) {
                eprintln!("Failed to copy animation file: {}", e);
                return;
            }

            // Build simple pack.json with relative URLs
            #[derive(serde::Serialize)]
            struct DefaultSettings {
                #[serde(skip_serializing_if = "Option::is_none")]
                scale: Option<f32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                fps: Option<u32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                alignment: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                always_on_top: Option<bool>,
            }

            #[derive(serde::Serialize)]
            struct Anim {
                filename: String,
                url: String,
                display_name: String,
                #[serde(skip_serializing_if = "Vec::is_empty")]
                tags: Vec<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                default_settings: Option<DefaultSettings>,
            }

            #[derive(serde::Serialize)]
            struct Pack {
                id: String,
                name: String,
                author: String,
                version: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                description: Option<String>,
                #[serde(skip_serializing_if = "Vec::is_empty")]
                tags: Vec<String>,
                animations: Vec<Anim>,
            }

            let mut tags = config.tags.clone();
            tags.sort();
            tags.dedup();

            let default_settings = DefaultSettings {
                scale: config.scale,
                fps: config.fps,
                alignment: Some(config.align.clone()),
                always_on_top: Some(config.overlay),
            };

            let anim = Anim {
                filename: filename.to_string(),
                url: format!("animations/{}", filename),
                display_name: config.name.clone(),
                tags,
                default_settings: Some(default_settings),
            };

            let pack = Pack {
                id: pack_id.clone(),
                name: config.name.clone(),
                author: whoami::username(),
                version: "1.0.0".to_string(),
                description: None,
                tags: Vec::new(),
                animations: vec![anim],
            };

            let pack_json = match serde_json::to_string_pretty(&pack) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to serialize pack.json: {}", e);
                    return;
                }
            };

            if let Err(e) = std::fs::write(pack_root.join("pack.json"), pack_json) {
                eprintln!("Failed to write pack.json: {}", e);
            }
        }
    }
}

