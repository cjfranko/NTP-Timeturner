#!/bin/bash
set -e

echo ""
echo "─────────────────────────────────────────────"
echo "  Welcome to the NTP TimeTurner Installer"
echo "─────────────────────────────────────────────"
echo ""
echo "\"It's a very complicated piece of magic...\" – Hermione Granger"
echo "Preparing the Ministry-grade temporal interface..."
echo ""

# ---------------------------------------------------------
# Step 1: Update and upgrade packages
# ---------------------------------------------------------
echo "Step 1: Updating package lists and upgrading..."
sudo apt update && sudo apt upgrade -y

# ---------------------------------------------------------
# Step 2: Install essential development and serial tools
# ---------------------------------------------------------
echo "Step 2: Installing core tools and dependencies..."
sudo apt install -y git curl python3 python3-pip build-essential cmake \
  arduino-cli teensy-loader-cli python3-serial

# ---------------------------------------------------------
# Step 3: Download NTP TimeTurner script
# ---------------------------------------------------------
echo "Step 3: Downloading TimeTurner daemon script..."
cd /home/pi
wget -O timeturner.py https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/timeturner.py
chmod +x timeturner.py

# ---------------------------------------------------------
# Step 4: Install Python requirements
# ---------------------------------------------------------
echo "Step 4: Installing Python packages..."
pip3 install --break-system-packages pylibltc

# ---------------------------------------------------------
# Step 5: Prepare Teensy 4.0 programming
# ---------------------------------------------------------
echo "Step 5: Configuring Teensy programming environment..."
if [ ! -d "$HOME/teensy-firmware" ]; then
  git clone https://github.com/cjfranko/ltc-to-serial-teensy.git "$HOME/teensy-firmware"
fi
cd "$HOME/teensy-firmware"
arduino-cli compile --fqbn arduino:avr:teensy40 .
arduino-cli upload -p /dev/ttyACM0 --fqbn arduino:avr:teensy40 .

# ---------------------------------------------------------
# Step 6: Download and apply splash screen
# ---------------------------------------------------------
echo "Step 6: Downloading and applying custom splash screen..."
cd /home/pi
wget -O splash.png https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/splash.png
if [ -f splash.png ]; then
  sudo cp splash.png /usr/share/plymouth/themes/pix/splash.png
  sudo chmod 644 /usr/share/plymouth/themes/pix/splash.png
  echo "✅ Splash screen updated."
else
  echo "⚠️ splash.png not found — skipping."
fi

# ---------------------------------------------------------
# Step 7: Enable systemd service
# ---------------------------------------------------------
echo "Step 7: Setting up systemd service..."
sudo cp timeturner.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable timeturner.service

# ---------------------------------------------------------
# Final Message & Reboot
# ---------------------------------------------------------
echo "─────────────────────────────────────────────"
echo "  Setup Complete — Temporal alignment achieved."
echo "─────────────────────────────────────────────"
echo "Rebooting in 15 seconds to apply settings..."
sleep 15
sudo reboot
