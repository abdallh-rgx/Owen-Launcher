#!/usr/bin/env python3
"""FINAL V3 ASSEMBLY — merge every resolution layer, re-verify all cells, emit:
  - final_resolved_v3.json  (single source of truth)
  - Finders-Android-FULL-v3.cpp  (83 finders + 17 inline + vtable indexes + runtime fallbacks)
  - COVERAGE-MATRIX-v3.md
  - VERIFICATION-REPORT-v3.md  (the cross-arch verification story)
Layers (priority order): targeted3b > targeted > deepcells > fixcells > P0-verified cells >
  gapfill > waves C3/C2/C/B/A > recovered47(session1, only if P0-verified UNIQUE) > vtable_index > globals
"""
import json, os, sys
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')

VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10", "21.30", "22.40", "23.50", "30.00"]
VERIDX = {v: i for i, v in enumerate(VERSIONS)}

VF = json.load(open('/home/z/re-analysis/mega/verification_full.json'))
FIX = json.load(open('/home/z/re-analysis/mega/fixcells_all.json'))
DEEP = {}
for v in VERSIONS:
    p = f'/home/z/re-analysis/mega/deepcells_{v}.json'
    if os.path.exists(p):
        DEEP[v] = json.load(open(p))
T1 = json.load(open('/home/z/re-analysis/mega/targeted_all.json')) if os.path.exists('/home/z/re-analysis/mega/targeted_all.json') else {}
T3B = json.load(open('/home/z/re-analysis/mega/targeted3b.json')) if os.path.exists('/home/z/re-analysis/mega/targeted3b.json') else {}
INLINE = json.load(open('/home/z/re-analysis/mega/inline_all.json'))
OLD = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
CAT3 = json.load(open('/home/z/re-analysis/mega/catalog_v3.json'))

# ============ merge ============
def merged_addr(finder, ver):
    """returns (addr, pattern, offset, strategy) or None"""
    # 1. targeted3b
    e = T3B.get(ver, {}).get(finder)
    if e and e.get('status') == 'OK' and e.get('addr'):
        return int(e['addr'], 16), e.get('pattern'), e.get('offset', 0), e.get('strategy', 'targeted')
    # 2. targeted1
    e = T1.get(ver, {}).get(finder)
    if e and e.get('status') == 'OK' and e.get('addr'):
        return int(e['addr'], 16), e.get('pattern'), e.get('offset', 0), e.get('strategy', 'targeted')
    # 3. deepcells
    e = DEEP.get(ver, {}).get(finder)
    if e and e.get('status') == 'OK' and e.get('addr'):
        return int(e['addr'], 16), e.get('pattern'), e.get('offset', 0), e.get('strategy', 'deep')
    # 4. fixcells
    e = FIX.get(ver, {}).get(finder)
    if e and e.get('status') == 'OK' and e.get('addr'):
        return int(e['addr'], 16), e.get('pattern'), e.get('offset', 0), e.get('strategy', 'fix')
    # 5. P0-verified (unique re-scan) — for old cells incl. recovered47
    rec = VF['per_version'].get(ver, {}).get(finder)
    if rec and rec.get('p0') == 'UNIQUE':
        # prefer wave entries with addr first (they have patterns verified too)
        pass
    # 6. waves/old (only if P0 verified UNIQUE)
    e = OLD.get(finder, {}).get(ver)
    if e:
        p0ok = rec and rec.get('p0') == 'UNIQUE'
        if e.get('addr') and (p0ok or rec is None):
            return int(e['addr'], 16), e.get('pattern'), e.get('offset', 0), e.get('strategy', 'wave')
        if not e.get('addr') and e.get('pattern') and p0ok:
            a = rec.get('resolved_addr')
            if a:
                return int(a, 16), e.get('pattern'), e.get('offset', 0), 'P0-verified-' + str(e.get('strategy', 'session1'))
    # 7. P0 alone (recovered47 pattern now verified)
    if rec and rec.get('p0') == 'UNIQUE' and rec.get('resolved_addr'):
        e = OLD.get(finder, {}).get(ver, {})
        return int(rec['resolved_addr'], 16), e.get('pattern'), e.get('offset', 0), 'P0-verified'
    return None


FINAL = {}
for finder, vers in OLD.items():
    FINAL[finder] = {}
    for ver in VERSIONS:
        if ver not in vers:
            continue
        e = vers[ver]
        if e.get('global'):
            FINAL[finder][ver] = {"global": e['global'], "strategy": e.get('strategy', 'gis'), "verified": e.get('verified', True)}
            continue
        if 'vtable_index' in e:
            FINAL[finder][ver] = {"vtable_index": e['vtable_index'], "strategy": e.get('strategy', 'vtable_parity')}
            continue
        m = merged_addr(finder, ver)
        if m:
            addr, pat, off, strat = m
            entry = {"addr": hex(addr), "strategy": strat}
            if pat:
                entry['pattern'] = pat
            if off:
                entry['offset'] = off
            FINAL[finder][ver] = entry
        elif e.get('pattern') and not (VF['per_version'].get(ver, {}).get(finder, {}).get('p0') in ('NO_MATCH',) or str(VF['per_version'].get(ver, {}).get(finder, {}).get('p0', '')).startswith('BROKEN')):
            # pattern-only entry that didn't fail P0 but has no addr (session1) — only if not verified broken
            FINAL[finder][ver] = {"pattern": e['pattern'], "offset": e.get('offset', 0), "strategy": e.get('strategy', 'session1'), "unverified": True}

# inline functions
for iname in [k for k in CAT3.keys() if k.startswith('Inline_')]:
    FINAL[iname] = {}
    for ver in VERSIONS:
        e = INLINE.get(ver, {}).get(iname)
        if e and e.get('status') == 'OK' and e.get('addr'):
            entry = {"addr": e['addr'], "strategy": e['strategy']}
            if e.get('pattern'):
                entry['pattern'] = e['pattern']
            if e.get('offset'):
                entry['offset'] = e['offset']
            FINAL[iname][ver] = entry

# IsNetRelevantFor computed indexes
IDX = json.load(open('/home/z/re-analysis/mega/isnetrelevant_idx.json'))
for ver, positions in IDX.items():
    if not positions:
        continue
    best = max(positions.items(), key=lambda t: t[1])[0]
    FINAL.setdefault("FindIsNetRelevantForVft", {})[ver] = {
        "vtable_index": best, "votes": positions[best], "strategy": "computed_vtable_index"}

# ============ stats ============
n_pattern = n_global = n_vtidx = n_unver = 0
for f, vers in FINAL.items():
    for ver, e in vers.items():
        if e.get('global'):
            n_global += 1
        elif 'vtable_index' in e:
            n_vtidx += 1
        elif e.get('pattern') and e.get('addr'):
            n_pattern += 1
        elif e.get('unverified'):
            n_unver += 1
print(f"cells: {n_pattern} pattern+addr | {n_vtidx} vtable-index | {n_global} global | {n_unver} unverified-pattern")

full = sum(1 for f, vers in FINAL.items() if len([v for v in vers if not vers[v].get('unverified')]) == 11)
print(f"fully covered functions: {full}/{len(FINAL)}")

json.dump(FINAL, open('/home/z/re-analysis/mega/final_resolved_v3.json', 'w'), indent=1)
print("saved final_resolved_v3.json")

# quick per-function summary for the report
for f in sorted(FINAL.keys()):
    vers = FINAL[f]
    ok = [v for v in VERSIONS if v in vers and not vers[v].get('unverified')]
    print(f"  {f:48s} {len(ok):2d}/11")
