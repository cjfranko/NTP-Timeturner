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
    sudo apt install -y build-essential libudev-dev pkg-config
elif [ "$PKG_MANAGER" == "dnf" ]; then
    sudo dnf install -y gcc make perl-devel libudev-devel pkg-config
elif [ "$PKG_MANAGER" == "pacman" ]; then
    sudo pacman -Sy --noconfirm base-devel libudev pkg-config
fi
echo "Common build dependencies installed."

# --- Remove NTPD and install Chrony, NMTUI, Adjtimex ---
echo "Removing NTPD (if installed) and installing Chrony, NMTUI, Adjtimex..."

if [ "$PKG_MANAGER" == "apt" ]; then
    sudo apt update
    sudo apt remove -y ntp || true # Remove ntp if it exists, ignore if not
    sudo apt install -y chrony nmtui adjtimex
    sudo systemctl enable chrony --now
elif [ "$PKG_MANAGER" == "dnf" ]; then
    sudo dnf remove -y ntp || true
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
echo "A default 'config.yml' will be created there on first run."
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
