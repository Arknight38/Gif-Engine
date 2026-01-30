
mod store_repro {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct GifConfig {
        pub path: PathBuf,
        pub name: String,
        pub fps: Option<u32>,
        pub overlay: bool,
    }

    #[derive(Serialize, Deserialize, Default, Debug)]
    pub struct Store {
        pub gifs: HashMap<String, GifConfig>,
    }

    impl Store {
        pub fn load() -> Self {
            let config_dir = PathBuf::from(".");
            let store_path = config_dir.join("gif-engine-repro").join("store.json");

            if store_path.exists() {
                println!("Store file exists at {:?}", store_path);
                match fs::read_to_string(&store_path) {
                    Ok(content) => {
                        println!("Read content: {}", content);
                        match serde_json::from_str(&content) {
                            Ok(store) => return store,
                            Err(e) => println!("JSON parse error: {}", e),
                        }
                    }
                    Err(e) => println!("Read error: {}", e),
                }
            } else {
                println!("Store file does not exist at {:?}", store_path);
            }
            
            Self::default()
        }

        pub fn save(&self) -> Result<(), std::io::Error> {
            let config_dir = PathBuf::from(".");
            let app_dir = config_dir.join("gif-engine-repro");
            
            if !app_dir.exists() {
                fs::create_dir_all(&app_dir)?;
            }
            
            let store_path = app_dir.join("store.json");
            let content = serde_json::to_string_pretty(self)?;
            fs::write(store_path, content)?;
            println!("Saved to {:?}", store_path);
            
            Ok(())
        }

        pub fn add_gif(&mut self, name: String, path: PathBuf) {
            let config = GifConfig {
                path,
                name: name.clone(),
                fps: None,
                overlay: true,
            };
            self.gifs.insert(name, config);
        }
    }
}

use store_repro::Store;
use std::path::PathBuf;

fn main() {
    println!("--- Test 1: Load (should be empty) ---");
    let mut store = Store::load();
    println!("Store: {:?}", store);

    println!("--- Test 2: Add GIF ---");
    store.add_gif("test_anim".to_string(), PathBuf::from("assets/test.gif"));
    println!("Store after add: {:?}", store);

    println!("--- Test 3: Save ---");
    store.save().unwrap();

    println!("--- Test 4: Reload ---");
    let store2 = Store::load();
    println!("Store2: {:?}", store2);
    
    if store2.gifs.contains_key("test_anim") {
        println!("SUCCESS: Animation persisted.");
    } else {
        println!("FAILURE: Animation lost.");
    }
}
