// In this file, you can define the paths to your local icon image files.
const iconMap = {
    ltcStatus: {
        'LOCK': { src: 'assets/timeturner_ltc_green.png', tooltip: 'LTC signal is locked and stable.' },
        'FREE': { src: 'assets/timeturner_ltc_orange.png', tooltip: 'LTC signal is in freewheel mode.' },
        'default': { src: 'assets/timeturner_ltc_red.png', tooltip: 'LTC signal is not detected.' }
    },
    ntpActive: {
        true: { src: 'assets/timeturner_ntp_green.png', tooltip: 'NTP service is active.' },
        false: { src: 'assets/timeturner_ntp_red.png', tooltip: 'NTP service is inactive.' }
    },
    syncStatus: {
        'IN SYNC': { src: 'assets/timeturner_sync_green.png', tooltip: 'System clock is in sync with LTC source.' },
        'CLOCK AHEAD': { src: 'assets/timeturner_sync_orange.png', tooltip: 'System clock is ahead of the LTC source.' },
        'CLOCK BEHIND': { src: 'assets/timeturner_sync_orange.png', tooltip: 'System clock is behind the LTC source.' },
        'TIMETURNING': { src: 'assets/timeturner_timeturning.png', tooltip: 'Timeturner offset is active.' },
        'default': { src: 'assets/timeturner_sync_red.png', tooltip: 'Sync status is unknown.' }
    },
    jitterStatus: {
        'GOOD': { src: 'assets/timeturner_jitter_green.png', tooltip: 'Clock jitter is within acceptable limits.' },
        'AVERAGE': { src: 'assets/timeturner_jitter_orange.png', tooltip: 'Clock jitter is moderate.' },
        'BAD': { src: 'assets/timeturner_jitter_red.png', tooltip: 'Clock jitter is high and may affect accuracy.' },
        'default': { src: 'assets/timeturner_jitter_red.png', tooltip: 'Jitter status is unknown.' }
    },
    deltaStatus: {
        'good': { src: 'assets/timeturner_delta_green.png', tooltip: 'Clock delta is 0ms.' },
        'average': { src: 'assets/timeturner_delta_orange.png', tooltip: 'Clock delta is less than 10ms.' },
        'bad': { src: 'assets/timeturner_delta_red.png', tooltip: 'Clock delta is 10ms or greater.' }
    },
    frameRate: {
        '23.98fps': { src: 'assets/timeturner_2398.png', tooltip: '23.98 frames per second' },
        '24.00fps': { src: 'assets/timeturner_24.png', tooltip: '24.00 frames per second' },
        '25.00fps': { src: 'assets/timeturner_25.png', tooltip: '25.00 frames per second' },
        '29.97fps': { src: 'assets/timeturner_2997.png', tooltip: '29.97 frames per second' },
        '30.00fps': { src: 'assets/timeturner_30.png', tooltip: '30.00 frames per second' },
        'default': { src: 'assets/timeturner_default.png', tooltip: 'Unknown frame rate' }
    },
    lockRatio: {
        'good': { src: 'assets/timeturner_lock_green.png', tooltip: 'Lock ratio is 100%.' },
        'average': { src: 'assets/timeturner_lock_orange.png', tooltip: 'Lock ratio is 90% or higher.' },
        'bad': { src: 'assets/timeturner_lock_red.png', tooltip: 'Lock ratio is below 90%.' }
    }
};
