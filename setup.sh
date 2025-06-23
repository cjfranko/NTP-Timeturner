#!/bin/bash
set -e

echo "🔧 Installing dependencies for NTP Timeturner..."
echo "Hermione, Time to Twist the Timeturner..."

# Update and upgrade packages
sudo apt update && sudo apt upgrade -y

# Essential tools
sudo apt install -y git curl python3 python3-pip build-essential

# Audio tools
sudo apt install -y alsa-utils ffmpeg portaudio19-dev python3-pyaudio

# LTC decoding tools
sudo apt install -y ltc-tools

# Optional: Network management (if needed later)
# sudo apt install -y network-manager

# Python packages
pip3 install numpy

echo "✅ Setup complete. Reboot recommended if this is the first run."
