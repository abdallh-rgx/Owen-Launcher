#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# repack-apk.sh — make dist/OwenLauncher-v1.8.0-release.apk LEGALLY installable.
#
# The Rust packer produced an APK with three fatal defects (proven on the
# API-30 emulator, run #10/#11 of the Control Room):
#   1. broken v1 signature  → INSTALL_PARSE_FAILED_NO_CERTIFICATES
#   2. empty 12-byte resources.arsc (0 packages) → rejected resource table
#   3. hand-written binary AXML quirks → aapt: "missing android:name …"
#
# This script rebuilds the manifest + resource table with Google's own aapt
# (source: assets/tools/AndroidManifest.src.xml), transplants them into the
# packed APK, re-aligns and signs with apksigner (v2+v3). Result passes
# `aapt dump badging` and `apksigner verify` cleanly.
#
# Requirements (download once):
#   build-tools: https://dl.google.com/android/repository/build-tools_r34-linux.zip
#   platform:    https://dl.google.com/android/repository/platform-34_r02.zip
#   java (keytool) for the keystore
#
# Usage:
#   BT=/path/to/android-14 ./repack-apk.sh   # from owen-launcher-rs/tools/
# ---------------------------------------------------------------------------
set -euo pipefail
cd "$(dirname "$0")/.."

BT="${BT:?set BT=/path/to/build-tools/android-14}"
AJ="${AJ:-/tmp/plat/android-34/android.jar}"
KEYSTORE="assets/tools/owen-debug.keystore"
KS_PASS="${KS_PASS:-owen123}"
MANIFEST_SRC="assets/tools/AndroidManifest.src.xml"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

IN="dist/OwenLauncher-v1.8.0-release.apk"
[ -f "$IN" ] || { echo "missing $IN"; exit 1; }

echo "[1/6] aapt: compile the real manifest + resource table"
mkdir -p "$WORK/res/values"
cat > "$WORK/res/values/strings.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<resources>
  <string name="app_name">Owen Launcher</string>
</resources>
EOF
"$BT/aapt" package -f -M "$MANIFEST_SRC" -S "$WORK/res" -I "$AJ" -F "$WORK/stub.apk"
cd "$WORK" && unzip -q -o stub.apk -d stub_x && cd - >/dev/null

echo "[2/6] transplant manifest + resources.arsc into the packed APK"
python3 - "$IN" "$WORK/out.apk" "$WORK/stub_x" <<'PY'
import sys, zipfile
src, dst, stub = sys.argv[1:4]
new_manifest = open(f"{stub}/AndroidManifest.xml", "rb").read()
new_arsc = open(f"{stub}/resources.arsc", "rb").read()
zin = zipfile.ZipFile(src)
zout = zipfile.ZipFile(dst, "w")
for it in zin.infolist():
    if it.filename == "resources.arsc":
        zout.writestr(zipfile.ZipInfo(it.filename), new_arsc, compress_type=zipfile.ZIP_STORED)
    elif it.filename == "AndroidManifest.xml":
        zout.writestr(zipfile.ZipInfo(it.filename), new_manifest, compress_type=zipfile.ZIP_STORED)
    else:
        zout.writestr(it, zin.read(it.filename), compress_type=it.compress_type)
zout.close()
print("  manifest + arsc transplanted")
PY

echo "[3/6] strip any stale META-INF signature remnants"
zip -q -d "$WORK/out.apk" 'META-INF/CERT.RSA' 'META-INF/CERT.SF' 'META-INF/MANIFEST.MF' 2>/dev/null || true

echo "[4/6] zipalign (-p 4)"
"$BT/zipalign" -f -p 4 "$WORK/out.apk" "$WORK/aligned.apk"
"$BT/zipalign" -c -p 4 "$WORK/aligned.apk"

echo "[5/6] apksigner (v1+v2+v3, owen-debug keystore)"
"$BT/apksigner" sign \
  --ks "$KEYSTORE" --ks-key-alias owen \
  --ks-pass "pass:$KS_PASS" --key-pass "pass:$KS_PASS" \
  --v1-signing-enabled true --v2-signing-enabled true --v3-signing-enabled true \
  --out "$WORK/final.apk" "$WORK/aligned.apk"

echo "[6/6] verify"
"$BT/apksigner" verify "$WORK/final.apk"
"$BT/aapt" dump badging "$WORK/final.apk" | head -3

cp "$WORK/final.apk" "$IN"
echo "REPACKED: $IN ($(stat -c%s "$IN") bytes)"
