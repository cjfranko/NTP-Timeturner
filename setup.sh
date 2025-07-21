#!/bin/bash
set -e

echo "--- TimeTurner Setup ---"

# 1. Build the release binary
echo "📦 Building release binary with Cargo..."
if ! command -v cargo &> /dev/null
then
    echo "❌ Cargo is not installed. Please install Rust and Cargo first."
    echo "Visit https://rustup.rs/ for instructions."
    exit 1
fi
cargo build --release
echo "✅ Build complete."

# 2. Create installation directories
INSTALL_DIR="/opt/timeturner"
BIN_DIR="/usr/local/bin"
echo "🔧 Creating directories..."
sudo mkdir -p $INSTALL_DIR
echo "✅ Directory $INSTALL_DIR created."

# 3. Install binary
echo "🚀 Installing timeturner binary..."
sudo cp target/release/ntp_timeturner $INSTALL_DIR/timeturner
sudo ln -sf $INSTALL_DIR/timeturner $BIN_DIR/timeturner
echo "✅ Binary installed to $INSTALL_DIR and linked to $BIN_DIR."

# 4. Install systemd service file
echo "⚙️  Installing systemd service..."
sudo cp timeturner.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable timeturner.service
echo "✅ Systemd service installed and enabled."

echo ""
echo "--- Setup Complete ---"
echo "The TimeTurner daemon is now installed."
echo "The working directory is $INSTALL_DIR."
echo "A default 'config.yml' will be created there on first run."
echo ""
echo "To start the service, run:"
echo "  sudo systemctl start timeturner.service"
echo ""
echo "To view live logs, run:"
echo "  journalctl -u timeturner.service -f"
echo ""
echo "To run the interactive TUI instead, simply run from the project directory:"
echo "  cargo run"
echo "Or from anywhere after installation:"
echo "  timeturner"
echo ""
