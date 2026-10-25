#!/usr/bin/env python3
"""WAVE B — string-anchored Erbium functions (memory-safe, one version per run).
Usage: waveB.py <version>
Semantics mirror Erbium: target = function CONTAINING the log string ref
(with Android de-outlining: string ref may live in a tiny outlined stub;
real function = BL caller of the stub).
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx

ver = sys.argv[1]
ctx = VersionCtx.get(ver)
t = ctx.tool

STUB_MAX = 0x60

def is_b_thunk(fn):
    sz = ctx.func_size(fn)
    if sz > 8 or sz < 4:
        return False
    o = t.va2off(fn)
    if o is None: return False
    w = struct.unpack_from("<I", t.elf, o)[0]
    return (w & 0xFC000000) == 0x14000000

def real_fn(fn):
    """de-outline chain: stub → B-thunk(4B) → real function.
    Climb while tiny; stop at first caller >= STUB_MAX."""
    cur, layers = fn, 0
    for _ in range(6):
        sz = ctx.func_size(cur)
        if sz >= STUB_MAX:
            break
        callers = set()
        for pc, tgt in t.callers_of(cur):
            f = t.func_of(pc)
            if f is not None and f != cur:
                callers.add(f)
        if not callers:
            break
        big = [c for c in callers if ctx.func_size(c) >= STUB_MAX]
        if big:
            cur = max(big, key=lambda f: ctx.func_size(f))
            layers += 1
            break
        cur = max(callers, key=lambda f: ctx.func_size(f))
        layers += 1
    return cur, layers

def anchored_candidates(s):
    """functions anchoring string s (de-outlined). Returns list of (fn, layers, n_refs)."""
    fns = ctx.anchored_funcs(s)
    out = []
    for fn, pcs in fns.items():
        r, layers = real_fn(fn)
        out.append((r, layers, len(pcs)))
    # merge duplicates
    merged = {}
    for fn, layers, n in out:
        if fn not in merged or layers < merged[fn][0]:
            merged[fn] = (layers, n)
    return [(fn, v[0], v[1]) for fn, v in merged.items()]

def pick_and_pattern(cands, min_sz=0x40, max_sz=0x20000):
    """pick best candidate and generate unique pattern.
    Preference: deoutlined (layers>0) with size in range; then plain."""
    from pattern_fix import gen_pattern_windowed
    best = None
    scored = []
    for fn, layers, nrefs in cands:
        sz = ctx.func_size(fn)
        if sz < min_sz or sz > max_sz:
            continue
        # heuristic score: layers>0 means we climbed out of a stub (good),
        # size reasonable, multiple refs of same string inside = suspicious
        score = (1 if layers > 0 else 0) * 100 + min(sz, 0x3000) / 0x3000 * 50 - (nrefs - 1) * 5
        scored.append((score, fn, sz))
    scored.sort(reverse=True)
    for score, fn, sz in scored:
        r = ctx.gen_pattern(fn)
        if r and r.get('ok'):
            return {"status": "OK", "addr": hex(fn), "size": hex(sz), "pattern": r['pattern'], "n_ins": r['n_ins']}
        r = gen_pattern_windowed(ctx, fn)
        if r and r.get('ok'):
            return {"status": "OK", "addr": hex(fn), "size": hex(sz), "pattern": r['pattern'],
                    "offset": r['offset'], "n_ins": r['n_ins']}
    if scored:
        fn, sz = scored[0][1], scored[0][2]
        return {"status": "no_unique_pattern", "addr": hex(fn), "size": hex(sz),
                "candidates": len(scored)}
    if cands:
        return {"status": "all_tiny_or_huge", "cands": [(hex(f), hex(ctx.func_size(f))) for f, _, _ in cands[:4]]}
    return {"status": "no_string"}

TARGETS = {
    "FindFlushDormancy": [
        "FlushDormancy: %s. Connection: %s",
        "FlushDormancy: %s",
    ],
    "FindStartBecomingDormant": [
        "StartBecomingDormant: %s",
    ],
    "FindUpdateSafeZonesPhase": [
        "AFortGameModeAthena::UpdateSafeZonesPhase",
    ],
    "FindQueueStatEvent": [
        "UFortQuestManager::QueueStatEvent: %s tried to queue a stat event",
        "UFortQuestManager::QueueStatEvent",
    ],
    "FindSelectAndSetupMyBuildingLevel": [
        "ABuildingFoundation::SelectAndSetupMyBuildingLevel - Cannot get WorldManager!!",
        "ABuildingFoundation::SelectAndSetupMyBuildingLevel",
    ],
    "FindPayBuildableClassPlacementCost": [
        "Failed to remove item %s during pay building costs, item duplicated!",
        "during pay building costs, item duplicated",
    ],
    "FindSpawnInitialSafeZone": [
        "FortGameModeAthena::SpawnInitialSafeZone bShouldSpawnSafeZoneIndicator == false",
        "SpawnInitialSafeZone bShouldSpawnSafeZoneIndicator",
    ],
    "FindSpawnLoot": [
        "ABuildingContainer::SpawnLoot() called on %s (%s)...",
        "ABuildingContainer::SpawnLoot() called on %s (%s)",
        "ABuildingContainer::SpawnLoot() >>> START called on %s (%s)",
    ],
    "FindPickSupplyDropLocation": [
        "PickSupplyDropLocation: Failed to find valid location using rejection.  Using safe zone location.",
        "PickSupplyDropLocation: Failed to find valid location using rejection",
        "AFortAthenaMapInfo::PickSupplyDropLocation",
    ],
}

out = {}
for finder, strs in TARGETS.items():
    res = None
    for s in strs:
        cands = anchored_candidates(s)
        if cands:
            res = pick_and_pattern(cands)
            res['string'] = s
            break
    if res is None:
        res = {"status": "no_string", "tried": strs[:2]}
    out[finder] = res
    print(f"{finder}: {res.get('status')} {res.get('addr','')} sz={res.get('size','')}")

# ---- not_unique fixes from wave A (thunk fallbacks + windowed) ----
NAT = json.load(open(f'/home/z/my-project/data/db/natives-v{ver}.json'))
from pattern_fix import gen_pattern_windowed

FIX = {
    "FindRemoveInventoryItem": "ServerRemoveInventoryItem",
    "FindReset": "Reset",
}
for finder, nat in FIX.items():
    prev = {}
    p = f'/home/z/re-analysis/mega/waveA_{ver}.json'
    if os.path.exists(p):
        prev = json.load(open(p)).get(finder, {})
    if prev.get('status') == 'OK':
        out[finder] = prev
        continue
    if nat not in NAT:
        out[finder] = prev if prev else {"status": "no_native"}
        continue
    e = NAT[nat]
    got = prev
    cand = []
    if 'orig' in e:
        cand.append(('orig', int(e['orig'], 16)))
    cand.append(('thunk', int(e['thunk'], 16)))
    for kind, addr in cand:
        if ctx.func_size(addr) >= 0x30:
            r = gen_pattern_windowed(ctx, addr, max_span=100)
            if r and r.get('ok'):
                got = {"status": "OK", "kind": kind, "addr": hex(addr), "size": hex(ctx.func_size(addr)),
                       "pattern": r['pattern'], "offset": r['offset'], "native": nat}
                break
    out[finder] = got
    print(f"{finder}(fix): {got.get('status')} {got.get('addr','')}")

json.dump(out, open(f'/home/z/re-analysis/mega/waveB_{ver}.json', 'w'), indent=1)
print("saved waveB_" + ver)
