#!/usr/bin/env python3
"""Vtable-run anchoring: x64 vtables from .reloc (DIR64 relocations), ARM vtables from rels.
Matching rule: if a matched function pair (a,b) sits in x64-run R_x and arm-run R_a,
then x64 function at position i in R_x ↔ arm function at position i in R_a
(virtual order preserved across ABIs for single inheritance)."""
import sys, os, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from x64tool import X64

def x64_vtable_runs(x):
    """scan .rdata/.data for absolute code pointers (qwords → .text); group into consecutive runs.
    File contains preferred-base absolute VAs (ASLR handled at load)."""
    data = x.full
    text_lo, text_hi = x.text_va, x.text_va + len(x.text)
    ptrs = []
    for sec in ('.rdata', '.data', '_RDATA'):
        if sec not in x.sections:
            continue
        vaddr, vsize, roff, rsize = x.sections[sec]
        arr = np.frombuffer(data, dtype=np.uint64, count=rsize // 8, offset=roff)
        base_va = x.base + vaddr
        mask = (arr >= text_lo) & (arr < text_hi)
        hit = np.flatnonzero(mask)
        for i in hit:
            ptrs.append((base_va + int(i) * 8, int(arr[i])))
    ptrs.sort()
    # group consecutive slots (stride 8)
    runs = []
    cur = []
    last_slot = None
    for slot, tgt in ptrs:
        if last_slot is not None and slot - last_slot == 8:
            cur.append((slot, tgt))
        else:
            if len(cur) >= 4:
                runs.append(cur)
            cur = [(slot, tgt)]
        last_slot = slot
    if len(cur) >= 4:
        runs.append(cur)
    return runs


def arm_vtable_runs(ctx):
    """ARM vtable runs from rels dump (slot, func) sorted, consecutive stride 8"""
    rels = ctx.tool.rels  # (off=slot, add=func)
    rels = rels[rels[:, 0].argsort()]
    runs = []
    cur = []
    last = None
    tv = ctx.text_va
    ts = ctx.text_size
    for slot, func in rels:
        slot, func = int(slot), int(func)
        if not (tv <= func < tv + ts):
            continue
        if last is not None and slot - last == 8:
            cur.append((slot, func))
        else:
            if len(cur) >= 4:
                runs.append(cur)
            cur = [(slot, func)]
        last = slot
    if len(cur) >= 4:
        runs.append(cur)
    return runs


class VTableAnchor:
    def __init__(self, x, ctx):
        self.x = x
        self.ctx = ctx
        self.x_runs = x64_vtable_runs(x)
        self.a_runs = arm_vtable_runs(ctx)
        # index: func → [(run_idx, pos)]
        self.x_fpos = {}
        for ri, run in enumerate(self.x_runs):
            for pos, (slot, f) in enumerate(run):
                self.x_fpos.setdefault(f, []).append((ri, pos))
        self.a_fpos = {}
        for ri, run in enumerate(self.a_runs):
            for pos, (slot, f) in enumerate(run):
                self.a_fpos.setdefault(f, []).append((ri, pos))
        # run pair candidates: from matched seeds
        self.run_pairs = {}  # (x_run, a_run) -> [matched positions]

    def feed_seed(self, a, b):
        for xr, xp in self.x_fpos.get(a, []):
            for ar, ap in self.a_fpos.get(b, []):
                self.run_pairs.setdefault((xr, ar), []).append((xp, ap))

    def candidate_for(self, f):
        """x64 func f → ARM candidates via vtable position mapping"""
        out = []
        for xr, xp in self.x_fpos.get(f, []):
            for (xrr, arr), pospairs in self.run_pairs.items():
                if xrr != xr:
                    continue
                # offset consistency: use median offset between matched positions
                offs = [ap - xpp for xpp, ap in pospairs]
                if not offs:
                    continue
                # require ≥1 seed and consistent offsets (all equal ±1)
                from collections import Counter
                cnt = Counter(offs)
                off, n = cnt.most_common(1)[0]
                if n < len(offs) * 0.5:
                    continue
                ap = xp + off
                if 0 <= ap < len(self.a_runs[arr]):
                    out.append((self.a_runs[arr][ap][1], n, off))
        return out


if __name__ == '__main__':
    import time
    t0 = time.time()
    x = X64.get()
    from engine import VersionCtx
    ctx = VersionCtx('21.30')
    vt = VTableAnchor(x, ctx)
    print(f"x64 runs: {len(vt.x_runs)} | arm runs: {len(vt.a_runs)} ({time.time()-t0:.0f}s)", flush=True)
    # feed with native seeds
    from wave2b import X64Ctx, ArmCtx, Matcher
    from x2a_run import seed_natives
    xa = X64Ctx.__new__(X64Ctx)  # reuse x
    xa.x = x
    # rebuild xa quickly (edges already built?) — instead just use x funcs:
    # simpler: load map from x2a_map.json
    m = json.load(open('/home/z/re-analysis/mega/x2a_map.json'))
    n = 0
    for a, b in m.items():
        vt.feed_seed(int(a, 16), int(b, 16))
        n += 1
    print(f"fed {n} seeds → run_pairs: {len(vt.run_pairs)}", flush=True)
    # test: SetChannelActor x64=0x1418cbce0, SetWorld x64=0x141b4231c, SendClientAdjustment 0x148648098
    for name, f in [('SetChannelActor', 0x1418cbce0), ('SetWorld', 0x141b4231c),
                    ('SendClientAdjustment', 0x148648098), ('GetWorldContext', 0x140c9e030),
                    ('SetPickupItems', 0x146cc8d98), ('Reset', 0x14864786c)]:
        cands = vt.candidate_for(f)
        print(f"{name:24s} x64={hex(f)} vtable-cands: {[(hex(c), n, off) for c, n, off in cands[:4]]}", flush=True)
