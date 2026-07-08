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

# Shell completions ---------------------------------------------------------
# zsh: drop `_zz` on the $fpath. Default targets oh-my-zsh's custom dir; override
# with ZSH_COMPLETION_DIR. Remember to remove any `compdef ... zz` line that
# would override it, then re-run `compinit`.
ZSH_COMPLETION_DIR="${ZSH_COMPLETION_DIR:-${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}/completions}"
if [ -d "$(dirname "$ZSH_COMPLETION_DIR")" ]; then
  mkdir -p "$ZSH_COMPLETION_DIR"
  cp "$DIR/completions/_zz" "$ZSH_COMPLETION_DIR/_zz"
  echo "Installed zsh completion to $ZSH_COMPLETION_DIR/_zz"
  echo "  (run 'compinit' or restart your shell; drop any 'compdef ... zz' line first)"
fi
echo "bash users: add 'source $DIR/completions/zz.bash' to your ~/.bashrc"

echo "Installation complete."
echo "Make sure $INSTALL_DIR is on your PATH."
