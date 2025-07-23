use crate::config::Config;
use crate::sync_logic::LtcFrame;
use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveTime, TimeZone};
use std::process::Command;

/// Check if Chrony is active
pub fn ntp_service_active() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("systemctl").args(&["is-active", "chrony"]).output() {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == "active"
        } else {
            false
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        // systemctl is not available on non-Linux platforms.
        false
    }
}

/// Toggle Chrony (not used yet)
#[allow(dead_code)]
pub fn ntp_service_toggle(start: bool) {
    #[cfg(target_os = "linux")]
    {
        let action = if start { "start" } else { "stop" };
        let _ = Command::new("systemctl").args(&[action, "chrony"]).status();
    }
    #[cfg(not(target_os = "linux"))]
    {
        // No-op on non-Linux.
        // The parameter is unused, but the function is dead code anyway.
        let _ = start;
    }
}

pub fn calculate_target_time(frame: &LtcFrame, config: &Config) -> DateTime<Local> {
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
    dt_local + ChronoDuration::milliseconds(frame_offset_ms)
}

pub fn trigger_sync(frame: &LtcFrame, config: &Config) -> Result<String, ()> {
    let dt_local = calculate_target_time(frame, config);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimeturnerOffset;
    use chrono::{Timelike, Utc};

    // Helper to create a test frame
    fn get_test_frame(h: u32, m: u32, s: u32, f: u32) -> LtcFrame {
        LtcFrame {
            status: "LOCK".to_string(),
            hours: h,
            minutes: m,
            seconds: s,
            frames: f,
            frame_rate: 25.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_ntp_service_active_on_non_linux() {
        // On non-Linux platforms, this should always be false.
        #[cfg(not(target_os = "linux"))]
        assert!(!ntp_service_active());
    }

    #[test]
    fn test_calculate_target_time_no_offset() {
        let frame = get_test_frame(10, 20, 30, 0);
        let config = Config::default();
        let target_time = calculate_target_time(&frame, &config);

        assert_eq!(target_time.hour(), 10);
        assert_eq!(target_time.minute(), 20);
        assert_eq!(target_time.second(), 30);
    }

    #[test]
    fn test_calculate_target_time_with_positive_offset() {
        let frame = get_test_frame(10, 20, 30, 0);
        let mut config = Config::default();
        config.timeturner_offset = TimeturnerOffset {
            hours: 1,
            minutes: 5,
            seconds: 10,
            frames: 12, // 12 frames at 25fps is 480ms
        };

        let target_time = calculate_target_time(&frame, &config);

        assert_eq!(target_time.hour(), 11);
        assert_eq!(target_time.minute(), 25);
        assert_eq!(target_time.second(), 40);
        // 480ms
        assert_eq!(target_time.nanosecond(), 480_000_000);
    }

    #[test]
    fn test_calculate_target_time_with_negative_offset() {
        let frame = get_test_frame(10, 20, 30, 12); // 12 frames = 480ms
        let mut config = Config::default();
        config.timeturner_offset = TimeturnerOffset {
            hours: -1,
            minutes: -5,
            seconds: -10,
            frames: -12, // -480ms
        };

        let target_time = calculate_target_time(&frame, &config);

        assert_eq!(target_time.hour(), 9);
        assert_eq!(target_time.minute(), 15);
        assert_eq!(target_time.second(), 20);
        assert_eq!(target_time.nanosecond(), 0);
    }
}
