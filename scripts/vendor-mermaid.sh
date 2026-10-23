#!/usr/bin/env bash
# Baixa uma versão fixa do Mermaid para render-kit/vendor (uso offline).
# Downloads a pinned Mermaid build into render-kit/vendor (offline use).
set -euo pipefail

VERSION="${1:-11.17.2}"
EXPECTED_SHA256="581ed7d74bd9048d0e3a91363927d72ef22942d7722546b27f7cc29e35390eb8"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

(cd "$TMP" && npm pack "mermaid@${VERSION}" --silent >/dev/null && tar xzf "mermaid-${VERSION}.tgz")
cp "$TMP/package/dist/mermaid.min.js" "$ROOT/render-kit/vendor/mermaid.min.js"
cp "$TMP/package/LICENSE" "$ROOT/render-kit/vendor/mermaid.LICENSE"
echo "$VERSION" > "$ROOT/render-kit/vendor/MERMAID_VERSION"

ACTUAL="$(shasum -a 256 "$ROOT/render-kit/vendor/mermaid.min.js" | cut -d' ' -f1)"
if [[ "$VERSION" == "11.17.2" && "$ACTUAL" != "$EXPECTED_SHA256" ]]; then
  echo "checksum mismatch: $ACTUAL" >&2
  exit 1
fi
echo "mermaid $VERSION vendored ($ACTUAL)"
