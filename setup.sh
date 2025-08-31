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
            # Stop the service to allow overwriting the binary, ignore errors if not running
            echo "Stopping existing TimeTurner service..."
            sudo systemctl stop timeturner.service || true
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
    # We no longer need hotspot dependencies
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


# --- The entire WiFi hotspot and captive portal section has been removed ---


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
    sudo systemctl enable tim_turner.service
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
    echo "  journalctl -u timeturner.service -f"
    echo ""
fi
echo "To run the interactive TUI instead, simply run from the project directory:"
echo "  cargo run"
echo "Or from anywhere after installation:"
echo "  timeturner"
echo ""
