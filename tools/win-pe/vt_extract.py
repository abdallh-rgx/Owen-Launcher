#!/usr/bin/env python3
"""Extract vtables from Windows exe (no RTTI) via anchor functions.
1. Scan .rdata/.data for qword == anchor VA -> vtable slot candidates
2. Walk slot to full vtable boundaries (arrays of .text VAs)
3. Locate known members inside vtables -> compute indices
4. Cross-check anchor relationships (IsNetReady vs FlushDormancy etc.)"""
import json
import struct
import sys

sys.path.insert(0, '/home/z/re-analysis/tools')
from xref2 import PE


def find_vtables_with(pe, func_rva):
    """Find all vtable slots containing func_rva. Returns [(vt_rva, index, n_methods)]."""
    va = pe.image_base + func_rva
    d = pe.d
    needle = struct.pack('<Q', va)
    out = []
    t = pe.sections['.text']
    start = 0
    while True:
        i = d.find(needle, start)
        if i < 0:
            break
        start = i + 1
        rva = pe.off_to_rva(i)
        if rva is None:
            continue
        sec = None
        for name, s in pe.sections.items():
            if s['va'] <= rva < s['va'] + max(s['vsize'], s['rsize']) and name in ('.rdata', '.data', '.data.rel.ro'):
                sec = name
                break
        if not sec:
            continue
        # walk back to vtable start: previous qwords must be .text VAs too (allow 0)
        j = i
        back = 0
        while back < 4096 * 8:
            k = j - 8
            if k < 0:
                break
            v, = struct.unpack_from('<Q', d, k)
            frva = v - pe.image_base if v else 0
            if v and not (t['va'] <= frva < t['va'] + t['vsize']):
                break
            j = k
            back += 8
        # walk forward
        e = i
        fwd = 0
        while fwd < 8192 * 8:
            k = e + 8
            if k + 8 > len(d):
                break
            v, = struct.unpack_from('<Q', d, k)
            frva = v - pe.image_base if v else 0
            if v and not (t['va'] <= frva < t['va'] + t['vsize']):
                break
            e = k
            fwd += 8
        n = (e - j) // 8 + 1
        if 3 <= n <= 4096:
            idx = (i - j) // 8
            vt_rva = pe.off_to_rva(j)
            out.append((vt_rva, idx, n))
    return out


def vtable_methods(pe, vt_rva, n):
    d = pe.d
    off = pe.rva_to_off(vt_rva)
    return [struct.unpack_from('<Q', d, off + 8 * k)[0] - pe.image_base for k in range(n)]


if __name__ == '__main__':
    exe = sys.argv[1]
    pe = PE(exe)
    ver_off = pe.d.find(b'Fortnite+Release-')
    ver = pe.d[ver_off:ver_off + 40].split(b'\x00')[0].decode().split('-')[1]

    # anchor set: from x64 patterns + string/descriptor resolution
    anchors = {}
    try:
        p = json.load(open(f'/home/z/re-analysis/out/win-x64pat-{ver}.json'))
        if p['version'] == ver:
            for k, v in p['found'].items():
                anchors[k] = int(v['rva'], 16)
    except Exception:
        pass
    try:
        lg = json.load(open('/home/z/re-analysis/out/hard-logv2-27.11.json'))
        MANUAL = {
            'FindFlushDormancy': '0x73dac7c',
            'FindStartBecomingDormant': '0x71fa978',
            'FindEnterAircraft': '0x958f200',
            'FindHandleMatchHasStarted': '0x90f5ee0',
            'FindSelectAndSetupMyBuildingLevel': '0x944cc64',
            'FindQueueStatEvent': '0xa1652d4',
            'FindSetGamePhase': '0xa4bd354',
            'FindFinishedTargetSpline': '0x5cc414e',
            'FindPickSupplyDropLocation': '0x9073524',
            'FindPayBuildableClassPlacementCost': '0xa03c4f0',
        }
        for k, v in MANUAL.items():
            anchors.setdefault(k, int(v, 16))
    except Exception as e:
        print('logv2 skip', e)

    print(f'=== {exe.split("/")[-1]} v{ver}: {len(anchors)} anchors')
    # find vtables per anchor
    vt_index = {}
    for name, frva in sorted(anchors.items()):
        vts = find_vtables_with(pe, frva)
        rows = []
        for vt_rva, idx, n in vts[:6]:
            rows.append({'vt': hex(vt_rva), 'idx': idx, 'n': n})
        vt_index[name] = {'func': hex(frva), 'vts': rows}
        if vts:
            print(f'{name}: func {hex(frva)} in {len(vts)} vtables: ' + ', '.join(f'vt {hex(v)}[{i}]/{n}' for v, i, n in vts[:4]))
        else:
            print(f'{name}: func {hex(frva)} NOT in any vtable (non-virtual)')

    # cross relationships: for pairs of anchors in the SAME vtable, compute delta
    print('\n=== vtable co-membership deltas ===')
    vts_of = {}
    for name, info in vt_index.items():
        for row in info['vts']:
            vts_of.setdefault(row['vt'], []).append((name, row['idx']))
    for vt, members in sorted(vts_of.items()):
        if len(members) >= 2:
            members.sort(key=lambda x: x[1])
            print(f'vtable {vt}: ' + ' | '.join(f'{n}[{i}]' for n, i in members))

    with open(f'/home/z/re-analysis/out/win-vtindex-{ver}.json', 'w') as f:
        json.dump(vt_index, f, indent=1)
    print('saved out/win-vtindex-%s.json' % ver)
