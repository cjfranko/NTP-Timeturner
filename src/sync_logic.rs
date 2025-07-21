use chrono::{DateTime, Local, Timelike, Utc};
use regex::Captures;
use std::collections::VecDeque;

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
    /// Stores the last up-to-20 timecode Δ measurements in ms.
    pub clock_delta_history: VecDeque<i64>,
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
            clock_delta_history: VecDeque::with_capacity(20),
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

    /// Record one timecode Δ in ms.
    pub fn record_clock_delta(&mut self, delta_ms: i64) {
        if self.clock_delta_history.len() == 20 {
            self.clock_delta_history.pop_front();
        }
        self.clock_delta_history.push_back(delta_ms);
    }

    /// Clear all stored jitter measurements.
    pub fn clear_offsets(&mut self) {
        self.offset_history.clear();
    }

    /// Clear all stored timecode Δ measurements.
    pub fn clear_clock_deltas(&mut self) {
        self.clock_delta_history.clear();
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
                self.clear_clock_deltas();
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

    /// Median timecode Δ over stored history, in ms.
    pub fn average_clock_delta(&self) -> i64 {
        if self.clock_delta_history.is_empty() {
            return 0;
        }

        let mut sorted_deltas: Vec<i64> = self.clock_delta_history.iter().cloned().collect();
        sorted_deltas.sort_unstable();

        let mid = sorted_deltas.len() / 2;
        if sorted_deltas.len() % 2 == 0 {
            // Even number of elements, average the two middle ones
            (sorted_deltas[mid - 1] + sorted_deltas[mid]) / 2
        } else {
            // Odd number of elements, return the middle one
            sorted_deltas[mid]
        }
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
// This module provides the logic for handling LTC (Linear Timecode) frames and maintaining state.
#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_average_clock_delta_is_median() {
        let mut state = LtcState::new();

        // Establish a stable set of values
        for _ in 0..19 {
            state.record_clock_delta(2);
        }
        state.record_clock_delta(100); // Add an outlier

        // With 19 `2`s and one `100`, the median should still be `2`.
        // The simple average would be (19*2 + 100) / 20 = 138 / 20 = 6.
        assert_eq!(
            state.average_clock_delta(),
            2,
            "Median should ignore the outlier"
        );

        // Test with an even number of elements
        state.clear_clock_deltas();
        state.record_clock_delta(1);
        state.record_clock_delta(2);
        state.record_clock_delta(3);
        state.record_clock_delta(100);
        // sorted: [1, 2, 3, 100]. mid two are 2, 3. average is (2+3)/2 = 2.
        assert_eq!(
            state.average_clock_delta(),
            2,
            "Median of even numbers should be correct"
        );
    }
}
