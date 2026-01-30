use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use sysinfo::{Pid, System};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app::dirs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunningProcess {
    pub pid: u32,
    pub name: String,
    pub start_time: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ProcessStore {
    pub processes: HashMap<u32, RunningProcess>,
}

impl ProcessStore {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let store_path = config_dir.join("gif-engine").join("running.json");

        if store_path.exists() {
            if let Ok(content) = fs::read_to_string(&store_path) {
                if let Ok(store) = serde_json::from_str::<ProcessStore>(&content) {
                    return store;
                }
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
        
        let store_path = app_dir.join("running.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(store_path, content)?;
        
        Ok(())
    }

    pub fn add_process(&mut self, pid: u32, name: String) {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.processes.insert(pid, RunningProcess {
            pid,
            name,
            start_time,
        });
        let _ = self.save();
    }

    pub fn remove_process(&mut self, pid: u32) {
        self.processes.remove(&pid);
        let _ = self.save();
    }

    pub fn cleanup_dead_processes(&mut self) {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let dead_pids: Vec<u32> = self.processes.keys()
            .filter(|&&pid| sys.process(Pid::from_u32(pid)).is_none())
            .cloned()
            .collect();
            
        for pid in dead_pids {
            self.processes.remove(&pid);
        }
        let _ = self.save();
    }
    
    pub fn kill_process(&mut self, pid: u32) -> bool {
        let sys = System::new_all();
        if let Some(process) = sys.process(Pid::from_u32(pid)) {
            process.kill();
            self.remove_process(pid);
            return true;
        }
        // If it's already dead, just remove it
        self.remove_process(pid);
        false
    }

    // Helper to remove self (for use by the child process before exiting)
    pub fn remove_self(&mut self) {
        let pid = std::process::id();
        self.remove_process(pid);
    }
}


