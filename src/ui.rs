// src/ui.rs

use std::{
    io::{stdout, Write},
    process::{self, Command},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use chrono::{Local, Timelike, Utc};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::sync_logic::LtcState;

/// Launch the TUI; reads `offset` live from the file-watcher.
pub fn start_ui(
    state: Arc<Mutex<LtcState>>,
    serial_port: String,
    offset: Arc<Mutex<i64>>,
) {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    terminal::enable_raw_mode().unwrap();

    loop {
        // 1️⃣ Read current hardware offset
        let hw_offset_ms = *offset.lock().unwrap();

        // 2️⃣ Measure & record jitter only when LOCKED; clear on FREE
        {
            let mut st = state.lock().unwrap();
            if let Some(frame) = &st.latest {
                if frame.status == "LOCK" {
                    let now = Utc::now();
                    let raw = (now - frame.timestamp).num_milliseconds();
                    let measured = raw - hw_offset_ms;
                    st.record_offset(measured);
                } else {
                    st.clear_offsets();
                }
            }
        }

        // 3️⃣ Draw static UI
        queue!(
            stdout,
            MoveTo(0, 0),
            Clear(ClearType::All),
            Hide,
            MoveTo(2, 1), Print("NTP Timeturner v2 - Rust Port"),
            MoveTo(2, 2), Print(format!("Using Serial Port: {}", serial_port)),
        )
        .unwrap();

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
            let sys_str = format!(
                "{:02}:{:02}:{:02}.{:03}",
                now_local.hour(),
                now_local.minute(),
                now_local.second(),
                now_local.timestamp_subsec_millis()
            );
            queue!(
                stdout,
                MoveTo(2, 7),
                Print(format!("System Clock : {}", sys_str))
            )
            .unwrap();
        }

        // Footer
        queue!(
            stdout,
            MoveTo(2, 12),
            Print("[S] Set system clock to LTC    [Q] Quit")
        )
        .unwrap();

        stdout.flush().unwrap();

        // 4️⃣ Overlay Sync Jitter / Status / Ratio
        if let Ok(st) = state.lock() {
            let avg_ms     = st.average_jitter();
            let avg_frames = st.average_frames();
            let (jcol, jtxt) = if avg_ms.abs() < 10 {
                (Color::Green, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
            } else if avg_ms.abs() < 40 {
                (Color::Yellow, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
            } else {
                (Color::Red, format!("{:+} ms ({:+} frames)", avg_ms, avg_frames))
            };
            queue!(
                stdout,
                MoveTo(2, 8),
                SetForegroundColor(jcol),
                Print("Sync Jitter  : "),
                Print(jtxt),
                ResetColor,
            )
            .ok();

            let status = st.timecode_match();
            let scol = if status == "IN SYNC" { Color::Green } else { Color::Red };
            queue!(
                stdout,
                MoveTo(2, 9),
                SetForegroundColor(scol),
                Print(format!("Sync Status  : {}", status)),
                ResetColor,
            )
            .ok();

            let ratio = st.lock_ratio();
            queue!(
                stdout,
                MoveTo(2, 10),
                Print(format!("Lock Ratio   : {:.1}% LOCK", ratio)),
            )
            .ok();

            stdout.flush().ok();
        }

        // 5️⃣ Handle keypress
        if poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(evt) = read().unwrap() {
                match evt.code {
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'s') => {
                        // SYNC now
                        if let Ok(st) = state.lock() {
                            if let Some(frame) = &st.latest {
                                // compute ms from frames
                                let ms_from_frames =
                                    ((frame.frames as f64 / frame.frame_rate) * 1000.0).round() as i64;
                                // total microseconds
                                let total_us = (ms_from_frames + hw_offset_ms) * 1000;
                                // build date string "HH:MM:SS.mmm"
                                let ts = format!(
                                    "{:02}:{:02}:{:02}.{:03}",
                                    frame.hours,
                                    frame.minutes,
                                    frame.seconds,
                                    ((total_us / 1000) % 1000)
                                );
                                // run `sudo date -s "HH:MM:SS.mmm"`
                                let status = Command::new("sudo")
                                    .arg("date")
                                    .arg("-s")
                                    .arg(&ts)
                                    .status();
                                let msg = if let Ok(s) = status {
                                    if s.success() {
                                        format!("✔ Synced to LTC: {}", ts)
                                    } else {
                                        format!("❌ date cmd failed")
                                    }
                                } else {
                                    format!("❌ failed to spawn date")
                                };
                                // print confirmation at row 14
                                queue!(
                                    stdout,
                                    MoveTo(2, 14),
                                    Print(msg),
                                )
                                .ok();
                                stdout.flush().ok();
                            }
                        }
                    }
                    KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => {
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();
                        terminal::disable_raw_mode().unwrap();
                        process::exit(0);
                    }
                    _ => {}
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}
