#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/leluongiac/mdf.git"

# Catppuccin Mocha colors (with fallback if stdout is not a TTY)
if [ -t 1 ]; then
  PEACH='\033[38;2;250;179;135m'
  GREEN='\033[38;2;166;227;161m'
  YELLOW='\033[38;2;249;226;175m'
  RED='\033[38;2;243;139;168m'
  SUBTEXT='\033[38;2;166;173;200m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  PEACH='' GREEN='' YELLOW='' RED='' SUBTEXT='' BOLD='' RESET=''
fi

info()    { printf "${PEACH}${BOLD}==>${RESET} %s\n" "$1"; }
success() { printf "${GREEN}${BOLD}✔${RESET} %s\n" "$1"; }
warn()    { printf "${YELLOW}${BOLD}!${RESET} %s\n" "$1"; }
error()   { printf "${RED}${BOLD}✘${RESET} %s\n" "$1" >&2; }

CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
CARGO_BIN_DIR="$CARGO_HOME_DIR/bin"

info "Installing mdf (Make Desktop File)..."

# 1. Check for Cargo / Rust toolchain
if ! command -v cargo >/dev/null 2>&1; then
  warn "Cargo (the Rust package manager) was not found on your system."
  printf "${SUBTEXT}mdf is written in Rust and needs to be compiled from source using Cargo.${RESET}\n"

  reply="n"
  if [ -r /dev/tty ]; then
    printf "Would you like to install the Rust toolchain via rustup now? [y/N]: "
    read -r reply </dev/tty || reply="n"
  fi

  case "$reply" in
    [yY]|[yY][eE][sS])
      if ! command -v curl >/dev/null 2>&1; then
        error "'curl' is required to download rustup. Please install curl and try again."
        exit 1
      fi
      info "Installing Rust via rustup..."
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      if [ -f "$CARGO_HOME_DIR/env" ]; then
        # shellcheck disable=SC1091
        . "$CARGO_HOME_DIR/env"
      fi
      if ! command -v cargo >/dev/null 2>&1; then
        error "Cargo is still unavailable after Rust installation."
        printf "${SUBTEXT}Please restart your shell or ensure '%s' is in PATH, then run this script again.${RESET}\n" "$CARGO_BIN_DIR" >&2
        exit 1
      fi
      ;;
    *)
      error "Installation canceled. Please install Rust from https://rustup.rs/ and run this script again."
      exit 1
      ;;
  esac
fi

# 2. Detect local repository via script path (safe against PWD and 'curl | bash')
LOCAL_REPO=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  if [ -f "$SCRIPT_DIR/Cargo.toml" ] && grep -q '^name = "mdf"' "$SCRIPT_DIR/Cargo.toml"; then
    LOCAL_REPO="$SCRIPT_DIR"
  fi
fi

# 3. Build and install mdf
if [ -n "$LOCAL_REPO" ]; then
  info "Local repository detected at '$LOCAL_REPO'. Building and installing..."
  cargo install --path "$LOCAL_REPO"
else
  if ! command -v git >/dev/null 2>&1; then
    error "'git' is required to clone the repository. Please install git and try again."
    exit 1
  fi

  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT

  info "Cloning repository into temporary directory..."
  git clone --depth 1 "$REPO_URL" "$TMP_DIR"

  info "Building and installing mdf..."
  cargo install --path "$TMP_DIR"
fi

# 4. Verify installed binary
if [ -x "$CARGO_BIN_DIR/mdf" ]; then
  INSTALLED_BIN="$CARGO_BIN_DIR/mdf"
else
  INSTALLED_BIN="$(command -v mdf || true)"
fi

if [ -z "$INSTALLED_BIN" ] || [ ! -x "$INSTALLED_BIN" ]; then
  error "Installation finished, but the 'mdf' executable could not be found."
  exit 1
fi

success "mdf has been installed to $INSTALLED_BIN"

# 5. Check optional system utilities
if ! command -v desktop-file-validate >/dev/null 2>&1 || ! command -v update-desktop-database >/dev/null 2>&1; then
  printf "\n"
  warn "Optional package 'desktop-file-utils' was not fully detected."
  printf "${SUBTEXT}Installing 'desktop-file-utils' via your distro's package manager enables automatic validation and menu database refreshes.${RESET}\n"
fi

# 6. Verify binary is reachable in PATH
if command -v mdf >/dev/null 2>&1; then
  printf "\n"
  success "Ready! Run 'mdf' from anywhere in your terminal."
else
  printf "\n"
  warn "'$CARGO_BIN_DIR' is not currently in your PATH."
  printf "${SUBTEXT}Add the following line to your ~/.bashrc or ~/.zshrc, then restart your terminal:${RESET}\n"
  printf "  export PATH=\"%s:\$PATH\"\n" "$CARGO_BIN_DIR"
fi