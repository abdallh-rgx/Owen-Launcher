#!/usr/bin/env python3
"""Two-hop LogV2 resolution: string -> abs-ptr descriptor -> riprel code refs -> function.
Resolves the hard Erbium functions on modern Windows exes (27.11+)."""
import json
import struct
import sys

import capstone

sys.path.insert(0, '/home/z/re-analysis/tools')
from xref2 import PE, XrefEngine
from resolve_hard import full_string_range, decode_at


def resolve_logv2(pe, xe, needle, wide=True):
    """string -> descriptor ptr -> code refs."""
    out = []
    d = pe.d
    offs = pe.find_utf16(needle) if wide else pe.find_ascii(needle)
    seen = set()
    for off in offs:
        s, e = full_string_range(pe, off, wide)
        if s in seen:
            continue
        seen.add(s)
        full = decode_at(pe, s, wide)
        rva = pe.off_to_rva(s)
        if rva is None:
            continue
        va = pe.image_base + rva
        # find abs pointers to this string
        ptr = struct.pack('<Q', va)
        locs, start = [], 0
        while True:
            i = d.find(ptr, start)
            if i < 0:
                break
            locs.append(i)
            start = i + 1
        for loc in locs[:8]:
            loc_rva = pe.off_to_rva(loc)
            if loc_rva is None:
                continue
            # riprel refs to the descriptor location
            res = xe.scan_targets({loc_rva: f'd{loc_rva}'}, calls=False, riprel=True)
            refs = [v for vs in res.values() for v in vs]
            funcs = {}
            for ref in refs:
                f = pe.func_of(ref)
                if f:
                    funcs.setdefault(f[0], [0, f[1] - f[0], []])[0] += 1
                    funcs[f[0]][2].append(hex(ref))
            out.append({'string': full, 'desc_rva': hex(loc_rva), 'refs': [hex(r) for r in refs[:8]],
                        'funcs': {hex(k): v for k, v in funcs.items()}})
    return out


if __name__ == '__main__':
    exe = sys.argv[1] if len(sys.argv) > 1 else 'binaries/win/FortniteClient-27.11.exe'
    pe = PE(exe)
    ver_off = pe.d.find(b'Fortnite+Release-')
    print('exe:', pe.d[ver_off:ver_off + 40].split(b'\x00')[0].decode())
    xe = XrefEngine(pe)

    PROBES = [
        ('FindFlushDormancy', 'FlushDormancy: %s', True),
        ('FindStartBecomingDormant', 'StartBecomingDormant: %s', True),
        ('FindEnterAircraft', 'EnterAircraft: [%s]', True),
        ('FindHandleMatchHasStarted', 'HandleMatchHasStarted', True),
        ('FindSpawnInitialSafeZone', 'SpawnInitialSafeZone', True),
        ('FindSelectAndSetupMyBuildingLevel', 'SelectAndSetupMyBuildingLevel', True),
        ('FindUpdateSafeZonesPhase', 'UpdateSafeZonesPhase', True),
        ('FindPreSendUpdate', 'PreSendUpdate', True),
        ('FindSendClientAdjustment', 'SendClientAdjustment', True),
        ('FindGetWorldContext', 'GetWorldContext', True),
        ('FindQueueStatEvent', 'QueueStatEvent', True),
        ('FindClientHasInitializedLevelFor', 'ClientHasInitializedLevelFor', True),
        ('FindIsNetReady', 'IsNetReady', True),
        ('FindInitializeFlightPath', 'InitializeFlightPath', False),
        ('FindPayBuildableClassPlacementCost', 'pay building costs', True),
        ('FindCantBuild', 'build', True),
    ]
    report = {}
    for name, needle, wide in PROBES:
        rows = resolve_logv2(pe, xe, needle, wide)
        print(f'=== {name}')
        hit = False
        for r in rows:
            if not r['funcs']:
                continue
            hit = True
            print(f"  str {r['string'][:80]!r}")
            print(f"    desc@{r['desc_rva']} refs={len(r['refs'])}")
            for fs, (cnt, sz, rfs) in sorted(r['funcs'].items(), key=lambda x: -x[1][0])[:6]:
                print(f'      -> func {fs} size {hex(sz)} x{cnt}')
        if not hit:
            print('  (no descriptor refs)')
        report[name] = rows
    with open('/home/z/re-analysis/out/hard-logv2-27.11.json', 'w') as f:
        json.dump(report, f, indent=1)
    print('saved out/hard-logv2-27.11.json')
