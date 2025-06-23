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
sudo apt install -y git curl python3 python3-pip build-essential

# Audio tools
sudo apt install -y alsa-utils ffmpeg portaudio19-dev python3-pyaudio

# LTC decoding tools
sudo apt install -y ltc-tools

# Python packages
pip3 install numpy

echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete"
echo "─────────────────────────────────────────────"
echo ""
echo "The TimeTurner is ready. But remember:"
echo "\"You must not be seen.\" – Hermione Granger"
echo ""
