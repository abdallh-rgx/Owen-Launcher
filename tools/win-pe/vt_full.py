#!/usr/bin/env python3
"""Consolidated vtable index extraction for Windows exes.
Anchors = x64 patterns + direct string refs + LogV2 descriptor refs.
Output: per-version vtable membership + deltas + full vtable dumps for key classes."""
import glob
import json
import struct
import sys

sys.path.insert(0, '/home/z/re-analysis/tools')
from xref2 import PE
from vt_extract import find_vtables_with, vtable_methods

# manually curated best candidates from the resolution runs
MANUAL_2711 = {
    'FindFlushDormancy': 0x73dac7c,
    'FindStartBecomingDormant': 0x71fa978,
    'FindEnterAircraft': 0x958f200,
    'FindHandleMatchHasStarted': 0x90f5ee0,
    'FindSelectAndSetupMyBuildingLevel': 0x944cc64,
    'FindQueueStatEvent': 0xa1652d4,
    'FindSetGamePhase': 0xa4bd354,
    'FindFinishedTargetSpline': 0x5cc414e,
    'FindPickSupplyDropLocation': 0x9073524,
    'FindPayBuildableClassPlacementCost': 0xa03c4f0,
}
MANUAL_2130 = {
    'FindSetGamePhase': 0x6701a24,
    'FindFinishedTargetSpline': 0x5162560,
    'FindPickSupplyDropLocation': 0x667bce4,
}


def load_anchors(ver):
    anchors = {}
    try:
        p = json.load(open(f'/home/z/re-analysis/out/win-x64pat-{ver}.json'))
        for k, v in p['found'].items():
            anchors[k] = int(v['rva'], 16)
    except Exception as e:
        print('x64pat load fail', e)
    for k, v in (MANUAL_2711 if ver == '27.11' else MANUAL_2130 if ver == '21.30' else {}).items():
        anchors.setdefault(k, v)
    return anchors


def main(exe):
    pe = PE(exe)
    ver_off = pe.d.find(b'Fortnite+Release-')
    ver = pe.d[ver_off:ver_off + 40].split(b'\x00')[0].decode().split('-')[1]
    anchors = load_anchors(ver)
    print(f'=== {exe.split("/")[-1]} v{ver}: {len(anchors)} anchors')

    vt_index = {}
    for name, frva in sorted(anchors.items()):
        vts = find_vtables_with(pe, frva)
        vt_index[name] = {'func': hex(frva), 'vts': [{'vt': hex(v), 'idx': i, 'n': n} for v, i, n in vts[:8]]}
        tag = ', '.join(f'{hex(v)}[{i}]/{n}' for v, i, n in vts[:4]) or 'non-virtual'
        print(f'{name}: {hex(frva)} -> {tag}')

    # dump key vtables fully: NetConnection (FlushDormancy), PlayerController (GetPlayerViewPoint),
    # ActorChannel, NetDriver (GetMaxTickRate? no — TickFlush), Channel (SetChannelActor)
    KEY_VTS = {}
    for name, info in vt_index.items():
        for row in info['vts']:
            KEY_VTS.setdefault(row['vt'], []).append((name, row['idx']))
    dump = {}
    for vt, members in KEY_VTS.items():
        if len(members) >= 1:
            n = max(r['n'] for r in vt_index[members[0][0]]['vts'] if r['vt'] == vt)
            dump[vt] = {'members': {m: i for m, i in members}, 'n': n}

    # pair deltas for ALL anchor pairs sharing a vtable
    deltas = {}
    for vt, members in KEY_VTS.items():
        if len(members) >= 2:
            for a, (na, ia) in enumerate(members):
                for nb, ib in members[a + 1:]:
                    d = ib - ia
                    deltas.setdefault(f'{na}->{nb}' if d > 0 else f'{nb}->{na}', set()).add(abs(d))
    print('\n=== deltas (consistent?) ===')
    for k, ds in sorted(deltas.items()):
        print(f'{k}: delta {ds} {"CONSISTENT" if len(ds)==1 else "VARIES"}')

    out = {'version': ver, 'vt_index': vt_index, 'deltas': {k: list(v) for k, v in deltas.items()}}
    with open(f'/home/z/re-analysis/out/win-vt-full-{ver}.json', 'w') as f:
        json.dump(out, f, indent=1)
    print('saved out/win-vt-full-%s.json' % ver)

    # NetConnection vtable: find IsNetReady = FlushDormancy-17?
    if 'FindFlushDormancy' in anchors:
        for row in vt_index['FindFlushDormancy']['vts']:
            vt, idx, n = int(row['vt'], 16), row['idx'], row['n']
            if idx >= 17:
                methods = vtable_methods(pe, vt, n)
                cand = methods[idx - 17]
                f = pe.func_of(cand)
                print(f'FlushDormancy vt {hex(vt)}[{idx}]: idx-17 -> {hex(cand)} size {hex(f[1]-f[0]) if f else "?"} (IsNetReady?)')


if __name__ == '__main__':
    for exe in sys.argv[1:]:
        main(exe)
