#!/usr/bin/env bash
# Builds render-kit/vendor/mermaid-odg.min.js: Mermaid (ESM, for its layout API) + elkjs +
# ODiagramador's layouts (render-kit/src) in one offline IIFE bundle.
# Gera o bundle offline do render-kit (Mermaid + ELK + layouts do ODiagramador).
set -euo pipefail

MERMAID_VERSION="${MERMAID_VERSION:-12.2.0}"
ELKJS_VERSION="${ELKJS_VERSION:-0.9.3}"
ESBUILD_VERSION="${ESBUILD_VERSION:-0.25.10}"
ZENUML_VERSION="${ZENUML_VERSION:-1.0.1}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cp -R "$ROOT/render-kit/src" "$WORK/src"
cd "$WORK"
npm init -y >/dev/null
npm install --silent --no-audit --no-fund \
  "mermaid@${MERMAID_VERSION}" "elkjs@${ELKJS_VERSION}" "esbuild@${ESBUILD_VERSION}" \
  "@mermaid-js/mermaid-zenuml@${ZENUML_VERSION}"

# Mermaid's layout helper (createCommonLayoutRenderer): exported by the package up to 11.x; from 12 it only
# lives in an internal chunk (the same module instance the core uses), so point at whichever has it.
if grep -q "createCommonLayoutRenderer" node_modules/mermaid/dist/mermaid.core.mjs; then
  echo 'export { createCommonLayoutRenderer } from "mermaid";' > src/mermaid-layout-api.mjs
else
  CHUNK="$(grep -l "function createCommonLayoutRenderer" node_modules/mermaid/dist/chunks/mermaid.core/*.mjs | head -1)"
  [[ -n "$CHUNK" ]] || { echo "createCommonLayoutRenderer not found in mermaid@$MERMAID_VERSION" >&2; exit 1; }
  echo "export { createCommonLayoutRenderer } from \"mermaid/${CHUNK#node_modules/mermaid/}\";" > src/mermaid-layout-api.mjs
fi
cat src/mermaid-layout-api.mjs

npx esbuild src/entry.mjs \
  --bundle --minify --format=iife --platform=browser --target=es2020 \
  --legal-comments=none --log-level=warning \
  --outfile="$ROOT/render-kit/vendor/mermaid-odg.min.js"

cp node_modules/mermaid/LICENSE "$ROOT/render-kit/vendor/mermaid.LICENSE"
cp node_modules/elkjs/LICENSE.md "$ROOT/render-kit/vendor/elkjs.LICENSE" 2>/dev/null \
  || cp node_modules/elkjs/LICENSE "$ROOT/render-kit/vendor/elkjs.LICENSE"
echo "$MERMAID_VERSION" > "$ROOT/render-kit/vendor/MERMAID_VERSION"
echo "$ELKJS_VERSION" > "$ROOT/render-kit/vendor/ELKJS_VERSION"
cp node_modules/@mermaid-js/mermaid-zenuml/LICENSE "$ROOT/render-kit/vendor/zenuml.LICENSE" 2>/dev/null || true
ls -la "$ROOT/render-kit/vendor/mermaid-odg.min.js"
