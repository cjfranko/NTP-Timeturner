#!/bin/bash
set -e

echo "--- TimeTurner Setup ---"

# Determine package manager
PKG_MANAGER=""
if command -v apt &> /dev/null; then
    PKG_MANAGER="apt"
elif command -v dnf &> /dev/null; then
    PKG_MANAGER="dnf"
elif command -v pacman &> /dev/null; then
    PKG_MANAGER="pacman"
else
    echo "Error: No supported package manager (apt, dnf, pacman) found. Please install dependencies manually."
    exit 1
fi

echo "Detected package manager: $PKG_MANAGER"

# --- Update System Packages ---
echo "Updating system packages..."
if [ "$PKG_MANAGER" == "apt" ]; then
    sudo apt update
    sudo DEBIAN_FRONTEND=noninteractive apt upgrade -y -o Dpkg::Options::="--force-confold"
elif [ "$PKG_MANAGER" == "dnf" ]; then
    sudo dnf upgrade -y
elif [ "$PKG_MANAGER" == "pacman" ]; then
    sudo pacman -Syu --noconfirm
fi
echo "System packages updated."

# --- Install Rust/Cargo if not installed ---
if ! command -v cargo &> /dev/null; then
    echo "Rust/Cargo not found. Installing Rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Source cargo's env for the current shell session
    # This is for the current script's execution path, typically rustup adds to .bashrc/.profile for future sessions.
    # We need it now, but for non-interactive script, sourcing won't affect parent shell.
    # However, cargo build below will rely on it being in PATH. rustup makes sure of this if it installs.
    # For safety, ensure PATH is updated.
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "Rust/Cargo installed successfully."
else
    echo "Rust/Cargo is already installed."
fi

# --- Install common build dependencies for Rust ---
echo "Installing common build dependencies..."
if [ "$PKG_MANAGER" == "apt" ]; then
    sudo apt update
    sudo apt install -y build-essential libudev-dev pkg-config curl wget
elif [ "$PKG_MANAGER" == "dnf" ]; then
    sudo dnf install -y gcc make perl-devel libudev-devel pkg-config curl wget
elif [ "$PKG_MANAGER" == "pacman" ]; then
    sudo pacman -Sy --noconfirm base-devel libudev pkg-config curl
fi
echo "Common build dependencies installed."

# --- Install Python dependencies for testing ---
echo "🐍 Installing Python dependencies for test scripts..."
if [ "$PKG_MANAGER" == "apt" ]; then
    # python3-serial is the name for pyserial in apt
    sudo apt install -y python3 python3-pip python3-serial
elif [ "$PKG_MANAGER" == "dnf" ]; then
    # python3-pyserial is the name for pyserial in dnf
    sudo dnf install -y python3 python3-pip python3-pyserial
elif [ "$PKG_MANAGER" == "pacman" ]; then
    # python-pyserial is the name for pyserial in pacman
    sudo pacman -Sy --noconfirm python python-pip python-pyserial
fi
# sudo pip3 install pyserial # This is replaced by the native package manager installs above
echo "✅ Python dependencies installed."

# --- Apply custom splash screen ---
if [[ "$(uname)" == "Linux" ]]; then
    echo "🖼️  Applying custom splash screen..."
    SPLASH_URL="https://raw.githubusercontent.com/cjfranko/NTP-Timeturner/refs/heads/main/splash.png"
    PLYMOUTH_THEME_DIR="/usr/share/plymouth/themes/pix"
    PLYMOUTH_IMAGE_PATH="${PLYMOUTH_THEME_DIR}/splash.png"

    sudo mkdir -p "${PLYMOUTH_THEME_DIR}"
    echo "Downloading splash image from ${SPLASH_URL}..."
    sudo curl -L "${SPLASH_URL}" -o "${PLYMOUTH_IMAGE_PATH}"
    
    if [ -f "${PLYMOUTH_IMAGE_PATH}" ]; then
        echo "Splash image downloaded. Updating Plymouth configuration..."
        # Set 'pix' as the default plymouth theme if not already.
        # This is a common theme that expects splash.png.
        sudo update-alternatives --install /usr/share/plymouth/themes/default.plymouth default.plymouth "${PLYMOUTH_THEME_DIR}/pix.plymouth" 100 || true
        # Ensure the pix theme exists and is linked
        if [ ! -f "${PLYMOUTH_THEME_DIR}/pix.plymouth" ]; then
             echo "Creating dummy pix.plymouth for update-initramfs"
             echo "[Plymouth Theme]" | sudo tee "${PLYMOUTH_THEME_DIR}/pix.plymouth" > /dev/null
             echo "Name=Pi Splash" | sudo tee -a "${PLYMOUTH_THEME_DIR}/pix.plymouth" > /dev/null
             echo "Description=TimeTurner Raspberry Pi Splash Screen" | sudo tee -a "${PLYMOUTH_THEME_DIR}/pix.plymouth" > /dev/null
             echo "SpriteAnimation=/splash.png" | sudo tee -a "${PLYMOUTH_THEME_DIR}/pix.plymouth" > /dev/null
        fi

        # Update the initial RAM filesystem to include the new splash screen
        sudo update-initramfs -u
        echo "✅ Custom splash screen applied. Reboot may be required to see changes."
    else
        echo "❌ Failed to download splash image from ${SPLASH_URL}."
    fi
else
    echo "⚠️  Skipping splash screen configuration on non-Linux OS."
fi

# --- Remove NTPD and install Chrony, NMTUI, Adjtimex ---
echo "Removing NTPD (if installed) and installing Chrony, NMTUI, Adjtimex..."

# --- Remove NTPD and install Chrony, NMTUI, Adjtimex ---
echo "Removing NTPD (if installed) and installing Chrony, NMTUI, Adjtimex..."

if [ "$PKG_MANAGER" == "apt" ]; then
    sudo apt update
    sudo apt remove -y ntp || true # Remove ntp if it exists, ignore if not
    sudo apt install -y chrony network-manager adjtimex
    sudo systemctl enable chrony --now
elif [ "$PKG_MANAGER" == "dnf" ]; then
    sudo dnf remove -y ntp
    sudo dnf install -y chrony NetworkManager-tui adjtimex
    sudo systemctl enable chronyd --now
elif [ "$PKG_MANAGER" == "pacman" ]; then
    sudo pacman -Sy --noconfirm ntp || true
    sudo pacman -R --noconfirm ntp || true # Ensure ntp is removed
    sudo pacman -Sy --noconfirm chrony networkmanager adjtimex
    sudo systemctl enable chronyd --now
    sudo systemctl enable NetworkManager --now # nmtui relies on NetworkManager
fi

echo "NTPD removed (if present). Chrony, NMTUI, and Adjtimex installed and configured."

# --- Install and configure WiFi hotspot and captive portal ---
echo "📡 Installing and configuring WiFi hotspot and captive portal..."

if [ "$PKG_MANAGER" == "apt" ]; then
    # Stop the service if it's running from a previous installation to prevent "Text file busy" error.
    echo "Stopping existing nodogsplash service before installation..."
    sudo systemctl stop nodogsplash || true

    # We will use dnsmasq for DHCP, as the compiled nodogsplash version may not support the internal DHCP server.
    # sudo apt-get remove --purge -y dnsmasq || true # This line is no longer needed.

    # Install dependencies for hotspot and for building nodogsplash.
    sudo apt install -y hostapd dnsmasq git libmicrohttpd-dev libjson-c-dev iptables
    
    # Force iptables-legacy for nodogsplash
    echo "Setting iptables-legacy mode for nodogsplash..."
    sudo update-alternatives --set iptables /usr/sbin/iptables-legacy
    sudo update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy

    echo "Building and installing nodogsplash from source..."
    # Create a temporary directory for the build
    BUILD_DIR=$(mktemp -d)
    git clone https://github.com/nodogsplash/nodogsplash.git "$BUILD_DIR"
    
    # Run the build in a subshell to isolate the directory change
    (
        cd "$BUILD_DIR"
        make
        sudo make install
        
        # Manually install the systemd service file as 'make install' might not do it.
        # This makes the script more robust.
        if [ -f "debian/nodogsplash.service" ]; then
            echo "Manually installing systemd service file..."
            sudo cp debian/nodogsplash.service /etc/systemd/system/nodogsplash.service
            # Reload systemd to recognize the new service
            sudo systemctl daemon-reload
        else
            echo "⚠️  Warning: nodogsplash.service file not found in source. Cannot set up as a service."
        fi
    )

    # Clean up the build directory
    sudo rm -rf "$BUILD_DIR"
    echo "✅ nodogsplash installed successfully."

    # Disable the standalone hostapd service to let NetworkManager manage it.
    sudo systemctl disable hostapd
    sudo systemctl mask hostapd
    sudo systemctl enable nodogsplash
else
    echo "This script is designed for Debian-based systems like Raspberry Pi OS."
    echo "Skipping WiFi hotspot setup."
fi

# Stop services to configure
# Ensure services exist before trying to stop them
sudo systemctl stop hostapd || true
sudo systemctl stop dnsmasq || true

# Ensure NetworkManager is managing wlan0 by removing any conflicting configurations.
# This is the critical fix for the "No suitable device" error.
echo "Ensuring NetworkManager is managing wlan0..."
sudo rm -f /etc/NetworkManager/conf.d/99-unmanaged-wlan0.conf
sudo systemctl reload NetworkManager

# Configure static IP for wlan0 using NetworkManager (nmcli)
echo "Configuring static IP for wlan0 using NetworkManager..."

# Define the connection name
CON_NAME="TimeTurner-AP"

# If a connection with this name already exists, delete it to ensure a clean slate.
if nmcli c show --active | grep -q "$CON_NAME"; then
    sudo nmcli c down "$CON_NAME" || true
fi
if nmcli c show | grep -q "$CON_NAME"; then
    echo "Deleting existing '$CON_NAME' connection profile..."
    sudo nmcli c delete "$CON_NAME" || true
fi

# Create a new connection profile for the Access Point with a static IP.
echo "Creating new '$CON_NAME' connection profile..."
sudo nmcli c add type wifi ifname wlan0 con-name "$CON_NAME" autoconnect yes ssid "TimeTurner"
sudo nmcli c modify "$CON_NAME" 802-11-wireless.mode ap 802-11-wireless.band bg
sudo nmcli c modify "$CON_NAME" 802-11-wireless-security.key-mgmt wpa-psk
sudo nmcli c modify "$CON_NAME" 802-11-wireless-security.psk "harry-ron-hermione"
sudo nmcli c modify "$CON_NAME" ipv4.method manual ipv4.addresses 10.0.252.1/24

# Configure dnsmasq for DHCP
echo "Configuring dnsmasq..."
sudo tee /etc/dnsmasq.conf > /dev/null <<EOF
interface=wlan0
dhcp-range=10.0.252.10,10.0.252.50,255.255.255.0,24h
address=/#/10.0.252.1
EOF

# Configure nodogsplash for captive portal
echo "Configuring nodogsplash..."
sudo tee /etc/nodogsplash/nodogsplash.conf > /dev/null <<EOF
GatewayInterface wlan0
GatewayAddress 10.0.252.1
MaxClients 250
AuthIdleTimeout 480
FirewallRuleSet preauthenticated-users {
    FirewallRule allow tcp port 80
    FirewallRule allow tcp port 53
    FirewallRule allow udp port 53
}
RedirectURL http://10.0.252.1/static/index.html
EOF

# Restart services in the correct order and add delays to prevent race conditions
echo "Restarting services..."
# Bring up the new AP connection using nmcli
sudo nmcli c up "$CON_NAME"

# Wait for the interface to come up and get the IP address
echo "Waiting for wlan0 to be configured..."
IP_CHECK=""
# Loop for up to 30 seconds waiting for the IP
for i in {1..15}; do
    # The '|| true' prevents the script from exiting if grep finds no match
    IP_CHECK=$(ip -4 addr show wlan0 | grep -oP '(?<=inet\s)\d+(\.\d+){3}' || true)
    if [ "$IP_CHECK" == "10.0.252.1" ]; then
        break
    fi
    echo "Still waiting for IP..."
    sleep 2
done

# Check for the IP address before starting nodogsplash
if [ "$IP_CHECK" == "10.0.252.1" ]; then
    echo "✅ wlan0 configured with IP $IP_CHECK."
    sudo systemctl restart dnsmasq
    if command -v nodogsplash &> /dev/null; then
        echo "Attempting to start nodogsplash service..."
        if ! sudo systemctl restart nodogsplash; then
            echo "❌ nodogsplash service failed to start. Displaying logs..."
            # Give a moment for logs to be written
            sleep 2
            sudo journalctl -u nodogsplash.service --no-pager -n 50
            echo ""
            echo "To debug further, run nodogsplash in the foreground with this command:"
            echo "  sudo /usr/bin/nodogsplash -f -d 7"
            echo ""
            exit 1
        fi
        echo "✅ nodogsplash service started successfully."
    fi
else
    echo "❌ Error: wlan0 failed to get the static IP 10.0.252.1. Found: '$IP_CHECK'."
    echo "Please check 'sudo nmcli c show \"$CON_NAME\"' and 'ip addr show wlan0'."
    exit 1
fi

echo "✅ WiFi hotspot and captive portal configured. SSID: TimeTurner, IP: 10.0.252.1"
echo "Clients will be redirected to http://10.0.252.1/static/index.html"

# 1. Build the release binary
echo "📦 Building release binary with Cargo..."
# No need to check for cargo again, as it's handled above
cargo build --release
echo "✅ Build complete."

# 2. Create installation directories
INSTALL_DIR="/opt/timeturner"
BIN_DIR="/usr/local/bin"
echo "🔧 Creating directories..."
sudo mkdir -p $INSTALL_DIR
echo "✅ Directory $INSTALL_DIR created."

# 3. Install binary and static web files
echo "🚀 Installing timeturner binary and web assets..."
sudo cp target/release/ntp_timeturner $INSTALL_DIR/timeturner
# The static directory contains the web UI files
sudo cp -r static $INSTALL_DIR/
sudo ln -sf $INSTALL_DIR/timeturner $BIN_DIR/timeturner
echo "✅ Binary and assets installed to $INSTALL_DIR, and binary linked to $BIN_DIR."

# 4. Install systemd service file
# Only needed for Linux systems (e.g., Raspberry Pi OS)
if [[ "$(uname)" == "Linux" ]]; then
    echo "⚙️  Installing systemd service for Linux..."
    sudo cp timeturner.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable timeturner.service
    echo "✅ Systemd service installed and enabled."
else
    echo "⚠️  Skipping systemd service installation on non-Linux OS."
fi

echo ""
echo "--- Setup Complete ---"
echo "The TimeTurner daemon is now installed."
echo "The working directory is $INSTALL_DIR."
# Copy default config.yml from repo if it exists
if [ -f config.yml ]; then
    sudo cp config.yml $INSTALL_DIR/
    echo "Default 'config.yml' installed to $INSTALL_DIR."
else
    echo "⚠️  No default 'config.yml' found in repository. Please add one if needed."
fi
echo ""
if [[ "$(uname)" == "Linux" ]]; then
    echo "To start the service, run:"
    echo "  sudo systemctl start timeturner.service"
    echo ""
    echo "To view live logs, run:"
    echo "  journalctl -u tim_turner.service -f"
    echo ""
fi
echo "To run the interactive TUI instead, simply run from the project directory:"
echo "  cargo run"
echo "Or from anywhere after installation:"
echo "  timeturner"
echo ""
