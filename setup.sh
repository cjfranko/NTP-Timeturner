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
  || echo "Warning: Some audio dependencies may have failed to install — continuing anyway."

# ---------------------------------------------------------
# Step 4: Install Python packages
# ---------------------------------------------------------
echo ""
echo "Step 4: Installing Python packages..."
pip3 install numpy --break-system-packages

# ---------------------------------------------------------
# Step 5: Build and install libltc (needed by ltc-tools)
# ---------------------------------------------------------
echo ""
echo "Step 5: Building libltc (the heart of our time-magic)..."
cd ~
if [ ! -d "libltc" ]; then
  echo "Cloning libltc from GitHub..."
  git clone https://github.com/x42/libltc.git
fi
cd libltc

echo "Installing libltc build dependencies..."
./autogen.sh
./configure

echo "Compiling libltc..."
make

echo "Installing libltc..."
sudo make install
sudo ldconfig

# ---------------------------------------------------------
# Step 6: Build and install ltc-tools
# ---------------------------------------------------------
echo ""
echo "Step 6: Building ltc-tools (with a gentle nudge)..."
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
# Final Message
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete"
echo "─────────────────────────────────────────────"
echo ""
echo "The TimeTurner is ready. But remember:"
echo "\"You must not be seen.\" – Hermione Granger"
echo "Guidance provided by Luna, Department of Temporal Engineering."
echo ""
