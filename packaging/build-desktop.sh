#!/usr/bin/env bash
# ONEOFFICE Cross-Platform Desktop Packaging Script
set -euo pipefail

echo "============================================="
echo " ONEOFFICE Cross-Platform Desktop Packaging  "
echo "============================================="

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-native}"
OUTPUT_DIR="$ROOT_DIR/dist/desktop"
mkdir -p "$OUTPUT_DIR"

echo "Target: $TARGET"
echo "Output Directory: $OUTPUT_DIR"

case "$TARGET" in
  native)
    echo "Building release binary for current host platform..."
    cargo build --release -p oneoffice-desktop
    cp "target/release/oneoffice-desktop" "$OUTPUT_DIR/"
    echo "Built: $OUTPUT_DIR/oneoffice-desktop"
    ;;
  macos-universal)
    echo "Building macOS Universal Binary (arm64 + x86_64)..."
    rustup target add aarch64-apple-darwin x86_64-apple-darwin || true
    cargo build --release -p oneoffice-desktop --target aarch64-apple-darwin
    cargo build --release -p oneoffice-desktop --target x86_64-apple-darwin
    lipo -create \
      "target/aarch64-apple-darwin/release/oneoffice-desktop" \
      "target/x86_64-apple-darwin/release/oneoffice-desktop" \
      -output "$OUTPUT_DIR/ONEOFFICE-macos-universal"
    chmod +x "$OUTPUT_DIR/ONEOFFICE-macos-universal"
    echo "Built: $OUTPUT_DIR/ONEOFFICE-macos-universal"
    ;;
  linux-release)
    echo "Building Linux x86_64 release binary..."
    cargo build --release -p oneoffice-desktop --target x86_64-unknown-linux-gnu
    cp "target/x86_64-unknown-linux-gnu/release/oneoffice-desktop" "$OUTPUT_DIR/ONEOFFICE-linux-x86_64"
    echo "Built: $OUTPUT_DIR/ONEOFFICE-linux-x86_64"
    ;;
  windows-release)
    echo "Building Windows x86_64 release binary..."
    cargo build --release -p oneoffice-desktop --target x86_64-pc-windows-msvc
    cp "target/x86_64-pc-windows-msvc/release/oneoffice-desktop.exe" "$OUTPUT_DIR/ONEOFFICE-windows-x64.exe"
    echo "Built: $OUTPUT_DIR/ONEOFFICE-windows-x64.exe"
    ;;
  *)
    echo "Unknown target: $TARGET"
    echo "Usage: $0 [native|macos-universal|linux-release|windows-release]"
    exit 1
    ;;
esac

echo "Packaging complete."
