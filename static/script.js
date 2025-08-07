document.addEventListener('DOMContentLoaded', () => {
    const mockApiData = {
        status: {
            ltc_status: 'LOCK',
            ltc_timecode: '10:20:30:00',
            frame_rate: '25.00',
            lock_ratio: 99.5,
            system_clock: '10:20:30.500',
            system_date: '2025-08-07',
            ntp_active: true,
            sync_status: 'IN SYNC',
            timecode_delta_ms: 5,
            timecode_delta_frames: 0.125,
            jitter_status: 'GOOD',
            interfaces: ['192.168.1.100 (eth0)', '10.0.0.5 (wlan0)'],
        },
        config: {
            hardwareOffsetMs: 10,
            autoSyncEnabled: true,
            defaultNudgeMs: 2,
            timeturnerOffset: {
                hours: 1,
                minutes: 2,
                seconds: 3,
                frames: 4,
                milliseconds: 50
            },
        },
        logs: [
            '2025-08-07 10:20:30 [INFO] Starting up...',
            '2025-08-07 10:20:31 [INFO] Found serial device on /dev/ttyACM0.',
            '2025-08-07 10:20:32 [INFO] LTC LOCK detected. Frame rate: 25.00fps.',
            '2025-08-07 10:20:35 [INFO] Initial sync complete. Clock adjusted by -15ms.',
        ]
    };

    let lastApiData = null;
    let lastApiFetchTime = null;

    const statusElements = {
        ltcStatus: document.getElementById('ltc-status'),
        ltcTimecode: document.getElementById('ltc-timecode'),
        frameRate: document.getElementById('frame-rate'),
        lockRatio: document.getElementById('lock-ratio'),
        systemClock: document.getElementById('system-clock'),
        systemDate: document.getElementById('system-date'),
        ntpActive: document.getElementById('ntp-active'),
        syncStatus: document.getElementById('sync-status'),
        deltaMs: document.getElementById('delta-ms'),
        deltaFrames: document.getElementById('delta-frames'),
        jitterStatus: document.getElementById('jitter-status'),
        interfaces: document.getElementById('interfaces'),
        logs: document.getElementById('logs'),
    };

    const hwOffsetInput = document.getElementById('hw-offset');
    const autoSyncCheckbox = document.getElementById('auto-sync-enabled');
    const offsetInputs = {
        h: document.getElementById('offset-h'),
        m: document.getElementById('offset-m'),
        s: document.getElementById('offset-s'),
        f: document.getElementById('offset-f'),
        ms: document.getElementById('offset-ms'),
    };
    const saveConfigButton = document.getElementById('save-config');
    const manualSyncButton = document.getElementById('manual-sync');
    const syncMessage = document.getElementById('sync-message');

    const nudgeDownButton = document.getElementById('nudge-down');
    const nudgeUpButton = document.getElementById('nudge-up');
    const nudgeValueInput = document.getElementById('nudge-value');
    const nudgeMessage = document.getElementById('nudge-message');

    const dateInput = document.getElementById('date-input');
    const setDateButton = document.getElementById('set-date');
    const dateMessage = document.getElementById('date-message');

    function updateStatus(data) {
        const ltcStatus = data.ltc_status || 'UNKNOWN';
        const ltcIconSrc = iconMap.ltcStatus[ltcStatus] || iconMap.ltcStatus.default;
        statusElements.ltcStatus.innerHTML = `<img src="${ltcIconSrc}" class="status-icon" alt=""> ${ltcStatus}`;
        statusElements.ltcStatus.className = ltcStatus.toLowerCase();
        statusElements.ltcTimecode.textContent = data.ltc_timecode;
        statusElements.frameRate.textContent = data.frame_rate;
        statusElements.lockRatio.textContent = data.lock_ratio.toFixed(2);
        statusElements.systemClock.textContent = data.system_clock;
        statusElements.systemDate.textContent = data.system_date;

        const ntpIconSrc = iconMap.ntpActive[data.ntp_active];
        if (data.ntp_active) {
            statusElements.ntpActive.innerHTML = `<img src="${ntpIconSrc}" class="status-icon" alt=""> Active`;
            statusElements.ntpActive.className = 'active';
        } else {
            statusElements.ntpActive.innerHTML = `<img src="${ntpIconSrc}" class="status-icon" alt=""> Inactive`;
            statusElements.ntpActive.className = 'inactive';
        }

        const syncStatus = data.sync_status || 'UNKNOWN';
        const syncIconSrc = iconMap.syncStatus[syncStatus] || iconMap.syncStatus.default;
        statusElements.syncStatus.innerHTML = `<img src="${syncIconSrc}" class="status-icon" alt=""> ${syncStatus}`;
        statusElements.syncStatus.className = syncStatus.replace(/\s+/g, '-').toLowerCase();

        statusElements.deltaMs.textContent = data.timecode_delta_ms;
        statusElements.deltaFrames.textContent = data.timecode_delta_frames;

        const jitterStatus = data.jitter_status || 'UNKNOWN';
        const jitterIconSrc = iconMap.jitterStatus[jitterStatus] || iconMap.jitterStatus.default;
        statusElements.jitterStatus.innerHTML = `<img src="${jitterIconSrc}" class="status-icon" alt=""> ${jitterStatus}`;
        statusElements.jitterStatus.className = jitterStatus.toLowerCase();

        statusElements.interfaces.innerHTML = '';
        if (data.interfaces.length > 0) {
            data.interfaces.forEach(ip => {
                const li = document.createElement('li');
                li.textContent = ip;
                statusElements.interfaces.appendChild(li);
            });
        } else {
            const li = document.createElement('li');
            li.textContent = 'No active interfaces found.';
            statusElements.interfaces.appendChild(li);
        }
    }

    function animateClocks() {
        if (!lastApiData || !lastApiFetchTime) return;

        const elapsedMs = new Date() - lastApiFetchTime;

        // Animate System Clock
        if (lastApiData.system_clock && lastApiData.system_clock.includes(':')) {
            const parts = lastApiData.system_clock.split(/[:.]/);
            if (parts.length === 4) {
                const baseDate = new Date();
                baseDate.setHours(parseInt(parts[0], 10), parseInt(parts[1], 10), parseInt(parts[2], 10));
                baseDate.setMilliseconds(parseInt(parts[3], 10));

                const newDate = new Date(baseDate.getTime() + elapsedMs);

                const h = String(newDate.getHours()).padStart(2, '0');
                const m = String(newDate.getMinutes()).padStart(2, '0');
                const s = String(newDate.getSeconds()).padStart(2, '0');
                const ms = String(newDate.getMilliseconds()).padStart(3, '0');
                statusElements.systemClock.textContent = `${h}:${m}:${s}.${ms}`;
            }
        }

        // Animate LTC Timecode - only if status is LOCK
        if (lastApiData.ltc_status === 'LOCK' && lastApiData.ltc_timecode && lastApiData.ltc_timecode.match(/[:;]/) && lastApiData.frame_rate) {
            const separator = lastApiData.ltc_timecode.includes(';') ? ';' : ':';
            const tcParts = lastApiData.ltc_timecode.split(/[:;]/);
            const frameRate = parseFloat(lastApiData.frame_rate);

            if (tcParts.length === 4 && !isNaN(frameRate) && frameRate > 0) {
                let h = parseInt(tcParts[0], 10);
                let m = parseInt(tcParts[1], 10);
                let s = parseInt(tcParts[2], 10);
                let f = parseInt(tcParts[3], 10);

                const msPerFrame = 1000.0 / frameRate;
                const elapsedFrames = Math.floor(elapsedMs / msPerFrame);

                f += elapsedFrames;

                const frameRateInt = Math.round(frameRate);

                s += Math.floor(f / frameRateInt);
                f %= frameRateInt;

                m += Math.floor(s / 60);
                s %= 60;

                h += Math.floor(m / 60);
                m %= 60;

                h %= 24;

                statusElements.ltcTimecode.textContent =
                    `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}${separator}${String(f).padStart(2, '0')}`;
            }
        }
    }

    async function fetchStatus() {
        // Mock implementation to allow UI development without a running backend.
        const data = mockApiData.status;
        updateStatus(data);
        lastApiData = data;
        lastApiFetchTime = new Date();
    }

    async function fetchConfig() {
        // Mock implementation
        const data = mockApiData.config;
        hwOffsetInput.value = data.hardwareOffsetMs;
        autoSyncCheckbox.checked = data.autoSyncEnabled;
        offsetInputs.h.value = data.timeturnerOffset.hours;
        offsetInputs.m.value = data.timeturnerOffset.minutes;
        offsetInputs.s.value = data.timeturnerOffset.seconds;
        offsetInputs.f.value = data.timeturnerOffset.frames;
        offsetInputs.ms.value = data.timeturnerOffset.milliseconds || 0;
        nudgeValueInput.value = data.defaultNudgeMs;
    }

    async function saveConfig() {
        const config = {
            hardwareOffsetMs: parseInt(hwOffsetInput.value, 10) || 0,
            autoSyncEnabled: autoSyncCheckbox.checked,
            defaultNudgeMs: parseInt(nudgeValueInput.value, 10) || 0,
            timeturnerOffset: {
                hours: parseInt(offsetInputs.h.value, 10) || 0,
                minutes: parseInt(offsetInputs.m.value, 10) || 0,
                seconds: parseInt(offsetInputs.s.value, 10) || 0,
                frames: parseInt(offsetInputs.f.value, 10) || 0,
                milliseconds: parseInt(offsetInputs.ms.value, 10) || 0,
            }
        };

        // Mock implementation
        console.log('Saving mock config:', config);
        alert('Configuration saved (mock).');
    }

    async function fetchLogs() {
        // Mock implementation
        const logs = mockApiData.logs;
        statusElements.logs.textContent = logs.join('\n');
        // Auto-scroll to the bottom
        statusElements.logs.scrollTop = statusElements.logs.scrollHeight;
    }

    async function triggerManualSync() {
        syncMessage.textContent = 'Issuing sync command...';
        // Mock implementation
        setTimeout(() => {
            syncMessage.textContent = 'Success: Manual sync triggered (mock).';
        }, 1000);
        setTimeout(() => { syncMessage.textContent = ''; }, 5000);
    }

    async function nudgeClock(ms) {
        nudgeMessage.textContent = 'Nudging clock...';
        // Mock implementation
        setTimeout(() => {
            nudgeMessage.textContent = `Success: Clock nudged by ${ms}ms (mock).`;
        }, 500);
        setTimeout(() => { nudgeMessage.textContent = ''; }, 3000);
    }

    async function setDate() {
        const date = dateInput.value;
        if (!date) {
            alert('Please select a date.');
            return;
        }

        dateMessage.textContent = 'Setting date...';
        // Mock implementation
        setTimeout(() => {
            dateMessage.textContent = `Success: Date set to ${date} (mock).`;
            // To make it look real, we can update the system date display
            if (lastApiData) {
                mockApiData.status.system_date = date;
                fetchStatus();
            }
        }, 1000);
        setTimeout(() => { dateMessage.textContent = ''; }, 5000);
    }

    saveConfigButton.addEventListener('click', saveConfig);
    manualSyncButton.addEventListener('click', triggerManualSync);
    nudgeDownButton.addEventListener('click', () => {
        const ms = parseInt(nudgeValueInput.value, 10) || 0;
        nudgeClock(-ms);
    });
    nudgeUpButton.addEventListener('click', () => {
        const ms = parseInt(nudgeValueInput.value, 10) || 0;
        nudgeClock(ms);
    });
    setDateButton.addEventListener('click', setDate);

    // Initial data load
    fetchStatus();
    fetchConfig();
    fetchLogs();

    // Refresh data every 2 seconds - MOCKED
    // setInterval(fetchStatus, 2000);
    // setInterval(fetchLogs, 2000);
    setInterval(animateClocks, 50); // High-frequency clock animation
});
