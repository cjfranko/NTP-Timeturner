# 🕰️ NTP Timeturner

**NTP Timeturner** is a Raspberry Pi-based stratum 1 time server that sets its system clock based on incoming SMPTE LTC (Linear Timecode) audio. Designed for broadcast and production environments, it allows LTC-based time sync across local networks via NTP.

---

## 📦 Features

- 🕒 Decodes LTC timecode from audio input (25fps supported)
- 🌐 Serves NTP time to local devices
- ⚡ Fast, reliable startup using a USB audio interface
- 🔌 Optional OLED display for system time and sync status (coming soon)
- 🛜 Web interface for Wi-Fi config and status (in development)

---

## 🚀 Quick Start

### ✅ Requirements

- Raspberry Pi 3 or newer
- Debian Bookworm
- USB audio interface with 3.5mm mic/line input
- SMPTE LTC source (e.g. video playout, Blackmagic device)

### ⚙️ Setup Instructions

1. Clone the repo:
   ```bash
   git clone https://github.com/cjfranko/NTP-Timeturner.git
   cd NTP-Timeturner
