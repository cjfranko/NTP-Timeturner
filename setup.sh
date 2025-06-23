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

# Update and upgrade packages
sudo apt update && sudo apt upgrade -y

# Essential tools
sudo apt install -y git curl python3 python3-pip build-essential cmake

# Audio tools
sudo apt install -y alsa-utils ffmpeg portaudio19-dev python3-pyaudio libasound2-dev libjack-jackd2-dev

# Python packages
pip3 install numpy

# Install ltc-tools from source if not available
if ! command -v ltcdump >/dev/null 2>&1; then
  echo "ltc-tools not found, building from source..."
  cd ~
  if [ ! -d "libltc" ]; then
    git clone https://github.com/x42/libltc.git
  fi
  cd libltc
  mkdir -p build && cd build
  cmake ..
  make
  sudo make install
  sudo ldconfig
else
  echo "ltc-tools already installed."
fi

echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete"
echo "─────────────────────────────────────────────"
echo ""
echo "The TimeTurner is ready. But remember:"
echo "\"You must not be seen.\" – Hermione Granger"
echo ""
