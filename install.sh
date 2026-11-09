#!/bin/sh
set -e

# StyleFreeze Unix installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Mresyzz/stylefreeze/main/install.sh | sh

REPO="Mresyzz/stylefreeze"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux)
    ASSET="stylefreeze-x86_64-unknown-linux-gnu.tar.gz"
    ;;
  darwin)
    if [ "$ARCH" = "arm64" ]; then
      ASSET="stylefreeze-aarch64-apple-darwin.tar.gz"
    else
      ASSET="stylefreeze-x86_64-apple-darwin.tar.gz"
    fi
    ;;
  *)
    echo "[ERROR] Unsupported operating system: $OS"
    exit 1
    ;;
esac

TARGET_DIR="${HOME}/.local/bin"
mkdir -p "$TARGET_DIR"

URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
echo "[INFO] Downloading StyleFreeze from $URL..."

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

curl -sSL "$URL" -o "$TMP_DIR/stylefreeze.tar.gz"
tar -xzf "$TMP_DIR/stylefreeze.tar.gz" -C "$TMP_DIR"

install -m 755 "$TMP_DIR/stylefreeze" "$TARGET_DIR/stylefreeze"

echo "[OK] StyleFreeze installed successfully to $TARGET_DIR/stylefreeze"
if ! echo "$PATH" | grep -q "$TARGET_DIR"; then
  echo "[INFO] Add $TARGET_DIR to your PATH:"
  echo "       export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
