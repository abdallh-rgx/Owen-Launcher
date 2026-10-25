#!/usr/bin/env python3
"""WAVE C3 — fingerprint resolution for all remaining Tier-3 targets.
Usage: waveC3.py <version>"""
import sys, json, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx
from xfp import FpMatcher, resolve
from pattern_fix import gen_pattern_windowed

ver = sys.argv[1]
ctx = VersionCtx.get(ver)
t = ctx.tool
fm = FpMatcher(ctx)

# specs per target (translated for ARM64: imm>4095 byte → movz; d→ldr_w; q→ldr_x)
TARGETS = {
    "FindSendClientAdjustment": {
        "specs": [("ldr_w", 1972), ("ldr_w", 1976), ("ldrb", 1988), ("ldrb", 1688)],
        "size": (0x100, 0x400),
    },
    "FindCantBuild": {
        "specs": [("ldrb", 176), ("ldr_x", 184), ("ldr_x", 192)],
        "size": (0x80, 0x220),
    },
    "FindConstructAbilitySpec": {
        "specs": [("ldrb", 224), ("ldrb", 360), ("ldr_x", 1616)],
        "size": (0x200, 0x600),
    },
    "FindSetPickupItems": {
        "specs": [("ldrb", 992), ("ldr_x", 64), ("ldr_x", 72)],
        "size": (0x38, 0x180),
    },
    "FindCreateNetDriverWorldContext": {
        "specs": [("ldr_w", 3144), ("ldr_w", 400), ("ldr_w", 520)],
        "size": (0x100, 0x300),
    },
    "FindSetChannelActor": {
        "specs": [("ldr_x", 1720), ("ldr_w", 240), ("ldr_x", 688)],
        "size": (0x100, 0x300),
    },
    "FindFlushDormancy": {
        "specs": [("ldr_x", 5168), ("ldr_w", 504), ("cmp_imm", 1000)],
        "size": (0x100, 0x400),
    },
    "FindIsNetReady": {
        "specs": [("ldr_w", 0x698), ("str_w", 0x1f8), ("movz_w", 0x3290), ("movz_x", 0x3230)],
        "size": (0x24, 0x90), "min_hits": 3,
    },
}

out = {}
for finder, cfg in TARGETS.items():
    r = resolve(ctx, cfg["specs"], cfg["size"], min_hits=cfg.get("min_hits", len(cfg["specs"]) - 1))
    r.setdefault("status", "fp_no_match")
    out[finder] = r
    print(f"{finder}: {r.get('status')} {r.get('addr','')} sz={r.get('size','')} hits={r.get('fp_hits','')}")


# ---- vtable-delta fallbacks for FlushDormancy (17) and CHILF (16) from IsNetReady ----
if out.get("FindIsNetReady", {}).get("status") == "OK":
    isn = int(out["FindIsNetReady"]["addr"], 16)
    rels = t.rels
    offs = rels[:, 0].astype(np.int64); adds = rels[:, 1].astype(np.int64)
    text_lo, text_hi = ctx.text_va, ctx.text_va + ctx.text_size
    in_text = (adds >= text_lo) & (adds < text_hi)
    order = np.argsort(offs, kind="stable"); offs, adds, in_text = offs[order], adds[order], in_text[order]
    A_RUNS2 = []; A_ADDR2 = {}
    i, n = 0, len(offs)
    while i < n:
        if not in_text[i]: i += 1; continue
        j = i
        while j + 1 < n and in_text[j+1] and offs[j+1] == offs[j] + 8: j += 1
        if j - i + 1 >= 8:
            rid = len(A_RUNS2); run = [int(a2) for a2 in adds[i:j+1]]
            A_RUNS2.append(run)
            for k, a2 in enumerate(run): A_ADDR2.setdefault(a2, []).append((rid, k))
        i = j + 1

    from collections import Counter
    def delta_majority(delta, size_range):
        votes = Counter()
        for rid, idx in A_ADDR2.get(isn, []):
            ti = idx + delta
            if 0 <= ti < len(A_RUNS2[rid]):
                cand = A_RUNS2[rid][ti]
                if t.func_of(cand) == cand and size_range[0] <= ctx.func_size(cand) <= size_range[1]:
                    votes[cand] += 1
        if not votes:
            return None
        cand, cnt = votes.most_common(1)[0]
        if cnt < 2:
            return None
        r = ctx.gen_pattern(cand)
        if r and r.get('ok'):
            return {"status": "OK", "addr": hex(cand), "size": hex(ctx.func_size(cand)),
                    "pattern": r['pattern'], "strategy": "isnetready_vt_delta", "delta": delta, "votes": cnt}
        r = gen_pattern_windowed(ctx, cand, max_span=100)
        if r and r.get('ok'):
            return {"status": "OK", "addr": hex(cand), "size": hex(ctx.func_size(cand)),
                    "pattern": r['pattern'], "offset": r['offset'], "strategy": "isnetready_vt_delta", "delta": delta, "votes": cnt}
        return None

    if out.get("FindFlushDormancy", {}).get("status") != "OK":
        fd = delta_majority(17, (0x80, 0x400))
        if fd:
            out["FindFlushDormancy"] = fd
        print("FlushDormancy(vtfb):", out["FindFlushDormancy"].get('status'), out["FindFlushDormancy"].get('addr'))
    if out.get("FindClientHasInitializedLevelFor", {}).get("status") != "OK":
        ch = delta_majority(16, (0x40, 0x200))
        if ch:
            out["FindClientHasInitializedLevelFor"] = ch
        print("CHILF(vtfb):", out["FindClientHasInitializedLevelFor"].get('status'), out["FindClientHasInitializedLevelFor"].get('addr'))

# ---- CHILF: (original block superseded) ----
if out["FindIsNetReady"].get("status") == "OK":
    isn = int(out["FindIsNetReady"]["addr"], 16)
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
            rid = len(A_RUNS); run = [int(a2) for a2 in adds[i:j+1]]
            A_RUNS.append(run)
            for k, a2 in enumerate(run): A_ADDR.setdefault(a2, []).append((rid, k))
        i = j + 1
    got = None
    for delta in (14, 15, 16, 13):
        for rid, idx in A_ADDR.get(isn, []):
            ti = idx + delta
            if 0 <= ti < len(A_RUNS[rid]):
                cand = A_RUNS[rid][ti]
                if t.func_of(cand) != cand: continue
                sz = ctx.func_size(cand)
                if not (0x1c <= sz <= 0x280): continue
                # CHILF structure: reads [x1+0x20] (actor guid) or map lookups
                o = t.va2off(cand)
                words = struct.unpack_from(f"<{min(sz//4, 50)}I", t.elf, o)
                has_actor20 = any(((w & 0xFFC00000) == 0xF9400000) and (((w >> 10) & 0xFFF) == 4) for w in words)
                if not has_actor20: continue
                r = ctx.gen_pattern(cand)
                if r and r.get('ok'):
                    got = {"status": "OK", "addr": hex(cand), "size": hex(sz), "pattern": r['pattern'],
                           "strategy": "isnetready_vt_delta", "delta": delta}
                    break
                r = gen_pattern_windowed(ctx, cand, max_span=80)
                if r and r.get('ok'):
                    got = {"status": "OK", "addr": hex(cand), "size": hex(sz), "pattern": r['pattern'],
                           "offset": r['offset'], "strategy": "isnetready_vt_delta", "delta": delta}
                    break
        if got: break
    if got:
        out["FindClientHasInitializedLevelFor"] = got
    elif out.get("FindClientHasInitializedLevelFor", {}).get("status") != "OK":
        out["FindClientHasInitializedLevelFor"] = {"status": "chilf_failed"}
elif out.get("FindClientHasInitializedLevelFor", {}).get("status") != "OK":
    out["FindClientHasInitializedLevelFor"] = {"status": "needs_isnetready"}
print("FindClientHasInitializedLevelFor:", out["FindClientHasInitializedLevelFor"].get('status'), out["FindClientHasInitializedLevelFor"].get('addr'))

json.dump(out, open(f'/home/z/re-analysis/mega/waveC3_{ver}.json', 'w'), indent=1)
print("saved waveC3_" + ver)
