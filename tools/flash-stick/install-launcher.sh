#!/usr/bin/env bash
# Puts "Flash a Faraday stick" in the desktop's app menu, so the window
# (tools/flash-stick/flash-stick.py) opens with a click where the file
# manager does not run flash-stick.desktop on a double-click. The entry
# runs the script in this checkout, so it always writes this checkout's
# out/stick/ images.
#
#   tools/flash-stick/install-launcher.sh             the app menu
#   tools/flash-stick/install-launcher.sh --desktop   and an icon on the desktop
#
# The entry is ~/.local/share/applications/faraday-flash-stick.desktop;
# deleting that file (and the desktop's copy) takes it away again.
set -euo pipefail

die() { printf 'install-launcher: %s\n' "$1" >&2; exit 1; }

desktop=0
case "${1:-}" in
    "") ;;
    --desktop) desktop=1 ;;
    *) die "usage: install-launcher.sh [--desktop]" ;;
esac

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/flash-stick.py"
case "$script" in
    *[\"\`\$\\]*) die "the checkout's path has a character a launcher cannot quote: $script" ;;
esac

apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$apps"
entry="$apps/faraday-flash-stick.desktop"
cat > "$entry" <<EOF
[Desktop Entry]
Type=Application
Name=Flash a Faraday stick
Comment=Write a Faraday stick image from $(dirname "$(dirname "$here")") to a USB stick
Exec="$script"
Icon=drive-removable-media
Terminal=false
Categories=Development;
StartupNotify=true
EOF
chmod 0755 "$entry"
echo "App menu: $entry"
command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true

if [ "$desktop" = 1 ]; then
    dir="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"
    [ -d "$dir" ] || die "no desktop folder at $dir"
    cp "$entry" "$dir/"
    chmod 0755 "$dir/faraday-flash-stick.desktop"
    # GNOME's desktop icons run a launcher only once it is trusted.
    gio set "$dir/faraday-flash-stick.desktop" metadata::trusted true 2>/dev/null || true
    echo "Desktop : $dir/faraday-flash-stick.desktop"
fi
