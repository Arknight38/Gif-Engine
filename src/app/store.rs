use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String, // "dark", "light"
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_minimize_to_tray() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            minimize_to_tray: default_minimize_to_tray(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
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
        };
        self.gifs.insert(name, config);
        
        Ok(())
    }
    
    pub fn get_gif(&self, name: &str) -> Option<&GifConfig> {
        self.gifs.get(name)
    }
}


