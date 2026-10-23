#!/usr/bin/env bash
# Builds render-kit/vendor/mermaid-odg.min.js: Mermaid (ESM, for its layout API) + elkjs +
# ODiagramador's layouts (render-kit/src) in one offline IIFE bundle.
# Gera o bundle offline do render-kit (Mermaid + ELK + layouts do ODiagramador).
set -euo pipefail

MERMAID_VERSION="${MERMAID_VERSION:-11.17.2}"
ELKJS_VERSION="${ELKJS_VERSION:-0.9.3}"
ESBUILD_VERSION="${ESBUILD_VERSION:-0.25.10}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cp -R "$ROOT/render-kit/src" "$WORK/src"
cd "$WORK"
npm init -y >/dev/null
npm install --silent --no-audit --no-fund \
  "mermaid@${MERMAID_VERSION}" "elkjs@${ELKJS_VERSION}" "esbuild@${ESBUILD_VERSION}"

npx esbuild src/entry.mjs \
  --bundle --minify --format=iife --platform=browser --target=es2020 \
  --legal-comments=none --log-level=warning \
  --outfile="$ROOT/render-kit/vendor/mermaid-odg.min.js"

cp node_modules/mermaid/LICENSE "$ROOT/render-kit/vendor/mermaid.LICENSE"
cp node_modules/elkjs/LICENSE.md "$ROOT/render-kit/vendor/elkjs.LICENSE" 2>/dev/null \
  || cp node_modules/elkjs/LICENSE "$ROOT/render-kit/vendor/elkjs.LICENSE"
echo "$MERMAID_VERSION" > "$ROOT/render-kit/vendor/MERMAID_VERSION"
echo "$ELKJS_VERSION" > "$ROOT/render-kit/vendor/ELKJS_VERSION"
ls -la "$ROOT/render-kit/vendor/mermaid-odg.min.js"
