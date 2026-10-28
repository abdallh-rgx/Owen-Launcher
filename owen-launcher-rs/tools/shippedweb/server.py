#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
server.py — Owen PartyHub phone server (v2, Voltronite edition)
================================================================

ZERO DEPENDENCIES — pure Python 3 standard library. Runs on Termux,
Pydroid 3, or any desktop Python 3.7+.

WHAT IT DOES
------------
  1. Serves the patched Fortnite PartyHub web build (offline/entry) from its
     own folder — you can run this script from anywhere.
  2. SPA fallback: unknown extension-less routes -> offline/entry/index.html,
     and relative assets from nested routes (/offline/en-US/main.js) resolve
     to offline/entry/main.js.
  3. POST /owen-graphql -> the VOLTRONITE RELAY:
       - reads the Voltronite address from the X-Owen-Backend header (sent by
         owen-bridge.js — the URL lives in window.OWEN_CONFIG.VOLTRONITE_URL
         inside offline/entry/owen-bridge.js)
       - obtains a real access token from Voltronite
         (POST /account/api/oauth/token, grant_type=password, cached per
         user, refreshed once on 401)
       - forwards {operationName, query, variables} + Bearer token to
         <backend>/partyhub/graphql and relays the JSON answer
       - server-side forwarding is REQUIRED because Voltronite sends no CORS
         headers, so a browser page can never call it directly.

RUN
---
    python server.py             # http://127.0.0.1:8081 -> /offline/en-US/
    PORT=9000 python server.py   # custom port

Then open http://127.0.0.1:8081 in the browser (Chrome on the phone).
"""

import functools
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

# ---------------------------------------------------------------------------
# configuration (the Voltronite address itself lives in owen-bridge.js)
# ---------------------------------------------------------------------------
PORT = int(os.environ.get("PORT", "8081"))
DEFAULT_BACKEND = os.environ.get("OWEN_BACKEND", "http://127.0.0.1:3551")
DEFAULT_USER = os.environ.get("OWEN_USER", "OwenTester")
ROOT = Path(__file__).resolve().parent  # the shippedweb folder
ENTRY = ROOT / "offline" / "entry"
ENTRY_URL = "/offline/en-US/"           # where the app lives
ENTRY_INDEX_URL = "/offline/entry/index.html"

# ---------------------------------------------------------------------------
# Voltronite relay: token cache + forward
# ---------------------------------------------------------------------------
_tokens = {}  # (backend, user) -> access_token


def voltronite_token(backend, user, force=False):
    key = (backend, user)
    if not force and key in _tokens:
        return _tokens[key]
    data = ("grant_type=password&username=" + urllib.parse.quote(user)).encode()
    req = urllib.request.Request(
        backend.rstrip("/") + "/account/api/oauth/token",
        data=data,
        headers={"content-type": "application/x-www-form-urlencoded"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=15) as res:
        payload = json.loads(res.read().decode("utf-8"))
    token = payload.get("access_token")
    if not token:
        raise RuntimeError("oauth response has no access_token")
    _tokens[key] = token
    return token


def voltronite_forward(backend, user, body):
    """Forward one graphql body to <backend>/partyhub/graphql (refresh once on 401)."""
    token = voltronite_token(backend, user)
    for attempt in (0, 1):
        req = urllib.request.Request(
            backend.rstrip("/") + "/partyhub/graphql",
            data=json.dumps(body).encode("utf-8"),
            headers={
                "content-type": "application/json",
                "authorization": "Bearer " + token,
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=20) as res:
                return json.loads(res.read().decode("utf-8"))
        except urllib.error.HTTPError as err:
            if err.code == 401 and attempt == 0:
                token = voltronite_token(backend, user, force=True)
                continue
            detail = err.read().decode("utf-8", "replace")[:300]
            raise RuntimeError("backend HTTP %d: %s" % (err.code, detail))
    raise RuntimeError("unreachable")


# ---------------------------------------------------------------------------
# handler
# ---------------------------------------------------------------------------
class OwenHandler(SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write("[%s] %s\n" % (self.log_date_time_string(), fmt % args))

    # ---- SPA fallback + relative-asset resolution -------------------------
    def send_head(self):
        url_path = self.path.split("?", 1)[0]
        fs_path = self.translate_path(url_path)
        if not os.path.isdir(fs_path) and not os.path.exists(fs_path):
            last = url_path.rstrip("/").rsplit("/", 1)[-1]
            if "." not in last:
                # unknown extension-less route -> the SPA entry page
                self.path = ENTRY_INDEX_URL
            elif (ENTRY / last).exists():
                # relative asset requested from a nested SPA route
                # (/offline/en-US/main.kairos.*.js -> offline/entry/main.kairos.*.js)
                self.path = "/offline/entry/" + last
        return super().send_head()

    # ---- root redirect ----------------------------------------------------
    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", ""):
            self.send_response(302)
            self.send_header("Location", ENTRY_URL)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        super().do_GET()

    # ---- CORS (harmless: everything here is same-origin) ------------------
    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Content-Length", "0")
        self.end_headers()

    # ---- the Voltronite relay ---------------------------------------------
    def do_POST(self):
        path = self.path.split("?", 1)[0]
        if not (
            path.endswith("/owen-graphql")
            or path == "/owen-graphql"
            or path.endswith("/graphql")
            or path == "/graphql"
        ):
            self.send_error(404)
            return

        try:
            length = int(self.headers.get("Content-Length") or 0)
            raw = self.rfile.read(length) if length else b"{}"
            body = json.loads(raw.decode("utf-8") or "{}")
        except Exception as err:
            self._relay_error(400, "bad json: %s" % err)
            return

        backend = (self.headers.get("X-Owen-Backend") or DEFAULT_BACKEND).strip()
        user = (self.headers.get("X-Owen-User") or DEFAULT_USER).strip()
        if not backend.lower().startswith(("http://", "https://")):
            self._relay_error(400, "bad backend url: %r" % backend)
            return

        op = body.get("operationName") or "?"
        try:
            answer = voltronite_forward(backend, user, body)
            self._relay_json(200, answer)
            print("  [RELAY] %-28s user=%-14s -> 200  (%s)" % (op, user, backend))
        except Exception as err:
            # 502 -> owen-bridge.js falls back to local answers so the UI
            # keeps rendering even when the backend is unreachable.
            self._relay_error(502, "%s" % err)
            print("  [RELAY] %-28s user=%-14s -> FAILED: %s" % (op, user, err))

    def _relay_json(self, status, obj):
        payload = json.dumps(obj).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def _relay_error(self, status, message):
        self._relay_json(status, {"errors": [{"message": str(message)}]})


if __name__ == "__main__":
    if not (ENTRY / "index.html").exists():
        print("ERROR: offline/entry/index.html not found next to server.py")
        sys.exit(1)

    handler = functools.partial(OwenHandler, directory=str(ROOT))
    url = "http://127.0.0.1:%d" % PORT
    print("=" * 62)
    print("  OWEN PARTYHUB  (Voltronite edition)")
    print("=" * 62)
    print("  Open in the browser : %s" % url)
    print("  App entry           : %s%s" % (url, ENTRY_URL))
    print("  Default backend     : %s" % DEFAULT_BACKEND)
    print("    (the backend URL is sent per request by owen-bridge.js --")
    print("     edit window.OWEN_CONFIG.VOLTRONITE_URL in")
    print("     offline/entry/owen-bridge.js to change it)")
    print("=" * 62)
    with ThreadingHTTPServer(("0.0.0.0", PORT), handler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nbye")
