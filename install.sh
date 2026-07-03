#!/usr/bin/env sh
set -e

REPO="ahyammona/mimona"
INSTALL_DIR="/usr/local/bin"
BINARY="mimona"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
RESET='\033[0m'

echo ""
echo "${BOLD}  Installing Mimona...${RESET}"
echo ""

# ── Detect OS and arch ────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  ARTIFACT="mimona-linux-x86_64" ;;
      aarch64) ARTIFACT="mimona-linux-arm64" ;;
      arm64)   ARTIFACT="mimona-linux-arm64" ;;
      *)
        echo "${RED}  Unsupported architecture: $ARCH${RESET}"
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64) ARTIFACT="mimona-macos-x86_64" ;;
      arm64)  ARTIFACT="mimona-macos-arm64" ;;
      *)
        echo "${RED}  Unsupported architecture: $ARCH${RESET}"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "${RED}  Unsupported OS: $OS${RESET}"
    echo "  For Windows, download from: https://github.com/$REPO/releases/latest"
    exit 1
    ;;
esac

# ── Check dependencies ────────────────────────────────────────────────────────
check_dep() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "${YELLOW}  ! $1 is not installed.${RESET}"
    echo "    $2"
    echo ""
    MISSING_DEPS=1
  fi
}

MISSING_DEPS=0
check_dep curl   "Install curl: https://curl.se"
check_dep unzip  "Install unzip via your package manager (e.g. apt install unzip)"
check_dep node   "Install Node.js 18+: https://nodejs.org  (needed for WhatsApp bridge)"
check_dep ollama "Install Ollama: https://ollama.com  (needed for AI inference)"

if [ "$MISSING_DEPS" = "1" ]; then
  echo "${YELLOW}  Install the above dependencies and re-run this script.${RESET}"
  echo ""
  exit 1
fi

# ── Get latest release version ────────────────────────────────────────────────
echo "  Checking latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' \
  | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$LATEST" ]; then
  echo "${RED}  Could not fetch latest release. Check your internet connection.${RESET}"
  exit 1
fi

echo "  Latest version: ${CYAN}$LATEST${RESET}"
echo ""

# ── Download and install the binary ──────────────────────────────────────────
BINARY_URL="https://github.com/$REPO/releases/download/$LATEST/$ARTIFACT"
TMP_BIN="$(mktemp)"

echo "  Downloading $ARTIFACT..."
curl -fsSL "$BINARY_URL" -o "$TMP_BIN"
chmod +x "$TMP_BIN"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP_BIN" "$INSTALL_DIR/$BINARY"
else
  echo "  Requesting sudo to install binary to $INSTALL_DIR..."
  sudo mv "$TMP_BIN" "$INSTALL_DIR/$BINARY"
fi

echo "  ${GREEN}✓ Binary installed${RESET}"

# ── Download and unpack the assets bundle ─────────────────────────────────────
ASSETS_URL="https://github.com/$REPO/releases/download/$LATEST/mimona-assets.zip"
TMP_ZIP="$(mktemp).zip"
TMP_DIR="$(mktemp -d)"

echo "  Downloading assets bundle..."
curl -fsSL "$ASSETS_URL" -o "$TMP_ZIP"

echo "  Unpacking assets..."
unzip -q "$TMP_ZIP" -d "$TMP_DIR"

# Install assets next to the binary so mimona serve finds them
ASSET_DEST="$(dirname "$(command -v $BINARY)")"

if [ -w "$ASSET_DEST" ]; then
  cp -r "$TMP_DIR/mimona-assets/whatsapp-bridge" "$ASSET_DEST/"
  cp -r "$TMP_DIR/mimona-assets/frontend"        "$ASSET_DEST/"
  cp    "$TMP_DIR/mimona-assets/registry.json"   "$ASSET_DEST/"
else
  echo "  Requesting sudo to install assets to $ASSET_DEST..."
  sudo cp -r "$TMP_DIR/mimona-assets/whatsapp-bridge" "$ASSET_DEST/"
  sudo cp -r "$TMP_DIR/mimona-assets/frontend"        "$ASSET_DEST/"
  sudo cp    "$TMP_DIR/mimona-assets/registry.json"   "$ASSET_DEST/"
fi

rm -f "$TMP_ZIP"
rm -rf "$TMP_DIR"

echo "  ${GREEN}✓ Assets installed${RESET}"

# Copy .env.example → .env for the bridge if not already present
ENV_FILE="$ASSET_DEST/whatsapp-bridge/.env"
ENV_EXAMPLE="$ASSET_DEST/whatsapp-bridge/.env.example"
if [ ! -f "$ENV_FILE" ] && [ -f "$ENV_EXAMPLE" ]; then
  cp "$ENV_EXAMPLE" "$ENV_FILE"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "${GREEN}${BOLD}  ✓ Mimona $LATEST installed!${RESET}"
echo ""
echo "  ${BOLD}Quick start:${RESET}"
echo ""
echo "    ${CYAN}mimona serve${RESET}              — start everything"
echo "    ${CYAN}mimona pull tinyllama:1b${RESET}  — download a model"
echo "    ${CYAN}mimona run tinyllama:1b${RESET}   — chat in the terminal"
echo ""
echo "  ${BOLD}Web UI:${RESET}       http://127.0.0.1:11435"
echo "  ${BOLD}WhatsApp:${RESET}     http://127.0.0.1:11435/whatsapp.html"
echo ""
echo "  ${BOLD}Docs:${RESET} https://github.com/$REPO"
echo ""
