#!/bin/sh
# icon.sh: renders assets/icon/headquarter.svg into assets/icon/png/ and
# assets/icon/Headquarter.icns. macOS only; the outputs are committed, so releases and
# Linux need nothing but the files.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
out=$repo/assets/icon
mkdir -p "$out/png"
swift "$repo/scripts/render-icon.swift" "$out/headquarter.svg" "$out/png"
set_dir=$(mktemp -d)/Headquarter.iconset
mkdir -p "$set_dir"
for size in 16 32 128 256 512; do
  cp "$out/png/$size.png" "$set_dir/icon_${size}x${size}.png"
  cp "$out/png/$((size * 2)).png" "$set_dir/icon_${size}x${size}@2x.png"
done
iconutil --convert icns --output "$out/Headquarter.icns" "$set_dir"
rm -r "$(dirname "$set_dir")"
