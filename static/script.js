document.addEventListener('DOMContentLoaded', () => {
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
    };

    const hwOffsetInput = document.getElementById('hw-offset');
    const saveOffsetButton = document.getElementById('save-offset');
    const manualSyncButton = document.getElementById('manual-sync');
    const syncMessage = document.getElementById('sync-message');

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

    async function fetchStatus() {
        try {
            const response = await fetch('/api/status');
            if (!response.ok) throw new Error('Failed to fetch status');
            const data = await response.json();
            updateStatus(data);
        } catch (error) {
            console.error('Error fetching status:', error);
        }
    }

    async function fetchConfig() {
        try {
            const response = await fetch('/api/config');
            if (!response.ok) throw new Error('Failed to fetch config');
            const data = await response.json();
            hwOffsetInput.value = data.hardware_offset_ms;
        } catch (error) {
            console.error('Error fetching config:', error);
        }
    }

    async function saveConfig() {
        const offset = parseInt(hwOffsetInput.value, 10);
        if (isNaN(offset)) {
            alert('Invalid hardware offset value.');
            return;
        }

        try {
            const response = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ hardware_offset_ms: offset }),
            });
            if (!response.ok) throw new Error('Failed to save config');
            alert('Configuration saved.');
        } catch (error) {
            console.error('Error saving config:', error);
            alert('Error saving configuration.');
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

    saveOffsetButton.addEventListener('click', saveConfig);
    manualSyncButton.addEventListener('click', triggerManualSync);

    // Initial data load
    fetchStatus();
    fetchConfig();

    // Refresh data every 2 seconds
    setInterval(fetchStatus, 2000);
});
