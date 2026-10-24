#!/usr/bin/env python3
"""X64 tool — cross-arch feature extraction from FortniteClient-Win64-Shipping.exe (21.30)
- .pdata function table (exact boundaries)
- string literals (.rdata, ascii+utf16) + rip-relative xrefs (vectorized)
- Erbium x64 pattern scan (fast numpy)
- function features: strings / immediates / call targets / size
"""
import struct
import numpy as np
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

EXE = "/tmp/work2/src/FortniteClient-Win64-Shipping.exe"

class X64:
    _inst = None

    def __init__(self, path=EXE):
        with open(path, "rb") as f:
            self.full = f.read()
        d = self.full
        e_lfanew = struct.unpack_from("<I", d, 0x3C)[0]
        nsec = struct.unpack_from("<H", d, e_lfanew + 6)[0]
        opt_size = struct.unpack_from("<H", d, e_lfanew + 20)[0]
        self.base = struct.unpack_from("<Q", d, e_lfanew + 24 + 24)[0]
        sec_off = e_lfanew + 24 + opt_size
        self.sections = {}
        for i in range(nsec):
            off = sec_off + i * 40
            name = d[off:off+8].rstrip(b"\0").decode(errors="replace")
            vsize, vaddr, rsize, roff = struct.unpack_from("<IIII", d, off + 8)
            self.sections[name] = (vaddr, vsize, roff, rsize)
        tva, tvsz, toff, trsz = self.sections[".text"]
        self.text_va = self.base + tva
        self.text = np.frombuffer(d[toff:toff+trsz], dtype=np.uint8)
        self.rva_text = tva
        # pdata → function table
        pva, pvsz, poff, prsz = self.sections[".pdata"]
        n = pvsz // 12
        entries = np.frombuffer(d[poff:poff+n*12], dtype=np.uint32).reshape(-1, 3)
        funcs = entries[:, 0].copy()
        funcs.sort()
        self.pdata_funcs = funcs  # RVAs sorted
        self.pdata_ends = dict()  # begin→end quick? build array pairs
        pairs = np.zeros((n, 2), dtype=np.uint64)
        pairs[:, 0] = entries[:, 0]
        pairs[:, 1] = entries[:, 1]
        pairs = pairs[np.argsort(pairs[:, 0])]
        self.pdata_pairs = pairs
        self.cs = Cs(CS_ARCH_X86, CS_MODE_64)
        self.cs.detail = False

    @classmethod
    def get(cls):
        if cls._inst is None:
            cls._inst = cls()
        return cls._inst

    # ---------- addressing ----------
    def va2off(self, va):
        rva = va - self.base
        for name, (vaddr, vsize, roff, rsize) in self.sections.items():
            if vaddr <= rva < vaddr + vsize:
                o = roff + (rva - vaddr)
                if o < len(self.full):
                    return o
        return None

    def off2va(self, off):
        for name, (vaddr, vsize, roff, rsize) in self.sections.items():
            if roff <= off < roff + rsize:
                return self.base + vaddr + (off - roff)
        return None

    # ---------- functions ----------
    def func_of(self, va):
        rva = va - self.base
        i = np.searchsorted(self.pdata_funcs, rva, "right") - 1
        if i < 0:
            return None
        b, e = self.pdata_pairs[i]
        if b <= rva < e:
            return self.base + int(b)
        return None

    def func_end(self, fn):
        rva = fn - self.base
        i = np.searchsorted(self.pdata_pairs[:, 0], rva, "right") - 1
        if i >= 0 and self.pdata_pairs[i, 0] == rva:
            return self.base + int(self.pdata_pairs[i, 1])
        return fn + 0x1000

    def func_size(self, fn):
        return self.func_end(fn) - fn

    # ---------- strings ----------
    def find_literals(self, s, limit=10):
        """(lit_start_va, wide, full_literal) for substring s; search .rdata"""
        res = []
        for enc, wide in ((s.encode("utf-16-le"), True), (s.encode(), False)):
            hay = self.full
            i = hay.find(enc)
            while i != -1 and len(res) < limit:
                va = self.off2va(i)
                if va is not None and self._sec_of_off(i) in (".rdata", ".data"):
                    if wide:
                        j = i
                        while j > 1 and not (hay[j-2] == 0 and hay[j-1] == 0):
                            j -= 2
                        k = i
                        while k < len(hay) - 1 and not (hay[k] == 0 and hay[k+1] == 0):
                            k += 2
                        lit = hay[j:k].decode("utf-16-le", errors="replace")
                        res.append((self.off2va(j), True, lit))
                        i = j + 2 if False else i  # keep i at substring
                    else:
                        j = i
                        while j > 0 and hay[j-1] != 0:
                            j -= 1
                        k = hay.find(b"\0", i)
                        lit = hay[j:k].decode("ascii", errors="replace")
                        res.append((self.off2va(j), False, lit))
                i = hay.find(enc, i + 2)
        return res

    def _sec_of_off(self, off):
        for name, (vaddr, vsize, roff, rsize) in self.sections.items():
            if roff <= off < roff + rsize:
                return name
        return None

    # ---------- rip-relative xrefs (vectorized) ----------
    def rip_xrefs(self, targets):
        """for set of target VAs: all (site_va, target) LEA/MOV rip-rel refs in .text
        handles: [REX(48/4C/49/4D...)] 8D/8B modrm(mod=00, rm=101) disp32"""
        tset = {int(t) for t in targets}
        t = self.text
        n = len(t)
        # positions of opcode candidates: 0x8D (LEA) and 0x8B (MOV r64)
        # modrm must satisfy (modrm & 0xC7) == 0x05
        # case A: REX.W at p-1 (0x40-0x4F), opcode at p, modrm at p+1, disp at p+2..p+6
        # case B: no REX: opcode at p, modrm p+1, disp p+2..p+6 (32-bit operand)
        out = []
        for opc in (0x8D, 0x8B):
            pos = np.flatnonzero(t == opc)
            pos = pos[(pos > 2) & (pos < n - 6)]
            modrm = t[pos + 1]
            ok = (modrm & 0xC7) == 0x05
            pos = pos[ok]
            # disp32 → target
            disp = t[pos + 2].astype(np.int64) | (t[pos + 3].astype(np.int64) << 8) | \
                   (t[pos + 4].astype(np.int64) << 16) | (t[pos + 5].astype(np.int64) << 24)
            disp = np.where(disp >= 0x80000000, disp - 0x100000000, disp)
            # instruction end = opcode_pos + 6 ALWAYS (disp ends at pos+5; REX is before opcode)
            has_rex = (t[pos - 1] & 0xF0) == 0x40
            site_rva = self.rva_text + pos  # rva of opcode
            end_rva = self.rva_text + pos + 6
            target = self.base + end_rva + disp
            for i in range(len(pos)):
                tg = int(target[i])
                if tg in tset:
                    out.append((int(self.base + site_rva[i]) - (1 if has_rex[i] else 0), tg))
        return out

    def anchored_funcs_x64(self, s):
        """R1 on x64: funcs containing rip-refs to literals with substring s"""
        lits = self.find_literals(s)
        if not lits:
            return {}
        tgts = []
        for va, wide, lit in lits:
            tgts.append(va)
            # str+k references (like "llowCommandletRendering")
            for k in range(1, min(len(lit), 60)):
                tgts.append(va + k * (2 if wide else 1))
        xrefs = self.rip_xrefs(set(tgts))
        fns = {}
        for site, tg in xrefs:
            f = self.func_of(site)
            if f:
                fns.setdefault(f, []).append(site)
        return fns

    # ---------- pattern scan ----------
    def scan_pattern(self, pat, limit=10):
        """regex-based masked scan over .text (C speed)"""
        import re as _re
        parts = pat.split()
        if not parts:
            return []
        rx_parts = []
        for p in parts:
            if p in ("?", "??", "???"):
                rx_parts.append(b".")
            else:
                rx_parts.append(_re.escape(bytes([int(p, 16)])))
        rx = _re.compile(b"".join(rx_parts), _re.DOTALL)
        m = len(parts)
        # text buffer = mmap-like access into self.full (bytes)
        buf = self.full
        tva, tvsz, toff, trsz = self.sections[".text"]
        lo, hi = toff, toff + trsz - m
        hits = []
        for mt in rx.finditer(buf, lo, hi):
            hits.append(int(self.text_va + (mt.start() - toff)))
            if len(hits) >= limit:
                break
        return hits

    # ---------- features ----------
    def func_features(self, fn, max_ins=4000):
        """strings referenced, immediates, call targets, size"""
        o = self.va2off(fn)
        end = self.func_end(fn)
        size = end - fn
        code = self.full[o:o + size]
        strings = set()
        imms = set()
        calls = []
        for ins in self.cs.disasm(code, fn):
            mn = ins.mnemonic
            if mn in ("lea", "mov") and "rip" in ins.op_str:
                # compute target
                try:
                    disp = int(ins.op_str.split("rip")[1].split("]")[0].replace(" ", "").replace("+", ""), 16) \
                        if ("rip +" in ins.op_str or "rip -" in ins.op_str or "rip+" in ins.op_str) else 0
                except Exception:
                    disp = 0
                # robust: use capstone arithmetic
                tgt = ins.address + ins.size + _rip_disp(ins)
                s = self._read_str(tgt)
                if s:
                    strings.add(s)
            if mn in ("mov", "cmp", "test", "and", "or", "xor", "sub", "add") and "#" not in ins.op_str:
                for tok in ins.op_str.replace(",", " ").split():
                    if tok.startswith("0x") or tok.isdigit():
                        try:
                            v = int(tok, 0)
                            if 0xFF < v < 0x100000000:
                                imms.add(v)
                        except ValueError:
                            pass
            if mn == "call":
                try:
                    tgt = int(ins.op_str.split()[-1], 16)
                    calls.append(tgt)
                except Exception:
                    pass
            if len(strings) > 40:
                break
        return {"size": size, "strings": strings, "imms": imms, "calls": calls}

    def _read_str(self, va, minlen=6):
        o = self.va2off(va)
        if o is None:
            return None
        raw = self.full[o:o+240]
        a = raw.split(b"\0")[0]
        if len(a) >= minlen and all(32 <= c < 127 for c in a):
            return a.decode()
        # utf16
        out = []
        for i in range(0, len(raw) - 1, 2):
            ch = raw[i] | (raw[i+1] << 8)
            if ch == 0:
                break
            if not (32 <= ch < 127):
                return None
            out.append(chr(ch))
        if len(out) >= minlen:
            return "".join(out)
        return None


def _rip_disp(ins):
    """extract rip-relative displacement from capstone op_str"""
    ops = ins.op_str
    if "rip" not in ops:
        return 0
    part = ops.split("rip")[1]
    part = part.split("]")[0].strip()
    part = part.replace(" ", "")
    if part.startswith("+"):
        part = part[1:]
    neg = part.startswith("-")
    if neg:
        part = part[1:]
    try:
        v = int(part, 16) if part else 0
    except ValueError:
        return 0
    return -v if neg else v


if __name__ == "__main__":
    x = X64.get()
    print(f"base={hex(x.base)} text={hex(x.text_va)}+{hex(len(x.text))} funcs={len(x.pdata_funcs)}")
    # smoke: STAT_NetTickFlush anchor on windows
    fns = x.anchored_funcs_x64("STAT_NetTickFlush")
    print("TickFlush x64 candidates:", {hex(k): len(v) for k, v in fns.items()})
    # smoke: pattern
    hits = x.scan_pattern("48 89 5C 24 ? 48 89 6C 24 ? 56 57 41 56 48 83 EC ? 33 DB 4C 8B F2 48 8B F9 48 39 1A")
    print("CanAfford pattern hits:", [hex(h) for h in hits])
