#!/usr/bin/env python3
"""Full 27.11 ground-truth extraction: natives, vtables, function bounds (.pdata),
string xrefs for Erbium hard functions. Saves to out/win-27.11-gt.json"""
import json
import struct
import sys

sys.path.insert(0, '/home/z/re-analysis/tools')
from pe_analysis import PEAnalyzer

exe = sys.argv[1] if len(sys.argv) > 1 else 'binaries/win/FortniteClient-27.11.exe'
a = PEAnalyzer(exe)
ver = a.detect_version()
print(f'[{exe}] version {ver} CL {a.cl}')

# 1) function bounds from .pdata (RUNTIME_FUNCTION array)
pdata = a.sections.get('.pdata')
funcs = []
if pdata:
    off, end = pdata['raw'], pdata['raw'] + pdata['rsize']
    d = a.data
    while off + 12 <= end:
        s, e, u = struct.unpack_from('<III', d, off)
        if s == 0 and e == 0:
            break
        funcs.append((s, e))
        off += 12
print('function bounds from .pdata:', len(funcs))

fbounds = {}  # start_rva -> size
for s, e in funcs:
    fbounds[s] = e - s

def func_of(rva):
    """Return (start, size) of function containing rva, or None."""
    # binary search
    import bisect
    starts = func_starts
    i = bisect.bisect_right(starts, rva) - 1
    if i >= 0:
        s = starts[i]
        e = s + fbounds[s]
        if s <= rva < e:
            return s, e - s
    return None

import bisect
func_starts = sorted(fbounds.keys())

# 2) natives
natives = a.extract_natives()
print('natives:', len(natives))

# 3) vtables (RTTI)
vtables = a.extract_vtables()
print('vtables:', len(vtables))

# 4) probe the Erbium hard functions
HARD_STRINGS = {
    'FindUpdateSafeZonesPhase': ('AFortGameModeAthena::UpdateSafeZonesPhase', True),
    'FindPreSendUpdate': ('ReplicationSystem_PreSendUpdate', True),
    'FindFlushDormancy': ('FlushDormancy: %s. Connection: %s', True),
    'FindStartBecomingDormant': ('StartBecomingDormant: %s', True),
    'FindEnterAircraft': ('EnterAircraft: [%s] is attempting to enter aircraft after having already exited.', True),
    'FindHandleMatchHasStarted': ('AFortGameModeAthena::HandleMatchHasStarted: NumPlayers: %i', True),
    'FindSpawnInitialSafeZone': ('FortGameModeAthena::SpawnInitialSafeZone bShouldSpawnSafeZoneIndicator == false, skipping spawning Safe Zone Indicator', True),
    'FindSelectAndSetupMyBuildingLevel': ('AFortPlayerControllerAthena::SelectAndSetupMyBuildingLevel', False),
    'FindPickSupplyDropLocation': ('PickSupplyDropLocation: Attempting to pick spawn location for air drop in zone', False),
    'FindUpdateSafeZonesPhase2': ('UpdateSafeZonesPhase', True),
}
hard = {}
for name, (s, wide) in HARD_STRINGS.items():
    refs = a.find_string_xrefs(s, wide=wide)
    hard[name] = {'string': s, 'refs': [hex(r) for r in refs[:20]], 'nrefs': len(refs)}
    print(f'{name}: string xrefs={len(refs)} -> {[hex(r) for r in refs[:6]]}')

out = {
    'exe': exe,
    'version': ver,
    'cl': a.cl,
    'image_base': a.image_base,
    'sections': {k: {kk: hex(vv) if isinstance(vv, int) else vv for kk, vv in v.items()} for k, v in a.sections.items()},
    'n_natives': len(natives),
    'n_vtables': len(vtables),
    'n_funcs': len(funcs),
    'hard_strings': hard,
}

# save heavy data separately
with open('/home/z/re-analysis/out/win-natives-27.11.json', 'w') as f:
    json.dump({k: hex(v) for k, v in natives.items()}, f)
with open('/home/z/re-analysis/out/win-vtables-27.11.json', 'w') as f:
    json.dump({k: {'rva': hex(v['rva']), 'methods': [hex(m) for m in v['methods']]} for k, v in vtables.items()}, f)
with open('/home/z/re-analysis/out/win-funcs-27.11.json', 'w') as f:
    json.dump([[hex(s), hex(e - s)] for s, e in funcs], f)

# vtable index knowledge base for Erbium: which classes and indices matter
VT_PROBES = {
    'FindOnItemInstanceAddedVft': ('FortItem', None),
    'FindIsNetRelevantForVft': ('ActorChannel', None),
    'FindSpawnDecoVft': ('FortDecoTool', None),
    'FindCloseActorChannel': ('ActorChannel', None),
    'FindInitListen': ('IpConnection', None),
    'FindTickFlush': ('NetDriver', None),
    'FindStartBecomingDormant': ('ActorChannel', None),
    'FindFlushDormancy': ('NetConnection', None),
    'FindIsNetReady': ('NetConnection', None),
    'FindSendClientAdjustment': ('PlayerController', None),
    'FindCallPreReplication': ('ActorChannel', None),
}
vt_info = {}
for fn, (cls, _) in VT_PROBES.items():
    # find classes whose name contains the probe
    matches = {k: v for k, v in vtables.items() if cls.lower() in k.lower()}
    vt_info[fn] = {k: {'rva': hex(v['rva']), 'n': len(v['methods'])} for k, v in list(matches.items())[:12]}
    print(f'VT {fn} [{cls}]: {len(matches)} classes, e.g. {list(matches.keys())[:5]}')

out['vt_probes'] = vt_info
with open('/home/z/re-analysis/out/win-27.11-gt.json', 'w') as f:
    json.dump(out, f, indent=1)
print('saved out/win-27.11-gt.json + natives + vtables + funcs')
