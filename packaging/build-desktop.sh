#!/usr/bin/env bash
# ONEOFFICE Cross-Platform Desktop Packaging Script
set -euo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH:-}"

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

    if [[ "$(uname -s)" == "Darwin" ]]; then
      echo "Assembling macOS Application Bundle (ONEOFFICE.app)..."
      APP_BUNDLE="$OUTPUT_DIR/ONEOFFICE.app"
      mkdir -p "$APP_BUNDLE/Contents/MacOS"
      mkdir -p "$APP_BUNDLE/Contents/Resources"
      cp "target/release/oneoffice-desktop" "$APP_BUNDLE/Contents/MacOS/oneoffice-desktop"
      chmod +x "$APP_BUNDLE/Contents/MacOS/oneoffice-desktop"
      if [[ -f "$ROOT_DIR/extensions/oneoffice-vscode/icon.png" ]]; then
        cp "$ROOT_DIR/extensions/oneoffice-vscode/icon.png" "$APP_BUNDLE/Contents/Resources/AppIcon.png"
      fi
      cat << 'PLIST' > "$APP_BUNDLE/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>oneoffice-desktop</string>
    <key>CFBundleIdentifier</key>
    <string>com.oneoffice.desktop</string>
    <key>CFBundleName</key>
    <string>ONEOFFICE</string>
    <key>CFBundleDisplayName</key>
    <string>ONEOFFICE</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0-alpha.1</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>HWPX Document</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>hwpx</string>
                <string>hwp</string>
            </array>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
        </dict>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Office Document</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>docx</string>
                <string>xlsx</string>
                <string>pptx</string>
                <string>pdf</string>
            </array>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
        </dict>
    </array>
</dict>
</plist>
PLIST
      echo "Assembled macOS App: $APP_BUNDLE"
    fi
    ;;
  macos-dmg)
    echo "Building macOS Application Bundle and Distributable DMG..."
    cargo build --release -p oneoffice-desktop
    cp "target/release/oneoffice-desktop" "$OUTPUT_DIR/"
    APP_BUNDLE="$OUTPUT_DIR/ONEOFFICE.app"
    mkdir -p "$APP_BUNDLE/Contents/MacOS"
    mkdir -p "$APP_BUNDLE/Contents/Resources"
    cp "target/release/oneoffice-desktop" "$APP_BUNDLE/Contents/MacOS/oneoffice-desktop"
    chmod +x "$APP_BUNDLE/Contents/MacOS/oneoffice-desktop"
    if [[ -f "$ROOT_DIR/extensions/oneoffice-vscode/icon.png" ]]; then
      cp "$ROOT_DIR/extensions/oneoffice-vscode/icon.png" "$APP_BUNDLE/Contents/Resources/AppIcon.png"
    fi
    cat << 'PLIST' > "$APP_BUNDLE/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>oneoffice-desktop</string>
    <key>CFBundleIdentifier</key>
    <string>com.oneoffice.desktop</string>
    <key>CFBundleName</key>
    <string>ONEOFFICE</string>
    <key>CFBundleDisplayName</key>
    <string>ONEOFFICE</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0-alpha.1</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>HWPX Document</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>hwpx</string>
                <string>hwp</string>
            </array>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
        </dict>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Office Document</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>docx</string>
                <string>xlsx</string>
                <string>pptx</string>
                <string>pdf</string>
            </array>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
        </dict>
    </array>
</dict>
</plist>
PLIST
    DMG_STAGE="$OUTPUT_DIR/dmg_stage"
    rm -rf "$DMG_STAGE"
    mkdir -p "$DMG_STAGE"
    cp -R "$APP_BUNDLE" "$DMG_STAGE/"
    ln -s /Applications "$DMG_STAGE/Applications"
    DMG_PATH="$OUTPUT_DIR/ONEOFFICE-0.1.0-alpha.1.dmg"
    rm -f "$DMG_PATH"
    hdiutil create -volname "ONEOFFICE" -srcfolder "$DMG_STAGE" -ov -format UDZO "$DMG_PATH"
    rm -rf "$DMG_STAGE"
    echo "Created Distributable macOS DMG: $DMG_PATH"
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
    echo "Usage: $0 [native|macos-dmg|macos-universal|linux-release|windows-release]"
    exit 1
    ;;
esac

echo "Packaging complete."
