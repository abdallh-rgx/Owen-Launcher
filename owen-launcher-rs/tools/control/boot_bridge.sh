#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# boot_bridge.sh — stream the emulator boot log to the Control Room site.
#
# Runs beside the emulator during boot (BEFORE driver.sh takes over): every
# 5s it ships new /tmp/emulator.log bytes to /api/control/live/log and keeps
# a heartbeat alive, so the boot is visible live on the site — including
# crash reasons when the emulator dies.
#
# Usage: boot_bridge.sh CONTROL_URL CONTROL_TOKEN FINISH_MARKER
# Exits when FINISH_MARKER appears (touched on boot success OR failure).
# ---------------------------------------------------------------------------
set -uo pipefail
BASE="${1:?CONTROL_URL required}"
TOKEN="${2:?token required}"
MARKER="${3:?marker required}"
LOG=/tmp/emulator.log
last=0
loops=0

post() { # $1 path  $2 json
  curl -sS -m 12 -X POST "$BASE$1" \
    -H "x-owen-token: $TOKEN" -H 'content-type: application/json' \
    --data-binary "$2" >/dev/null 2>&1 || true
}

# أول سطر حتى يظهر الدليل على الموقع فوراً
post /api/control/live/log '{"lines":[{"level":"DRIVER","src":"driver","text":"[boot] emulator starting…"}]}'

while [ ! -e "$MARKER" ]; do
  sleep 5
  loops=$((loops + 1))

  # 1) شحن أسطر المحاكي الجديدة
  sz=$(stat -c%s "$LOG" 2>/dev/null || echo 0)
  if [ "$sz" -gt "$last" ]; then
    LAST_OFF="$last" python3 - > /tmp/boot-bridge-payload.json <<'PY'
import json, os
off = int(os.environ.get("LAST_OFF", "0"))
data = open("/tmp/emulator.log", "rb").read()[off:off + 16000]
text = data.decode("utf-8", "replace")
lines = [l for l in text.splitlines() if l.strip()][-25:]
print(json.dumps({
    "lines": [
        {"level": "DRIVER", "src": "driver", "text": "[emulator] " + l}
        for l in lines
    ]
}))
PY
    if [ -s /tmp/boot-bridge-payload.json ]; then
      post /api/control/live/log "$(cat /tmp/boot-bridge-payload.json)"
    fi
    last=$sz
  fi

  # 2) نبضة كل ~20 ثانية
  if [ $((loops % 4)) -eq 1 ]; then
    tail_line="$(tail -n 1 "$LOG" 2>/dev/null | head -c 150)"
    post /api/control/heartbeat "$(jq -nc --arg m "[boot] ${tail_line:-emulator booting…}" \
      '{state:"BOOT", phase:"المحاكي يقلع (جسر السجل)", device:"emulator (booting)", message:$m, logTail:""}')"
  fi
done
echo "boot bridge: marker seen — exiting"
