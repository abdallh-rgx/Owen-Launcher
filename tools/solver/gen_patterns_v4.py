#!/usr/bin/env python3
"""Generate verified ARM64 patterns for the NEW v4 cells (gaps the solver
filled).  One binary loaded at a time; patterns verified unique-in-.text
(same P0 methodology as all waves)."""
import sys, os, json, struct, re
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from so_analysis import SoAnalysis

VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10",
            "21.30", "22.40", "23.50", "30.00"]
STAGE = "/home/z/repo/tools/solver/results"
V4 = json.load(open(f"{STAGE}/final_resolved_v4.json"))

def so_path(v):
    for nm in ("libUnreal.so", "libUE4.so"):
        p = f"/tmp/work2/src/{v}/{nm}"
        if os.path.exists(p):
            return p
    return None


def pat_word_mask(w):
    if (w & 0x9F000000) == 0x90000000:   # ADRP
        return (w & 0x9F00001F), 0x9F00001F
    if (w & 0x9F000000) == 0x10000000:   # ADR
        return (w & 0x9F00001F), 0x9F00001F
    if (w & 0x7C000000) == 0x14000000:   # B/BL
        return (w & 0xFC000000), 0xFC000000
    if (w & 0xFF000010) == 0x54000000:   # B.cond
        return (w & 0xFF00000F), 0xFF00000F
    if (w & 0x7E000000) == 0x34000000:   # CBZ/CBNZ
        return (w & 0xFF00001F), 0xFF00001F
    if (w & 0x7E000000) == 0x36000000:   # TBZ/TBNZ
        return (w & 0xFFF8001F), 0xFFF8001F
    return (w, 0xFFFFFFFF)


def render(pairs):
    out = []
    for w, m in pairs:
        b = struct.pack("<I", w)
        mb = struct.pack("<I", m)
        for i in range(4):
            out.append(f"{b[i]:02X}" if mb[i] == 0xFF else "??")
    return " ".join(out)


def gen_pattern(a, fn, max_ins=44):
    """smallest unique window pattern for function fn (starts at fn)."""
    sz = a.func_size(fn)
    n_total = min(sz // 4, max_ins)
    if n_total < 4:
        return None
    off = a.va2off(fn)
    words = struct.unpack_from(f"<{n_total}I", a.elf, off)
    pairs = [pat_word_mask(w) for w in words]
    # ICF-stub check
    if n_total <= 2 and (words[0] & 0xFFFFFC1F) == 0xD65F0000:
        return None
    for n in range(8, n_total + 1, 2):
        pat = render(pairs[:n])
        hits = a.scan_pattern(pat, limit=3)
        if len(hits) == 1 and hits[0] == fn:
            return {"pattern": pat, "n_ins": n, "unique": True}
        if len(hits) == 0:
            break
    pat = render(pairs)
    hits = a.scan_pattern(pat, limit=5)
    if len(hits) == 1 and hits[0] == fn:
        return {"pattern": pat, "n_ins": n_total, "unique": True}
    # windowed: try interior windows
    for start in range(0, max(1, n_total - 8), 2):
        for n in range(min(24, n_total - start), 7, -2):
            pat = render(pairs[start:start + n])
            hits = a.scan_pattern(pat, limit=3)
            if len(hits) == 1 and hits[0] - fn == start * 4:
                return {"pattern": pat, "offset": start * 4, "n_ins": n, "unique": True}
    return None


out = {}
for v in VERSIONS:
    # collect NEW cells for this version
    todo = {}
    for fn, cells in V4.items():
        c = cells.get(v)
        if isinstance(c, dict) and c.get("match") == "NEW" and c.get("addr"):
            todo[fn] = int(c["addr"], 16)
    if not todo:
        continue
    a = SoAnalysis(so_path(v))
    res = {}
    for fn, addr in todo.items():
        if not a.is_func(addr):
            res[fn] = {"addr": hex(addr), "pattern": None, "why": "not-a-func-start"}
            continue
        p = gen_pattern(a, addr)
        if p:
            res[fn] = {"addr": hex(addr), "pattern": p["pattern"],
                       "offset": p.get("offset", 0), "unique": True}
        else:
            res[fn] = {"addr": hex(addr), "pattern": None, "why": "no-unique-window"}
    out[v] = res
    n_ok = sum(1 for r in res.values() if r["pattern"])
    print(f"[{v}] {n_ok}/{len(res)} new cells patterned", flush=True)
    del a

json.dump(out, open(f"{STAGE}/new_patterns_v4.json", "w"), indent=1)
total_ok = sum(1 for vers in out.values() for r in vers.values() if r["pattern"])
total = sum(len(vers) for vers in out.values())
print(f"TOTAL: {total_ok}/{total} NEW cells with unique patterns")
