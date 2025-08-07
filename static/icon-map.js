// In this file, you can define the paths to your local icon image files.
const iconMap = {
    ltcStatus: {
        'LOCK': 'assets/timetuner_ltc_green.png',
        'FREE': 'assets/timetuner_ltc_orange.png',
        'default': 'assets/timetuner_ltc_red.png'
    },
    ntpActive: {
        true: 'assets/timetuner_ntp_green.png',
        false: 'assets/timetuner_ntp_red.png'
    },
    syncStatus: {
        'IN SYNC': 'assets/timetuner_sync_green.png',
        'CLOCK AHEAD': 'assets/timetuner_sync_orange.png',
        'CLOCK BEHIND': 'assets/timetuner_sync_orange.png',
        'TIMETURNING': 'assets/timetuner_timeturning.png',
        'default': 'assets/timetuner_sync_red.png'
    },
    jitterStatus: {
        'GOOD': 'assets/timetuner_jitter_green.png',
        'AVERAGE': 'assets/timetuner_jitter_orange.png',
        'BAD': 'assets/timetuner_jitter_red.png',
        'default': 'assets/timetuner_jitter_red.png'
    }
};
