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
use std::collections::VecDeque;

/// Launch the TUI; reads `offset` live from the file-watcher and performs auto-sync if out of sync.
pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    offset: Arc<Mutex<i64>>,
) {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    terminal::enable_raw_mode().unwrap();

    // Recent log of messages (last 10)
    let mut logs: VecDeque<String> = VecDeque::with_capacity(10);
    let mut out_of_sync_since: Option<Instant> = None;

    // Delta display cache: update once per second
    let mut last_delta_update = Instant::now() - Duration::from_secs(1);
    let mut cached_delta_ms: i64 = 0;
    let mut cached_delta_frames: i64 = 0;

    loop {
        // 1️⃣ Read hardware offset
        let hw_offset_ms = *offset.lock().unwrap();

        // 2️⃣ Measure & record jitter and Timecode Δ when LOCKED; clear both on FREE
        {
            let mut st = state.lock().unwrap();
            if let Some(frame) = st.latest.clone() {
                if frame.status == "LOCK" {
                    let measured = (Utc::now() - frame.timestamp).num_milliseconds() - hw_offset_ms;
                    st.record_offset(measured);
                    let local = Local::now();
                    let sub_ms = ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                    let base_time = NaiveTime::from_hms_opt(
                        frame.hours,
                        frame.minutes,
                        frame.seconds,
                    )
                    .unwrap_or(local.time());
                    let offset_dt = local.date_naive().and_time(base_time)
                        + ChronoDuration::milliseconds(sub_ms);
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

        // Update cached delta once per second
        if last_delta_update.elapsed() >= Duration::from_secs(1) {
            cached_delta_ms = avg_delta;
            // compute frames from ms
            if let Ok(st) = state.lock() {
                if let Some(frame) = &st.latest {
                    let ms_pf = 1000.0 / frame.frame_rate;
                    cached_delta_frames = (cached_delta_ms as f64 / ms_pf).round() as i64;
                }
            }
            last_delta_update = Instant::now();
        }

        // Auto-sync: if OUT OF SYNC or Δ >10ms for 5s
        if status == "OUT OF SYNC" || cached_delta_ms.abs() > 10 {
            if let Some(start) = out_of_sync_since {
                if start.elapsed() >= Duration::from_secs(5) {
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
                            let ts = format!("{:02}:{:02}:{:02}.{:03}",
                                ltc_dt.hour(), ltc_dt.minute(), ltc_dt.second(), ltc_dt.timestamp_subsec_millis());
                            let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
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

        // 4️⃣ Draw static UI
        queue!(
            stdout,
            MoveTo(0, 0), Clear(ClearType::All),
            MoveTo(2, 1), Print("NTP Timeturner v2 - Rust Port"),
            MoveTo(2, 2), Print(format!("Using Serial Port: {}", serial_port)),
        )
        .unwrap();

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
                )
                .unwrap();
            } else {
                queue!(
                    stdout,
                    MoveTo(2, 4), Print("LTC Status   : (waiting)"),
                    MoveTo(2, 5), Print("LTC Timecode : …"),
                    MoveTo(2, 6), Print("Frame Rate   : …"),
                )
                .unwrap();
            }
            let now_local = Local::now();
            let sys_str = format!("{:02}:{:02}:{:02}.{:03}",
                now_local.hour(), now_local.minute(), now_local.second(), now_local.timestamp_subsec_millis());
            queue!(stdout, MoveTo(2, 7), Print(format!("System Clock : {}", sys_str))).unwrap();
        }

        // 6️⃣ Overlay in new order
        let dcol = if cached_delta_ms.abs() < 20 {
            Color::Green
        } else if cached_delta_ms.abs() < 100 {
            Color::Yellow
        } else {
            Color::Red
        };
        queue!(
            stdout,
            MoveTo(2, 8), SetForegroundColor(dcol),
            Print(format!("Timecode Δ   : {:+} ms ({:+} frames)", cached_delta_ms, cached_delta_frames)),
            ResetColor,
        )
        .unwrap();

        let scol = if status == "IN SYNC" { Color::Green } else { Color::Red };
        queue!(
            stdout,
            MoveTo(2, 9), SetForegroundColor(scol), Print(format!("Sync Status  : {}", status)), ResetColor,
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
            MoveTo(2, 10), SetForegroundColor(jcol), Print(format!("Sync Jitter  : {}", jstatus)), ResetColor,
        )
        .unwrap();

        queue!(
            stdout,
            MoveTo(2, 11), Print(format!("Lock Ratio   : {:.1}% LOCK", ratio)),
        )
        .unwrap();

        // Footer
        queue!(
            stdout,
            MoveTo(2, 13), Print("[S] Set system clock to LTC    [Q] Quit"),
        )
        .unwrap();

        // 7️⃣ Recent logs
        for (i, log_msg) in logs.iter().enumerate() {
            queue!(stdout, MoveTo(2, 15 + i as u16), Print(log_msg)).unwrap();
        }

        stdout.flush().unwrap();

        // 8️⃣ Handle manual sync/quit
        if poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(evt) = read().unwrap() {
                if let KeyCode::Char(c) = evt.code {
                    if c.eq_ignore_ascii_case(&'q') {
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();
                        terminal::disable_raw_mode().unwrap();
                        process::exit(0);
                    }
                    if c.eq_ignore_ascii_case(&'s') {
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
                                let ts = format!("{:02}:{:02}:{:02}.{:03}",
                                    ltc_dt.hour(), ltc_dt.minute(), ltc_dt.second(), ltc_dt.timestamp_subsec_millis());
                                let res = Command::new("sudo").arg("date").arg("-s").arg(&ts).status();
                                let msg = if res.as_ref().map_or(false, |s| s.success()) {
                                    format!("✔ Synced exactly to LTC: {}", ts)
                                } else {
                                    "❌ date cmd failed".into()
                                };
                                if logs.len() == 10 { logs.pop_front(); }
                                logs.push_back(msg);
                            }
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}
