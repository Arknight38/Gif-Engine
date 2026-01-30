use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
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

    pub fn add_gif(&mut self, name: String, path: PathBuf) {
        let abs_path = fs::canonicalize(&path).unwrap_or(path);
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
    }
    
    pub fn get_gif(&self, name: &str) -> Option<&GifConfig> {
        self.gifs.get(name)
    }
}

// Simple helper to find config dir since `dirs` crate isn't added yet, let's just use local for now or add `dirs`
// Wait, I should add `dirs` if I want to use it. Or just use a local file for now.
// Let's use local file "anime-store.json" for simplicity in this session.
mod dirs {
    use std::path::PathBuf;
    pub fn config_dir() -> Option<PathBuf> {
        Some(PathBuf::from(".")) 
    }
}
