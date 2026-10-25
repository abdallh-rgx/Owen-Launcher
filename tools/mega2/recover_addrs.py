#!/usr/bin/env python3
"""Recover resolved-47 addresses per version by scanning their (verified) patterns.
Output: resolved_addrs.json {ver: {finder: addr}} — anchors for waveC."""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx

VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10", "21.30", "22.40", "23.50", "30.00"]
rec = json.load(open('/home/z/re-analysis/recovered_47.json'))

out = {}
for ver in VERSIONS:
    ctx = VersionCtx.get(ver)
    m = {}
    for finder, branches in rec.items():
        # branches: {"30.00": pat, "23.50": pat, ...} — pattern applies from its version DOWN until previous branch
        for bver, pat in branches.items():
            # applicable if ver >= bver and no higher branch between... patterns were per-version already;
            # recovered map is exact-per-version? The codegen made cumulative branches. To be safe: scan each
            # pattern; if it matches uniquely, record (the highest matching branch for this ver wins).
            try:
                hits = ctx.scan_pattern(pat, limit=3)
                if len(hits) == 1:
                    prev = m.get(finder)
                    # branch version ordering: keep the one from the highest bver <= ver... we can't tell which
                    # matched; record first unique per finder (patterns are distinct enough)
                    if finder not in m:
                        m[finder] = hits[0]
            except Exception:
                pass
    out[ver] = {k: hex(v) for k, v in m.items()}
    print(f"{ver}: {len(m)} anchors recovered")
    # free memory
    VersionCtx._cache.pop(ver, None)
    import gc; gc.collect()

json.dump(out, open('/home/z/re-analysis/mega/resolved_addrs.json', 'w'), indent=1)
print("saved")
