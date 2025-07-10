#!/bin/bash
set -e

echo ""
echo "─────────────────────────────────────────────"
echo "  Welcome to the NTP TimeTurner Installer"
echo "─────────────────────────────────────────────"
echo ""
echo "\"It's a very complicated piece of magic...\" – Hermione Granger"
echo "Preparing the Ministry-grade temporal interface with PTP precision..."
echo ""

# ---------------------------------------------------------
# Step 1: Update and upgrade packages
# ---------------------------------------------------------
echo "Step 1: Updating package lists and upgrading..."
sudo apt update && sudo apt upgrade -y

# ---------------------------------------------------------
# Step 2: Install core tools and dependencies
# ---------------------------------------------------------
echo "Step 2: Installing required tools and PTP dependencies..."
sudo apt install -y git curl python3 python3-pip build-essential cmake \
  python3-serial libusb-dev linuxptp ethtool

# ---------------------------------------------------------
# Step 2.5: Install teensy-loader-cli from source
# ---------------------------------------------------------
echo "Installing teensy-loader-cli manually from source..."
cd "$HOME"
if [ ! -d teensy_loader_cli ]; then
  git clone https://github.com/PaulStoffregen/teensy_loader_cli.git
fi
cd teensy_loader_cli
make
sudo install -m 755 teensy_loader_cli /usr/local/bin/teensy-loader-cli

echo "Verifying teensy-loader-cli..."
teensy-loader-cli --version || echo "⚠️ teensy-loader-cli failed to install properly"

# ---------------------------------------------------------
# Step 2.6: Install udev rules for Teensy
# ---------------------------------------------------------
echo "Installing udev rules for Teensy access..."
cd "$HOME"
wget -O 49-teensy.rules https://www.pjrc.com/teensy/49-teensy.rules
sudo cp 49-teensy.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
echo "✅ Teensy udev rules installed. Reboot required to take full effect."

# ---------------------------------------------------------
# Step 2.7: Configure PTP hardware timestamping support
# ---------------------------------------------------------
echo "Configuring PTP hardware timestamping support..."
# Enable hardware timestamping on network interfaces if supported
sudo ethtool -T eth0 2>/dev/null | grep -q "hardware-transmit" && echo "✅ Hardware timestamping supported on eth0" || echo "⚠️ Hardware timestamping not available on eth0"

# ---------------------------------------------------------
# Step 3: Install Arduino CLI manually (latest version)
# ---------------------------------------------------------
echo "Step 3: Downloading and installing arduino-cli..."
cd "$HOME"
curl -fsSL https://downloads.arduino.cc/arduino-cli/arduino-cli_latest_Linux_ARM64.tar.gz -o arduino-cli.tar.gz
tar -xzf arduino-cli.tar.gz
sudo mv arduino-cli /usr/local/bin/
rm arduino-cli.tar.gz

echo "Verifying arduino-cli install..."
arduino-cli version || echo "⚠️ arduino-cli install failed or not found in PATH"

# ---------------------------------------------------------
# Step 4: Download and apply splash screen
# ---------------------------------------------------------
echo "Step 4: Downloading and applying splash screen..."
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
# Step 4.5: Configure Plymouth to stay on screen longer
# ---------------------------------------------------------
echo "Step 4.5: Configuring splash screen timing..."

# Ensure 'quiet splash' is in /boot/cmdline.txt
sudo sed -i 's/\(\s*\)console=tty1/\1quiet splash console=tty1/' /boot/cmdline.txt
echo "✅ Set 'quiet splash' in /boot/cmdline.txt"

# Update Plymouth config
sudo sed -i 's/^Theme=.*/Theme=pix/' /etc/plymouth/plymouthd.conf
sudo sed -i 's/^ShowDelay=.*/ShowDelay=0/' /etc/plymouth/plymouthd.conf || echo "ShowDelay=0" | sudo tee -a /etc/plymouth/plymouthd.conf
sudo sed -i 's/^DeviceTimeout=.*/DeviceTimeout=10/' /etc/plymouth/plymouthd.conf || echo "DeviceTimeout=10" | sudo tee -a /etc/plymouth/plymouthd.conf
sudo sed -i 's/^DisableFadeIn=.*/DisableFadeIn=true/' /etc/plymouth/plymouthd.conf || echo "DisableFadeIn=true" | sudo tee -a /etc/plymouth/plymouthd.conf
echo "✅ Updated /etc/plymouth/plymouthd.conf"

# Create autostart delay to keep splash visible until desktop is ready
mkdir -p "$HOME/.config/autostart"
cat << EOF > "$HOME/.config/autostart/delayed-plymouth-exit.desktop"
[Desktop Entry]
Type=Application
Name=Delayed Plymouth Exit
Exec=/bin/sh -c "sleep 3 && /usr/bin/plymouth quit"
X-GNOME-Autostart-enabled=true
EOF
echo "✅ Splash screen will exit 3 seconds after desktop starts"

# ---------------------------------------------------------
# Step 5: Download Teensy firmware
# ---------------------------------------------------------
echo "Step 5: Downloading Teensy firmware..."
cd "$HOME"
wget -O ltc_audiohat_lock.ino.hex https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/firmware/ltc_audiohat_lock.ino.hex

# ---------------------------------------------------------
# Final Message & Reboot
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete — Rebooting in 15 seconds..."
echo "─────────────────────────────────────────────"
echo "NOTE: Teensy firmware ready in $HOME, but not auto-flashed."
echo "Boot splash will remain until desktop loads."
echo ""
echo "PTP Integration Features:"
echo "• IEEE 1588 PTP v2 client for sub-microsecond precision"
echo "• Hardware timestamping support (if available)"
echo "• Real-time offset monitoring and jitter measurement"
echo "• Configurable via config.json (ptp_enabled, ptp_interface)"
echo ""
sleep 15
sudo reboot
