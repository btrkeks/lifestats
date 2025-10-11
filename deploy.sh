#!/bin/bash

# LifeStats Deploy Script
# Builds release binaries, builds web UI, and copies binaries to ~/.local/bin
# Also manages systemd services

set -e # Exit on error

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST_DIR="$HOME/.local/bin"

# Stop services before deploying
echo "Stopping systemd services..."
systemctl --user stop lifestats-web-api.service 2>/dev/null || echo "  lifestats-web-api.service not running or not found"
systemctl --user stop lifestats-web-ui.service 2>/dev/null || echo "  lifestats-web-ui.service not running or not found"

echo ""
echo "Building LifeStats in release mode..."
cd "$PROJECT_ROOT"

# Build all binaries in release mode
cargo build --release

# Build web UI
echo ""
echo "Building Web UI..."
cd "$PROJECT_ROOT/ui/web-ui"
npm install
npm run build

echo ""
echo "Web UI build complete"
cd "$PROJECT_ROOT"

# Create destination directory if it doesn't exist
mkdir -p "$DEST_DIR"

# List of binaries to deploy
# Format: "binary_name_in_target:installed_name"
BINARIES=(
  "lifestats-web-api:lifestats-web-api"
  "lifestats-stopwatch-logger:lifestats-stopwatch-logger"
  "midi_logger:lifestats-midi-logger"
)

# Check which binaries were actually built
BUILT_BINARIES=()
for entry in "${BINARIES[@]}"; do
  IFS=':' read -r binary_name installed_name <<<"$entry"
  if [ -f "target/release/$binary_name" ]; then
    BUILT_BINARIES+=("$entry")
  else
    echo "Warning: Binary '$binary_name' not found in target/release/, skipping..."
  fi
done

# Copy binaries
echo ""
echo "Deploying binaries to $DEST_DIR..."
for entry in "${BUILT_BINARIES[@]}"; do
  IFS=':' read -r binary_name installed_name <<<"$entry"
  echo "  - Installing $installed_name"
  cp "target/release/$binary_name" "$DEST_DIR/$installed_name"
  chmod +x "$DEST_DIR/$installed_name"
done

echo ""
echo "Deployment complete! Installed ${#BUILT_BINARIES[@]} binaries:"
for entry in "${BUILT_BINARIES[@]}"; do
  IFS=':' read -r binary_name installed_name <<<"$entry"
  echo "  - $installed_name"
done

# Restart services
echo ""
echo "Restarting systemd services..."
systemctl --user start lifestats-web-api.service && echo "  ✓ lifestats-web-api.service started" || echo "  ✗ Failed to start lifestats-web-api.service"
systemctl --user start lifestats-web-ui.service && echo "  ✓ lifestats-web-ui.service started" || echo "  ✗ Failed to start lifestats-web-ui.service"

echo ""
echo "Checking service status..."
systemctl --user status lifestats-web-api.service --no-pager -l || true
echo ""
systemctl --user status lifestats-web-ui.service --no-pager -l || true
