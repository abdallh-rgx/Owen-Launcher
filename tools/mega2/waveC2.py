#!/usr/bin/env python3
"""WAVE C v2 — cross-arch via EMPIRICAL vtable deltas (Windows run members).
Empirical deltas (from Windows 21.30 exe):
  InitListen = TickFlush_idx - 15      (run 2198: InitListen@2150, TickFlush@2165)
  StartBecomingDormant = Close_idx + 11 (run 124: Close@288, SBD@299)
  FlushDormancy = IsNetReady_idx + 17   (run 3227: IsNetReady@1432, FD@1449)
Usage: waveC2.py <version>"""
import sys, json, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx
from pattern_fix import gen_pattern_windowed

ver = sys.argv[1]
ctx = VersionCtx.get(ver)
t = ctx.tool

addrs = json.load(open('/home/z/re-analysis/mega/resolved_addrs.json'))[ver]
xwin = json.load(open('/home/z/re-analysis/mega/xwin.json'))

# ---------------- Android runs from rels ----------------
rels = t.rels
offs = rels[:, 0].astype(np.int64)
adds = rels[:, 1].astype(np.int64)
text_lo, text_hi = ctx.text_va, ctx.text_va + ctx.text_size
in_text = (adds >= text_lo) & (adds < text_hi)
order = np.argsort(offs, kind="stable")
offs, adds, in_text = offs[order], adds[order], in_text[order]

A_RUNS = []
A_ADDR = {}
i, n = 0, len(offs)
while i < n:
    if not in_text[i]:
        i += 1
        continue
    j = i
    while j + 1 < n and in_text[j + 1] and offs[j + 1] == offs[j] + 8:
        j += 1
    if j - i + 1 >= 8:
        rid = len(A_RUNS)
        run = [int(a) for a in adds[i:j + 1]]
        A_RUNS.append(run)
        for k, a in enumerate(run):
            A_ADDR.setdefault(a, []).append((rid, k))
    i = j + 1

def A(finder):
    v = addrs.get(finder)
    return int(v, 16) if v else None

def patgen(fn):
    if ctx.func_size(fn) < 0x20:
        return None
    r = ctx.gen_pattern(fn)
    if r and r.get('ok'):
        return {"status": "OK", "addr": hex(fn), "size": hex(ctx.func_size(fn)),
                "pattern": r['pattern'], "n_ins": r['n_ins']}
    r = gen_pattern_windowed(ctx, fn, max_span=110)
    if r and r.get('ok'):
        return {"status": "OK", "addr": hex(fn), "size": hex(ctx.func_size(fn)),
                "pattern": r['pattern'], "offset": r['offset']}
    return None

def candidates_by_delta(anchor_addr, delta, size_range=(0x20, 0x2000), max_cands=8):
    """all run[idx+delta] candidates for anchor's runs"""
    out = []
    for rid, idx in A_ADDR.get(anchor_addr, []):
        ti = idx + delta
        if 0 <= ti < len(A_RUNS[rid]):
            cand = A_RUNS[rid][ti]
            sz = ctx.func_size(cand)
            if size_range[0] <= sz <= size_range[1]:
                out.append((cand, rid, ti, sz))
    # dedupe keep order
    seen = set()
    res = []
    for c in out:
        if c[0] not in seen:
            seen.add(c[0])
            res.append(c)
    return res[:max_cands]

def try_resolve(finder, anchor_finder, delta, size_range, validate=None):
    a = A(anchor_finder)
    if a is None:
        return {"status": "no_android_anchor", "anchor": anchor_finder}
    cands = candidates_by_delta(a, delta, size_range)
    if not cands:
        return {"status": "no_candidates", "anchor": anchor_finder, "delta": delta}
    results = []
    for cand, rid, ti, sz in cands:
        # is it a function start?
        if t.func_of(cand) != cand:
            continue
        if validate and not validate(cand):
            continue
        p = patgen(cand)
        if p:
            p.update({"strategy": "vtable_delta", "anchor": anchor_finder,
                      "delta": delta, "cand_run": rid})
            results.append(p)
    if len(results) == 1:
        return results[0]
    if len(results) > 1:
        # multiple — pick the first but mark ambiguous
        results[0]["ambiguous"] = len(results)
        return results[0]
    return {"status": "pattern_failed", "cands": [(hex(c), hex(s)) for c, _, _, s in cands]}

out = {}

# ---- InitListen = TickFlush - 15 ----
# UE5 InitListen structure: vtable call (notify->NotifyAcceptingConnection) + global flag check.
def val_initlisten(fn):
    sz = ctx.func_size(fn)
    if sz > 0x400:
        return False
    o = t.va2off(fn)
    words = struct.unpack_from(f"<{min(sz // 4, 40)}I", t.elf, o)
    has_blr = any((w & 0xFFFFFC00) == 0xD63F0000 for w in words)  # BLR
    return has_blr

out["FindInitListen"] = try_resolve("FindInitListen", "FindTickFlush", -15, (0x60, 0x400), val_initlisten)
print("FindInitListen:", out["FindInitListen"].get('status'), out["FindInitListen"].get('addr'))

# ---- StartBecomingDormant = Close + 11 ----
out["FindStartBecomingDormant"] = try_resolve("FindStartBecomingDormant", "FindCloseActorChannel", +11, (0x40, 0x800))
print("FindStartBecomingDormant:", out["FindStartBecomingDormant"].get('status'), out["FindStartBecomingDormant"].get('addr'))

# ---- IsNetReady: structural tiny vtable member (TBZ W1 / CBZ + LDR W0,[X0,imm] + NEG) ----
isn = None
# windows IsNetReady shape: test dl,dl; jz +; mov eax,[rcx+X]; neg eax; ret-ish
for cand, rid, ti, sz in candidates_by_delta(A("FindCloseActorChannel") or 0, 102 - 288, (0x1c, 0x80)) if A("FindCloseActorChannel") else []:
    if t.func_of(cand) != cand:
        continue
    o = t.va2off(cand)
    words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
    # look for: CBZ/TBZ W1; LDR Wn,[X0,#imm]; NEG Wn,Wn
    has_branch_w1 = any(((w & 0x7E000000) == 0x34000000 and (w & 0x1F) == 1) or
                        ((w & 0x7F000000) in (0x36000000, 0x37000000) and (w & 0x1F) <= 1)
                        for w in words)
    has_neg = any((w & 0xFFE00000) == 0x4B200000 for w in words)  # NEG Wd,Wn = SUB Wd,Wn,WZR
    if has_branch_w1 and has_neg:
        p = patgen(cand)
        if p:
            p.update({"strategy": "vtable_close_delta_struct", "delta": 102 - 288})
            isn = p
            break
if isn:
    out["FindIsNetReady"] = isn
else:
    out["FindIsNetReady"] = {"status": "structural_failed"}
print("FindIsNetReady:", out["FindIsNetReady"].get('status'), out["FindIsNetReady"].get('addr'))

# ---- FlushDormancy = IsNetReady + 17 (if IsNetReady found); else strings ----
if out["FindIsNetReady"].get("status") == "OK":
    isn_addr = int(out["FindIsNetReady"]["addr"], 16)
    res = None
    for cand, rid, ti, sz in candidates_by_delta(isn_addr, +17, (0x40, 0x800)):
        if t.func_of(cand) != cand:
            continue
        p = patgen(cand)
        if p:
            p.update({"strategy": "vtable_isnetready_delta", "delta": 17})
            res = p
            break
    out["FindFlushDormancy"] = res or {"status": "fd_failed"}
else:
    out["FindFlushDormancy"] = {"status": "needs_isnetready"}
print("FindFlushDormancy:", out["FindFlushDormancy"].get('status'), out["FindFlushDormancy"].get('addr'))

# ---- ClientHasInitializedLevelFor = IsNetReady + 14 or +15 ----
if out["FindIsNetReady"].get("status") == "OK":
    isn_addr = int(out["FindIsNetReady"]["addr"], 16)
    got = None
    for delta in (+14, +15, +16, +13):
        for cand, rid, ti, sz in candidates_by_delta(isn_addr, delta, (0x1c, 0x300)):
            if t.func_of(cand) != cand:
                continue
            # CHILF structure: reads map/array [this+X], compares actor ptr
            p = patgen(cand)
            if p:
                p.update({"strategy": "vtable_isnetready_delta", "delta": delta})
                got = p
                break
        if got:
            break
    out["FindClientHasInitializedLevelFor"] = got or {"status": "chilf_failed"}
else:
    out["FindClientHasInitializedLevelFor"] = {"status": "needs_isnetready"}
print("FindClientHasInitializedLevelFor:", out["FindClientHasInitializedLevelFor"].get('status'), out["FindClientHasInitializedLevelFor"].get('addr'))

# ---- SendClientAdjustment: UCMC vtable — structural: Windows run 66 idx 486.
# No Android anchor in UCMC... try: candidates by neighbors? skip structural run matching here.
out["FindSendClientAdjustment"] = {"status": "pending_structural"}

# ---- ClearAbility: scan inside GAACO for BL targets (Erbium semantics) ----
gaaco = A("FindGiveAbilityAndActivateOnce")
if gaaco:
    sz = ctx.func_size(gaaco)
    o = t.va2off(gaaco)
    words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
    bls = []
    for i, w in enumerate(words):
        if (w & 0xFC000000) == 0x94000000:  # BL
            imm = w & 0x3FFFFFF
            if imm & (1 << 25):
                imm -= 1 << 26
            tgt = gaaco + i * 4 + imm * 4
            bls.append((gaaco + i * 4, tgt))
    # ClearAbility = the LAST BLs whose target is a decent function (Erbium: mov rcx; call; jmp)
    # heuristic: target size 0x100-0x800, target != natives
    got = None
    NAT = json.load(open(f'/home/z/my-project/data/db/natives-v{ver}.json'))
    nat_addrs = set()
    for e in NAT.values():
        nat_addrs.add(int(e['thunk'], 16))
        if 'orig' in e:
            nat_addrs.add(int(e['orig'], 16))
    for pc, tg in reversed(bls):
        if tg in nat_addrs or tg == gaaco:
            continue
        tsz = ctx.func_size(tg)
        if 0x80 <= tsz <= 0x1000 and t.func_of(tg) == tg:
            p = patgen(tg)
            if p:
                p.update({"strategy": "gaaco_body_bl", "call_site": hex(pc)})
                got = p
                break
    out["FindClearAbility"] = got or {"status": "gaaco_scan_failed", "n_bls": len(bls)}
else:
    out["FindClearAbility"] = {"status": "no_gaaco"}
print("FindClearAbility:", out["FindClearAbility"].get('status'), out["FindClearAbility"].get('addr'))

# ---- UpdateIrisReplicationViews (30.00 only): TickFlush body → BL before PreSendUpdate BL ----
if ver == "30.00":
    tf = A("FindTickFlush")
    psu = A("FindPreSendUpdate")
    if tf and psu:
        sz = ctx.func_size(tf)
        o = t.va2off(tf)
        words = struct.unpack_from(f"<{sz // 4}I", t.elf, o)
        bls = []
        for i, w in enumerate(words):
            if (w & 0xFC000000) == 0x94000000:
                imm = w & 0x3FFFFFF
                if imm & (1 << 25):
                    imm -= 1 << 26
                bls.append((tf + i * 4, tf + i * 4 + imm * 4))
        # find BL to psu; take the BL just before it
        got = None
        for k, (pc, tg) in enumerate(bls):
            if tg == psu and k > 0:
                # walk back to previous BL with a different target
                for j in range(k - 1, max(-1, k - 8), -1):
                    pc2, tg2 = bls[j]
                    if tg2 != psu and ctx.func_size(tg2) > 0x40:
                        p = patgen(tg2)
                        if p:
                            p.update({"strategy": "tickflush_prePSU_bl"})
                            got = p
                        break
                break
        out["FindUpdateIrisReplicationViews"] = got or {"status": "iris_scan_failed"}
        print("FindUpdateIrisReplicationViews:", out["FindUpdateIrisReplicationViews"].get('status'))

json.dump(out, open(f'/home/z/re-analysis/mega/waveC2_{ver}.json', 'w'), indent=1)
print("saved waveC2_" + ver)
