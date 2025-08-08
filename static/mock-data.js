// This file contains mock data sets for UI development and testing without a live backend.
const mockApiDataSets = {
    allGood: {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '10:20:30:00',
            frame_rate: '25.00fps',
            lock_ratio: 99.5,
            system_clock: '10:20:30.500',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'IN SYNC',
            timecode_delta_ms: 5,
            timecode_delta_frames: 0.125,
            jitter_status: 'GOOD',
            interfaces: ['192.168.1.100/24 (eth0)', '10.0.0.5/8 (wlan0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 1, minutes: 2, seconds: 3, frames: 4, milliseconds: 50 },
        },
        logs: [
            '2025-08-07 10:20:30 [INFO] Starting up...',
            '2025-08-07 10:20:32 [INFO] LTC LOCK detected. Frame rate: 25.00fps.',
            '2025-08-07 10:20:35 [INFO] Initial sync complete. Clock adjusted by -15ms.',
        ]
    },
    ltcFree: {
        status: {
            ltc_status: 'FREE',
            ltc_timecode: '11:22:33:11',
            frame_rate: '25.00fps',
            lock_ratio: 40.2,
            system_clock: '11:22:33.800',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'IN SYNC',
            timecode_delta_ms: 3,
            timecode_delta_frames: 0.075,
            jitter_status: 'GOOD',
            interfaces: ['192.168.1.100/24 (eth0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 0, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 },
        },
        logs: [ '2025-08-07 11:22:30 [WARN] LTC signal lost, entering freewheel.' ]
    },
    clockAhead: {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '12:00:05:00',
            frame_rate: '25.00fps',
            lock_ratio: 98.1,
            system_clock: '12:00:04.500',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'CLOCK AHEAD',
            timecode_delta_ms: -500,
            timecode_delta_frames: -12.5,
            jitter_status: 'AVERAGE',
            interfaces: ['192.168.1.100/24 (eth0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 0, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 },
        },
        logs: [ '2025-08-07 12:00:00 [WARN] System clock is ahead of LTC source by 500ms.' ]
    },
    clockBehind: {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '13:30:10:00',
            frame_rate: '25.00fps',
            lock_ratio: 99.9,
            system_clock: '13:30:10.800',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'CLOCK BEHIND',
            timecode_delta_ms: 800,
            timecode_delta_frames: 20,
            jitter_status: 'AVERAGE',
            interfaces: ['192.168.1.100/24 (eth0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 0, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 },
        },
        logs: [ '2025-08-07 13:30:00 [WARN] System clock is behind LTC source by 800ms.' ]
    },
    timeturning: {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '14:00:00:00',
            frame_rate: '25.00fps',
            lock_ratio: 100,
            system_clock: '15:02:03.050',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'TIMETURNING',
            timecode_delta_ms: 3723050, // a big number
            timecode_delta_frames: 93076,
            jitter_status: 'GOOD',
            interfaces: ['192.168.1.100/24 (eth0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: false,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 1, minutes: 2, seconds: 3, frames: 4, milliseconds: 50 },
        },
        logs: [ '2025-08-07 14:00:00 [INFO] Timeturner offset is active.' ]
    },
    badJitter: {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '15:15:15:15',
            frame_rate: '25.00fps',
            lock_ratio: 95.0,
            system_clock: '15:15:15.515',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'IN SYNC',
            timecode_delta_ms: 10,
            timecode_delta_frames: 0.25,
            jitter_status: 'BAD',
            interfaces: ['192.168.1.100/24 (eth0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 0, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 },
        },
        logs: [ '2025-08-07 15:15:00 [ERROR] High jitter detected on LTC source.' ]
    },
    ntpInactive: {
        status: {
            ltc_status: 'UNKNOWN',
            ltc_timecode: '--:--:--:--',
            frame_rate: '--',
            lock_ratio: 0,
            system_clock: '16:00:00.000',
            system_date: '2025-08-07',
            ntp_active: false,
            sync_status: 'UNKNOWN',
            timecode_delta_ms: 0,
            timecode_delta_frames: 0,
            jitter_status: 'UNKNOWN',
            interfaces: [],
        },
        config: {
            hardwareOffsetMs: 0,
            autoSyncEnabled: false,
            defaultNudgeMs: 2,
            timeturnerOffset: { hours: 0, minutes: 0, seconds: 0, frames: 0, milliseconds: 0 },
        },
        logs: [ '2025-08-07 16:00:00 [INFO] NTP service is inactive.' ]
    }
};
