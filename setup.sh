#!/bin/bash
set -e

echo ""
echo "─────────────────────────────────────────────"
echo "  Welcome to the NTP Timeturner Installer"
echo "─────────────────────────────────────────────"
echo ""
echo "\"It's a very complicated piece of magic...\" – Hermione Granger"
echo "Initialising temporal calibration sequence..."
echo "Requesting clearance from the Ministry of Time Standards..."
echo ""

# ---------------------------------------------------------
# Step 1: Update and upgrade packages
# ---------------------------------------------------------
echo ""
echo "Step 1: Updating package lists..."
sudo apt update

echo "Upgrading installed packages..."
sudo apt upgrade -y

# ---------------------------------------------------------
# Step 2: Install essential development tools
# ---------------------------------------------------------
echo ""
echo "Step 2: Installing development tools..."
sudo apt install -y git curl python3 python3-pip build-essential autoconf automake libtool cmake

# ---------------------------------------------------------
# Step 3: Install audio and media dependencies
# ---------------------------------------------------------
echo ""
echo "Step 3: Installing audio libraries and tools..."
sudo apt install -y alsa-utils ffmpeg   portaudio19-dev python3-pyaudio   libasound2-dev libjack-jackd2-dev   libsndfile-dev   python3-numpy python3-matplotlib   || echo "Warning: Some audio dependencies may have failed to install — continuing anyway."

echo ""
echo "Installing 'sounddevice' with pip3 (system-wide)..."
pip3 install --break-system-packages sounddevice

# ---------------------------------------------------------
# Step 4: Download NTP Timeturner scripts and assets
# ---------------------------------------------------------
echo ""
echo "Step 4: Downloading scripts and splash screen from GitHub..."
cd /home/hermione

wget -O timeturner.py https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/timeturner.py
wget -O ltc_probe.py https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/ltc_probe.py
wget -O test_audioinput.py https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/test_audioinput.py
wget -O splash.png https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/splash.png

# ---------------------------------------------------------
# Step 5: Build and install libltc (the heart of our time-magic)
# ---------------------------------------------------------
echo ""
echo "Step 5: Building libltc..."
cd ~
if [ ! -d "libltc" ]; then
  echo "Cloning libltc from GitHub..."
  git clone https://github.com/x42/libltc.git
fi
cd libltc

if [ ! -f "./configure" ]; then
  echo "Running autogen.sh to prepare build..."
  ./autogen.sh
fi

echo "Configuring libltc..."
./configure

echo "Compiling libltc..."
make

echo "Installing libltc..."
sudo make install
sudo ldconfig

# ---------------------------------------------------------
# Step 6: Clone and build custom ltc-tools (with ltcstream)
# ---------------------------------------------------------
echo ""
echo "Step 6: Building custom ltc-tools (ltc-tools-timeturner)..."
cd ~
if [ ! -d "ltc-tools-timeturner" ]; then
  echo "Cloning your custom ltc-tools fork..."
  git clone https://github.com/cjfranko/ltc-tools-timeturner.git
fi
cd ltc-tools-timeturner

echo "Compiling ltc-tools and ltcstream..."
make HAVE_LIBLTC=true LOADLIBES="-lasound"

echo "Installing ltc-tools..."
sudo make install
sudo ldconfig

# ---------------------------------------------------------
# Step 7: Apply Custom Splash Screen
# ---------------------------------------------------------
echo ""
echo "Step 7: Applying splash screen..."
if [ -f "/home/hermione/splash.png" ]; then
  sudo cp /home/hermione/splash.png /usr/share/plymouth/themes/pix/splash.png
  sudo chmod 644 /usr/share/plymouth/themes/pix/splash.png
  echo "✅ Splash screen updated."
else
  echo "⚠️ splash.png not found — skipping splash screen update."
fi

# ---------------------------------------------------------
# Step 8: Make Python scripts executable
# ---------------------------------------------------------
echo ""
echo "Step 8: Making *.py scripts executable..."
shopt -s nullglob
PYFILES=(/home/hermione/*.py)
if [ ${#PYFILES[@]} -gt 0 ]; then
  chmod +x /home/hermione/*.py
  echo "✅ Python scripts marked executable."
else
  echo "⚠️ No Python scripts found."
fi
shopt -u nullglob

# ---------------------------------------------------------
# Final Message & Reboot Option
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete"
echo "─────────────────────────────────────────────"
echo ""
echo "The TimeTurner is ready. But remember:"
echo "\"You must not be seen.\" – Hermione Granger"
echo "Visual enhancements are in place. Terminal timeline is stable."
echo ""
echo "The system will reboot in 30 seconds to complete setup..."
echo "Press [Enter] to reboot immediately, or Ctrl+C to cancel."

read -t 30 -p "" || true
sleep 1
sudo reboot
