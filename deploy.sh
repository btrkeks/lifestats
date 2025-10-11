#!/bin/bash

# LifeStats Deploy Script
# Builds release binaries and copies them to ~/.local/bin

set -e # Exit on error

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST_DIR="$HOME/.local/bin"

echo "Building LifeStats in release mode..."
cd "$PROJECT_ROOT"

# Build all binaries in release mode
cargo build --release

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
