#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Owen Control Room driver v2 — runs INSIDE the GitHub Actions runner.
#
# emulator-control.yml boots a headless Android emulator (API 30 x86_64 with
# ARM translation so the aarch64-only OwenLauncher APK runs), installs it
# with `adb install -g` (grant all runtime permissions), launches it, then
# this driver takes over. NEW in v2:
#
#   • LIVE STREAM   — pushes a downscaled JPEG of the phone screen every
#                    ~1.2s to /api/control/live/frame (the site shows it
#                    as a live broadcast; clicking it annotates tasks).
#   • LIVE LOG      — tails `adb logcat` + this driver's own log to
#                    /api/control/live/log so crashes are visible instantly.
#   • CRASH WATCH   — tails `logcat -b crash`; new crash → CRASH lines on
#                    the site, heartbeat alert + a crash report artifact.
#   • SIGNALS       — polls /api/control/signals/pending:
#       download_done → notes it and moves on (next task is post-download)
#       start_sdk     → runs the SDK dump routine (MobileDumper)
#       restart_app   → force-stop + relaunch OwenLauncher
#       crash_report  → collects a full crash report right now
#       note          → free text from the operator
#   • ASK LOOP (v1) — screenshot → PENDING task on the site → the human
#                    clicks the button → we tap + sleep(wait) → repeat.
#
# Required env:
#   CONTROL_URL     e.g. https://<preview-domain>   (no trailing slash)
#   CONTROL_TOKEN   shared secret (default matches the site default)
# Optional env:
#   APK_PATH, OWEN_PKG, DRIVER_MAX_MINUTES (default 325)
#   DUMP_APK  (path to a MobileDumper-7 APK inside the repo, for start_sdk)
#   DUMP_PKG  (package name of the dump APK, default = OWEN_PKG)
#   FRAME_IVL (seconds between live frames, default 1.2)
# ---------------------------------------------------------------------------
set -uo pipefail

CONTROL_URL="${CONTROL_URL:?CONTROL_URL is not set}"
CONTROL_TOKEN="${CONTROL_TOKEN:-owen-ctl-7f4k9qz2m8}"
APK_PATH="${APK_PATH:-owen-launcher-rs/dist/OwenLauncher-v1.8.0-release.apk}"
PKG="${OWEN_PKG:-com.abdallhrgx.owenlauncher}"
DUMP_APK="${DUMP_APK:-}"
DUMP_PKG="${DUMP_PKG:-$PKG}"
FRAME_IVL="${FRAME_IVL:-1.2}"
MAX_MIN="${DRIVER_MAX_MINUTES:-325}"

BASE="${CONTROL_URL%/}"
DEADLINE=$(( SECONDS + MAX_MIN * 60 ))
SHOTS_ARCHIVE=/tmp/owen-shots
SDK_DUMP_DIR=/tmp/owen-sdk-dump
LOGCAT_FILE=/tmp/owen-logcat.txt
CRASH_FILE=/tmp/owen-crash.txt
mkdir -p "$SHOTS_ARCHIVE" "$SDK_DUMP_DIR"
: > /tmp/owen-driver.log
: > "$LOGCAT_FILE"
: > "$CRASH_FILE"
: > /tmp/owen-crash-msg

[ -n "${ANDROID_HOME:-}" ] && export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
ADB="$(command -v adb || true)"; ADB="${ADB:-adb}"

STEP=0
TASK_ID=""
STATE="BOOT"
PHASE="starting driver"
MESSAGE="بدء خط القيادة…"
DEVICE="emulator"
DEV_W=1080
DEV_H=2280

log() { printf '%s %s\n' "$(date -u +%H:%M:%S)" "$*" >> /tmp/owen-driver.log; }

die() { log "FATAL: $*"; STATE="ERROR"; PHASE="توقف"; MESSAGE="$*";
  hb_now; exit 1; }

# --- http helpers -------------------------------------------------------------
auth_headers=(-H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json')
post_json() { # $1=url ; body = $2 أو عبر stdin (أنبوب)
  local url="$1"
  if [ -n "${2-}" ]; then
    curl -sS -m 25 -X POST "$url" "${auth_headers[@]}" --data-binary "$2" 2>/dev/null || true
  else
    curl -sS -m 25 -X POST "$url" "${auth_headers[@]}" --data-binary @- 2>/dev/null || true
  fi
}

# --- heartbeat -----------------------------------------------------------------
focus()  { "$ADB" shell dumpsys window 2>/dev/null | tr -d '\r' \
             | grep -m1 -E "mCurrentFocus|mFocusedWindow" | head -c 220; }
focus_app() { focus | grep -oE '[a-zA-Z0-9._]+/[a-zA-Z0-9._]+' | head -1; }
uptime_str() { printf '%dm%02ds' $((SECONDS/60)) $((SECONDS%60)); }

hb_now() {
  local logtail crashmsg
  logtail="$( { tail -n 30 /tmp/owen-driver.log; echo '--- logcat ---';
                tail -n 60 "$LOGCAT_FILE"; } | tail -n 110 )"
  crashmsg="$(cat /tmp/owen-crash-msg 2>/dev/null | head -c 300)"
  [ -n "$crashmsg" ] && MESSAGE="🚨 $crashmsg"
  jq -n --arg s "$STATE" --arg p "$PHASE" --arg d "$DEVICE" \
        --arg m "$MESSAGE" --arg l "$logtail" \
    '{state:$s, phase:$p, device:$d, message:$m, logTail:$l}' \
  | post_json "$BASE/api/control/heartbeat" >/dev/null
  [ -n "$crashmsg" ] || true
}

hb_loop() { while [ $SECONDS -lt $DEADLINE ]; do hb_now; sleep 20; done; }
hb_loop & HB_PID=$!
trap 'kill $HB_PID 2>/dev/null; true' EXIT

# --- screenshots ----------------------------------------------------------------
take_shot() {  # writes /tmp/owen-shot.png
  "$ADB" exec-out screencap -p > /tmp/owen-shot.png 2>/dev/null
  [ -s /tmp/owen-shot.png ]
}

# Downscale to a small JPEG for the live stream (PIL when available,
# otherwise ship the PNG as-is). Prints: ORIG_W ORIG_H LIVE_W LIVE_H
make_live_jpeg() {
  python3 - <<'PY' 2>/dev/null || { cp /tmp/owen-shot.png /tmp/owen-live.jpg; echo "1080 2280 1080 2280"; }
import struct
src = '/tmp/owen-shot.png'
dst = '/tmp/owen-live.jpg'
data = open(src, 'rb').read()
w, h = struct.unpack('>II', data[16:24])
try:
    from PIL import Image
    im = Image.open(src)
    nw = 420
    nh = max(1, int(h * nw / w))
    im = im.resize((nw, nh))
    im.convert('RGB').save(dst, 'JPEG', quality=62)
    print(f"{w} {h} {nw} {nh}")
except Exception:
    import shutil
    shutil.copyfile(src, dst)
    print(f"{w} {h} {w} {h}")
PY
}

# --- LIVE STREAM loop -------------------------------------------------------------
frame_loop() {
  local dims ow lh lw lh2 b64 app
  while [ $SECONDS -lt $DEADLINE ]; do
    if take_shot; then
      dims="$(make_live_jpeg)"
      ow="$(awk '{print $1}' <<<"$dims")"; lh="$(awk '{print $2}' <<<"$dims")"
      lw="$(awk '{print $3}' <<<"$dims")"; lh2="$(awk '{print $4}' <<<"$dims")"
      b64="$(base64 -w0 /tmp/owen-live.jpg 2>/dev/null || true)"
      if [ -n "$b64" ]; then
        app="$(focus_app)"
        jq -n --arg img "$b64" --argjson w "${lw:-420}" --argjson h "${lh2:-880}" \
              --argjson dw "$DEV_W" --argjson dh "$DEV_H" --arg a "${app:-}" \
          '{image:$img, w:$w, h:$h, deviceW:$dw, deviceH:$dh, app:$a}' \
        | post_json "$BASE/api/control/live/frame" >/dev/null
      fi
    fi
    sleep "$FRAME_IVL"
  done
}

# --- LIVE LOG loop -----------------------------------------------------------------
log_level_of() { # classify a logcat -v time line
  case "$1" in
    *" F "*|*" F:"*|*"FATAL"*) echo CRASH;;
    *" E "*|*" E:"*|*" E/"*) echo ERROR;;
    *" W "*|*" W:"*|*" W/"*) echo WARN;;
    *) echo INFO;;
  esac
}

post_log_batch() { # $1 = json array of {level,src,text}
  [ "$1" = "[]" ] && return 0
  jq -n --argjson lines "$1" '{lines:$lines}' \
    | post_json "$BASE/api/control/live/log" >/dev/null
}

log_loop() {
  local drv_off=0 lc_off=0 batch line lvl
  while [ $SECONDS -lt $DEADLINE ]; do
    batch="[]"
    # driver's own log
    if [ -s /tmp/owen-driver.log ]; then
      sz=$(stat -c%s /tmp/owen-driver.log 2>/dev/null || echo 0)
      if [ "$sz" -gt "$drv_off" ]; then
        while IFS= read -r line; do
          [ -n "$line" ] || continue
          batch="$(jq -c --arg t "$line" '. + [{level:"DRIVER",src:"driver",text:$t}]' <<<"$batch")"
        done < <(tail -c +$((drv_off + 1)) /tmp/owen-driver.log | head -c 20000 | head -40)
        drv_off=$sz
      fi
    fi
    # logcat
    if [ -s "$LOGCAT_FILE" ]; then
      sz=$(stat -c%s "$LOGCAT_FILE" 2>/dev/null || echo 0)
      if [ "$sz" -gt "$lc_off" ]; then
        while IFS= read -r line; do
          [ -n "$line" ] || continue
          lvl="$(log_level_of "$line")"
          [ "$lvl" = "INFO" ] && continue          # only W/E/F travel — keep the stream light
          batch="$(jq -c --arg l "$lvl" --arg t "$line" '. + [{level:$l,src:"logcat",text:$t}]' <<<"$batch")"
        done < <(tail -c +$((lc_off + 1)) "$LOGCAT_FILE" | head -c 40000 | head -60)
        lc_off=$sz
      fi
    fi
    # crash buffer
    if [ -s "$CRASH_FILE" ]; then
      sz=$(stat -c%s "$CRASH_FILE" 2>/dev/null || echo 0)
      if [ "$sz" -gt 0 ] && [ "$sz" -gt "${CR_OFF:-0}" ]; then
        while IFS= read -r line; do
          [ -n "$line" ] || continue
          batch="$(jq -c --arg t "$line" '. + [{level:"CRASH",src:"crash",text:$t}]' <<<"$batch")"
        done < <(tail -c +$((${CR_OFF:-0} + 1)) "$CRASH_FILE" | head -c 30000 | head -80)
        CR_OFF=$sz
      fi
    fi
    post_log_batch "$batch"
    sleep 2
  done
}

# --- CRASH watcher -------------------------------------------------------------------
crash_loop() {
  local sz ts
  while [ $SECONDS -lt $DEADLINE ]; do
    if [ -s "$CRASH_FILE" ]; then
      sz=$(stat -c%s "$CRASH_FILE" 2>/dev/null || echo 0)
      if [ "$sz" -gt 0 ]; then
        ts=$(date +%H%M%S)
        cp "$CRASH_FILE" "$SHOTS_ARCHIVE/crash-$(date +%Y%m%d)-$ts.txt"
        head -c 400 "$CRASH_FILE" > /tmp/owen-crash-msg
        log "CRASH detected — report saved to owen-shots/crash-*$ts.txt"
        MESSAGE="🚨 كراش! $(head -c 120 "$CRASH_FILE" | tr '\n' ' ')"
        hb_now
        : > "$CRASH_FILE"; : > /tmp/owen-crash-msg
      fi
    fi
    sleep 3
  done
}

# --- SIGNALS -------------------------------------------------------------------------
signal_loop() {
  local resp id type note result n
  while [ $SECONDS -lt $DEADLINE ]; do
    resp="$(curl -sS -m 15 "$BASE/api/control/signals/pending" "${auth_headers[@]}" 2>/dev/null || true)"
    n="$(jq -r '.signals | length' <<<"$resp" 2>/dev/null || echo 0)"
    if [ "${n:-0}" -gt 0 ] 2>/dev/null; then
      while IFS=$'\t' read -r id type note; do
        [ -n "$id" ] || continue
        result="تم الاستلام"
        case "$type" in
          download_done)
            echo 1 > /tmp/owen-flag-download-done
            result="سُجّل انتهاء التحميل الداخلي — أكمل لتحديد الزر التالي"
            log "SIGNAL download_done: $note"
            ;;
          start_sdk)
            echo 1 > /tmp/owen-flag-sdk
            result="سأبدأ توليد SDK بعد لحظات"
            log "SIGNAL start_sdk: $note"
            ;;
          restart_app)
            "$ADB" shell am force-stop "$PKG" 2>/dev/null
            sleep 1
            "$ADB" shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
            result="أعدت تشغيل $PKG"
            log "SIGNAL restart_app — relaunched $PKG"
            ;;
          crash_report)
            "$ADB" logcat -d -b crash -v time > "$SHOTS_ARCHIVE/crash-report-$(date +%H%M%S).txt" 2>/dev/null
            "$ADB" logcat -d -t 300 > "$SHOTS_ARCHIVE/logcat-around-crash-$(date +%H%M%S).txt" 2>/dev/null
            result="جمعت تقرير الكراش الكامل — راجع الـ artifacts"
            log "SIGNAL crash_report — full report collected"
            ;;
          note)
            result="وصلت الملاحظة: $note"
            log "SIGNAL note: $note"
            ;;
        esac
        post_json "$BASE/api/control/signals/$id/ack" \
          "$(jq -n --arg r "$result" '{result:$r}')" >/dev/null
      done < <(jq -r '.signals[] | [.id, .type, (.note // "")] | @tsv' <<<"$resp" 2>/dev/null)
    fi
    sleep 5
  done
}

# --- SDK DUMP routine (MobileDumper-7) ---------------------------------------------------
run_sdk_dump() {
  STATE="SDK_DUMP"; PHASE="توليد SDK (MobileDumper)"
  MESSAGE="بدء روتين توليد SDK…"
  log "SDK dump routine started (dump_apk=${DUMP_APK:-none})"
  hb_now
  if [ -z "$DUMP_APK" ] || [ ! -f "$DUMP_APK" ]; then
    MESSAGE="لا يوجد DUMP_APK في المستودع — شغّل الـ dump يدوياً أو أضف مسار MobileDumper عبر مدخل dump_apk"
    PHASE="تعذّر بدء توليد SDK"
    log "DUMP_APK missing — cannot automate SDK generation"
    hb_now
    return 0
  fi
  log "installing dump apk: $DUMP_APK"
  "$ADB" install -g -r "$DUMP_APK" >> /tmp/owen-driver.log 2>&1 || log "dump apk install failed (continuing)"
  "$ADB" shell am force-stop "$DUMP_PKG" 2>/dev/null
  sleep 1
  "$ADB" shell monkey -p "$DUMP_PKG" -c android.intent.category.LAUNCHER 1 >> /tmp/owen-driver.log 2>&1 || true
  log "dump package launched: $DUMP_PKG — polling /sdcard for dump files"

  local dirs='"/sdcard/Android/data/'"$DUMP_PKG"'/files/Dumps" "/sdcard/Dumps" "/sdcard/Android/data/'"$DUMP_PKG"'/files"'
  local i=0 last=-1 stable=0 files
  while [ $i -lt 100 ]; do   # up to ~50 minutes (100 × 30s)
    files="$("$ADB" shell find $dirs -type f 2>/dev/null | tr -d '\r' | wc -l)"
    log "SDK dump poll #$i: $files files on device"
    if [ "${files:-0}" -gt 0 ]; then
      if [ "$files" = "$last" ]; then
        stable=$((stable + 1))
        [ "$stable" -ge 4 ] && break   # stable for ~2 minutes → done
      else
        stable=0
      fi
    fi
    last="${files:-0}"
    sleep 30
  done

  log "pulling dump files…"
  for d in "/sdcard/Android/data/$DUMP_PKG/files/Dumps" "/sdcard/Dumps" "/sdcard/Android/data/$DUMP_PKG/files"; do
    "$ADB" pull "$d" "$SDK_DUMP_DIR/" >> /tmp/owen-driver.log 2>&1 || true
  done
  if [ -n "$(find "$SDK_DUMP_DIR" -type f 2>/dev/null | head -1)" ]; then
    (cd /tmp && zip -qr "$SHOTS_ARCHIVE/sdk-dump-$(date +%H%M%S).zip" owen-sdk-dump) || true
    MESSAGE="تم توليد SDK وسحب الملفات — راجع owen-control-session artifacts"
    PHASE="اكتمل توليد SDK ✅"
    log "SDK dump pulled + zipped into artifacts"
  else
    MESSAGE="انتهى انتظار ملفات الـ SDK لكن لم يُعثر على ملفات — راجع السجل"
    PHASE="لم تُوجد ملفات dump"
    log "no dump files found on device"
  fi
  hb_now
}

# --- boot -----------------------------------------------------------------------
STATE="WAIT_BOOT"; PHASE="بانتظار إقلاع المحاكي"; MESSAGE="المحاكي يقلع…"
log "waiting for emulator boot (adb=$ADB)"
[ -x "$(command -v "$ADB" 2>/dev/null)" ] || die "adb not found on PATH"
"$ADB" wait-for-device || die "adb wait-for-device failed"
for i in $(seq 1 240); do
  boot="$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')"
  [ "$boot" = "1" ] && break
  sleep 5
  [ $SECONDS -ge $DEADLINE ] && die "emulator did not finish booting"
done
[ "$boot" = "1" ] || die "emulator did not finish booting"

DEVICE="Android $("$ADB" shell getprop ro.build.version.release 2>/dev/null | tr -d '\r') • $("$ADB" shell getprop ro.product.model 2>/dev/null | tr -d '\r')"
# physical resolution (for live-stream coordinate mapping)
wm_line="$("$ADB" shell wm size 2>/dev/null | tr -d '\r' | grep -m1 'Physical size:' || true)"
if [ -n "$wm_line" ]; then
  DEV_W="$(awk '{print $3}' <<<"${wm_line%:*}" | cut -dx -f1)"; DEV_H="$(awk '{print $3}' <<<"${wm_line%:*}" | cut -dx -f2)"
  [ -n "$DEV_W" ] && [ -n "$DEV_H" ] || { DEV_W=1080; DEV_H=2280; }
fi
log "booted: $DEVICE (${DEV_W}x${DEV_H})"

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

# start the background streamers (after boot so logcat starts clean)
nohup "$ADB" logcat -v time -T 1 >> "$LOGCAT_FILE" 2>/dev/null &
nohup "$ADB" logcat -b crash -v time -T 1 >> "$CRASH_FILE" 2>/dev/null &
frame_loop   & FRAME_PID=$!
log_loop     & LOG_PID=$!
crash_loop   & CRASH_PID=$!
signal_loop  & SIG_PID=$!
trap 'kill $HB_PID $FRAME_PID $LOG_PID $CRASH_PID $SIG_PID 2>/dev/null; true' EXIT
log "background loops started: frame(1/${FRAME_IVL}s) log(2s) crash(3s) signals(5s)"

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
  # operator asked for SDK generation → run the dump routine
  if [ -f /tmp/owen-flag-sdk ]; then
    rm -f /tmp/owen-flag-sdk
    run_sdk_dump
  fi

  STATE="ASK"
  STEP=$((STEP+1))
  PHASE="لقطة الشاشة للخطوة $STEP"
  if ! take_shot; then log "screencap failed, retry in 3s"; sleep 3; continue; fi
  cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-$(date +%H%M%S).png"

  local_title="خطوة $STEP — حدّد الزر التالي"
  local_instr="انقر على مكان الزر المطلوب (على لقطة الطلب هنا أو على البث المباشر)، ثم اكتب وظيفته وحدّد مدة الانتظار واضغط «إرسال». إذا كانت الشاشة مجرّد تحميل، اختر «انتظار فقط» مع المدة المناسبة."
  if [ "$STEP" -eq 1 ]; then
    local_title="خطوة 1 — أول لقطة بعد فتح التطبيق"
    local_instr="هذه أول لقطة حقيقية من المحاكي بعد تشغيل OwenLauncher. انقر على الزر التالي المطلوب ضغطه (مثلاً زر التحميل/التثبيت)، اكتب وظيفته، وحدّد مدة الانتظار."
  fi
  if [ -f /tmp/owen-flag-download-done ]; then
    local_title="خطوة $STEP (بعد انتهاء التحميل ✅) — حدّد الزر التالي"
    rm -f /tmp/owen-flag-download-done
  fi

  b64png="$(base64 -w0 /tmp/owen-shot.png)"
  meta="$(jq -nc --arg app "$(focus_app)" --arg f "$(focus)" \
                     --arg u "$(uptime_str)" '{app:$app,focus:$f,uptime:$u}')"
  resp="$(jq -n --arg sk "step-$STEP" --arg t "$local_title" --arg i "$local_instr" \
                --arg img "$b64png" --argjson m "$meta" \
          '{stepKey:$sk,title:$t,instruction:$i,image:$img,meta:$m}' \
    | curl -sS -m 120 -X POST "$BASE/api/control/tasks" \
        -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
        --data-binary @- 2>/dev/null || true)"
  TASK_ID="$(jq -r '.id // empty' <<<"$resp" 2>/dev/null)"
  if [ -z "$TASK_ID" ]; then
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
    # SDK signal interrupts the ask loop
    if [ -f /tmp/owen-flag-sdk ]; then
      curl -sf -m 20 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
        -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
        --data-binary '{"status":"CANCELLED"}' >/dev/null 2>&1 || true
      break
    fi

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
        b64="$(base64 -w0 /tmp/owen-shot.png)"
        curl -sf -m 60 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
          -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
          --data-binary "$(jq -n --arg img "$b64" '{image:$img}')" >/dev/null 2>&1 || true
        shot_hash="$(md5sum /tmp/owen-shot.png | cut -d' ' -f1)"
      fi
      last_refresh=$SECONDS
    fi

    # Auto-refresh the pending screenshot when the screen changed on its own.
    if [ $((SECONDS - last_refresh)) -ge 45 ]; then
      if take_shot; then
        h="$(md5sum /tmp/owen-shot.png | cut -d' ' -f1)"
        if [ "$h" != "$shot_hash" ]; then
          log "screen changed while pending — auto-refreshing the image"
          cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-auto-$(date +%H%M%S).png"
          b64="$(base64 -w0 /tmp/owen-shot.png)"
          curl -sf -m 60 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
            -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
            --data-binary "$(jq -n --arg img "$b64" '{image:$img}')" >/dev/null 2>&1 || true
          shot_hash="$h"
        fi
      fi
      last_refresh=$SECONDS
    fi

    sleep 2.5
  done

  [ "$answered" = "1" ] || continue   # (deadline or sdk-flag → loop back around)
  log "step $STEP answer: action=$ACTION x=$X y=$Y wait=${WAITMS}ms desc=$DESC"

  if [ "$ACTION" = "done" ]; then
    PHASE="إنهاء بطلبك"; MESSAGE="أنهيت الجلسة — شكراً!"
    log "human requested session end"
    break
  fi

  if [ "$ACTION" = "skip" ]; then
    PHASE="تخطي — لقطة جديدة"
    curl -sf -m 20 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
      -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
      --data-binary '{"status":"CANCELLED"}' >/dev/null 2>&1 || true
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
  if [ "$w" -gt 0 ]; then
    # keep the live stream flowing during long waits: sleep in 2s slices
    rem=$(( (w + 999) / 1000 ))
    while [ "$rem" -gt 0 ]; do
      sleep $(( rem > 2 ? 2 : rem ))
      rem=$(( rem - 2 ))
    done
  fi

  sleep 2
  if take_shot; then
    cp /tmp/owen-shot.png /tmp/owen-after.png
    cp /tmp/owen-shot.png "$SHOTS_ARCHIVE/step-$(printf '%03d' "$STEP")-after-$(date +%H%M%S).png"
  else
    : > /tmp/owen-after.png
  fi
  b64a=""
  [ -s /tmp/owen-after.png ] && b64a="$(base64 -w0 /tmp/owen-after.png)"
  if [ -n "$b64a" ]; then
    curl -sf -m 90 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
      -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
      --data-binary "$(jq -n --arg img "$b64a" '{status:"APPLIED",afterImage:$img}')" >/dev/null 2>&1 || true
  else
    curl -sf -m 20 -X PATCH "$BASE/api/control/tasks/$TASK_ID" \
      -H "x-owen-token: $CONTROL_TOKEN" -H 'content-type: application/json' \
      --data-binary '{"status":"APPLIED"}' >/dev/null 2>&1 || true
  fi
  : > /tmp/owen-after.png
done

STATE="DONE"; PHASE="انتهت الجلسة"
MESSAGE="انتهت جلسة القيادة ($STEP خطوة). يمكنك إعادة تشغيل الجلسة من الموقع في أي وقت."
log "session finished after $STEP steps"
hb_now
exit 0
