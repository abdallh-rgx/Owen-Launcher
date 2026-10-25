#!/usr/bin/env python3
"""XVT — Windows vtable runs + target indices (run once).
Output: xwin_vt.json {finder: {win_runs: [[idx, run_len], ...]}}"""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from x64tool import X64

x = X64.get()
xwin = json.load(open('/home/z/re-analysis/mega/xwin.json'))

tva, tvsz, toff, trsz = x.sections[".text"]
text_lo = x.base + tva
text_hi = x.base + tva + tvsz

# .rdata + .data as u64 arrays → runs of consecutive code pointers
runs = []
addr2runs = {}

for sec in (".rdata", ".data"):
    vaddr, vsize, roff, rsize = x.sections[sec]
    n = rsize // 8
    arr = np.frombuffer(x.full[roff:roff + n * 8], dtype="<u8")
    is_code = (arr >= text_lo) & (arr < text_hi)
    # runs: maximal sequences of consecutive is_code
    idx = np.flatnonzero(is_code)
    if len(idx) == 0:
        continue
    splits = np.flatnonzero(np.diff(idx) > 1)
    starts = np.concatenate(([idx[0]], idx[splits + 1]))
    ends = np.concatenate((idx[splits], [idx[-1]]))
    for s, e in zip(starts, ends):
        if e - s + 1 < 8:  # vtables have >= 8 entries
            continue
        run_va = x.base + vaddr + int(s) * 8
        members = [int(v) for v in arr[s:e + 1]]
        run_id = len(runs)
        runs.append((run_va, members))
        for i, m in enumerate(members):
            addr2runs.setdefault(m, []).append((run_id, i, run_va))

print(f"windows vtable runs: {len(runs)}; ptrs in runs: {len(addr2runs)}")

out = {}
for finder, e in xwin.items():
    if e.get('status') != 'OK' or not e.get('win_func'):
        continue
    fn = int(e['win_func'], 16)
    hits = addr2runs.get(fn, [])
    # prefer LONG runs (real class vtables)
    hits.sort(key=lambda h: -len(runs[h[0]][1]))
    out[finder] = {
        "win_runs": [(rid, idx, len(runs[rid][1]), hex(runs[rid][0])) for rid, idx, _ in hits[:6]],
    }
    print(f"{finder}: {len(hits)} runs; top idx={hits[0][1] if hits else None} len={len(runs[hits[0][0]][1]) if hits else 0}")

# save runs too (for structural matching)
json.dump({
    "targets": out,
    "runs": [[hex(rv), [hex(m) for m in mem]] for rv, mem in runs],
}, open('/home/z/re-analysis/mega/xwin_vt.json', 'w'))
print("saved xwin_vt.json")
