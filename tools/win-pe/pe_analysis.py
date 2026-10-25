#!/usr/bin/env python3
"""PE analyzer for FortniteClient-Win64-Shipping.exe — universal ground-truth extractor.

Extracts:
- Section layout (.text, .rdata, .data)
- FNativeFunctionRegistrar native tables: {Name -> thunk RVA}  (UE4/UE5 static registration)
- RTTI vtables: class name -> {vtable RVA, [method RVAs]}
- String table: ASCII + UTF-16LE strings with xrefs
- Version detection from "Fortnite+Release-XX.XX-CL-YYYYYYY"
"""
import json
import re
import struct
import sys
from collections import defaultdict

import pefile


class PEAnalyzer:
    def __init__(self, path):
        self.path = path
        self.pe = pefile.PE(path, fast_load=True)  # headers only — no dirs (RAM!)
        self.image_base = self.pe.OPTIONAL_HEADER.ImageBase
        self.sections = {}
        for s in self.pe.sections:
            name = s.Name.rstrip(b'\x00').decode(errors='replace')
            self.sections[name] = {
                'va': s.VirtualAddress,
                'vsize': s.Misc_VirtualSize,
                'raw': s.PointerToRawData,
                'rsize': s.SizeOfRawData,
                'chars': s.Characteristics,
            }
        self._data = None
        self.version = None
        self.cl = None

    @property
    def data(self):
        if self._data is None:
            with open(self.path, 'rb') as f:
                self._data = f.read()
        return self._data

    def rva_to_off(self, rva):
        for s in self.sections.values():
            if s['va'] <= rva < s['va'] + max(s['vsize'], s['rsize']):
                off = rva - s['va'] + s['raw']
                if off < len(self.data):
                    return off
        return None

    def va_to_off(self, va):
        return self.rva_to_off(va - self.image_base)

    def off_to_rva(self, off):
        for s in self.sections.values():
            if s['raw'] <= off < s['raw'] + s['rsize']:
                return off - s['raw'] + s['va']
        return None

    def read(self, rva, n):
        off = self.rva_to_off(rva)
        if off is None:
            return None
        return self.data[off:off + n]

    def read_u32(self, rva):
        b = self.read(rva, 4)
        return struct.unpack('<I', b)[0] if b and len(b) == 4 else None

    def read_u64(self, rva):
        b = self.read(rva, 8)
        return struct.unpack('<Q', b)[0] if b and len(b) == 8 else None

    # ---------------- version ----------------
    def detect_version(self):
        m = re.search(rb'Fortnite\+Release-(\d+\.\d+)-CL-(\d+)', self.data)
        if m:
            self.version = m.group(1).decode()
            self.cl = m.group(2).decode()
        return self.version

    # ---------------- strings ----------------
    def find_ascii(self, needle: bytes):
        """All file offsets of an ASCII string."""
        out, start = [], 0
        while True:
            i = self.data.find(needle, start)
            if i < 0:
                break
            out.append(i)
            start = i + 1
        return out

    def find_utf16(self, needle: str):
        """All file offsets of a UTF-16LE string."""
        b = needle.encode('utf-16-le')
        out, start = [], 0
        while True:
            i = self.data.find(b, start)
            if i < 0:
                break
            out.append(i)
            start = i + 2
        return out

    # ---------------- natives via FNativeFunctionRegistrar ----------------
    def extract_natives(self):
        """UE static linking registers natives via TPair<FName, FNativeFuncPtr> arrays.
        Pattern: sequence of (FName(8), padding, ptr) where ptr points into .text and
        FName is a valid FName (ComparisonIndex low bits). We instead scan .rdata for
        pointer-pairs: [name_ptr_ascii?]. UE4.23+: FName is 8 bytes (ComparisonIndex,
        Number). The registrar arrays hold {FName, NativeFuncPtr} — detect runs of
        (qword FName, qword func ptr) where func ptrs are all in .text and ascending-ish.
        """
        text = self.sections.get('.text')
        if not text:
            return {}
        natives = {}
        # Heuristic: scan .rdata + .data for 8-byte aligned pairs (fname, fnptr) where
        # consecutive entries differ by 16 bytes and fnptr in .text.
        for secname in ('.rdata', '.data'):
            sec = self.sections.get(secname)
            if not sec:
                continue
            base_off = sec['raw']
            end = sec['raw'] + sec['rsize']
            d = self.data
            run_start = None
            prev_fn = None
            i = base_off
            while i + 16 <= end:
                fn = struct.unpack_from('<Q', d, i + 8)[0]
                rva = fn - self.image_base
                in_text = text['va'] <= rva < text['va'] + text['vsize'] if fn else False
                if in_text:
                    if run_start is None:
                        run_start = i
                    prev_fn = fn
                else:
                    if run_start is not None and i - run_start >= 16 * 8:
                        self._scan_native_run(run_start, i, natives)
                    run_start = None
                i += 16
            if run_start is not None and end - run_start >= 16 * 8:
                self._scan_native_run(run_start, end, natives)
        return natives

    def _scan_native_run(self, start_off, end_off, natives):
        """A run of (FName, funcptr) pairs. Resolve FName -> string via NamePool is not
        possible statically without engine internals; but UE also has
        FNativeFunctionRegistrar::RegisterFunction(FName, const TCHAR*, fnptr) called with
        inline string args. Instead: rely on adjacent (wstr, fnptr) registrar layout used
        by Fortnite: pairs of (const TCHAR* name, fnptr). Try both layouts."""
        d = self.data
        # Layout B: (const TCHAR* namePtr, fnPtr)
        for off in range(start_off, end_off, 16):
            namep = struct.unpack_from('<Q', d, off)[0]
            fn = struct.unpack_from('<Q', d, off + 8)[0]
            if not namep or not fn:
                continue
            noff = self.va_to_off(namep)
            if noff is None:
                continue
            # try UTF-16 then ASCII
            s = None
            raw = d[noff:noff + 256]
            z = raw.find(b'\x00\x00')
            if z > 0 and z % 2 == 0:
                try:
                    cand = raw[:z].decode('utf-16-le')
                    if cand and all(31 < ord(c) < 127 for c in cand):
                        s = cand
                except Exception:
                    pass
            if not s:
                z = raw.find(b'\x00')
                if 0 < z < 128:
                    try:
                        cand = raw[:z].decode('ascii')
                        if cand and all(31 < ord(c) < 127 for c in cand):
                            s = cand
                    except Exception:
                        pass
            if s and len(s) > 2 and re.match(r'^[A-Za-z0-9_]+$', s):
                natives[s] = fn - self.image_base

    # ---------------- vtables via RTTI ----------------
    def extract_vtables(self):
        """MSVC RTTI: vtable[-1] points to RTTI Complete Object Locator; locator+12
        (TypeDescriptor offset) -> TypeDescriptor; TypeDescriptor+16 = class name."""
        vtables = {}
        for secname in ('.rdata', '.data'):
            sec = self.sections.get(secname)
            if not sec:
                continue
            d = self.data
            for off in range(sec['raw'], sec['raw'] + sec['rsize'] - 8, 8):
                p = struct.unpack_from('<Q', d, off)[0]
                if not p:
                    continue
                rva = p - self.image_base
                loff = self.rva_to_off(rva)
                if loff is None:
                    continue
                # RTTI Complete Object Locator signature (32-bit RVAs): 00 00 00 00/01
                sig = struct.unpack_from('<I', d, loff)[0]
                if sig != 0 and sig != 1:
                    continue
                try:
                    td_off32, cd_off32, cb_off32 = struct.unpack_from('<III', d, loff + 4)
                except Exception:
                    continue
                # TypeDescriptor at rva td_off32 (relative to imagebase, 32-bit)
                tdo = self.rva_to_off(td_off32)
                if tdo is None:
                    continue
                # TypeDescriptor: vftable(8) spare(8) name(ascii z)
                name_off = tdo + 16
                z = d.find(b'\x00', name_off, name_off + 160)
                if z < 0:
                    continue
                try:
                    name = d[name_off:z].decode('ascii')
                except Exception:
                    continue
                if not re.match(r'^\.\?AV[A-Za-z0-9_@]+@$', name):
                    continue
                # vtable methods: qwords after the locator ptr until null/non-code
                methods = []
                moff = off + 8
                text = self.sections['.text']
                while moff + 8 <= len(d):
                    mp = struct.unpack_from('<Q', d, moff)[0]
                    if not mp:
                        break
                    mrva = mp - self.image_base
                    if not (text['va'] <= mrva < text['va'] + text['vsize']):
                        break
                    methods.append(mrva)
                    moff += 8
                if len(methods) >= 3:
                    cls = name[4:-2]  # strip .?AV and @
                    vtables[cls] = {'rva': self.off_to_rva(off), 'methods': methods}
        return vtables

    # ---------------- xrefs (data -> code) ----------------
    def find_code_refs_to_rva(self, target_rva, kind='rip'):
        """Find RIP-relative references (lea/mov) in .text to target_rva."""
        text = self.sections['.text']
        d = self.data
        refs = []
        target_va = self.image_base + target_rva
        # RIP rel32: scan .text for 4-byte disp where (instr_end + disp) == target
        # We do a fast scan: for each position p in text, consider it as disp32;
        # addr_of_next_instr unknown, but for lea the disp is the last 4 bytes.
        # Instead scan for absolute encodings is unreliable; use capstone disasm
        # around candidate matches: simpler — scan every offset, check
        # (rva_of(p+4) + disp) == target. This catches all `xx xx xx xx disp32` where
        # instruction ends at p+4 (true for lea r?, [rip+disp] with modrm 05).
        for p in range(text['raw'], text['raw'] + text['rsize'] - 4):
            disp = struct.unpack_from('<i', d, p)[0]
            p_rva = p - text['raw'] + text['va']
            if p_rva + 4 + disp == target_rva:
                refs.append(p_rva)
        return refs

    def find_string_xrefs(self, needle: str, wide=True):
        """Find code refs (RIP-relative) to a string. Returns list of ref RVAs."""
        out = []
        offs = self.find_utf16(needle) if wide else self.find_ascii(needle.encode())
        for off in offs:
            rva = self.off_to_rva(off)
            if rva is None:
                continue
            out.extend(self.find_code_refs_to_rva(rva))
        return out


if __name__ == '__main__':
    path = sys.argv[1]
    a = PEAnalyzer(path)
    print('version:', a.detect_version(), 'CL', a.cl)
    print('sections:', {k: hex(v['va']) for k, v in a.sections.items()})
    # Quick natives test
    nat = a.extract_natives()
    print('natives found:', len(nat))
    for probe in ('ServerKickPlayer', 'K2_GiveAbility', 'ClearAbility', 'OnEndGameKickPlayers'):
        print(' ', probe, '->', hex(nat[probe]) if probe in nat else 'MISS')
