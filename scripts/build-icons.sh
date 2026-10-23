#!/usr/bin/env bash
# Regenerates the app icons from assets/app-icon/odiagramador.svg (+ the macOS variant).
# Needs a rasterizer: rsvg-convert, or SVGPNG=/path/to/tool taking <in.svg> <out.png> <size>.
# Outputs (committed): assets/app-icon/AppIcon.icns (macOS), odiagramador.ico (Windows), png/ (Linux, docs).
set -euo pipefail
cd "$(dirname "$0")/../assets/app-icon"
render() {
  if [[ -n "${SVGPNG:-}" ]]; then "$SVGPNG" "$1" "$2" "$3"
  else rsvg-convert -w "$3" -h "$3" "$1" -o "$2"; fi
}
tmp=$(mktemp -d)
# macOS .icns
set_dir="$tmp/AppIcon.iconset"; mkdir -p "$set_dir"
for s in 16 32 128 256 512; do
  render odiagramador-macos.svg "$set_dir/icon_${s}x${s}.png" "$s"
  render odiagramador-macos.svg "$set_dir/icon_${s}x${s}@2x.png" "$((s * 2))"
done
iconutil -c icns "$set_dir" -o AppIcon.icns
# Windows .ico (PNG entries) and Linux/docs PNGs
mkdir -p png
for s in 16 24 32 48 64 128 256 512; do render odiagramador.svg "png/odiagramador-$s.png" "$s"; done
python3 - <<'PY'
import struct
sizes = [16, 24, 32, 48, 64, 128, 256]
blobs = [open(f"png/odiagramador-{s}.png", "rb").read() for s in sizes]
out = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
for s, b in zip(sizes, blobs):
    out += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(b), offset)
    offset += len(b)
open("odiagramador.ico", "wb").write(out + b"".join(blobs))
PY
rm -rf "$tmp"
echo "icons: AppIcon.icns odiagramador.ico png/"
