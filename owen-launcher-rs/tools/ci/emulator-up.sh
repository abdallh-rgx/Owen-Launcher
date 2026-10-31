#!/usr/bin/env bash
# tools/ci/emulator-up.sh — Android x86_64 emulator with ARM translation
# (API 30 google_apis ships libndk_translation), booted headless for the
# OGS 12.41 E2E.
#
# Environment:
#   ANDROID_HOME      — SDK root (setup-android already ran)
#   OWEN_AVD          — AVD name (default owen1241)
#   OWEN_BOOT_TIMEOUT — seconds to wait for sys.boot_completed (default 900)
#
# Recipe hardened across rounds:
#   * `platforms;android-30` ships with the image install (avdmanager needs it)
#   * every sdkmanager call inherits the step's stdin (a closed stdin makes
#     cmdline-tools abort mid "Loading local repository")
#   * the image must CONTAIN system.img — a partial-download ghost dir once
#     produced an AVD that never booted and an `adb wait-for-device` hang
#   * `adb wait-for-device` is wrapped in `timeout` (it hangs forever on a
#     dead emulator) and the boot loop tails emulator.log on failure
#
# Leaves the emulator running (nohup) and `adb root` applied.
set -euo pipefail

AVD="${OWEN_AVD:-owen1241}"
TIMEOUT="${OWEN_BOOT_TIMEOUT:-900}"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
for d in "$ANDROID_HOME"/cmdline-tools/*/bin; do
  [ -d "$d" ] && export PATH="$d:$PATH"
done
SDKBIN="latest"; [ -d "$ANDROID_HOME/cmdline-tools/latest" ] || SDKBIN=$(ls "$ANDROID_HOME/cmdline-tools" | head -1)
SM="$ANDROID_HOME/cmdline-tools/$SDKBIN/bin/sdkmanager"
AVDM="$ANDROID_HOME/cmdline-tools/$SDKBIN/bin/avdmanager"
[ -x "$SM" ] || SM=sdkmanager
[ -x "$AVDM" ] || AVDM=avdmanager

echo "== kvm: $(test -e /dev/kvm && ls -l /dev/kvm || echo 'NO (TCG — will be slow)')"

# ---- system image + platform: one call, real-file validation ----------------
WANT="system-images;android-30;google_apis;x86_64"
img_dir() { echo "$ANDROID_HOME/system-images/${1#system-images;}"; }
have_image() { [ -f "$(img_dir "$1")/system.img" ] || [ -f "$(img_dir "$1")/system.img.androidbuild" ]; }

if ! have_image "$WANT"; then
  echo "== installing $WANT + platforms;android-30 (timeout 480s)"
  yes | timeout 480 "$SM" --licenses >/dev/null 2>&1 || true
  timeout 480 "$SM" --install "$WANT" "platforms;android-30" "platform-tools" >/tmp/sdk-img.log 2>&1 \
    || echo "::warning::image install incomplete — $(tail -2 /tmp/sdk-img.log | tr '\n' ' ')"
fi

IMG="$WANT"
if ! have_image "$IMG"; then
  echo "== android-30 image unavailable — falling back to any google_apis x86_64 image WITH system.img"
  for FOUND in $(ls -d "$ANDROID_HOME"/system-images/*/*/x86_64 2>/dev/null); do
    if [ -f "$FOUND/system.img" ] || [ -f "$FOUND/system.img.androidbuild" ]; then
      REL=${FOUND#"$ANDROID_HOME"/system-images/}
      IMG="system-images;${REL////;}"; echo "== using $IMG"; break
    fi
  done
  have_image "$IMG" || { echo "::error::no usable x86_64 system image"; exit 1; }
fi

yes | timeout 240 "$SM" --licenses >/dev/null 2>&1 || true

echo no | "$AVDM" --force --create avd -n "$AVD" --device pixel_5 \
  --package "$IMG" --sdcard 2048M >/dev/null 2>&1 \
  || echo no | "$AVDM" create avd -n "$AVD" --device pixel_5 --package "$IMG" >/dev/null 2>&1 \
  || echo "::warning::avdmanager create failed (existing AVD?)"

AVD_HOME="$HOME/.android/avd/$AVD.avd"
if [ -f "$AVD_HOME/config.ini" ]; then
  if grep -q '^disk.dataPartition.size' "$AVD_HOME/config.ini"; then
    sed -i 's/^disk.dataPartition.size=.*/disk.dataPartition.size=16G/' "$AVD_HOME/config.ini"
  else
    echo 'disk.dataPartition.size=16G' >> "$AVD_HOME/config.ini"
  fi
  grep -q '^hw.ramSize' "$AVD_HOME/config.ini" || echo 'hw.ramSize=4096' >> "$AVD_HOME/config.ini"
  grep -q '^hw.lcd.width' "$AVD_HOME/config.ini" || { echo 'hw.lcd.width=1280' >> "$AVD_HOME/config.ini"; echo 'hw.lcd.height=720' >> "$AVD_HOME/config.ini"; }
fi

# Kill anything stale, boot headless.
adb kill-server 2>/dev/null || true
pkill -f "qemu-system" 2>/dev/null || true
nohup emulator -avd "$AVD" -no-window -gpu swiftshader_indirect -no-snapshot \
  -no-audio -no-boot-anim -camera-back none -wipe-data >/tmp/emulator.log 2>&1 &
EMU_PID=$!

# wait-for-device without a timeout hangs forever on a dead emulator.
timeout 180 adb wait-for-device || {
  echo "::error::emulator never became adb-visible"; tail -60 /tmp/emulator.log; exit 1
}
deadline=$((SECONDS + TIMEOUT))
while [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" != "1" ]; do
  if [ $SECONDS -ge $deadline ]; then
    echo "::error::emulator did not boot within ${TIMEOUT}s"; tail -80 /tmp/emulator.log; exit 1
  fi
  kill -0 "$EMU_PID" 2>/dev/null || { echo "::error::emulator process died"; tail -80 /tmp/emulator.log; exit 1; }
  sleep 5
done
adb root >/dev/null 2>&1 || echo "::warning::adb root refused"
adb wait-for-device
sleep 10
adb shell input keyevent 82 >/dev/null 2>&1 || true   # dismiss the lockscreen
adb shell getprop ro.product.cpu.abilist || true
echo "== emulator up ($(date -u +%H:%M:%S))"
