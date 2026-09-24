#!/usr/bin/env bash
# macOS 패키징: .app 번들 + tar.gz + .dmg (dmg는 macOS에서만)
# 사용법: ./scripts/package-macos.sh [target-triple]
#   예) ./scripts/package-macos.sh aarch64-apple-darwin
set -euo pipefail

TARGET="${1:-aarch64-apple-darwin}"
VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
NAME="nebulya-launcher"
APP_NAME="Nebulya Launcher"
DIST="dist"
BIN="target/${TARGET}/release/${NAME}"

echo "==> binary: $BIN (v$VERSION)"
test -x "$BIN" || { echo "릴리즈 바이너리가 없습니다. 먼저 cargo build --release --target $TARGET"; exit 1; }

rm -rf "$DIST/macos" && mkdir -p "$DIST/macos"

# 1) .app 번들 조립
APP="$DIST/macos/${APP_NAME}.app/Contents/MacOS"
mkdir -p "$APP" "$DIST/macos/${APP_NAME}.app/Contents/Resources"
cp "$BIN" "$APP/${NAME}"
cat > "$DIST/macos/${APP_NAME}.app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key><string>gg.nebulya.launcher</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>${NAME}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# 2) tar.gz (어느 OS에서든 생성 가능)
tar -czf "$DIST/macos/${NAME}-${VERSION}-${TARGET}.tar.gz" -C "$DIST/macos" "${APP_NAME}.app"
echo "==> tarball OK"

# 3) .dmg (macOS에서만)
if [[ "$(uname)" == "Darwin" ]]; then
  DMG_STAGING="$DIST/macos/dmg-staging"
  rm -rf "$DMG_STAGING" && mkdir -p "$DMG_STAGING"
  cp -R "$DIST/macos/${APP_NAME}.app" "$DMG_STAGING/"
  hdiutil create -volname "${APP_NAME}" -srcfolder "$DMG_STAGING" -ov -format UDZO \
    "$DIST/macos/${NAME}-${VERSION}-${TARGET}.dmg"
  echo "==> dmg OK"
else
  echo "==> macOS가 아니라 dmg는 건너뜀 (CI macos 러너에서 생성됨)"
fi

ls -lh "$DIST/macos"
