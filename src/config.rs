// src/config.rs

use notify::{
    recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult,
    Watcher,
};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub hardware_offset_ms: i64,
    #[serde(default)]
    pub ptp_enabled: bool,
    #[serde(default = "default_ptp_interface")]
    pub ptp_interface: String,
}

fn default_ptp_interface() -> String {
    "eth0".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hardware_offset_ms: 0,
            ptp_enabled: false,
            ptp_interface: default_ptp_interface(),
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Self {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            return Self::default();
        }
        serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Failed to parse config.json: {}, using default", e);
            Self::default()
        })
    }
}

pub fn watch_config(path: &str) -> Arc<Mutex<Config>> {
    let initial_config = Config::load(&PathBuf::from(path));
    let config = Arc::new(Mutex::new(initial_config));

    let watch_path = PathBuf::from(path);
    let watch_path_for_cb = watch_path.clone();
    let config_for_cb = Arc::clone(&config);

    std::thread::spawn(move || {
        let event_handler = move |res: NotifyResult<Event>| {
            if let Ok(evt) = res {
                if matches!(evt.kind, EventKind::Modify(_)) {
                    let new_cfg = Config::load(&watch_path_for_cb);
                    eprintln!("🔄 Reloaded config.json: {:?}", new_cfg);
                    let mut cfg = config_for_cb.lock().unwrap();
                    *cfg = new_cfg;
                }
            }
        };

        let mut watcher: RecommendedWatcher =
            recommended_watcher(event_handler).expect("Failed to create file watcher");

        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch config.json");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    config
}
