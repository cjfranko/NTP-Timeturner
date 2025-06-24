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
sudo apt install -y alsa-utils ffmpeg \
  portaudio19-dev python3-pyaudio \
  libasound2-dev libjack-jackd2-dev \
  libsndfile-dev \
  python3-numpy python3-matplotlib python3-sounddevice \
  || echo "Warning: Some audio dependencies may have failed to install — continuing anyway."

# ---------------------------------------------------------
# Step 4: Build and install libltc (needed by ltc-tools)
# ---------------------------------------------------------
echo ""
echo "Step 4: Building libltc (the heart of our time-magic)..."
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
# Step 5: Build and install ltc-tools
# ---------------------------------------------------------
echo ""
echo "Step 5: Building ltc-tools (with a gentle nudge)..."
cd ~
if [ ! -d "ltc-tools" ]; then
  echo "Cloning ltc-tools from GitHub..."
  git clone https://github.com/x42/ltc-tools.git
fi
cd ltc-tools

echo "Compiling ltc-tools (bypassing package check)..."
make HAVE_LIBLTC=true

echo "Installing ltc-tools..."
sudo make install
sudo ldconfig

# ---------------------------------------------------------
# Step 6: Apply Custom Splash Screen
# ---------------------------------------------------------
echo ""
echo "Step 6: Applying splash screen..."
if [ -f "/home/hermione/splash.png" ]; then
  sudo cp /home/hermione/splash.png /usr/share/plymouth/themes/pix/splash.png
  sudo chmod 644 /usr/share/plymouth/themes/pix/splash.png
  echo "✅ Splash screen updated."
else
  echo "⚠️ splash.png not found — skipping splash screen update."
fi

# ---------------------------------------------------------
# Step 7: Make Python scripts executable
# ---------------------------------------------------------
echo ""
echo "Step 7: Making *.py scripts executable (if any)..."
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
