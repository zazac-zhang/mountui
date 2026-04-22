#!/usr/bin/env bash
set -euo pipefail

# MountUI curl install script
# Usage: curl -fsSL https://raw.githubusercontent.com/USER/mountui/main/scripts/install.sh | bash

REPO="pony/mountui"
BIN_NAME="mountui"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_API="https://api.github.com/repos/${REPO}/releases"

# --- Color output helpers ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# --- Detect platform ---
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        *)       error "Unsupported OS: $os"; exit 1 ;;
    esac

    case "$arch" in
        x86_64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)       error "Unsupported architecture: $arch"; exit 1 ;;
    esac

    echo "${os}-${arch}"
}

# --- Detect download URL ---
# For releases: download from GitHub Releases assets
# For pre-release / no releases: fall back to building from source
get_release_url() {
    local platform="$1"

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        error "Neither curl nor wget is installed. Please install one of them."
        exit 1
    fi

    # Try to get the latest release
    local release_json=""
    if command -v curl &>/dev/null; then
        release_json="$(curl -fsSL "${GITHUB_API}/latest" 2>/dev/null)" || true
    else
        release_json="$(wget -qO- "${GITHUB_API}/latest" 2>/dev/null)" || true
    fi

    if [ -z "$release_json" ] || echo "$release_json" | grep -q "Not Found"; then
        warn "No release found. Falling back to build from source..."
        echo "SOURCE"
        return
    fi

    local tag download_url
    tag="$(echo "$release_json" | grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4)"
    local asset_pattern="${BIN_NAME}-${platform}"

    download_url="$(echo "$release_json" | grep -o "https://[^ ]*${asset_pattern}[^ ]*" | head -1)" || true

    if [ -z "$download_url" ]; then
        warn "No pre-built binary found for ${platform} in release ${tag}. Falling back to build from source..."
        echo "SOURCE"
        return
    fi

    echo "$download_url"
}

# --- Install from release ---
install_from_release() {
    local url="$1"
    local tmpfile
    tmpfile="$(mktemp)"

    info "Downloading ${BIN_NAME} from ${url}..."
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$tmpfile"
    else
        wget -q "$url" -O "$tmpfile"
    fi

    chmod +x "$tmpfile"
    mv "$tmpfile" "${INSTALL_DIR}/${BIN_NAME}"
    info "Installed to ${INSTALL_DIR}/${BIN_NAME}"
}

# --- Build from source ---
build_from_source() {
    info "Building ${BIN_NAME} from source..."

    # Check for rustc/cargo
    if ! command -v cargo &>/dev/null; then
        error "cargo is not installed. Please install Rust: https://rustup.rs/"
        error "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    local tmpdir
    tmpdir="$(mktemp -d)"
    info "Cloning repository..."
    git clone --depth 1 "https://github.com/${REPO}.git" "$tmpdir"

    cd "$tmpdir"
    info "Building release binary..."
    cargo build --release

    mkdir -p "$INSTALL_DIR"
    cp "target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"

    # Clean up temp dir
    rm -rf "$tmpdir"
    info "Installed to ${INSTALL_DIR}/${BIN_NAME}"
}

# --- Main ---
main() {
    info "MountUI Installer"
    echo ""

    # Allow override via env var
    if [ -n "${MOUNTUI_VERSION:-}" ]; then
        info "Requested version: ${MOUNTUI_VERSION}"
    fi

    # Ensure install directory exists
    mkdir -p "$INSTALL_DIR"
    if ! echo ":$PATH:" | grep -q ":${INSTALL_DIR}:"; then
        warn "${INSTALL_DIR} is not in your PATH."
        warn "Add it with: export PATH=\"\$PATH:${INSTALL_DIR}\""
        warn "Then run: ${BIN_NAME}"
    fi

    local platform
    platform="$(detect_platform)"
    info "Detected platform: ${platform}"

    local url
    url="$(get_release_url "$platform")"

    if [ "$url" = "SOURCE" ]; then
        build_from_source
    else
        install_from_release "$url"
    fi

    echo ""
    info "Installation complete!"
    info "Run '${BIN_NAME}' to start."
}

main
