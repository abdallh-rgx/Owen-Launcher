#!/usr/bin/env python3
"""GAP FILLER — final quick wins. Usage: gapfill.py <version>
1. PostInitializeSpawnedBuildingActor = vtable slot AFTER InitializeBuildingActor
2. IsNetReady structural (tiny vtable member: TBZ W1 + NEG + LDR/STR pair)
3. FlushDormancy / CHILF via IsNetReady vtable deltas (17 / 16)
4. ListenCall (7.40-16.x): "LoadMap: failed to Listen" → BL site with cbz w0 after
5. Iris (30.00): TickFlush → BL before PreSendUpdate BL (robust: scan all BLs)
"""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx
from pattern_fix import gen_pattern_windowed

ver = sys.argv[1]
ctx = VersionCtx.get(ver)
t = ctx.tool
addrs = json.load(open('/home/z/re-analysis/mega/resolved_addrs.json'))[ver]

# android runs
rels = t.rels
offs = rels[:, 0].astype(np.int64); adds = rels[:, 1].astype(np.int64)
text_lo, text_hi = ctx.text_va, ctx.text_va + ctx.text_size
in_text = (adds >= text_lo) & (adds < text_hi)
order = np.argsort(offs, kind="stable"); offs, adds, in_text = offs[order], adds[order], in_text[order]
A_RUNS = []; A_ADDR = {}
i, n = 0, len(offs)
while i < n:
    if not in_text[i]: i += 1; continue
    j = i
    while j + 1 < n and in_text[j+1] and offs[j+1] == offs[j] + 8: j += 1
    if j - i + 1 >= 8:
        rid = len(A_RUNS); run = [int(a) for a in adds[i:j+1]]
        A_RUNS.append(run)
        for k, a in enumerate(run): A_ADDR.setdefault(a, []).append((rid, k))
    i = j + 1

def patgen(fn):
    if ctx.func_size(fn) < 0x20: return None
    r = ctx.gen_pattern(fn)
    if r and r.get('ok'):
        return {"pattern": r['pattern'], "addr": hex(fn), "size": hex(ctx.func_size(fn))}
    r = gen_pattern_windowed(ctx, fn, max_span=110)
    if r and r.get('ok'):
        return {"pattern": r['pattern'], "addr": hex(fn), "size": hex(ctx.func_size(fn)), "offset": r['offset']}
    return None

out = {}

# ---------- 1. PostInitializeSpawnedBuildingActor ----------
iba = addrs.get("FindInitializeBuildingActor") or addrs.get("FindInitializeBuildingActor_next")
if iba:
    iba = int(iba, 16)
    got = None
    from collections import Counter
    votes = Counter()
    for rid, idx in A_ADDR.get(iba, []):
        if idx + 1 < len(A_RUNS[rid]):
            cand = A_RUNS[rid][idx + 1]
            if t.func_of(cand) == cand and 0x30 <= ctx.func_size(cand) <= 0x3000:
                votes[cand] += 1
    if votes:
        cand, cnt = votes.most_common(1)[0]
        if cnt >= 2 or len(votes) == 1:
            p = patgen(cand)
            if p:
                p.update({"strategy": "vtable_next_of_IBA", "votes": cnt})
                got = p
    out["FindPostInitializeSpawnedBuildingActor"] = got or {"status": "failed"}
    print("PostInit:", out["FindPostInitializeSpawnedBuildingActor"].get('status'), out["FindPostInitializeSpawnedBuildingActor"].get('addr'))
else:
    out["FindPostInitializeSpawnedBuildingActor"] = {"status": "no_IBA"}

# ---------- 2. IsNetReady structural ----------
isn = None
if addrs.get("FindIsNetRelevantForVft"):
    # candidates: tiny vtable members with TBZ/CBZ W1 + NEG
    cands = set()
    for a in A_ADDR:
        if a in (text_lo, text_hi): continue
        sz = ctx.func_size(a)
        if not (0x1c <= sz <= 0x90): continue
        o = t.va2off(a)
        if o is None: continue
        words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
        has_w1_branch = any((((w & 0x7E000000) == 0x34000000) and (w & 0x1F) == 1) or
                            (((w & 0x7F000000) in (0x36000000, 0x37000000)) and (w & 0x1F) == 0)
                            for w in words)
        has_neg = any(((w & 0xFF1F0000) == 0x4B1F0000 and (w & 0x80000000) == 0) for w in words)
        has_ldr_str = any((w & 0xFFC00000) == 0xB9400000 for w in words) and any((w & 0xFFC00000) == 0xB9000000 for w in words)
        if has_w1_branch and has_neg and has_ldr_str and t.func_of(a) == a:
            cands.add(a)
    scored = []
    for c in cands:
        p = patgen(c)
        if p:
            scored.append(p)
    if len(scored) == 1:
        isn = scored[0]; isn["strategy"] = "structural_tiny_vt"
    elif scored:
        # multiple — keep best by fewest callers? just mark first ambiguous
        isn = scored[0]; isn["strategy"] = "structural_tiny_vt"; isn["ambiguous"] = len(scored)
if isn:
    out["FindIsNetReady"] = isn
    print("IsNetReady:", isn.get('addr'), "ambig" if isn.get('ambiguous') else "")
else:
    out["FindIsNetReady"] = {"status": "structural_failed"}
    print("IsNetReady: FAILED")

# ---------- 3. FlushDormancy / CHILF via IsNetReady delta ----------
if isn and not isn.get('ambiguous'):
    isn_a = int(isn['addr'], 16)
    from collections import Counter
    for finder, delta in (("FindFlushDormancy", 17), ("FindClientHasInitializedLevelFor", 16)):
        votes = Counter()
        for rid, idx in A_ADDR.get(isn_a, []):
            ti = idx + delta
            if 0 <= ti < len(A_RUNS[rid]):
                cand = A_RUNS[rid][ti]
                if t.func_of(cand) == cand and 0x30 <= ctx.func_size(cand) <= 0x600:
                    votes[cand] += 1
        got = None
        if votes:
            cand, cnt = votes.most_common(1)[0]
            if cnt >= 2 or len(votes) == 1:
                p = patgen(cand)
                if p:
                    p.update({"strategy": f"isnetready_delta_{delta}", "votes": cnt})
                    got = p
        out[finder] = got or {"status": "failed"}
        print(f"{finder}:", (out[finder] or {}).get('addr'))

# ---------- 4. ListenCall (old versions) ----------
if ver in ("7.40", "9.41", "13.40", "17.40", "5.41"):
    xf = ctx.xrefs_to_string("LoadMap: failed to Listen")
    if xf:
        f = t.func_of(xf[0][0])
        if f:
            sz = ctx.func_size(f)
            o = t.va2off(f)
            words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
            # find BL whose next instruction tests w0 (cbz/tbz w0)
            site = None
            for k in range(len(words) - 1):
                w = words[k]
                if (w & 0xFC000000) == 0x94000000:
                    w2 = words[k + 1]
                    if ((w2 & 0x7E000000) == 0x34000000 and (w2 & 0x1F) == 0) or \
                       ((w2 & 0x7F000000) in (0x36000000, 0x37000000) and (w2 & 0x1F) == 0):
                        site = f + k * 4
                        break
            if site:
                p = ctx.gen_pattern(site)  # pattern at the BL site (patch target)
                if p and p.get('ok'):
                    out["FindListenCall"] = {"pattern": p['pattern'], "addr": hex(site),
                                             "size": hex(sz), "strategy": "listen_string_bl"}
                    print("ListenCall:", hex(site))
                else:
                    out["FindListenCall"] = {"status": "site_pattern_failed", "addr": hex(site)}
            else:
                out["FindListenCall"] = {"status": "no_bl_site"}

# ---------- 5. Iris (30.00) ----------
if ver == "30.00":
    tf = addrs.get("FindTickFlush")
    psu = addrs.get("FindPreSendUpdate")
    if tf and psu:
        tf, psu = int(tf, 16), int(psu, 16)
        sz = ctx.func_size(tf)
        o = t.va2off(tf)
        words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
        bls = []
        for k, w in enumerate(words):
            if (w & 0xFC000000) == 0x94000000:
                imm = w & 0x3FFFFFF
                if imm & (1 << 25): imm -= 1 << 26
                bls.append((tf + k * 4, tf + k * 4 + imm * 4))
        got = None
        for k, (pc, tg) in enumerate(bls):
            if tg == psu:
                for j in range(k - 1, max(-1, k - 12), -1):
                    pc2, tg2 = bls[j]
                    if tg2 != psu and ctx.func_size(tg2) > 0x60 and t.func_of(tg2) == tg2:
                        p = patgen(tg2)
                        if p:
                            p["strategy"] = "tickflush_prePSU"
                            got = p
                        break
                break
        out["FindUpdateIrisReplicationViews"] = got or {"status": "iris_failed", "n_bls": len(bls)}
        print("Iris:", (got or {}).get('addr'))

json.dump(out, open(f'/home/z/re-analysis/mega/gapfill_{ver}.json', 'w'), indent=1)
print("saved gapfill_" + ver)
