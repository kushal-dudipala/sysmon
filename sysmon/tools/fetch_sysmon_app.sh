#!/usr/bin/env bash
set -euo pipefail

APP=sysmon
PROFILE=release   # or "dev"

# Paths
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"      
REPO_PARENT="$(cd -- "${REPO_ROOT}/.." && pwd)"     

# Build
echo "→ Building ($PROFILE)…"
cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --${PROFILE}

BIN="${REPO_ROOT}/target/${PROFILE}/${APP}"

# Create .app inside repo root (unchanged)
APPDIR="${REPO_ROOT}/${APP}.app"
CONTENTS="${APPDIR}/Contents"
MACOS_DIR="${CONTENTS}/MacOS"

echo "→ Creating app bundle at: ${APPDIR}"
rm -rf "${APPDIR}"
mkdir -p "${MACOS_DIR}"

cp "${REPO_ROOT}/macos/Info.plist" "${CONTENTS}/Info.plist"
cp "${BIN}" "${MACOS_DIR}/${APP}"

# Ad‑hoc sign 
if command -v codesign >/dev/null 2>&1; then
  echo "→ Ad-hoc signing bundle…"
  codesign --force --deep -s - "${APPDIR}" || echo "codesign (ad-hoc) failed; continuing"
else
  echo "codesign not found; skipping sign"
fi

# Create ZIP
ZIP_PATH="${REPO_PARENT}/${APP}.zip"
echo "→ Creating ZIP at: ${ZIP_PATH}"
rm -f "${ZIP_PATH}"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "${APPDIR}" "${ZIP_PATH}"

echo "Built app: ${APPDIR}"
echo "Built zip: ${ZIP_PATH}"
