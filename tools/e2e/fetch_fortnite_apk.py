#!/usr/bin/env python3
"""fetch_fortnite_apk.py — download a Fortnite Android APK from uptodown.

Given `--uptodown-id` (or `--version` / `--season`, resolved against
`owen-launcher-rs/crates/owen-launcher/src/core/catalogs/version_catalog.rs`),
resolve the *direct* APK URL from the uptodown download page and stream it to
`--out`, mirroring the exact resolution rules of the on-device launcher
(`owen-launcher/src/uptodown/resolver.rs`, the 1:1 port of
`UptodownResolver.kt`):

  1. GET https://fortnite.en.uptodown.com/android/download/<id> with the
     launcher's mobile User-Agent (Android 13 / Pixel 7 / Chrome 120).
  2. `#detail-download-button-xapk` → its `data-url` counts only when it
     contains `dw.uptodown.com`; if it is already a
     `https://dw.uptodown.com/dwn/…` URL use it directly, otherwise follow it
     (Referer = the download page, like the WebView) and take the landing URL
     or the first `dw.uptodown.com/dwn/` URL embedded in the interstitial.
  3. `#detail-download-button` → same treatment.
  4. Fallback: the first `https?://dw.uptodown.com/dwn/…` URL found anywhere
     in the download-page HTML (what the WebView download listener catches).

The file is streamed with progress output and a hard size cap (default 3 GiB),
then validated as a ZIP/APK (PK\\x03\\x04 magic).

Resolution strategies (`--strategy`, default `auto`):

  http   the plain-HTTP rules above. Works whenever the page still embeds a
         `data-url` / `dw.uptodown.com/dwn/…` link server-side.
  chrome if `http` finds nothing (the current situation: uptodown moved the
         direct link behind a Cloudflare Turnstile token +
         `POST /ajax/app/<app>/file/<id>/download-url`), launch the
         preinstalled headless Chrome over CDP, load the download page, click
         `#detail-download-button`, let Turnstile auto-solve, and capture the
         `https://dw.uptodown.com/dwn/…` request. Needs `pip install
         websocket-client` (pure-python) and a Chrome/Chromium binary
         (`CHROME_BIN` overrides autodetection).
  auto   try `http`, then `chrome` (the CI default).

Only the Python stdlib is used for the `http` strategy (safe for CI — no
pip install); the optional `chrome` strategy additionally needs the tiny
pure-python `websocket-client` package plus a Chrome/Chromium binary.

Exit codes:
    0  success (valid APK written to --out)
    2  usage error
    3  version/season lookup failed (not in version_catalog.rs, or no
       uptodown_id)
    4  could not resolve a direct dw.uptodown.com URL
    5  download failed (HTTP error, interrupted, over the size cap, or the
       overall deadline passed)
    6  downloaded file is not a ZIP/APK

On any of 4/5/6 the marker line `FORTNITE_APK_DOWNLOAD_FAILED: <reason>` is
printed to stderr — the android-e2e workflow surfaces it as the step failure
message (the user wants real failures, never silent skips).

Examples:
    # by uptodown id (what the E2E matrix passes):
    python3 tools/e2e/fetch_fortnite_apk.py \\
        --uptodown-id 2249844 --out fortnite-12.61.apk

    # by version string (highest CL build of that version in the catalog):
    python3 tools/e2e/fetch_fortnite_apk.py --version 12.61 --out fn.apk

    # by season number (latest version of that season):
    python3 tools/e2e/fetch_fortnite_apk.py --season 12 --out fn.apk
"""

from __future__ import annotations

import argparse
import base64
import gzip
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import zlib

# ---------------------------------------------------------------------------
# constants — mirror owen-launcher/src/uptodown/{api,resolver}.rs verbatim
# ---------------------------------------------------------------------------

HOST = "https://fortnite.en.uptodown.com"
UA = (
    "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36"
)
DIRECT_PREFIXES = ("https://dw.uptodown.com/dwn/", "http://dw.uptodown.com/dwn/")
DIRECT_HOST = "dw.uptodown.com"
XAPK_BUTTON_ID = "detail-download-button-xapk"
DOWNLOAD_BUTTON_ID = "detail-download-button"

CHUNK = 1024 * 1024                # 1 MiB streaming chunks
PROGRESS_EVERY = 50 * 1024 * 1024  # print every 50 MiB
DEFAULT_MAX_BYTES = 3 * 1024 ** 3  # 3 GiB hard cap (task requirement)
DEFAULT_DEADLINE = 15 * 60         # 15 minutes (task requirement)

# tools/e2e/fetch_fortnite_apk.py → <repo>/owen-launcher-rs/crates/…
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.dirname(os.path.dirname(_SCRIPT_DIR))
DEFAULT_CATALOG = os.path.join(
    _REPO_ROOT, "owen-launcher-rs", "crates", "owen-launcher", "src",
    "core", "catalogs", "version_catalog.rs",
)


# ---------------------------------------------------------------------------
# version_catalog.rs parsing (regex, no Rust toolchain needed)
# ---------------------------------------------------------------------------

_ENTRY_RE = re.compile(r"OgVersion\s*\{([^}]*)\}", re.S)
_FIELD_RES = {
    "season": re.compile(r"\bseason:\s*(\d+)"),
    "version": re.compile(r'\bversion:\s*"([^"]+)"'),
    "cl": re.compile(r"\bcl:\s*(-?\d+)"),
    "uptodown_id": re.compile(r"\buptodown_id:\s*Some\((\d+)\)"),
    "is_dev": re.compile(r"\bis_dev:\s*(true|false)"),
}


def parse_catalog(path: str):
    """Parse the `OgVersion { … }` entries out of version_catalog.rs."""
    try:
        with open(path, "r", encoding="utf-8") as fh:
            src = fh.read()
    except OSError as exc:
        die(3, f"cannot read version catalog {path}: {exc}")
    entries = []
    for match in _ENTRY_RE.finditer(src):
        blob = match.group(1)
        entry = {}
        for name, rx in _FIELD_RES.items():
            m = rx.search(blob)
            if not m:
                continue
            if name in ("season", "cl", "uptodown_id"):
                entry[name] = int(m.group(1))
            elif name == "is_dev":
                entry[name] = m.group(1) == "true"
            else:
                entry[name] = m.group(1)
        # is_dev defaults to false in the struct; only keep real releases.
        if "version" in entry and "season" in entry and not entry.get("is_dev"):
            entries.append(entry)
    if not entries:
        die(3, f"no OgVersion entries parsed from {path} (regex drift?)")
    return entries


def resolve_id(args: argparse.Namespace, entries) -> tuple[int, str]:
    """--uptodown-id / --version / --season → (uptodown_id, version)."""
    if args.uptodown_id:
        return args.uptodown_id, args.version or f"id{args.uptodown_id}"
    if args.version:
        matches = [e for e in entries if e.get("version") == args.version]
        if not matches:
            available = sorted({e["version"] for e in entries})
            die(3, f"version {args.version!r} not in version_catalog.rs; "
                   f"available: {', '.join(available)}")
        best = max(matches, key=lambda e: (e.get("cl", 0), e.get("uptodown_id", 0) or 0))
        if not best.get("uptodown_id"):
            die(3, f"version {args.version!r} has no uptodown_id in the catalog")
        return best["uptodown_id"], best["version"]
    if args.season is not None:
        matches = [e for e in entries if e.get("season") == args.season]
        if not matches:
            seasons = sorted({e["season"] for e in entries})
            die(3, f"season {args.season} not in version_catalog.rs; "
                   f"available seasons: {seasons}")
        best = max(matches, key=lambda e: (e.get("cl", 0), e.get("uptodown_id", 0) or 0))
        if not best.get("uptodown_id"):
            die(3, f"latest of season {args.season} ({best['version']}) has no uptodown_id")
        return best["uptodown_id"], best["version"]
    die(2, "one of --uptodown-id / --version / --season is required")


# ---------------------------------------------------------------------------
# HTTP helpers (stdlib only)
# ---------------------------------------------------------------------------

class Fetcher:
    """Tiny urllib wrapper with a wall-clock deadline and the launcher UA."""

    def __init__(self, deadline: float):
        self.deadline = deadline

    def _left(self) -> float:
        return self.deadline - time.monotonic()

    def get(self, url: str, referer: str | None = None) -> tuple[str, bytes]:
        """GET → (final_url, body). Follows redirects like the WebView."""
        if self._left() <= 0:
            raise TimeoutError("deadline passed")
        headers = {
            "User-Agent": UA,
            "Accept": "text/html,application/xhtml+xml,*/*;q=0.8",
            "Accept-Encoding": "identity",
        }
        if referer:
            headers["Referer"] = referer
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=30) as resp:  # nosec: fixed https host
            final_url = resp.geturl()
            raw = resp.read()
            enc = (resp.headers.get("Content-Encoding") or "").lower()
            if "gzip" in enc:
                raw = gzip.decompress(raw)
            elif "deflate" in enc:
                raw = zlib.decompress(raw)
            return final_url, raw

    def stream_to_file(self, url: str, referer: str, out_path: str,
                       max_bytes: int) -> int:
        """Stream `url` to `out_path`; returns the number of bytes written."""
        if self._left() <= 0:
            raise TimeoutError("deadline passed")
        headers = {
            "User-Agent": UA,
            "Referer": referer,
            "Accept": "*/*",
            "Accept-Encoding": "identity",
        }
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=60) as resp, \
                open(out_path, "wb") as fh:  # nosec: fixed https host
            declared = int(resp.headers.get("Content-Length") or 0)
            if declared and declared > max_bytes:
                raise OverflowError(
                    f"server declares {declared} bytes, over the {max_bytes}-byte cap")
            done = 0
            next_mark = PROGRESS_EVERY
            t0 = time.monotonic()
            while True:
                if self._left() <= 0:
                    raise TimeoutError("deadline passed")
                chunk = resp.read(CHUNK)
                if not chunk:
                    break
                done += len(chunk)
                if done > max_bytes:
                    raise OverflowError(f"over the {max_bytes}-byte cap")
                fh.write(chunk)
                if done >= next_mark:
                    rate = done / max(1e-9, time.monotonic() - t0)
                    print(f"  … {done / 1048576:8.0f} MiB "
                          f"({rate / 1048576:5.1f} MiB/s)", flush=True)
                    next_mark += PROGRESS_EVERY
            return done


# ---------------------------------------------------------------------------
# URL resolution — a 1:1 python port of UptodownResolver (resolver.rs)
# ---------------------------------------------------------------------------

class ResolveError(Exception):
    """No direct dw.uptodown.com/dwn/ URL could be obtained."""


def decode_entities(value: str) -> str:
    return value.replace("&amp;", "&")


def extract_attr(tag: str, attr: str) -> str | None:
    """`attr="v"` / `attr='v'` / `attr=v` from one tag (extract_attr in rs)."""
    for quote in ('"', "'"):
        literal = f"{attr}={quote}"
        pos = tag.find(literal)
        if pos >= 0:
            start = pos + len(literal)
            end = tag.find(quote, start)
            if end < 0:
                end = len(tag)
            return decode_entities(tag[start:end])
    literal = f"{attr}="
    pos = tag.find(literal)
    if pos >= 0:
        start = pos + len(literal)
        m = re.compile(r"\s").search(tag, start)
        end = m.start() if m else len(tag)
        if end > start:
            return decode_entities(tag[start:end])
    return None


def find_button_data_url(html: str, button_id: str) -> str | None:
    """getElementById(id) + its data-url (find_button_data_url in rs)."""
    positions = [p for p in (html.find(f'id="{button_id}"'),
                             html.find(f"id='{button_id}'")) if p >= 0]
    if not positions:
        return None
    id_pos = min(positions)
    tag_start = html.rfind("<", 0, id_pos)
    if tag_start < 0:
        return None
    close = html.find(">", id_pos)
    if close < 0:
        return None
    return extract_attr(html[tag_start:close], "data-url")


def find_direct_url(text: str) -> str | None:
    """First dw.uptodown.com/dwn/ URL, terminated by ws/quote/angle (rs rule)."""
    for prefix in DIRECT_PREFIXES:
        pos = text.find(prefix)
        if pos >= 0:
            rest = text[pos:]
            m = re.compile(r'[\s"\'<>]').search(rest)
            end = m.start() if m else len(rest)
            return decode_entities(rest[:end])
    return None


def is_direct(url: str) -> bool:
    return url.startswith(DIRECT_PREFIXES)


def resolve_direct_url(fetch: Fetcher, version_id: int) -> str:
    """resolve_direct_url(http, versionId) from resolver.rs, same order."""
    page_url = f"{HOST}/android/download/{version_id}"
    print(f"[resolve] download page: {page_url}", flush=True)
    _, raw = fetch.get(page_url)
    html = raw.decode("utf-8", "replace")
    if not html:
        die(4, f"download page {page_url} returned an empty body")

    # CLICK_SCRIPT step 1 (xpak button), then step 2 (regular button).
    for button_id in (XAPK_BUTTON_ID, DOWNLOAD_BUTTON_ID):
        url = find_button_data_url(html, button_id)
        if url is None:
            continue
        print(f"[resolve] {button_id}: data-url={url[:100]}", flush=True)
        if DIRECT_HOST in url:
            if is_direct(url):
                return url
            # location.href = u → follow it (Referer = the download page).
            print(f"[resolve] following {button_id} data-url …", flush=True)
            landed, body = fetch.get(url, referer=page_url)
            if is_direct(landed):
                return landed
            embedded = find_direct_url(body.decode("utf-8", "replace"))
            if embedded:
                return embedded

    # Fallback: any dw.uptodown.com/dwn/ URL the page itself carries.
    embedded = find_direct_url(html)
    if embedded:
        print("[resolve] fallback: embedded dw.uptodown.com/dwn/ URL", flush=True)
        return embedded
    raise ResolveError(
        f"no dw.uptodown.com/dwn/ URL found on {page_url} — uptodown now gates "
        f"the direct link behind a Cloudflare Turnstile token + "
        f"POST /ajax/app/<id>/file/<fid>/download-url; use --strategy chrome "
        f"(headless Chrome over CDP) or --strategy auto")


# ---------------------------------------------------------------------------
# chrome strategy — headless Chrome over CDP (Turnstile + download-url AJAX)
# ---------------------------------------------------------------------------
# The current uptodown flow (download.js): clicking #detail-download-button
# solves an invisible Cloudflare Turnstile, then POSTs to
#   /ajax/app/<appID>/file/<fileID>/download-url   {token, onlyXapk}
# which returns {data:{downloadURL}}; the file lives at
#   https://dw.uptodown.com/dwn/<downloadURL>.
# We drive a real (headless) Chrome over CDP so the Turnstile solves itself,
# click the button, and read the AJAX response body.

CHROME_CANDIDATES = ("google-chrome", "google-chrome-stable", "chromium",
                     "chromium-browser", "chrome")
# A plain desktop UA: Turnstile is far friendlier to "normal Chrome" than to
# the default HeadlessChrome string (the file GET afterwards uses `UA`).
CHROME_UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
             "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _find_chrome() -> str | None:
    cand = os.environ.get("CHROME_BIN")
    if cand and os.path.isfile(cand):
        return cand
    for name in CHROME_CANDIDATES:
        found = shutil.which(name)
        if found:
            return found
    return None


def _cdp_http(port: int, path: str) -> bytes:
    with urllib.request.urlopen(
        f"http://127.0.0.1:{port}{path}", timeout=10) as resp:  # nosec: local
        return resp.read()


def resolve_via_chrome(version_id: int, deadline: float) -> str:
    """Resolve the direct URL with headless Chrome + CDP. Raises ResolveError."""
    try:
        import websocket  # websocket-client (pure python)
    except ImportError:
        raise ResolveError(
            "chrome strategy unavailable: `pip install websocket-client` "
            "is required") from None

    chrome = _find_chrome()
    if not chrome:
        raise ResolveError(
            "chrome strategy unavailable: no Chrome/Chromium binary found "
            "(install one or set CHROME_BIN)")

    port = _free_port()
    user_data = tempfile.mkdtemp(prefix="owen-fetch-chrome-")
    page_url = f"{HOST}/android/download/{version_id}"
    args = [
        chrome, "--headless=new", "--no-sandbox", "--disable-gpu",
        "--disable-dev-shm-usage", "--disable-background-networking",
        f"--remote-debugging-port={port}", f"--user-agent={CHROME_UA}",
        f"--user-data-dir={user_data}", "about:blank",
    ]
    print(f"[chrome] {chrome} --remote-debugging-port={port}", flush=True)
    proc = subprocess.Popen(args, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    ws = None
    try:
        # wait for the DevTools endpoint
        ws_url = None
        while time.monotonic() < deadline:
            try:
                targets = json.loads(_cdp_http(port, "/json"))
                pages = [t for t in targets if t.get("type") == "page"]
                if pages:
                    ws_url = pages[0]["webSocketDebuggerUrl"]
                    break
            except (urllib.error.URLError, OSError, ValueError):
                pass
            time.sleep(0.5)
        if not ws_url:
            raise ResolveError("chrome DevTools endpoint never came up")

        ws = websocket.create_connection(ws_url, timeout=10)
        ws.settimeout(0.5)
        seq = 0

        def cmd(method: str, params: dict | None = None) -> int:
            nonlocal seq
            seq += 1
            ws.send(json.dumps({"id": seq, "method": method,
                                "params": params or {}}))
            return seq

        ajax_ids: set[str] = set()           # responseReceived ids of interest
        direct: str | None = None
        clicked = False

        cmd("Network.enable")
        cmd("Page.enable")
        cmd("Page.navigate", {"url": page_url})
        print(f"[chrome] navigating: {page_url}", flush=True)

        while time.monotonic() < deadline:
            try:
                raw = ws.recv()
            except Exception:
                continue  # timeout tick — loop back to the deadline check
            if not raw:
                raise ResolveError("chrome DevTools socket closed")
            evt = json.loads(raw)
            method = evt.get("method", "")

            if method == "Page.loadEventFired" and not clicked:
                # give Turnstile scripts a beat, then click the download button
                time.sleep(2.0)
                cmd("Runtime.evaluate", {
                    "expression":
                        "(function(){var b=document.getElementById("
                        "'detail-download-button');"
                        "return b? (b.click(), 'clicked') : 'missing';})()",
                    "returnByValue": True})
                clicked = True
                print("[chrome] #detail-download-button clicked — waiting for "
                      "Turnstile + /download-url …", flush=True)

            elif method == "Network.requestWillBeSent":
                url = evt["params"].get("request", {}).get("url", "")
                if url.startswith(DIRECT_PREFIXES):
                    direct = url  # the browser itself hit the file URL

            elif method == "Network.responseReceived":
                url = evt["params"].get("response", {}).get("url", "")
                if "/download-url" in url:
                    ajax_ids.add(evt["params"]["requestId"])

            elif method == "Network.loadingFinished":
                rid = evt["params"]["requestId"]
                if rid in ajax_ids:
                    ajax_ids.discard(rid)
                    cmd("Network.getResponseBody", {"requestId": rid})
                    # the body arrives as a command response next tick

            if "id" in evt and "result" in evt:
                body_b64 = evt["result"].get("body")
                if body_b64:
                    body = base64.b64decode(body_b64).decode("utf-8", "replace")
                    m = re.search(r'"downloadURL"\s*:\s*"([^"]+)"', body)
                    if m:
                        direct = f"https://dw.uptodown.com/dwn/{m.group(1)}"

            if direct:
                print(f"[chrome] resolved: {direct[:100]}", flush=True)
                return direct

        raise ResolveError(
            "headless Chrome did not obtain a dw.uptodown.com/dwn/ URL in "
            "time (Turnstile refused to auto-solve or the page changed)")
    finally:
        if ws is not None:
            try:
                ws.close()
            except Exception:
                pass
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
        shutil.rmtree(user_data, ignore_errors=True)


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def die(code: int, reason: str):
    if code in (4, 5, 6):
        print(f"FORTNITE_APK_DOWNLOAD_FAILED: {reason}", file=sys.stderr)
    print(f"error: {reason}", file=sys.stderr)
    sys.exit(code)


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Fetch a Fortnite Android APK from uptodown "
                    "(mirrors UptodownResolver.kt / resolver.rs).")
    src = ap.add_mutually_exclusive_group()
    src.add_argument("--uptodown-id", type=int, default=None,
                     help="uptodown version id (version_catalog.rs uptodown_id)")
    src.add_argument("--version", default=None,
                     help="Fortnite version string, e.g. 12.61 (highest CL build)")
    src.add_argument("--season", type=int, default=None,
                     help="season number — latest version of that season")
    ap.add_argument("--out", required=True, help="output APK path")
    ap.add_argument("--strategy", choices=("auto", "http", "chrome"),
                    default="auto",
                    help="URL resolution strategy (default: auto = http then "
                         "headless-Chrome for the Turnstile flow)")
    ap.add_argument("--catalog", default=DEFAULT_CATALOG,
                    help="version_catalog.rs path (default: auto-detected)")
    ap.add_argument("--timeout-secs", type=int, default=DEFAULT_DEADLINE,
                    help=f"overall deadline in seconds (default {DEFAULT_DEADLINE})")
    ap.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES,
                    help=f"size cap in bytes (default {DEFAULT_MAX_BYTES})")
    args = ap.parse_args()

    if args.timeout_secs <= 0:
        die(2, "--timeout-secs must be positive")
    if args.max_bytes <= 0:
        die(2, "--max-bytes must be positive")
    if not os.path.isdir(os.path.dirname(os.path.abspath(args.out)) or "."):
        die(2, f"output directory of --out does not exist: {args.out}")

    entries = parse_catalog(args.catalog)
    version_id, version = resolve_id(args, entries)
    print(f"target: Fortnite {version} (uptodown id {version_id}) — "
          f"catalog {args.catalog} ({len(entries)} releases)", flush=True)

    fetch = Fetcher(time.monotonic() + args.timeout_secs)
    page_url = f"{HOST}/android/download/{version_id}"

    def _http_resolve() -> str:
        try:
            return resolve_direct_url(fetch, version_id)
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise ResolveError(f"resolution request failed: {exc}") from None

    try:
        if args.strategy == "chrome":
            direct = resolve_via_chrome(version_id,
                                        time.monotonic() + args.timeout_secs)
        elif args.strategy == "http":
            direct = _http_resolve()
        else:  # auto: plain HTTP first, headless Chrome if the page is empty
            try:
                direct = _http_resolve()
            except ResolveError as exc:
                print(f"[resolve] http strategy failed: {exc}",
                      file=sys.stderr, flush=True)
                print("[resolve] falling back to headless Chrome "
                      "(Turnstile + /download-url AJAX)…", flush=True)
                direct = resolve_via_chrome(
                    version_id, time.monotonic() + args.timeout_secs)
    except ResolveError as exc:
        die(4, str(exc))
    print(f"direct URL: {direct}", flush=True)
    tmp = args.out + ".part"
    try:
        size = fetch.stream_to_file(direct, referer=page_url, out_path=tmp,
                                    max_bytes=args.max_bytes)
    except TimeoutError as exc:
        die(5, f"download did not finish within {args.timeout_secs}s: {exc}")
    except OverflowError as exc:
        die(5, str(exc))
    except (urllib.error.URLError, OSError) as exc:
        die(5, f"download failed: {exc}")

    with open(tmp, "rb") as fh:
        magic = fh.read(4)
    if magic != b"PK\x03\x04":
        try:
            os.remove(tmp)
        except OSError:
            pass
        die(6, f"{tmp} is not a ZIP/APK (magic {magic!r}); "
             "the resolved URL probably served an HTML error page")

    os.replace(tmp, args.out)
    print(f"OK: {args.out} ({size} bytes = {size / 1048576:.0f} MiB) "
          f"for Fortnite {version}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
