// In this file, you can define the paths to your local icon image files.
const iconMap = {
    ltcStatus: {
        'LOCK': 'assets/timeturner_ltc_green.png',
        'FREE': 'assets/timeturner_ltc_orange.png',
        'default': 'assets/timeturner_ltc_red.png'
    },
    ntpActive: {
        true: 'assets/timeturner_ntp_green.png',
        false: 'assets/timeturner_ntp_red.png'
    },
    syncStatus: {
        'IN SYNC': 'assets/timeturner_sync_green.png',
        'CLOCK AHEAD': 'assets/timeturner_sync_orange.png',
        'CLOCK BEHIND': 'assets/timeturner_sync_orange.png',
        'TIMETURNING': 'assets/timeturner_timeturning.png',
        'default': 'assets/timeturner_sync_red.png'
    },
    jitterStatus: {
        'GOOD': 'assets/timeturner_jitter_green.png',
        'AVERAGE': 'assets/timeturner_jitter_orange.png',
        'BAD': 'assets/timeturner_jitter_red.png',
        'default': 'assets/timeturner_jitter_red.png'
    }
};
