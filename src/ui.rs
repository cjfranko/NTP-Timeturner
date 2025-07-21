// src/ui.rs

use std::{
    io::{stdout, Write},
    process::{self, Command},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use std::collections::VecDeque;

use chrono::{
    DateTime, Local, Timelike, Utc,
    Duration as ChronoDuration,
};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use get_if_addrs::get_if_addrs;
use crate::sync_logic::LtcState;

/// Check if Chrony is active
fn ntp_service_active() -> bool {
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

pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    offset: Arc<Mutex<i64>>,
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
        // 1️⃣ hardware offset
        let hw_offset_ms = *offset.lock().unwrap();

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

                    // Δ via UTC lane
                    let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                    let ltc_arrival = frame.timestamp
                        + ChronoDuration::milliseconds(hw_offset_ms + sub_ms);
                    let delta_ms = (Utc::now() - ltc_arrival).num_milliseconds();
                    st.record_clock_delta(delta_ms);
                } else {
                    st.clear_offsets();
                    st.clear_clock_deltas();
                }
            }
        }

        // 4️⃣ averages & status override
        let (avg_ms, _avg_frames, _, lock_ratio, avg_delta) = {
            let st = state.lock().unwrap();
            (
                st.average_jitter(),
                st.average_frames(),
                st.timecode_match().to_string(),
                st.lock_ratio(),
                st.average_clock_delta(),
            )
        };

        let sync_status = if avg_delta.abs() <= 5 { "IN SYNC" } else { "OUT OF SYNC" };

        // 5️⃣ cache Δ once/sec
        if last_delta_update.elapsed() >= Duration::from_secs(1) {
            cached_delta_ms = avg_delta;
            cached_delta_frames = avg_ms; // or recalc from frame if you like
            last_delta_update = Instant::now();
        }

        // 6️⃣ auto‑sync
        if sync_status == "OUT OF SYNC" {
            if let Some(start) = out_of_sync_since {
                if start.elapsed() >= Duration::from_secs(5) {
                    // sync exactly to LTC arrival
                    if let Some(frame) = &state.lock().unwrap().latest {
                        let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                        let ltc_arrival = frame.timestamp
                            + ChronoDuration::milliseconds(hw_offset_ms + sub_ms);
                        // format from UTC instant into local time
                        let ts_local: DateTime<Local> =
                            DateTime::from(ltc_arrival);
                        let ts = format!(
                            "{:02}:{:02}:{:02}.{:03}",
                            ts_local.hour(),
                            ts_local.minute(),
                            ts_local.second(),
                            ts_local.timestamp_subsec_millis()
                        );
                        let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
                        let entry = if res.as_ref().map_or(false, |s| s.success()) {
                            format!("🔄 Auto‑synced to LTC: {}", ts)
                        } else {
                            "❌ Auto‑sync failed".into()
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

        // 7️⃣ header
        queue!(
            stdout,
            MoveTo(0, 0), Clear(ClearType::All),
            MoveTo(2, 1), Print("Have Blue - NTP Timeturner"),
            MoveTo(2, 2), Print(format!("Serial Port      : {}", serial_port)),
            MoveTo(2, 3), Print(format!("Chrony Service   : {}", if ntp_active { "RUNNING" } else { "MISSING" })),
            MoveTo(2, 4), Print(format!("Interfaces       : {}", interfaces.join(", "))),
        ).unwrap();

        // 8️⃣ LTC & system clock
        if let Some(frame) = &state.lock().unwrap().latest {
            queue!(
                stdout,
                MoveTo(2, 6), Print(format!("LTC Status       : {}", frame.status)),
                MoveTo(2, 7), Print(format!("LTC Timecode     : {:02}:{:02}:{:02}:{:02}",
                    frame.hours, frame.minutes, frame.seconds, frame.frames
                )),
                MoveTo(2, 8), Print(format!("Frame Rate       : {:.2}fps", frame.frame_rate)),
            ).unwrap();
        } else {
            queue!(
                stdout,
                MoveTo(2, 6), Print("LTC Status       : (waiting)"),
                MoveTo(2, 7), Print("LTC Timecode     : …"),
                MoveTo(2, 8), Print("Frame Rate       : …"),
            ).unwrap();
        }

        // show Pi’s own clock
        let now_local: DateTime<Local> = DateTime::from(Utc::now());
        let sys_ts = format!(
            "{:02}:{:02}:{:02}.{:03}",
            now_local.hour(),
            now_local.minute(),
            now_local.second(),
            now_local.timestamp_subsec_millis()
        );
        queue!(stdout, MoveTo(2, 9), Print(format!("System Clock     : {}", sys_ts))).unwrap();

        // 9️⃣ metrics
        let dcol = if cached_delta_ms.abs() < 20 { Color::Green }
            else if cached_delta_ms.abs() < 100 { Color::Yellow }
            else { Color::Red };
        queue!(
            stdout,
            MoveTo(2, 11), SetForegroundColor(dcol),
            Print(format!("Timecode Δ       : {:+} ms ({:+} frames)", cached_delta_ms, cached_delta_frames)),
            ResetColor,
        ).unwrap();

        let scol = if sync_status == "IN SYNC" { Color::Green } else { Color::Red };
        queue!(
            stdout,
            MoveTo(2, 12), SetForegroundColor(scol),
            Print(format!("Sync Status      : {}", sync_status)),
            ResetColor,
        ).unwrap();

        let jstatus = if avg_ms.abs() < 10 { "GOOD" }
            else if avg_ms.abs() < 40 { "AVERAGE" }
            else { "BAD" };
        let jcol = if jstatus == "GOOD" { Color::Green }
            else if jstatus == "AVERAGE" { Color::Yellow }
            else { Color::Red };
        queue!(
            stdout,
            MoveTo(2, 13), SetForegroundColor(jcol),
            Print(format!("Sync Jitter      : {}", jstatus)),
            ResetColor,
        ).unwrap();

        queue!(
            stdout,
            MoveTo(2, 14), Print(format!("Lock Ratio       : {:.1}% LOCK", lock_ratio)),
        ).unwrap();

        // 10️⃣ footer + logs
        queue!(
            stdout,
            MoveTo(2, 16), Print("[S] Sync sys clock to LTC    [Q] Quit"),
        ).unwrap();
        for (i, msg) in logs.iter().enumerate() {
            queue!(stdout, MoveTo(2, 18 + i as u16), Print(msg)).unwrap();
        }

        stdout.flush().unwrap();

        // 11️⃣ manual sync & quit
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
                            // compute the exact timestamp again
                            let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                            let ltc_arrival = frame.timestamp
                                + ChronoDuration::milliseconds(hw_offset_ms + sub_ms);
                            let ts_local: DateTime<Local> = DateTime::from(ltc_arrival);
                            let ts = format!(
                                "{:02}:{:02}:{:02}.{:03}",
                                ts_local.hour(),
                                ts_local.minute(),
                                ts_local.second(),
                                ts_local.timestamp_subsec_millis()
                            );
                            let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
                            let entry = if res.as_ref().map_or(false, |s| s.success()) {
                                format!("✔ Synced exactly to LTC: {}", ts)
                            } else {
                                "❌ date cmd failed".into()
                            };
                            if logs.len() == 10 { logs.pop_front() }
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
