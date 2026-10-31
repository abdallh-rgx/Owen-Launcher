#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Owen Control Room driver — runs INSIDE the GitHub Actions runner.
#
#   emulator-control.yml boots a headless Android emulator (API 30 x86_64
#   with ARM translation so the aarch64-only OwenLauncher APK runs),
#   installs it with `adb install -g` (grant all runtime permissions),
#   launches it, then this driver takes over:
#
#     1. screenshot  → POST /api/control/tasks        (PENDING on the site)
#     2. poll        → the human clicks the button on the site and submits
#     3. execute     → adb shell input tap/swipe/keyevent + sleep(waitMs)
#     4. after-shot  → PATCH /api/control/tasks/{id}  (status=APPLIED)
#     5. repeat until the human answers "done" or the session deadline.
#
# Every ~20s a heartbeat keeps the site informed (state, phase, focus
# window, driver log + logcat tail). No fake screenshots ever: every image
# posted here is a live `adb exec-out screencap` from the emulator.
#
# Required env:
#   CONTROL_URL     e.g. https://<preview-domain>   (no trailing slash)
#   CONTROL_TOKEN   shared secret (default matches the site default)
# Optional env:
#   APK_PATH, OWEN_PKG, DRIVER_MAX_MINUTES (default 325)
# ---------------------------------------------------------------------------
set -uo pipefail

CONTROL_URL="${CONTROL_URL:?CONTROL_URL is not set}"
CONTROL_TOKEN="${CONTROL_TOKEN:-owen-ctl-7f4k9qz2m8}"
APK_PATH="${APK_PATH:-owen-launcher-rs/dist/OwenLauncher-v1.8.0-release.apk}"
PKG="${OWEN_PKG:-com.abdallhrgx.owenlauncher}"
MAX_MIN="${DRIVER_MAX_MINUTES:-325}"

BASE="${CONTROL_URL%/}"
DEADLINE=$(( SECONDS + MAX_MIN * 60 ))
SHOTS_ARCHIVE=/tmp/owen-shots
mkdir -p "$SHOTS_ARCHIVE"
: > /tmp/owen-driver.log

[ -n "${ANDROID_HOME:-}" ] && export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
ADB="$(command -v adb || true)"; ADB="${ADB:-adb}"

STEP=0
TASK_ID=""
STATE="BOOT"
PHASE="starting driver"
MESSAGE="بدء خط القيادة…"
DEVICE="emulator"

log() { printf '%s %s\n' "$(date -u +%H:%M:%S)" "$*" >> /tmp/owen-driver.log; }

die() { log "FATAL: $*"; STATE="ERROR"; PHASE="توقف"; MESSAGE="$*";
  hb_now; exit 1; }

# --- heartbeat ---------------------------------------------------------------
focus()  { "$ADB" shell dumpsys window 2>/dev/null | tr -d '\r' \
             | grep -m1 -E "mCurrentFocus|mFocusedWindow" | head -c 220; }
focus_app() { focus | grep -oE '[a-zA-Z0-9._]+/[a-zA-Z0-9._]+' | head -1; }
uptime_str() { printf '%dm%02ds' $((SECONDS/60)) $((SECONDS%60)); }

hb_now() {
  local logtail
  logtail="$( { tail -n 30 /tmp/owen-driver.log; echo '--- logcat ---';
                "$ADB" logcat -d -t 100 2>/dev/null | tail -n 70; } | tail -n 120 )"
  jq -n --arg s "$STATE" --arg p "$PHASE" --arg d "$DEVICE" \
        --arg m "$MESSAGE" --arg l "$logtail" \
    '{state:$s, phase:$p, device:$d, message:$m, logTail:$l}' \
  | curl -sf -m 20 -X POST "$BASE/api/control/heartbeat" \
      -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
      --data-binary @- >/dev/null || true
}

hb_loop() { while :; do hb_now; sleep 20; done; }
hb_loop & HB_PID=$!
trap 'kill $HB_PID 2>/dev/null; true' EXIT

# --- adb helpers --------------------------------------------------------------
take_shot() {  # writes /tmp/owen-shot.png
  "$ADB" exec-out screencap -p > /tmp/owen-shot.png 2>/dev/null
  [ -s /tmp/owen-shot.png ]
}

wait_boot() {
  local i boot
  for i in $(seq 1 240); do
    boot="$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')"
    [ "$boot" = "1" ] && return 0
    sleep 5
    [ $SECONDS -ge $DEADLINE ] && return 1
  done
  return 1
}

post_task() {  # $1=title  $2=instruction  → sets TASK_ID
  local b64 meta resp title="$1" instr="$2"
  b64="$(base64 -w0 /tmp/owen-shot.png)"
  meta="$(jq -nc --arg app "$(focus_app)" --arg f "$(focus)" \
                     --arg u "$(uptime_str)" '{app:$app,focus:$f,uptime:$u}')"
  resp="$(jq -n --arg sk "step-$STEP" --arg t "$title" --arg i "$instr" \
                --arg img "$b64" --argjson m "$meta" \
          '{stepKey:$sk,title:$t,instruction:$i,image:$img,meta:$m}' \
    | curl -sS -m 120 -X POST "$BASE/api/control/tasks" \
        -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
        --data-binary @-)" || resp=""
  TASK_ID="$(jq -r '.id // empty' <<<"$resp" 2>/dev/null)"
  [ -n "$TASK_ID" ]
}

patch_task() {  # extra jq args, e.g.: patch_task --arg img "$b64" '{image:$img}'
  [ -n "$TASK_ID" ] || return 1
  local body
  body="$(jq -nc "$@")"
  curl -sf -m 120 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
    -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
    --data-binary "$body" >/dev/null || true
}

patch_fresh_image() {
  local b64
  b64="$(base64 -w0 /tmp/owen-shot.png)"
  patch_task --arg img "$b64" '{image:$img}'
}

patch_applied() {
  local b64
  if [ -s /tmp/owen-after.png ]; then
    b64="$(base64 -w0 /tmp/owen-after.png)"
    patch_task --arg img "$b64" '{status:"APPLIED",afterImage:$img}'
  else
    patch_task '{status:"APPLIED"}'
  fi
}

# --- boot ---------------------------------------------------------------------
STATE="WAIT_BOOT"; PHASE="بانتظار إقلاع المحاكي"; MESSAGE="المحاكي يقلع…"
log "waiting for emulator boot (adb=$ADB)"
[ -x "$(command -v "$ADB" 2>/dev/null)" ] || die "adb not found on PATH"
"$ADB" wait-for-device || die "adb wait-for-device failed"
wait_boot || die "emulator did not finish booting"

DEVICE="Android $("$ADB" shell getprop ro.build.version.release 2>/dev/null | tr -d '\r') • $("$ADB" shell getprop ro.product.model 2>/dev/null | tr -d '\r')"
log "booted: $DEVICE"

# Wake the screen, kill animations, keep it awake, skip the setup wizard.
"$ADB" shell input keyevent 224 || true
"$ADB" shell wm dismiss-keyguard || true
"$ADB" shell settings put global window_animation_scale 0.0 || true
"$ADB" shell settings put global transition_animation_scale 0.0 || true
"$ADB" shell settings put global animator_duration_scale 0.0 || true
"$ADB" shell settings put global screen_off_timeout 2147483647 || true
"$ADB" shell settings put global device_provisioned 1 || true
"$ADB" shell settings put secure user_setup_complete 1 || true
"$ADB" shell am force-stop com.google.android.setupwizard 2>/dev/null || true
"$ADB" shell input keyevent 82 || true
sleep 2

# --- install (adb install -g = grant all runtime permissions) -----------------
STATE="INSTALL"; PHASE="تثبيت OwenLauncher"; MESSAGE="adb install -g …"
log "installing $APK_PATH (grant all permissions)"
[ -f "$APK_PATH" ] || die "APK not found at $APK_PATH"
ok=""
for i in 1 2 3; do
  if "$ADB" install -g -r "$APK_PATH" >> /tmp/owen-driver.log 2>&1; then ok=1; break; fi
  log "install attempt $i failed, retrying…"; sleep 5
done
[ -n "$ok" ] || die "adb install -g failed three times"
"$ADB" shell pm list packages 2>/dev/null | grep -q "$PKG" || die "$PKG not installed after adb install -g"
log "installed + granted: $PKG"

# --- launch -------------------------------------------------------------------
STATE="LAUNCH"; PHASE="فتح OwenLauncher"; MESSAGE="monkey launch…"
"$ADB" shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 >> /tmp/owen-driver.log 2>&1 || true
sleep 10

# --- main loop ----------------------------------------------------------------
while [ $SECONDS -lt $DEADLINE ]; do
  STATE="ASK"
  STEP=$((STEP+1))
  PHASE="لقطة الشاشة للخطوة $STEP"
  if ! take_shot; then log "screencap failed, retry in 3s"; sleep 3; continue; fi
  cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-$(date +%H%M%S).png"

  local_title="خطوة $STEP — حدّد الزر التالي"
  local_instr="انقر على مكان الزر المطلوب في الصورة، ثم اكتب وظيفته وحدّد مدة الانتظار بالأسفل واضغط «إرسال». إذا كانت الشاشة مجرّد تحميل، اختر «انتظار فقط» مع المدة المناسبة."
  if [ "$STEP" -eq 1 ]; then
    local_title="خطوة 1 — أول لقطة بعد فتح التطبيق"
    local_instr="هذه أول لقطة حقيقية من المحاكي بعد تشغيل OwenLauncher. انقر على الزر التالي المطلوب ضغطه (مثلاً زر التحميل/التثبيت)، اكتب وظيفته، وحدّد مدة الانتظار."
  fi

  if ! post_task "$local_title" "$local_instr"; then
    log "post_task failed for step $STEP; retry in 6s"; sleep 6; continue
  fi
  log "step $STEP posted as task $TASK_ID — waiting for the human"
  PHASE="بانتظار إجابتك على الخطوة $STEP"
  MESSAGE="الخطوة $STEP معروضة على الموقع — بانتظار تحديدك للزر"

  ACTION=""; X=""; Y=""; X2=""; Y2=""; WAITMS="0"; DESC=""
  last_refresh=$SECONDS
  shot_hash="$(md5sum /tmp/owen-shot.png | cut -d' ' -f1)"

  answered=0
  while [ $SECONDS -lt $DEADLINE ]; do
    resp="$(curl -sS -m 20 "$BASE/api/control/tasks/$TASK_ID" \
              -H "x-owen-token: $CONTROL_TOKEN" 2>/dev/null || true)"
    status="$(jq -r '.task.status // "ERR"' <<<"$resp" 2>/dev/null)"

    if [ "$status" = "ANSWERED" ] || [ "$status" = "APPLIED" ] || [ "$status" = "CANCELLED" ]; then
      if [ "$status" = "ANSWERED" ]; then
        ACTION="$(jq -r '.task.action // "tap"' <<<"$resp")"
        X="$(jq   -r '.task.x  // 0 | floor'   <<<"$resp")"
        Y="$(jq   -r '.task.y  // 0 | floor'   <<<"$resp")"
        X2="$(jq  -r '.task.x2 // 0 | floor'   <<<"$resp")"
        Y2="$(jq  -r '.task.y2 // 0 | floor'   <<<"$resp")"
        WAITMS="$(jq -r '.task.waitMs // 0 | floor' <<<"$resp")"
        DESC="$(jq  -r '.task.description // ""' <<<"$resp")"
        answered=1
      fi
      break
    fi

    # The human asked for a fresh frame (retake button).
    wants="$(jq -r '.task.wantsRetake // false' <<<"$resp" 2>/dev/null)"
    if [ "$wants" = "true" ]; then
      log "human requested retake"
      if take_shot; then
        cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-retake-$(date +%H%M%S).png"
        patch_fresh_image
        shot_hash="$(md5sum /tmp/owen-shot.png | cut -d' ' -f1)"
      fi
      last_refresh=$SECONDS
    fi

    # Auto-refresh the pending screenshot when the screen changed on its own
    # (loading screens, animations) so the human never annotates a stale frame.
    if [ $((SECONDS - last_refresh)) -ge 45 ]; then
      if take_shot; then
        h="$(md5sum /tmp/owen-shot.png | cut -d' ' -f1)"
        if [ "$h" != "$shot_hash" ]; then
          log "screen changed while pending — auto-refreshing the image"
          cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-auto-$(date +%H%M%S).png"
          patch_fresh_image
          shot_hash="$h"
        fi
      fi
      last_refresh=$SECONDS
    fi

    sleep 2.5
  done

  [ "$answered" = "1" ] || break   # deadline hit with no answer
  log "step $STEP answer: action=$ACTION x=$X y=$Y wait=${WAITMS}ms desc=$DESC"

  if [ "$ACTION" = "done" ]; then
    PHASE="إنهاء بطلبك"; MESSAGE="أنهيت الجلسة — شكراً!"
    log "human requested session end"
    break
  fi

  if [ "$ACTION" = "skip" ]; then
    PHASE="تخطي — لقطة جديدة"
    patch_task '{status:"CANCELLED"}'
    continue
  fi

  STATE="APPLY"; PHASE="تنفيذ: $ACTION ($X,$Y) ثم انتظار ${WAITMS}ms"
  case "$ACTION" in
    tap)   "$ADB" shell input tap "$X" "$Y" >> /tmp/owen-driver.log 2>&1 || true ;;
    swipe) "$ADB" shell input swipe "$X" "$Y" "$X2" "$Y2" 600 >> /tmp/owen-driver.log 2>&1 || true ;;
    back)  "$ADB" shell input keyevent 4  >> /tmp/owen-driver.log 2>&1 || true ;;
    home)  "$ADB" shell input keyevent 3  >> /tmp/owen-driver.log 2>&1 || true ;;
    wait)  : ;;
    *)     log "unknown action '$ACTION' — treating as wait" ;;
  esac

  w="${WAITMS:-0}"
  case "$w" in ''|*[!0-9]*) w=0;; esac
  [ "$w" -gt 600000 ] && w=600000
  if [ "$w" -gt 0 ]; then sleep "$(( (w + 999) / 1000 ))"; fi

  sleep 2
  if take_shot; then
    cp /tmp/owen-shot.png /tmp/owen-after.png
    cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-after-$(date +%H%M%S).png"
  else
    : > /tmp/owen-after.png
  fi
  patch_applied
  : > /tmp/owen-after.png
done

STATE="DONE"; PHASE="انتهت الجلسة"
MESSAGE="انتهت جلسة القيادة ($STEP خطوة). يمكنك إعادة تشغيل الـ Action في أي وقت."
log "session finished after $STEP steps"
hb_now
exit 0
