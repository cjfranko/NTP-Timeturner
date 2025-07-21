// src/main.rs

mod api;
mod config;
mod sync_logic;
mod serial_input;
mod ui;

use crate::api::start_api_server;
use crate::config::watch_config;
use crate::sync_logic::LtcState;
use crate::serial_input::start_serial_thread;
use crate::ui::start_ui;

use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
};
use tokio::task::{self, LocalSet};

/// Default config content, embedded in the binary.
const DEFAULT_CONFIG: &str = r#"
# Hardware offset in milliseconds for correcting capture latency.
hardwareOffsetMs: 20

# Time-turning offsets. All values are added to the incoming LTC time.
# These can be positive or negative.
timeturnerOffset:
  hours: 0
  minutes: 0
  seconds: 0
  frames: 0
"#;

/// If no `config.yml` exists alongside the binary, write out the default.
fn ensure_config() {
    let p = Path::new("config.yml");
    if !p.exists() {
        fs::write(p, DEFAULT_CONFIG.trim())
            .expect("Failed to write default config.yml");
        eprintln!("⚙️  Emitted default config.yml");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 🔄 Ensure there's always a config.json present
    ensure_config();

    // 1️⃣ Start watching config.yml for changes
    let config = watch_config("config.yml");
    println!("🔧 Watching config.yml...");

    // 2️⃣ Channel for raw LTC frames
    let (tx, rx) = mpsc::channel();
    println!("✅ Channel created");

    // 3️⃣ Shared state for UI and serial reader
    let ltc_state = Arc::new(Mutex::new(LtcState::new()));
    println!("✅ State initialised");

    // 4️⃣ Spawn the serial reader thread (no offset here)
    {
        let tx_clone    = tx.clone();
        let state_clone = ltc_state.clone();
        thread::spawn(move || {
            println!("🚀 Serial thread launched");
            start_serial_thread(
                "/dev/ttyACM0",
                115200,
                tx_clone,
                state_clone,
                0, // ignored in serial path
            );
        });
    }

    // 5️⃣ Spawn the UI renderer thread, passing the live config Arc
    {
        let ui_state     = ltc_state.clone();
        let config_clone = config.clone();
        let port         = "/dev/ttyACM0".to_string();
        thread::spawn(move || {
            println!("🖥️ UI thread launched");
            start_ui(ui_state, port, config_clone);
        });
    }

    // 6️⃣ Set up a LocalSet for the API server.
    let local = LocalSet::new();
    local
        .run_until(async move {
            // 7️⃣ Spawn the API server thread
            {
                let api_state = ltc_state.clone();
                let config_clone = config.clone();
                task::spawn_local(async move {
                    if let Err(e) = start_api_server(api_state, config_clone).await {
                        eprintln!("API server error: {}", e);
                    }
                });
            }

            // 8️⃣ Keep main thread alive by consuming LTC frames in a blocking task
            println!("📡 Main thread entering loop...");
            let _ = task::spawn_blocking(move || {
                // This will block the thread, but it's a blocking-safe thread.
                for _frame in rx {
                    // no-op
                }
            })
            .await;
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// RAII guard to ensure config file is cleaned up after test.
    struct ConfigGuard;

    impl Drop for ConfigGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file("config.yml");
        }
    }

    #[test]
    fn test_ensure_config() {
        let _guard = ConfigGuard; // Cleanup when _guard goes out of scope.

        // --- Test 1: File creation ---
        // Pre-condition: config.yml does not exist.
        let _ = fs::remove_file("config.yml");

        ensure_config();

        // Post-condition: config.yml exists and has default content.
        let p = Path::new("config.yml");
        assert!(p.exists(), "config.yml should have been created");
        let contents = fs::read_to_string(p).expect("Failed to read created config.yml");
        assert_eq!(contents, DEFAULT_CONFIG.trim(), "config.yml content should match default");

        // --- Test 2: File is not overwritten ---
        // Pre-condition: config.yml exists with different content.
        let custom_content = "hardwareOffsetMs: 999";
        fs::write("config.yml", custom_content)
            .expect("Failed to write custom config.yml for test");

        ensure_config();

        // Post-condition: config.yml still has the custom content.
        let contents_after = fs::read_to_string("config.yml")
            .expect("Failed to read config.yml after second ensure_config call");
        assert_eq!(contents_after, custom_content, "config.yml should not be overwritten");
    }
}
