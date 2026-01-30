use dialoguer::{theme::ColorfulTheme, Select, Input, Confirm};
use console::Term;
use std::process::Command;
use crate::tui::store::Store;
use crate::tui::process::ProcessStore;
use std::path::{Path, PathBuf};
use std::fs;

#[allow(dead_code)]
pub fn run_menu() {
    let mut store = Store::load();
    let mut process_store = ProcessStore::load();
    let term = Term::stdout();

    loop {
        // Cleanup dead processes on every loop
        process_store.cleanup_dead_processes();

        term.clear_screen().ok();
        println!("GifEngine - Desktop Anime Manager");
        println!("---------------------------------");

        let choices = vec![
            "Add Gif/APNG", 
            "Play Animation", 
            "Manage Running Animations",
            "List/Edit Animations", 
            "Exit"
        ];
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Main Menu")
            .default(0)
            .items(&choices)
            .interact()
            .unwrap();

        match selection {
            0 => add_menu(&mut store),
            1 => play_menu(&store, &mut process_store),
            2 => manage_running_menu(&mut process_store),
            3 => list_menu(&mut store),
            4 => break,
            _ => {}
        }
        
        store.save().unwrap_or_else(|e| eprintln!("Failed to save store: {}", e));
    }
}

#[allow(dead_code)]
fn manage_running_menu(process_store: &mut ProcessStore) {
    if process_store.processes.is_empty() {
        println!("No animations currently running.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }

    let mut pids: Vec<u32> = process_store.processes.keys().cloned().collect();
    pids.sort();
    
    let mut items: Vec<String> = pids.iter()
        .map(|pid| {
            let proc = &process_store.processes[pid];
            format!("{} (PID: {})", proc.name, pid)
        })
        .collect();
    items.push("Back".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select animation to STOP")
        .items(&items)
        .interact()
        .unwrap();

    if selection == items.len() - 1 {
        return;
    }

    let pid_to_kill = pids[selection];
    if process_store.kill_process(pid_to_kill) {
        println!("Stopped process {}", pid_to_kill);
    } else {
        println!("Process {} was already dead.", pid_to_kill);
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
}

#[allow(dead_code)]
fn add_menu(store: &mut Store) {
    let choices = vec!["Enter File Path", "Scan Directory", "Back"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Add Animation")
        .items(&choices)
        .interact()
        .unwrap();
        
    match selection {
        0 => add_by_path(store),
        1 => scan_directory(store),
        _ => {}
    }
}

#[allow(dead_code)]
fn add_by_path(store: &mut Store) {
    let path_str: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter file path (gif/png/apng)")
        .interact_text()
        .unwrap();
        
    let path = PathBuf::from(&path_str);
    if !path.exists() {
        println!("File does not exist!");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }

    add_file_to_store(store, path);
}

fn scan_directory(store: &mut Store) {
    let mut scan_options = vec!["Current Directory (.)".to_string()];
    
    if Path::new("assets").exists() {
        scan_options.push("assets".to_string());
    }
    if Path::new("test_assets").exists() {
        scan_options.push("test_assets".to_string());
    }
    scan_options.push("Custom Path".to_string());
    scan_options.push("Back".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select directory to scan")
        .items(&scan_options)
        .default(0)
        .interact()
        .unwrap();

    let dir = match selection {
        x if scan_options[x] == "Back" => return,
        x if scan_options[x].starts_with("Current") => PathBuf::from("."),
        x if scan_options[x] == "Custom Path" => {
             let p: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter directory path")
                .interact_text()
                .unwrap();
             PathBuf::from(p)
        },
        x => PathBuf::from(&scan_options[x]),
    };
    
    if !dir.exists() || !dir.is_dir() {
        println!("Invalid directory!");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }
    
    let mut found_files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext.to_lowercase().as_str() {
                    "gif" | "png" | "apng" => found_files.push(path),
                    _ => {}
                }
            }
        }
    }
    
    if found_files.is_empty() {
        println!("No animations found.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }
    
    let options: Vec<String> = found_files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
        
    let selections = dialoguer::MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select files to add (Space to select, Enter to confirm)")
        .items(&options)
        .interact()
        .unwrap();

    if selections.is_empty() {
        println!("No files selected. Remember to use Space to select items!");
        std::thread::sleep(std::time::Duration::from_secs(2));
        return;
    }
        
    for idx in selections {
        add_file_to_store(store, found_files[idx].clone());
    }
}

fn add_file_to_store(store: &mut Store, path: PathBuf) {
    let default_name = path.file_stem().unwrap().to_string_lossy().into_owned();
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter name for {:?}", default_name))
        .default(default_name)
        .interact_text()
        .unwrap();

    store.add_gif(name, path);
    println!("Added successfully!");
    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn play_menu(store: &Store, process_store: &mut ProcessStore) {
    if store.gifs.is_empty() {
        println!("No GIFs added yet.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }

    let mut items: Vec<String> = store.gifs.keys().cloned().collect();
    items.push("Back".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Animation to Play")
        .items(&items)
        .default(0)
        .interact()
        .unwrap();

    if selection == items.len() - 1 {
        return;
    }

    let name = &items[selection];
    if let Some(config) = store.get_gif(name) {
        println!("Launching {}...", name);
        
        // Spawn child process
        let exe = std::env::current_exe().unwrap();
        let mut cmd = Command::new(exe);
        cmd.arg("play")
           .arg(&config.path);
           
        if let Some(fps) = config.fps {
            cmd.arg("--fps").arg(fps.to_string());
        }
        
        if let Some(scale) = config.scale {
            cmd.arg("--scale").arg(scale.to_string());
        }

        if config.overlay {
            cmd.arg("--overlay");
        }

        // Detach process
        match cmd.spawn() {
            Ok(child) => {
                println!("Started successfully! PID: {}", child.id());
                process_store.add_process(child.id(), name.clone());
            },
            Err(e) => println!("Failed to start: {}", e),
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn list_menu(store: &mut Store) {
     if store.gifs.is_empty() {
        println!("No animations added yet.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }

    let mut items: Vec<String> = store.gifs.keys().cloned().collect();
    items.push("Back".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Animation to edit")
        .items(&items)
        .interact()
        .unwrap();
        
    if selection == items.len() - 1 {
        return;
    }

    let name = items[selection].clone();
    
    // Edit menu
    if let Some(config) = store.gifs.get_mut(&name) {
        println!("Editing {}", name);
        
        loop {
            let edit_choices = vec![
                "Change FPS",
                "Change Scale",
                "Change Position",
                "Toggle Overlay",
                "Preview",
                "Back"
            ];
            
            let edit_sel = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Edit options for {}", name))
                .items(&edit_choices)
                .default(0)
                .interact()
                .unwrap();
                
            match edit_sel {
                0 => {
                    let fps = Input::<u32>::with_theme(&ColorfulTheme::default())
                        .with_prompt("Target FPS (0 for default)")
                        .default(config.fps.unwrap_or(0))
                        .interact()
                        .unwrap();
                        
                    if fps > 0 {
                        config.fps = Some(fps);
                    } else {
                        config.fps = None;
                    }
                },
                1 => {
                    let scale = Input::<f32>::with_theme(&ColorfulTheme::default())
                        .with_prompt("Scale Factor (e.g. 0.5 for half size, 1.0 for original)")
                        .default(config.scale.unwrap_or(1.0))
                        .interact()
                        .unwrap();
                        
                    if scale > 0.0 && (scale - 1.0).abs() > f32::EPSILON {
                        config.scale = Some(scale);
                    } else {
                        config.scale = None;
                    }
                },
                2 => {
                    let current_x = config.position.map(|p| p.0).unwrap_or(100);
                    let current_y = config.position.map(|p| p.1).unwrap_or(100);
                    
                    let use_pos = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enable custom position?")
                        .default(config.position.is_some())
                        .interact()
                        .unwrap();
                        
                    if use_pos {
                        let x = Input::<i32>::with_theme(&ColorfulTheme::default())
                            .with_prompt("X Coordinate")
                            .default(current_x)
                            .interact()
                            .unwrap();
                            
                        let y = Input::<i32>::with_theme(&ColorfulTheme::default())
                            .with_prompt("Y Coordinate")
                            .default(current_y)
                            .interact()
                            .unwrap();
                            
                        config.position = Some((x, y));
                    } else {
                        config.position = None;
                    }
                },
                3 => {
                    let overlay = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enable Overlay (Always on top)?")
                        .default(config.overlay)
                        .interact()
                        .unwrap();
                    config.overlay = overlay;
                },
                4 => {
                    // Preview Mode
                    // Spawn a temporary process for preview
                    println!("Launching preview (Close window to stop)...");
                    let exe = std::env::current_exe().unwrap();
                    let mut cmd = Command::new(exe);
                    cmd.arg("play")
                       .arg(&config.path);
                       
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
                    if config.overlay {
                        cmd.arg("--overlay");
                    }
                    
                    match cmd.spawn() {
                        Ok(mut child) => {
                             // Wait for user to finish preview
                             println!("Preview running. Press Enter to stop and return to edit menu.");
                             let _ = std::io::stdin().read_line(&mut String::new());
                             let _ = child.kill();
                        },
                        Err(e) => println!("Failed to preview: {}", e),
                    }
                },
                5 => break,
                _ => {}
            }
        }
    }
}
