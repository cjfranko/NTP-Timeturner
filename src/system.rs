use crate::config::Config;
use crate::sync_logic::LtcFrame;
use chrono::{Duration as ChronoDuration, Local, NaiveTime, TimeZone};
use std::process::Command;

/// Check if Chrony is active
pub fn ntp_service_active() -> bool {
    if let Ok(output) = Command::new("systemctl").args(&["is-active", "chrony"]).output() {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim() == "active"
    } else {
        false
    }
}

/// Toggle Chrony (not used yet)
#[allow(dead_code)]
pub fn ntp_service_toggle(start: bool) {
    let action = if start { "start" } else { "stop" };
    let _ = Command::new("systemctl").args(&[action, "chrony"]).status();
}

pub fn trigger_sync(frame: &LtcFrame, config: &Config) -> Result<String, ()> {
    let today_local = Local::now().date_naive();
    let ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as u32;
    let timecode = NaiveTime::from_hms_milli_opt(frame.hours, frame.minutes, frame.seconds, ms)
        .expect("Invalid LTC timecode");

    let naive_dt = today_local.and_time(timecode);
    let mut dt_local = Local
        .from_local_datetime(&naive_dt)
        .single()
        .expect("Ambiguous or invalid local time");

    // Apply timeturner offset
    let offset = &config.timeturner_offset;
    dt_local = dt_local
        + ChronoDuration::hours(offset.hours)
        + ChronoDuration::minutes(offset.minutes)
        + ChronoDuration::seconds(offset.seconds);
    // Frame offset needs to be converted to milliseconds
    let frame_offset_ms = (offset.frames as f64 / frame.frame_rate * 1000.0).round() as i64;
    dt_local = dt_local + ChronoDuration::milliseconds(frame_offset_ms);
    #[cfg(target_os = "linux")]
    let (ts, success) = {
        let ts = dt_local.format("%H:%M:%S.%3f").to_string();
        let success = Command::new("sudo")
            .arg("date")
            .arg("-s")
            .arg(&ts)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        (ts, success)
    };

    #[cfg(target_os = "macos")]
    let (ts, success) = {
        // macOS `date` command format is `mmddHHMMccyy.SS`
        let ts = dt_local.format("%m%d%H%M%y.%S").to_string();
        let success = Command::new("sudo")
            .arg("date")
            .arg(&ts)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        (ts, success)
    };

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let (ts, success) = {
        // Unsupported OS, always fail
        let ts = dt_local.format("%H:%M:%S.%3f").to_string();
        eprintln!("Unsupported OS for time synchronization");
        (ts, false)
    };

    if success {
        Ok(ts)
    } else {
        Err(())
    }
}
