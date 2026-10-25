#!/usr/bin/env python3
"""XANCHORS — Windows addresses of anchor functions (resolved-47 with x64 patterns).
Deltas: UNetConnection virtual slot distances from UE 5.0 source declaration order.
Output: xanchors.json"""
import sys, json, re, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from x64tool import X64

cat = json.load(open('/home/z/re-analysis/mega/catalog.json'))
x = X64.get()

ANCHORS = ["FindCloseActorChannel", "FindTickFlush", "FindReplicateActor",
           "FindCallPreReplication", "FindSetChannelActorForDestroy", "FindGetPlayerViewPoint",
           "FindServerReplicateActors", "FindCreateChannel", "FindSendRequestNow",
           "FindGiveAbilityAndActivateOnce", "FindSetPickupTarget", "FindHandleMatchHasStarted",
           "FindUpdateSafeZonesPhase", "FindHandlePostSafeZonePhaseChanged"]

out = {}
for name in ANCHORS:
    e = cat.get(name, {})
    pats = [p for p in e.get('patterns', []) if '// ' not in p and len(p) > 12]
    strs = e.get('strings', [])
    found = None
    for p in pats:
        hits = x.scan_pattern(p, limit=5)
        ok = [h for h in hits if x.func_of(h) == h]
        if ok:
            found = ('pattern', ok[0], p)
            break
        if hits:
            # backscan
            off = x.va2off(hits[0])
            for i in range(0, 0x2000):
                o = off - i
                if o > 0 and x.full[o-1] in (0xC3, 0xCC, 0x90):
                    va = x.off2va(o)
                    if va and x.func_of(va) == va and x.full[o:o+3] in (b"\x48\x89\x5C", b"\x48\x8B\xC4", b"\x40\x53", b"\x40\x55", b"\x4C\x8B\xDC"):
                        found = ('pattern+backscan', va, p)
                        break
            if found:
                break
    if not found and strs:
        for s in strs:
            fns = x.anchored_funcs_x64(s[:40])
            if fns:
                f = min(fns.keys(), key=lambda a: x.func_size(a))
                found = ('string', f, s[:40])
                break
    if found:
        kind, va, meta = found
        out[name] = {"win": hex(va), "kind": kind, "size": hex(x.func_size(va)), "meta": meta[:60]}
        print(f"{name}: {hex(va)} ({kind}) sz={hex(x.func_size(va))}")
    else:
        out[name] = {"status": "not_found"}
        print(f"{name}: NOT FOUND ({len(pats)} pats, {len(strs)} strs)")

# ---- UNetConnection virtual deltas from UE 5.0 source ----
SRC = "/tmp/work2/src/UnrealEngine-5.0.0-release/Engine/Source/Runtime/Engine/Classes/Engine/NetConnection.h"
lines = open(SRC, errors='ignore').read().splitlines()
# find lines of the three functions
marks = {}
for i, ln in enumerate(lines):
    if "IsNetReady(" in ln and "virtual" in ln:
        marks.setdefault("IsNetReady", i)
    if "ClientHasInitializedLevelFor(" in ln and "virtual" in ln:
        marks.setdefault("ClientHasInitializedLevelFor", i)
    if "FlushDormancy(" in ln and "virtual" in ln:
        marks.setdefault("FlushDormancy", i)
print("\nUNetConnection virtual decl lines:", marks)

# count NEW virtual declarations (override keyword present → not new slot;
# '= 0'/PURE_VIRTUAL → new slot; plain virtual → new slot)
VIRT = re.compile(r'^\s*(ENGINE_API\s+)?virtual\s+.*\(')
def count_new_virtuals(a, b):
    n = 0
    for ln in lines[a+1:b]:
        if VIRT.match(ln):
            n += 1  # count every virtual decl between; override vs new doesn't matter for DELTA
            # (deltas between virtuals in the SAME class = count of virtuals between them,
            #  regardless of override — overrides occupy the same slot as base, so this
            #  over-counts if overrides of BASE virtuals are redeclared... but redeclaration
            #  with 'override' does NOT add a slot. Filter those:)
            if "override" in ln:
                n -= 1
    return n

if len(marks) == 3:
    a, c, f = marks["IsNetReady"], marks["ClientHasInitializedLevelFor"], marks["FlushDormancy"]
    d1 = count_new_virtuals(a, c)   # slots between IsNetReady → ClientHasInitializedLevelFor
    d2 = count_new_virtuals(c, f)   # ClientHasInitializedLevelFor → FlushDormancy
    out["_unetconnection_deltas"] = {
        "IsNetReady_to_ClientHasInitializedLevelFor": d1,
        "ClientHasInitializedLevelFor_to_FlushDormancy": d2,
    }
    print(f"deltas: IsNetReady→CHILF={d1}, CHILF→FlushDormancy={d2}")

json.dump(out, open('/home/z/re-analysis/mega/xanchors.json', 'w'), indent=1)
print("saved xanchors.json")
