#!/bin/bash
set -e

echo ""
echo "─────────────────────────────────────────────"
echo "  Welcome to the NTP TimeTurner Installer"
echo "─────────────────────────────────────────────"
echo ""
echo "\"It's a very complicated piece of magic...\" – Hermione Granger"
echo "Preparing the Ministry-grade temporal interface..."
echo ""

# ---------------------------------------------------------
# Step 1: Update and upgrade packages
# ---------------------------------------------------------
echo "Step 1: Updating package lists and upgrading..."
sudo apt update && sudo apt upgrade -y

# ---------------------------------------------------------
# Step 2: Install core tools and Python dependencies
# ---------------------------------------------------------
echo "Step 2: Installing required tools..."
sudo apt install -y git curl python3 python3-pip build-essential cmake \
  python3-serial libusb-dev

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
sudo cp teensy_loader_cli /usr/local/bin/teensy-loader-cli

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
echo "✅ Teensy udev rules installed. Reboot required to take effect."

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
# Step 5: Download Teensy firmware
# ---------------------------------------------------------
echo "Step 5: Downloading Teensy firmware..."
cd "$HOME"
wget -O ltc_audiohat_lock.ino.hex https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/master/firmware/ltc_audiohat_lock.ino.hex

# ---------------------------------------------------------
# Step 6: Create flash script and one-time service
# ---------------------------------------------------------
echo "Step 6: Creating one-time flash script and service..."
cat << 'EOF' > "$HOME/flash_teensy_once.sh"
#!/bin/bash
echo "🔧 Running one-time Teensy flash..."
if ls /dev/ttyACM* 1> /dev/null 2>&1; then
  echo "✅ Teensy detected. Flashing firmware..."
  /usr/local/bin/teensy-loader-cli --mcu=TEENSY40 -w -v "$HOME/ltc_audiohat_lock.ino.hex"
  echo "✅ Firmware upload complete."
else
  echo "⚠️ No Teensy detected on /dev/ttyACM* — skipping flash."
fi

# Clean up: remove script and disable this service
rm -- "$HOME/flash_teensy_once.sh"
systemctl --user disable flash-teensy.service
rm -- "$HOME/.config/systemd/user/flash-teensy.service"
EOF

chmod +x "$HOME/flash_teensy_once.sh"

mkdir -p "$HOME/.config/systemd/user"
cat << EOF > "$HOME/.config/systemd/user/flash-teensy.service"
[Unit]
Description=One-time Teensy Flash
After=default.target

[Service]
Type=oneshot
ExecStart=$HOME/flash_teensy_once.sh
RemainAfterExit=true

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reexec
systemctl --user daemon-reload
systemctl --user enable flash-teensy.service

echo "✅ One-time flash service scheduled to run after reboot."

# ---------------------------------------------------------
# Final Message & Auto-Reboot
# ---------------------------------------------------------
echo ""
echo "─────────────────────────────────────────────"
echo "  Setup Complete — Rebooting in 15 seconds..."
echo "─────────────────────────────────────────────"
echo "Teensy will be flashed automatically on next boot (once)."
echo ""
sleep 15
sudo reboot
