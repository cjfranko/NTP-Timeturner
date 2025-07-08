mod sync_logic;
mod serial_input;
mod ui;

use crate::sync_logic::LtcState;
use crate::serial_input::start_serial_thread;
use crate::ui::start_ui;

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

fn main() {
    println!("🧪 Timeturner startup...");

    let (tx, rx) = mpsc::channel();
    println!("✅ Channel created");

    let ltc_state = Arc::new(Mutex::new(LtcState::new()));
    println!("✅ State initialised");

    start_serial_thread("/dev/ttyACM0", 115200, tx.clone(), ltc_state.clone());
    println!("🚀 Serial thread launched");

    let ui_state = ltc_state.clone();
    thread::spawn(move || {
        println!("🖥️ UI thread started");
        start_ui(ui_state);
    });

    println!("📡 Main thread entering loop...");

    for frame in rx {
        println!(
            "📥 Received LTC frame: {:02}:{:02}:{:02}:{:02} [{}]",
            frame.hours, frame.minutes, frame.seconds, frame.frames, frame.status
        );
    }
}
