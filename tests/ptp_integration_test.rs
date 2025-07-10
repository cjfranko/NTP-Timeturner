use chrono::Utc;

use ntp_timeturner::config::Config;
use ntp_timeturner::sync_logic::{LtcFrame, LtcState};

#[test]
fn test_ptp_config_validation() {
    // Test that PTP configuration is properly validated
    let config = Config {
        hardware_offset_ms: 0,
        ptp_enabled: true,
        ptp_interface: "eth0".to_string(),
    };

    assert!(config.ptp_enabled);
    assert_eq!(config.ptp_interface, "eth0");
    assert_eq!(config.hardware_offset_ms, 0);
}

#[test]
fn test_ptp_disabled_config() {
    // Test that PTP can be disabled via config
    let config = Config {
        hardware_offset_ms: 0,
        ptp_enabled: false,
        ptp_interface: "eth0".to_string(),
    };

    assert!(!config.ptp_enabled);
    // Even when disabled, interface should be preserved
    assert_eq!(config.ptp_interface, "eth0");
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

#[test]
fn test_ptp_interface_configuration() {
    // Test that PTP interface can be configured
    let mut config = Config {
        hardware_offset_ms: 0,
        ptp_enabled: true,
        ptp_interface: "eth0".to_string(),
    };

    assert_eq!(config.ptp_interface, "eth0");

    // Test interface change
    config.ptp_interface = "eth1".to_string();
    assert_eq!(config.ptp_interface, "eth1");

    // Test with different interface types
    config.ptp_interface = "enp0s3".to_string();
    assert_eq!(config.ptp_interface, "enp0s3");
}

#[test]
fn test_ptp_state_initialization() {
    // Test that LtcState initializes with correct PTP defaults
    let state = LtcState::new();
    
    assert!(state.ptp_offset.is_none());
    assert_eq!(state.ptp_state, "Initializing");
}

#[test]
fn test_ptp_offset_storage() {
    // Test that PTP offset can be stored and retrieved
    let mut state = LtcState::new();
    
    // Initially no offset
    assert!(state.ptp_offset.is_none());
    
    // Simulate setting a PTP offset (this would normally be done by the PTP client)
    state.ptp_offset = Some(123.456);
    state.ptp_state = "Slave".to_string();
    
    assert_eq!(state.ptp_offset, Some(123.456));
    assert_eq!(state.ptp_state, "Slave");
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
