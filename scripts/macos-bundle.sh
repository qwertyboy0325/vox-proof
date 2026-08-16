#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo build --locked --release -p voxproof-desktop

VERSION="$(awk -F'"' '/^version = / { print $2; exit }' apps/desktop/Cargo.toml)"
DIST="$ROOT/dist"
APP="$DIST/VoxProof.app"
BINARY="$ROOT/target/release/voxproof-desktop"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"

cat >"$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>voxproof-desktop</string>
  <key>CFBundleIdentifier</key>
  <string>dev.voxproof.desktop</string>
  <key>CFBundleName</key>
  <string>VoxProof</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

cp "$BINARY" "$APP/Contents/MacOS/voxproof-desktop"
chmod +x "$APP/Contents/MacOS/voxproof-desktop"

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
fi

echo "Built $APP"
