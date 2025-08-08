document.addEventListener('DOMContentLoaded', () => {
    // --- Mock Data Configuration ---
    // Set to true to use mock data, false for live API.
    const useMockData = false; 
    let currentMockSetKey = 'allGood'; // Default mock data set

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
        deltaStatus: document.getElementById('delta-status'),
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

    // --- Collapsible Sections ---
    const controlsToggle = document.getElementById('controls-toggle');
    const controlsContent = document.getElementById('controls-content');
    const logsToggle = document.getElementById('logs-toggle');
    const logsContent = document.getElementById('logs-content');

    // --- Mock Controls Setup ---
    const mockControls = document.getElementById('mock-controls');
    const mockDataSelector = document.getElementById('mock-data-selector');

    function setupMockControls() {
        if (useMockData) {
            mockControls.style.display = 'block';
            
            // Populate dropdown
            Object.keys(mockApiDataSets).forEach(key => {
                const option = document.createElement('option');
                option.value = key;
                option.textContent = key.replace(/([A-Z])/g, ' $1').replace(/^./, str => str.toUpperCase());
                mockDataSelector.appendChild(option);
            });

            mockDataSelector.value = currentMockSetKey;

            // Handle selection change
            mockDataSelector.addEventListener('change', (event) => {
                currentMockSetKey = event.target.value;
                // Re-fetch all data from the new mock set
                fetchStatus();
                fetchConfig();
                fetchLogs();
            });
        }
    }

    function updateStatus(data) {
        const ltcStatus = data.ltc_status || 'UNKNOWN';
        const ltcIconInfo = iconMap.ltcStatus[ltcStatus] || iconMap.ltcStatus.default;
        statusElements.ltcStatus.innerHTML = `<img src="${ltcIconInfo.src}" class="status-icon" alt="" title="${ltcIconInfo.tooltip}">`;
        statusElements.ltcStatus.className = ltcStatus.toLowerCase();
        statusElements.ltcTimecode.textContent = data.ltc_timecode;

        const frameRate = data.frame_rate || 'unknown';
        const frameRateIconInfo = iconMap.frameRate[frameRate] || iconMap.frameRate.default;
        statusElements.frameRate.innerHTML = `<img src="${frameRateIconInfo.src}" class="status-icon" alt="" title="${frameRateIconInfo.tooltip}">`;

        const lockRatio = data.lock_ratio;
        let lockRatioCategory;
        if (lockRatio === 100) {
            lockRatioCategory = 'good';
        } else if (lockRatio >= 90) {
            lockRatioCategory = 'average';
        } else {
            lockRatioCategory = 'bad';
        }
        const lockRatioIconInfo = iconMap.lockRatio[lockRatioCategory];
        statusElements.lockRatio.innerHTML = `<img src="${lockRatioIconInfo.src}" class="status-icon" alt="" title="${lockRatioIconInfo.tooltip}">`;
        statusElements.systemClock.textContent = data.system_clock;
        statusElements.systemDate.textContent = data.system_date;

        // Autofill the date input, but don't overwrite user edits.
        if (!lastApiData || dateInput.value === lastApiData.system_date) {
            dateInput.value = data.system_date;
        }

        const ntpIconInfo = iconMap.ntpActive[!!data.ntp_active];
        if (data.ntp_active) {
            statusElements.ntpActive.innerHTML = `<img src="${ntpIconInfo.src}" class="status-icon" alt="" title="${ntpIconInfo.tooltip}">`;
            statusElements.ntpActive.className = 'active';
        } else {
            statusElements.ntpActive.innerHTML = `<img src="${ntpIconInfo.src}" class="status-icon" alt="" title="${ntpIconInfo.tooltip}">`;
            statusElements.ntpActive.className = 'inactive';
        }

        const syncStatus = data.sync_status || 'UNKNOWN';
        const syncIconInfo = iconMap.syncStatus[syncStatus] || iconMap.syncStatus.default;
        statusElements.syncStatus.innerHTML = `<img src="${syncIconInfo.src}" class="status-icon" alt="" title="${syncIconInfo.tooltip}">`;
        statusElements.syncStatus.className = syncStatus.replace(/\s+/g, '-').toLowerCase();

        // Delta Status
        const deltaMs = data.timecode_delta_ms;
        let deltaCategory;
        if (deltaMs === 0) {
            deltaCategory = 'good';
        } else if (Math.abs(deltaMs) < 10) {
            deltaCategory = 'average';
        } else {
            deltaCategory = 'bad';
        }
        const deltaIconInfo = iconMap.deltaStatus[deltaCategory];
        const deltaText = `${data.timecode_delta_ms} ms (${data.timecode_delta_frames} frames)`;
        statusElements.deltaStatus.innerHTML = `<img src="${deltaIconInfo.src}" class="status-icon" alt="" title="${deltaIconInfo.tooltip}"><span>${deltaText}</span>`;

        const jitterStatus = data.jitter_status || 'UNKNOWN';
        const jitterIconInfo = iconMap.jitterStatus[jitterStatus] || iconMap.jitterStatus.default;
        statusElements.jitterStatus.innerHTML = `<img src="${jitterIconInfo.src}" class="status-icon" alt="" title="${jitterIconInfo.tooltip}">`;
        statusElements.jitterStatus.className = jitterStatus.toLowerCase();

        if (data.interfaces.length > 0) {
            statusElements.interfaces.textContent = data.interfaces.join(' | ');
        } else {
            statusElements.interfaces.textContent = 'No active interfaces found.';
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
        if (useMockData) {
            const data = mockApiDataSets[currentMockSetKey].status;
            updateStatus(data);
            lastApiData = data;
            lastApiFetchTime = new Date();
            return;
        }
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
        if (useMockData) {
            const data = mockApiDataSets[currentMockSetKey].config;
            hwOffsetInput.value = data.hardwareOffsetMs;
            autoSyncCheckbox.checked = data.autoSyncEnabled;
            offsetInputs.h.value = data.timeturnerOffset.hours;
            offsetInputs.m.value = data.timeturnerOffset.minutes;
            offsetInputs.s.value = data.timeturnerOffset.seconds;
            offsetInputs.f.value = data.timeturnerOffset.frames;
            offsetInputs.ms.value = data.timeturnerOffset.milliseconds || 0;
            nudgeValueInput.value = data.defaultNudgeMs;
            return;
        }
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

        if (useMockData) {
            console.log('Mock save:', config);
            alert('Configuration saved (mock).');
            // We can also update the mock data in memory to see changes reflected
            mockApiDataSets[currentMockSetKey].config = config;
            return;
        }

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
        if (useMockData) {
            // Use a copy to avoid mutating the original mock data array
            const logs = mockApiDataSets[currentMockSetKey].logs.slice();
            // Show latest 20 logs, with the newest at the top.
            logs.reverse();
            statusElements.logs.textContent = logs.slice(0, 20).join('\n');
            return;
        }
        try {
            const response = await fetch('/api/logs');
            if (!response.ok) throw new Error('Failed to fetch logs');
            const logs = await response.json();
            // Show latest 20 logs, with the newest at the top.
            logs.reverse();
            statusElements.logs.textContent = logs.slice(0, 20).join('\n');
        } catch (error) {
            console.error('Error fetching logs:', error);
            statusElements.logs.textContent = 'Error fetching logs.';
        }
    }

    async function triggerManualSync() {
        syncMessage.textContent = 'Issuing sync command...';
        if (useMockData) {
            syncMessage.textContent = 'Success: Manual sync triggered (mock).';
            setTimeout(() => { syncMessage.textContent = ''; }, 5000);
            return;
        }
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
        if (useMockData) {
            nudgeMessage.textContent = `Success: Clock nudged by ${ms}ms (mock).`;
            setTimeout(() => { nudgeMessage.textContent = ''; }, 3000);
            return;
        }
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

    async function setDate() {
        const date = dateInput.value;
        if (!date) {
            alert('Please select a date.');
            return;
        }

        dateMessage.textContent = 'Setting date...';
        if (useMockData) {
            mockApiDataSets[currentMockSetKey].status.system_date = date;
            dateMessage.textContent = `Success: Date set to ${date} (mock).`;
            fetchStatus(); // re-render
            setTimeout(() => { dateMessage.textContent = ''; }, 5000);
            return;
        }
        try {
            const response = await fetch('/api/set_date', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ date: date }),
            });
            const data = await response.json();
            if (response.ok) {
                dateMessage.textContent = `Success: ${data.message}`;
                // Fetch status again to update the displayed date immediately
                fetchStatus();
            } else {
                dateMessage.textContent = `Error: ${data.message}`;
            }
        } catch (error) {
            console.error('Error setting date:', error);
            dateMessage.textContent = 'Failed to send date command.';
        }
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

    // --- Collapsible Section Listeners ---
    controlsToggle.addEventListener('click', () => {
        const isActive = controlsContent.classList.toggle('active');
        controlsToggle.classList.toggle('active', isActive);
    });

    logsToggle.addEventListener('click', () => {
        const isActive = logsContent.classList.toggle('active');
        logsToggle.classList.toggle('active', isActive);
    });

    // Initial data load
    setupMockControls();
    fetchStatus();
    fetchConfig();
    fetchLogs();

    // Refresh data every 2 seconds if not using mock data
    if (!useMockData) {
        setInterval(fetchStatus, 2000);
        setInterval(fetchLogs, 2000);
    }
    setInterval(animateClocks, 50); // High-frequency clock animation
});
