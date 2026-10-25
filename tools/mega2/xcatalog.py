#!/usr/bin/env python3
"""XCATALOG v2 — Windows 21.30 targets for Tier-3 functions.
- pattern-based: verify hit is function start (or apply Erbium's backscan)
- string-based: anchored_funcs_x64 + backscan to function start (Erbium semantics)
- special: EncryptionPatch (patch site), OnItemInstanceAddedVft (vtable call offset)
Output: xwin.json"""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from x64tool import X64

cat = json.load(open('/home/z/re-analysis/mega/catalog.json'))
x = X64.get()

# ---------- windows backscan to function start (Erbium: scan back for prologue) ----------
PROLOGOS = [(b"\x48\x89\x5C", 0), (b"\x48\x8B\xC4", 0), (b"\x40\x53", 0), (b"\x40\x55", 0),
            (b"\x48\x83\xEC", 0), (b"\x4C\x8B\xDC", 0), (b"\x48\x8B\xC4", 0), (b"\x55", 0), (b"\x53", 0)]

def backscan_func_start(site, maxback=0x2000, skip=0):
    """scan backwards from site for function prologue bytes; skip N candidates"""
    off = x.va2off(site)
    buf = x.full
    found = 0
    for i in range(0, maxback):
        o = off - i
        if o < 0:
            break
        # candidate: preceded by RET/INT3 or padding
        prev = buf[o-1] if o > 0 else 0
        if prev in (0xC3, 0xCC, 0x90, 0x00):
            for sig, _ in PROLOGOS:
                if buf[o:o+len(sig)] == sig:
                    va = x.off2va(o)
                    if x.func_of(va) == va:  # is pdata function start
                        if found < skip:
                            found += 1
                            continue
                        return va
    return None

def win_features(fn):
    start_off = x.va2off(fn)
    end_off = x.va2off(x.func_end(fn))
    res_str, calls = {}, []
    buf = x.full
    lim = min(end_off, start_off + 0x9000)
    for o in range(start_off, lim - 6):
        b0, b1, b2 = buf[o], buf[o+1], buf[o+2]
        if b0 in (0x48, 0x4C, 0x49, 0x4D) and b1 in (0x8D, 0x8B) and (b2 & 0xC7) == 0x05:
            disp = struct.unpack_from("<i", buf, o + 3)[0]
            tgt = x.off2va(o) + 7 + disp
            vo = x.va2off(tgt)
            if vo is None: continue
            raw = x.full[vo:vo + 280]
            s = None
            a = raw.split(b"\0")[0]
            if len(a) >= 6 and all(32 <= c < 127 for c in a[:32]):
                s = a.decode(errors="replace")
            else:
                try:
                    wtxt = raw.decode("utf-16-le", errors="strict").split("\0")[0]
                    if len(wtxt) >= 6 and all(32 <= ord(c) < 127 for c in wtxt[:32]):
                        s = wtxt
                except Exception:
                    pass
            if s:
                res_str.setdefault(s, tgt)
        if b0 == 0xE8:
            rel = struct.unpack_from("<i", buf, o + 1)[0]
            calls.append(x.off2va(o) + 5 + rel)
    return res_str, calls

NX = json.load(open('/home/z/re-analysis/mega/natives_x64.json'))
thunk2name = {}
for name, v in NX.items():
    if isinstance(v.get('x64_thunk'), list):
        for t in v['x64_thunk']:
            thunk2name.setdefault(int(t, 16), []).append(name)

def analyze(finder, fn, extra=None):
    strings, calls = win_features(fn)
    natives = []
    for tgt in calls:
        if tgt in thunk2name:
            natives += thunk2name[tgt]
    d = {"status": "OK", "win_func": hex(fn), "size": hex(x.func_size(fn)),
         "strings": sorted(strings.keys())[:30],
         "string_vas": {k: hex(v) for k, v in list(strings.items())[:30]},
         "natives": sorted(set(natives))[:40], "n_calls": len(calls)}
    if extra:
        d.update(extra)
    return d

out = {}

# ============ pattern-based (with function-start verification) ============
PATS = {
    "FindCantBuild": None, "FindClientHasInitializedLevelFor": None,
    "FindConstructAbilitySpec": None, "FindSetPickupItems": None,
    "FindCanPlaceBuildableClassInStructuralGrid": None,
    "FindMinigameSettingsBuilding__BeginPlay": None,
    "FindIsNetReady": None, "FindCreateNetDriver": None,
    "FindCreateNetDriverWorldContext": None, "FindSetChannelActor": None,
    "FindSendClientAdjustment": None, "FindUpdateIrisReplicationViews": None,
    "FindInitListen": None, "FindReset": None, "FindSetGamePhase": None,
    "FindOnItemInstanceAddedVft": None, "FindEncryptionPatch": None,
    "FindInitializeFlightPath": None,
}

for name in PATS:
    e = cat.get(name, {})
    pats = [p for p in e.get('patterns', []) if '// ' not in p and len(p) > 10]
    best = None
    for p in pats:
        hits = x.scan_pattern(p, limit=8)
        # verify: prefer hit that IS a function start (pdata)
        for h in hits:
            f = x.func_of(h)
            if f == h:
                best = (h, p, len(hits))
                break
        if best:
            break
        # fallback: backscan from first hit
        if hits:
            f = backscan_func_start(hits[0], skip=0)
            if f:
                best = (f, p, len(hits))
                break
    if best is None:
        out[name] = {"status": "no_x64_match", "n_pats": len(pats)}
        print(f"{name}: NO match")
        continue
    fn, p, nhits = best
    out[name] = analyze(name, fn, {"x64_pattern": p, "n_hits": nhits})
    print(f"{name}: {hex(fn)} sz={out[name]['size']} hits={nhits} strs={len(out[name]['strings'])} nats={len(out[name]['natives'])}")

# ============ string-based (Windows side) ============
STRS = {
    "FindFlushDormancy": ["FlushDormancy: %s. Connection: %s"],
    "FindStartBecomingDormant": ["StartBecomingDormant: %s"],
    "FindUpdateSafeZonesPhase": ["AFortGameModeAthena::UpdateSafeZonesPhase"],
    "FindSpawnLoot": ["ABuildingContainer::SpawnLoot() called on %s (%s)...", "ABuildingContainer::SpawnLoot() called on %s (%s)", "ABuildingContainer::SpawnLoot() >>> START called on %s (%s)"],
    "FindPickSupplyDropLocation": ["PickSupplyDropLocation: Failed to find valid location using rejection.  Using safe zone location.", "AFortAthenaMapInfo::PickSupplyDropLocation"],
    "FindSpawnInitialSafeZone": ["FortGameModeAthena::SpawnInitialSafeZone bShouldSpawnSafeZoneIndicator == false"],
    "FindClearAbility": None,  # derived from GiveAbilityAndActivateOnce body — skip here
}

for name, strs in STRS.items():
    if strs is None:
        continue
    got = None
    for s in strs:
        fns = x.anchored_funcs_x64(s)
        if fns:
            # smallest function containing (Erbium scans back to start anyway)
            f = min(fns.keys(), key=lambda a: x.func_size(a))
            got = analyze(name, f, {"win_string": s})
            break
    out[name] = got or {"status": "no_string_win"}
    print(f"{name}: {out[name].get('win_func','—')} ({out[name]['status']})")

# ============ specials ============
# EncryptionPatch: pattern hit = the patch byte (0x7F jg). Find containing function + site.
pats = [p for p in cat.get("FindEncryptionPatch", {}).get('patterns', []) if '// ' not in p]
for p in pats:
    hits = x.scan_pattern(p, limit=5)
    if hits:
        site = hits[0]
        # find the 0x7F within the match (patch target)
        off = x.va2off(site)
        seg = x.full[off:off+16]
        idx = seg.find(b"\x7F")
        patch_va = site + idx
        fn = x.func_of(patch_va)
        out["FindEncryptionPatch"] = {
            "status": "OK", "patch_site": hex(patch_va), "win_func": hex(fn),
            "size": hex(x.func_size(fn)), "x64_pattern": p}
        strs, calls = win_features(fn)
        out["FindEncryptionPatch"]["strings"] = sorted(strs.keys())[:20]
        print(f"FindEncryptionPatch: site={hex(patch_va)} in {hex(fn)}")
        break

# OnItemInstanceAddedVft: extract the vtable call offset from matched pattern
pats = [p for p in cat.get("FindOnItemInstanceAddedVft", {}).get('patterns', []) if '// ' not in p]
for p in pats:
    hits = x.scan_pattern(p, limit=5)
    if hits:
        site = hits[0]
        off = x.va2off(site)
        # pattern: 41 57 48 83 EC ? 48 8B 01 4C 8B F2 48 8B F1 FF 90 XX XX XX XX
        # find FF 90 within
        seg = x.full[off:off+40]
        i = seg.find(b"\xFF\x90")
        if i >= 0:
            vt_off = struct.unpack_from("<I", seg, i + 2)[0]
            fn = backscan_func_start(site)
            out["FindOnItemInstanceAddedVft"] = {
                "status": "OK", "vtable_byte_offset": vt_off, "vtable_index": vt_off // 8,
                "win_infunc": hex(site), "win_func": hex(fn) if fn else None,
                "x64_pattern": p}
            print(f"FindOnItemInstanceAddedVft: inFunc={hex(site)} vt_off={hex(vt_off)} idx={vt_off//8}")
            break

json.dump(out, open('/home/z/re-analysis/mega/xwin.json', 'w'), indent=1)
print("\nsaved xwin.json")
