document.addEventListener('DOMContentLoaded', () => {
    let lastApiData = null;
    let lastApiFetchTime = null;

    const statusElements = {
        ltcStatus: document.getElementById('ltc-status'),
        ltcTimecode: document.getElementById('ltc-timecode'),
        frameRate: document.getElementById('frame-rate'),
        lockRatio: document.getElementById('lock-ratio'),
        systemClock: document.getElementById('system-clock'),
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

    function updateStatus(data) {
        statusElements.ltcStatus.textContent = data.ltc_status;
        statusElements.ltcTimecode.textContent = data.ltc_timecode;
        statusElements.frameRate.textContent = data.frame_rate;
        statusElements.lockRatio.textContent = data.lock_ratio.toFixed(2);
        statusElements.systemClock.textContent = data.system_clock;

        statusElements.ntpActive.textContent = data.ntp_active ? 'Active' : 'Inactive';
        statusElements.ntpActive.className = data.ntp_active ? 'active' : 'inactive';

        statusElements.syncStatus.textContent = data.sync_status;
        statusElements.syncStatus.className = data.sync_status.replace(/\s+/g, '-').toLowerCase();

        statusElements.deltaMs.textContent = data.timecode_delta_ms;
        statusElements.deltaFrames.textContent = data.timecode_delta_frames;

        statusElements.jitterStatus.textContent = data.jitter_status;
        statusElements.jitterStatus.className = data.jitter_status.toLowerCase();

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
        if (lastApiData.ltc_status === 'LOCK' && lastApiData.ltc_timecode && lastApiData.ltc_timecode.includes(':') && lastApiData.frame_rate) {
            const tcParts = lastApiData.ltc_timecode.split(':');
            const frameRate = parseFloat(lastApiData.frame_rate);
            if (tcParts.length === 4 && !isNaN(frameRate) && frameRate > 0) {
                let h = parseInt(tcParts[0], 10);
                let m = parseInt(tcParts[1], 10);
                let s = parseInt(tcParts[2], 10);
                let f = parseInt(tcParts[3], 10);

                const frameRateInt = Math.round(frameRate);
                const msPerFrame = 1000.0 / frameRate;
                const elapsedFrames = Math.floor(elapsedMs / msPerFrame);

                // Compute total seconds including fractional part
                let totalSeconds = h * 3600 + m * 60 + s + f / frameRateInt;
                totalSeconds += elapsedMs / 1000.0;

                // Convert back to components
                let newSeconds = Math.floor(totalSeconds);
                f = Math.round((totalSeconds - newSeconds) * frameRateInt) % frameRateInt;

                h = Math.floor(newSeconds / 3600);
                newSeconds %= 3600;
                m = Math.floor(newSeconds / 60);
                s = Math.floor(newSeconds % 60);
                h %= 24;

                statusElements.ltcTimecode.textContent =
                    `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}:${String(f).padStart(2, '0')}`;
            }
        }
    }

    async function fetchStatus() {
        try {
            const response = await fetch('/api/status');
            if (!response.ok) throw new Error('Failed to fetch status');
            const data = await response.json();
            updateStatus(data);
            lastApiData = data;
            lastApiFetchTime = new Date();
        } catch (error) {
            console.error('Error fetching status:', error);
            lastApiData = null;
            lastApiFetchTime = null;
        }
    }

    async function fetchConfig() {
        try {
            const response = await fetch('/api/config');
            if (!response.ok) throw new Error('Failed to fetch config');
            const data = await response.json();
            hwOffsetInput.value = data.hardwareOffsetMs;
            autoSyncCheckbox.checked = data.autoSyncEnabled;
            offsetInputs.h.value = data.timeturnerOffset.hours;
            offsetInputs.m.value = data.timeturnerOffset.minutes;
            offsetInputs.s.value = data.timeturnerOffset.seconds;
            offsetInputs.f.value = data.timeturnerOffset.frames;
            offsetInputs.ms.value = data.timeturnerOffset.milliseconds || 0;
            nudgeValueInput.value = data.defaultNudgeMs;
        } catch (error) {
            console.error('Error fetching config:', error);
        }
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

        try {
            const response = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config),
            });
            if (!response.ok) throw new Error('Failed to save config');
            alert('Configuration saved.');
        } catch (error) {
            console.error('Error saving config:', error);
            alert('Error saving configuration.');
        }
    }

    async function fetchLogs() {
        try {
            const response = await fetch('/api/logs');
            if (!response.ok) throw new Error('Failed to fetch logs');
            const logs = await response.json();
            statusElements.logs.textContent = logs.join('\n');
            // Auto-scroll to the bottom
            statusElements.logs.scrollTop = statusElements.logs.scrollHeight;
        } catch (error) {
            console.error('Error fetching logs:', error);
            statusElements.logs.textContent = 'Error fetching logs.';
        }
    }

    async function triggerManualSync() {
        syncMessage.textContent = 'Issuing sync command...';
        try {
            const response = await fetch('/api/sync', { method: 'POST' });
            const data = await response.json();
            if (response.ok) {
                syncMessage.textContent = `Success: ${data.message}`;
            } else {
                syncMessage.textContent = `Error: ${data.message}`;
            }
        } catch (error) {
            console.error('Error triggering sync:', error);
            syncMessage.textContent = 'Failed to send sync command.';
        }
        setTimeout(() => { syncMessage.textContent = ''; }, 5000);
    }

    async function nudgeClock(ms) {
        nudgeMessage.textContent = 'Nudging clock...';
        try {
            const response = await fetch('/api/nudge_clock', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ microseconds: ms * 1000 }),
            });
            const data = await response.json();
            if (response.ok) {
                nudgeMessage.textContent = `Success: ${data.message}`;
            } else {
                nudgeMessage.textContent = `Error: ${data.message}`;
            }
        } catch (error) {
            console.error('Error nudging clock:', error);
            nudgeMessage.textContent = 'Failed to send nudge command.';
        }
        setTimeout(() => { nudgeMessage.textContent = ''; }, 3000);
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

    // Initial data load
    fetchStatus();
    fetchConfig();
    fetchLogs();

    // Refresh data every 2 seconds
    setInterval(fetchStatus, 2000);
    setInterval(fetchLogs, 2000);
    setInterval(animateClocks, 50); // High-frequency clock animation
});
