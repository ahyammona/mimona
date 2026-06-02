#!/usr/bin/env sh
set -e

REPO="ahyammona/mimona"   # change to your GitHub username/repo
INSTALL_DIR="/usr/local/bin"
BINARY="mimona"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

echo ""
echo "${BOLD}  Installing Mimona...${RESET}"
echo ""

# Detect OS and arch
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

# Get latest release version from GitHub API
echo "  Checking latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' \
  | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$LATEST" ]; then
  echo "${RED}  Could not fetch latest release. Check your internet connection.${RESET}"
  exit 1
fi

echo "  Latest version: ${CYAN}$LATEST${RESET}"

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/$ARTIFACT"

# Download
TMP="$(mktemp)"
echo "  Downloading $ARTIFACT..."
curl -fsSL "$DOWNLOAD_URL" -o "$TMP"
chmod +x "$TMP"

# Install
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP" "$INSTALL_DIR/$BINARY"
else
  echo "  Requesting sudo to install to $INSTALL_DIR..."
  sudo mv "$TMP" "$INSTALL_DIR/$BINARY"
fi

echo ""
echo "${GREEN}  ✓ Mimona $LATEST installed!${RESET}"
echo ""
echo "  Run:  ${BOLD}mimona serve${RESET}"
echo "  Then open: ${CYAN}http://127.0.0.1:11435${RESET}"
echo ""