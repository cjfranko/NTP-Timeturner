// src/ui.rs

use std::{
    io::{stdout, Write},
    process::{self, Command},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use std::collections::VecDeque;

use chrono::{Local, Timelike, Utc, NaiveTime, Duration as ChronoDuration, TimeZone};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use get_if_addrs::get_if_addrs;
use crate::sync_logic::LtcState;

/// Check if the Chrony service is active
fn ntp_service_active() -> bool {
    if let Ok(output) = Command::new("systemctl").args(&["is-active", "chrony"]).output() {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim() == "active"
    } else {
        false
    }
}

/// Toggle the Chrony service (start if `start` is true, stop otherwise)

fn _ntp_service_toggle(start: bool) {
    let action = if start { "start" } else { "stop" };
    let _ = Command::new("systemctl").args(&[action, "chrony"]).status();
}

/// Launch the full-featured TUI; reads `offset` live and performs auto-sync if out of sync.
pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    offset: Arc<Mutex<i64>>,
) {
    let mut stdout = stdout();
    // Enter alternate screen and hide cursor
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    terminal::enable_raw_mode().unwrap();

    // Recent log of messages (last 10)
    let mut logs: VecDeque<String> = VecDeque::with_capacity(10);
    // Tracks when we first detected out-of-sync
    let mut out_of_sync_since: Option<Instant> = None;

    // For caching the timecode delta display once per second
    let mut last_delta_update = Instant::now() - Duration::from_secs(1);
    let mut cached_delta_ms: i64 = 0;
    let mut cached_delta_frames: i64 = 0;

    loop {
        // 1️⃣ Read hardware offset from watcher
        let hw_offset_ms = *offset.lock().unwrap();

        // 2️⃣ Check Chrony status and gather network interfaces
        let ntp_active = ntp_service_active();
        let interfaces: Vec<String> = get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter(|ifa| !ifa.is_loopback())
            .map(|ifa| ifa.ip().to_string())
            .collect();

        // 3️⃣ Measure & record jitter and Timecode Δ when LOCKED; clear on FREE
        {
            let mut st = state.lock().unwrap();
            if let Some(frame) = st.latest.clone() {
                if frame.status == "LOCK" {
                    // Jitter in ms
                    let now = Utc::now();
                    let raw = (now - frame.timestamp).num_milliseconds();
                    let measured = raw - hw_offset_ms;
                    st.record_offset(measured);

                    // Timecode delta
                    let local = Local::now();
                    let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                    let base_time = NaiveTime::from_hms_opt(frame.hours, frame.minutes, frame.seconds)
                        .unwrap_or(local.time());
                    let offset_dt = local.date_naive().and_time(base_time)
                        + ChronoDuration::milliseconds(sub_ms);
                    let ltc_dt = Local.from_local_datetime(&offset_dt)
                        .single()
                        .unwrap_or(local);
                    let delta_ms = local.signed_duration_since(ltc_dt).num_milliseconds();
                    st.record_clock_delta(delta_ms);
                } else {
                    st.clear_offsets();
                    st.clear_clock_deltas();
                }
            }
        }

        // 4️⃣ Compute averages & statuses
let (avg_ms, _avg_frames, status_str, lock_ratio, avg_delta) = {
            let st = state.lock().unwrap();
            (
                st.average_jitter(),
                st.average_frames(),
                st.timecode_match().to_string(),
                st.lock_ratio(),
                st.average_clock_delta(),
            )
        };

        // 5️⃣ Update cached delta once per second
        if last_delta_update.elapsed() >= Duration::from_secs(1) {
            cached_delta_ms = avg_delta;
            // Recompute frames equivalent
            if let Ok(st2) = state.lock() {
                if let Some(frame) = &st2.latest {
                    let ms_pf = 1000.0 / frame.frame_rate;
                    cached_delta_frames = (cached_delta_ms as f64 / ms_pf).round() as i64;
                }
            }
            last_delta_update = Instant::now();
        }

        // 6️⃣ Auto-sync if "OUT OF SYNC" or Δ >5ms for 5s
        if status_str == "OUT OF SYNC" || cached_delta_ms.abs() > 5 {
            if let Some(start) = out_of_sync_since {
                if start.elapsed() >= Duration::from_secs(5) {
                    // Perform sync to LTC
                    if let Ok(stl) = state.lock() {
                        if let Some(frame) = &stl.latest {
                            let local_now = Local::now();
                            let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0)
                                .round() as i64;
                            let base_time = NaiveTime::from_hms_opt(
                                frame.hours,
                                frame.minutes,
                                frame.seconds,
                            ).unwrap_or(local_now.time());
                            let offset_dt = local_now.date_naive().and_time(base_time)
                                + ChronoDuration::milliseconds(sub_ms);
                            let ltc_dt = Local.from_local_datetime(&offset_dt)
                                .single()
                                .unwrap_or(local_now);
                            let ts = format!("{:02}:{:02}:{:02}.{:03}",
                                ltc_dt.hour(),
                                ltc_dt.minute(),
                                ltc_dt.second(),
                                ltc_dt.timestamp_subsec_millis()
                            );
                            let res = Command::new("sudo")
                                .arg("date")
                                .arg("-s")
                                .arg(&ts)
                                .status();
                            let msg = if res.as_ref().map_or(false, |s| s.success()) {
                                format!("🔄 Auto-synced to LTC: {}", ts)
                            } else {
                                "❌ Auto-sync failed".into()
                            };
                            if logs.len() == 10 {
                                logs.pop_front();
                            }
                            logs.push_back(msg);
                        }
                    }
                    out_of_sync_since = None;
                }
            } else {
                out_of_sync_since = Some(Instant::now());
            }
        } else {
            out_of_sync_since = None;
        }

        // 7️⃣ Draw static UI header
        queue!(
            stdout,
            MoveTo(0, 0), Clear(ClearType::All),
            MoveTo(2, 1), Print("Have Blue - NTP Timeturner - FrameWorks Testing"),
            MoveTo(2, 2), Print(format!("Serial Port      : {}", serial_port)),
            MoveTo(2, 3), Print(format!("Chrony Service   : {}", if ntp_active { "RUNNING" } else { "MISSING" })),
            MoveTo(2, 4), Print(format!("Interfaces       : {}", interfaces.join(", "))),
        )
        .unwrap();

        // 8️⃣ Draw LTC and System Clock
        if let Ok(st) = state.lock() {
            if let Some(frame) = &st.latest {
                queue!(
                    stdout,
                    MoveTo(2, 6), Print(format!("LTC Status       : {}", frame.status)),
                    MoveTo(2, 7), Print(format!(
                        "LTC Timecode     : {:02}:{:02}:{:02}:{:02}",
                        frame.hours, frame.minutes, frame.seconds, frame.frames
                    )),
                    MoveTo(2, 8), Print(format!("Frame Rate       : {:.2}fps", frame.frame_rate)),
                )
                .unwrap();
            } else {
                queue!(
                    stdout,
                    MoveTo(2, 6), Print("LTC Status       : (waiting)"),
                    MoveTo(2, 7), Print("LTC Timecode     : …"),
                    MoveTo(2, 8), Print("Frame Rate       : …"),
                )
                .unwrap();
            }
            let now_local = Local::now();
            let sys_ts = format!("{:02}:{:02}:{:02}.{:03}",
                now_local.hour(), now_local.minute(), now_local.second(), now_local.timestamp_subsec_millis()
            );
            queue!(stdout, MoveTo(2, 9), Print(format!("System Clock     : {}", sys_ts))).unwrap();
        }

        // 9️⃣ Overlay metrics in new order
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
        )
        .unwrap();

        let scol = if status_str == "IN SYNC" {
            Color::Green
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 12), SetForegroundColor(scol),
            Print(format!("Sync Status      : {}", status_str)),
            ResetColor,
        )
        .unwrap();

        let jstatus = if avg_ms.abs() < 10 {
            "GOOD"
        } else if avg_ms.abs() < 40 {
            "AVERAGE"
        } else {
            "BAD"
        };
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
        )
        .unwrap();

        queue!(
            stdout,
            MoveTo(2, 14), Print(format!("Lock Ratio       : {:.1}% LOCK", lock_ratio)),
        )
        .unwrap();

        // 10️⃣ Footer and logs
        queue!(
            stdout,
            MoveTo(2, 16), Print("[S] Sync system clock to LTC    [Q] Quit"),
        )
        .unwrap();
        for (i, log_msg) in logs.iter().enumerate() {
            queue!(stdout, MoveTo(2, 18 + i as u16), Print(log_msg)).unwrap();
        }

        stdout.flush().unwrap();

        // 11️⃣ Handle manual sync and quit keys
        if poll(Duration::from_millis(50)).unwrap() {
            if let Event::Key(evt) = read().unwrap() {
                match evt.code {
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => {
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();
                        terminal::disable_raw_mode().unwrap();
                        process::exit(0);
                    }
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'s') => {
                        if let Ok(stlock) = state.lock() {
                            if let Some(frame) = &stlock.latest {
                                let local_now = Local::now();
                                let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0)
                                    .round() as i64;
                                let base_time = NaiveTime::from_hms_opt(
                                    frame.hours,
                                    frame.minutes,
                                    frame.seconds,
                                )
                                .unwrap_or(local_now.time());
                                let offset_dt = local_now.date_naive().and_time(base_time)
                                    + ChronoDuration::milliseconds(sub_ms);
                                let ltc_dt = Local.from_local_datetime(&offset_dt)
                                    .single()
                                    .unwrap_or(local_now);
                                let ts = format!(
                                    "{:02}:{:02}:{:02}.{:03}",
                                    ltc_dt.hour(),
                                    ltc_dt.minute(),
                                    ltc_dt.second(),
                                    ltc_dt.timestamp_subsec_millis(),
                                );
                                let res = Command::new("sudo")
                                    .arg("date")
                                    .arg("-s")
                                    .arg(&ts)
                                    .status();
                                let msg = if res.as_ref().map_or(false, |s| s.success()) {
                                    format!("✔ Synced exactly to LTC: {}", ts)
                                } else {
                                    "❌ date cmd failed".into()
                                };
                                if logs.len() == 10 {
                                    logs.pop_front();
                                }
                                logs.push_back(msg);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        thread::sleep(Duration::from_millis(25));
    }
}
