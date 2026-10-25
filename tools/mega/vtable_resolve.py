#!/usr/bin/env python3
"""LIGHT vtable resolution — no graph edges, no BFS.
Uses x2a_map.json seeds + VTableAnchor to resolve all wave2a targets.
Saves into x2a_targets.json (merging)."""
import sys, os, json
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from x64tool import X64
from engine import VersionCtx
from vtable_x64 import VTableAnchor

def main():
    x = X64.get()
    ctx = VersionCtx('21.30')
    vt = VTableAnchor(x, ctx)
    m = json.load(open('/home/z/re-analysis/mega/x2a_map.json'))
    for a, b in m.items():
        vt.feed_seed(int(a, 16), int(b, 16))
    print(f"seeds fed: run_pairs={len(vt.run_pairs)}", flush=True)

    w2a = json.load(open('/home/z/re-analysis/mega/wave2a.json'))
    try:
        targets = json.load(open('/home/z/re-analysis/mega/x2a_targets.json'))
    except FileNotFoundError:
        targets = {}
    wave1 = json.load(open('/home/z/re-analysis/mega/wave1.json'))
    resolved_2130 = {f: v['21.30']['addr'] for f, v in wave1.items()
                     if v.get('21.30', {}).get('status') == 'ok' and v['21.30'].get('addr')}

    n = 0
    for fname, e in w2a.items():
        if fname in resolved_2130:
            targets.setdefault(fname, {'status': 'wave1', 'android': resolved_2130[fname]})
            continue
        if not e.get('x64_addr'):
            continue
        if targets.get(fname, {}).get('android') and targets.get(fname, {}).get('pattern'):
            continue  # already resolved
        a = int(e['x64_addr'], 16)
        if not x.func_of(a) == a:
            continue
        cands = vt.candidate_for(a)
        if not cands:
            continue
        # group candidates by address
        from collections import Counter
        cnt = Counter(c for c, nn, off in cands)
        best, votes = cnt.most_common(1)[0]
        # size sanity
        xs = x.func_size(a)
        asz = ctx.func_size(best)
        ratio = (asz / xs) if xs and asz else 0
        rec = {'x64': e['x64_addr'], 'method': 'vtable-run', 'votes': votes,
               'vt_cands': len(cands), 'size_ratio': round(ratio, 2)}
        if votes >= 2 and 0.25 <= ratio <= 4.0:
            rec['android'] = hex(best)
            p = ctx.gen_pattern(best)
            if p:
                if p.get('icf_stub'):
                    rec['pattern'] = 'ICF_STUB'
                    rec['icf'] = True
                else:
                    rec['pattern'] = p['pattern']
                    rec['unique'] = p['ok']
            n += 1
            print(f"{fname:44s} → {hex(best)} votes={votes} ratio={ratio:.2f} pat={'✓' if rec.get('unique') else '✗'}", flush=True)
        else:
            targets.setdefault(fname, rec)
            print(f"{fname:44s} ~ {hex(best)} votes={votes} ratio={ratio:.2f} (weak)", flush=True)
        targets[fname] = rec if 'android' in rec else targets.get(fname, rec)
    json.dump(targets, open('/home/z/re-analysis/mega/x2a_targets.json', 'w'), indent=1)
    print(f"resolved via vtable: {n}")
    ctx.tool.close()

if __name__ == '__main__':
    main()
