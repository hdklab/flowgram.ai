#!/usr/bin/env bash
# Copyright (c) 2025 Bytedance Ltd. and/or its affiliates
# SPDX-License-Identifier: MIT
# Install a user-level FlowGram launcher in the application menu and on the desktop.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
desktop_app="$root/apps/desktop"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
desktop_dir="$(xdg-user-dir DESKTOP 2>/dev/null || printf '%s/Desktop' "$HOME")"
install_dir="$data_home/flowgram"
backend="${FLOWGRAM_GDK_BACKEND:-wayland}"

case "$backend" in
  ""|x11|wayland) ;;
  *) echo "FLOWGRAM_GDK_BACKEND must be x11 or wayland" >&2; exit 1 ;;
esac

if command -v flowgram-desktop >/dev/null 2>&1; then
  target=$(command -v flowgram-desktop)
else
  source_app="${1:-}"
  if [ -z "$source_app" ]; then
    source_app=$(ls "$root"/target/release/bundle/appimage/*.AppImage 2>/dev/null | head -1 || true)
  fi
  if [ -z "$source_app" ] || [ ! -f "$source_app" ]; then
    echo "Usage: $0 [FlowGram.AppImage]" >&2
    echo "Build one from apps/desktop: node ../../common/scripts/install-run-rushx.js desktop:build:linux" >&2
    exit 1
  fi
  mkdir -p "$install_dir"
  install -m 0755 "$source_app" "$install_dir/FlowGram.AppImage"
  target="$install_dir/FlowGram.AppImage"
fi

# GDK_SCALE/GDK_DPI_SCALE are X11-only HiDPI workarounds; with the compositor
# scaling the window they shift pointer hit-testing, so drop them for the app.
prefix="env -u GDK_SCALE -u GDK_DPI_SCALE FLOWGRAM_GDK_BACKEND=$backend "

entry() {
  cat <<EOF
[Desktop Entry]
Type=Application
Name=FlowGram
Comment=Workflow desktop workbench
Exec=${prefix}"${target}" %U
Icon=flowgram-desktop
Terminal=false
Categories=Development;
Keywords=workflow;agent;AI;
StartupWMClass=flowgram-desktop
EOF
}

for size in 32x32 128x128 256x256; do
  install -Dm644 "$desktop_app/src-tauri/icons/$size.png" \
    "$data_home/icons/hicolor/$size/apps/flowgram-desktop.png"
done
install -Dm644 "$desktop_app/src-tauri/icons/icon.png" \
  "$data_home/icons/hicolor/1024x1024/apps/flowgram-desktop.png"

entry > "$data_home/applications/flowgram-desktop.desktop"
chmod 0644 "$data_home/applications/flowgram-desktop.desktop"
entry > "$desktop_dir/FlowGram.desktop"
chmod 0755 "$desktop_dir/FlowGram.desktop"
# GNOME only launches desktop files marked trusted; other desktops ignore this.
gio set "$desktop_dir/FlowGram.desktop" metadata::trusted true 2>/dev/null || true
update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true

echo "Launcher: $data_home/applications/flowgram-desktop.desktop"
echo "Desktop:  $desktop_dir/FlowGram.desktop"
echo "Exec:     $target"
