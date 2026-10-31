#!/usr/bin/env python3
"""tools/ci/drive.py — Phase 3: drive Fortnite 12.41 inside the emulator.

The whole E2E in one deterministic loop:
  1. owen-server on :8080 (host) + `adb reverse tcp:8080`
  2. install the shim-patched APK, push owen.json/ogsen.json
  3. launch → wait for the content download (real Epic CDN via our redirect)
  4. blind-tap the login UI; the in-game webview auto-completes on our
     /login page (password grant → exchange code → RedirectUrl/exchange)
  5. lobby screenshot the moment the backend sees grant=exchange_code
  6. libOwenOGS opens Apollo_Terrain after `world_open_delay` → the
     "OGS: match world up" marker → in-match screenshot

Outputs: /tmp/e2e/{lobby.png,match.png,status.json,logcat.txt,server.log,shots/}
Exit code 0 iff lobby.png exists (match.png best-effort — reported honestly).
"""
import json
import re
import os
import subprocess
import sys
import time

PKG = "com.epicgames.fortnite"
ACTIVITY = f"{PKG}/com.epicgames.ue4.GameActivity"
OUT = "/tmp/e2e"
PORT = 8080
BASE = f"http://127.0.0.1:{PORT}"
W, H = 1280, 720
DOWNLOAD_TIMEOUT = int(os.environ.get("OWEN_DOWNLOAD_TIMEOUT", "2700"))
LOGIN_TIMEOUT = int(os.environ.get("OWEN_LOGIN_TIMEOUT", "900"))
MATCH_TIMEOUT = int(os.environ.get("OWEN_MATCH_TIMEOUT", "1500"))


def adb(*args, timeout=60):
    return subprocess.run(
        ["adb", *args], capture_output=True, text=True, timeout=timeout
    )


def adb_ok(*args, timeout=60):
    r = adb(*args, timeout=timeout)
    if r.returncode != 0:
        print(f"[adb-fail] adb {' '.join(args)}: {r.stderr.strip()[:300]}")
    return r.returncode == 0


def shot(name):
    os.makedirs(f"{OUT}/shots", exist_ok=True)
    path = f"{OUT}/{name}"
    with open(path, "wb") as f:
        r = subprocess.run(["adb", "exec-out", "screencap", "-p"], stdout=f, timeout=60)
    size = os.path.getsize(path) if os.path.exists(path) else 0
    print(f"[shot] {name} ({size}B)")
    return size


def periodic_shots(stop, interval=25):
    i = 0
    while time.time() < stop:
        shot(f"shots/{i:03d}.png")
        i += 1
        time.sleep(interval)


def logcat_grep(pattern):
    r = adb("logcat", "-d", timeout=90)
    return pattern in (r.stdout or "")


def server_log_grep(pattern):
    try:
        with open(f"{OUT}/server.log", "r", errors="replace") as f:
            return pattern in f.read()
    except FileNotFoundError:
        return False


def http(path):
    import urllib.request

    try:
        with urllib.request.urlopen(BASE + path, timeout=10) as r:
            return r.read().decode(errors="replace")
    except Exception as e:
        return f"__err__ {e}"


def files_dir_bytes():
    r = adb("shell", "du", "-sk", f"/storage/emulated/0/Android/data/{PKG}/files", timeout=60)
    try:
        return int((r.stdout or "0").split()[0])
    except (ValueError, IndexError):
        return 0


def push_config():
    """owen.json (redirecter) + ogsen.json (the mod) into the app's files dir."""
    files = f"/storage/emulated/0/Android/data/{PKG}/files"
    adb_ok("shell", "mkdir", "-p", files)
    owen = {
        "version": 1,
        "server": {"base_url": f"http://127.0.0.1:{PORT}", "xmpp_host": "127.0.0.1", "xmpp_port": PORT},
        "webview": {"login_url": f"{BASE}/login", "force_cleartext": True, "partyhub": "off"},
        "hooks": {"curl": True, "art_webview": False},
        "manifest": {"mode": "auto"},
        "redirect": {"domains": [".epicgames.com", ".epicgames.net", ".epicgames.dev"]},
    }
    ogsen = {
        "version": 1,
        "game": "12.41",
        "features": {
            "gis_flip": True,
            "netmode_hooks": True,
            "bots": True,
            "bosses": True,
            "booths": True,
            "vehicles": True,
            "exec_hooks": True,
            "bots_tick_thread": True,
            "legacy_ai_mang": True,
            "max_bots": 100,
            "world_open_delay": int(os.environ.get("OWEN_WORLD_OPEN_DELAY", "90")),
        },
        "vt_indices": {},
    }
    for name, obj in (("owen.json", owen), ("ogsen.json", ogsen)):
        with open(f"/tmp/{name}", "w") as f:
            json.dump(obj, f, indent=2)
        adb_ok("push", f"/tmp/{name}", f"/data/local/tmp/{name}")
        adb_ok("shell", f"cp /data/local/tmp/{name} {files}/{name}")
        adb_ok("shell", f"chmod 644 {files}/{name}")
    print("[cfg] owen.json + ogsen.json pushed")


def wait_download(timeout):
    """The real content download (chunkdbs from the live CDN via our redirect)."""
    print(f"[dl] waiting for the content download (≤{timeout}s)…")
    deadline = time.time() + timeout
    last, stable, first = -1, 0, 0
    while time.time() < deadline:
        cur = files_dir_bytes()
        if cur > 512 * 1024:  # >512MiB on disk
            if first == 0:
                first = cur
                print(f"[dl] >512MiB present ({cur // 1024}MiB)")
            if cur == last:
                stable += 1
                if stable >= 3:  # ~60s without growth
                    print(f"[dl] settled at {cur // 1024}MiB")
                    return True
            else:
                stable = 0
        else:
            stable = 0
        if last != -1 and cur != last:
            print(f"[dl] {cur // 1024}MiB …")
        last = cur
        time.sleep(20)
    print("[dl] TIMEOUT — proceeding anyway")
    return False


def do_login(timeout):
    """Blind-tap the login UI; the webview auto-completes on our /login."""
    print(f"[login] tapping through the login UI (≤{timeout}s)…")
    deadline = time.time() + timeout
    attempt = 0
    while time.time() < deadline:
        if server_log_grep("grant=exchange_code"):
            print("[login] exchange_code grant seen — login complete")
            return True
        attempt += 1
        # 1) 'tap to start' / any interstitial → center tap
        adb_ok("shell", "input", "tap", str(W // 2), str(int(H * 0.90)))
        time.sleep(6)
        # 2) the sign-in CTA (bottom third, centered)
        adb_ok("shell", "input", "tap", str(W // 2), str(int(H * 0.86)))
        time.sleep(10)
        if server_log_grep("grant=exchange_code"):
            print("[login] exchange_code grant seen — login complete")
            return True
        # 3) some builds put the CTA mid-screen
        adb_ok("shell", "input", "tap", str(W // 2), str(int(H * 0.55)))
        time.sleep(10)
        if attempt % 3 == 0:
            shot(f"login-try{attempt:02d}.png")
    print("[login] TIMEOUT — no exchange_code grant")
    return False




def _ui_tap(pattern):
    """uiautomator dump → find a node matching `pattern` (text= or
    resource-id=) → tap the exact center of its bounds."""
    adb("shell", "uiautomator", "dump", "/sdcard/ui.xml", timeout=30)
    xml = adb("shell", "cat", "/sdcard/ui.xml", timeout=30).stdout or ""
    m = None
    for node in re.finditer(r"<node[^>]*/>", xml):
        s = node.group(0)
        if re.search(pattern, s):
            m = re.search(r"bounds=\[(\d+),(\d+)\]\[(\d+),(\d+)\]", s)
            if m:
                x = (int(m.group(1)) + int(m.group(3))) // 2
                y = (int(m.group(2)) + int(m.group(4))) // 2
                adb_ok("shell", "input", "tap", str(x), str(y))
                print(f"[gates] tapped {pattern!r} @ ({x},{y})")
                return True
    return False


def dismiss_gates():
    """The runtime-permission gate ("Choose what to allow Fortnite to access")
    blocks GameActivity before libUE4 ever loads. The gate is a real system
    view, so uiautomator sees its buttons — tap the exact node centers."""
    for i in range(10):
        time.sleep(4)
        f = adb("shell", "dumpsys", "window")
        focus = next((l.strip()[:96] for l in (f.stdout or "").splitlines() if "mCurrentFocus" in l), "")
        print(f"[gates] {i}: {focus}")
        low = focus.lower()
        if any(k in low for k in ("permissioncontroller", "grantpermissions", "packageinstaller", "installer")):
            _ui_tap(r'text="Continue"') or _ui_tap(r'text="ALLOW"') or _ui_tap(r'text="Allow"') or _ui_tap(r'resource-id="android:id/button1"')
        m2 = adb("shell", "pidof", PKG)
        if m2.stdout.strip():
            return


def main():
    os.makedirs(OUT, exist_ok=True)
    apk = sys.argv[1] if len(sys.argv) > 1 else "/tmp/fortnite-12.41-owen.apk"
    with_ogs = os.environ.get("WITH_OGS", "0") == "1"

    # 0) the server + reverse tunnel
    health = http("/")
    print(f"[srv] {health[:180]}")
    assert '"owen-server"' in health, "owen-server is not up on :8080"
    adb_ok("reverse", f"tcp:{PORT}", f"tcp:{PORT}")
    print(f"[srv] adb reverse tcp:{PORT} in place")

    # continuous logcat
    log = open(f"{OUT}/logcat.txt", "w")
    subprocess.Popen(["adb", "logcat"], stdout=log, stderr=subprocess.DEVNULL)

    # 1) install + config
    r = adb("install", "-r", "-t", apk, timeout=600)
    print(f"[install] rc={r.returncode} {r.stdout.strip()[-200:]} {r.stderr.strip()[-200:]}")
    if r.returncode != 0:
        adb_ok("uninstall", PKG)
        r = adb("install", "-t", apk, timeout=600)
        assert r.returncode == 0, "APK install failed"
    push_config()

    # 2) launch + periodic screenshots in the background
    adb_ok("shell", "monkey", "-p", PKG, "-c", "android.intent.category.LAUNCHER", "1")
    dismiss_gates()
    import threading

    stop_at = time.time() + DOWNLOAD_TIMEOUT + LOGIN_TIMEOUT + 1200
    t = threading.Thread(target=periodic_shots, args=(stop_at,), daemon=True)
    t.start()

    # 3) content download (real CDN through the redirect layer)
    wait_download(DOWNLOAD_TIMEOUT)

    # 4) login → lobby
    status = {"apk": apk, "with_ogs": with_ogs}
    if do_login(LOGIN_TIMEOUT):
        time.sleep(75)  # frontend settle
        shot("lobby.png")
        status["lobby"] = os.path.exists(f"{OUT}/lobby.png")
    else:
        status["lobby"] = False

    # 5) the match (libOwenOGS opens Apollo_Terrain after world_open_delay)
    if with_ogs and status.get("lobby"):
        print(f"[match] waiting for 'OGS: match world up' (≤{MATCH_TIMEOUT}s)…")
        deadline = time.time() + MATCH_TIMEOUT
        while time.time() < deadline:
            if logcat_grep("OGS: match world up"):
                time.sleep(45)  # let the world render
                shot("match.png")
                status["match"] = os.path.exists(f"{OUT}/match.png")
                break
            time.sleep(15)
        else:
            status["match"] = False
    else:
        status["match"] = False
        if not with_ogs:
            print("[match] skipped (libOwenOGS.so not in this build — reported honestly)")

    # 6) evidence pack
    adb_ok("logcat", "-d", timeout=120)
    with open(f"{OUT}/status.json", "w") as f:
        json.dump(status, f, indent=2)
    print(f"[done] {json.dumps(status)}")
    sys.exit(0 if status.get("lobby") else 1)


if __name__ == "__main__":
    if "--gates-only" in sys.argv:
        # dump.sh uses this: bash's set -e traps made the shell version exit
        # early; python is deterministic.
        import time as _t
    # minimal bootstrap: gates need no server
    if "--gates-only" in sys.argv:
        dismiss_gates()
        sys.exit(0)
    main()
