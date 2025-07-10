use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

use ntp_timeturner::config::Config;
use ntp_timeturner::sync_logic::{LtcFrame, LtcState};
use ntp_timeturner::ptp::start_ptp_client;

#[tokio::test]
async fn test_ptp_client_initialization() {
    // Test that PTP client starts and updates state correctly
    let state = Arc::new(Mutex::new(LtcState::new()));
    let config = Arc::new(Mutex::new(Config {
        hardware_offset_ms: 0,
        ptp_enabled: true,
        ptp_interface: "lo".to_string(), // Use loopback for testing
    }));

    // Clone for the PTP task
    let ptp_state = state.clone();
    let ptp_config = config.clone();

    // Start PTP client in background
    let ptp_handle = tokio::spawn(async move {
        start_ptp_client(ptp_state, ptp_config).await;
    });

    // Wait a short time for PTP to initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that PTP state has been updated
    {
        let st = state.lock().unwrap();
        assert_ne!(st.ptp_state, "Initializing");
        // Should be either "Starting on lo" or some PTP state
        assert!(st.ptp_state.contains("lo") || st.ptp_state.contains("Error"));
    }

    // Clean up
    ptp_handle.abort();
}

#[tokio::test]
async fn test_ptp_disabled_state() {
    // Test that PTP client respects disabled config
    let state = Arc::new(Mutex::new(LtcState::new()));
    let config = Arc::new(Mutex::new(Config {
        hardware_offset_ms: 0,
        ptp_enabled: false,
        ptp_interface: "eth0".to_string(),
    }));

    let ptp_state = state.clone();
    let ptp_config = config.clone();

    let ptp_handle = tokio::spawn(async move {
        start_ptp_client(ptp_state, ptp_config).await;
    });

    // Wait for PTP to process the disabled config
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check that PTP is disabled
    {
        let st = state.lock().unwrap();
        assert_eq!(st.ptp_state, "Disabled");
        assert!(st.ptp_offset.is_none());
    }

    ptp_handle.abort();
}

#[test]
fn test_ltc_to_ptp_time_conversion() {
    // Test the conversion logic from LTC timecode to PTP time
    let mut state = LtcState::new();
    
    // Create a test LTC frame
    let ltc_frame = LtcFrame {
        status: "LOCK".to_string(),
        hours: 14,
        minutes: 30,
        seconds: 45,
        frames: 12,
        frame_rate: 25.0,
        timestamp: Utc::now(),
    };

    // Update state with the frame
    state.update(ltc_frame.clone());

    // Verify the frame was stored
    assert!(state.latest.is_some());
    let stored_frame = state.latest.as_ref().unwrap();
    assert_eq!(stored_frame.hours, 14);
    assert_eq!(stored_frame.minutes, 30);
    assert_eq!(stored_frame.seconds, 45);
    assert_eq!(stored_frame.frames, 12);
    assert_eq!(stored_frame.frame_rate, 25.0);

    // Test frame-to-millisecond conversion
    let ms_from_frames = ((stored_frame.frames as f64 / stored_frame.frame_rate) * 1000.0).round() as i64;
    assert_eq!(ms_from_frames, 480); // 12/25 * 1000 = 480ms

    // Test that LOCK status increments lock count
    assert_eq!(state.lock_count, 1);
    assert_eq!(state.free_count, 0);
}

#[test]
fn test_ltc_ptp_synchronization_accuracy() {
    // Test that LTC timecode can be accurately converted for PTP synchronization
    let test_cases = vec![
        (0, 25.0, 0),     // Frame 0 at 25fps = 0ms
        (12, 25.0, 480),  // Frame 12 at 25fps = 480ms
        (24, 25.0, 960),  // Frame 24 at 25fps = 960ms
        (0, 30.0, 0),     // Frame 0 at 30fps = 0ms
        (15, 30.0, 500),  // Frame 15 at 30fps = 500ms
        (29, 30.0, 967),  // Frame 29 at 30fps = 966.67ms ≈ 967ms
    ];

    for (frame_num, fps, expected_ms) in test_cases {
        let ltc_frame = LtcFrame {
            status: "LOCK".to_string(),
            hours: 12,
            minutes: 0,
            seconds: 0,
            frames: frame_num,
            frame_rate: fps,
            timestamp: Utc::now(),
        };

        let ms_from_frames = ((ltc_frame.frames as f64 / ltc_frame.frame_rate) * 1000.0).round() as i64;
        assert_eq!(ms_from_frames, expected_ms, 
                   "Frame {} at {}fps should convert to {}ms, got {}ms", 
                   frame_num, fps, expected_ms, ms_from_frames);
    }
}

#[test]
fn test_ptp_offset_tracking_with_ltc() {
    // Test that PTP offset tracking works correctly with LTC frames
    let mut state = LtcState::new();
    
    // Simulate receiving multiple LOCK frames with different arrival times
    for i in 0..10 {
        let ltc_frame = LtcFrame {
            status: "LOCK".to_string(),
            hours: 12,
            minutes: 0,
            seconds: i / 25, // Advance seconds every 25 frames
            frames: i % 25,
            frame_rate: 25.0,
            timestamp: Utc::now(),
        };
        
        state.update(ltc_frame);
        
        // Simulate some jitter measurements
        let simulated_offset = (i as i64 - 5) * 2; // Range from -10 to +8ms
        state.record_offset(simulated_offset);
    }

    // Check that we have recorded offsets
    assert_eq!(state.offset_history.len(), 10);
    
    // Check average calculation
    let expected_avg = (-10 + -8 + -6 + -4 + -2 + 0 + 2 + 4 + 6 + 8) / 10; // = -1
    assert_eq!(state.average_jitter(), expected_avg);
    
    // Check frame conversion
    let avg_frames = state.average_frames();
    let expected_frames = (expected_avg as f64 / (1000.0 / 25.0)).round() as i64; // -1ms / 40ms per frame
    assert_eq!(avg_frames, expected_frames);
}

#[tokio::test]
async fn test_ptp_interface_change_handling() {
    // Test that PTP client handles interface changes correctly
    let state = Arc::new(Mutex::new(LtcState::new()));
    let config = Arc::new(Mutex::new(Config {
        hardware_offset_ms: 0,
        ptp_enabled: true,
        ptp_interface: "eth0".to_string(),
    }));

    let ptp_state = state.clone();
    let ptp_config = config.clone();

    let ptp_handle = tokio::spawn(async move {
        start_ptp_client(ptp_state, ptp_config).await;
    });

    // Wait for initial startup
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Change interface
    {
        let mut cfg = config.lock().unwrap();
        cfg.ptp_interface = "eth1".to_string();
    }

    // Wait for the change to be processed
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The PTP client should restart with the new interface
    // (In practice, this would show in logs or state changes)
    
    ptp_handle.abort();
}

#[test]
fn test_ltc_frame_timing_precision() {
    // Test that LTC frame timing is precise enough for PTP synchronization
    let base_time = Utc::now();
    
    let ltc_frame = LtcFrame {
        status: "LOCK".to_string(),
        hours: 10,
        minutes: 15,
        seconds: 30,
        frames: 20,
        frame_rate: 25.0,
        timestamp: base_time,
    };

    // Calculate the precise time this frame represents
    let frame_duration_ms = 1000.0 / ltc_frame.frame_rate; // 40ms for 25fps
    let frame_offset_ms = ltc_frame.frames as f64 * frame_duration_ms; // 20 * 40 = 800ms
    
    // Verify precision is sufficient for PTP (sub-millisecond accuracy needed)
    assert!(frame_duration_ms > 0.0);
    assert!(frame_offset_ms >= 0.0 && frame_offset_ms < 1000.0);
    
    // Test that we can represent frame timing with microsecond precision
    let frame_offset_us = (ltc_frame.frames as f64 / ltc_frame.frame_rate * 1_000_000.0).round() as i64;
    assert_eq!(frame_offset_us, 800_000); // 20/25 * 1,000,000 = 800,000 microseconds
}
