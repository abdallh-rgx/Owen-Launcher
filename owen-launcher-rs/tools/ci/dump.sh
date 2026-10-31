#!/usr/bin/env bash
# tools/ci/dump.sh — Phase 1: the MobileDumper-7 SDK dump of the REAL Android
# 12.41 game, taken inside the booted emulator.
#
# Strategy A (preferred): stock APK + the x86_64 MobileDumper-7 executable
#   run from the adb shell as root — zero APK modification.
# Strategy B (fallback, SELinux/ptrace issues): the shim-injected APK with
#   libOwenDumper.so (the arm64 MobileDumper lib auto-dumps 60s after load)
#   — the exact injection path the product uses.
#
# Output: /tmp/og-dump/ (CppSDK/) + /tmp/og-dump.tar.zst
set -euo pipefail
cd "$(dirname "$0")/../.."
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/home/z/ndk/android-ndk-r27c}"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
for d in "$ANDROID_HOME"/cmdline-tools/*/bin; do [ -d "$d" ] && export PATH="$d:$PATH"; done
PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:/home/z/.cargo/bin:$PATH"

OUT=/tmp/og-dump
rm -rf "$OUT"; mkdir -p "$OUT"
DUMP_TAR=/tmp/og-dump.tar.zst

# ---- build both dumper flavors from vendor/MobileDumper-7 -------------------
echo "== building MobileDumper-7 (arm64 lib + x86_64 exe)"
( cd vendor/MobileDumper-7
  ./build.sh --platform android --target lib --build release --arch arm64 >/tmp/md-lib.log 2>&1 || { tail -20 /tmp/md-lib.log; exit 1; }
  ./build.sh --platform android --target exe --build release --arch x86_64 >/tmp/md-exe.log 2>&1 || { tail -20 /tmp/md-exe.log; exit 1; } )
EXE=vendor/MobileDumper-7/builds/android/exe/release/x86_64/MobileDumper-7
LIB=vendor/MobileDumper-7/builds/android/lib/release/arm64/libMobileDumper-7.so
ls -la "$EXE" "$LIB"

# ---- build the shim + libowen.so (needed for strategy B and phase 2) --------
echo "== building libowen.so + the loader shim"
mkdir -p target/shim
aarch64-linux-android24-clang -shared -fPIC -O2 -o target/shim/libUE4shim.so tools/shim/libue4_shim.c -landroid -llog
CARGO_TARGET_DIR="$PWD/target" cargo build -p owen-redirecter --target aarch64-linux-android --release 2>&1 | grep -v "warning" || true
test -f target/aarch64-linux-android/release/libowen.so

# ---- the stock APK ----------------------------------------------------------
bash tools/ci/reassemble.sh /tmp/fortnite-12.41.apk

# ---- the emulator (API 30 google_apis x86_64, ARM translation) ----------------
bash tools/ci/emulator-up.sh

launch() {
  echo "== launching com.epicgames.fortnite"
  adb shell monkey -p com.epicgames.fortnite -c android.intent.category.LAUNCHER 1 2>&1 | tail -3
}

# The game boots through an Android runtime-permission gate ("Choose what to
# allow Fortnite to access") BEFORE GameActivity ever loads libUE4 — the
# engine wait loop dead-ends on it. The gate is a REAL system view, so
# uiautomator sees its buttons: dump the tree, tap the exact node centers
# (Continue / Allow / button1), fall back to blind taps.
gate_tap_text() {
  # $1 = the text to find; taps the node's bounds center via uiautomator.
  adb shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1 || return 1
  local xml
  xml=$(adb shell cat /sdcard/ui.xml 2>/dev/null | tr -d '\r') || return 1
  local m
  m=$(printf '%s' "$xml" | grep -o "text=\"$1\"[^>]*bounds=\"\[[0-9]*,[0-9]*\]\[[0-9]*,[0-9]*\]\"" | head -1)
  [ -z "$m" ] && return 1
  local b
  b=$(printf '%s' "$m" | grep -o '\[[0-9]*,[0-9]*\]\[[0-9]*,[0-9]*\]')
  local x1 y1 x2 y2
  x1=$(printf '%s' "$b" | cut -d, -f1 | tr -d '[')
  y1=$(printf '%s' "$b" | cut -d, -f2 | tr -d ']')
  x2=$(printf '%s' "$b" | cut -d, -f3 | tr -d '[')
  y2=$(printf '%s' "$b" | cut -d, -f4 | tr -d ']')
  adb shell input tap $(((x1 + x2) / 2)) $(((y1 + y2) / 2)) >/dev/null 2>&1 && echo "  tapped '$1' @ ($(((x1 + x2) / 2)),$(((y1 + y2) / 2)))"
}

dismiss_gates() {
  local size W H focus i tapped
  size=$(adb shell wm size 2>/dev/null | grep -o '[0-9]*x[0-9]*' | head -1)
  W=${size%x*}; W=${W:-1080}
  H=${size#*x}; H=${H:-2340}
  echo "== gate dismissal (${W}x${H}; no early exit, verbose)"
  for i in 1 2 3 4 5 6 7 8 9 10 11 12; do
    sleep 4
    focus=$(adb shell dumpsys window 2>/dev/null | grep -m1 -E 'mCurrentFocus' || true)
    echo "  gate[$i] $(date -u +%H:%M:%S): ${focus:0:92}"
    tapped=0
    case "$focus" in
      *permissioncontroller*|*GrantPermissions*|*packageinstaller*)
        # uiautomator first (exact node centers)
        if gate_tap_text "Continue"; then tapped=1; fi
        if [ $tapped -eq 0 ] && gate_tap_text "ALLOW"; then tapped=1; fi
        if [ $tapped -eq 0 ] && gate_tap_text "Allow"; then tapped=1; fi
        if [ $tapped -eq 0 ]; then
          # blind spots: the A11 permission sheet's action row (Cancel |
          # Continue) sits at the very bottom-right; try several candidates.
          adb shell input tap $((W * 72 / 100)) $((H * 88 / 100)) >/dev/null 2>&1
          sleep 1
          adb shell input tap $((W * 80 / 100)) $((H * 90 / 100)) >/dev/null 2>&1
          sleep 1
          adb shell input tap $((W * 65 / 100)) $((H * 92 / 100)) >/dev/null 2>&1
          echo "  gate[$i]: blind-tapped 3 Continue candidates"
        fi
        ;;
    esac
    # log whether the gate is gone
    focus=$(adb shell dumpsys window 2>/dev/null | grep -m1 -E 'mCurrentFocus' || true)
    case "$focus" in *permissioncontroller*) : ;; *) echo "  gate[$i]: focus left the permission controller";; esac
  done
  echo "== gates done"
  return 0
}

# post-launch diagnostics: what did the game actually do?
diagnose() {
  sleep 20
  echo "== device state:"; adb devices || true
  echo "== package:"; adb shell pm path com.epicgames.fortnite 2>&1 | head -2 || true
  echo "== pid:"; adb shell pidof com.epicgames.fortnite 2>&1 | head -1 || true
  echo "== fatal/crash tail:"; adb logcat -d 2>/dev/null | grep -iE "epicgames|fortnite|libUE4|AndroidRuntime|FATAL|SIGSEGV|SIGILL|ndk_translation" | tail -25 || true
  adb exec-out screencap -p > /tmp/dump-phase-screen.png 2>/dev/null || true
  echo "screenshot: $(wc -c < /tmp/dump-phase-screen.png 2>/dev/null || echo 0) bytes"
}

# ============================ Strategy A =====================================
echo "== strategy A: shell dumper on the stock game"
adb install -r -t /tmp/fortnite-12.41.apk || { adb uninstall com.epicgames.fortnite >/dev/null 2>&1; adb install -t /tmp/fortnite-12.41.apk; }
launch
dismiss_gates
diagnose
# wait for libUE4.so to map (engine boot) — GObjects exists from module init
for i in $(seq 1 60); do
  adb shell su 0 true >/dev/null 2>&1 || true
  if adb shell 'grep -q libUE4.so /proc/$(pidof com.epicgames.fortnite)/maps 2>/dev/null' 2>/dev/null; then break; fi
  sleep 5
done
# pidof exits 1 when the package is not running — never let that kill the script.
PID=$(adb shell pidof com.epicgames.fortnite 2>/dev/null | tr -d '\r' || true)
echo "game pid: ${PID:-none}"
if [ -n "${PID:-}" ]; then
  adb push "$EXE" /data/local/tmp/MobileDumper-7 >/dev/null
  adb shell chmod 755 /data/local/tmp/MobileDumper-7
  adb shell 'PID=$(pidof com.epicgames.fortnite); /data/local/tmp/MobileDumper-7 --package com.epicgames.fortnite --output /data/local/tmp/dump --suspend --mem 1' | tee /tmp/md-exe-run.log || true
  sleep 5
  adb pull /data/local/tmp/dump "$OUT/CppSDK" >/dev/null 2>&1 || rm -rf "$OUT/CppSDK"
fi

ok() { test -f "$OUT/CppSDK/CppSDK/SDK.hpp" || test -f "$OUT/CppSDK/SDK.hpp"; }

if ! ok; then
  echo "::warning::strategy A produced no dump — falling back to strategy B (shim + libOwenDumper.so)"
  # ========================== Strategy B =====================================
  adb uninstall com.epicgames.fortnite >/dev/null 2>&1 || true
  cp "$LIB" /tmp/libOwenDumper.so
  CARGO_TARGET_DIR="$PWD/target" cargo build --release -p owen-patcher --bin shim >/dev/null 2>&1
  ./target/release/shim --in /tmp/fortnite-12.41.apk --out /tmp/fortnite-12.41-dump.apk \
    --shim target/shim/libUE4shim.so \
    --mod libOwenDumper.so=/tmp/libOwenDumper.so \
    --min-bytes 1000000 >/tmp/shim-report.json || { cat /tmp/shim-report.json; exit 1; }
  adb install -r -t /tmp/fortnite-12.41-dump.apk
  launch
  dismiss_gates
  echo "waiting for the in-process auto-dump (60s after load)…"
  for i in $(seq 1 40); do
    sleep 15
    adb pull "/storage/emulated/0/Android/data/com.epicgames.fortnite/files" /tmp/mdfiles >/dev/null 2>&1 || true
    if [ -d /tmp/mdfiles ]; then
      # the dump root = the app external files dir; find CppSDK anywhere in it
      FOUND=$(find /tmp/mdfiles -maxdepth 4 -name SDK.hpp 2>/dev/null | head -1 || true)
      if [ -n "$FOUND" ]; then
        ROOT=$(dirname "$(dirname "$FOUND")")   # …/CppSDK/SDK.hpp → CppSDK root
        mkdir -p "$OUT/CppSDK"
        cp -r "$ROOT"/* "$OUT/CppSDK"/
        break
      fi
    fi
  done
fi

if ! ok; then
  echo "::error::no SDK dump produced by either strategy"
  adb logcat -d > /tmp/dump-phase-logcat.txt || true
  exit 1
fi

# normalize: $OUT/CppSDK is the DUMP ROOT (contains SDK.hpp + SDK/)
echo "== dump ok:"; find "$OUT" -maxdepth 3 -name 'SDK.hpp' -o -maxdepth 3 -name '*.usmap' | head
tar -C /tmp -caf "$DUMP_TAR" og-dump
du -sh "$OUT" "$DUMP_TAR"
