use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::io::{Write, Read};
use crate::app::dirs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GifConfig {
    pub path: PathBuf,
    pub name: String,
    pub fps: Option<u32>,
    pub scale: Option<f32>,
    pub position: Option<(i32, i32)>,
    #[serde(default)]
    pub align: String, // "top-left", "top-right", "bottom-left", "bottom-right", "center", "custom"
    #[serde(default)]
    pub monitor: usize,
    pub overlay: bool,
    #[serde(default)]
    pub tags: Vec<String>, // Tags for organization and search
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String, // "dark", "light"
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_click_through")]
    pub click_through: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_click_through() -> bool {
    false
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            minimize_to_tray: default_minimize_to_tray(),
            click_through: default_click_through(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Store {
    pub gifs: HashMap<String, GifConfig>,
    #[serde(default)]
    pub settings: AppSettings,
}

impl Store {
    fn gifs_dir() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("gif-engine").join("gifs")
    }

    pub fn load() -> Self {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let store_path = config_dir.join("gif-engine").join("store.json");

        if store_path.exists() {
            match fs::read_to_string(&store_path) {
                Ok(content) => {
                    match serde_json::from_str::<Store>(&content) {
                        Ok(store) => return store,
                        Err(e) => {
                            eprintln!("Error parsing store.json: {}", e);
                            // If parsing fails, backup the corrupted file and return default
                            let _ = fs::rename(&store_path, store_path.with_extension("json.bak"));
                        }
                    }
                }
                Err(e) => eprintln!("Error reading store.json: {}", e),
            }
        }
        
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_dir = config_dir.join("gif-engine");
        
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }
        
        let store_path = app_dir.join("store.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(store_path, content)?;
        
        Ok(())
    }

    pub fn add_gif(&mut self, name: String, path: PathBuf) -> Result<(), std::io::Error> {
        // Ensure the gifs directory exists
        let gifs_dir = Self::gifs_dir();
        if !gifs_dir.exists() {
            fs::create_dir_all(&gifs_dir)?;
        }

        // Get the file extension from the original path
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("gif");
        
        // Create the destination path in appdata
        let dest_path = gifs_dir.join(format!("{}.{}", name, extension));
        
        // Copy the file to appdata
        fs::copy(&path, &dest_path)?;
        
        // Use the copied file's absolute path
        let abs_path = fs::canonicalize(&dest_path).unwrap_or(dest_path);
        
        let config = GifConfig {
            path: abs_path,
            name: name.clone(),
            fps: None,
            scale: None,
            position: None,
            align: "center".to_string(),
            monitor: 0,
            overlay: true,
            tags: Vec::new(),
        };
        self.gifs.insert(name, config);
        
        Ok(())
    }
    
    pub fn get_gif(&self, name: &str) -> Option<&GifConfig> {
        self.gifs.get(name)
    }
    
    /// Export store to JSON string
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Export store and all GIF files to a ZIP archive
    pub fn export_zip(&self, zip_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        use zip::write::{FileOptions, ZipWriter};
        use zip::CompressionMethod;
        use std::fs::File;
        
        // Create ZIP file
        let file = File::create(zip_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);
        
        // Copy all GIF files to the zip
        for (_name, config) in &self.gifs {
            // Get filename from path
            let filename = config.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            // Copy the file to the zip
            if config.path.exists() {
                let mut file_content = Vec::new();
                let mut file = File::open(&config.path)?;
                file.read_to_end(&mut file_content)?;
                
                // Add file to zip with name in "gifs/" directory
                let zip_file_path = format!("gifs/{}", filename);
                zip.start_file(&zip_file_path, options)?;
                zip.write_all(&file_content)?;
            }
        }
        
        // Export JSON (paths will be updated on import)
        let json_content = serde_json::to_string_pretty(self)?;
        zip.start_file("store.json", options)?;
        zip.write_all(json_content.as_bytes())?;
        
        zip.finish()?;
        Ok(())
    }
    
    /// Import store from ZIP archive, copying GIFs to local directory
    pub fn import_zip(&mut self, zip_path: &PathBuf, merge: bool) -> Result<(), Box<dyn std::error::Error>> {
        use zip::read::ZipArchive;
        use std::fs::File;
        
        // Open ZIP file
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        // Ensure gifs directory exists
        let gifs_dir = Self::gifs_dir();
        if !gifs_dir.exists() {
            fs::create_dir_all(&gifs_dir)?;
        }
        
        // Extract and copy GIF files, tracking extracted filenames
        let mut store_json: Option<String> = None;
        let mut extracted_files: Vec<String> = Vec::new();
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.name().to_string();
            
            if file_path == "store.json" {
                // Read store.json
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                store_json = Some(content);
            } else if file_path.starts_with("gifs/") {
                // Extract GIF file
                let filename = file_path.strip_prefix("gifs/").unwrap_or(&file_path);
                let dest_path = gifs_dir.join(filename);
                
                // Copy file to local gifs directory
                let mut dest_file = File::create(&dest_path)?;
                std::io::copy(&mut file, &mut dest_file)?;
                extracted_files.push(filename.to_string());
            }
        }
        
        // Parse and import store
        if let Some(json) = store_json {
            let mut imported: Store = serde_json::from_str(&json)?;
            
            // Update all paths to point to local gifs directory
            // Match by filename from original path, or try to find matching extracted file
            for (name, config) in imported.gifs.iter_mut() {
                // Try to get filename from original path
                let original_filename = config.path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string());
                
                // Try to find matching file in extracted files
                let filename = if let Some(ref orig) = original_filename {
                    // Check if this exact filename was extracted
                    if extracted_files.contains(orig) {
                        orig.clone()
                    } else {
                        // Try to find by name with common extensions
                        let mut found = None;
                        for ext in &["gif", "apng", "png"] {
                            let test_name = format!("{}.{}", name, ext);
                            if extracted_files.contains(&test_name) {
                                found = Some(test_name);
                                break;
                            }
                        }
                        found.unwrap_or_else(|| {
                            // Fallback to original filename or construct from name
                            orig.clone()
                        })
                    }
                } else {
                    // No original filename, try to find by name
                    let mut found = None;
                    for ext in &["gif", "apng", "png"] {
                        let test_name = format!("{}.{}", name, ext);
                        if extracted_files.contains(&test_name) {
                            found = Some(test_name);
                            break;
                        }
                    }
                    found.unwrap_or_else(|| format!("{}.gif", name))
                };
                
                // Update path to local gifs directory
                let local_path = gifs_dir.join(&filename);
                if local_path.exists() {
                    // Use canonical path if possible
                    config.path = fs::canonicalize(&local_path).unwrap_or(local_path);
                } else {
                    // If file doesn't exist, keep original path structure but update base
                    config.path = local_path;
                }
            }
            
            // Merge or replace
            if merge {
                // Merge: add/update animations, update settings
                for (name, config) in imported.gifs {
                    self.gifs.insert(name, config);
                }
                // Merge settings (prefer imported settings)
                self.settings = imported.settings;
            } else {
                // Replace: completely replace with imported data
                self.gifs = imported.gifs;
                self.settings = imported.settings;
            }
        } else {
            return Err("store.json not found in ZIP archive".into());
        }
        
        Ok(())
    }
    
    /// Import store from JSON string, merging with existing data
    pub fn import_json(&mut self, json: &str, merge: bool) -> Result<(), serde_json::Error> {
        let imported: Store = serde_json::from_str(json)?;
        
        if merge {
            // Merge: add/update animations, update settings
            for (name, config) in imported.gifs {
                self.gifs.insert(name, config);
            }
            // Merge settings (prefer imported settings)
            self.settings = imported.settings;
        } else {
            // Replace: completely replace with imported data
            self.gifs = imported.gifs;
            self.settings = imported.settings;
        }
        
        Ok(())
    }
}


