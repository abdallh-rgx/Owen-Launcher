#!/usr/bin/env python3
"""Owen Control Room — local mock driver (E2E test without an emulator).

Simulates the GitHub Actions driver against a running dev site:
  • pushes live JPEG frames (~1 fps) with a fake phone screen
  • pushes logcat-ish lines + driver lines
  • posts annotation tasks (screenshot + instruction), waits for the answer,
    "applies" it and posts the next task
  • drains + acks manual signals (download_done / start_sdk / restart_app /
    crash_report / note) like the real driver

Usage:
  CONTROL_URL=http://localhost:3000 python3 mock_driver.py [--minutes 3]
Stop: touch /tmp/owen-mock-stop
"""
import base64
import io
import json
import sys
import time
import urllib.request
from datetime import datetime, timezone

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("PIL required: pip install pillow")
    sys.exit(1)

CONTROL_URL = (sys.argv[sys.argv.index("--url") + 1] if "--url" in sys.argv else "http://localhost:3000").rstrip("/")
TOKEN = "owen-ctl-7f4k9qz2m8"
MINUTES = float(sys.argv[sys.argv.index("--minutes") + 1]) if "--minutes" in sys.argv else 3
BASE = CONTROL_URL
DEV_W, DEV_H = 1080, 2280

STOP_FILE = "/tmp/owen-mock-stop"
DEADLINE = time.time() + MINUTES * 60
step = 0
screen_mode = "launcher"  # launcher → downloading → lobby


def post(path, payload, timeout=20):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(payload).encode(),
        headers={
            "Content-Type": "application/json",
            "x-owen-token": TOKEN,
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read().decode() or "{}")
    except Exception as e:  # noqa: BLE001
        return {"error": str(e)}


def get(path, timeout=15):
    req = urllib.request.Request(BASE + path, headers={"x-owen-token": TOKEN})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read().decode() or "{}")
    except Exception as e:  # noqa: BLE001
        return {"error": str(e)}


def patch(path, payload, timeout=30):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json", "x-owen-token": TOKEN},
        method="PATCH",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read().decode() or "{}")
    except Exception as e:  # noqa: BLE001
        return {"error": str(e)}


def fake_screen(mode, n, live=True):
    img = Image.new("RGB", (DEV_W, DEV_H), (12, 14, 22))
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, DEV_W, 140], fill=(24, 28, 44))
    d.text((40, 50), f"OWEN MOCK — {mode} — t={n}", fill=(120, 220, 160))
    if mode == "launcher":
        d.rounded_rectangle([340, 900, 740, 1030], radius=24, fill=(251, 191, 36))
        d.text((430, 950), "INSTALL / تحميل", fill=(10, 10, 10))
        d.rounded_rectangle([340, 1150, 740, 1280], radius=24, fill=(60, 66, 90))
        d.text((470, 1200), "SETTINGS", fill=(230, 232, 240))
    elif mode == "downloading":
        d.text((60, 800), "Downloading game data…", fill=(160, 170, 190))
        d.rectangle([140, 900, 940, 940], outline=(90, 96, 120), width=3)
        d.rectangle([140, 900, 140 + int(800 * ((n % 30) / 30)), 940], fill=(52, 211, 153))
        d.text((430, 990), f"{(n % 30) * 3}%", fill=(52, 211, 153))
    else:
        d.text((60, 700), "LOBBY — press PLAY", fill=(160, 220, 170))
        d.rounded_rectangle([380, 1500, 700, 1660], radius=28, fill=(52, 211, 153))
        d.text((520, 1560), "PLAY", fill=(6, 40, 30))
    buf = io.BytesIO()
    if live:
        small = img.resize((420, int(DEV_H * 420 / DEV_W)))
        small.save(buf, "JPEG", quality=62)
    else:
        img.save(buf, "PNG")
    return buf.getvalue()


def main():
    print(f"mock driver → {BASE} for {MINUTES} min")
    n = 0
    last_hb = 0
    last_log = 0
    last_task_post = 0
    task = None
    global step

    post("/api/control/heartbeat", {
        "state": "MOCK", "phase": "محاكاة محلية",
        "device": "Mock Pixel 4 (local)", "message": "Mock driver يعمل", "logTail": "",
    })

    while time.time() < DEADLINE:
        try:
            with open(STOP_FILE):
                print("stop file — exiting")
                break
        except FileNotFoundError:
            pass
        n += 1
        now = time.time()

        # live frame ~1fps
        jpg = fake_screen(screen_mode, n, live=True)
        r = post("/api/control/live/frame", {
            "image": base64.b64encode(jpg).decode(),
            "w": 420, "h": int(DEV_H * 420 / DEV_W),
            "deviceW": DEV_W, "deviceH": DEV_H,
            "app": f"com.abdallhrgx.owenlauncher/{screen_mode}",
        }, timeout=15)
        if "error" in r:
            print("frame error:", r["error"])

        # logs every ~2s
        if now - last_log > 2:
            last_log = now
            post("/api/control/live/log", {"lines": [
                {"level": "DRIVER", "src": "driver",
                 "text": f"{datetime.now(timezone.utc).strftime('%H:%M:%S')} mock loop n={n} mode={screen_mode}"},
                {"level": "INFO", "src": "logcat",
                 "text": f"{datetime.now(timezone.utc).strftime('%m-%d %H:%M:%S.')}{n%1000:03d} I/OwenMock: frame {n} rendered"},
                {"level": "WARN", "src": "logcat",
                 "text": f"{datetime.now(timezone.utc).strftime('%m-%d %H:%M:%S.')}{n%1000:03d} W/Choreographer: skipped frames"},
            ]})
            if n % 15 == 0:
                post("/api/control/live/log", {"lines": [
                    {"level": "ERROR", "src": "logcat",
                     "text": "E/libc: Access denied finding property \"vendor.debug.mock\""},
                ]})

        # heartbeat every ~10s
        if now - last_hb > 10:
            last_hb = now
            post("/api/control/heartbeat", {
                "state": "MOCK", "phase": f"الوضع: {screen_mode} — إطار {n}",
                "device": "Mock Pixel 4 (local)",
                "message": f"mock يعمل — خطوة {step}", "logTail": "mock log tail…",
            })

        # signals: drain + ack (like the real driver)
        sig = get("/api/control/signals/pending")
        for s in sig.get("signals", []):
            t = s["type"]
            result = {
                "download_done": "سُجّل انتهاء التحميل — تحويل الشاشة إلى اللوبي",
                "start_sdk": "سأبدأ توليد SDK (محاكاة)",
                "restart_app": "أعدت تشغيل التطبيق (محاكاة)",
                "crash_report": "جمعت تقرير الكراش (محاكاة)",
                "note": "وصلت الملاحظة ✅",
            }.get(t, "تم الاستلام")
            post(f"/api/control/signals/{s['id']}/ack", {"result": result})
            print("acked signal:", t)
            if t == "download_done":
                globals()["screen_mode"] = "lobby"

        # task loop: one pending task at a time
        if task is None and now - last_task_post > 1.5:
            step += 1
            png = fake_screen(screen_mode, n, live=False)
            r = post("/api/control/tasks", {
                "stepKey": f"mock-step-{step}",
                "title": f"خطوة {step} — حدّد الزر التالي (mock)",
                "instruction": "هذه شاشة وهمية للاختبار: انقر على الزر البرتقالي INSTALL، اكتب وصفاً، وحدد انتظار ثانيتين ثم أرسل.",
                "image": base64.b64encode(png).decode(),
                "meta": {"app": "mock/app", "focus": "mock", "uptime": f"{n}s", "note": "mock"},
            }, timeout=30)
            task = r.get("id")
            last_task_post = now
            print("posted task", task)

        if task:
            st = get(f"/api/control/tasks/{task}").get("task", {})
            status = st.get("status")
            if status == "ANSWERED":
                print("answer:", st.get("action"), st.get("x"), st.get("y"),
                      st.get("waitMs"), st.get("description"))
                if st.get("description") and "install" in st.get("description", "").lower():
                    globals()["screen_mode"] = "downloading"
                patch(f"/api/control/tasks/{task}", {"status": "APPLIED"})
                task = None
            elif status in ("CANCELLED", "APPLIED", "FAILED"):
                task = None

        time.sleep(1.1)

    post("/api/control/heartbeat", {
        "state": "DONE", "phase": "انتهى الـ mock",
        "device": "Mock Pixel 4 (local)", "message": "mock driver انتهى", "logTail": "",
    })
    print("mock driver done")


if __name__ == "__main__":
    main()
