#!/usr/bin/env bash
# Builds ODiagramador.app: Rust core (static) + SwiftUI app + bundled `odg` CLI/MCP server + render-kit.
#   apps/macos/build-app.sh [OUT_DIR]
# Env: UNIVERSAL=1  → arm64 + x86_64 (Apple Silicon and Intel) in one app
#      VERSION=x.y.z → CFBundleShortVersionString
#      ZIP=1        → also writes ODiagramador-<version>-macos[-universal].zip next to the app
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$ROOT/dist}"
export MACOSX_DEPLOYMENT_TARGET=13.0

cp "$ROOT/crates/odg-ffi/include/odg_ffi.h" "$HERE/Sources/COdg/odg_ffi.h"

if [[ "${UNIVERSAL:-0}" == "1" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --manifest-path "$ROOT/Cargo.toml" --target "$t" -p odg-ffi -p odg-cli
  done
  mkdir -p "$ROOT/target/universal"
  lipo -create "$ROOT"/target/{aarch64,x86_64}-apple-darwin/release/libodg_ffi.a -output "$ROOT/target/universal/libodg_ffi.a"
  lipo -create "$ROOT"/target/{aarch64,x86_64}-apple-darwin/release/odg -output "$ROOT/target/universal/odg"
  export ODG_FFI_LIB="$ROOT/target/universal/libodg_ffi.a"
  # one build per architecture + lipo (multi --arch builds need the full Xcode)
  for triple in arm64-apple-macosx13.0 x86_64-apple-macosx13.0; do
    (cd "$HERE" && swift build -c release --triple "$triple")
  done
  BIN="$ROOT/target/universal/ODiagramador"
  lipo -create \
    "$(cd "$HERE" && swift build -c release --triple arm64-apple-macosx13.0 --show-bin-path)/ODiagramador" \
    "$(cd "$HERE" && swift build -c release --triple x86_64-apple-macosx13.0 --show-bin-path)/ODiagramador" \
    -output "$BIN"
  ODG="$ROOT/target/universal/odg"
  SUFFIX="macos-universal"
else
  cargo build --release --manifest-path "$ROOT/Cargo.toml" -p odg-ffi -p odg-cli
  (cd "$HERE" && swift build -c release)
  BIN="$HERE/.build/release/ODiagramador"
  ODG="$ROOT/target/release/odg"
  SUFFIX="macos-$(uname -m)"
fi

APP="$OUT/ODiagramador.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/render-kit/vendor"
cp "$BIN" "$APP/Contents/MacOS/ODiagramador"
cp "$ODG" "$APP/Contents/MacOS/odg"
cp "$HERE/Info.plist" "$APP/Contents/Info.plist"
if [[ -n "${VERSION:-}" ]]; then
  plutil -replace CFBundleShortVersionString -string "$VERSION" "$APP/Contents/Info.plist"
fi
cp "$ROOT/LICENSE" "$APP/Contents/Resources/LICENSE"
cp "$ROOT/assets/app-icon/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
mkdir -p "$APP/Contents/Resources/icons"
cp "$ROOT/assets/icons/"*.svg "$ROOT/assets/icons/README.md" "$APP/Contents/Resources/icons/"
cp "$ROOT/render-kit/"{index.html,bridge.js,palette.js,renderers.js,viewer.css} "$APP/Contents/Resources/render-kit/"
cp "$ROOT/render-kit/vendor/"{mermaid-odg.min.js,mermaid.LICENSE,elkjs.LICENSE,MERMAID_VERSION,ELKJS_VERSION} "$APP/Contents/Resources/render-kit/vendor/"
# ad-hoc signature so Gatekeeper accepts the local build (use a Developer ID for distribution)
codesign --force --deep --sign - "$APP" >/dev/null
echo "$APP"

if [[ "${ZIP:-0}" == "1" ]]; then
  V="$(plutil -extract CFBundleShortVersionString raw "$APP/Contents/Info.plist")"
  ZIPFILE="$OUT/ODiagramador-$V-$SUFFIX.zip"
  rm -f "$ZIPFILE"
  ditto -c -k --keepParent "$APP" "$ZIPFILE"
  echo "$ZIPFILE"
fi
