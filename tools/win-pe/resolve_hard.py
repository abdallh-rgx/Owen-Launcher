#!/usr/bin/env python3
"""Resolve hard Erbium functions on a Windows exe via full-string xrefs.
Finds the containing string (scan to \x00\x00 boundary), scans riprel refs,
maps refs to functions via .pdata, and prints structural fingerprints."""
import json
import struct
import sys

import capstone

sys.path.insert(0, '/home/z/re-analysis/tools')
from xref2 import PE, XrefEngine


def full_string_range(pe, off, wide=True):
    """Return (start_off, end_off) of the null-terminated string containing off."""
    d = pe.d
    if wide:
        # scan back to \x00\x00 aligned even boundary
        s = off
        while s >= 2 and d[s - 2:s] != b'\x00\x00':
            s -= 2
        e = off
        while e + 2 <= len(d) and d[e:e + 2] != b'\x00\x00':
            e += 2
        return s, e + 2
    else:
        s = off
        while s >= 1 and d[s - 1] != 0:
            s -= 1
        e = off
        while e < len(d) and d[e] != 0:
            e += 1
        return s, e + 1


def decode_at(pe, off, wide=True, maxlen=400):
    d = pe.d
    if wide:
        raw = d[off:off + maxlen * 2]
        z = raw.find(b'\x00\x00')
        if z >= 0 and z % 2 == 0:
            raw = raw[:z]
        return raw.decode('utf-16-le', errors='replace')
    else:
        raw = d[off:off + maxlen]
        z = raw.find(b'\x00')
        if z >= 0:
            raw = raw[:z]
        return raw.decode('ascii', errors='replace')


def resolve(pe, xe, needle, wide=True, max_str=400):
    """Find full strings containing needle, scan refs, return candidates."""
    out = []
    offs = pe.find_utf16(needle) if wide else pe.find_ascii(needle)
    seen_starts = set()
    for off in offs:
        s, e = full_string_range(pe, off, wide)
        if s in seen_starts:
            continue
        seen_starts.add(s)
        full = decode_at(pe, s, wide)
        if len(full) > max_str or '\x00' in full:
            continue
        rva = pe.off_to_rva(s)
        if rva is None:
            continue
        targets = {rva: f'r{rva}'}
        res = xe.scan_targets(targets, calls=False, riprel=True)
        refs = [v for vs in res.values() for v in vs]
        out.append({'string': full, 'rva': hex(rva), 'refs': refs})
    return out


def fingerprint(pe, start_rva, size, max_ins=250):
    """Disassemble and extract a structural fingerprint."""
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    md.detail = True
    off = pe.rva_to_off(start_rva)
    code = pe.d[off:off + size]
    fp = {'calls': [], 'imm64': [], 'disp': [], 'strings': [], 'n_ins': 0}
    for i in md.disasm(code, pe.image_base + start_rva):
        fp['n_ins'] += 1
        if fp['n_ins'] > max_ins:
            break
        if i.mnemonic == 'call' and i.op_str.startswith('0x'):
            fp['calls'].append(hex(int(i.op_str, 16) - pe.image_base))
        for op in i.operands:
            if op.type == capstone.x86.X86_OP_IMM and abs(op.imm) > 0xFFFF and op.imm < 2**64:
                fp['imm64'].append(hex(op.imm & 0xFFFFFFFFFFFFFFFF))
            if op.type == capstone.x86.X86_OP_MEM and op.mem.base in (capstone.x86.X86_REG_RIP,):
                # lea/mov rip+disp: resolve target
                tgt = i.address + i.size + op.mem.disp - pe.image_base
                fp['disp'].append({'at': hex(i.address - pe.image_base), 'target': hex(tgt)})
    return fp


if __name__ == '__main__':
    exe = sys.argv[1] if len(sys.argv) > 1 else 'binaries/win/FortniteClient-27.11.exe'
    pe = PE(exe)
    ver_off = pe.d.find(b'Fortnite+Release-')
    print('exe:', exe.split('/')[-1], pe.d[ver_off:ver_off + 40].split(b'\x00')[0].decode())
    xe = XrefEngine(pe)

    HARD = [
        # (name, needle, wide, must_contain_hint_or_None)
        ('FindFlushDormancy', 'FlushDormancy: %s', True, None),
        ('FindStartBecomingDormant', 'StartBecomingDormant: %s', True, None),
        ('FindUpdateSafeZonesPhase', 'UpdateSafeZonesPhase', True, None),
        ('FindPreSendUpdate', 'PreSendUpdate', True, None),
        ('FindSendClientAdjustment', 'SendClientAdjustment', True, None),
        ('FindGetWorldContext', 'GetWorldContext', True, None),
        ('FindQueueStatEvent', 'QueueStatEvent', True, None),
        ('FindSetGamePhase', 'SetGamePhase', False, None),
        ('FindInitializeFlightPath', 'InitializeFlightPath', False, None),
        ('FindFinishedTargetSpline', 'FinishedTargetSpline', False, None),
        ('FindClientHasInitializedLevelFor', 'ClientHasInitializedLevelFor', True, None),
        ('FindIsNetReady', 'IsNetReady', True, None),
        ('GIs-anchor', 'AllowCommandletRendering', True, None),
        ('FindEnterAircraft', 'EnterAircraft: [%s]', True, None),
        ('FindHandleMatchHasStarted', 'HandleMatchHasStarted', True, None),
        ('FindSpawnInitialSafeZone', 'SpawnInitialSafeZone', True, None),
        ('FindSelectAndSetupMyBuildingLevel', 'SelectAndSetupMyBuildingLevel', True, None),
        ('FindPickSupplyDropLocation', 'PickSupplyDropLocation', False, None),
        ('FindPayBuildableClassPlacementCost', 'Failed to remove item %s during pay building costs', True, None),
        ('FindCantBuild', 'Cannot place building', True, None),
        ('FindOnRep_ZiplineState', 'ZiplineState', True, None),
    ]
    report = {}
    for name, needle, wide, hint in HARD:
        cands = resolve(pe, xe, needle, wide)
        print(f'=== {name}')
        rows = []
        for c in cands:
            funcs = {}
            for ref in c['refs']:
                f = pe.func_of(ref)
                if f:
                    funcs.setdefault(f[0], [0, f[1] - f[0]])[0] += 1
            s = c['string']
            print(f"  str {s[:90]!r} refs={len(c['refs'])}")
            for fs, (cnt, sz) in sorted(funcs.items(), key=lambda x: -x[1][0]):
                print(f'    -> func {hex(fs)} size {hex(sz)} x{cnt}')
                rows.append({'string': s, 'func': hex(fs), 'size': sz, 'xrefs': cnt, 'ref_rvas': [hex(r) for r in c['refs'][:6]]})
        report[name] = rows
    with open('/home/z/re-analysis/out/hard-resolve-{pe.d[ver_off+18:ver_off+23].decode()}.json', 'w') as f:
        json.dump(report, f, indent=1)
    print('saved out/hard-resolve-27.11.json')
