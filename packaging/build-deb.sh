#!/usr/bin/env bash
# 构建 deb 包：deskmason_<version>_<arch>.deb
#
# 用法：packaging/build-deb.sh [输出目录]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT/target/deb}"

VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
ARCH="$(dpkg --print-architecture)"
PKG="deskmason"
STAGE="$OUT_DIR/${PKG}_${VERSION}_${ARCH}"

echo "==> 构建 release 二进制"
cargo build --release -p dm-app --manifest-path "$ROOT/Cargo.toml"

# 兼容受限环境：PATH 里的 rm 可能被替换，优先用系统 rm。
RM="${RM:-/usr/bin/rm}"
"$RM" -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" \
         "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/scalable/apps" \
         "$STAGE/usr/share/doc/$PKG"

install -m 0755 "$ROOT/target/release/deskmason-ui" "$STAGE/usr/bin/deskmason"
install -m 0644 "$ROOT/packaging/deskmason.desktop" "$STAGE/usr/share/applications/deskmason.desktop"
install -m 0644 "$ROOT/packaging/deskmason.svg" "$STAGE/usr/share/icons/hicolor/scalable/apps/deskmason.svg"

cat > "$STAGE/usr/share/doc/$PKG/copyright" <<'COPY'
DeskMason — 桌面瓦匠
Copyright (C) 2026 Simon
Licensed under the GNU General Public License v3.0 only.
See /usr/share/common-licenses/GPL-3 for the full text.
COPY

INSTALLED_SIZE="$(du -sk "$STAGE" | cut -f1)"
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: $PKG
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_SIZE
Maintainer: Simon <1047078635@qq.com>
Depends: libc6, libqt6core6, libqt6gui6, libqt6qml6, libqt6quick6, libqt6network6, libqt6dbus6, libsqlite3-0, libxcb1
Description: Desktop tiles for Deepin (桌面瓦匠)
 DeskMason puts folder-backed tiles on the Deepin desktop: file widgets with
 drag and drop, notes, todo, weather, music, search and glance widgets.
CONTROL

dpkg-deb --build --root-owner-group "$STAGE" "$OUT_DIR/${PKG}_${VERSION}_${ARCH}.deb"
echo "==> 完成：$OUT_DIR/${PKG}_${VERSION}_${ARCH}.deb"
