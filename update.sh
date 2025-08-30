#!/bin/bash
set -e

echo "--- TimeTurner Update Script ---"

# 1. Fetch the latest changes from the git repository
echo "🔄 Pulling latest changes from GitHub..."
git pull origin main

# 2. Rebuild the release binary
echo "📦 Building release binary with Cargo..."
cargo build --release

# 3. Stop the currently running service to release the file lock
echo "🛑 Stopping TimeTurner service..."
sudo systemctl stop timeturner.service

# 4. Copy the new binary to the installation directory
echo "🚀 Deploying new binary..."
sudo cp target/release/timeturner /opt/timeturner/timeturner

# 5. Restart the service with the new binary
echo "✅ Restarting TimeTurner service..."
sudo systemctl restart timeturner.service

echo ""
echo "Update complete. To check the status of the service, run:"
echo "  systemctl status timeturner.service"