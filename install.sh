#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

mkdir -p "$INSTALL_DIR"

echo "Building zz..."
cd "$DIR"
cargo build --release

echo "Installing zz to $INSTALL_DIR..."
cp target/release/zz "$INSTALL_DIR/zz"

echo "Installation complete."
echo "Make sure $INSTALL_DIR is on your PATH."
