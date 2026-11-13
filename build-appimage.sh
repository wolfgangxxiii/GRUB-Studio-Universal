#!/usr/bin/env bash
set -euo pipefail

project_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
cd "$project_dir"

command -v cargo >/dev/null || { echo "Rust/Cargo toolchain not installed; install rustup in a Toolbox/Distrobox." >&2; exit 1; }
pkg-config --exists gtk4 || { echo "GTK4 development files not found (gtk4-devel / libgtk-4-dev)." >&2; exit 1; }
command -v linuxdeploy >/dev/null || { echo "linuxdeploy not found. AppImage generation requires linuxdeploy." >&2; exit 1; }
command -v rsvg-convert >/dev/null || { echo "librsvg (rsvg-convert) is required to generate the AppImage icon." >&2; exit 1; }

cargo test --all-targets
cargo build --release --bins
appdir="$project_dir/build/GRUB-Studio-Universal.AppDir"
mkdir -p "$appdir/usr/bin"
install -m 755 target/release/grub-studio-universal "$appdir/usr/bin/grub-studio-universal"
install -m 755 target/release/grub-studio-helper "$appdir/usr/bin/grub-studio-helper"
install -Dm644 packaging/grub-studio-universal.desktop "$appdir/usr/share/applications/grub-studio-universal.desktop"
install -Dm644 assets/grub-studio.svg "$appdir/usr/share/icons/hicolor/scalable/apps/grub-studio-universal.svg"
# AppImage's .DirIcon must be a bitmap; shipping only the scalable SVG caused
# appimagetool to warn about an invalid directory icon in earlier releases.
icon_png="$project_dir/build/grub-studio-universal.png"
rsvg-convert -w 256 -h 256 -o "$icon_png" assets/grub-studio.svg
install -Dm644 "$icon_png" "$appdir/usr/share/icons/hicolor/256x256/apps/grub-studio-universal.png"
install -m644 "$icon_png" "$appdir/grub-studio-universal.png"
ln -sfn grub-studio-universal.png "$appdir/.DirIcon"

# linuxdeploy copies linked libraries and assembles the AppImage runtime.
# GTK4 + GLib runtime resources must be checked on a second distribution.
export ARCH=x86_64
export OUTPUT="GRUB-Studio-Universal-v2.2.0-x86_64.AppImage"
linuxdeploy \
  --appdir "$appdir" \
  --executable "$appdir/usr/bin/grub-studio-universal" \
  --executable "$appdir/usr/bin/grub-studio-helper" \
  --desktop-file packaging/grub-studio-universal.desktop \
  --icon-file "$icon_png" \
  --output appimage

echo "Built: $project_dir/$OUTPUT"
echo "Verify on Bazzite KDE, Fedora GNOME and at least one Debian/Ubuntu distro before distributing."
