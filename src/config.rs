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

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimeturnerOffset {
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub frames: i64,
    #[serde(default)]
    pub milliseconds: i64,
}

impl TimeturnerOffset {
    pub fn is_active(&self) -> bool {
        self.hours != 0
            || self.minutes != 0
            || self.seconds != 0
            || self.frames != 0
            || self.milliseconds != 0
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub hardware_offset_ms: i64,
    #[serde(default)]
    pub timeturner_offset: TimeturnerOffset,
    #[serde(default = "default_nudge_ms")]
    pub default_nudge_ms: i64,
}

fn default_nudge_ms() -> i64 {
    2 // Default nudge is 2ms
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
        serde_yaml::from_str(&contents).unwrap_or_else(|e| {
            log::warn!("Failed to parse config, using default: {}", e);
            Self::default()
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hardware_offset_ms: 0,
            timeturner_offset: TimeturnerOffset::default(),
            default_nudge_ms: default_nudge_ms(),
        }
    }
}

pub fn save_config(path: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let contents = serde_yaml::to_string(config)?;
    fs::write(path, contents)?;
    Ok(())
}

pub fn watch_config(path: &str) -> Arc<Mutex<Config>> {
    let initial_config = Config::load(&PathBuf::from(path));
    let config = Arc::new(Mutex::new(initial_config));

    let watch_path = PathBuf::from(path);
    let watch_path_for_cb = watch_path.clone();
    let config_for_cb = Arc::clone(&config);

    std::thread::spawn(move || {
        let mut watcher: RecommendedWatcher = recommended_watcher(move |res: NotifyResult<Event>| {
            if let Ok(evt) = res {
                if matches!(evt.kind, EventKind::Modify(_)) {
                    let new_cfg = Config::load(&watch_path_for_cb);
                    let mut cfg = config_for_cb.lock().unwrap();
                    *cfg = new_cfg;
                    log::info!("🔄 Reloaded config.yml: {:?}", *cfg);
                }
            }
        })
        .expect("Failed to create file watcher");

        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch config.yml");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    config
}
