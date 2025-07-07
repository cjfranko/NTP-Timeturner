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
# Step 2: Install core tools and Python requirements
# ---------------------------------------------------------
echo "Step 2: Installing tools and Python libraries..."
sudo apt install -y git curl python3 python3-pip build-essential cmake \
  teensy-loader-cli python3-serial

# ---------------------------------------------------------
# Step 3: Install Arduino CLI manually
# ---------------------------------------------------------
echo "Step 3: Installing arduino-cli manually... from home"
cd "$HOME"
curl -fsSL https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh | sh
sudo mv bin/arduino-cli /usr/local/bin/

echo "Verifying arduino-cli install..."
arduino-cli version || echo "⚠️ arduino-cli install failed or not found in PATH"

# ---------------------------------------------------------
# Step 4: Install Python package(s)
# ---------------------------------------------------------
echo "Step 4: Installing Python packages..."
pip3 install --break-system-packages pylibltc

# ---------------------------------------------------------
# Step 5: Download and apply splash screen
# ---------------------------------------------------------
echo "Step 5: Downloading and applying splash screen..."
cd "$HOME"
wget -O splash.png https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/splash.png

if [ -f splash.png ]; then
  sudo cp splash.png /usr/share/plymouth/themes/pix/splash.png
  sudo chmod 644 /usr/share/plymouth/themes/pix/splash.png
  echo "✅ Splash screen applied."
else
  echo "⚠️ splash.png not found — skipping."
fi

# ---------------------------------------------------------
# Step 6: Download Teensy firmware and flash if available
# ---------------------------------------------------------
echo "Step 6: Downloading Teensy firmware..."
cd "$HOME"
wget -O ltc_audiohat_lock.ino.hex https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/firmware/ltc_audiohat_lock.ino.hex

echo "Checking for connected Teensy 4.0..."
if ls /dev/ttyACM* 1> /dev/null 2>&1; then
  echo "✅ Teensy detected. Flashing firmware..."
  teensy-loader-cli --mcu=TEENSY40 -w -v ltc_audiohat_lock.ino.hex
  echo "✅ Firmware uploaded successfully."
else
  echo "⚠️ No Teensy detected on /dev/ttyACM* — firmware not flashed."
fi

# ---------------------------------------------------------
# Final Message
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete — Time magic is in place."
echo "─────────────────────────────────────────────"
echo "Teensy tools are installed, firmware is ready."
echo "Your Raspberry Pi is now ready for testing."
echo ""
