#!/usr/bin/env python3
"""Anchor availability matrix: for every catalog string, check presence in each version.
Also fixes xref resolution: substring → literal start → refs."""
import sys, os, json, struct
import numpy as np
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx, VERSIONS

def find_substrings(ctx, s, limit=40):
    """all (lit_start_va, sub_off_in_lit) occurrences of substring s (ascii & utf16).
    walk back to literal start (double-null or single null boundary)."""
    t = ctx.tool
    ro_va, ro_off, ro_size = ctx.ro_va, ctx.ro_off, ctx.ro_size
    data = t.elf[ro_off:ro_off + ro_size]
    out = []
    for enc, wide in ((s.encode("utf-16-le"), True), (s.encode(), False)):
        i = data.find(enc)
        while i != -1 and len(out) < limit:
            # walk back to literal start: previous 0x0000 (wide) or 0x00 (ascii)
            if wide:
                j = i
                while j > 1 and not (data[j-2] == 0 and data[j-1] == 0):
                    j -= 2
                lit_start = j
                # find literal end
                k = i
                while k < len(data) - 1 and not (data[k] == 0 and data[k+1] == 0):
                    k += 2
                lit = data[lit_start:k].decode("utf-16-le", errors="replace")
            else:
                j = i
                while j > 0 and data[j-1] != 0:
                    j -= 1
                lit_start = j
                k = data.find(b"\0", i)
                lit = data[lit_start:k].decode("ascii", errors="replace")
            out.append((ro_va + lit_start, wide, lit, i - lit_start))
            i = data.find(enc, i + 2)
    return out

def xrefs_to_literal(ctx, s):
    """refs (pc) targeting anywhere within literals containing substring s"""
    t = ctx.tool
    pcs = []
    for lit_start, wide, lit, _ in find_substrings(ctx, s):
        lit_len = (len(lit) + 1) * (2 if wide else 1)
        for pc, tgt in t.refs_to(lit_start):
            pcs.append((pc, tgt, lit_start, lit, wide))
        # also refs into the literal (str+k usage)
        for k in range(2, min(lit_len, 0x80), 2 if wide else 1):
            for pc, tgt in t.refs_to(lit_start + k):
                pcs.append((pc, tgt, lit_start + k, lit, wide))
    return pcs

def anchored_funcs_v2(ctx, s):
    fns = {}
    for pc, tgt, lit_addr, lit, wide in xrefs_to_literal(ctx, s):
        f = ctx.tool.func_of(pc)
        if f is not None:
            fns.setdefault(f, []).append((pc, lit))
    return fns

if __name__ == "__main__":
    import sys
    cat = json.load(open('/home/z/re-analysis/mega/catalog.json'))
    # unique strings from catalog
    strings = {}
    for name, v in cat.items():
        for s in v['strings']:
            if len(s) >= 8:
                strings.setdefault(s, []).append(name)
    print(f"{len(strings)} unique anchor strings", flush=True)
    # resume support
    matrix = {}
    if os.path.exists('/home/z/re-analysis/mega/anchor_matrix.json'):
        try:
            matrix = json.load(open('/home/z/re-analysis/mega/anchor_matrix.json'))
        except Exception:
            matrix = {}
    vers = sys.argv[1:] or VERSIONS
    for ver in vers:
        if ver in matrix and len(matrix[ver]) == len(strings):
            print(f"{ver}: cached", flush=True)
            continue
        ctx = VersionCtx.get(ver)
        row = {}
        for s in strings:
            occ = find_substrings(ctx, s, limit=4)
            row[s] = len(occ)
        matrix[ver] = row
        n_avail = sum(1 for v in row.values() if v > 0)
        print(f"{ver}: {n_avail}/{len(strings)} anchors present", flush=True)
        json.dump(matrix, open('/home/z/re-analysis/mega/anchor_matrix.json', 'w'), indent=1)
    # per-string availability summary
    print("\n=== strings missing in some versions ===", flush=True)
    for s, users in sorted(strings.items()):
        missing = [ver for ver in VERSIONS if ver in matrix and matrix[ver][s] == 0]
        if missing:
            print(f"  [{len(missing)} missing] {s[:60]}  (users: {','.join(u.replace('Find','') for u in users[:3])})", flush=True)
