// src/ui.rs

use std::{
    io::{stdout, Write},
    process::{self, Command},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::{Local, Timelike, Utc, NaiveTime, Duration as ChronoDuration, TimeZone};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::sync_logic::LtcState;

/// Launch the TUI; reads `offset` live from the file-watcher and performs auto-sync if out of sync.
pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    offset: Arc<Mutex<i64>>,
) {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    terminal::enable_raw_mode().unwrap();

    // Track when delta goes out of threshold
    let mut out_of_sync_since: Option<Instant> = None;

    loop {
        // 1️⃣ Read hardware offset
        let hw_offset_ms = *offset.lock().unwrap();

        // 2️⃣ Measure & record jitter and Timecode Δ when LOCKED; clear both on FREE
        {
            let mut st = state.lock().unwrap();
            if let Some(frame) = st.latest.clone() {
                if frame.status == "LOCK" {
                    // Jitter measurement
                    let now = Utc::now();
                    let measured = (now - frame.timestamp).num_milliseconds() - hw_offset_ms;
                    st.record_offset(measured);
                    // Timecode Δ measurement
                    let local = Local::now();
                    let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                    let base_time = NaiveTime::from_hms_opt(
                        frame.hours,
                        frame.minutes,
                        frame.seconds,
                    ).unwrap_or(local.time());
                    let today = local.date_naive();
                    let offset_dt = today.and_time(base_time) + ChronoDuration::milliseconds(sub_ms);
                    let ltc_dt = Local.from_local_datetime(&offset_dt).single().unwrap_or(local);
                    let delta_ms = local.signed_duration_since(ltc_dt).num_milliseconds();
                    st.record_clock_delta(delta_ms);
                } else {
                    st.clear_offsets();
                    st.clear_clock_deltas();
                }
            }
        }

        // 3️⃣ Compute averages and status
        let (avg_ms, avg_frames, status, ratio, avg_delta) = {
            let st = state.lock().unwrap();
            (
                st.average_jitter(),
                st.average_frames(),
                st.timecode_match().to_string(),
                st.lock_ratio(),
                st.average_clock_delta(),
            )
        };

        // Auto-sync: if OUT OF SYNC or Δ >10ms for 5s
        if status == "OUT OF SYNC" || avg_delta.abs() > 10 {
            if let Some(start) = out_of_sync_since {
                if start.elapsed() >= Duration::from_secs(5) {
                    // perform sync
                    if let Ok(st) = state.lock() {
                        if let Some(frame) = &st.latest {
                            let local_now = Local::now();
                            let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                            let base_time = NaiveTime::from_hms_opt(
                                frame.hours,
                                frame.minutes,
                                frame.seconds,
                            ).unwrap_or(local_now.time());
                            let offset_dt = local_now.date_naive().and_time(base_time)
                                + ChronoDuration::milliseconds(sub_ms);
                            let ltc_dt = Local.from_local_datetime(&offset_dt).single().unwrap_or(local_now);
                            let ts = format!("{:02}:{:02}:{:02}.{:03}", ltc_dt.hour(), ltc_dt.minute(), ltc_dt.second(), ltc_dt.timestamp_subsec_millis());
                            let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
                            let msg = if res.as_ref().map_or(false, |s| s.success()) {
                                format!("🔄 Auto-synced to LTC: {}", ts)
                            } else {
                                "❌ Auto-sync failed".into()
                            };
                            queue!(stdout, MoveTo(2, 14), Print(msg)).unwrap();
                            stdout.flush().unwrap();
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

        // 4️⃣ Draw static UI
        queue!(
            stdout,
            MoveTo(0, 0), Clear(ClearType::All),
            MoveTo(2, 1), Print("NTP Timeturner v2 - Rust Port"),
            MoveTo(2, 2), Print(format!("Using Serial Port: {}", serial_port)),
        ).unwrap();

        // 5️⃣ Draw LTC & System Clock
        if let Ok(st) = state.lock() {
            if let Some(frame) = &st.latest {
                queue!(
                    stdout,
                    MoveTo(2, 4), Print(format!("LTC Status   : {}", frame.status)),
                    MoveTo(2, 5), Print(format!(
                        "LTC Timecode : {:02}:{:02}:{:02}:{:02}",
                        frame.hours, frame.minutes, frame.seconds, frame.frames
                    )),
                    MoveTo(2, 6), Print(format!("Frame Rate   : {:.2}fps", frame.frame_rate)),
                ).unwrap();
            } else {
                queue!(
                    stdout,
                    MoveTo(2, 4), Print("LTC Status   : (waiting)"),
                    MoveTo(2, 5), Print("LTC Timecode : …"),
                    MoveTo(2, 6), Print("Frame Rate   : …"),
                ).unwrap();
            }
            let now_local = Local::now();
            let sys_str = format!("{:02}:{:02}:{:02}.{:03}",
                now_local.hour(), now_local.minute(), now_local.second(), now_local.timestamp_subsec_millis());
            queue!(stdout, MoveTo(2, 7), Print(format!("System Clock : {}", sys_str))).unwrap();
        }

        // 6️⃣ Overlay in new order: Delta, Status, Jitter, Ratio
        // Timecode Δ below System Clock
        let dcol = if avg_delta.abs() < 20 {
            Color::Green
        } else if avg_delta.abs() < 100 {
            Color::Yellow
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 8), SetForegroundColor(dcol), Print(format!("Timecode Δ   : {:+} ms", avg_delta)), ResetColor,
        ).unwrap();

        // Sync Status
        let scol = if status == "IN SYNC" { Color::Green } else { Color::Red };
        queue!(
            stdout,
            MoveTo(2, 9), SetForegroundColor(scol), Print(format!("Sync Status  : {}", status)), ResetColor,
        ).unwrap();

        // Sync Jitter under Status
        let (jcol, jtxt) = if avg_ms.abs() < 10 {
            (Color::Green, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
        } else if avg_ms.abs() < 40 {
            (Color::Yellow, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
        } else {
            (Color::Red, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
        };
        queue!(
            stdout,
            MoveTo(2, 10), SetForegroundColor(jcol), Print("Sync Jitter  : "), Print(jtxt), ResetColor,
        ).unwrap();

        // Lock Ratio below Jitter
        queue!(
            stdout,
            MoveTo(2, 11), Print(format!("Lock Ratio   : {:.1}% LOCK", ratio)),
        ).unwrap();

        // Blank line at 12, Footer at 13
        queue!(
            stdout,
            MoveTo(2, 13), Print("[S] Set system clock to LTC    [Q] Quit"),
        ).unwrap();

        stdout.flush().unwrap();

        // 7️⃣ Handle quit/manual sync in poll
        if poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(evt) = read().unwrap() {
                if let KeyCode::Char(c) = evt.code {
                    if c.eq_ignore_ascii_case(&'q') {
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();
                        terminal::disable_raw_mode().unwrap();
                        process::exit(0);
                    }
                    if c.eq_ignore_ascii_case(&'s') {
                        // manual sync logic duplicated...
                        if let Ok(st) = state.lock() {
                            if let Some(frame) = &st.latest {
                                let local_now = Local::now();
                                let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                                let base_time = NaiveTime::from_hms_opt(
                                    frame.hours,
                                    frame.minutes,
                                    frame.seconds,
                                ).unwrap_or(local_now.time());
                                let offset_dt = local_now.date_naive().and_time(base_time) + ChronoDuration::milliseconds(sub_ms);
                                let ltc_dt = Local.from_local_datetime(&offset_dt).single().unwrap_or(local_now);
                                let ts = format!("{:02}:{:02}:{:02}.{:03}", ltc_dt.hour(), ltc_dt.minute(), ltc_dt.second(), ltc_dt.timestamp_subsec_millis());
                                let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
                                let msg = if res.as_ref().map_or(false, |s| s.success()) {
                                    format!("✔ Synced exactly to LTC: {}", ts)
                                } else {
                                    "❌ date cmd failed".into()
                                };
                                queue!(stdout, MoveTo(2, 14), Print(msg)).unwrap();
                                stdout.flush().unwrap();
                            }
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}