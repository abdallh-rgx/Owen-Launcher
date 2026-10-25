#!/usr/bin/env python3
"""FIX CELLS — resolve the 95 broken/mismatched cells from verification.
Strategies (per cell, in order):
  S1 native: per-version native thunk/orig (offsets.h) — direct address → gen pattern
  S2 pattern-transfer: try the SAME function's pattern from other versions (adjacent first)
  S3 string-anchor: catalog strings → anchored_funcs (de-outlined) → best candidate
  S4 vtable-transfer: for virtual functions — take the resolved address's vtable position
     on a working version, map to same position via native-seeded vtable runs
Output: fixcells_<ver>.json  {finder: {status, addr, pattern, strategy, ...}}
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, load_impls, VERSIONS
from pattern_fix import gen_pattern_windowed

RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
VF = json.load(open('/home/z/re-analysis/mega/verification_full.json'))
CAT3 = json.load(open('/home/z/re-analysis/mega/catalog_v3.json'))
NATMAP = json.load(open('/home/z/my-project/data/db/erbium-native-map.json'))
IMPLS = load_impls()

VERIDX = {v: i for i, v in enumerate(VERSIONS)}
BROKEN = [(b[0], b[1], b[2] if len(b) > 2 else None) for b in VF['broken']]

# special names: finder -> native names to try (curated; NATMAP has misleading entries)
NATIVE_OVERRIDES = {
    "FindRemoveInventoryItem": ["ServerRemoveInventoryItem"],
    "FindOnRep_ZiplineState": ["OnRep_ZiplineState"],
    "FindGiveAbility": ["K2_GiveAbility"],
    "FindGiveAbilityAndActivateOnce": ["K2_GiveAbilityAndActivateOnce"],
    "FindClearAbility": ["ClearAbility"],
    "FindKickPlayer": ["ServerKickPlayer", "ServerKickPlayer_2", "OnEndGameKickPlayers"],
    "FindGetPlayerViewPoint": ["GetPlayerViewPoint"],
    "FindSetInventoryStateValue": ["ServerSetInventoryStateValue"],
    "FindRemoveInventoryStateValue": ["ServerRemoveInventoryStateValue"],
    "FindReset": ["Reset"],
    "FindPickTeam": ["PickTeam"],
    "FindStartAircraftPhase": ["StartAircraftPhase"],
    "FindSetState": ["SetState"],
    "FindSetGamePhase": ["SetGamePhase"],
    "FindSpawnInitialSafeZone": ["SpawnInitialSafeZone"],
    "FindSpawnLoot": ["SpawnLoot"],
    "FindPickSupplyDropLocation": ["PickSupplyDropLocation"],
    "FindInitializeFlightPath": ["InitializeFlightPath"],
    "FindSetPickupTarget": ["BlueprintSetPickupTarget"],
    "FindEnterAircraft": ["EnterAircraftClient", "ClientEnterAircraft", "OnEnterAircraft"],
    "FindSendRequestNow": ["SendRequestNow"],
    "FindInternalTryActivateAbility": ["InternalTryActivateAbility", "K2_InternalTryActivateAbility"],
    "FindRemoveFromAlivePlayers": ["RemoveFromAlivePlayers"],
    "FindStartStreamingAdditionalPlaylistLevel": ["StartStreamingAdditionalPlaylistLevel"],
    "FindNotifyGameMemberAdded": ["NotifyGameMemberAdded"],
    "FindFinishedTargetSpline": ["FinishedTargetSpline"],
    "FindLoadPlayset": ["LoadPlayset"],
}

# finders where pattern may legitimately hit NON-function-start (patch sites)
PATCH_SITES = {"FindEncryptionPatch", "FindGameSessionPatch", "FindSetIsDoorOpen", "FindListenCall"}


def try_pattern(ctx, addr, min_sz=0x24):
    """generate unique pattern for function at addr"""
    if ctx.func_size(addr) < min_sz:
        r = gen_pattern_windowed(ctx, addr, max_span=80)
        if r and r.get('ok'):
            return r
        return None
    r = ctx.gen_pattern(addr, max_ins=48)
    if r and r.get('ok'):
        return r
    r = gen_pattern_windowed(ctx, addr, max_span=120)
    if r and r.get('ok'):
        return r
    return None


def fix_version(ver):
    out_path = f'/home/z/re-analysis/mega/fixcells_{ver}.json'
    cells = [(f, p0) for (v, f, p0) in BROKEN if v == ver]
    if not cells:
        return {}
    if os.path.exists(out_path):
        return json.load(open(out_path))

    ctx = VersionCtx.get(ver)
    t = ctx.tool
    out = {}

    # collect other-version patterns for transfer (S2)
    pat_bank = {}
    for f, _ in cells:
        bank = []
        for v2, e in RESOLVED.get(f, {}).items():
            if v2 != ver and e.get('pattern'):
                bank.append((abs(VERIDX[v2] - VERIDX[ver]), v2, e['pattern'], e.get('offset', 0)))
        bank.sort()
        pat_bank[f] = bank

    for finder, p0 in cells:
        rec = {"was": p0}
        # ---- S1: native ----
        natnames = NATIVE_OVERRIDES.get(finder, []) + NATMAP.get(finder, [])
        done = False
        for nat in natnames:
            ie = IMPLS.get(ver, {}).get(nat)
            if not ie:
                continue
            for kind in ('orig', 'thunk'):
                addr = ie.get(kind)
                if not addr:
                    continue
                if nat in NATMAP.get(finder, []) and finder == "FindSetWorld":
                    continue  # SetWorld is NOT K2_SetWorldLocation — skip misleading
                sz = ctx.func_size(addr)
                if sz < 0x20:
                    continue
                r = try_pattern(ctx, addr)
                if r:
                    rec.update({"status": "OK", "strategy": f"native:{nat}:{kind}",
                                "addr": hex(addr), "size": hex(sz), **{k: r[k] for k in ('pattern', 'offset') if k in r}})
                    done = True
                    break
            if done:
                break
        if done:
            out[finder] = rec
            print(f"[{ver}] {finder:48s} → NATIVE {rec['addr']} ({rec['strategy']})")
            continue

        # ---- S2: pattern transfer from other versions ----
        best = None
        for dist, v2, pat, off in pat_bank.get(finder, []):
            hits = ctx.scan_pattern(pat, limit=3)
            if len(hits) == 1:
                tgt = hits[0] + off
                fs = t.is_func(tgt)
                best = {"status": "OK", "strategy": f"transfer:{v2}", "addr": hex(tgt),
                        "pattern": pat, "offset": off, "is_func_start": bool(fs)}
                if fs:
                    break
                # non-func-start: keep as fallback (might be patch site)
        if best and (best.get('is_func_start') or finder in PATCH_SITES or best['strategy'].startswith('transfer')):
            # accept transfers even mid-function IF the source version was mid-function too (offset>0)
            src_v = best['strategy'].split(':')[1]
            src_off = RESOLVED.get(finder, {}).get(src_v, {}).get('offset', 0)
            if best.get('is_func_start') or src_off > 0 or finder in PATCH_SITES:
                out[finder] = best
                print(f"[{ver}] {finder:48s} → TRANSFER from {src_v} {best['addr']}")
                continue
        # ---- S3: string anchors ----
        entry = CAT3.get(finder, {})
        strs = entry.get('strings', [])
        cands_all = []
        for s in strs[:6]:
            try:
                fns = ctx.anchored_funcs(s)
            except Exception:
                continue
            for fn, pcs in fns.items():
                # de-outline climb
                cur = fn
                for _ in range(5):
                    if ctx.func_size(cur) >= 0x60:
                        break
                    callers = set()
                    for pc, tgt in t.callers_of(cur):
                        f2 = t.func_of(pc)
                        if f2 is not None and f2 != cur:
                            callers.add(f2)
                    if not callers:
                        break
                    cur = max(callers, key=lambda f3: ctx.func_size(f3))
                if ctx.func_size(cur) >= 0x40:
                    cands_all.append((cur, s))
        scored = {}
        for fn, s in cands_all:
            scored[fn] = s
        for fn, s in scored.items():
            r = try_pattern(ctx, fn)
            if r:
                out[finder] = {"status": "OK", "strategy": f"string:{s[:40]}", "addr": hex(fn),
                               "size": hex(ctx.func_size(fn)), **{k: r[k] for k in ('pattern', 'offset') if k in r}}
                print(f"[{ver}] {finder:48s} → STRING {hex(fn)}")
                done = True
                break
        if done:
            continue
        if best:
            out[finder] = best
            print(f"[{ver}] {finder:48s} → TRANSFER-fallback {best['addr']}")
            continue
        out[finder] = {"status": "UNRESOLVED", "was": p0}
        print(f"[{ver}] {finder:48s} × unresolved")

    json.dump(out, open(out_path, 'w'), indent=1)
    VersionCtx._cache.pop(ver, None)
    gc.collect()
    return out


def main():
    total = {}
    n_ok = n_fail = 0
    for ver in VERSIONS:
        res = fix_version(ver)
        total[ver] = res
        for f, r in res.items():
            if r.get('status') == 'OK':
                n_ok += 1
            else:
                n_fail += 1
    print(f"\n=== FIXED {n_ok}/{n_ok+n_fail} broken cells ===")
    json.dump(total, open('/home/z/re-analysis/mega/fixcells_all.json', 'w'), indent=1)
    print("saved fixcells_all.json")


if __name__ == '__main__':
    main()
