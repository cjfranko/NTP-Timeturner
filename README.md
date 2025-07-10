# 🕰️ NTP Timeturner (alpha)

**An LTC-driven NTP server for Raspberry Pi, built with broadcast precision and a hint of magic.**

Inspired by the TimeTurner in the Harry Potter<sup>[1](#myfootnote1)</sup> series, this project synchronises timecode-locked systems by decoding incoming LTC (Linear Time Code) and broadcasting it as NTP — with precision as Hermione would insist upon. 


###### <a name="myfootnote1">1</a>: *Editor's Note: Trans rights are Your Human Rights 🏳️‍⚧️. While the author of the Harry Potter series holds horrible backwards old-fashioned and abhorant views. we believe in supporting trans, non-binary, and gender-diverse people.If you find this software useful please consider donating to [Mermaids](https://mermaidsuk.org.uk/), a UK charity supporting trans, non-binary, and gender-diverse children, young people, and their families since 1995.
---

## 📦 Hardware Requirements

- Raspberry Pi 5 (Dev Platform) but should be supported by Pi v3 (or better)
- Debian Bookworm (64-bit recommended)
- Teensy 4.0 - https://thepihut.com/products/teensy-4-0-headers
- Audio Adapter Board for Teensy 4.0 (Rev D) - https://thepihut.com/products/audio-adapter-board-for-teensy-4-0
- Ethernet connection (recommended for stable NTP broadcast)
- Optional: LTC generator for input testing - Windows/Mac App - https://timecodesync.com/generator/

---

## 🛠️ Software Features

- Reads SMPTE LTC from Audio Interface (3.5mm TRS but adaptable to BNC/XLR)
- Converts LTC into NTP-synced time
- Broadcasts time via local NTP server
- Supports configurable time offsets (hours, minutes, seconds, milliseconds)
- Systemd service support for headless operation
- Optional splash screen branding at boot

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
