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
    # Install dependencies for hotspot and for building nodogsplash
    sudo apt install -y hostapd dnsmasq git libmicrohttpd-dev libjson-c-dev
    
    echo "Building and installing nodogsplash from source..."
    # Create a temporary directory for the build
    BUILD_DIR=$(mktemp -d)
    git clone https://github.com/nodogsplash/nodogsplash.git "$BUILD_DIR"
    
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

    # Clean up the build directory
    cd ..
    sudo rm -rf "$BUILD_DIR"
    echo "✅ nodogsplash installed successfully."

    sudo systemctl unmask hostapd
    sudo systemctl enable hostapd
    sudo systemctl enable nodogsplash
else
    echo "This script is designed for Debian-based systems like Raspberry Pi OS."
    echo "Skipping WiFi hotspot setup."
fi

# Stop services to configure
# Ensure services exist before trying to stop them
sudo systemctl stop hostapd || true
sudo systemctl stop dnsmasq || true
if command -v nodogsplash &> /dev/null; then
    sudo systemctl stop nodogsplash || true
fi

# Configure static IP for wlan0 using NetworkManager (nmcli)
echo "Configuring static IP for wlan0 using NetworkManager..."
# Check if a connection for wlan0 already exists and delete it to start fresh
if nmcli -t -f NAME,DEVICE c show --active | grep -q "wlan0"; then
    ACTIVE_CON=$(nmcli -t -f NAME,DEVICE c show --active | grep "wlan0" | cut -d':' -f1)
    echo "Temporarily deactivating existing connection on wlan0: $ACTIVE_CON"
    sudo nmcli c down "$ACTIVE_CON" || true
fi

# Create a new connection profile for the AP
# This sets the static IP and configures it to not manage this interface for other connections.
sudo tee /etc/NetworkManager/conf.d/99-unmanaged-wlan0.conf > /dev/null <<EOF
[keyfile]
unmanaged-devices=interface-name:wlan0
EOF
# Reload NetworkManager to apply the unmanaged device setting
sudo systemctl reload NetworkManager

# Now, configure the static IP using a method that doesn't rely on NetworkManager's main loop
# We will use dhcpcd for this specific interface, as it's simpler for a static AP setup.
# First, ensure dhcpcd is installed.
if [ "$PKG_MANAGER" == "apt" ]; then
    sudo apt install -y dhcpcd5
fi

# Configure static IP for wlan0 in dhcpcd.conf
sudo tee /etc/dhcpcd.conf > /dev/null <<EOF
# A sample configuration for dhcpcd.
# See dhcpcd.conf(5) for details.

# Allow users of this group to interact with dhcpcd via the control socket.
#controlgroup wheel

# Inform the DHCP server of our hostname for DDNS.
hostname

# Use the hardware address of the interface for the Client ID.
clientid

# Persist interface configuration when dhcpcd exits.
persistent

# Rapid commit support.
# Safe to enable by default because it requires the equivalent option set
# on the server to actually work.
option rapid_commit

# A list of options to request from the DHCP server.
option domain_name_servers, domain_name, domain_search, host_name
option classless_static_routes
# Respect the network MTU. This is applied to DHCP routes.
option interface_mtu

# A hook script is provided to lookup the hostname if not set by the DHCP
# server, but it should not be run by default.
nohook lookup-hostname

# Static IP configuration for TimeTurner AP
interface wlan0
    static ip_address=10.0.252.1/24
    nohook wpa_supplicant
EOF

# Configure dnsmasq for DHCP
echo "Configuring dnsmasq..."
sudo tee /etc/dnsmasq.conf > /dev/null <<EOF
interface=wlan0
dhcp-range=10.0.252.10,10.0.252.50,255.255.255.0,24h
address=/#/10.0.252.1
EOF

# Configure hostapd
echo "Configuring hostapd..."
sudo tee /etc/hostapd/hostapd.conf > /dev/null <<EOF
interface=wlan0
driver=nl80211
ssid=TimeTurner
hw_mode=g
channel=7
wmm_enabled=0
macaddr_acl=0
auth_algs=1
ignore_broadcast_ssid=0
wpa=2
wpa_passphrase=harry-ron-hermione
wpa_key_mgmt=WPA-PSK
rsn_pairwise=CCMP
EOF

sudo sed -i 's|#DAEMON_CONF=""|DAEMON_CONF="/etc/hostapd/hostapd.conf"|' /etc/default/hostapd

# Configure nodogsplash for captive portal
echo "Configuring nodogsplash..."
sudo tee /etc/nodogsplash/nodogsplash.conf > /dev/null <<EOF
GatewayInterface wlan0
GatewayAddress 10.0.252.1
MaxClients 250
ClientIdleTimeout 480
FirewallRuleSet preauthenticated-users {
    FirewallRule allow tcp port 80
    FirewallRule allow tcp port 53
    FirewallRule allow udp port 53
}
RedirectURL http://10.0.252.1/static/index.html
EOF

# Restart services in the correct order and add delays to prevent race conditions
echo "Restarting services..."
sudo systemctl restart dhcpcd
sudo systemctl restart hostapd

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
        sudo systemctl restart nodogsplash
    fi
else
    echo "❌ Error: wlan0 failed to get the static IP 10.0.252.1. Found: '$IP_CHECK'."
    echo "Please check 'sudo systemctl status hostapd' and 'sudo systemctl status dhcpcd'."
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
sudo cp target/release/ntp_tim_turner $INSTALL_DIR/timeturner
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
