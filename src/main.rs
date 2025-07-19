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
