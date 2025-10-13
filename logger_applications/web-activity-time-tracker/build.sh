#!/bin/bash

# Build script for web-activity-time-tracker Chrome extension

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building web-activity-time-tracker Chrome extension..."
echo ""

# Check if node_modules exists, if not install dependencies
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    if command -v pnpm &> /dev/null; then
        pnpm install
    elif command -v npm &> /dev/null; then
        npm install
    else
        echo "Error: Neither pnpm nor npm found. Please install Node.js and pnpm/npm."
        exit 1
    fi
    echo ""
fi

# Build the extension
echo "Building extension..."
if command -v pnpm &> /dev/null; then
    pnpm exec vite build --mode production
else
    npx vite build --mode production
fi

echo ""
echo "✓ Build complete!"
echo ""
echo "To load the extension in Chrome:"
echo "1. Open Chrome and go to chrome://extensions/"
echo "2. Enable 'Developer mode' (toggle in top-right corner)"
echo "3. Click 'Load unpacked'"
echo "4. Select the 'dist' directory: $SCRIPT_DIR/dist"
echo ""
