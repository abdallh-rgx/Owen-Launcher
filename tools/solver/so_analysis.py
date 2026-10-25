#!/usr/bin/env python3
"""UNIVERSAL .so analyzer for Erbium function finding — works on ANY
Fortnite Android ARM64 binary (libUE4.so / libUnreal.so), versions 5.x–30.00.

Everything is derived from the ELF itself — no external dumps, no per-version
knowledge, no symbol tables required:

  * ELF parsing (program headers, sections, .dynamic)
  * Relocations: DT_RELA / DT_REL / DT_ANDROID_RELA(A) / DT_RELR / APS2 packed
  * Function boundaries: .eh_frame_hdr table + full .eh_frame CIE/FDE walk
  * Vectorized .text decodes (numpy):
      - ADRP+ADD  materialization refs   (rodata/data pointers)
      - ADRP+LDR  GOT / literal loads    (globals through GOT)
      - ADR       .text literal pools    (UTF-16 strings living in .text!)
      - BL/B      direct call graph
  * Strings: ASCII + UTF-16LE in .rodata and inside .text literal pools
  * VTable runs: relocation-slot runs in .data.rel.ro
  * Native UFunction table: {Name char* → exec thunk} relocation pairs
  * Version auto-detection from "Fortnite+Release-XX.XX-CL-YYYYYYY"

This module is the foundation of erbium_solver.py (the universal Erbium
offset solver).  It is deliberately self-contained: numpy is the only
non-stdlib dependency.
"""
import os
import re
import struct
import bisect
import numpy as np

R_AARCH64_RELATIVE = 1027
PT_LOAD, PT_DYNAMIC = 1, 2


def _sleb(buf, pos):
    result = 0
    shift = 0
    while True:
        b = buf[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        shift += 7
        if not (b & 0x80):
            break
        if shift > 63:
            break
    # sign extension from the last byte
    if shift < 64 and (b & 0x40):
        result |= -(1 << shift)
    return result, pos


class SoAnalysis:
    """Universal analyzer.  Load once per binary; all indexes built lazily."""

    def __init__(self, so_path):
        self.path = so_path
        with open(so_path, "rb") as f:
            self.elf = f.read()
        e = self.elf
        if e[:4] != b"\x7fELF" or e[4] != 2 or e[5] != 1:
            raise ValueError("not an ELF64 LE binary")
        (self.e_type, self.e_machine) = struct.unpack_from("<HH", e, 16)
        self.e_phoff, = struct.unpack_from("<Q", e, 32)
        self.e_shoff, = struct.unpack_from("<Q", e, 40)
        self.e_phentsize, self.e_phnum = struct.unpack_from("<HH", e, 54)
        self.e_shentsize, self.e_shnum, self.e_shstrndx = struct.unpack_from("<HHH", e, 58)

        # ---------- program headers ----------
        self.phdrs = []
        for i in range(self.e_phnum):
            o = self.e_phoff + i * self.e_phentsize
            p_type, p_flags = struct.unpack_from("<II", e, o)
            p_offset, p_vaddr = struct.unpack_from("<QQ", e, o + 8)
            p_filesz, = struct.unpack_from("<Q", e, o + 32)
            self.phdrs.append((p_type, p_flags, p_offset, p_vaddr, p_filesz))

        # ---------- sections ----------
        if self.e_shoff and self.e_shnum:
            sh = []
            for i in range(self.e_shnum):
                o = self.e_shoff + i * self.e_shentsize
                sh_name, sh_type = struct.unpack_from("<II", e, o)
                sh_flags, sh_addr, sh_offset, sh_size = struct.unpack_from("<QQQQ", e, o + 8)
                sh.append((sh_name, sh_type, sh_flags, sh_addr, sh_offset, sh_size))
            strtab_off = sh[self.e_shstrndx][4]
            self.sections = {}
            for sh_name, sh_type, sh_flags, sh_addr, sh_offset, sh_size in sh:
                end = e.index(b"\0", strtab_off + sh_name)
                nm = e[strtab_off + sh_name:end].decode()
                self.sections[nm] = (sh_addr, sh_offset, sh_size)
        else:
            self.sections = {}
        self._dyn = self._parse_dynamic()

        # .text
        if ".text" in self.sections:
            self.text_va, self.text_off, self.text_size = self.sections[".text"]
        else:  # stripped section headers: find executable PT_LOAD
            for p_type, p_flags, p_offset, p_vaddr, p_filesz in self.phdrs:
                if p_type == PT_LOAD and (p_flags & 1):  # PF_X
                    self.text_va, self.text_off, self.text_size = p_vaddr, p_offset, p_filesz
                    break
        self.text_words = None  # lazy

        # .rodata
        if ".rodata" in self.sections:
            self.ro_va, self.ro_off, self.ro_size = self.sections[".rodata"]
        else:
            self.ro_va = self.ro_off = self.ro_size = 0

        # .data.rel.ro
        if ".data.rel.ro" in self.sections:
            self.drr_va, self.drr_off, self.drr_size = self.sections[".data.rel.ro"]
        else:
            self.drr_va = self.drr_off = self.drr_size = 0

        # lazy indexes
        self._rels = None            # sorted (off, add) RELATIVE slots
        self._rels_by_off = None
        self._funcs = None           # sorted starts
        self._fends = None
        self._refs = None            # (tgt, pc) ADRP+ADD
        self._refl = None            # (tgt, pc) ADRP+LDR literal (GOT)
        self._adr = None             # (tgt, pc) ADR literal
        self._bl = None              # (pc, tgt)
        self._strings_ascii = None
        self._strings_utf16 = None
        self._vt_runs = None
        self._natives = None
        self._version = "?"
        self._detect_version()

    # =========================== addressing ===========================
    def va2off(self, va):
        for p_type, _fl, p_offset, p_vaddr, p_filesz in self.phdrs:
            if p_type == PT_LOAD and p_vaddr <= va < p_vaddr + p_filesz:
                return p_offset + (va - p_vaddr)
        return None

    def read(self, va, n):
        o = self.va2off(va)
        return None if o is None else self.elf[o:o + n]

    def u32(self, va):
        o = self.va2off(va)
        return None if o is None else struct.unpack_from("<I", self.elf, o)[0]

    def u64(self, va):
        o = self.va2off(va)
        return None if o is None else struct.unpack_from("<Q", self.elf, o)[0]

    def in_text(self, va):
        return self.text_va <= va < self.text_va + self.text_size

    def off2va_text(self, off):
        return self.text_va + (off - self.text_off)

    # =========================== dynamic/relocs ===========================
    def _parse_dynamic(self):
        dyn = {"rela": 0, "relasz": 0, "relaent": 24, "rel": 0, "relsz": 0, "relent": 16,
               "arela": 0, "arelasz": 0, "arel": 0, "arelsz": 0, "relr": 0, "relrsz": 0}
        p = None
        for p_type, _fl, p_offset, p_vaddr, p_filesz in self.phdrs:
            if p_type == PT_DYNAMIC:
                p, sz = p_offset, p_filesz
                break
        if p is None:
            return dyn
        for k in range(0, sz - 15, 16):
            tag, val = struct.unpack_from("<QQ", self.elf, p + k)
            if tag == 0:
                break
            m = {7: "rela", 8: "relasz", 9: "relaent", 17: "rel", 18: "relsz", 19: "relent",
                 0x6FFFE000: "arela", 0x60000011: "arela", 0x6FFFE001: "arelasz",
                 0x6FFFF000: "arel", 0x6000000F: "arel", 0x6FFFF001: "arelsz",
                 0x6FFFE000 + 1: None, 36: "relr", 38: "relrsz"}
            key = m.get(tag)
            if key:
                dyn[key] = val
        return dyn

    def _decode_relocs(self):
        """all R_AARCH64_RELATIVE slots → np array (off, add) sorted by off"""
        slots = []
        d = self._dyn

        def add(off, val):
            slots.append((off, val))

        # plain RELA / REL
        for base_key, sz_key, ent_key, is_rel in (("rela", "relasz", "relaent", False),
                                                  ("rel", "relsz", "relent", True)):
            off_v, sz_v = d[base_key], d[sz_key]
            if not off_v or not sz_v:
                continue
            o = self.va2off(off_v)
            if o is None:
                continue
            ent = d[ent_key]
            for k in range(0, sz_v - ent + 1, ent):
                ro, ri = struct.unpack_from("<QQ", self.elf, o + k)
                if (ri & 0xFFFFFFFF) != R_AARCH64_RELATIVE:
                    continue
                if is_rel:
                    q = self.va2off(ro)
                    if q is None:
                        continue
                    val, = struct.unpack_from("<Q", self.elf, q)
                else:
                    val, = struct.unpack_from("<Q", self.elf, o + k + 16)
                add(ro, val)

        # Android packed APS2/APR2 (robust routine below)
        for base_key, sz_key in (("arela", "arelasz"), ("arel", "arelsz")):
            off_v, sz_v = d[base_key], d[sz_key]
            if not off_v or sz_v < 8:
                continue
            o = self.va2off(off_v)
            if o is None:
                continue
            buf = self.elf[o:o + sz_v]
            if buf[:4] not in (b"APS2", b"APR2"):
                continue
            slots.extend(self._decode_aps2_region(o, sz_v))

        # RELR
        if d["relr"] and d["relrsz"]:
            o = self.va2off(d["relr"])
            if o is not None:
                base = 0
                for k in range(0, d["relrsz"] - 7, 8):
                    w, = struct.unpack_from("<Q", self.elf, o + k)
                    if w & 1:
                        base = w & ~1
                        q = self.va2off(base)
                        if q is not None:
                            add(base, struct.unpack_from("<Q", self.elf, q)[0])
                    else:
                        for b in range(63):
                            if w & (1 << b):
                                a = base + (b + 1) * 8
                                q = self.va2off(a)
                                if q is not None:
                                    add(a, struct.unpack_from("<Q", self.elf, q)[0])

        if not slots:  # last-resort: any packed region even without dyn entries
            for nm in (".rela.dyn", ".rel.dyn"):
                if nm in self.sections:
                    va, off, size = self.sections[nm]
                    buf = self.elf[off:off + min(size, 16)]
                    if buf[:4] in (b"APS2", b"APR2"):
                        slots = self._decode_aps2_region(off, size)
                        if slots:
                            break
        arr = np.array(slots, dtype=np.uint64).reshape(-1, 2)
        order = np.argsort(arr[:, 0], kind="stable")
        arr = arr[order]
        # dedupe (keep first)
        keep = np.ones(len(arr), dtype=bool)
        if len(arr) > 1:
            keep[1:] = arr[1:, 0] != arr[:-1, 0]
        return arr[keep]

    def _decode_aps2_region(self, off, sz):
        """decode one APS2/APR2 packed-relocation region → [(slot_va, addend)]"""
        slots = []
        buf = self.elf[off:off + sz]
        if len(buf) < 8 or buf[:4] not in (b"APS2", b"APR2"):
            return slots
        pos = 4
        count, pos = _sleb(buf, pos)
        offset, pos = _sleb(buf, pos)
        addend = 0
        done = 0
        while done < count and pos < len(buf):
            gsize, pos = _sleb(buf, pos)
            gflags, pos = _sleb(buf, pos)
            g_offdelta = g_info = g_addend = 0
            if gflags & 2:
                g_offdelta, pos = _sleb(buf, pos)
            if gflags & 1:
                g_info, pos = _sleb(buf, pos)
            if (gflags & 8) and (gflags & 4):
                g_addend, pos = _sleb(buf, pos)
            for _ in range(gsize):
                if done >= count or pos >= len(buf):
                    break
                if not (gflags & 2):
                    delta, pos = _sleb(buf, pos)
                    offset += delta
                else:
                    offset += g_offdelta
                if gflags & 1:
                    info = g_info
                else:
                    info, pos = _sleb(buf, pos)
                if gflags & 8:
                    if gflags & 4:
                        addend = g_addend
                    else:
                        dlt, pos = _sleb(buf, pos)
                        addend += dlt
                    val = addend & 0xFFFFFFFFFFFFFFFF
                    have = True
                else:
                    q = self.va2off(offset & 0xFFFFFFFFFFFFFFFF)
                    if q is None:
                        have = False
                    else:
                        val, = struct.unpack_from("<Q", self.elf, q)
                        have = True
                done += 1
                if have and (info & 0xFFFFFFFF) == R_AARCH64_RELATIVE:
                    slots.append((offset & 0xFFFFFFFFFFFFFFFF, val))
        return slots

    @property
    def rels(self):
        if self._rels is None:
            self._rels = self._decode_relocs()
        return self._rels

    def rel_value(self, off_va):
        """addend of the RELATIVE relocation at slot off_va (None if absent)"""
        r = self.rels
        s = np.searchsorted(r[:, 0], off_va, "left")
        e = np.searchsorted(r[:, 0], off_va, "right")
        return int(r[s, 1]) if e > s else None

    # =========================== functions (.eh_frame) ===========================
    def _build_funcs(self):
        """function starts+ends from .eh_frame_hdr + .eh_frame"""
        ranges = {}

        def add(start, end):
            if self.in_text(start):
                if start not in ranges or (end and (ranges[start] == 0 or end > ranges[start])):
                    if end == 0 or ranges.get(start, 0) == 0:
                        ranges[start] = end
                    else:
                        ranges[start] = max(ranges[start], end)

        # --- .eh_frame_hdr binary search table ---
        if ".eh_frame_hdr" in self.sections:
            va, off, size = self.sections[".eh_frame_hdr"]
            p = self.elf[off:off + size]
            if size >= 12 and p[0] == 1:
                eptr_enc, cnt_enc, tab_enc = p[1], p[2], p[3]
                pos = 4
                hdr_va = va
                r = self._read_enc_ptr_simple(p, pos, eptr_enc, hdr_va + pos, hdr_va)
                if r:
                    pos = r[1]
                r = self._read_enc_ptr_simple(p, pos, cnt_enc, hdr_va + pos, hdr_va)
                if r:
                    cnt = r[0]
                    pos = r[1]
                    if 0 < cnt <= 4_000_000 and pos + cnt * 8 <= size:
                        for k in range(cnt):
                            a = self._read_enc_ptr_simple(p, pos, tab_enc, hdr_va + pos, hdr_va)
                            if not a:
                                break
                            pos = a[1]
                            b = self._read_enc_ptr_simple(p, pos, tab_enc, hdr_va + pos, hdr_va)
                            if not b:
                                break
                            pos = b[1]
                            add(a[0], 0)

        # --- full .eh_frame walk ---
        if ".eh_frame" in self.sections:
            va, off, size = self.sections[".eh_frame"]
            p = self.elf[off:off + size]
            eh_va = va
            if size >= 8:
                # pass 1: CIEs (position → fde encoding)
                cies = {}
                pos = 0
                while pos + 4 <= size:
                    ln, = struct.unpack_from("<I", p, pos)
                    if ln == 0 or ln == 0xFFFFFFFF:
                        break
                    after = pos + 4 + ln
                    if after > size:
                        break
                    if struct.unpack_from("<I", p, pos + 4)[0] == 0:  # CIE
                        q = pos + 8
                        if q < size and p[q] != 1:
                            pos = after
                            continue
                        q += 1
                        aug = bytearray()
                        while q < size and p[q]:
                            aug.append(p[q]); q += 1
                        q += 1
                        # code align uleb / data align sleb / ret reg uleb
                        while q < size and (p[q] & 0x80):
                            q += 1
                        q += 1
                        while q < size:
                            b = p[q]; q += 1
                            if not (b & 0x80):
                                break
                        while q < size and (p[q] & 0x80):
                            q += 1
                        q += 1
                        fde_enc = 0x1B
                        if aug[:1] == b"z":
                            alen = 0
                            sh = 0
                            while q < size:
                                b = p[q]; q += 1
                                alen |= (b & 0x7F) << sh
                                sh += 7
                                if not (b & 0x80):
                                    break
                            aend = q + alen
                            for c in aug[1:]:
                                if c == ord("L"):
                                    q += 1
                                elif c == ord("P"):
                                    if q < size:
                                        pe = p[q]; q += 1
                                        r = self._read_enc_ptr_simple(p, q, pe, eh_va + q, eh_va)
                                        if r:
                                            q = r[1]
                                elif c == ord("R"):
                                    if q < size:
                                        fde_enc = p[q]; q += 1
                                else:
                                    break
                            q = aend
                        cies[pos] = fde_enc
                    pos = after
                # pass 2: FDEs
                pos = 0
                dr_base = eh_va
                hdr_va2 = self.sections.get(".eh_frame_hdr", (0, 0, 0))[0]
                while pos + 4 <= size:
                    ln, = struct.unpack_from("<I", p, pos)
                    if ln == 0 or ln == 0xFFFFFFFF:
                        break
                    after = pos + 4 + ln
                    if after > size:
                        break
                    cid = struct.unpack_from("<I", p, pos + 4)[0]
                    if cid != 0:
                        cie_pos = pos + 4 - cid
                        enc = cies.get(cie_pos, 0x1B)
                        q = pos + 8
                        ok = False
                        iloc = 0
                        if (enc & 0xF0) == 0x30:
                            for base in (eh_va, hdr_va2, 0):
                                r = self._read_enc_ptr_simple(p, q, enc, eh_va + q, base)
                                if r and self.in_text(r[0]):
                                    iloc = r[0]; q = r[1]; ok = True
                                    dr_base = base
                                    break
                        else:
                            r = self._read_enc_ptr_simple(p, q, enc, eh_va + q, dr_base)
                            if r and self.in_text(r[0]):
                                iloc = r[0]; q = r[1]; ok = True
                            if not ok:
                                r = self._read_enc_ptr_simple(p, q, enc, eh_va + q, 0)
                                if r and self.in_text(r[0]):
                                    iloc = r[0]; q = r[1]; ok = True
                        if ok:
                            fmt = enc & 0x0F
                            rng = 0
                            if fmt in (0x03, 0x0B, 0x09) and q + 4 <= size:
                                rng, = struct.unpack_from("<I", p, q)
                            elif fmt in (0x0A, 0x04) and q + 8 <= size:
                                rng, = struct.unpack_from("<Q", p, q)
                            elif fmt in (0x02, 0x0C) and q + 2 <= size:
                                rng, = struct.unpack_from("<H", p, q)
                            add(iloc, iloc + rng if rng else 0)
                    pos = after

        starts = np.array(sorted(ranges), dtype=np.uint64)
        ends = np.array([ranges[int(s)] for s in starts], dtype=np.uint64)
        text_end = self.text_va + self.text_size
        for i in range(len(starts)):
            if ends[i] == 0 or ends[i] > text_end:
                ends[i] = starts[i + 1] if i + 1 < len(starts) and starts[i + 1] <= text_end else text_end
        self._funcs, self._fends = starts, ends
        return starts

    def _read_enc_ptr_simple(self, buf, pos, enc, pc, datarel):
        """clean pointer-encoding decoder → (value, new_pos) | None"""
        if enc == 0xFF:
            return None
        base, fmt = enc & 0xF0, enc & 0x0F
        n = pos
        if fmt == 0x00:
            if n + 8 > len(buf):
                return None
            v, = struct.unpack_from("<Q", buf, n); n += 8
        elif fmt in (0x02, 0x0C):
            if n + 2 > len(buf):
                return None
            v, = struct.unpack_from("<H", buf, n); n += 2
        elif fmt in (0x03, 0x09, 0x0B):
            if n + 4 > len(buf):
                return None
            v, = struct.unpack_from("<i" if fmt in (0x09, 0x0B) else "<I", buf, n); n += 4
        elif fmt in (0x04, 0x0A):
            if n + 8 > len(buf):
                return None
            v, = struct.unpack_from("<q" if fmt == 0x0A else "<Q", buf, n); n += 8
        elif fmt == 0x01:
            if n >= len(buf):
                return None
            v = buf[n]; n += 1
        else:
            return None
        if base == 0x10:      # pcrel — stored value is relative to its own slot
            v = pc + v
        elif base == 0x30:    # datarel
            v = datarel + v
        return v & 0xFFFFFFFFFFFFFFFF, n

    @property
    def funcs(self):
        if self._funcs is None:
            self._build_funcs()
        return self._funcs

    def func_of(self, va):
        i = bisect.bisect_right(self.funcs, va) - 1
        return int(self.funcs[i]) if i >= 0 else None

    def func_end(self, fstart):
        i = bisect.bisect_left(self.funcs, fstart)
        return int(self._fends[i]) if i < len(self.funcs) else None

    def func_size(self, fstart):
        e = self.func_end(fstart)
        return (e - fstart) if e else 0

    def is_func(self, va):
        i = bisect.bisect_left(self.funcs, va)
        return i < len(self.funcs) and int(self.funcs[i]) == va

    def next_func(self, va):
        i = bisect.bisect_right(self.funcs, va)
        return int(self.funcs[i]) if i < len(self.funcs) else None

    # =========================== .text decodes ===========================
    @property
    def words(self):
        if self.text_words is None:
            n = self.text_size // 4
            self.text_words = np.frombuffer(self.elf, dtype="<u4", count=n, offset=self.text_off)
        return self.text_words

    def _scan_refs(self):
        """vectorized ADRP+ADD / ADRP+LDR / ADR decode across all of .text"""
        w = self.words.astype(np.uint64)
        n = len(w)
        pc = self.text_va + np.arange(n, dtype=np.uint64) * 4

        # ---- ADRP ----
        adrp = (w & np.uint64(0x9F000000)) == np.uint64(0x90000000)
        ai = np.flatnonzero(adrp)
        if len(ai):
            wa = w[ai]
            rd = wa & np.uint64(0x1F)
            lo = (wa >> np.uint64(29)) & np.uint64(3)
            hi = (wa >> np.uint64(5)) & np.uint64(0x7FFFF)
            simm = ((hi << np.uint64(2)) | lo).astype(np.int64)
            simm = (simm ^ np.int64(1 << 20)) - np.int64(1 << 20)   # sext 21
            page = (pc[ai] & np.uint64(0xFFFFFFFFFFFFF000)) + (simm.astype(np.uint64) << np.uint64(12))

            refs = []   # (tgt, pc)
            refl = []   # (tgt, pc) GOT loads
            KMAX = 8
            for k in range(1, KMAX + 1):
                j = ai + k
                valid = j < n
                if not valid.any():
                    break
                jj = j[valid]
                wk = w[jj]
                # ADD Xd, Xn, #imm (64-bit)
                is_add = (wk & np.uint64(0xFF800000)) == np.uint64(0x91000000)
                rn_add = (wk >> np.uint64(5)) & np.uint64(0x1F)
                sub_add = is_add & (rn_add == rd[valid])
                if sub_add.any():
                    match = np.zeros(len(ai), dtype=bool)
                    match[valid] = sub_add
                    idx = np.flatnonzero(match)
                    ii = ai[idx]
                    imm12 = (w[ii + k] >> np.uint64(10)) & np.uint64(0xFFF)
                    sh = (w[ii + k] >> np.uint64(22)) & np.uint64(3)
                    imm = np.where(sh == 1, imm12 << np.uint64(12), imm12)
                    tgt = page[idx] + imm
                    refs.append(np.stack([tgt, pc[ii]], axis=1))
                    rd[idx] = np.uint64(99)   # consumed
                # LDR Xt, [Xn, #imm] (adrp+ldr → GOT / literal)
                is_ldr = (wk & np.uint64(0xFFC00000)) == np.uint64(0xF9400000)
                rn_ldr = (wk >> np.uint64(5)) & np.uint64(0x1F)
                sub_ldr = is_ldr & (rn_ldr == rd[valid])
                if sub_ldr.any():
                    match = np.zeros(len(ai), dtype=bool)
                    match[valid] = sub_ldr
                    idx = np.flatnonzero(match)
                    ii = ai[idx]
                    imm12 = (w[ii + k] >> np.uint64(10)) & np.uint64(0xFFF)
                    tgt = page[idx] + imm12 * np.uint64(8)
                    refl.append(np.stack([tgt, pc[ii]], axis=1))
                    rd[idx] = np.uint64(99)   # consumed
            self._refs = np.concatenate(refs) if refs else np.zeros((0, 2), np.uint64)
            self._refl = np.concatenate(refl) if refl else np.zeros((0, 2), np.uint64)
            self._refs_by_tgt = np.argsort(self._refs[:, 0], kind="stable")
            self._refl_by_tgt = np.argsort(self._refl[:, 0], kind="stable")
        else:
            self._refs = np.zeros((0, 2), np.uint64)
            self._refl = np.zeros((0, 2), np.uint64)
            self._refs_by_tgt = np.zeros(0, int)
            self._refl_by_tgt = np.zeros(0, int)

        # ---- ADR (literal pool strings inside .text) ----
        adr = (w & np.uint64(0x9F000000)) == np.uint64(0x10000000)
        if adr.any():
            idx = np.flatnonzero(adr)
            wa = w[idx]
            immlo = (wa >> np.uint64(29)) & np.uint64(3)
            immhi = (wa >> np.uint64(5)) & np.uint64(0x7FFFF)
            simm = ((immhi << np.uint64(2)) | immlo).astype(np.int64)
            simm = (simm ^ np.int64(1 << 20)) - np.int64(1 << 20)
            tgt = (pc[idx].astype(np.int64) + simm).astype(np.uint64)
            self._adr = np.stack([tgt, pc[idx]], axis=1)
            self._adr_by_tgt = np.argsort(self._adr[:, 0], kind="stable")
        else:
            self._adr = np.zeros((0, 2), np.uint64)
            self._adr_by_tgt = np.zeros(0, int)

        # ---- BL / B ----
        bl = (w & np.uint64(0xFC000000)) == np.uint64(0x94000000)
        if bl.any():
            idx = np.flatnonzero(bl)
            imm = (w[idx] & np.uint64(0x03FFFFFF)).astype(np.int64)
            imm = (imm ^ np.int64(1 << 25)) - np.int64(1 << 25)
            tgt = (pc[idx].astype(np.int64) + (imm << np.int64(2))).astype(np.uint64)
            self._bl = np.stack([pc[idx], tgt], axis=1)
            self._bl_by_tgt = np.argsort(self._bl[:, 1], kind="stable")
            self._bl_by_pc = np.argsort(self._bl[:, 0], kind="stable")
        else:
            self._bl = np.zeros((0, 2), np.uint64)
            self._bl_by_tgt = self._bl_by_pc = np.zeros(0, int)

    def _ensure_scan(self):
        if self._refs is None:
            self._scan_refs()

    # ---- query API ----
    def refs_to(self, target):
        self._ensure_scan()
        s = np.searchsorted(self._refs[self._refs_by_tgt, 0], target, "left")
        e = np.searchsorted(self._refs[self._refs_by_tgt, 0], target, "right")
        return [(int(p), int(t)) for t, p in self._refs[self._refs_by_tgt][s:e]]

    def refl_to(self, target):
        self._ensure_scan()
        s = np.searchsorted(self._refl[self._refl_by_tgt, 0], target, "left")
        e = np.searchsorted(self._refl[self._refl_by_tgt, 0], target, "right")
        return [(int(p), int(t)) for t, p in self._refl[self._refl_by_tgt][s:e]]

    def adr_to(self, target):
        self._ensure_scan()
        s = np.searchsorted(self._adr[self._adr_by_tgt, 0], target, "left")
        e = np.searchsorted(self._adr[self._adr_by_tgt, 0], target, "right")
        return [(int(p), int(t)) for t, p in self._adr[self._adr_by_tgt][s:e]]

    def callers_of(self, fn):
        self._ensure_scan()
        s = np.searchsorted(self._bl[self._bl_by_tgt, 1], fn, "left")
        e = np.searchsorted(self._bl[self._bl_by_tgt, 1], fn, "right")
        return [(int(p), int(t)) for p, t in self._bl[self._bl_by_tgt][s:e]]

    def bls_in(self, fn):
        """BL sites inside function (pc-sorted)"""
        self._ensure_scan()
        fend = self.func_end(fn)
        s = np.searchsorted(self._bl[self._bl_by_pc, 0], fn, "left")
        e = np.searchsorted(self._bl[self._bl_by_pc, 0], fend, "left")
        return [(int(p), int(t)) for p, t in self._bl[self._bl_by_pc][s:e] if fn <= p < fend]

    def refs_in(self, fn):
        """ADRP+ADD materializations inside function → [(tgt, pc)]"""
        self._ensure_scan()
        order = np.argsort(self._refs[:, 1], kind="stable")
        fend = self.func_end(fn)
        s = np.searchsorted(self._refs[order, 1], fn, "left")
        e = np.searchsorted(self._refs[order, 1], fend, "left")
        return [(int(t), int(p)) for t, p in self._refs[order][s:e] if fn <= p < fend]

    def adr_in(self, fn):
        self._ensure_scan()
        order = np.argsort(self._adr[:, 1], kind="stable")
        fend = self.func_end(fn)
        s = np.searchsorted(self._adr[order, 1], fn, "left")
        e = np.searchsorted(self._adr[order, 1], fend, "left")
        return [(int(t), int(p)) for t, p in self._adr[order][s:e] if fn <= p < fend]

    def read_string_at(self, va, minlen=4, maxlen=160):
        """ASCII or UTF-16LE string at va → ('A'|'W', str) | None"""
        raw = self.read(va, maxlen * 2 + 2)
        if not raw:
            return None
        a = raw.split(b"\0", 1)[0]
        if len(a) >= minlen and all(32 <= c < 127 for c in a):
            return ("A", a.decode())
        out = []
        for i in range(0, len(raw) - 1, 2):
            ch = raw[i] | (raw[i + 1] << 8)
            if ch == 0:
                break
            if not (32 <= ch < 127):
                return None
            out.append(chr(ch))
        if len(out) >= minlen:
            return ("W", "".join(out))
        return None

    def find_strings(self, s, wide=True, limit=40):
        """all VAs where string s occurs (utf16 and/or ascii), searching
        .rodata AND .text literal pools."""
        res = []
        pats = []
        if wide:
            pats.append(s.encode("utf-16-le") + b"\0\0")
        pats.append(s.encode() + b"\0")
        regions = []
        if self.ro_size:
            regions.append(("ro", self.ro_off, self.ro_size, self.ro_va))
        regions.append(("text", self.text_off, self.text_size, self.text_va))
        for _rname, off, size, base in regions:
            data = self.elf[off:off + size]
            for pat in pats:
                i = data.find(pat)
                while i != -1 and len(res) < limit:
                    res.append(base + i)
                    i = data.find(pat, i + 2)
                if res:
                    return res
        return res

    # =========================== vtables ===========================
    def vtable_runs(self, min_len=3):
        """relocation-slot runs (stride 8) in .data.rel.ro → list of runs;
        run = {base, slots:[funcVA...]}. Includes vtables + UFunction arrays."""
        if self._vt_runs is not None:
            return self._vt_runs
        rels = self.rels
        if not len(rels) or not self.drr_size:
            self._vt_runs = []
            return self._vt_runs
        lo, hi = self.drr_va, self.drr_va + self.drr_size
        m = (rels[:, 0] >= lo) & (rels[:, 0] < hi)
        sub = rels[m]
        runs = []
        if len(sub):
            cur_base = int(sub[0, 0])
            cur = [int(sub[0, 1])]
            for i in range(1, len(sub)):
                off, add = int(sub[i, 0]), int(sub[i, 1])
                if off == cur_base + len(cur) * 8:
                    cur.append(add)
                else:
                    if len(cur) >= min_len:
                        runs.append({"base": cur_base, "slots": cur})
                    cur_base, cur = off, [add]
            if len(cur) >= min_len:
                runs.append({"base": cur_base, "slots": cur})
        self._vt_runs = runs
        return runs

    def vtable_of_func(self, fn):
        """runs containing slot == fn"""
        out = []
        for r in self.vtable_runs():
            try:
                idx = r["slots"].index(fn)
                out.append((r, idx))
            except ValueError:
                pass
        return out

    # =========================== natives ===========================
    def native_table(self):
        """{UFunctionName → [thunk VAs]} — universal Path A:
        a string occurrence S gets a relocation slot pointing at it
        (in writable data); the slot's ±8 buddy pointing into .text is
        the native exec thunk.  Works on every version, no symbols needed."""
        if self._natives is not None:
            return self._natives
        natives = {}
        rels = self.rels
        if len(rels) >= 2:
            off_arr = rels[:, 0]
            add_arr = rels[:, 1]
            text_lo, text_hi = self.text_va, self.text_va + self.text_size
            # ---- pre-filter: slot pointing at readable string regions ----
            str_lo, str_hi = text_hi, text_lo  # empty by default
            regions = []
            if self.ro_size:
                regions.append((self.ro_va, self.ro_va + self.ro_size))
            regions.append((text_lo, text_hi))
            # candidate slots: add in a string region
            cand_mask = np.zeros(len(rels), dtype=bool)
            for lo, hi in regions:
                cand_mask |= (add_arr >= lo) & (add_arr < hi)
            ci = np.flatnonzero(cand_mask)
            if len(ci):
                # buddy lookup: for each candidate slot off, find slot off+8 and off-8
                # rels sorted by off → binary search
                plus = np.searchsorted(off_arr, off_arr[ci] + 8)
                plus_c = np.clip(plus, 0, len(off_arr) - 1)
                has_plus = off_arr[plus_c] == off_arr[ci] + 8
                plus_val = add_arr[plus_c]
                ok_plus = has_plus & (plus_val >= text_lo) & (plus_val < text_hi) & (plus_val != add_arr[ci])
                minus = np.searchsorted(off_arr, off_arr[ci] - 8)
                minus_c = np.clip(minus, 0, len(off_arr) - 1)
                has_minus = off_arr[minus_c] == off_arr[ci] - 8
                minus_val = add_arr[minus_c]
                ok_minus = has_minus & (minus_val >= text_lo) & (minus_val < text_hi) & (minus_val != add_arr[ci])
                hit_local = np.flatnonzero(ok_plus | ok_minus)
                for h in hit_local:
                    i = int(ci[h])
                    sva = int(add_arr[i])
                    s = self.read_string_at(sva, 4, 96)
                    if not s or s[0] != "A":
                        continue
                    nm = s[1]
                    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", nm):
                        continue
                    if not (4 <= len(nm) <= 90):
                        continue
                    if ok_plus[h]:
                        natives.setdefault(nm, []).append(int(plus_val[h]))
                    if ok_minus[h]:
                        natives.setdefault(nm, []).append(int(minus_val[h]))
        self._natives = natives
        return natives

    def native_addr(self, name):
        """primary thunk address for a UFunction name (None if unknown)"""
        nat = self.native_table()
        v = nat.get(name)
        return v[0] if v else None

    # =========================== version ===========================
    def _detect_version(self):
        pats = [rb"Fortnite\+Release-(\d+\.\d+)-CL-(\d+)",
                rb"Fortnite/Release-(\d+\.\d+)",
                rb"Fortnite\+Release\+(\d+\.\d+)"]
        regions = []
        if self.ro_size:
            regions.append((self.ro_off, self.ro_size))
        regions.append((self.text_off, min(self.text_size, 20_000_000)))
        for off, size in regions:
            data = self.elf[off:off + size]
            for pat in pats:
                m = re.search(pat, data)
                if m:
                    self._version = m.group(1).decode()
                    return
        self._version = "?"

    @property
    def version(self):
        return self._version

    # =========================== pattern scan ===========================
    def scan_pattern(self, pat, limit=10):
        """masked hex pattern ("F9 0F ?? F8") over .text → [VAs] (regex-based)"""
        self._ensure_scan()
        parts = pat.split()
        rx_parts = []
        for p in parts:
            if p in ("?", "??"):
                rx_parts.append(b".")
            else:
                rx_parts.append(re.escape(bytes([int(p, 16)])))
        rx = re.compile(b"".join(rx_parts), re.DOTALL)
        m = len(parts)
        lo, hi = self.text_off, self.text_off + self.text_size
        hits = []
        for mt in rx.finditer(self.elf, lo, hi - m + 1):
            hits.append(self.text_va + (mt.start() - lo))
            if len(hits) >= limit:
                break
        return hits

    # =========================== de-outline ===========================
    def deoutline(self, fn, stub_max=0x60, max_layers=4):
        """Android outlined-log awareness: tiny stub → B-thunk → real caller.
        Returns (real_func, layers)."""
        cur = fn
        layers = 0
        for _ in range(max_layers):
            sz = self.func_size(cur)
            if sz >= stub_max:
                break
            callers = set()
            for pc, _t in self.callers_of(cur):
                f = self.func_of(pc)
                if f is not None and f != cur:
                    callers.add(f)
            if not callers:
                break
            if len(callers) == 1:
                cur = next(iter(callers))
                layers += 1
                continue
            nxt = max(callers, key=lambda f: self.func_size(f))
            if self.func_size(nxt) <= sz:
                break
            cur = nxt
            layers += 1
        return cur, layers

    def strings_of(self, fn, minlen=6):
        """every string (ADRP+ADD and ADR literal) referenced inside fn"""
        out = {}
        for tgt, pc in self.refs_in(fn):
            s = self.read_string_at(tgt, minlen)
            if s:
                out.setdefault(s[1], pc)
        for tgt, pc in self.adr_in(fn):
            s = self.read_string_at(tgt, minlen)
            if s:
                out.setdefault(s[1], pc)
        return out

    def close(self):
        pass


if __name__ == "__main__":
    import sys, time
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/work2/src/21.30/libUnreal.so"
    t0 = time.time()
    a = SoAnalysis(path)
    print(f"loaded {len(a.elf)/1e6:.0f}MB in {time.time()-t0:.1f}s — version detected: {a.version}")
    t0 = time.time()
    print(f"funcs: {len(a.funcs)} ({time.time()-t0:.1f}s)")
    t0 = time.time()
    a._ensure_scan()
    print(f"refs: {len(a._refs)}  refl(GOT): {len(a._refl)}  adr: {len(a._adr)}  bl: {len(a._bl)} ({time.time()-t0:.1f}s)")
    t0 = time.time()
    nat = a.native_table()
    print(f"natives: {len(nat)} ({time.time()-t0:.1f}s)")
    t0 = time.time()
    print(f"vtable runs: {len(a.vtable_runs())} ({time.time()-t0:.1f}s)")
    # smoke: find a known function via natives
    for probe in ("ClearAbility", "K2_GiveAbility", "ServerKickPlayer"):
        print(probe, "->", hex(a.native_addr(probe)) if a.native_addr(probe) else None)
