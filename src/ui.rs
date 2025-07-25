﻿use std::{
    io::{stdout, Write},
    process::{self, Command},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use std::collections::VecDeque;

use chrono::{
    DateTime, Local, Timelike, Utc,
    NaiveTime, TimeZone, Duration as ChronoDuration,
};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::config::Config;
use get_if_addrs::get_if_addrs;
use crate::sync_logic::{LtcFrame, LtcState};

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
fn ntp_service_toggle(start: bool) {
    let action = if start { "start" } else { "stop" };
    let _ = Command::new("systemctl").args(&[action, "chrony"]).status();
}

pub fn get_sync_status(delta_ms: i64, config: &Config) -> &'static str {
    if config.timeturner_offset.is_active() {
        "TIMETURNING"
    } else if delta_ms.abs() <= 8 {
        "IN SYNC"
    } else if delta_ms > 10 {
        "CLOCK AHEAD"
    } else {
        "CLOCK BEHIND"
    }
}

pub fn get_jitter_status(jitter_ms: i64) -> &'static str {
    if jitter_ms.abs() < 10 {
        "GOOD"
    } else if jitter_ms.abs() < 40 {
        "AVERAGE"
    } else {
        "BAD"
    }
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

pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    config: Arc<Mutex<Config>>,
) {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    terminal::enable_raw_mode().unwrap();

    let mut logs: VecDeque<String> = VecDeque::with_capacity(10);
    let mut out_of_sync_since: Option<Instant> = None;
    let mut last_delta_update = Instant::now() - Duration::from_secs(1);
    let mut cached_delta_ms: i64 = 0;
    let mut cached_delta_frames: i64 = 0;

    loop {
        // 1️⃣ config
        let cfg = config.lock().unwrap().clone();
        let hw_offset_ms = cfg.hardware_offset_ms;

        // 2️⃣ Chrony + interfaces
        let ntp_active = ntp_service_active();
        let interfaces: Vec<String> = get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter(|ifa| !ifa.is_loopback())
            .map(|ifa| ifa.ip().to_string())
            .collect();

        // 3️⃣ jitter + Δ
        {
            let mut st = state.lock().unwrap();
            if let Some(frame) = st.latest.clone() {
                if frame.status == "LOCK" {
                    // jitter
                    let now_utc = Utc::now();
                    let raw = (now_utc - frame.timestamp).num_milliseconds();
                    let measured = raw - hw_offset_ms;
                    st.record_offset(measured);

                    // Δ = system clock - LTC timecode (use LOCAL time, with offset)
                    let today_local = Local::now().date_naive();
                    let ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as u32;
                    let tc_naive = NaiveTime::from_hms_milli_opt(
                        frame.hours, frame.minutes, frame.seconds, ms,
                    ).expect("Invalid LTC timecode");
                    let naive_dt_local = today_local.and_time(tc_naive);
                    let mut dt_local = Local
                        .from_local_datetime(&naive_dt_local)
                        .single()
                        .expect("Invalid local time");

                    // Apply timeturner offset before calculating delta
                    let offset = &cfg.timeturner_offset;
                    dt_local = dt_local
                        + ChronoDuration::hours(offset.hours)
                        + ChronoDuration::minutes(offset.minutes)
                        + ChronoDuration::seconds(offset.seconds);
                    let frame_offset_ms = (offset.frames as f64 / frame.frame_rate * 1000.0).round() as i64;
                    dt_local = dt_local + ChronoDuration::milliseconds(frame_offset_ms);

                    let delta_ms = (Local::now() - dt_local).num_milliseconds();
                    st.record_clock_delta(delta_ms);
                } else {
                    st.clear_offsets();
                    st.clear_clock_deltas();
                }
            }
        }

        // 4️⃣ averages & status override
        let (avg_jitter_ms, _avg_frames, _, lock_ratio, avg_delta) = {
            let st = state.lock().unwrap();
            (
                st.average_jitter(),
                st.average_frames(),
                st.timecode_match().to_string(),
                st.lock_ratio(),
                st.average_clock_delta(),
            )
        };

        // 5️⃣ cache Δ once/sec & Δ in frames
        if last_delta_update.elapsed() >= Duration::from_secs(1) {
            cached_delta_ms = avg_delta;
            if let Some(frame) = &state.lock().unwrap().latest {
                let frame_ms = 1000.0 / frame.frame_rate;
                cached_delta_frames = ((avg_delta as f64 / frame_ms).round()) as i64;
            } else {
                cached_delta_frames = 0;
            }
            last_delta_update = Instant::now();
        }

        // 6️⃣ sync status wording
        let sync_status = get_sync_status(cached_delta_ms, &cfg);

        // 7️⃣ auto‑sync (same as manual but delayed)
        if sync_status != "IN SYNC" && sync_status != "TIMETURNING" {
            if let Some(start) = out_of_sync_since {
                if start.elapsed() >= Duration::from_secs(5) {
                    if let Some(frame) = &state.lock().unwrap().latest {
                        let entry = match trigger_sync(frame, &cfg) {
                            Ok(ts) => format!("🔄 Auto‑synced to LTC: {}", ts),
                            Err(_) => "❌ Auto‑sync failed".into(),
                        };
                        if logs.len() == 10 { logs.pop_front(); }
                        logs.push_back(entry);
                    }
                    out_of_sync_since = None;
                }
            } else {
                out_of_sync_since = Some(Instant::now());
            }
        } else {
            out_of_sync_since = None;
        }

        // 8️⃣ header & LTC metrics display
        {
            let st = state.lock().unwrap();
            let opt = st.latest.as_ref();
            let status_str = opt.map(|f| f.status.as_str()).unwrap_or("(waiting)");
            let tc_str = match opt {
                Some(f) => format!("LTC Timecode     : {:02}:{:02}:{:02}:{:02}",
                                   f.hours, f.minutes, f.seconds, f.frames),
                None => "LTC Timecode     : …".to_string(),
            };
            let fr_str = match opt {
                Some(f) => format!("Frame Rate       : {:.2}fps", f.frame_rate),
                None => "Frame Rate       : …".to_string(),
            };

            queue!(
                stdout,
                MoveTo(0, 0), Clear(ClearType::All),
                MoveTo(2, 1), Print("Have Blue - NTP Timeturner"),
                MoveTo(2, 2), Print(format!("Serial Port      : {}", serial_port)),
                MoveTo(2, 3), Print(format!("Chrony Service   : {}",
                    if ntp_active { "RUNNING" } else { "MISSING" })),
                MoveTo(2, 4), Print(format!("Interfaces       : {}",
                    interfaces.join(", "))),
                MoveTo(2, 6), Print(format!("LTC Status       : {}", status_str)),
                MoveTo(2, 7), Print(tc_str),
                MoveTo(2, 8), Print(fr_str),
            ).unwrap();
        }

        // system clock
        let now_local: DateTime<Local> = DateTime::from(Utc::now());
        let sys_ts = format!(
            "{:02}:{:02}:{:02}.{:03}",
            now_local.hour(),
            now_local.minute(),
            now_local.second(),
            now_local.timestamp_subsec_millis(),
        );
        queue!(stdout,
            MoveTo(2, 9), Print(format!(
                "System Clock     : {}",
                sys_ts
            ))).unwrap();

        // Δ display
        let dcol = if cached_delta_ms.abs() < 20 {
            Color::Green
        } else if cached_delta_ms.abs() < 100 {
            Color::Yellow
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 11), SetForegroundColor(dcol),
            Print(format!("Timecode Δ       : {:+} ms ({:+} frames)", cached_delta_ms, cached_delta_frames)),
            ResetColor,
        ).unwrap();

        // sync status
        let scol = if sync_status == "IN SYNC" {
            Color::Green
        } else if sync_status == "TIMETURNING" {
            Color::Cyan
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 12), SetForegroundColor(scol),
            Print(format!("Sync Status      : {}", sync_status)),
            ResetColor,
        ).unwrap();

        // jitter & lock ratio
        let jstatus = get_jitter_status(avg_jitter_ms);
        let jcol = if jstatus == "GOOD" {
            Color::Green
        } else if jstatus == "AVERAGE" {
            Color::Yellow
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 13), SetForegroundColor(jcol),
            Print(format!("Sync Jitter      : {}", jstatus)),
            ResetColor,
        ).unwrap();
        queue!(
            stdout,
            MoveTo(2, 14), Print(format!("Lock Ratio       : {:.1}% LOCK",
                lock_ratio
            )),
        ).unwrap();

        // footer + logs
        queue!(
            stdout,
            MoveTo(2, 16), Print("[S] Sync System Clock to LTC    [Q] Quit"),
        ).unwrap();
        for (i, msg) in logs.iter().enumerate() {
            queue!(stdout, MoveTo(2, 18 + i as u16), Print(msg)).unwrap();
        }

        stdout.flush().unwrap();

        // manual sync & quit
        if poll(Duration::from_millis(50)).unwrap() {
            if let Event::Key(evt) = read().unwrap() {
                match evt.code {
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => {
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();
                        terminal::disable_raw_mode().unwrap();
                        process::exit(0);
                    }
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'s') => {
                        if let Some(frame) = &state.lock().unwrap().latest {
                            let entry = match trigger_sync(frame, &cfg) {
                                Ok(ts) => format!("✔ Synced exactly to LTC: {}", ts),
                                Err(_) => "❌ date cmd failed".into(),
                            };
                            if logs.len() == 10 { logs.pop_front(); }
                            logs.push_back(entry);
                        }
                    }
                    _ => {}
                }
            }
        }

        thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::TimeturnerOffset;

    #[test]
    fn test_get_sync_status() {
        let mut config = Config::default();
        assert_eq!(get_sync_status(0, &config), "IN SYNC");
        assert_eq!(get_sync_status(8, &config), "IN SYNC");
        assert_eq!(get_sync_status(-8, &config), "IN SYNC");
        assert_eq!(get_sync_status(9, &config), "CLOCK BEHIND");
        assert_eq!(get_sync_status(10, &config), "CLOCK BEHIND");
        assert_eq!(get_sync_status(11, &config), "CLOCK AHEAD");
        assert_eq!(get_sync_status(-9, &config), "CLOCK BEHIND");
        assert_eq!(get_sync_status(-100, &config), "CLOCK BEHIND");

        // Test TIMETURNING status
        config.timeturner_offset = TimeturnerOffset { hours: 1, minutes: 0, seconds: 0, frames: 0 };
        assert_eq!(get_sync_status(0, &config), "TIMETURNING");
        assert_eq!(get_sync_status(100, &config), "TIMETURNING");
    }

    #[test]
    fn test_get_jitter_status() {
        assert_eq!(get_jitter_status(5), "GOOD");
        assert_eq!(get_jitter_status(-5), "GOOD");
        assert_eq!(get_jitter_status(9), "GOOD");
        assert_eq!(get_jitter_status(10), "AVERAGE");
        assert_eq!(get_jitter_status(39), "AVERAGE");
        assert_eq!(get_jitter_status(-39), "AVERAGE");
        assert_eq!(get_jitter_status(40), "BAD");
        assert_eq!(get_jitter_status(-40), "BAD");
    }
}
