use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{Clear, ClearType},
    cursor::MoveTo,
};

use crate::sync_logic::LtcState;

pub fn render_ui(state: &Arc<Mutex<LtcState>>) -> std::io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    if let Ok(s) = state.lock() {
        if let Some(frame) = &s.latest {
            writeln!(stdout, "🕰️ NTP Timeturner (Rust Draft)")?;
            writeln!(stdout, "LTC Status   : {}", frame.status)?;
            writeln!(
                stdout,
                "LTC Timecode : {:02}:{:02}:{:02}:{:02}",
                frame.hours, frame.minutes, frame.seconds, frame.frames
            )?;
            writeln!(stdout, "Frame Rate   : {:.2} fps", frame.frame_rate)?;
            writeln!(stdout, "Timestamp    : {}", frame.timestamp)?;
            let total = s.lock_count + s.free_count;
            let ratio = if total > 0 {
                s.lock_count as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            writeln!(stdout, "Lock Ratio   : {:.1}% LOCK", ratio)?;
        } else {
            writeln!(stdout, "Waiting for LTC...")?;
        }
    }

    stdout.flush()?;
    Ok(())
}

pub fn start_ui(state: Arc<Mutex<LtcState>>) {
    // 🧠 This thread now DOES the rendering loop
    loop {
        if let Err(e) = render_ui(&state) {
            eprintln!("UI error: {}", e);
        }
        thread::sleep(Duration::from_millis(500));
    }
}
