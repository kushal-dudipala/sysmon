#!/usr/bin/env bash
set -euo pipefail

# Where this script lives -> repo root
cd "$(dirname "$0")/.."

# ---- paths ---------------------------------------------------------------
APP_ICONSET_DIR="macos/sysmon.iconset"  
ICNS_OUT="macos/sysmon.icns"

# Noto Emoji (tag v2.034) still provides per-emoji SVGs.
# “Mage” U+1F9D9:
SVG_URL="https://raw.githubusercontent.com/googlefonts/noto-emoji/v2.034/svg/emoji_u1f9d9.svg"
SVG_TMP="$(mktemp -t mage.XXXXXX).svg"

# ---- deps ---------------------------------------------------------------
need() { command -v "$1" >/dev/null 2>&1; }

# Prefer rsvg-convert (librsvg), fallback to inkscape
CONVERTER=""
if need rsvg-convert; then
  CONVERTER="rsvg"
elif need inkscape; then
  CONVERTER="inkscape"
else
  echo "Need an SVG→PNG renderer."
  echo "   Install one of:"
  echo "     brew install librsvg        # provides 'rsvg-convert'"
  echo "     brew install --cask inkscape"
  exit 1
fi

# ---- sanity check on iconset dir name -----------------------------------
if [[ "${APP_ICONSET_DIR##*.}" != "iconset" ]]; then
  echo "APP_ICONSET_DIR must end with .iconset (got: '$APP_ICONSET_DIR')"
  exit 2
fi

# ---- fetch SVG -----------------------------------------------------------
echo "⇣ Downloading Noto Emoji 🧙 SVG…"
curl -fsSL "$SVG_URL" -o "$SVG_TMP"

# ---- prepare iconset -----------------------------------------------------
rm -rf "$APP_ICONSET_DIR"
mkdir -p "$APP_ICONSET_DIR"

render_one() {
  local base="$1" scale="$2"
  local px=$(( base * scale ))
  local name="icon_${base}x${base}"
  [[ "$scale" -eq 2 ]] && name="${name}@2x"
  local out="$APP_ICONSET_DIR/${name}.png"

  if [[ "$CONVERTER" == "rsvg" ]]; then
    rsvg-convert -w "$px" -h "$px" "$SVG_TMP" -o "$out"
  else
    inkscape "$SVG_TMP" --export-filename="$out" --export-width="$px" --export-height="$px" >/dev/null 2>&1
  fi

  echo "$name.png"
}

# Required macOS iconset members (16/32/128/256/512 at 1x and 2x)
for size in 16 32 128 256 512; do
  render_one "$size" 1
  render_one "$size" 2
done

# ---- build .icns ---------------------------------------------------------
echo "Building $ICNS_OUT …"
iconutil -c icns "$APP_ICONSET_DIR" -o "$ICNS_OUT" || {
  echo "iconutil reported: Invalid Iconset."
  echo "   Check there are exactly 10 PNGs named:"
  echo "   icon_16x16.png, icon_16x16@2x.png, icon_32x32.png, icon_32x32@2x.png,"
  echo "   icon_128x128.png, icon_128x128@2x.png, icon_256x256.png, icon_256x256@2x.png,"
  echo "   icon_512x512.png, icon_512x512@2x.png"
  exit 3
}

echo "Wrote $ICNS_OUT"

# ----  add/refresh license notice for Noto Emoji ---------------
LICENSE_URL="https://fuchsia.googlesource.com/third_party/github.com/googlefonts/noto-emoji/+/refs/heads/main/LICENSE?format=TEXT"

mkdir -p THIRD_PARTY_LICENSES
if curl -fsSL "$LICENSE_URL" | base64 --decode > THIRD_PARTY_LICENSES/NotoEmoji-APACHE-2.0.txt; then
  echo "Added THIRD_PARTY_LICENSES/NotoEmoji-APACHE-2.0.txt."
else
  echo "Could not fetch Noto Emoji license text (network/URL issue)."
  echo "    You can add it later from the project mirror."
fi