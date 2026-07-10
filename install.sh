#!/usr/bin/env bash
# zz installer — downloads a prebuilt binary from the latest GitHub release.
# No Rust toolchain required.
#
#   curl -fsSL https://raw.githubusercontent.com/enekos/zz/main/install.sh | bash
#
# Overrides via env:
#   INSTALL_DIR          where to put the binary    (default: ~/.local/bin)
#   ZZ_VERSION           tag to install             (default: latest, e.g. v0.3.0)
#   ZSH_COMPLETION_DIR   where to drop _zz           (default: oh-my-zsh custom)
set -euo pipefail

REPO="enekos/zz"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
SHARE_DIR="${SHARE_DIR:-$HOME/.local/share/zz}"

err() { echo "error: $*" >&2; exit 1; }

# Detect platform ----------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Darwin-arm64)               target="aarch64-apple-darwin" ;;
  Darwin-x86_64)              target="x86_64-apple-darwin" ;;
  Linux-x86_64|Linux-amd64)   target="x86_64-unknown-linux-musl" ;;
  Linux-aarch64|Linux-arm64)  target="aarch64-unknown-linux-musl" ;;
  *) err "unsupported platform: $os-$arch — build from source: https://github.com/$REPO#from-source" ;;
esac

# Resolve release tag ------------------------------------------------------
tag="${ZZ_VERSION:-latest}"
if [ "$tag" = "latest" ]; then
  tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)"
fi
[ -n "${tag:-}" ] || err "could not determine release tag (no releases yet?)"

url="https://github.com/$REPO/releases/download/$tag/zz-$target.tar.gz"
echo "Installing zz $tag ($target)..."

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$url" -o "$tmp/zz.tar.gz" || err "download failed: $url"
tar xzf "$tmp/zz.tar.gz" -C "$tmp"
src="$tmp/zz-$target"

# Install binary -----------------------------------------------------------
mkdir -p "$INSTALL_DIR"
install -m 0755 "$src/zz" "$INSTALL_DIR/zz"
echo "  binary    -> $INSTALL_DIR/zz"

# Install completions ------------------------------------------------------
mkdir -p "$SHARE_DIR"
cp -r "$src/completions" "$SHARE_DIR/completions"

ZSH_COMPLETION_DIR="${ZSH_COMPLETION_DIR:-${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}/completions}"
if [ -d "$(dirname "$ZSH_COMPLETION_DIR")" ]; then
  mkdir -p "$ZSH_COMPLETION_DIR"
  cp "$src/completions/_zz" "$ZSH_COMPLETION_DIR/_zz"
  echo "  zsh comp  -> $ZSH_COMPLETION_DIR/_zz (run 'compinit'; drop any 'compdef ... zz' line)"
fi
echo "  bash comp -> source $SHARE_DIR/completions/zz.bash from your ~/.bashrc"

# Post-install checks ------------------------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo; echo "NOTE: $INSTALL_DIR is not on your PATH. Add:"; \
     echo "      export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

command -v zoxide >/dev/null 2>&1 || {
  echo; echo "NOTE: zz needs zoxide for directory resolution:"
  echo "      https://github.com/ajeetdsouza/zoxide"
}

echo
echo "Done. Verify with: zz --version"
