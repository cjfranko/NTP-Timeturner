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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use crate::sync_logic::LtcState;
    use regex::Regex;

    fn get_ltc_regex() -> Regex {
        Regex::new(
            r"\[(LOCK|FREE)\]\s+(\d{2}):(\d{2}):(\d{2})[:;](\d{2})\s+\|\s+([\d.]+)fps",
        ).unwrap()
    }

    #[test]
    fn test_process_lock_line() {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(LtcState::new()));
        let re = get_ltc_regex();
        let line = "[LOCK] 10:20:30:00 | 25.00fps";

        // Simulate the processing logic from start_serial_thread
        if let Some(caps) = re.captures(line) {
            let arrival = Utc::now();
            if let Some(frame) = LtcFrame::from_regex(&caps, arrival) {
                {
                    let mut st = state.lock().unwrap();
                    st.update(frame.clone());
                }
                let _ = tx.send(frame);
            }
        }

        let st = state.lock().unwrap();
        assert_eq!(st.lock_count, 1);
        assert_eq!(st.free_count, 0);
        let received_frame = rx.try_recv().unwrap();
        assert_eq!(received_frame.status, "LOCK");
        assert_eq!(received_frame.hours, 10);
    }

    #[test]
    fn test_process_free_line() {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(LtcState::new()));
        let re = get_ltc_regex();
        let line = "[FREE] 01:02:03:04 | 29.97fps";

        // Simulate the processing logic
        if let Some(caps) = re.captures(line) {
            let arrival = Utc::now();
            if let Some(frame) = LtcFrame::from_regex(&caps, arrival) {
                {
                    let mut st = state.lock().unwrap();
                    st.update(frame.clone());
                }
                let _ = tx.send(frame);
            }
        }

        let st = state.lock().unwrap();
        assert_eq!(st.lock_count, 0);
        assert_eq!(st.free_count, 1);
        let received_frame = rx.try_recv().unwrap();
        assert_eq!(received_frame.status, "FREE");
        assert_eq!(received_frame.frame_rate, 29.97);
    }

    #[test]
    fn test_ignore_non_matching_line() {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(LtcState::new()));
        let re = get_ltc_regex();
        let line = "this is not a valid ltc line";

        // Simulate the processing logic
        if let Some(caps) = re.captures(line) {
            let arrival = Utc::now();
            if let Some(frame) = LtcFrame::from_regex(&caps, arrival) {
                {
                    let mut st = state.lock().unwrap();
                    st.update(frame.clone());
                }
                let _ = tx.send(frame);
            }
        }

        let st = state.lock().unwrap();
        assert_eq!(st.lock_count, 0);
        assert_eq!(st.free_count, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_ignore_line_with_bad_parseable_data() {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(LtcState::new()));
        let re = get_ltc_regex();
        // The regex will match, but `from_regex` should fail to parse "1.2.3.4" as f64
        let line = "[LOCK] 10:20:30:00 | 1.2.3.4fps";

        // Simulate the processing logic
        if let Some(caps) = re.captures(line) {
            let arrival = Utc::now();
            if let Some(frame) = LtcFrame::from_regex(&caps, arrival) {
                {
                    let mut st = state.lock().unwrap();
                    st.update(frame.clone());
                }
                let _ = tx.send(frame);
            }
        } else {
            panic!("Regex should have matched");
        }

        let st = state.lock().unwrap();
        assert_eq!(st.lock_count, 0);
        assert_eq!(st.free_count, 0);
        assert!(rx.try_recv().is_err());
    }
}
