#!/usr/bin/env bash
# Builds ODiagramador.app: Rust core (static) + SwiftUI app + bundled `odg` CLI/MCP server + render-kit.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$ROOT/dist}"

cargo build --release --manifest-path "$ROOT/Cargo.toml" -p odg-ffi -p odg-cli
cp "$ROOT/crates/odg-ffi/include/odg_ffi.h" "$HERE/Sources/COdg/odg_ffi.h"
(cd "$HERE" && swift build -c release)

APP="$OUT/ODiagramador.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/render-kit/vendor"
cp "$HERE/.build/release/ODiagramador" "$APP/Contents/MacOS/ODiagramador"
cp "$ROOT/target/release/odg" "$APP/Contents/MacOS/odg"
cp "$HERE/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/render-kit/"{index.html,bridge.js,viewer.css} "$APP/Contents/Resources/render-kit/"
cp "$ROOT/render-kit/vendor/"{mermaid.min.js,mermaid.LICENSE,MERMAID_VERSION} "$APP/Contents/Resources/render-kit/vendor/"
# ad-hoc signature so Gatekeeper accepts the local build (use a Developer ID for distribution)
codesign --force --deep --sign - "$APP" >/dev/null
echo "$APP"
