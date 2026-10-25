#!/usr/bin/env python3
"""MERGE solver results (all versions) into final_resolved_v4.json and
VALIDATE every solver resolution against the v3 ground truth:
  - MATCH      solver addr == v3 addr (independent confirmation)
  - NEW        no v3 address → gap filled
  - CONFLICT   solver addr != v3 addr → v3 wins (flagged for review)

Also emits the v4 coverage matrix.
"""
import json, os, glob
from collections import Counter

STAGE = "/home/z/repo/tools/solver/results"
V3 = json.load(open('/home/z/repo/erbium-finders/final_resolved_v3.json'))
VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10",
            "21.30", "22.40", "23.50", "30.00"]

out = {}
stats = Counter()
conflicts = []
new_finds = {}

for v in VERSIONS:
    p = f"{STAGE}/solver_{v}.json"
    if not os.path.exists(p):
        continue
    d = json.load(open(p))
    for row in d["results"]:
        name = row["name"]
        if row.get("type") == "vtable_index":
            continue  # vtable indices handled separately
        addr = int(row["addr"], 16)
        is_global = row.get("type") == "global"
        v3cell = V3.get(name, {}).get(v)
        v3addr = None
        v3global = None
        if isinstance(v3cell, dict):
            if v3cell.get("addr"):
                v3addr = int(v3cell["addr"], 16)
            if v3cell.get("global"):
                v3global = int(v3cell["global"], 16)
        rec = {
            ("global" if is_global else "addr"): hex(addr),
            "strategy": row["strategy"],
            "verify": row.get("verify", ""),
            "confidence": row.get("confidence", 0),
        }
        if is_global and v3global is not None:
            if addr == v3global:
                rec["match"] = "MATCH"
                stats[f"{v}:match"] += 1
            else:
                rec["match"] = "CONFLICT"
                rec["v3"] = hex(v3global)
                conflicts.append((v, name, hex(addr), hex(v3global)))
                stats[f"{v}:conflict"] += 1
        elif not is_global and v3addr is not None:
            if addr == v3addr:
                rec["match"] = "MATCH"
                stats[f"{v}:match"] += 1
            else:
                rec["match"] = "CONFLICT"
                rec["v3"] = hex(v3addr)
                conflicts.append((v, name, hex(addr), hex(v3addr)))
                stats[f"{v}:conflict"] += 1
        elif not is_global and v3addr is None and not is_global:
            rec["match"] = "NEW"
            new_finds.setdefault(name, {})[v] = hex(addr)
            stats[f"{v}:new"] += 1
        out.setdefault(name, {})[v] = rec

# merge v3 cells that the solver didn't produce (unresolved on that version)
for name, cells in V3.items():
    for v, cell in cells.items():
        if v not in VERSIONS or not isinstance(cell, dict):
            continue
        if v in out.get(name, {}):
            continue
        if cell.get("addr") or cell.get("global") or cell.get("vtable_index") is not None:
            merged = dict(cell)
            merged["match"] = "V3-ONLY"
            out.setdefault(name, {})[v] = merged
            stats[f"{v}:v3only"] += 1

json.dump(out, open(f"{STAGE}/final_resolved_v4.json", "w"), indent=1)

# coverage
total = len(out) * len(VERSIONS)
filled = sum(1 for cells in out.values() for v in VERSIONS
             if cells.get(v) and (cells[v].get("addr") or cells[v].get("global")
                                  or cells[v].get("vtable_index") is not None))
print(f"v4 matrix: {filled}/{total} ({100*filled/total:.0f}%)  functions: {len(out)}")
print(f"conflicts (v3 wins): {len(conflicts)}")
for c in conflicts[:20]:
    print("  CONFLICT", c)
print(f"NEW resolutions (gaps filled): {sum(len(v) for v in new_finds.values())}")
for name, vers in sorted(new_finds.items(), key=lambda x: -len(x[1])):
    print(f"  NEW {name}: {len(vers)} versions → {sorted(vers.keys())}")
