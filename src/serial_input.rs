use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use chrono::Utc;
use regex::Regex;
use crate::sync_logic::{LtcFrame, LtcState};

pub fn start_serial_thread(
    port_path: &str,
    baud_rate: u32,
    sender: Sender<LtcFrame>,
    state: Arc<Mutex<LtcState>>,
) {
    println!("📡 Attempting to open serial port: {} @ {} baud", port_path, baud_rate);

    let port = serialport::new(port_path, baud_rate)
        .timeout(std::time::Duration::from_millis(1000))
        .open();

    match &port {
        Ok(_) => println!("✅ Serial port opened successfully"),
        Err(e) => {
            eprintln!("❌ Failed to open serial port: {}", e);
            return; // Exit early, no point continuing
        }
    }

    let reader = std::io::BufReader::new(port.unwrap());
    let re = Regex::new(r"\[(LOCK|FREE)\]\s+(\d{2}):(\d{2}):(\d{2})[:;](\d{2})\s+\|\s+([\d.]+)fps")
        .unwrap();

    println!("🔄 Starting LTC read loop...");

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(caps) = re.captures(&line) {
                let frame = LtcFrame::from_regex(&caps, Utc::now());
                if let Some(frame) = frame {
                    sender.send(frame.clone()).ok();
                    let mut state_lock = state.lock().unwrap();
                    state_lock.update(frame);
                }
            }
        }
    }
}
