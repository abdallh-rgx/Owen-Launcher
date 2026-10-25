#!/usr/bin/env python3
"""MEGA ENGINE — Erbium→Android ARM64 pattern factory
Core primitives:
  - VersionCtx: UETool wrapper + fast .text numpy arrays + pc-sorted refs
  - anchored_funcs: string → containing functions (R1)
  - function features: size / strings referenced / bl count / constants
  - pattern factory: gen ARM64 byte pattern (wildcards on PC-rel immediates),
    verify exactly-one match in .text
"""
import os, sys, struct, json
import numpy as np
sys.path.insert(0, '/home/z/re-analysis')
from uetool import UETool

SO_DIR = "/tmp/work2/src"
VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10", "21.30", "22.40", "23.50", "30.00"]
DUMP = {"5.41": "v541", "7.40": "v740", "9.41": "v941", "13.40": "v1340", "17.50": "v1750",
        "18.40": "v1840", "19.10": "v1910", "21.30": "v2130", "22.40": "v2240", "23.50": "v2350", "30.00": "v3000"}

class VersionCtx:
    _cache = {}

    def __init__(self, ver):
        self.ver = ver
        so = os.path.join(SO_DIR, ver, "libUnreal.so")
        if not os.path.exists(so):
            so = os.path.join(SO_DIR, ver, "libUE4.so")
        self.tool = UETool(so, f"/home/z/re-analysis/dumps/{DUMP[ver]}")
        t = self.tool
        self.text_va, self.text_off, self.text_size = t.sections[".text"]
        self._refs_by_pc = None
        self._bl_by_pc_arr = None
        # rodata range for string checks
        if ".rodata" in t.sections:
            self.ro_va, self.ro_off, self.ro_size = t.sections[".rodata"]

    @property
    def refs_by_pc(self):
        if self._refs_by_pc is None:
            order = np.argsort(self.tool.refs[:, 1], kind="stable")
            self._refs_by_pc = self.tool.refs[order]     # (tgt, pc) sorted by pc
        return self._refs_by_pc

    @property
    def refs_pc(self):
        return self.refs_by_pc[:, 1]

    @property
    def bl_by_pc_arr(self):
        if self._bl_by_pc_arr is None:
            self._bl_by_pc_arr = self.tool.bl[self.tool.bl[:, 0].argsort(kind="stable")]
        return self._bl_by_pc_arr

    @property
    def bl_pc(self):
        return self.bl_by_pc_arr[:, 0]

    @classmethod
    def get(cls, ver):
        if ver not in cls._cache:
            cls._cache[ver] = cls(ver)
        return cls._cache[ver]

    # ---------- string anchors ----------
    def occurrences(self, s, wide=None):
        """addresses of string literal occurrences (try ascii + utf16)"""
        t = self.tool
        res = []
        encs = [(False,) ] if wide is False else ([(True,), (False,)] if wide is None else [(True,)])
        for (w,) in encs:
            res += [(a, w) for a in t.find_string(s, w)]
            if res:
                break
        return res

    def xrefs_to_string(self, s, wide=None, contain_k=0):
        """all (pc, target, occurrence_addr) where code references the string (or string+k)"""
        t = self.tool
        out = []
        occs = self.occurrences(s, wide)
        slen = len(s)
        for addr, w in occs:
            for k in range(0, contain_k + 1):
                for pc, tgt in t.refs_to(addr + k):
                    out.append((pc, tgt, addr, w, k))
        return out

    def anchored_funcs(self, s, wide=None, contain_k=0):
        """R1: functions containing a reference to string s → {func: [pcs]}"""
        t = self.tool
        fns = {}
        for pc, tgt, addr, w, k in self.xrefs_to_string(s, wide, contain_k):
            f = t.func_of(pc)
            if f is not None:
                fns.setdefault(f, []).append(pc)
        return fns

    # ---------- function features ----------
    def func_end(self, fn):
        return self.tool.func_end(fn)

    def func_size(self, fn):
        return self.func_end(fn) - fn

    def bls_in(self, fn):
        """BL sites inside function (pc sorted)"""
        s = np.searchsorted(self.bl_pc, fn, "left")
        e = np.searchsorted(self.bl_pc, self.func_end(fn), "left")
        return [(int(pc), int(tg)) for pc, tg in self.bl_by_pc_arr[s:e] if fn <= pc < self.func_end(fn)]

    def refs_in(self, fn):
        """ADRP+ADD refs inside function"""
        s = np.searchsorted(self.refs_pc, fn, "left")
        e = np.searchsorted(self.refs_pc, self.func_end(fn), "left")
        return [(int(tg), int(pc)) for tg, pc in self.refs_by_pc[s:e] if fn <= pc < self.func_end(fn)]

    def read_string_at(self, va, minlen=5, maxlen=200):
        """try read ascii / utf16 string at va"""
        t = self.tool
        o = t.va2off(va)
        if o is None:
            return None
        raw = t.elf[o:o + maxlen * 2]
        # ascii
        a = raw.split(b"\0")[0]
        if len(a) >= minlen and all(32 <= c < 127 for c in a):
            return ("A", a.decode())
        # utf16
        w = raw[: maxlen * 2]
        out = []
        for i in range(0, len(w) - 1, 2):
            ch = w[i] | (w[i + 1] << 8)
            if ch == 0:
                break
            if not (32 <= ch < 127):
                out = None
                break
            out.append(chr(ch))
        if out and len(out) >= minlen:
            return ("W", "".join(out))
        return None

    def func_strings(self, fn):
        """strings referenced by function"""
        out = {}
        for tg, pc in self.refs_in(fn):
            s = self.read_string_at(tg)
            if s and len(s[1]) >= 6:
                out.setdefault(s[1], []).append(pc)
        return out

    def func_constants(self, fn, max_ins=600):
        """immediates: MOVZ/MOVK W/X, CMP imm, AND imm (decoded via capstone)"""
        t = self.tool
        n = min(self.func_size(fn) // 4, max_ins)
        o = t.va2off(fn)
        code = t.elf[o:o + n * 4]
        imms = {}
        for ins in t.cs.disasm(code, fn):
            for op in ins.operands if hasattr(ins, 'operands') and ins.operands else []:
                pass
            # capstone without detail: parse op_str for #imm
            for tok in ins.op_str.split(", "):
                if tok.startswith("#0x") or tok.startswith("#"):
                    try:
                        v = int(tok[1:], 0)
                        if v > 0xFF and v < 0x100000000 and v not in (0x1000,):
                            imms.setdefault(v, ins.address)
                    except ValueError:
                        pass
            if ins.address > fn + n * 4:
                break
        return imms

    # ---------- pattern factory ----------
    def _pat_word_mask(self, w):
        """return (word, mask): keep ALL bytes except PC-relative immediates.
        (per-version patterns — maximal distinctiveness within the version)"""
        # ADRP: wildcard imm (page layout)
        if (w & 0x9F000000) == 0x90000000:
            return (w & 0x9F00001F), 0x9F00001F
        # ADR: wildcard imm
        if (w & 0x9F000000) == 0x10000000:
            return (w & 0x9F00001F), 0x9F00001F
        # B / BL (unconditional): wildcard imm26
        if (w & 0x7C000000) == 0x14000000:
            return (w & 0xFC000000), 0xFC000000
        # B.cond: wildcard imm19
        if (w & 0xFF000010) == 0x54000000:
            return (w & 0xFF00000F), 0xFF00000F
        # CBZ/CBNZ: wildcard imm19
        if (w & 0x7E000000) == 0x34000000:
            return (w & 0xFF00001F), 0xFF00001F
        # TBZ/TBNZ: wildcard imm14
        if (w & 0x7E000000) == 0x36000000:
            return (w & 0xFFF8001F), 0xFFF8001F
        # LDR/STR literal: wildcard imm19
        if (w & 0x3B000000) == 0x18000000 or (w & 0x3B000000) == 0x58000000:
            return (w & 0xFF000000), 0xFF000000
        return (w, 0xFFFFFFFF)

    def gen_pattern(self, fn, max_ins=40, min_ins=8):
        """generate pattern starting at function start; smallest unique window.
        returns {pattern, n_ins, hits, ok}"""
        t = self.tool
        n_total = min(self.func_size(fn) // 4, max_ins)
        if n_total < 2:
            return None
        o = t.va2off(fn)
        words = struct.unpack_from(f"<{n_total}I", t.elf, o)
        # ICF-ret check: function is tiny and starts with RET or ends immediately
        if n_total <= 2 and (words[0] & 0xFFFFFC1F) == 0xD65F0000:
            return {"icf_stub": True}
        pat_pairs = [self._pat_word_mask(w) for w in words]
        for n in range(min_ins, n_total + 1, 2):
            pat = self._render(pat_pairs[:n])
            hits = self.scan_pattern(pat, limit=3)
            if len(hits) == 1 and hits[0] == fn:
                return {"pattern": pat, "n_ins": n, "hits": 1, "ok": True}
            if len(hits) == 0:
                break  # should not happen (self-match broken → bug)
        # not unique up to max_ins → try full
        pat = self._render(pat_pairs)
        hits = self.scan_pattern(pat, limit=5)
        return {"pattern": pat, "n_ins": n_total, "hits": len(hits), "ok": len(hits) == 1 and hits[0] == fn,
                "hits_list": [hex(h) for h in hits[:5]]}

    def _render(self, pairs):
        out = []
        for w, m in pairs:
            b = struct.pack("<I", w)
            mb = struct.pack("<I", m)
            for i in range(4):
                out.append(f"{b[i]:02X}" if mb[i] == 0xFF else "??")
        return " ".join(out)

    def scan_pattern(self, pat, limit=10):
        """scan .text for pattern string (hex with ??) → list of VAs.
        Regex-based masked search: C speed, zero memory."""
        import re as _re
        parts = pat.split()
        if not parts:
            return []
        rx_parts = []
        for p in parts:
            if p in ("?", "??"):
                rx_parts.append(b".")
            else:
                rx_parts.append(_re.escape(bytes([int(p, 16)])))
        rx = _re.compile(b"".join(rx_parts), _re.DOTALL)
        m = len(parts)
        elf = self.tool.elf
        t_lo, t_hi = self.text_off, self.text_off + self.text_size
        hits = []
        for mt in rx.finditer(elf, t_lo, t_hi - m + 1):
            hits.append(int(self.text_va + (mt.start() - t_lo)))
            if len(hits) >= limit:
                break
        return hits


def vkey(ver):
    return "v" + ver.replace(".", "_")


def load_impls():
    """native thunk/impl addresses from uefinder offsets.h per version"""
    res = {}
    for ver in VERSIONS:
        p = os.path.join(SO_DIR, ver, "offsets.h")
        if not os.path.exists(p):
            continue
        txt = open(p).read()
        d = {}
        for m in __import__("re").finditer(r'#define UF_(\w+)_THUNK_RVA\s+(0x[0-9A-F]+)', txt):
            d.setdefault(m.group(1), {})["thunk"] = int(m.group(2), 16)
        for m in __import__("re").finditer(r'#define UF_(\w+)_ORIG_RVA\s+(0x[0-9A-F]+)', txt):
            d.setdefault(m.group(1), {})["orig"] = int(m.group(2), 16)
        res[ver] = d
    return res


if __name__ == "__main__":
    # smoke test: known function on 21.30 — UnEquipVehicleWeapon 0x6A1C864
    ctx = VersionCtx.get("21.30")
    r = ctx.gen_pattern(0x6A1C864)
    print("UnEquipVehicleWeapon 21.30:", {k: v for k, v in r.items() if k != 'hits_list'} if r else None)
    if r and r.get("pattern"):
        print("pattern:", r["pattern"][:120], "...")
    # string anchor smoke test
    fns = ctx.anchored_funcs("AFortGameModeAthena::UpdateSafeZonesPhase")
    print("UpdateSafeZonesPhase candidates:", {hex(k): len(v) for k, v in fns.items()})
