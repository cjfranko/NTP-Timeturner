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
    }
};
