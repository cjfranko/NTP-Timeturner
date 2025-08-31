# Fetch | Hachi (alpha)

**An LTC-driven NTP server for Raspberry Pi, built with broadcast precision**

Hachi synchronises timecode-locked systems by decoding incoming LTC (Linear Time Code) and broadcasting it as NTP/PTP — with the dedication our namesake would insist upon.

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

## 🚀 Installation

The `setup.sh` script compiles and installs the TimeTurner application. You can run it by cloning the repository with `git` or by using the `curl` command below for a git-free installation.

### Prerequisites

- **Internet Connection**: To download dependencies.
- **Curl and Unzip**: The script requires `curl` to download files and `unzip` for the git-free method. The setup script will attempt to install these if they are missing.

### Running the Installer (Recommended)

This command downloads the latest version, unpacks it, and runs the setup script. Paste it into your Raspberry Pi terminal:

```bash
curl -L https://github.com/cjfranko/NTP-Timeturner/archive/refs/heads/main.zip -o NTP-Timeturner.zip && \
unzip NTP-Timeturner.zip && \
cd NTP-Timeturner-main && \
chmod +x setup.sh && \
./setup.sh
```

### What the Script Does

The installation script automates the following steps:

1.  **Installs Dependencies**: Installs `git`, `curl`, `unzip`, and necessary build tools.
2.  **Compiles the Binary**: Runs `cargo build --release` to create an optimised executable.
3.  **Creates Directories**: Creates `/opt/timeturner` to store the application files.
4.  **Installs Files**: 
    - The compiled binary is copied to `/opt/timeturner/timeturner`.
    - The web interface assets from the `static/` directory are copied to `/opt/timeturner/static`.
    - A symbolic link is created from `/usr/local/bin/timeturner` to the binary, allowing it to be run from any location.
5.  **Sets up Systemd Service**: 
    - Copies the `timeturner.service` file to `/etc/systemd/system/`.
    - Enables the service to start automatically on system boot.

After installation is complete, the script will provide instructions to start the service manually or to run the application in its interactive terminal mode.

```bash
The working directory is /opt/timeturner.
Default 'config.yml' installed to /opt/timeturner.

To start the service, run:
  sudo systemctl start timeturner.service

To view live logs, run:
  journalctl -u timeturner.service -f

To run the interactive TUI instead, simply run from the project directory:
  cargo run
Or from anywhere after installation:
  timeturner
```

---

## 🔄 Updating

If you installed TimeTurner by cloning the repository with `git`, you can use the `update.sh` script to easily update to the latest version.

**Note**: This script will not work if you used the `curl` one-line command for installation, as that method does not create a Git repository.

To run the update script, navigate to the `NTP-Timeturner-main` directory and run:
```bash
chmod +x update.sh && ./update.sh
```

The update script automates the following:
1.  Pulls the latest code from the `main` branch on GitHub.
2.  Rebuilds the application binary.
3.  Copies the new binary to `/opt/timeturner/`.
4.  Restarts the `timeturner` service to apply the changes.

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
local stratum 1

Add to bottom:
# Allow LAN clients
allow 0.0.0.0/0

# comment out:
pool 2.debian.pool.ntp.org iburst
sourcedir /run/chrony-dhcp
```

