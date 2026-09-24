#!/usr/bin/env bash
# Linux 패키징: portable tar.gz + (.deb, cargo-deb 설치 시)
# 사용법: ./scripts/package-linux.sh [target-triple]
#   예) ./scripts/package-linux.sh x86_64-unknown-linux-gnu
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
NAME="nebulya-launcher"
DIST="dist"
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
echo "==> tarball OK"

# 2) .deb (cargo-deb이 있을 때만)
if command -v cargo-deb >/dev/null 2>&1; then
  cargo deb --target "$TARGET" --output "$DIST/linux/${NAME}_${VERSION}_${TARGET}.deb"
  echo "==> deb OK"
else
  echo "==> cargo-deb 없음 — tar.gz만 생성 (설치: cargo install cargo-deb)"
fi

ls -lh "$DIST/linux"
