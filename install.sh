#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Checking build dependencies ==="
BUILD_DEPS="libgtk-3-dev libglib2.0-dev"
MISSING=""
for pkg in $BUILD_DEPS; do
    if ! dpkg -s "$pkg" &>/dev/null; then
        MISSING="$MISSING $pkg"
    fi
done
if [ -n "$MISSING" ]; then
    echo "Installing missing build deps:$MISSING"
    sudo apt-get install -y $MISSING
fi

echo "=== Installing cargo-deb ==="
if ! command -v cargo-deb &>/dev/null; then
    cargo install cargo-deb
fi

echo "=== Building .deb package ==="
cargo deb

DEB=$(ls -t target/debian/cpu-tweaks_*.deb 2>/dev/null | head -1)
if [ -z "$DEB" ]; then
    echo "ERROR: .deb not found"
    exit 1
fi

echo ""
echo "=== Package built: $DEB ==="
echo ""
echo "Install with:"
echo "  sudo dpkg -i $DEB"
echo ""
echo "Or install now? [y/N]"
read -r answer
if [[ "$answer" =~ ^[Yy]$ ]]; then
    sudo dpkg -i "$DEB"
    echo "Installed! Launch from Applications > System > CPU Tweaks"
fi
