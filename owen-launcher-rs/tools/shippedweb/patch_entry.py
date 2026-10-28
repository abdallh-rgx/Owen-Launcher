#!/usr/bin/env python3
"""
Owen Launcher - shippedweb offline entry patcher.

Patches a copy of Epic's "shippedweb" offline entry (the Kairos login page,
shipped at  assets/shippedweb/offline/entry/  inside Fortnite APKs) so that:

  1. The GraphQL endpoints point at YOUR backend (window.clientEnvConfig,
     injected before main.kairos.*.js - both http:// and https:// are kept
     verbatim, nothing is rewritten).
  2. The page no longer freezes on the Epic Games logo (the built-in mock
     bridge answers getstartupstatus with a dead-end status; owen-bridge.js
     answers it correctly and drives the login flow).
  3. On load the bundle performs its own identity GraphQL query
     (query {Account { myAccount {id}}}) against EPIC_SERVICE_GRAPHQL_URL,
     so your backend receives the request.

Usage:
    python3 patch_entry.py <shippedweb_dir> [--graphql URL] [--friends URL]
                           [--presence URL] [--party URL] [--out DIR]

    shippedweb_dir   folder that contains offline/entry/index.html
    --graphql        GraphQL endpoint (default /graphql, relative to the
                     server that serves the entry). Both http:// and
                     https:// absolute URLs are supported and preserved.
    --out            write the patched copy to DIR (default: <dir>-owen)

Example:
    python3 patch_entry.py ./shippedweb --graphql http://192.168.1.10:8090/graphql

After patching, serve the OUTPUT directory (not the original) with any static
server that also proxies/handles the GraphQL URL, then open:
    http://<host>:<port>/offline/en-US/
"""

import argparse
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BRIDGE_SRC = os.path.join(HERE, "owen-bridge.js")

ENTRY_REL = os.path.join("offline", "entry")
INDEX = os.path.join(ENTRY_REL, "index.html")
BRIDGE_NAME = "owen-bridge.js"


def fail(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(1)


def load(path: str) -> str:
    with open(path, "r", encoding="utf-8") as fh:
        return fh.read()


def save(path: str, content: str) -> None:
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(content)


def main() -> None:
    ap = argparse.ArgumentParser(description="Patch the shippedweb offline entry for a custom backend.")
    ap.add_argument("src", help="shippedweb folder (contains offline/entry/index.html)")
    ap.add_argument("--graphql", default="/graphql", help="GraphQL endpoint URL (verbatim http/https)")
    ap.add_argument("--friends", default=None, help="Friends service URL (defaults to --graphql)")
    ap.add_argument("--presence", default=None, help="Presence service URL (defaults to --graphql)")
    ap.add_argument("--party", default=None, help="Party service URL (defaults to --graphql)")
    ap.add_argument("--out", default=None, help="output directory (default <src>-owen)")
    args = ap.parse_args()

    src = os.path.abspath(args.src)
    index_path = os.path.join(src, INDEX)
    if not os.path.isfile(index_path):
        fail(f"{index_path} not found - pass the folder that contains offline/entry/index.html")

    out = os.path.abspath(args.out) if args.out else src.rstrip("/\\") + "-owen"
    if os.path.abspath(out) == src:
        fail("output directory must differ from source")

    if os.path.isdir(out):
        shutil.rmtree(out)
    shutil.copytree(src, out)

    friends = args.friends or args.graphql
    presence = args.presence or args.graphql
    party = args.party or args.graphql

    bridge_js = load(BRIDGE_SRC)
    # Embed the chosen endpoints into the bridge (keeps a single source of truth).
    bridge_js = bridge_js.replace('EPIC_SERVICE_GRAPHQL_URL: "/graphql"', f'EPIC_SERVICE_GRAPHQL_URL: "{args.graphql}"')
    bridge_js = bridge_js.replace('EPIC_SERVICE_FRIENDS_URL: "/graphql"', f'EPIC_SERVICE_FRIENDS_URL: "{friends}"')
    bridge_js = bridge_js.replace('EPIC_SERVICE_PRESENCE_URL: "/graphql"', f'EPIC_SERVICE_PRESENCE_URL: "{presence}"')
    bridge_js = bridge_js.replace('EPIC_SERVICE_PARTY_URL: "/graphql"', f'EPIC_SERVICE_PARTY_URL: "{party}"')
    save(os.path.join(out, ENTRY_REL, BRIDGE_NAME), bridge_js)

    html = load(os.path.join(out, INDEX))

    if "owen-bridge.js" in html:
        print("index.html already patched - refreshing script tag only")
        html = re.sub(r"<script src=\"\./owen-bridge\.js\"></script>\n?", "", html)

    # Remove any previously hand-injected clientEnvConfig blocks (older attempts).
    html = re.sub(
        r"<script>\s*window\.clientEnvConfig[\s\S]*?</script>\n?",
        "",
        html,
        count=1,
    )

    tag = '<script src="./owen-bridge.js"></script>\n'
    marker = '<script type="text/javascript" src="./polyfill'
    idx = html.find(marker)
    if idx == -1:
        # Fallback: inject right before the first main.kairos script or </head>.
        idx = html.find('<script type="text/javascript" src="./main')
        if idx == -1:
            idx = html.find("</head>")
    if idx == -1:
        fail("could not find an injection point in index.html")
    html = html[:idx] + tag + html[idx:]

    save(os.path.join(out, INDEX), html)

    print(f"patched copy written to: {out}")
    print(f"  graphql endpoint : {args.graphql}")
    print(f"  friends endpoint : {friends}")
    print(f"  presence endpoint: {presence}")
    print(f"  party endpoint   : {party}")
    print("serve THAT folder and open: http://<host>:<port>/offline/en-US/")


if __name__ == "__main__":
    main()
