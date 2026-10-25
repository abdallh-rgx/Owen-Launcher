#!/usr/bin/env python3
"""XFP — cross-architecture immediate-fingerprint matcher.
Windows function disasm → (op_class, imm) field-offset pairs → Android functions
containing the same pairs (struct layouts are ABI-independent).
Usage: as module. resolve(ver, win_func, {'ldr_w': [0x698, 0x1f8], 'ldrb': [0x3290], 'add_x': [0x3230]})"""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np

def win_field_imms(win_func, maxbytes=0x1000):
    """disassemble Windows function → {op_class: [imms]} for LDR/STR/CMP field offsets."""
    from x64tool import X64
    from capstone import Cs, CS_ARCH_X86, CS_MODE_64
    x = X64.get()
    cs = Cs(CS_ARCH_X86, CS_MODE_64)
    o = x.va2off(win_func)
    size = min(x.func_size(win_func), maxbytes)
    code = x.full[o:o + size]
    out = {"ldr_d": [], "ldrb": [], "str_d": [], "strb": [], "cmp_d": [], "cmp_b": [],
           "ldr_q": [], "str_q": [], "add": [], "imm": []}
    import re
    for ins in cs.disasm(code, win_func):
        m = ins.op_str
        if ins.mnemonic in ("mov", "cmp", "movzx", "test", "add", "sub", "movsxd"):
            # [reg + 0xNNN]
            mm = re.search(r"\[r[a-z0-9]+ \+ (0x[0-9a-f]+|\d+)\]", m)
            if mm:
                imm = int(mm.group(1), 0)
                if imm >= 0x40:
                    key = {"mov": "ldr_d" if "ptr" in m or "[" in m else "imm",
                           "cmp": "cmp_b" if "byte" in m else ("cmp_d" if "dword" in m else "cmp_q"),
                           "test": "cmp_b" if "byte" in m else "cmp_q",
                           "movzx": "ldrb" if "byte" in m else "ldr_d",
                           "movsxd": "ldr_d"}.get(ins.mnemonic, "imm")
                    if key != "imm":
                        out[key].append(imm)
            else:
                mm2 = re.match(r"^(0x[0-9a-f]+|\d+)$", m.split(",")[-1].strip())
                if mm2 and ins.mnemonic in ("cmp", "add", "sub", "test"):
                    v = int(mm2.group(1), 0)
                    if v > 0xFF:
                        out["imm"].append(v)
    return out

class FpMatcher:
    def __init__(self, ctx):
        self.ctx = ctx
        t = ctx.tool
        va, off, size = ctx.text_va, ctx.text_off, ctx.text_size
        self.words = np.frombuffer(t.elf, dtype="<u4", count=size // 4, offset=off)
        self.base_va = va

    def scan_class_imm(self, op_class, imm):
        """PCs (VAs) of matching instructions."""
        w = self.words
        if op_class in ("ldr_w", "str_w"):  # LDR/STR Wt,[Xn,#imm*4]
            opc = 0xB9400000 if op_class == "ldr_w" else 0xB9000000
            mask = np.uint32(0xFFC00000)
            target = np.uint32(opc)
            m = (w & mask) == target
            imm12 = np.uint32((imm >> 2) << 10)
            m &= (w & np.uint32(0xFFC00000) | np.uint32(0)) == np.uint32(0)  # noop
            # precise: ((w>>10)&0xFFF)==imm>>2 and opcode bits
            m2 = (((w >> 10) & 0xFFF) == (imm >> 2)) & ((w & 0xFFC00000) == opc)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class in ("ldr_x", "str_x"):  # LDR/STR Xt,[Xn,#imm*8]
            opc = 0xF9400000 if op_class == "ldr_x" else 0xF9000000
            m2 = (((w >> 10) & 0xFFF) == (imm >> 3)) & ((w & 0xFFC00000) == opc)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class in ("ldrb", "strb"):  # LDRB/STRB Wt,[Xn,#imm]
            opc = 0x39400000 if op_class == "ldrb" else 0x39000000
            m2 = (((w >> 10) & 0xFFF) == imm) & ((w & 0xFFC00000) == opc)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "add_x":  # ADD Xd,Xn,#imm
            m2 = (((w >> 10) & 0xFFF) == imm) & ((w & 0xFF800000) == 0x91000000)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "cmp_imm":  # CMP Wn/Xn,#imm
            # SUBS Xd, Xn, #imm (32-bit: 0x71000000 range)
            m2 = (((w >> 10) & 0xFFF) == imm) & ((w & 0xFF800000) == 0x71000000)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "ldr_h":  # LDR Ht/St (16-bit): imm*2
            opc = 0x79400000
            m2 = (((w >> 10) & 0xFFF) == (imm >> 1)) & ((w & 0xFFC00000) == opc)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "movz_w":  # MOVZ Wd, #imm16
            m2 = (((w >> 5) & 0xFFFF) == imm) & ((w & 0xFFE00000) == 0x52800000)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "movz_x":  # MOVZ Xd, #imm16
            m2 = (((w >> 5) & 0xFFFF) == imm) & ((w & 0xFFE00000) == 0xD2800000)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        if op_class == "movk_w":  # MOVK Wd, #imm16, LSL #16 (upper)
            m2 = (((w >> 5) & 0xFFFF) == imm) & ((w & 0xFFE00000) == 0x72A00000)
            return self.base_va + np.flatnonzero(m2).astype(np.uint64) * 4
        return np.array([], dtype=np.uint64)

    def find_functions(self, specs, min_hits=None, max_cands=40):
        """specs: list of (op_class, imm). Returns [(func, hits)] sorted by hits desc.
        A function matches if ALL specs hit inside it (strict) — then rank by extra info."""
        t = self.ctx.tool
        func_of = t.func_of
        per_func = {}
        for op_class, imm in specs:
            pcs = self.scan_class_imm(op_class, imm)
            fs = set()
            for pc in pcs.tolist():
                f = func_of(pc)
                if f:
                    fs.add(f)
            for f in fs:
                per_func[f] = per_func.get(f, 0) + 1
        need = min_hits if min_hits else len(specs)
        out = [(f, h) for f, h in per_func.items() if h >= need]
        out.sort(key=lambda x: -x[1])
        return out[:max_cands]

def resolve(ctx, specs, size_range, min_hits=None, exclude=None):
    """full resolve: fingerprint + pattern. specs from win_field_imms mapping."""
    fm = FpMatcher(ctx)
    cands = fm.find_functions(specs, min_hits=min_hits)
    results = []
    for f, hits in cands:
        if exclude and f in exclude:
            continue
        sz = ctx.func_size(f)
        if not (size_range[0] <= sz <= size_range[1]):
            continue
        if ctx.tool.func_of(f) != f:
            continue
        r = ctx.gen_pattern(f)
        if r and r.get('ok'):
            results.append({"status": "OK", "addr": hex(f), "size": hex(sz),
                            "pattern": r['pattern'], "fp_hits": hits, "strategy": "imm_fingerprint"})
        if len(results) >= 3:
            break
    if len(results) == 1:
        return results[0]
    if not results:
        return {"status": "fp_no_match"}
    results[0]["ambiguous"] = len(results)
    return results[0]

# mapping windows op_class → android scan ops
def specs_from_win(wf):
    """translate win_field_imms output to android (op_class, imm) specs"""
    specs = []
    for imm in set(wf.get("ldr_d", [])):
        specs.append(("ldr_w", imm))
    for imm in set(wf.get("ldrb", [])):
        specs.append(("ldrb", imm))
    for imm in set(wf.get("cmp_b", [])):
        specs.append(("ldrb", imm))  # byte cmp reads → ldrb on arm
    for imm in set(wf.get("cmp_d", [])):
        specs.append(("ldr_w", imm))
    for imm in set(wf.get("ldr_q", [])):
        specs.append(("ldr_x", imm))
    return specs
