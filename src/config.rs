// src/config.rs

use notify::{
    recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult,
    Watcher,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub hardware_offset_ms: i64,
}

impl Config {
    pub fn load(path: &PathBuf) -> Self {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Self { hardware_offset_ms: 0 },
        };
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            return Self { hardware_offset_ms: 0 };
        }
        serde_json::from_str(&contents).unwrap_or(Self { hardware_offset_ms: 0 })
    }
}

pub fn save_config(path: &str, config: &Config) -> std::io::Result<()> {
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(path, contents)
}

pub fn watch_config(path: &str) -> Arc<Mutex<i64>> {
    let initial = Config::load(&PathBuf::from(path)).hardware_offset_ms;
    let offset = Arc::new(Mutex::new(initial));

    // Owned PathBuf for watch() call
    let watch_path = PathBuf::from(path);
    // Clone for moving into the closure
    let watch_path_for_cb = watch_path.clone();
    let offset_for_cb = Arc::clone(&offset);

    std::thread::spawn(move || {
        // Move `watch_path_for_cb` into the callback
        let mut watcher: RecommendedWatcher = recommended_watcher(move |res: NotifyResult<Event>| {
            if let Ok(evt) = res {
                if matches!(evt.kind, EventKind::Modify(_)) {
                    let new_cfg = Config::load(&watch_path_for_cb);
                    let mut hw = offset_for_cb.lock().unwrap();
                    *hw = new_cfg.hardware_offset_ms;
                    eprintln!("🔄 Reloaded hardware_offset_ms = {}", *hw);
                }
            }
        })
        .expect("Failed to create file watcher");

        // Use the original `watch_path` here
        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch config.json");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    offset
}
