#!/usr/bin/env python3
"""Memory-safe PE xref engine (mmap + chunked numpy).
One pass computes disp32 candidates; per-target query finds:
- RIP-relative refs (lea/mov rip+disp32): target = rva(p+4) + disp32(p)
- relative calls/jmps (E8/E9 rel32):     target = rva(p+5) + rel32 at p+1
for byte-granular positions p across all of .text."""
import bisect
import mmap
import struct
import sys

import numpy as np

sys.path.insert(0, '/home/z/re-analysis/tools')


class PE:
    """Minimal memory-safe PE reader (mmap-backed)."""

    def __init__(self, path):
        self.f = open(path, 'rb')
        self.d = mmap.mmap(self.f.fileno(), 0, access=mmap.ACCESS_READ)
        pe_off = struct.unpack_from('<I', self.d, 0x3C)[0]
        assert self.d[pe_off:pe_off + 4] == b'PE\x00\x00'
        coff = pe_off + 4
        nsec, = struct.unpack_from('<H', self.d, coff + 2)
        opt_size, = struct.unpack_from('<H', self.d, coff + 16)
        opt = coff + 20
        magic, = struct.unpack_from('<H', self.d, opt)
        assert magic == 0x20b, 'not PE32+'
        self.image_base, = struct.unpack_from('<Q', self.d, opt + 24)
        sec_off = opt + opt_size
        self.sections = {}
        for i in range(nsec):
            o = sec_off + 40 * i
            name = self.d[o:o + 8].rstrip(b'\x00').decode(errors='replace')
            vsize, va, rsize, raw = struct.unpack_from('<IIII', self.d, o + 8)
            self.sections[name] = {'va': va, 'vsize': vsize, 'rsize': rsize, 'raw': raw}
        self._fbounds = None

    def rva_to_off(self, rva):
        for s in self.sections.values():
            if s['va'] <= rva < s['va'] + max(s['vsize'], s['rsize']):
                off = rva - s['va'] + s['raw']
                if off < len(self.d):
                    return off
        return None

    def off_to_rva(self, off):
        for s in self.sections.values():
            if s['raw'] <= off < s['raw'] + s['rsize']:
                return off - s['raw'] + s['va']
        return None

    def func_bounds(self):
        """RUNTIME_FUNCTION array from .pdata -> sorted [(start,end)]."""
        if self._fbounds is None:
            p = self.sections.get('.pdata')
            out = []
            if p:
                off, end = p['raw'], p['raw'] + p['rsize']
                while off + 12 <= end:
                    s, e, u = struct.unpack_from('<III', self.d, off)
                    if s == 0 and e == 0:
                        break
                    out.append((s, e))
                    off += 12
            out.sort()
            self._fbounds = out
        return self._fbounds

    def func_of(self, rva):
        fb = self.func_bounds()
        i = bisect.bisect_right(fb, (rva, 1 << 62)) - 1
        if i >= 0 and fb[i][0] <= rva < fb[i][1]:
            return fb[i]
        return None

    def find_bytes(self, needle: bytes):
        out, start = [], 0
        while True:
            i = self.d.find(needle, start)
            if i < 0:
                break
            out.append(i)
            start = i + 1
        return out

    def find_ascii(self, s):
        return self.find_bytes(s.encode())

    def find_utf16(self, s):
        return self.find_bytes(s.encode('utf-16-le'))


class XrefEngine:
    """Chunked byte-granular disp32/rel32 scanner over .text."""

    def __init__(self, pe: PE, chunk=6 * 1024 * 1024):
        self.pe = pe
        t = pe.sections['.text']
        self.tva, self.traw = t['va'], t['raw']
        self.tlen = t['rsize']
        self.CH = chunk

    def scan_targets(self, targets, calls=True, riprel=True):
        """targets: dict rva -> key. Returns {key: [ref_rvas]} where ref is the
        START rva of the referencing instruction (for riprel: p-3..p heuristic
        unknown; we return the disp position's instruction start estimate).
        For calls (E8/E9): ref = p (opcode position)."""
        res = {k: [] for k in targets.values()}
        tva, traw, tlen, CH = self.tva, self.traw, self.tlen, self.CH
        d = self.pe.d
        targets_arr = np.array(sorted(targets.keys()), dtype=np.int64)
        keys = [targets[k] for k in sorted(targets.keys())]
        c0 = 0
        while c0 < tlen - 8:
            c1 = min(c0 + CH, tlen - 4)
            seg = np.frombuffer(d, dtype=np.uint8, count=c1 - c0, offset=traw + c0).astype(np.int64, copy=False)
            b0, b1, b2, b3 = seg[:-4], seg[1:-3], seg[2:-2], seg[3:-1]
            dsp = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            dsp = np.where(dsp >= 2**31, dsp - 2**32, dsp)
            base_next = np.arange(c0, c1 - 4, dtype=np.int64) + tva + 4  # rva after disp32
            # riprel: instr_end + disp == target
            if riprel:
                tgt = (base_next + dsp)  # resolved riprel target per position
                for i, k in enumerate(keys):
                    hit = np.nonzero(tgt == targets_arr[i])[0]
                    for h in hit:
                        res[k].append((int(h) + c0) + tva)
            # call/jmp rel32: opcode at p-? Actually E8 rel32: opcode at q, rel at q+1,
            # next instr at q+5. Position p (disp start) => opcode at p-1, next_rva = p+5+tva.
            if calls:
                opc = np.where(b1 >= 0, seg[0:len(seg) - 4], 0)
                # for position p as START of rel32: opcode byte at p-1 — need shifted seg
                # easier: compute for positions q where d[q] in (E8,E9): rel at q+1
                # vectorized: rel target for position q+1 = rva(q+5)+rel(q+1)
                base_call = base_next + 1  # rva(q+5) where p=q+1
                tgt2 = base_call + dsp
                for i, k in enumerate(keys):
                    hit = np.nonzero(tgt2 == targets_arr[i])[0]
                    for h in hit:
                        p = int(h) + c0  # rel32 position
                        q = p - 1  # opcode position
                        ob = d[traw + q] if q >= 0 else 0
                        if ob in (0xE8, 0xE9):
                            res[k + '::call'].append(q + tva) if (k + '::call') in res else res.setdefault(k + '::call', []).append(q + tva)
            del dsp, base_next, seg
            c0 = c1
        return res

    def string_refs(self, needle, wide=True):
        offs = self.pe.find_utf16(needle) if wide else self.pe.find_ascii(needle)
        targets = {}
        for off in offs:
            rva = self.pe.off_to_rva(off)
            if rva is not None:
                targets[rva] = f'str:{needle[:30]}'
        if not targets:
            return {}
        return self.scan_targets(targets, calls=False, riprel=True)


if __name__ == '__main__':
    exe = sys.argv[1]
    pe = PE(exe)
    print('image_base', hex(pe.image_base), 'text', hex(pe.sections['.text']['va']), 'size', hex(pe.sections['.text']['rsize']))
    fb = pe.func_bounds()
    print('functions:', len(fb))
    xe = XrefEngine(pe)
    probes = {
        'FlushDormancy': ('FlushDormancy: %s. Connection: %s', True),
        'StartBecomingDormant': ('StartBecomingDormant: %s', True),
        'SendClientAdjustment': ('SendClientAdjustment', True),
        'GetWorldContext': ('GetWorldContext', True),
        'QueueStatEvent': ('QueueStatEvent', True),
        'SetGamePhase': ('SetGamePhase', False),
        'InitializeFlightPath': ('InitializeFlightPath', False),
        'FinishedTargetSpline': ('FinishedTargetSpline', False),
        'AllowCommandletRendering': ('AllowCommandletRendering', True),
    }
    for name, (s, wide) in probes.items():
        r = xe.string_refs(s, wide)
        flat = []
        for k, v in r.items():
            flat.extend(v)
        print(f'--- {name}: {len(flat)} refs')
        seen = set()
        for ref in sorted(set(flat)):
            f = pe.func_of(ref)
            fr = f'{hex(f[0])}+{hex(f[1]-f[0])}' if f else 'nofunc'
            if fr not in seen:
                seen.add(fr)
                print(f'   ref@{hex(ref)} in {fr}')
