#!/usr/bin/env python3
"""Run the universal solver on ALL versions (pattern mode + pure-adaptive
universality mode) — one process per version (memory limits)."""
import subprocess, sys, os, json, time

VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10",
            "21.30", "22.40", "23.50", "30.00"]
SO = {}
for v in VERSIONS:
    for nm in ("libUnreal.so", "libUE4.so"):
        p = f"/tmp/work2/src/{v}/{nm}"
        if os.path.exists(p):
            SO[v] = p
            break

OUT = "results"
os.makedirs(OUT, exist_ok=True)
summary = {}
for v in VERSIONS:
    if v not in SO:
        continue
    t0 = time.time()
    # 1) pattern mode
    r = subprocess.run([sys.executable, "erbium_solver.py", SO[v],
                        "--json", f"{OUT}/solver_{v}.json"],
                       capture_output=True, text=True, timeout=900)
    d = json.load(open(f"{OUT}/solver_{v}.json")) if os.path.exists(f"{OUT}/solver_{v}.json") else {}
    pat_resolved = d.get("resolved", 0)
    # offsets-only file
    r2 = subprocess.run([sys.executable, "erbium_solver.py", SO[v], "--offsets-only"],
                        capture_output=True, text=True, timeout=900)
    open(f"{OUT}/ERBIUM-OFFSETS-{v}.txt", "w").write(r2.stdout)
    # 2) universality mode (no patterns — simulates an UNSEEN version)
    r3 = subprocess.run([sys.executable, "erbium_solver.py", SO[v], "--no-patterns",
                         "--json", f"{OUT}/universal_{v}.json"],
                        capture_output=True, text=True, timeout=900)
    u = json.load(open(f"{OUT}/universal_{v}.json")) if os.path.exists(f"{OUT}/universal_{v}.json") else {}
    uni_resolved = u.get("resolved", 0)
    summary[v] = {"pattern_mode": pat_resolved, "universal_mode": uni_resolved,
                  "total": d.get("total", 94), "secs": round(time.time() - t0)}
    json.dump(summary, open(f"{OUT}/summary.json", "w"), indent=1)
    print(f"[{v}] patterns: {pat_resolved}/{d.get('total',94)}  "
          f"universal(no-patterns): {uni_resolved}/{u.get('total',94)}  "
          f"({summary[v]['secs']}s)", flush=True)
print("\n=== SUMMARY ===")
tp = sum(s["pattern_mode"] for s in summary.values())
tu = sum(s["universal_mode"] for s in summary.values())
tt = sum(s["total"] for s in summary.values())
print(f"pattern mode:    {tp}/{tt} ({100*tp/tt:.0f}%)")
print(f"universal mode:  {tu}/{tt} ({100*tu/tt:.0f}%)")
