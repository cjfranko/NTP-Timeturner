#!/bin/bash
set -e

echo "--- TimeTurner Setup ---"

# Check if TimeTurner is already installed.
INSTALL_DIR="/opt/timeturner"
if [ -f "${INSTALL_DIR}/timeturner" ]; then
    echo "✅ TimeTurner is already installed."
    # Ask the user what to do
    read -p "Do you want to (U)pdate, (R)einstall, or (A)bort? [U/r/a] " choice
    case "$choice" in
        r|R )
            echo "Proceeding with full re-installation..."
            # The script will continue to the installation steps below.
            ;;
        a|A )
            echo "Aborting setup."
            exit 0
            ;;
        * ) # Default to Update
            echo "Attempting to run the update script..."
            # Ensure we are in a git repository and the update script exists
            if [ -d ".git" ] && [ -f "update.sh" ]; then
                chmod +x update.sh
                ./update.sh
                # Exit cleanly after the update
                exit 0
            else
                echo "⚠️  Could not find 'update.sh' or not in a git repository."
                echo "Please re-clone the repository to get the update script, or remove the existing installation to run setup again:"
                echo "  sudo rm -rf ${INSTALL_DIR}"
                exit 1
            fi
            ;;
    esac
fi


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

# --- Configure Chrony to act as a local NTP server ---
echo "⚙️  Configuring Chrony to serve local time..."
# The path to chrony.conf can vary
if [ -f /etc/chrony/chrony.conf ]; then
    CHRONY_CONF="/etc/chrony/chrony.conf"
elif [ -f /etc/chrony.conf ]; then
    CHRONY_CONF="/etc/chrony.conf"
else
    CHRONY_CONF=""
fi

if [ -n "$CHRONY_CONF" ]; then
    # Comment out any existing pool, server, or sourcedir lines to prevent syncing with external sources
    echo "Disabling external NTP sources..."
    sudo sed -i -E 's/^(pool|server|sourcedir)/#&/' "$CHRONY_CONF"

    # Add settings to the top of the file to serve local clock
    # Using a temp file to prepend is safer than multiple sed calls
    TEMP_CONF=$(mktemp)
    cat <<EOF > "$TEMP_CONF"
# Serve the system clock as a reference at stratum 1
server 127.127.1.0
allow 127.0.0.0/8
local stratum 1

EOF
    # Append the rest of the original config file after our new lines
    cat "$CHRONY_CONF" >> "$TEMP_CONF"
    sudo mv "$TEMP_CONF" "$CHRONY_CONF"


    # Add settings to the bottom of the file to allow LAN clients
    echo "Allowing LAN clients..."
    sudo tee -a "$CHRONY_CONF" > /dev/null <<EOF

# Allow LAN clients to connect
allow 0.0.0.0/0
EOF

    # Restart chrony to apply changes (service name can be chrony or chronyd)
    echo "Restarting Chrony service..."
    sudo systemctl restart chrony || sudo systemctl restart chronyd
    echo "✅ Chrony configured."
else
    echo "⚠️  Warning: chrony.conf not found. Skipping Chrony configuration."
fi


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
    # We are now using the classic hostapd service, so unmask it.
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

# --- Configure networking for AP mode ---

# Tell NetworkManager to ignore wlan0 completely to prevent conflicts.
echo "Configuring NetworkManager to ignore wlan0..."
sudo tee /etc/NetworkManager/conf.d/99-unmanaged-wlan0.conf > /dev/null <<EOF
[keyfile]
unmanaged-devices=interface-name:wlan0
EOF
# Also remove the DNS disabling config as it's no longer relevant for this method
sudo rm -f /etc/NetworkManager/conf.d/99-disable-dns.conf
sudo systemctl reload NetworkManager

# Configure a static IP for wlan0 using dhcpcd.
echo "Configuring static IP for wlan0 via dhcpcd..."
# Ensure dhcpcd is installed
sudo apt install -y dhcpcd5

# First, remove any existing configurations for wlan0 to prevent conflicts.
# This is a more robust way to ensure our settings are applied.
sudo sed -i '/^interface wlan0/,/^\s*$/d' /etc/dhcpcd.conf

# Now, add our static IP config to the end of the file.
sudo tee -a /etc/dhcpcd.conf > /dev/null <<EOF

# Static IP configuration for Hachi Time AP
interface wlan0
    static ip_address=10.0.252.1/24
    nohook wpa_supplicant
EOF

# Configure hostapd for the Access Point
echo "Configuring hostapd..."
sudo tee /etc/hostapd/hostapd.conf > /dev/null <<EOF
interface=wlan0
driver=nl80211
ssid=Fetch-Hachi
hw_mode=g
channel=7
wmm_enabled=0
macaddr_acl=0
auth_algs=1
ignore_broadcast_ssid=0
EOF
# Point the hostapd service to the new config file.
sudo sed -i 's|#DAEMON_CONF=""|DAEMON_CONF="/etc/hostapd/hostapd.conf"|' /etc/default/hostapd

# Configure dnsmasq for DHCP
echo "Configuring dnsmasq..."
sudo tee /etc/dnsmasq.conf > /dev/null <<EOF
# Listen only on this interface
interface=wlan0
# Don't bind to all interfaces, only the one specified above
bind-interfaces
# Set the IP range for DHCP clients
dhcp-range=10.0.252.10,10.0.252.50,255.255.255.0,24h
# Provide a gateway address
dhcp-option=option:router,10.0.252.1
# Provide a DNS server address
dhcp-option=option:dns-server,10.0.252.1
# For captive portal, resolve all DNS queries to the AP itself
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
    # Allow DHCP for clients to get an IP address
    FirewallRule allow udp port 67
    FirewallRule allow udp port 68
    # Allow DNS for captive portal detection
    FirewallRule allow tcp port 53
    FirewallRule allow udp port 53
    # Allow HTTP for the captive portal redirect
    FirewallRule allow tcp port 80
}
RedirectURL http://10.0.252.1/static/index.html
EOF

# Restart services in the correct order and add delays to prevent race conditions
echo "Restarting services..."

# Stop and disable systemd-resolved to prevent any DNS/DHCP conflicts
echo "Disabling systemd-resolved to ensure dnsmasq has full control..."
sudo systemctl stop systemd-resolved || true
sudo systemctl disable systemd-resolved || true

# Restart dhcpcd to apply the static IP
sudo systemctl restart dhcpcd
# Restart hostapd to create the access point
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
    
    # Add a small delay to ensure the interface is fully ready for dnsmasq
    sleep 2 

    echo "Attempting to start dnsmasq service..."
    if ! sudo systemctl restart dnsmasq; then
        echo "❌ dnsmasq service failed to start. Displaying logs..."
        sleep 2
        sudo journalctl -u dnsmasq.service --no-pager -n 50
        exit 1
    fi
    echo "✅ dnsmasq service started successfully."

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
    echo "Please check 'sudo systemctl status hostapd' and 'sudo systemctl status dhcpcd'."
    exit 1
fi

echo "✅ WiFi hotspot and captive portal configured. SSID: Fetch-Hachi, IP: 10.0.252.1"
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
