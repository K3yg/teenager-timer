#!/usr/bin/env bash
# Build Stick Timer for macOS (universal: Apple Silicon + Intel).
# Run from anywhere:  bash desktop/scripts/build-mac.sh
set -euo pipefail
cd "$(dirname "$0")/.."

if ! xcode-select -p >/dev/null 2>&1; then
  echo "Installing Xcode Command Line Tools (a dialog will open). Run this script again when it finishes."
  xcode-select --install
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "Node.js is required: https://nodejs.org (or: brew install node)"
  exit 1
fi

rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm ci
npx tauri build --target universal-apple-darwin

OUT="src-tauri/target/universal-apple-darwin/release/bundle"
echo
echo "Done:"
ls -1 "$OUT/dmg/"*.dmg "$OUT/macos/"*.app 2>/dev/null
echo
echo "Install: open the .dmg and drag Stick Timer to Applications."
