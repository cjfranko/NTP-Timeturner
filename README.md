# 🕰️ NTP Timeturner (alpha)

**An LTC-driven NTP server for Raspberry Pi, built with broadcast precision and a hint of magic.**

Inspired by the TimeTurner in the Harry Potter series, this project synchronises timecode-locked systems by decoding incoming LTC (Linear Time Code) and broadcasting it as NTP — with precision as Hermione would insist upon.

Created by Chris Frankland-Wright and John Rogers

---

## 📦 Hardware Requirements

- Raspberry Pi 5 (Dev Platform) but should be supported by Pi v3 (or better)
- Debian Bookworm (64-bit recommended)
- Teensy 4.0 - https://thepihut.com/products/teensy-4-0-headers
- Audio Adapter Board for Teensy 4.0 (Rev D) - https://thepihut.com/products/audio-adapter-board-for-teensy-4-0
- Ethernet connection (recommended for <1ms sync NTP broadcast)
- Optional: LTC generator for input testing - Windows/Mac App - https://timecodesync.com/generator/
- NetTime: Software to sync Windows OS to custom NTP servers - https://www.timesynctool.com/
---

## 🛠️ Software Features

- Reads SMPTE LTC from Audio Interface (3.5mm TRS but adaptable to BNC/XLR)
- Converts LTC into NTP-synced time
- Broadcasts time via local NTP server
- Supports configurable time offsets (hours, minutes, seconds, frames or milliseconds)
- Systemd service support for headless operation
- Web-based UI for monitoring and control when running as a daemon

---

## 🖥️ Web Interface & API

When running as a background daemon, TimeTurner provides a web interface for monitoring and configuration.

- **Access**: The web UI is available at `http://<raspberry_pi_ip>:8080`.
- **Functionality**: You can view the real-time sync status, see logs, and change all configuration options directly from your browser.
- **API**: A JSON API is also exposed for programmatic access. See `docs/api.md` for full details.

---

## 🛠️ Known Issues

- Supported Frame Rates: 24/25fps
- Non Supported Frame Rates: 23.98/30/59.94/60
- Fractional framerates have drift or wrong wall clock sync issues

---

## 🚀 Installation (to update)

For Rust install you can do 
```bash
cargo install --git https://github.com/cjfranko/NTP-Timeturner
```
Clone and run the installer:

```bash
wget https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/setup.sh
chmod +x setup.sh
./setup.sh
```

---
## 🕰️ Chrony NTP 
```bash
chronyc sources | Checks Source
chronyc tracking | NTP Tracking
sudo nano /etc/chrony/chrony.conf | Default Chrony Conf File

Add to top:
# Serve the system clock as a reference at stratum 10
server 127.127.1.0
allow 127.0.0.0/8
local stratum 10

Add to bottom:
# Allow LAN clients
allow 0.0.0.0/0

# comment out:
pool 2.debian.pool.ntp.org iburst
sourcedir /run/chrony-dhcp
```

