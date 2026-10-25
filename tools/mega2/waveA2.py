#!/usr/bin/env python3
"""WAVE A v2 — memory-safe: one version per process invocation.
Usage: waveA2.py <version>  → writes partial JSON; driver merges."""
import json, sys, os, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx

ver = sys.argv[1]
NAT = json.load(open(f'/home/z/my-project/data/db/natives-v{ver}.json'))

MAP = {
    "FindRemoveInventoryItem": ["ServerRemoveInventoryItem"],
    "FindRemoveInventoryStateValue": ["ServerRemoveInventoryStateValue"],
    "FindSetInventoryStateValue": ["ServerSetInventoryStateValue"],
    "FindReset": ["Reset"],
    "FindSetGamePhase": ["SetGamePhase"],
    "FindInitializeFlightPath": ["InitializeFlightPath"],
    "FindSpawnInitialSafeZone": ["SpawnInitialSafeZone"],
    "FindSpawnLoot": ["SpawnLoot"],
    "FindPickSupplyDropLocation": ["PickSupplyDropLocation"],
    "FindClearAbility": ["ClearAbility"],
}

out = {}
ctx = VersionCtx.get(ver)
for finder, natives in MAP.items():
    entry = None
    source = natives[0]
    for nat in natives:
        if nat in NAT:
            e = NAT[nat]
            cand = []
            if 'orig' in e:
                cand.append(('orig', int(e['orig'], 16)))
            cand.append(('thunk', int(e['thunk'], 16)))
            entry = cand
            source = nat
            break
    if entry is None:
        out[finder] = {"status": "no_native"}
        continue
    got = None
    for kind, addr in entry:
        try:
            sz = ctx.func_size(addr)
            if sz >= 0x40:
                r = ctx.gen_pattern(addr)
                if r and r.get('ok'):
                    got = {"status": "OK", "kind": kind, "addr": hex(addr),
                           "size": hex(sz), "pattern": r['pattern'], "n_ins": r['n_ins'],
                           "native": source}
                    break
                elif r and r.get('icf_stub'):
                    got = got or {"status": "icf_stub", "kind": kind, "addr": hex(addr), "native": source}
                elif r:
                    got = got or {"status": "not_unique", "kind": kind, "addr": hex(addr),
                                  "hits": r.get('hits'), "native": source}
            else:
                got = got or {"status": "tiny", "kind": kind, "addr": hex(addr), "size": hex(sz), "native": source}
        except Exception as ex:
            got = got or {"status": "error", "err": str(ex)[:120]}
    out[finder] = got or {"status": "no_candidate"}

json.dump(out, open(f'/home/z/re-analysis/mega/waveA_{ver}.json', 'w'), indent=1)
for f, v in out.items():
    print(f"{f}: {v.get('status')} {v.get('kind','')} {v.get('addr','')}")
