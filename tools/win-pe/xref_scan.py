#!/usr/bin/env python3
"""Numpy-accelerated xref scanner + function identification for Windows PE.
Finds RIP-relative (and immediate MOV) refs from .text to any target RVA."""
import struct
import sys

import numpy as np

sys.path.insert(0, '/home/z/re-analysis/tools')
from pe_analysis import PEAnalyzer


class XrefScanner:
    def __init__(self, a: PEAnalyzer):
        self.a = a
        t = a.sections['.text']
        self.tva, self.tsize, self.traw = t['va'], t['vsize'], t['raw']
        d = a.data[self.traw:self.traw + t['rsize']]
        self.text = d
        n = (len(d) // 4) * 4
        self.disp = np.frombuffer(d[:n], dtype='<i4')  # disp32 candidates at every 4-byte slot
        # NOTE: this misses unaligned disp32. For lea/mov rip-rel, instructions are
        # usually 4-byte aligned in hot code but NOT guaranteed. We also do a byte scan
        # pass for completeness on specific targets.
        self.byff = np.frombuffer(d[:n], dtype='<u1')

    def refs_to(self, target_rva, aligned_only=True):
        """RIP-relative refs: instr_end_rva + disp == target_rva. Chunked for RAM."""
        out = []
        n = len(self.disp)
        pos_rva = np.arange(n, dtype=np.int64) * 4 + self.tva + 4
        cond = pos_rva + self.disp.astype(np.int64) == target_rva
        for i in np.nonzero(cond)[0]:
            out.append((int(i) * 4, 'a'))
        del pos_rva, cond
        if not aligned_only:
            CH = 8 * 1024 * 1024
            tb = self.text
            tl = len(tb) - 4
            c0 = 0
            while c0 < tl:
                c1 = min(c0 + CH, tl)
                seg = np.frombuffer(tb[c0:c1], dtype=np.uint8).copy()
                b0 = seg[:-4].astype(np.int64)
                b1 = seg[1:-3].astype(np.int64)
                b2 = seg[2:-2].astype(np.int64)
                b3 = seg[3:-1].astype(np.int64)
                disp = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
                disp = np.where(disp >= 2**31, disp - 2**32, disp)
                base_rva = np.arange(c0, c1 - 4, dtype=np.int64) + self.tva + 4
                cond2 = base_rva + disp == target_rva
                for i in np.nonzero(cond2)[0]:
                    p = int(i) + c0
                    if p % 4 != 0:
                        out.append((p, 'b'))
                del disp, base_rva, cond2, b0, b1, b2, b3
                c0 = c1
        return out

    def string_refs(self, needle, wide=True):
        a = self.a
        offs = a.find_utf16(needle) if wide else a.find_ascii(needle.encode())
        res2 = []
        for off in offs:
            rva = a.off_to_rva(off)
            if rva is None:
                continue
            for (poff, kind) in self.refs_to(rva, aligned_only=False):
                res2.append({'str_rva': hex(rva), 'ref_rva': hex(self.tva + poff)})
        return res2


def func_bounds(a: PEAnalyzer):
    pdata = a.sections.get('.pdata')
    out = []
    if pdata:
        off, end = pdata['raw'], pdata['raw'] + pdata['rsize']
        d = a.data
        while off + 12 <= end:
            s, e, u = struct.unpack_from('<III', d, off)
            if s == 0 and e == 0:
                break
            out.append((s, e))
            off += 12
    return out


if __name__ == '__main__':
    exe = sys.argv[1]
    a = PEAnalyzer(exe)
    print('version', a.detect_version())
    xs = XrefScanner(a)
    fb = func_bounds(a)
    import bisect
    starts = [s for s, e in fb]

    def func_of(rva):
        i = bisect.bisect_right(starts, rva) - 1
        if i >= 0:
            s, e = fb[i]
            if s <= rva < e:
                return s, e - s
        return None

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
        refs = xs.string_refs(s, wide)
        print(f'--- {name} ({len(refs)} refs)')
        for r in refs[:8]:
            f = func_of(int(r['ref_rva'], 16))
            fr = f'func {hex(f[0])}+{hex(f[1])}' if f else 'no-func'
            print(f'   ref@{r["ref_rva"]} str@{r["str_rva"]} -> {fr}')
