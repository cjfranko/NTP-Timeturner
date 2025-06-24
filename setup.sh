#!/bin/bash

# NTP Timeturner Setup Script
# Tested on Debian Bookworm - Raspberry Pi 3
# Author: cjfranko

echo "Step 1: Updating system packages..."
sudo apt-get update && sudo apt-get upgrade -y

echo "Step 2: Installing required system packages..."
sudo apt-get install -y git cmake build-essential libjack-jackd2-dev \
                        libsndfile1-dev libtool autoconf automake \
                        pkg-config libasound2-dev libfftw3-dev \
                        python3-full python3-venv python3-pip \
                        libltc-dev python3-numpy python3-matplotlib python3-sounddevice

echo "Step 3: Cloning libltc and ltc-tools..."
cd /home/hermione
git clone https://github.com/x42/libltc.git
git clone https://github.com/x42/ltc-tools.git

echo "Step 4: Building libltc (the heart of our time-magic)..."
cd libltc
mkdir -p build && cd build
cmake ..
make -j$(nproc)
sudo make install
sudo ldconfig

echo "Step 5: Building ltc-tools..."
cd /home/hermione/ltc-tools
make -j$(nproc)
sudo make install

echo "Step 6: Setting splash screen..."
sudo cp /home/hermione/splash.png /usr/share/plymouth/themes/pix/splash.png

echo "Step 7: Making timeturner scripts executable..."
chmod +x /home/hermione/*.py

echo "Step 8: Setup complete. System will reboot in 30 seconds unless you press Enter..."
echo "Press Ctrl+C or Enter now to cancel automatic reboot."
read -t 30 -p ">> " input && sudo reboot || sudo reboot
