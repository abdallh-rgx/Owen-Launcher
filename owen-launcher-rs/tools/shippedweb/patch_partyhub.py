#!/usr/bin/env python3
"""
patch_partyhub.py — turn the stock Fortnite "shippedweb" offline entry into a
standalone PartyHub web app that talks to a real backend (Voltronite).

What it does
------------
1. Copies the pristine shippedweb/offline/entry tree to --out.
2. Patches main.kairos.<hash>.js:
     webpack module 1041 (the app root / LoginContainer) is replaced with the
     hybrid root from partyhub-root.js: the original login flow until the
     startup status becomes "authenticated", then the FULL Kairos app
     (real router + AppEffects + useAppInitialization + AppStateInitializers).
3. Patches index.html:
     - strips previously injected debug/env scripts (idempotent),
     - sets window.appContext (route base — see --base),
     - injects clientEnvConfig (EPIC_SERVICE_GRAPHQL_URL = --graphql),
     - injects owen-bridge.js (v2) before the polyfill bundle,
     - keeps window.__epic_localization (UI strings) untouched.

Usage
-----
    python3 patch_partyhub.py --src <pristine-shippedweb> --out <patched-copy> \
        [--base /partyhub/offline] [--graphql ./graphql]

    --base   value for window.appContext. It is BOTH the route prefix the
             router matches (getPath()) and the URL prefix under which the
             page is served. Examples:
               "/partyhub/offline"  served behind the Owen web console proxy
                                    (https://host/partyhub/offline/en-US/)
               "offline"            served directly from the shippedweb root
                                    (https://host/offline/en-US/)
    --graphql  EPIC_SERVICE_GRAPHQL_URL. Relative "./graphql" works when the
               serving host answers the graphql adapter on every SPA route.

The patch is idempotent: running it twice produces the same output.
"""

import argparse
import re
import shutil
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent

# ---------------------------------------------------------------------------
# module 1041 replacement target
# ---------------------------------------------------------------------------

# The exact stock source of webpack module 1041 (the LoginContainer root) in
# main.kairos.8af1b828dc6188736f50.js. Function.prototype.toString is verbatim,
# so this substring occurs exactly once in the pristine bundle.
STOCK_ROOT_HEAD = (
    'function(e,t,n){"use strict";Object.defineProperty(t,"__esModule",{value:!0}),'
    't.default=void 0;var r=g(n(0)),i=v(n(5)),o=n(9),a=n(1060),s=n(259),u=n(54),'
    'l=g(n(414)),c=v(n(1269)),d=v(n(699)),f=v(n(702)),p=v(n(418));n(1275);'
)
STOCK_ROOT_TAIL = 't.default=b}'

# The stock tail of webpack module 170 (the GraphQlService singleton). In the
# online build the native bootstrap calls __GraphQlService.configure(config);
# this offline bundle never does, so every graphql query would POST to an empty
# URL (i.e. the page itself). The patch makes the module self-configure from
# window.clientEnvConfig.
STOCK_GQL_TAIL = (
    'i.default.__GraphQlService||(i.default.__GraphQlService=new c);'
    'var d=i.default.__GraphQlService;t.default=d'
)
PATCHED_GQL_TAIL = (
    'i.default.__GraphQlService||(i.default.__GraphQlService=new c);'
    'var d=i.default.__GraphQlService;'
    'try{d.configure(i.default.clientEnvConfig||{})}'
    'catch(owenErr){console.warn("[owen] GraphQlService.configure skipped:",'
    'owenErr&&owenErr.message)}'
    't.default=d'
)


def extract_stock_root(bundle: str) -> str:
    """Return the exact module-1041 source text from the bundle."""
    start = bundle.find(STOCK_ROOT_HEAD)
    if start < 0:
        raise SystemExit("stock module 1041 head not found (bundle already patched or a different build?)")
    end = bundle.find(STOCK_ROOT_TAIL, start)
    if end < 0:
        raise SystemExit("stock module 1041 tail not found")
    return bundle[start : end + len(STOCK_ROOT_TAIL)]


# ---------------------------------------------------------------------------
# index.html surgery
# ---------------------------------------------------------------------------

DEBUG_BLOCK_RE = re.compile(
    r"<script>\s*//?\s*OWEN.*?</script>", re.S | re.I
)
# previously-injected blocks (v1 patcher / user debug scripts) — removed so the
# patch is idempotent. Identified by the clientEnvConfig / owen-bridge markers.
OLD_ENV_BLOCK_RE = re.compile(
    r"<script>\s*window\.clientEnvConfig[\s\S]*?</script>", re.I
)
OLD_BRIDGE_TAG_RE = re.compile(
    r'<script[^>]*src="(?:\./)?owen-bridge\.js"[^>]*>\s*</script>', re.I
)
OLD_ARABIC_DEBUG_RE = re.compile(
    r"<script>\s*window\.addEventListener\('unhandledrejection'[\s\S]*?</script>", re.I
)
APPCONTEXT_RE = re.compile(r"window\.appContext\s*=\s*'[^']*';")


def patch_index_html(html: str, base: str, graphql: str) -> str:
    # strip any previously injected blocks (idempotency)
    html = OLD_BRIDGE_TAG_RE.sub("", html)
    html = OLD_ENV_BLOCK_RE.sub("", html)
    html = OLD_ARABIC_DEBUG_RE.sub("", html)

    # route base
    if APPCONTEXT_RE.search(html):
        html = APPCONTEXT_RE.sub(f"window.appContext = '{base}';", html)
    else:
        raise SystemExit("window.appContext assignment not found in index.html")

    # env + bridge, injected right before the polyfill script tag
    inject = (
        "<script>\n"
        "  window.clientEnvConfig = window.clientEnvConfig || {};\n"
        f"  window.clientEnvConfig.EPIC_SERVICE_GRAPHQL_URL = \"{graphql}\";\n"
        "  window.clientEnvConfig.EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS = false;\n"
        "</script>\n"
        '<script type="text/javascript" src="./owen-bridge.js"></script>\n'
    )
    m = re.search(r'<script[^>]*src="\./polyfill\.[^"]*"[^>]*>\s*</script>', html, re.I)
    if not m:
        raise SystemExit("polyfill script tag not found in index.html")
    html = html[: m.start()] + inject + html[m.start() :]
    return html


# ---------------------------------------------------------------------------

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True, help="pristine shippedweb folder (contains offline/entry)")
    ap.add_argument("--out", required=True, help="output folder (created/cleaned)")
    ap.add_argument("--base", default="/partyhub/offline", help="window.appContext value (route base)")
    ap.add_argument("--graphql", default="./graphql", help="EPIC_SERVICE_GRAPHQL_URL value")
    args = ap.parse_args()

    src = Path(args.src)
    out = Path(args.out)
    entry_src = src / "offline" / "entry"
    entry_out = out / "offline" / "entry"
    if not (entry_src / "index.html").exists():
        raise SystemExit(f"no offline/entry/index.html under {src}")

    if entry_out.exists():
        shutil.rmtree(out)
    entry_out.mkdir(parents=True)

    # 1. copy everything (entry page + assets)
    for f in entry_src.iterdir():
        if f.is_file():
            shutil.copy2(f, entry_out / f.name)

    # 2. patch the bundle
    bundles = list(entry_out.glob("main.kairos.*.js"))
    if len(bundles) != 1:
        raise SystemExit(f"expected exactly one main.kairos.*.js, found {len(bundles)}")
    bundle_path = bundles[0]
    bundle = bundle_path.read_text(encoding="utf-8")
    if "OwenRoot" in bundle:
        print("bundle already patched — nothing to do")
    else:
        stock = extract_stock_root(bundle)
        if bundle.count(stock) != 1:
            raise SystemExit("stock module 1041 is not unique — aborting")
        replacement = (TOOLS / "partyhub-root.js").read_text(encoding="utf-8")
        # strip the explanatory header comment — keep the payload small
        replacement = re.sub(r"^/\*[\s\S]*?\*/\s*", "", replacement).strip()
        bundle = bundle.replace(stock, replacement, 1)
        print(f"patched root module in {bundle_path.name} ({len(stock)} -> {len(replacement)} bytes)")

        if bundle.count(STOCK_GQL_TAIL) != 1:
            raise SystemExit("stock GraphQlService tail is not unique — aborting")
        bundle = bundle.replace(STOCK_GQL_TAIL, PATCHED_GQL_TAIL, 1)
        print("patched GraphQlService module (self-configure from clientEnvConfig)")
        bundle_path.write_text(bundle, encoding="utf-8")

    # 3. patch index.html
    html_path = entry_out / "index.html"
    html = html_path.read_text(encoding="utf-8")
    html = patch_index_html(html, args.base, args.graphql)
    html_path.write_text(html, encoding="utf-8")
    print(f"patched index.html (appContext={args.base!r}, graphql={args.graphql!r})")

    # 4. copy the bridge
    shutil.copy2(TOOLS / "owen-bridge.js", entry_out / "owen-bridge.js")
    print(f"copied owen-bridge.js -> {entry_out / 'owen-bridge.js'}")

    print("\nServe with: bun run mini-services/partyhub-web (or any static host)")
    print("that (a) falls back to index.html for SPA routes and (b) answers the")
    print("graphql adapter on every route (see mini-services/partyhub-web).")


if __name__ == "__main__":
    main()
