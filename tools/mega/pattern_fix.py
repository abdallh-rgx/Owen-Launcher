#!/usr/bin/env python3
"""Pattern fixer: regenerate patterns for entries marked non-unique.
Strategy: window search — try start offsets (0..24 ins) × lengths (8..40) until unique.
Also re-verify every existing pattern. Updates wave1.json / a2a.json / x2a_targets.json in place."""
import sys, os, json, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, DUMP, VERSIONS

def gen_pattern_windowed(ctx, fn, max_span=72):
    """windowed pattern: try (start, len) combos; return smallest unique"""
    t = ctx.tool
    n_total = min(ctx.func_size(fn) // 4, max_span)
    if n_total < 4:
        return None
    o = t.va2off(fn)
    words = struct.unpack_from(f"<{n_total}I", t.elf, o)
    if n_total <= 2 and (words[0] & 0xFFFFFC1F) == 0xD65F0000:
        return {'icf_stub': True}
    pairs = [ctx._pat_word_mask(w) for w in words]
    for start in (0, 4, 8, 12, 16, 24):
        if start >= n_total - 5:
            break
        for n in (6, 8, 10, 12, 16, 20, 24, 28, 32, 40):
            if start + n > n_total:
                break
            pat = ctx._render(pairs[start:start + n])
            hits = ctx.scan_pattern(pat, limit=3)
            if len(hits) == 1 and hits[0] == fn + start * 4:
                return {'pattern': pat, 'n_ins': n, 'start_ins': start, 'offset': start * 4, 'ok': True}
            if len(hits) == 0:
                break
    return None


def fix_file(path, per_version_entries, key_check=lambda r: (r.get('status') == 'ok' or r.get('addr') or r.get('android') or r.get('pattern'))):
    """generic fixer: for each entry with pattern non-unique or missing, regenerate.
    per_version_entries: iterable of (ver, get_addr(r), set_fields(r))"""
    data = json.load(open(path))
    ctx_cache = {}
    fixed = skipped = 0
    for item_key, entry in data.items() if isinstance(data, dict) else []:
        if not isinstance(entry, dict):
            continue
        for ver in VERSIONS:
            r = entry.get(ver) if ver in entry else None
            if r is None or not isinstance(r, dict):
                continue
            # addr?
            addr = None
            for k in ('addr', 'android'):
                if r.get(k) and isinstance(r[k], str) and r[k].startswith('0x'):
                    addr = int(r[k], 16)
                    break
            if not addr:
                continue
            needs = (r.get('pattern') in (None, 'ICF_STUB')) or (r.get('unique') is False) or (r.get('pattern') and not r.get('unique'))
            if not needs:
                continue
            if ver not in ctx_cache:
                ctx_cache[ver] = VersionCtx(ver)
            ctx = ctx_cache[ver]
            p = gen_pattern_windowed(ctx, addr)
            if p and p.get('ok'):
                r['pattern'] = p['pattern']
                r['unique'] = True
                r['pat_ins'] = p['n_ins']
                r['pat_offset'] = p['offset']
                if p['offset']:
                    r['note_pat_offset'] = f"pattern matches at func+{hex(p['offset'])} — add offset to result"
                fixed += 1
            elif p and p.get('icf_stub'):
                r['pattern'] = 'ICF_STUB'
                r['unique'] = False
            else:
                skipped += 1
    json.dump(data, open(path, 'w'), indent=1)
    return fixed, skipped


def fix_x2a(path='/home/z/re-analysis/mega/x2a_targets.json'):
    """x2a_targets shape: {fname: {x64, android, pattern, unique}} — treat as 21.30"""
    data = json.load(open(path))
    ctx = None
    fixed = skipped = 0
    for fname, r in data.items():
        if not isinstance(r, dict):
            continue
        addr = r.get('android')
        if not addr or not isinstance(addr, str) or not addr.startswith('0x'):
            continue
        needs = (not r.get('pattern')) or (r.get('unique') is False) or \
                (r.get('pattern') and r.get('unique') is None)
        if not needs:
            continue
        if ctx is None:
            ctx = VersionCtx('21.30')
        p = gen_pattern_windowed(ctx, int(addr, 16))
        if p and p.get('ok'):
            r['pattern'] = p['pattern']
            r['unique'] = True
            r['pat_offset'] = p['offset']
            fixed += 1
        elif p and p.get('icf_stub'):
            r['pattern'] = 'ICF_STUB'
            r['unique'] = False
        else:
            skipped += 1
    json.dump(data, open(path, 'w'), indent=1)
    if ctx:
        ctx.tool.close()
    return fixed, skipped


def main():
    # wave1: {fname: {ver: {status, addr, pattern, unique}}}
    f1, s1 = fix_file('/home/z/re-analysis/mega/wave1.json', None)
    print(f"wave1: fixed={f1} still-nonunique={s1}")
    # a2a: {fname: {ver: {addr, pattern, unique}}}
    if os.path.exists('/home/z/re-analysis/mega/a2a.json'):
        f2, s2 = fix_file('/home/z/re-analysis/mega/a2a.json', None)
        print(f"a2a: fixed={f2} still-nonunique={s2}")
    f3, s3 = fix_x2a()
    print(f"x2a: fixed={f3} still-nonunique={s3}")
    if os.path.exists("/home/z/re-analysis/mega/wave3.json"):
        f4, s4 = fix_file("/home/z/re-analysis/mega/wave3.json", None)
        print(f"wave3: fixed={f4} still-nonunique={s4}")
    print(f"x2a: fixed={f3} still-nonunique={s3}")

if __name__ == '__main__':
    main()
