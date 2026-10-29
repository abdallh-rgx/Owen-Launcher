#!/usr/bin/env python3
"""Pixel-probe UI automation for the Owen Launcher E2E workflow (android-e2e.yml).

The launcher is an egui NativeActivity app — it has NO accessibility nodes,
so it cannot be driven with uiautomator. This helper drives it the only
reliable way: fixed coordinates on a pinned emulator (1080x2280 @ 440dpi)
plus PIL pixel probes that find the app's own widgets from their colors
(charcoal panel / card brown / Owen gold / near-black text fields), taken
from crates/owen-app/src/ui.rs + lib.rs (widgets_gold):

    PANEL   #141009   CARD  #1e1a12   GOLD #ffb300   RED #f25c5c
    FIELD   #0b0b0f (extreme_bg)      GREEN #54d18a

System UI (the PackageInstaller confirmation dialog) DOES have accessibility
nodes — `tap-uiautomator TEXT` covers that one case.

Usage (run on the GitHub Actions runner, adb on PATH):
  python3 owen_ui.py screenshot /tmp/shots/x.png
  python3 owen_ui.py tap 540 1650
  python3 owen_ui.py swipe-up            # scroll the egui ScrollArea down
  python3 owen_ui.py text OwenTester     # adb input text (spaces → %s)
  python3 owen_ui.py bands               # debug: print every detected band
  python3 owen_ui.py tap-lang 2          # tap the 3rd language card (English)
  python3 owen_ui.py tap-gold bottom 0   # tap the lowest bright-gold button
  python3 owen_ui.py count-gold 2050 2280   # gold bands in a row range
  python3 owen_ui.py count-fields        # number of TextEdit fields visible
  python3 owen_ui.py tap-field 0         # tap the Nth (from top) text field
  python3 owen_ui.py has-red-text        # the app's red error card visible?
  python3 owen_ui.py wait-file /data/data/com.abdallhrgx.owenlauncher/files/owen_prefs.json '"en"' 30
  python3 owen_ui.py wait-shell 30       # shell view (bottom nav w/ gold pill)
  python3 owen_ui.py wait-details 15     # details screen (bottom nav gone)
  python3 owen_ui.py find-season public/owen/seasons 13   # art matcher → "TAP x y"
  python3 owen_ui.py wait-package com.epicgames.fortnite130013649278 900
  python3 owen_ui.py pidof com.epicgames.fortnite 5
  python3 owen_ui.py pull /data/data/com.abdallhrgx.owenlauncher/files/logs/owen_log.txt out.txt
  python3 owen_ui.py tap-uiautomator Install   # system dialog button by text
"""

import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

from PIL import Image

W, H = 1080, 2280  # hw.lcd.width/height — pinned by the workflow's AVD

# --- palette (owen-app/src/lib.rs widgets_gold + ui.rs) ----------------------
GOLD = (0xFF, 0xB3, 0x00)
PANEL = (0x14, 0x10, 0x09)
CARD = (0x1E, 0x1A, 0x12)
FIELD = (0x0B, 0x0B, 0x0F)
RED = (0xF2, 0x5C, 0x5C)


def adb(*args, timeout=90):
    """Run adb (list form — no shell quoting surprises) and return stdout."""
    cmd = ["adb"] + [str(a) for a in args]
    r = subprocess.run(cmd, capture_output=True, timeout=timeout)
    out = (r.stdout or b"").decode("utf-8", "replace")
    err = (r.stderr or b"").decode("utf-8", "replace")
    return out + (("\n[stderr] " + err) if err.strip() else "")


def sh(*args, timeout=90):
    """adb shell ... (list form)."""
    return adb("shell", *args, timeout=timeout)


# --- primitives --------------------------------------------------------------


def screenshot(out):
    raw = subprocess.run(["adb", "exec-out", "screencap", "-p"],
                         capture_output=True, timeout=60).stdout
    if not raw or not raw.startswith(b"\x89PNG"):
        # some adb builds mangle exec-out; retry via /sdcard
        sh("screencap", "-p", "/sdcard/owen_shot.png")
        subprocess.run(["adb", "pull", "/sdcard/owen_shot.png", out],
                       capture_output=True, timeout=60)
        raw = open(out, "rb").read()
    open(out, "wb").write(raw)
    return Image.open(out).convert("RGB")


def tap(x, y):
    sh("input", "tap", int(x), int(y))


def swipe_up(distance=1500, duration=300):
    """Scroll an egui ScrollArea down (content moves up). egui has no fling —
    the content follows the finger, so this is deterministic."""
    y1 = min(1900, H - 380)
    y2 = max(200, y1 - int(distance))
    sh("input", "swipe", 540, y1, 540, y2, int(duration))


def input_text(s):
    s = s.replace(" ", "%s")
    sh("input", "text", s)


# --- pixel probes ------------------------------------------------------------


def close(px, ref, tol):
    return all(abs(px[i] - ref[i]) <= tol for i in range(3))


def row_runs(img, y, match, min_run):
    """Widest run of `match` pixels in row y (used for band detection)."""
    w = img.width
    px = img.load()
    best = run = 0
    for x in range(w):
        if match(px[x, y]):
            run += 1
            best = max(best, run)
        else:
            run = 0
    return best if best >= min_run else 0


def bands(img, match, min_run, min_h, max_h=10_000):
    """Horizontal bands of consecutive rows whose widest run >= min_run."""
    out = []
    start = None
    for y in range(img.height):
        wide = row_runs(img, y, match, min_run) > 0
        if wide and start is None:
            start = y
        elif not wide and start is not None:
            if min_h <= y - start <= max_h:
                out.append((start, y))
            start = None
    if start is not None and min_h <= img.height - start <= max_h:
        out.append((start, img.height))
    return out


def is_gold(px):
    r, g, b = px
    return r >= 175 and 105 <= g <= 220 and b <= 80 and (r - b) >= 120 and (g - b) >= 60


def is_card(px):
    return close(px, CARD, 10)


def is_field(px):
    r, g, b = px
    return r <= 20 and g <= 20 and b <= 24  # extreme_bg #0b0b0f (+ antialias)


def is_red_text(px):
    r, g, b = px
    return r >= 190 and g <= 120 and b <= 120 and (r - g) >= 90


def gold_bands(img):
    # buttons ~40pt (110px) tall, nav pill ~30pt (82px); gold text is <55px
    return bands(img, is_gold, min_run=100, min_h=55, max_h=260)


def card_bands(img):
    return bands(img, is_card, min_run=int(img.width * 0.30), min_h=60, max_h=400)


def field_bands(img):
    return bands(img, is_field, min_run=280, min_h=32, max_h=200)


def red_bands(img):
    return bands(img, is_red_text, min_run=30, min_h=14, max_h=90)


def band_center(img, y0, y1):
    """x = center of the widest matching run in the middle row, y = middle."""
    y = (y0 + y1) // 2
    px = img.load()
    best = (0, 0, 0)  # len, x0, x1
    run = x0 = 0
    for x in range(img.width):
        if is_gold(px[x, y]):
            if run == 0:
                x0 = x
            run += 1
            if run > best[0]:
                best = (run, x0, x0 + run)
        else:
            run = 0
    if best[0] == 0:
        return (img.width // 2, y)
    return ((best[1] + best[2]) // 2, y)


# --- art matcher (season cards) ---------------------------------------------


def downsample(box_img, n=8):
    return list(box_img.resize((n, n), Image.BILINEAR).getdata())


def art_bbox(img, box):
    """Bounding box of the non-card-bg pixels inside `box` (the season art)."""
    px = img.load()
    x0b, y0b, x1b, y1b = box
    minx = miny = maxx = maxy = None
    step = 4
    for y in range(y0b, y1b, step):
        for x in range(x0b, x1b, step):
            p = px[x, y]
            if not (close(p, CARD, 12) or close(p, PANEL, 12)):
                minx = x if minx is None else min(minx, x)
                maxx = x if maxx is None else max(maxx, x)
                miny = y if miny is None else min(miny, y)
                maxy = y if maxy is None else max(maxy, y)
    if None in (minx, miny, maxx, maxy):
        return None
    if (maxx - minx) < 60 or (maxy - miny) < 60:
        return None  # too small to be the art (placeholder card)
    return (minx, miny, maxx + 1, maxy + 1)


def dist(a, b):
    """L1 distance between two flat getdata() fingerprints (64 RGB tuples)."""
    return sum(abs(a[i][k] - b[i][k]) for i in range(len(a)) for k in range(3))


def find_season(artdir, season):
    refs = {}
    img = screenshot("/tmp/owen_ui_find_season.png")
    # reference fingerprints for every season that could plausibly be on screen
    for n in range(5, 43):
        p = f"{artdir}/season_{n}_folder.png"
        try:
            ref_img = Image.open(p).convert("RGB")
        except OSError:
            continue
        refs[n] = downsample(ref_img)
    if season not in refs:
        print(f"NO-REF season {season} (art dir {artdir})")
        return False
    scored = []
    for (y0, y1) in card_bands(img):
        art = art_bbox(img, (40, y0, img.width - 40, y0 + int((y1 - y0) * 0.75)))
        if art is None:
            continue
        fp = downsample(img.crop(art))
        scored.extend((dist(fp, refs[n]), n, (y0, y1)) for n in refs)
    if not scored:
        print("NO-CARDS")
        return False
    scored.sort()
    best_d, best_n, best_band = scored[0]
    second = [d for d, n, _ in scored if n != best_n]
    if second and best_d > 0.72 * second[0]:
        print(f"AMBIGUOUS best={best_n} d={best_d} second={second[0]}")
        return False
    x, y = img.width // 2, (best_band[0] + best_band[1]) // 2
    print(f"MATCH season {best_n} d={best_d} band={best_band}")
    print(f"TAP {x} {y}")
    return True


# --- waiters -----------------------------------------------------------------


def wait_file(device_path, needle, timeout):
    deadline = time.time() + int(timeout)
    while time.time() < deadline:
        out = sh("cat", device_path, timeout=30)
        if needle in out:
            print(f"FOUND {needle!r} in {device_path}")
            return True
        time.sleep(2)
    print(f"TIMEOUT waiting for {needle!r} in {device_path}")
    return False


def wait_bottom_nav(want, timeout):
    """want=True → wait for the shell bottom nav (gold pill);
    want=False → wait until it is gone (details screen)."""
    deadline = time.time() + int(timeout)
    while time.time() < deadline:
        img = screenshot("/tmp/owen_ui_nav.png")
        gold = [b for b in gold_bands(img) if b[0] >= int(H * 0.90)]
        if bool(gold) == want:
            print(f"bottom-nav gold={'present' if want else 'gone'}: {gold}")
            return True
        time.sleep(2)
    print(f"TIMEOUT bottom-nav gold={'present' if want else 'gone'}")
    return False


def wait_package(pkg, timeout):
    deadline = time.time() + int(timeout)
    while time.time() < deadline:
        out = sh("pm", "list", "packages", timeout=30)
        if f"package:{pkg}" in out:
            print(f"INSTALLED {pkg}")
            return True
        time.sleep(5)
    print(f"TIMEOUT waiting for package {pkg}")
    return False


def pidof(pkg, timeout=10):
    deadline = time.time() + int(timeout)
    while time.time() < deadline:
        out = sh("pidof", pkg, timeout=30).strip()
        if out:
            print(f"ALIVE {pkg} pid={out.split()[0]}")
            return True
        time.sleep(3)
    print(f"NOT-RUNNING {pkg}")
    return False


# --- system dialog (PackageInstaller has accessibility nodes) ----------------


def tap_uiautomator(text, exact=True, timeout=30):
    deadline = time.time() + int(timeout)
    while time.time() < deadline:
        sh("uiautomator", "dump", "/sdcard/ui.xml", timeout=60)
        xml = sh("cat", "/sdcard/ui.xml", timeout=30)
        try:
            root = ET.fromstring(xml)
        except ET.ParseError:
            time.sleep(2)
            continue
        for node in root.iter("node"):
            hay = (node.get("text") or "") + " " + (node.get("content-desc") or "")
            hit = (hay.strip() == text) if exact else (text in hay)
            if hit:
                m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
                if m:
                    x1, y1, x2, y2 = map(int, m.groups())
                    tap((x1 + x2) // 2, (y1 + y2) // 2)
                    print(f"TAPPED uiautomator {text!r} at {(x1 + x2) // 2, (y1 + y2) // 2}")
                    return True
        time.sleep(2)
    print(f"TIMEOUT uiautomator {text!r}")
    return False


# --- command dispatch --------------------------------------------------------


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    cmd, rest = sys.argv[1], sys.argv[2:]

    if cmd == "screenshot":
        screenshot(rest[0])
        print(f"SAVED {rest[0]}")
    elif cmd == "tap":
        tap(rest[0], rest[1])
        print(f"TAPPED {rest[0]} {rest[1]}")
    elif cmd == "swipe-up":
        swipe_up(int(rest[0]) if rest else 1500)
        print("SWIPED")
    elif cmd == "text":
        input_text(rest[0])
        print(f"TYPED {rest[0]!r}")
    elif cmd == "bands":
        img = screenshot("/tmp/owen_ui_bands.png")
        print("gold:", gold_bands(img))
        print("cards:", card_bands(img))
        print("fields:", field_bands(img))
        print("red:", red_bands(img))
    elif cmd == "tap-lang":
        img = screenshot("/tmp/owen_ui_lang.png")
        cards = card_bands(img)
        i = int(rest[0])
        if i >= len(cards):
            print(f"NO lang card #{i} (have {len(cards)})")
            return 1
        y0, y1 = cards[i]
        tap(W // 2, (y0 + y1) // 2)
        print(f"TAPPED lang card #{i} at {W // 2, (y0 + y1) // 2} band={(y0, y1)}")
    elif cmd == "tap-gold":
        which, i = rest[0], int(rest[1])
        img = screenshot("/tmp/owen_ui_gold.png")
        bs = gold_bands(img)
        if which == "bottom":
            bs = sorted(bs, key=lambda b: -b[0])
        if i >= len(bs):
            print(f"NO gold band #{i} {which} (have {len(bs)})")
            return 1
        y0, y1 = bs[i]
        x, y = band_center(img, y0, y1)
        tap(x, y)
        print(f"TAPPED gold #{i} {which} at {x},{y} band={(y0, y1)}")
    elif cmd == "count-gold":
        y1, y2 = int(rest[0]), int(rest[1])
        img = screenshot("/tmp/owen_ui_cg.png")
        n = [b for b in gold_bands(img) if b[0] >= y1 and b[1] <= y2]
        print(f"GOLD-BANDS {len(n)} in y{y1}..y{y2}: {n}")
    elif cmd == "count-fields":
        img = screenshot("/tmp/owen_ui_cf.png")
        print(f"FIELDS {len(field_bands(img))}: {field_bands(img)}")
    elif cmd == "tap-field":
        img = screenshot("/tmp/owen_ui_field.png")
        fs = field_bands(img)
        i = int(rest[0])
        if i >= len(fs):
            print(f"NO field #{i} (have {len(fs)})")
            return 1
        y0, y1 = fs[i]
        tap(W // 2, (y0 + y1) // 2)
        print(f"TAPPED field #{i} at {W // 2, (y0 + y1) // 2} band={(y0, y1)}")
    elif cmd == "has-red-text":
        img = screenshot("/tmp/owen_ui_red.png")
        n = red_bands(img)
        print(f"RED-BANDS {len(n)}: {n}")
        return 0 if n else 1
    elif cmd == "wait-file":
        return 0 if wait_file(rest[0], rest[1], int(rest[2]) if len(rest) > 2 else 30) else 1
    elif cmd == "wait-shell":
        return 0 if wait_bottom_nav(True, int(rest[0])) else 1
    elif cmd == "wait-details":
        return 0 if wait_bottom_nav(False, int(rest[0])) else 1
    elif cmd == "find-season":
        return 0 if find_season(rest[0], int(rest[1])) else 1
    elif cmd == "wait-package":
        return 0 if wait_package(rest[0], int(rest[1]) if len(rest) > 1 else 900) else 1
    elif cmd == "pidof":
        return 0 if pidof(rest[0], int(rest[1]) if len(rest) > 1 else 10) else 1
    elif cmd == "pull":
        out = sh("cat", rest[0], timeout=60)
        open(rest[1], "w", encoding="utf-8", errors="replace").write(out)
        print(f"PULLED {rest[0]} → {rest[1]} ({len(out)} chars)")
    elif cmd == "tap-uiautomator":
        return 0 if tap_uiautomator(rest[0]) else 1
    else:
        print(f"unknown command {cmd!r}")
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
