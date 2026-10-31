#!/usr/bin/env bash
# tools/ci/emulator-up.sh — Android x86_64 emulator with ARM translation
# (API 30 google_apis ships libndk), booted headless for the OGS 12.41 E2E.
#
# Environment:
#   ANDROID_HOME  — SDK root (setup-android already ran)
#   OWEN_AVD      — AVD name (default owen1241)
#   OWEN_BOOT_TIMEOUT — seconds to wait for sys.boot_completed (default 900;
#                   TCG without KVM needs the long end)
#
# Leaves the emulator running (nohup) and `adb root` applied.
set -euo pipefail

AVD="${OWEN_AVD:-owen1241}"
TIMEOUT="${OWEN_BOOT_TIMEOUT:-900}"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
# cmdline-tools live off PATH by default; put every flavor on it.
for d in "$ANDROID_HOME"/cmdline-tools/*/bin; do
  [ -d "$d" ] && export PATH="$d:$PATH"
done

echo "== kvm: $(test -e /dev/kvm && echo yes || echo 'NO (TCG — will be slow)')"
test -e /dev/kvm || echo "::warning::no /dev/kvm on this runner — emulator runs under TCG"

# ---- system image: prefer what's already on the runner, else install with a
# hard timeout (sdkmanager hung 67 minutes on "Loading local repository" once —
# never again: every invocation gets `timeout` and </dev/null).
want="system-images;android-30;google_apis;x86_64"
have_image() { [ -d "$ANDROID_HOME/system-images/${1#system-images;}" ]; }
if ! have_image "$want"; then
  echo "== installing $want (timeout 480s)"
  timeout 480 sdkmanager --install "$want" </dev/null >/tmp/sdk-img.log 2>&1 \
    || echo "::warning::image install incomplete — $(tail -2 /tmp/sdk-img.log | tr '\n' ' ')"
fi
IMG="$want"
if ! have_image "$IMG"; then
  echo "== android-30 image unavailable — falling back to any preinstalled google_apis x86_64 image"
  FOUND=$(ls -d "$ANDROID_HOME"/system-images/*/google_apis/x86_64 "$ANDROID_HOME"/system-images/*/*google*x86_64 2>/dev/null | head -1 || true)
  if [ -n "$FOUND" ]; then
    REL=${FOUND#"$ANDROID_HOME"/system-images/}
    IMG="system-images;${REL////;}"
    echo "== using $IMG"
  else
    echo "::error::no x86_64 system image available and none could be installed"; exit 1
  fi
fi
yes | timeout 240 sdkmanager --licenses </dev/null >/dev/null 2>&1 || true

echo no | avdmanager create avd -n "$AVD" -k "$IMG" --device pixel 2>/dev/null \
  || echo "::warning::avdmanager create failed (existing AVD?)"

AVD_HOME="$HOME/.android/avd/$AVD.avd"
test -d "$AVD_HOME" && {
  cat >> "$AVD_HOME/config.ini" <<'EOF'
hw.ramSize=4096
hw.lcd.width=1280
hw.lcd.height=720
hw.lcd.density=320
disk.dataPartition.size=16G
hw.gpu.mode=off
vm.heapSize=512
EOF
}

# Kill anything stale, boot headless.
adb kill-server 2>/dev/null || true
pkill -f "qemu-system" 2>/dev/null || true
nohup emulator -avd "$AVD" -no-window -gpu swiftshader_indirect -no-snapshot \
  -no-audio -no-boot-anim -wipe-data >/tmp/emulator.log 2>&1 &

adb wait-for-device
deadline=$((SECONDS + TIMEOUT))
while [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" != "1" ]; do
  if [ $SECONDS -ge $deadline ]; then
    echo "::error::emulator did not boot within ${TIMEOUT}s"; tail -40 /tmp/emulator.log; exit 1
  fi
  sleep 5
done
adb root >/dev/null 2>&1 || echo "::warning::adb root refused"
adb wait-for-device
sleep 10
# give the ARM translator a moment after boot
adb shell getprop ro.product.cpu.abilist || true
echo "== emulator up ($(date -u +%H:%M:%S))"
