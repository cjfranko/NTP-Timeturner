#!/bin/bash

echo "✨ Welcome to the NTP Timeturner Installer"
echo "Preparing the Time Room... Stand by."

# ─────────────────────────────────────────────
# Step 1: Update System
# ─────────────────────────────────────────────
echo "Step 1: Updating system packages..."
sudo apt update && sudo apt upgrade -y

# ─────────────────────────────────────────────
# Step 2: Install Core Dependencies
# ─────────────────────────────────────────────
echo "Step 2: Installing core dependencies..."
sudo apt install -y \
    git cmake build-essential \
    libjack-jackd2-dev libsamplerate0-dev \
    libasound2-dev libsndfile1-dev \
    python3 python3-pip python3-numpy python3-matplotlib

# ─────────────────────────────────────────────
# Step 3: Install Python Audio Libraries
# ─────────────────────────────────────────────
echo "Step 3: Installing Python audio libraries..."
pip3 install sounddevice

# ─────────────────────────────────────────────
# Step 4: Install Splash Screen
# ─────────────────────────────────────────────
echo "Step 4: Installing custom splash screen..."
sudo cp splash.png /usr/share/plymouth/themes/pix/splash.png

# ─────────────────────────────────────────────
# Step 5: Build libltc
# ─────────────────────────────────────────────
echo "Step 5: Building libltc (the heart of our time-magic)..."
cd ~
if [ ! -d "libltc" ]; then
  git clone https://github.com/x42/libltc.git
fi
cd libltc
cmake .
make
sudo make install
sudo ldconfig

# ─────────────────────────────────────────────
# Step 6: Build ltc-tools
# ─────────────────────────────────────────────
echo "Step 6: Building ltc-tools..."
cd ~
if [ ! -d "ltc-tools" ]; then
  git clone https://github.com/x42/ltc-tools.git
fi
cd ltc-tools
make
sudo make install

# ─────────────────────────────────────────────
# Step 7: Set Hostname
# ─────────────────────────────────────────────
echo "Step 7: Configuring hostname..."
sudo hostnamectl set-hostname ntp-timeturner

# ─────────────────────────────────────────────
# Complete
# ─────────────────────────────────────────────
echo "✨ Installation complete."
echo "System will reboot in 30 seconds unless you press [Enter] to reboot now."
read -t 30 -p "Press [Enter] to reboot now or wait..." input
sudo reboot
