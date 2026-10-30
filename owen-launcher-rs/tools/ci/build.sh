#!/usr/bin/env bash
# tools/ci/build.sh — Phase 2: build libOwenOGS.so against the real Android
# dump (Phase 1) and shim-patch the play APK.
#
# Inputs : /tmp/og-dump (or dumps/ogs-1241-android.tar.zst from the repo)
# Outputs: /tmp/fortnite-12.41-owen.apk, /tmp/phase2/ (logs + the .so chain)
# The libOwenOGS.so build is best-effort: a C++ error wave must never block
# the E2E — the game ships with libOwen.so (redirecter) + libOwenOGS.so only
# when it compiled. Both shapes get tested; CI reports which one ran.
set -euo pipefail
cd "$(dirname "$0")/../.."
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/home/z/ndk/android-ndk-r27c}"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:/home/z/.cargo/bin:$PATH"
mkdir -p /tmp/phase2

# ---- the dump: artifact first, committed fallback ---------------------------
if [ ! -f /tmp/og-dump/CppSDK/SDK.hpp ] && [ ! -f /tmp/og-dump/CppSDK/CppSDK/SDK.hpp ]; then
  if [ -f dumps/ogs-1241-android.tar.zst ]; then
    echo "== restoring the committed dump"
    mkdir -p /tmp/og-dump
    tar -xf dumps/ogs-1241-android.tar.zst -C /tmp
  fi
fi
DUMPROOT=/tmp/og-dump/CppSDK
[ -f "$DUMPROOT/CppSDK/SDK.hpp" ] && DUMPROOT=/tmp/og-dump/CppSDK/CppSDK
if [ ! -f "$DUMPROOT/SDK.hpp" ]; then
  echo "::error::no SDK dump available (phase 1 must run first)"; exit 1
fi
echo "== dump root: $DUMPROOT"

# ---- libowen.so + shim (already validated in phase 1, rebuild idempotently) --
CARGO_TARGET_DIR="$PWD/target" cargo build -p owen-redirecter --target aarch64-linux-android --release >/tmp/phase2/redirecter.log 2>&1
aarch64-linux-android24-clang -shared -fPIC -O2 -o target/shim/libUE4shim.so tools/shim/libue4_shim.c -landroid -llog

# ---- libOwenOGS.so — the real conversion, best-effort ------------------------
OGS=""
echo "== building libOwenOGS.so (the OGS 12.41 port) against the Android dump"
if OGS_SDK_DIR="$DUMPROOT" ANDROID_NDK_HOME="$ANDROID_NDK_HOME" \
   CARGO_TARGET_DIR="$PWD/target" \
   cargo build -p owen-ogs --target aarch64-linux-android --release >/tmp/phase2/ogs-build.log 2>&1; then
  if [ -f target/aarch64-linux-android/release/libOwenOGS.so ]; then
    OGS=target/aarch64-linux-android/release/libOwenOGS.so
    echo "== libOwenOGS.so OK"
  fi
else
  echo "::warning::libOwenOGS.so failed to compile — see ogs-build.log (the E2E continues redirecter-only)"
fi
[ -s /tmp/phase2/ogs-build.log ] && { echo '--- last 40 lines ---'; tail -40 /tmp/phase2/ogs-build.log; } || true

# ---- shim-patch the game ------------------------------------------------------
bash tools/ci/reassemble.sh /tmp/fortnite-12.41.apk
CARGO_TARGET_DIR="$PWD/target" cargo build --release -p owen-patcher --bin shim >/tmp/phase2/shimbin.log 2>&1 || true
[ -x target/release/shim ] || { tail -20 /tmp/phase2/shimbin.log; exit 1; }

MODS="--mod libOwen.so=$PWD/target/aarch64-linux-android/release/libowen.so"
if [ -n "$OGS" ]; then MODS="$MODS --mod libOwenOGS.so=$PWD/$OGS"; fi
echo "== shim-patch: $MODS"
./target/release/shim --in /tmp/fortnite-12.41.apk --out /tmp/fortnite-12.41-owen.apk \
  --shim target/shim/libUE4shim.so $MODS --min-bytes 1000000 > /tmp/phase2/patch-report.json || {
  cat /tmp/phase2/patch-report.json; exit 1; }
cp /tmp/phase2/patch-report.json /tmp/phase2/patch-report.json.bak 2>/dev/null || true
ls -la /tmp/fortnite-12.41-owen.apk
echo "WITH_OGS=$([ -n "$OGS" ] && echo 1 || echo 0)" > /tmp/phase2/flags
unzip -l /tmp/fortnite-12.41-owen.apk | grep 'lib/' | head -12
