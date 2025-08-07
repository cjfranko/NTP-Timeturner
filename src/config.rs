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
    #[serde(default)]
    pub auto_sync_enabled: bool,
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
            auto_sync_enabled: false,
        }
    }
}

pub fn save_config(path: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut s = String::new();
    s.push_str("# Hardware offset in milliseconds for correcting capture latency.\n");
    s.push_str(&format!("hardwareOffsetMs: {}\n\n", config.hardware_offset_ms));

    s.push_str("# Enable automatic clock synchronization.\n");
    s.push_str("# When enabled, the system will perform an initial full sync, then periodically\n");
    s.push_str("# nudge the clock to keep it aligned with the LTC source.\n");
    s.push_str(&format!("autoSyncEnabled: {}\n\n", config.auto_sync_enabled));

    s.push_str("# Default nudge in milliseconds for adjtimex control.\n");
    s.push_str(&format!("defaultNudgeMs: {}\n\n", config.default_nudge_ms));

    s.push_str("# Time-turning offsets. All values are added to the incoming LTC time.\n");
    s.push_str("# These can be positive or negative.\n");
    s.push_str("timeturnerOffset:\n");
    s.push_str(&format!("  hours: {}\n", config.timeturner_offset.hours));
    s.push_str(&format!("  minutes: {}\n", config.timeturner_offset.minutes));
    s.push_str(&format!("  seconds: {}\n", config.timeturner_offset.seconds));
    s.push_str(&format!("  frames: {}\n", config.timeturner_offset.frames));
    s.push_str(&format!("  milliseconds: {}\n", config.timeturner_offset.milliseconds));

    fs::write(path, s)?;
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
