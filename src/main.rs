// src/main.rs

mod config;
mod sync_logic;
mod serial_input;
mod ui;

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

/// Embed the default config.json at compile time.
const DEFAULT_CONFIG: &str = include_str!("../config.json");

/// If no `config.json` exists alongside the binary, write out the default.
fn ensure_config() {
    let p = Path::new("config.json");
    if !p.exists() {
        fs::write(p, DEFAULT_CONFIG)
            .expect("Failed to write default config.json");
        eprintln!("⚙️  Emitted default config.json");
    }
}

fn main() {
    // 🔄 Ensure there's always a config.json present
    ensure_config();

    // 1️⃣ Start watching config.json for changes
    let hw_offset = watch_config("config.json");
    println!("🔧 Watching config.json (hardware_offset_ms)...");

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

    // 5️⃣ Spawn the UI renderer thread, passing the live offset Arc
    {
        let ui_state     = ltc_state.clone();
        let offset_clone = hw_offset.clone();
        let port         = "/dev/ttyACM0".to_string();
        thread::spawn(move || {
            println!("🖥️ UI thread launched");
            start_ui(ui_state, port, offset_clone);
        });
    }

    // 6️⃣ Keep main thread alive
    println!("📡 Main thread entering loop...");
    for _frame in rx {
        // no-op
    }
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
            let _ = fs::remove_file("config.json");
        }
    }

    #[test]
    fn test_ensure_config() {
        let _guard = ConfigGuard; // Cleanup when _guard goes out of scope.

        // --- Test 1: File creation ---
        // Pre-condition: config.json does not exist.
        let _ = fs::remove_file("config.json");

        ensure_config();

        // Post-condition: config.json exists and has default content.
        let p = Path::new("config.json");
        assert!(p.exists(), "config.json should have been created");
        let contents = fs::read_to_string(p).expect("Failed to read created config.json");
        assert_eq!(contents, DEFAULT_CONFIG, "config.json content should match default");

        // --- Test 2: File is not overwritten ---
        // Pre-condition: config.json exists with different content.
        let custom_content = "{\"hardware_offset_ms\": 999}";
        fs::write("config.json", custom_content)
            .expect("Failed to write custom config.json for test");

        ensure_config();

        // Post-condition: config.json still has the custom content.
        let contents_after = fs::read_to_string("config.json")
            .expect("Failed to read config.json after second ensure_config call");
        assert_eq!(contents_after, custom_content, "config.json should not be overwritten");
    }
}
