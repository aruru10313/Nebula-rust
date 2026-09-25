#!/usr/bin/env bash
# Linux 패키징: portable tar.gz + .deb
# 사용법: ./scripts/package-linux.sh [target-triple] [version]
#   예) ./scripts/package-linux.sh x86_64-unknown-linux-gnu 0.3.0
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
REQUESTED_VERSION="${2:-}"
NAME="nebulya-launcher"
DIST="dist"
CARGO_VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
if [ -n "$REQUESTED_VERSION" ] && [ "$REQUESTED_VERSION" != "$CARGO_VERSION" ]; then
  echo "버전 불일치: requested=$REQUESTED_VERSION cargo=$CARGO_VERSION" >&2
  exit 1
fi
VERSION="$CARGO_VERSION"
BIN="target/${TARGET}/release/${NAME}"

echo "==> binary: $BIN (v$VERSION)"
test -x "$BIN" || { echo "릴리즈 바이너리가 없습니다. 먼저 cargo build --release --target $TARGET"; exit 1; }

rm -rf "$DIST/linux" && mkdir -p "$DIST/linux"

# 1) portable tar.gz
STAGE="$DIST/linux/${NAME}-${VERSION}-${TARGET}"
mkdir -p "$STAGE"
cp "$BIN" "$STAGE/"
cp README.md "$STAGE/" 2>/dev/null || true
tar -czf "$DIST/linux/${NAME}-${VERSION}-${TARGET}.tar.gz" -C "$DIST/linux" "$(basename "$STAGE")"
rm -rf "$STAGE"
echo "==> tarball OK"

# 2) .deb (이미 빌드된 바이너리를 패키징; cargo-deb 필수)
if ! command -v cargo-deb >/dev/null 2>&1; then
  echo "cargo-deb이 없습니다. CI에서 고정 버전으로 설치하세요." >&2
  exit 1
fi
cargo deb --no-build --target "$TARGET" --output "$DIST/linux/${NAME}_${VERSION}_${TARGET}.deb"
echo "==> deb OK"
dpkg-deb --info "$DIST/linux/${NAME}_${VERSION}_${TARGET}.deb"
dpkg-deb --contents "$DIST/linux/${NAME}_${VERSION}_${TARGET}.deb"

ls -lh "$DIST/linux"
