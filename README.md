# 🕰️ NTP Timeturner

**An LTC-driven NTP server for Raspberry Pi, built with broadcast precision and a hint of magic.**

Inspired by Hermione Granger’s TimeTurner, this project synchronises timecode-locked systems by decoding incoming LTC (Linear Time Code) and broadcasting it as NTP — with precision down to the millisecond.

---

## 📦 Hardware Requirements

- Raspberry Pi 3 (or better)
- Debian Bookworm (64-bit recommended)
- USB audio input (e.g. USB to 3.5mm TRS adapter)
- Ethernet connection (recommended for stable NTP)
- Optional: Blackmagic Video Assist or LTC generator for input testing

---

## 🛠️ Software Features

- Reads SMPTE LTC from audio input (25p/50i to start, with more frame rate support to follow)
- Converts LTC into NTP-synced time
- Broadcasts time via local NTP server
- Supports configurable time offsets (hours, minutes, seconds, milliseconds)
- Systemd service support for headless operation
- Optional splash screen branding at boot

---

## 🚀 Installation

Clone and run the installer:

```bash
wget https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/setup.sh
chmod +x setup.sh
./setup.sh
