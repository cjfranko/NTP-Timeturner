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
sudo apt install -y git curl python3 python3-pip build-essential cmake

# ---------------------------------------------------------
# Step 3: Install audio and media dependencies
# ---------------------------------------------------------
echo ""
echo "Step 3: Installing audio libraries and tools..."
sudo apt install -y alsa-utils ffmpeg portaudio19-dev python3-pyaudio libasound2-dev libjack-jackd2-dev || echo "Warning: Some audio dependencies may have failed to install, continuing..."

# ---------------------------------------------------------
# Step 4: Install Python packages
# ---------------------------------------------------------
echo ""
echo "Step 4: Installing Python packages..."
pip3 install numpy

# ---------------------------------------------------------
# Step 5: Check for or install ltc-tools
# ---------------------------------------------------------
echo ""
echo "Step 5: Verifying LTC tools..."
if ! command -v ltcdump >/dev/null 2>&1; then
  echo "ltc-tools not found, building from source..."
  cd ~
  if [ ! -d "libltc" ]; then
    echo "Cloning libltc from GitHub..."
    git clone https://github.com/x42/libltc.git
  fi
  cd libltc
  mkdir -p build && cd build
  echo "Running CMake configuration..."
  cmake ..
  echo "Compiling libltc..."
  make
  echo "Installing libltc binaries..."
  sudo make install
  sudo ldconfig
else
  echo "ltc-tools already installed."
fi

# ---------------------------------------------------------
# Final Message
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete"
echo "─────────────────────────────────────────────"
echo ""
echo "The TimeTurner is ready. But remember:"
echo "\"You must not be seen.\" – Hermione Granger"
echo ""
