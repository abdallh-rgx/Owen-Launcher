#!/usr/bin/env python3
"""VERIFY-FULL — Complete verification of every Erbium→Android resolution cell.

P0 PATTERN-RESCAN: every saved pattern re-scanned on its version's .text
   → exactly-one-match? at recorded addr? hits function start?
P2 NATIVE-IDENTITY: native-backed finders — resolved addr vs per-version
   native thunk/orig (offsets.h dumper ground truth).
P1 CROSS-ARCH (21.30 only): Windows exe features vs Android features
   (strings, natives called, size ratio).

Writes incremental results to verify_cells_<ver>.json; final: verification_full.json
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, load_impls, VERSIONS

RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
NATMAP = json.load(open('/home/z/my-project/data/db/erbium-native-map.json'))
IMPLS = load_impls()

# per-version native addr→name maps
def build_thunk_map(ver):
    m = {}
    impl = IMPLS.get(ver, {})
    for name, e in impl.items():
        if e.get('orig'):
            m[e['orig']] = name
        if e.get('thunk') and e['thunk'] not in m:
            m[e['thunk']] = name
    return m


def native_candidates(finder):
    return NATMAP.get(finder, [])


def verify_version(ver):
    """P0+P2 for one version. Returns {finder: {verdict, details...}}"""
    out_path = f'/home/z/re-analysis/mega/verify_cells_{ver}.json'
    if os.path.exists(out_path):
        return json.load(open(out_path))

    ctx = VersionCtx.get(ver)
    t = ctx.tool
    tmap = build_thunk_map(ver)
    impl = IMPLS.get(ver, {})

    results = {}
    for finder, vers in RESOLVED.items():
        e = vers.get(ver)
        if not e:
            continue
        rec = {}
        if e.get('global'):
            # global cell: GOT-resolved global — no pattern to scan
            rec['kind'] = 'global'
            rec['p0'] = 'skip_global'
            results[finder] = rec
            continue
        pat = e.get('pattern')
        if not pat:
            results[finder] = {'kind': 'none'}
            continue
        rec['kind'] = 'pattern'
        # ---- P0: rescan ----
        hits = ctx.scan_pattern(pat, limit=5)
        rec['p0_hits'] = len(hits)
        rec['p0_hit_addrs'] = [hex(h) for h in hits[:5]]
        exp = e.get('addr')
        if len(hits) == 1:
            h = hits[0]
            off = e.get('offset', 0)
            target = h + off
            rec['p0'] = 'UNIQUE'
            rec['resolved_addr'] = hex(target)
            if exp and int(exp, 16) == target:
                rec['p0_addr_match'] = True
            elif exp:
                rec['p0_addr_match'] = False  # pattern hits different addr than recorded!
            # is function start?
            rec['is_func_start'] = bool(t.is_func(target))
            if not rec['is_func_start']:
                rec['containing_func'] = hex(t.func_of(target)) if t.func_of(target) else None
        else:
            rec['p0'] = f'BROKEN_{len(hits)}HITS' if hits else 'NO_MATCH'

        # ---- P2: native identity ----
        addr = int(rec['resolved_addr'], 16) if rec.get('resolved_addr') else (int(exp, 16) if exp else None)
        if addr:
            for natname in native_candidates(finder):
                ie = impl.get(natname)
                if not ie:
                    continue
                for kind in ('orig', 'thunk'):
                    if ie.get(kind) and ie[kind] == addr:
                        rec['p2_native'] = f'{natname}:{kind}'
                        break
                if rec.get('p2_native'):
                    break
            # also: native CALLED by this function? (weaker but meaningful)
        results[finder] = rec

    json.dump(results, open(out_path, 'w'), indent=1)
    print(f"[{ver}] {len(results)} cells verified -> {out_path}")
    # free memory
    VersionCtx._cache.pop(ver, None)
    gc.collect()
    return results


def main():
    all_results = {}
    for ver in VERSIONS:
        all_results[ver] = verify_version(ver)

    # ---- summary ----
    stats = {'UNIQUE': 0, 'BROKEN': 0, 'NO_MATCH': 0, 'ADDR_MISMATCH': 0,
             'native_id': 0, 'not_func_start': 0, 'globals': 0}
    broken_cells = []
    for ver, res in all_results.items():
        for finder, rec in res.items():
            p0 = rec.get('p0')
            if p0 == 'skip_global':
                stats['globals'] += 1
            elif p0 == 'UNIQUE':
                stats['UNIQUE'] += 1
                if rec.get('p0_addr_match') is False:
                    stats['ADDR_MISMATCH'] += 1
                    broken_cells.append((ver, finder, 'addr_mismatch', rec.get('resolved_addr')))
                if rec.get('p2_native'):
                    stats['native_id'] += 1
                if not rec.get('is_func_start'):
                    stats['not_func_start'] += 1
                    broken_cells.append((ver, finder, 'not_func_start', rec.get('resolved_addr')))
            elif p0 and p0.startswith('BROKEN'):
                stats['BROKEN'] += 1
                broken_cells.append((ver, finder, p0, rec.get('p0_hit_addrs')))
            elif p0 == 'NO_MATCH':
                stats['NO_MATCH'] += 1
                broken_cells.append((ver, finder, 'no_match', None))

    print("\n=========== VERIFICATION SUMMARY (P0+P2) ===========")
    print(f"UNIQUE match:       {stats['UNIQUE']}")
    print(f"  native-identity:  {stats['native_id']}")
    print(f"  not func start:   {stats['not_func_start']}")
    print(f"  addr mismatch:    {stats['ADDR_MISMATCH']}")
    print(f"BROKEN (>1 hits):   {stats['BROKEN']}")
    print(f"NO MATCH:           {stats['NO_MATCH']}")
    print(f"globals (skip):     {stats['globals']}")
    print(f"\nBroken/mismatched cells ({len(broken_cells)}):")
    for ver, finder, why, det in broken_cells[:60]:
        print(f"  {ver:6s} {finder:48s} {why} {det if det else ''}")

    json.dump({'per_version': all_results, 'stats': stats, 'broken': broken_cells},
              open('/home/z/re-analysis/mega/verification_full.json', 'w'), indent=1)
    print("\nsaved verification_full.json")


if __name__ == '__main__':
    main()
