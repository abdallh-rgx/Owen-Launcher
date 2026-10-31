#!/usr/bin/env python3
"""Owen Control Room — local mock driver (E2E test without an emulator).

Simulates the GitHub Actions driver against a running dev site — now built
for testing the AUTOPILOT loop end-to-end:

  • pushes live JPEG frames (~1 fps) with a fake phone screen
  • pushes logcat-ish lines + driver lines
  • posts annotation tasks (screenshot + instruction), waits for the answer
    (the autopilot answers via glm-4.6v), "applies" it and posts the next
  • screen state machine driven by WHERE the autopilot taps:
      launcher  → tap inside INSTALL  → downloading
      downloading → pct 40 → 75 → 100 → lobby
      lobby     → tap inside PLAY     → in_match
      in_match  → stays (gameplay HUD)
  • injects one fake CRASH batch mid-run to exercise the crash analyst +
    auto restart_app signal
  • drains + acks signals (download_done / start_sdk / restart_app /
    crash_report / note) like the real driver

Usage:
  CONTROL_URL=http://localhost:3000 python3 mock_driver.py [--minutes 6]
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
MINUTES = float(sys.argv[sys.argv.index("--minutes") + 1]) if "--minutes" in sys.argv else 6
BASE = CONTROL_URL
DEV_W, DEV_H = 1080, 2280

# أزرار حقيقية بإحداثيات جهاز — الطيار يجب أن يكتشفها بالرؤية
INSTALL_BBOX = (340, 900, 740, 1030)     # زر التثبيت البرتقالي
PLAY_BBOX = (380, 1500, 700, 1660)       # زر اللعب في اللوبي

STOP_FILE = "/tmp/owen-mock-stop"
DEADLINE = time.time() + MINUTES * 60
step = 0
screen_mode = "launcher"   # launcher → downloading → lobby → in_match
download_steps = 0
crash_injected = False
PCT_BY_STEP = [40, 75, 100]  # نسب التحميل لكل خطوة انتظار


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


def fake_screen(mode, n, pct=0, live=True):
    img = Image.new("RGB", (DEV_W, DEV_H), (12, 14, 22))
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, DEV_W, 140], fill=(24, 28, 44))
    d.text((40, 50), f"OWEN LAUNCHER — {mode} — t={n}", fill=(120, 220, 160))
    if mode == "launcher":
        d.text((60, 400), "Owen Launcher v1.8 — Fortnite 12.41 offline", fill=(200, 205, 220))
        d.text((60, 460), "Ready to install game data", fill=(150, 158, 178))
        d.rounded_rectangle(list(INSTALL_BBOX), radius=24, fill=(251, 191, 36))
        d.text((430, 950), "INSTALL / تثبيت", fill=(10, 10, 10))
        d.rounded_rectangle([340, 1150, 740, 1280], radius=24, fill=(60, 66, 90))
        d.text((470, 1200), "SETTINGS", fill=(230, 232, 240))
    elif mode == "downloading":
        d.text((60, 700), "Downloading game data…", fill=(160, 170, 190))
        d.rectangle([140, 900, 940, 940], outline=(90, 96, 120), width=3)
        d.rectangle([140, 900, 140 + int(800 * pct / 100), 940], fill=(52, 211, 153))
        d.text((430, 990), f"{pct}%", fill=(52, 211, 153))
        d.text((60, 1100), "please wait — do not close the app", fill=(110, 118, 140))
    elif mode == "lobby":
        # لوبي فورتنايت مبسّط: شخصية + قوائم أسفل + زر PLAY
        d.rectangle([0, 140, DEV_W, 1400], fill=(28, 34, 58))     # خلفية اللوبي
        d.ellipse([430, 420, 650, 700], fill=(90, 100, 140))      # رأس الشخصية
        d.rectangle([420, 700, 660, 1150], fill=(60, 72, 110))    # جسم الشخصية
        d.text((60, 200), "FORTNITE — LOBBY", fill=(240, 220, 130))
        d.text((60, 250), "Battle Royale  •  Season 12.41", fill=(170, 178, 200))
        for i, label in enumerate(["LOBBY", "LOCKER", "ITEM SHOP", "CAREER"]):
            x = 60 + i * 250
            d.rounded_rectangle([x, 1240, x + 220, 1350], radius=18, fill=(44, 50, 74))
            d.text((x + 40, 1285), label, fill=(190, 196, 214))
        d.rounded_rectangle(list(PLAY_BBOX), radius=28, fill=(52, 211, 153))
        d.text((520, 1560), "PLAY", fill=(6, 40, 30))
    else:  # in_match — واجهة لعب: HUD + منظور + خريطة مصغرة
        sky = (94, 130, 172)
        d.rectangle([0, 140, DEV_W, 1200], fill=sky)
        d.rectangle([0, 1200, DEV_W, DEV_H], fill=(64, 92, 58))     # الأرض
        d.rectangle([380, 1050, 700, 1200], fill=(96, 78, 60))      # مبنى
        d.text((60, 300), "MATCH LIVE — SOLO", fill=(255, 255, 255))
        d.text((60, 360), "Players left: 87", fill=(230, 235, 245))
        # HUD صحة/درع
        d.rectangle([60, 2050, 480, 2080], fill=(40, 200, 120))     # health
        d.rectangle([60, 2090, 350, 2110], fill=(80, 180, 255))     # shield
        d.text((700, 2060), "AR  ∞", fill=(255, 255, 255))
        # خريطة مصغرة
        d.rectangle([820, 160, 1040, 380], outline=(255, 255, 255), width=3)
        d.ellipse([920, 260, 940, 280], fill=(52, 211, 153))        # موقعي
        # مقصد التصويب
        d.line([530, 660, 550, 660], fill=(255, 255, 255), width=3)
        d.line([530, 690, 550, 690], fill=(255, 255, 255), width=3)
        d.line([530, 675, 550, 675], fill=(255, 255, 255), width=2)
    buf = io.BytesIO()
    if live:
        small = img.resize((420, int(DEV_H * 420 / DEV_W)))
        small.save(buf, "JPEG", quality=62)
    else:
        img.save(buf, "PNG")
    return buf.getvalue()


def in_bbox(x, y, bbox):
    return x is not None and y is not None and bbox[0] <= x <= bbox[2] and bbox[1] <= y <= bbox[3]


def main():
    global step, screen_mode, download_steps, crash_injected
    print(f"mock driver → {BASE} for {MINUTES} min")
    n = 0
    last_hb = 0
    last_log = 0
    last_task_post = 0
    task = None

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
        pct = PCT_BY_STEP[min(download_steps, len(PCT_BY_STEP) - 1)] if screen_mode == "downloading" else 0
        jpg = fake_screen(screen_mode, n, pct=pct, live=True)
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

        # حدث كراش وهمي واحد بعد ~45 ثانية — لاختبار المحلل الآلي + إعادة التشغيل
        if not crash_injected and n >= 40:
            crash_injected = True
            post("/api/control/live/log", {"lines": [
                {"level": "CRASH", "src": "crash",
                 "text": "FATAL EXCEPTION: main (mock)"},
                {"level": "CRASH", "src": "crash",
                 "text": "Process: com.abdallhrgx.owenlauncher, PID: 8412"},
                {"level": "CRASH", "src": "crash",
                 "text": "java.lang.UnsatisfiedLinkError: dlopen failed: library \"libOwen.so\" not found (mock)"},
                {"level": "CRASH", "src": "crash",
                 "text": "  at com.abdallhrgx.owenlauncher.MainActivity.onCreate(MainActivity.kt:88)"},
            ]})
            print("injected fake CRASH batch")

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
                "download_done": "سُجّل انتهاء التحميل — جاهز للانطلاق",
                "start_sdk": "بدأت توليد SDK (محاكاة) — سأسحب ملفات الـ dump",
                "restart_app": "أعدت تشغيل التطبيق (محاكاة)",
                "crash_report": "جمعت تقرير الكراش (محاكاة)",
                "note": "وصلت الملاحظة ✅",
            }.get(t, "تم الاستلام")
            post(f"/api/control/signals/{s['id']}/ack", {"result": result})
            print("acked signal:", t)
            if t == "restart_app":
                screen_mode = "launcher"  # الكراش يعيدنا للبداية (محاكاة واقعية)

        # task loop: one pending task at a time
        if task is None and now - last_task_post > 1.5:
            step += 1
            pct = PCT_BY_STEP[min(download_steps, len(PCT_BY_STEP) - 1)] if screen_mode == "downloading" else 0
            png = fake_screen(screen_mode, n, pct=pct, live=False)
            titles = {
                "launcher": "خطوة {s} — شاشة القاذفة (mock)",
                "downloading": "خطوة {s} — تحميل بيانات اللعبة (mock)",
                "lobby": "خطوة {s} — لوبي فورتنايت (mock)",
                "in_match": "خطوة {s} — داخل مباراة حية (mock)",
            }
            r = post("/api/control/tasks", {
                "stepKey": f"mock-step-{step}",
                "title": titles.get(screen_mode, "خطوة {s}").format(s=step),
                "instruction": "شاشة وهمية لاختبار الطيار الآلي — يفترض أن يضغط الزر الصحيح بنفسه.",
                "image": base64.b64encode(png).decode(),
                "meta": {"app": "mock/app", "focus": "mock", "uptime": f"{n}s", "note": "mock"},
            }, timeout=30)
            task = r.get("id")
            last_task_post = now
            print("posted task", task, "mode:", screen_mode)

        if task:
            st = get(f"/api/control/tasks/{task}").get("task", {})
            status = st.get("status")
            if status == "ANSWERED":
                act, x, y = st.get("action"), st.get("x"), st.get("y")
                print(f"answer: {act} ({x},{y}) wait={st.get('waitMs')}ms — {st.get('description')}")
                # آلة الحالة: التقدّم حسب موضع النقر الفعلي
                if screen_mode == "launcher" and act == "tap" and in_bbox(x, y, INSTALL_BBOX):
                    screen_mode = "downloading"
                    download_steps = 0
                    print("→ mode: downloading")
                elif screen_mode == "downloading":
                    download_steps += 1
                    if download_steps >= len(PCT_BY_STEP):
                        screen_mode = "lobby"
                        print("→ mode: lobby (اكتمل التحميل)")
                elif screen_mode == "lobby" and act == "tap" and in_bbox(x, y, PLAY_BBOX):
                    screen_mode = "in_match"
                    print("→ mode: in_match")
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
