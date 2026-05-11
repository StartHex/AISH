#!/bin/bash
# Bundle aish-gui as a macOS .app so it launches without Terminal.
# Usage: ./bundle.sh [debug|release]

set -euo pipefail
MODE="${1:-debug}"
BIN_DIR="../../target/$MODE"
BIN="$BIN_DIR/aish-gui"
APP_DIR="$BIN_DIR/AISH GUI.app"

if [ ! -f "$BIN" ]; then
    echo "Binary not found: $BIN"
    echo "Build first: CMAKE_PREFIX_PATH=/opt/homebrew/opt/qt@6 cargo build -p aish-gui --features gui-native"
    [ "$MODE" = "release" ] && echo "  (add --release for release mode)"
    exit 1
fi

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp "$BIN" "$APP_DIR/Contents/MacOS/aish-gui"

cat > "$APP_DIR/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>aish-gui</string>
    <key>CFBundleIdentifier</key>
    <string>com.aish.gui</string>
    <key>CFBundleName</key>
    <string>AISH GUI</string>
    <key>CFBundleDisplayName</key>
    <string>AISH GUI</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "Created: $APP_DIR"
echo "Run: open '$APP_DIR'"
