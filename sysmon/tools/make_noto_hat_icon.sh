#!/usr/bin/env bash
set -euo pipefail

# --- security/reproducibility defaults ---------------------------------------
umask 022

# Absolute paths for core tools (PATH-safe)
CURL="/usr/bin/curl"
ICONUTIL="/usr/bin/iconutil"
SHASUM="/usr/bin/shasum"
MKDIR="/bin/mkdir"
RM="/bin/rm"
MKDIR_BIN="$MKDIR"  # just for readability

# Fail early if required tools are missing
for t in "$CURL" "$ICONUTIL" "$SHASUM" "$MKDIR" "$RM" "/usr/bin/mktemp"; do
  [ -x "$t" ] || { echo "Missing required tool: $t"; exit 1; }
done

# --- repo root ----------------------------------------------------------------
cd "$(dirname "$0")/.."

# ---- paths -------------------------------------------------------------------
APP_ICONSET_DIR="macos/sysmon.iconset"
ICNS_OUT="macos/sysmon.icns"

# ---- source pinning -----------------------------------------------------------
# Pin to an immutable commit (Noto Emoji v2.034)
SVG_COMMIT="${SVG_COMMIT:-9a5261d871451f9b5183c93483cbd68ed916b1e9}"

# Emoji codepoint (default: 'mage' U+1F9D9). Allow override via env.
SVG_CODEPOINT="${SVG_CODEPOINT:-1f9d9}"

# Raw GitHub URLs pinned to the commit (no base64 decode needed)
SVG_URL="https://raw.githubusercontent.com/googlefonts/noto-emoji/${SVG_COMMIT}/svg/emoji_u${SVG_CODEPOINT}.svg"
LICENSE_URL="https://raw.githubusercontent.com/googlefonts/noto-emoji/${SVG_COMMIT}/LICENSE"

# Optional checksum for the SVG (set SVG_SHA256 to enforce)
SVG_SHA256="${SVG_SHA256:-}"

# Temp file with guaranteed cleanup
SVG_TMP="$(/usr/bin/mktemp -t mage.XXXXXX).svg"
trap 'rm -f "$SVG_TMP"' EXIT

# ---- deps (renderer) ----------------------------------------------------------
# Prefer rsvg-convert (librsvg), fallback to inkscape; store absolute path
need() { command -v "$1" >/dev/null 2>&1; }

CONVERTER_BIN=""
CONVERTER_KIND=""
if need rsvg-convert; then
  CONVERTER_BIN="$(command -v rsvg-convert)"
  CONVERTER_KIND="rsvg"
elif need inkscape; then
  CONVERTER_BIN="$(command -v inkscape)"
  CONVERTER_KIND="inkscape"
else
  echo "Need an SVG→PNG renderer."
  echo "   brew install librsvg        # provides 'rsvg-convert'"
  echo "   or: brew install --cask inkscape"
  exit 1
fi

# ---- fetch SVG ---------------------------------------------------------------
echo "⇣ Downloading Noto Emoji SVG (u${SVG_CODEPOINT})…"
"$CURL" -fsSL "$SVG_URL" -o "$SVG_TMP"

if [[ -n "$SVG_SHA256" ]]; then
  echo "${SVG_SHA256}  ${SVG_TMP}" | "$SHASUM" -a 256 -c - \
    || { echo "SVG sha256 mismatch; aborting."; exit 1; }
  echo "🔒 SVG sha256 verified."
else
  echo "ℹ️  No SVG_SHA256 provided; skipping checksum verification."
fi

# ---- prepare iconset ---------------------------------------------------------
$RM -rf "$APP_ICONSET_DIR"
$MKDIR_BIN -p "$APP_ICONSET_DIR"

render_one() {
  local base="$1" scale="$2"
  local px=$(( base * scale ))
  local name="icon_${base}x${base}"
  [[ "$scale" -eq 2 ]] && name="${name}@2x"
  local out="$APP_ICONSET_DIR/${name}.png"

  if [[ "$CONVERTER_KIND" == "rsvg" ]]; then
    "$CONVERTER_BIN" -w "$px" -h "$px" "$SVG_TMP" -o "$out"
  else
    # Inkscape CLI (1.0+)
    "$CONVERTER_BIN" "$SVG_TMP" \
      --export-filename="$out" \
      --export-width="$px" \
      --export-height="$px" >/dev/null 2>&1
  fi

  echo "✅ ${name}.png"
}

# Required macOS iconset members (16/32/128/256/512 at 1x and 2x)
for size in 16 32 128 256 512; do
  render_one "$size" 1
  render_one "$size" 2
done

# ---- build .icns -------------------------------------------------------------
echo "🧩 Building $ICNS_OUT …"
"$ICONUTIL" -c icns "$APP_ICONSET_DIR" -o "$ICNS_OUT" || {
  echo "❌ iconutil reported: Invalid Iconset."
  echo "   Expect exactly 10 PNGs:"
  echo "   icon_16x16.png, icon_16x16@2x.png, icon_32x32.png, icon_32x32@2x.png,"
  echo "   icon_128x128.png, icon_128x128@2x.png, icon_256x256.png, icon_256x256@2x.png,"
  echo "   icon_512x512.png, icon_512x512@2x.png"
  exit 3
}
echo "🎉 Wrote $ICNS_OUT"

# ---- add/refresh license notice for Noto Emoji -------------------------------
mkdir -p THIRD_PARTY_LICENSES
if "$CURL" -fsSL "$LICENSE_URL" > THIRD_PARTY_LICENSES/NotoEmoji-APACHE-2.0.txt; then
  echo "ℹ️  Added THIRD_PARTY_LICENSES/NotoEmoji-APACHE-2.0.txt."
else
  echo "⚠️  Could not fetch Noto Emoji license text."
fi
