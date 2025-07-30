use crate::config::Config;
use chrono::{DateTime, Local, Timelike, Utc};
use regex::Captures;
use std::collections::VecDeque;

const EWMA_ALPHA: f64 = 0.1;

#[derive(Clone, Debug)]
pub struct LtcFrame {
    pub status: String,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub frames: u32,
    pub frame_rate: f64,
    pub timestamp: DateTime<Utc>, // arrival stamp
}

impl LtcFrame {
    pub fn from_regex(caps: &Captures, timestamp: DateTime<Utc>) -> Option<Self> {
        Some(Self {
            status: caps[1].to_string(),
            hours: caps[2].parse().ok()?,
            minutes: caps[3].parse().ok()?,
            seconds: caps[4].parse().ok()?,
            frames: caps[5].parse().ok()?,
            frame_rate: caps[6].parse().ok()?,
            timestamp,
        })
    }

    /// Compare just HH:MM:SS against local time.
    pub fn matches_system_time(&self) -> bool {
        let local = Local::now();
        local.hour() == self.hours
            && local.minute() == self.minutes
            && local.second() == self.seconds
    }
}

pub struct LtcState {
    pub latest: Option<LtcFrame>,
    pub lock_count: u32,
    pub free_count: u32,
    /// Stores the last up-to-20 raw offset measurements in ms.
    pub offset_history: VecDeque<i64>,
    /// EWMA of clock delta.
    pub ewma_clock_delta: Option<f64>,
    pub last_match_status: String,
    pub last_match_check: i64,
}

impl LtcState {
    pub fn new() -> Self {
        Self {
            latest: None,
            lock_count: 0,
            free_count: 0,
            offset_history: VecDeque::with_capacity(20),
            ewma_clock_delta: None,
            last_match_status: "UNKNOWN".into(),
            last_match_check: 0,
        }
    }

    /// Record one measured jitter offset in ms.
    pub fn record_offset(&mut self, offset_ms: i64) {
        if self.offset_history.len() == 20 {
            self.offset_history.pop_front();
        }
        self.offset_history.push_back(offset_ms);
    }

    /// Update EWMA of clock delta.
    pub fn record_and_update_ewma_clock_delta(&mut self, delta_ms: i64) {
        let new_delta = delta_ms as f64;
        if let Some(current_ewma) = self.ewma_clock_delta {
            self.ewma_clock_delta = Some(EWMA_ALPHA * new_delta + (1.0 - EWMA_ALPHA) * current_ewma);
        } else {
            self.ewma_clock_delta = Some(new_delta);
        }
    }

    /// Clear all stored jitter measurements.
    pub fn clear_offsets(&mut self) {
        self.offset_history.clear();
    }

    /// Update LOCK/FREE counts and timecode-match status every 5 s.
    pub fn update(&mut self, frame: LtcFrame) {
        match frame.status.as_str() {
            "LOCK" => {
                self.lock_count += 1;

                // Recompute timecode-match every 5 seconds
                let now_secs = Utc::now().timestamp();
                if now_secs - self.last_match_check >= 5 {
                    self.last_match_status = if frame.matches_system_time() {
                        "IN SYNC"
                    } else {
                        "OUT OF SYNC"
                    }
                    .into();
                    self.last_match_check = now_secs;
                }
            }
            "FREE" => {
                self.free_count += 1;
                self.clear_offsets();
                self.ewma_clock_delta = None;
                self.last_match_status = "UNKNOWN".into();
            }
            _ => {}
        }

        self.latest = Some(frame);
    }

    /// Average jitter over stored history, in ms.
    pub fn average_jitter(&self) -> i64 {
        if self.offset_history.is_empty() {
            0
        } else {
            let sum: i64 = self.offset_history.iter().sum();
            sum / self.offset_history.len() as i64
        }
    }

    /// Convert average jitter into frames (rounded).
    pub fn average_frames(&self) -> i64 {
        if let Some(frame) = &self.latest {
            let ms_per_frame = 1000.0 / frame.frame_rate;
            (self.average_jitter() as f64 / ms_per_frame).round() as i64
        } else {
            0
        }
    }

    /// Get EWMA of clock delta, in ms.
    pub fn get_ewma_clock_delta(&self) -> i64 {
        self.ewma_clock_delta.map_or(0, |v| v.round() as i64)
    }

    /// Percentage of samples seen in LOCK state versus total.
    pub fn lock_ratio(&self) -> f64 {
        let total = self.lock_count + self.free_count;
        if total == 0 {
            0.0
        } else {
            self.lock_count as f64 / total as f64 * 100.0
        }
    }

    /// Get timecode-match status.
    pub fn timecode_match(&self) -> &str {
        &self.last_match_status
    }
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
// This module provides the logic for handling LTC (Linear Timecode) frames and maintaining state.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, TimeturnerOffset};
    use chrono::{Local, Utc};

    fn get_test_frame(status: &str, h: u32, m: u32, s: u32) -> LtcFrame {
        LtcFrame {
            status: status.to_string(),
            hours: h,
            minutes: m,
            seconds: s,
            frames: 0,
            frame_rate: 25.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_ltc_frame_matches_system_time() {
        let now = Local::now();
        let frame = get_test_frame("LOCK", now.hour(), now.minute(), now.second());
        assert!(frame.matches_system_time());
    }

    #[test]
    fn test_ltc_frame_does_not_match_system_time() {
        let now = Local::now();
        // Create a time that is one hour ahead, wrapping around 23:00
        let different_hour = (now.hour() + 1) % 24;
        let frame = get_test_frame("LOCK", different_hour, now.minute(), now.second());
        assert!(!frame.matches_system_time());
    }

    #[test]
    fn test_ltc_state_update_lock() {
        let mut state = LtcState::new();
        let frame = get_test_frame("LOCK", 10, 20, 30);
        state.update(frame);
        assert_eq!(state.lock_count, 1);
        assert_eq!(state.free_count, 0);
        assert!(state.latest.is_some());
    }

    #[test]
    fn test_ltc_state_update_free() {
        let mut state = LtcState::new();
        state.record_offset(100);
        assert!(!state.offset_history.is_empty());

        let frame = get_test_frame("FREE", 10, 20, 30);
        state.update(frame);
        assert_eq!(state.lock_count, 0);
        assert_eq!(state.free_count, 1);
        assert!(state.offset_history.is_empty()); // Offsets should be cleared
        assert_eq!(state.last_match_status, "UNKNOWN");
    }

    #[test]
    fn test_offset_history_management() {
        let mut state = LtcState::new();
        for i in 0..25 {
            state.record_offset(i);
        }
        assert_eq!(state.offset_history.len(), 20);
        assert_eq!(*state.offset_history.front().unwrap(), 5); // 0-4 are pushed out
        assert_eq!(*state.offset_history.back().unwrap(), 24);
    }

    #[test]
    fn test_timecode_match_status_in_sync() {
        let mut state = LtcState::new();
        state.last_match_check = 0; // Force check to run

        let now = Local::now();
        let frame_in_sync = get_test_frame("LOCK", now.hour(), now.minute(), now.second());
        state.update(frame_in_sync);
        assert_eq!(state.timecode_match(), "IN SYNC");
    }

    #[test]
    fn test_timecode_match_status_out_of_sync() {
        let mut state = LtcState::new();
        state.last_match_check = 0; // Force check to run

        let now = Local::now();
        let different_hour = (now.hour() + 1) % 24;
        let frame_out_of_sync = get_test_frame("LOCK", different_hour, now.minute(), now.second());
        state.update(frame_out_of_sync);

        assert_eq!(state.timecode_match(), "OUT OF SYNC");
    }

    #[test]
    fn test_timecode_match_throttling() {
        let mut state = LtcState::new();
        let now = Local::now();

        // First call. With the bug, status becomes UNKNOWN. With fix, OUT OF SYNC.
        // The test is written for the fixed behavior.
        state.last_match_check = 0;
        let frame_out_of_sync =
            get_test_frame("LOCK", (now.hour() + 1) % 24, now.minute(), now.second());
        state.update(frame_out_of_sync.clone());
        assert_eq!(
            state.timecode_match(),
            "OUT OF SYNC",
            "Initial status should be out of sync"
        );

        // Second call, immediately. Check should be throttled.
        // Status should not change, even though we pass an in-sync frame.
        let frame_in_sync = get_test_frame("LOCK", now.hour(), now.minute(), now.second());
        state.update(frame_in_sync.clone());
        assert_eq!(
            state.timecode_match(),
            "OUT OF SYNC",
            "Status should not change due to throttling"
        );

        // Third call, forcing check to run again.
        // Status should now update to IN SYNC.
        state.last_match_check = 0;
        state.update(frame_in_sync.clone());
        assert_eq!(
            state.timecode_match(),
            "IN SYNC",
            "Status should update after throttle period"
        );
    }

    #[test]
    fn test_ewma_clock_delta() {
        let mut state = LtcState::new();
        assert_eq!(state.get_ewma_clock_delta(), 0);

        // First value initializes the EWMA
        state.record_and_update_ewma_clock_delta(100);
        assert_eq!(state.get_ewma_clock_delta(), 100);

        // Second value moves it
        state.record_and_update_ewma_clock_delta(200);
        // 0.1 * 200 + 0.9 * 100 = 20 + 90 = 110
        assert_eq!(state.get_ewma_clock_delta(), 110);

        // Third value
        state.record_and_update_ewma_clock_delta(100);
        // 0.1 * 100 + 0.9 * 110 = 10 + 99 = 109
        assert_eq!(state.get_ewma_clock_delta(), 109);

        // Reset on FREE frame
        state.update(get_test_frame("FREE", 0, 0, 0));
        assert_eq!(state.get_ewma_clock_delta(), 0);
        assert!(state.ewma_clock_delta.is_none());
    }

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

        // Test auto-sync status
        // config.auto_sync_enabled = true;
        // assert_eq!(get_sync_status(0, &config), "IN SYNC");

        // Test TIMETURNING status takes precedence
        config.timeturner_offset = TimeturnerOffset { hours: 1, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 };
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
