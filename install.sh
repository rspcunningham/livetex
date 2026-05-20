#!/usr/bin/env sh
set -eu

WITH_DEPS=0

usage() {
  echo "Usage: ./install.sh [--with-deps]"
}

for arg in "$@"; do
  case "$arg" in
    --with-deps)
      WITH_DEPS=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "LiveTex requires macOS."
  exit 1
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

install_deps() {
  if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew is required for --with-deps: https://brew.sh"
    exit 1
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    brew install rust
  fi

  if ! command -v latexmk >/dev/null 2>&1; then
    brew install texlive
  fi

  if ! open -Ra Skim >/dev/null 2>&1; then
    brew install --cask skim
  fi
}

warn_missing_deps() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required. Install Rust or rerun with: ./install.sh --with-deps"
    exit 1
  fi

  if ! command -v latexmk >/dev/null 2>&1; then
    echo "warning: latexmk is not on PATH. Rerun with: ./install.sh --with-deps"
  fi

  if ! open -Ra Skim >/dev/null 2>&1; then
    echo "warning: Skim is not installed. Rerun with: ./install.sh --with-deps"
  fi
}

if [ "$WITH_DEPS" -eq 1 ]; then
  install_deps
fi

warn_missing_deps

cargo install --path "$SCRIPT_DIR" --force

defaults write net.sourceforge.skim-app.skim SKAutoCheckFileUpdate -bool true
defaults write net.sourceforge.skim-app.skim SKAutoReloadFileUpdate -bool true

echo "LiveTex installed."
echo "Run: livetex doctor"
