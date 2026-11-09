#!/bin/sh
set -e

# StyleFreeze Unix installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Mresyzz/stylefreeze/main/install.sh | sh

REPO="Mresyzz/stylefreeze"
DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=1
      ;;
  esac
done

if [ -n "$STYLEFREEZE_DRY_RUN" ] && [ "$STYLEFREEZE_DRY_RUN" != "0" ]; then
  DRY_RUN=1
fi

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

TARGET_DIR="${HOME:-/root}/.local/bin"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "[INFO] Running in dry-run mode."
  echo "[INFO] Detected OS: $OS"
  echo "[INFO] Detected Architecture: $ARCH"
  echo "[INFO] Target Asset: $ASSET"
  echo "[INFO] Download URL: $URL"
  echo "[INFO] Target Directory: $TARGET_DIR"
  echo "[OK] Dry-run checks completed successfully."
  exit 0
fi

if ! command -v tar >/dev/null 2>&1; then
  echo "[ERROR] 'tar' is required to unpack the release archive. Please install tar to continue."
  exit 1
fi

DOWNLOADER=""
if command -v curl >/dev/null 2>&1; then
  DOWNLOADER="curl"
elif command -v wget >/dev/null 2>&1; then
  DOWNLOADER="wget"
else
  echo "[ERROR] Neither curl nor wget found. Please install one to continue."
  exit 1
fi

mkdir -p "$TARGET_DIR"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "[INFO] Downloading StyleFreeze from $URL..."

if [ "$DOWNLOADER" = "curl" ]; then
  if ! curl -fsSL "$URL" -o "$TMP_DIR/stylefreeze.tar.gz"; then
    code=$?
    if [ "$code" -eq 77 ] || [ "$code" -eq 60 ]; then
      echo "[ERROR] SSL certificate error (curl exit code $code). Please ensure 'ca-certificates' is installed."
    else
      echo "[ERROR] Failed to download StyleFreeze archive (curl exit code $code)."
    fi
    exit "$code"
  fi
elif [ "$DOWNLOADER" = "wget" ]; then
  if ! wget -qO "$TMP_DIR/stylefreeze.tar.gz" "$URL"; then
    code=$?
    echo "[ERROR] Failed to download StyleFreeze archive (wget exit code $code)."
    exit "$code"
  fi
fi

if [ ! -s "$TMP_DIR/stylefreeze.tar.gz" ]; then
  echo "[ERROR] Downloaded archive is empty or invalid."
  exit 1
fi

tar --no-same-owner -xzf "$TMP_DIR/stylefreeze.tar.gz" -C "$TMP_DIR" 2>/dev/null || tar -xzf "$TMP_DIR/stylefreeze.tar.gz" -C "$TMP_DIR"

if [ ! -f "$TMP_DIR/stylefreeze" ]; then
  echo "[ERROR] Extracted archive did not contain the stylefreeze binary."
  exit 1
fi

cp -f "$TMP_DIR/stylefreeze" "$TARGET_DIR/stylefreeze"
chmod 755 "$TARGET_DIR/stylefreeze"

echo "[OK] StyleFreeze installed successfully to $TARGET_DIR/stylefreeze"
if ! echo "$PATH" | grep -q "$TARGET_DIR"; then
  echo "[INFO] Add $TARGET_DIR to your PATH:"
  echo "       export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
