// src/serial_input.rs

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
    _hardware_offset_ms: i64, // no longer used here
) {
    println!("📡 Opening serial port {} @ {} baud", port_path, baud_rate);

    let port = match serialport::new(port_path, baud_rate)
        .timeout(std::time::Duration::from_millis(1000))
        .open()
    {
        Ok(p) => {
            println!("✅ Serial port opened");
            p
        }
        Err(e) => {
            eprintln!("❌ Serial open failed: {}", e);
            return;
        }
    };

    let reader = std::io::BufReader::new(port);
    let re = Regex::new(
        r"\[(LOCK|FREE)\]\s+(\d{2}):(\d{2}):(\d{2})[:;](\d{2})\s+\|\s+([\d.]+)fps",
    )
    .unwrap();

    println!("🔄 Entering LTC read loop…");
    for line in reader.lines() {
        if let Ok(text) = line {
            if let Some(caps) = re.captures(&text) {
                let arrival = Utc::now();
                if let Some(frame) = LtcFrame::from_regex(&caps, arrival) {
                    // update LOCK/FREE counts & timestamp
                    {
                        let mut st = state.lock().unwrap();
                        st.update(frame.clone());
                    }
                    // forward raw frame
                    let _ = sender.send(frame);
                }
            }
        }
    }
}
