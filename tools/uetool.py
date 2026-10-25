#!/usr/bin/env python3
"""أداة تحليل ARM64 UE - تعتمد على dumps من uefinder_ext"""
import struct
import bisect
import numpy as np
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_LITTLE_ENDIAN

class UETool:
    def __init__(self, so_path, dump_prefix):
        self.so_path = so_path
        self.prefix = dump_prefix
        # تحميل ELF
        with open(so_path, "rb") as f:
            self.elf = f.read()
        e_shoff = struct.unpack_from("<Q", self.elf, 0x28)[0]
        e_shentsize = struct.unpack_from("<H", self.elf, 0x3A)[0]
        e_shnum = struct.unpack_from("<H", self.elf, 0x3C)[0]
        e_shstrndx = struct.unpack_from("<H", self.elf, 0x3E)[0]
        self.sections = {}
        raw = []
        for i in range(e_shnum):
            off = e_shoff + i * e_shentsize
            sh_name, sh_type = struct.unpack_from("<II", self.elf, off)
            sh_flags, sh_addr, sh_offset, sh_size = struct.unpack_from("<QQQQ", self.elf, off + 8)
            raw.append((sh_name, sh_addr, sh_offset, sh_size))
        strtab_off = raw[e_shstrndx][2]
        for sh_name, addr, off, size in raw:
            end = self.elf.index(b"\0", strtab_off + sh_name)
            nm = self.elf[strtab_off + sh_name:end].decode()
            self.sections[nm] = (addr, off, size)
        self.text_va, self.text_off, self.text_size = self.sections[".text"]
        # الدوال
        self.funcs = np.fromfile(dump_prefix + ".funcs", dtype=np.uint64)
        # المراجع
        self.refs = np.fromfile(dump_prefix + ".refs", dtype=np.uint64).reshape(-1, 2)   # tgt, pc
        self.refl = np.fromfile(dump_prefix + ".refl", dtype=np.uint64).reshape(-1, 2)   # tgt, pc (LDR)
        self.bl = np.fromfile(dump_prefix + ".bl", dtype=np.uint64).reshape(-1, 2)       # pc, tgt
        self.bl_by_pc = np.argsort(self.bl[:, 0], kind="stable")  # فهرس مفروز بالـ PC
        self.rels = np.fromfile(dump_prefix + ".rels", dtype=np.uint64).reshape(-1, 2)   # off, add
        # فرز للبحث الثنائي
        self.refs_by_tgt = np.argsort(self.refs[:, 0], kind="stable")
        self.refl_by_tgt = np.argsort(self.refl[:, 0], kind="stable")
        self.bl_by_tgt = np.argsort(self.bl[:, 1], kind="stable")
        self.rels_by_off = np.argsort(self.rels[:, 0], kind="stable")
        # capstone
        self.cs = Cs(CS_ARCH_ARM64, CS_MODE_LITTLE_ENDIAN)
        self.cs.detail = False

    # ---------- تحويل العناوين ----------
    def va2off(self, va):
        for addr, off, size in self.sections.values():
            if addr <= va < addr + size:
                return off + (va - addr)
        return None

    def read(self, va, n):
        o = self.va2off(va)
        if o is None:
            return None
        return self.elf[o:o+n]

    def u64(self, va):
        o = self.va2off(va)
        return struct.unpack_from("<Q", self.elf, o)[0] if o is not None else None

    def u32(self, va):
        o = self.va2off(va)
        return struct.unpack_from("<I", self.elf, o)[0] if o is not None else None

    # ---------- الدوال ----------
    def func_of(self, va):
        """الدالة التي تحتوي هذا العنوان"""
        i = bisect.bisect_right(self.funcs, va) - 1
        if i < 0:
            return None
        return int(self.funcs[i])

    def func_end(self, fstart):
        i = bisect.bisect_left(self.funcs, fstart)
        if i + 1 < len(self.funcs):
            return int(self.funcs[i+1])
        return int(self.funcs[-1] + 0x10000)

    def is_func(self, va):
        i = bisect.bisect_left(self.funcs, va)
        return i < len(self.funcs) and self.funcs[i] == va

    # ---------- المراجع ----------
    def refs_to(self, target):
        """كل مواقع ADRP+ADD التي تشير للهدف"""
        idx = self.refs_by_tgt
        s = np.searchsorted(self.refs[idx, 0], target, "left")
        e = np.searchsorted(self.refs[idx, 0], target, "right")
        return [(int(pc), int(t)) for t, pc in self.refs[idx][s:e]]

    def refl_to(self, target):
        """كل مواقع ADRP+LDR التي تحمل من العنوان"""
        idx = self.refl_by_tgt
        s = np.searchsorted(self.refl[idx, 0], target, "left")
        e = np.searchsorted(self.refl[idx, 0], target, "right")
        return [(int(pc), int(t)) for t, pc in self.refl[idx][s:e]]

    def callers_of(self, fn):
        """مواقع BL التي تستدعي fn"""
        idx = self.bl_by_tgt
        s = np.searchsorted(self.bl[idx, 1], fn, "left")
        e = np.searchsorted(self.bl[idx, 1], fn, "right")
        return [(int(pc), int(t)) for pc, t in self.bl[idx][s:e]]

    def callees_of(self, fn):
        """كل BL داخل الدالة fn → أهدافها"""
        fend = self.func_end(fn)
        idx = self.bl_by_pc
        s = np.searchsorted(self.bl[idx, 0], fn, "left")
        e = np.searchsorted(self.bl[idx, 0], fend, "left")
        return [(int(pc), int(t)) for pc, t in self.bl[idx][s:e] if fn <= pc < fend]

    def rel_value(self, off):
        """قيمة relocation عند offset (VA)"""
        idx = self.rels_by_off
        s = np.searchsorted(self.rels[idx, 0], off, "left")
        e = np.searchsorted(self.rels[idx, 0], off, "right")
        r = self.rels[idx][s:e]
        return int(r[0][1]) if len(r) else None

    # ---------- تفكيك ----------
    def disasm(self, va, count=40, until_ret=False):
        o = self.va2off(va)
        code = self.elf[o:o+count*4+16]
        out = []
        for ins in self.cs.disasm(code, va):
            out.append(f"0x{ins.address:X}:  {ins.mnemonic:8s} {ins.op_str}")
            if until_ret and ins.mnemonic == "ret":
                break
            if len(out) >= count:
                break
        return "\n".join(out)

    def func_disasm(self, fn, max_ins=400):
        fend = self.func_end(fn)
        n = min((fend - fn) // 4, max_ins)
        return self.disasm(fn, n)

    # ---------- strings ----------
    def find_string(self, s, wide=True):
        """إيجاد نص في .rodata — يرجع قائمة عناوين"""
        ro_addr, ro_off, ro_size = self.sections[".rodata"]
        if wide:
            pat = s.encode("utf-16-le") + b"\0\0"
        else:
            pat = s.encode() + b"\0"
        data = self.elf[ro_off:ro_off+ro_size]
        res = []
        i = data.find(pat)
        while i != -1 and len(res) < 30:
            res.append(ro_addr + i)
            i = data.find(pat, i + 2)
        return res

    def strings_at(self, va_list):
        """فحص النصوص عند عناوين"""
        out = []
        for va in va_list:
            o = self.va2off(va)
            if o is None:
                continue
            raw = self.elf[o:o+128]
            # جرّب wide
            w = raw[:64].decode("utf-16-le", errors="ignore").split("\0")[0]
            a = raw[:64].split(b"\0")[0].decode("ascii", errors="ignore")
            if w and all(32 <= ord(c) < 127 for c in w[:4]):
                out.append((va, "W", w))
            elif a and all(32 < ord(c) < 127 for c in a[:4]):
                out.append((va, "A", a))
        return out
