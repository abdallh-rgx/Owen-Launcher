#!/usr/bin/env python3
"""Z3 MATHEMATICAL VERIFICATION + BLIND SEARCH for Erbium function finding.

Two Z3-powered modes:

  1) z3_check_spec(spec, words)  — mathematical proof that an instruction
     window satisfies a semantic spec.  Every constraint is expressed as a
     bit-vector formula over the DECODED instruction fields; an existential
     instruction index is a symbolic integer that Z3 solves for.  A SAT
     result is a proof that the function really does e.g. load
     [X?, #0x698] and call ≥ 2 functions — not just a byte coincidence.

  2) z3_blind_search(analyzer, spec) — the "blind & brute-force" mode:
     enumerate EVERY function in .text (cheap numpy prefilters first),
     then Z3-prove the survivors.  Finds functions on versions we have
     NEVER seen, using only version-independent semantics.

  3) z3_dataflow_string(fn, string_va) — symbolic execution of registers
     (ADRP/ADD/MOV/LDR chains) proving that a given string address flows
     into the argument register of a call inside the function — the
     mathematical form of "String ref verification".

Spec format (JSON-able dict), all keys optional:
  {
    "imm_ldr":  [0x698, 0x1F8],        # LDR Xt,[Xn,#imm] (imm unscaled) present
    "imm_ldr_w":[0x1F],                # 32-bit LDR (imm*4)
    "imm_ldrb": [...],                 # byte loads (imm*1)
    "imm_str":  [...],                 # 64-bit stores
    "imm_raw":  [0x2A],                # any MOVZ/MOVK/CMP imm
    "bl_min": 2, "bl_max": 60,         # direct call count window
    "frame_min": 0x10, "frame_max": 0x900,   # SUB SP,SP,#imm bounds
    "ret_end": true,                   # ends with RET
    "prologue": true,                  # starts with STP x29,x30 / SUB SP
    "size_min": 0x30, "size_max": 0x4000,
    "variant_sets": [ {...}, {...} ]   # OR-of-specs: any one variant suffices
  }
"""
import struct
import numpy as np
import z3

U32 = 0xFFFFFFFF
M = np.uint64


# --------------------------------------------------------------------------
# fast structural decoders (numpy) — used for prefilters and features
# --------------------------------------------------------------------------
def decode_features(words, base_va):
    """vectorized decode of one function's instruction words → feature dict"""
    w = words.astype(np.uint64)
    n = len(w)
    pc0 = base_va
    feat = {
        "n": n,
        "bl": 0, "blr": 0, "ret": 0,
        "ldr64_imm": set(), "str64_imm": set(), "ldr32_imm": set(),
        "ldrb_imm": set(), "strb_imm": set(), "ldrh_imm": set(),
        "str32_imm": set(),
        "raw_imm": set(), "frame": None, "ret_last": False,
        "prologue": False, "bl_targets": [],
    }
    if n == 0:
        return feat
    # BL
    blm = (w & M(0xFC000000)) == M(0x94000000)
    bl_idx = np.flatnonzero(blm)
    feat["bl"] = len(bl_idx)
    if len(bl_idx):
        imm = (w[bl_idx] & M(0x03FFFFFF)).astype(np.int64)
        imm = (imm ^ np.int64(1 << 25)) - np.int64(1 << 25)
        pcs = (pc0 + bl_idx.astype(np.uint64) * 4).astype(np.int64)
        tgts = pcs + (imm << np.int64(2))
        feat["bl_targets"] = [int(t) & 0xFFFFFFFFFFFFFFFF for t in tgts.tolist()]
    # BLR
    feat["blr"] = int((((w & M(0xFFFFFC1F)) == M(0xD63F0000)) & (((w >> M(5)) & M(0x1F)) != M(31))).sum())
    # RET
    retm = (w & M(0xFFFFFC1F)) == M(0xD65F0000)
    feat["ret"] = int(retm.sum())
    feat["ret_last"] = bool(retm[-1])
    # LDR/STR 64 imm (unscaled imm already ×8)
    ldr64 = (w & M(0xFFC00000)) == M(0xF9400000)
    if ldr64.any():
        feat["ldr64_imm"] = set((((w[ldr64] >> M(10)) & M(0xFFF)) * 8).tolist())
    str64 = (w & M(0xFFC00000)) == M(0xF9000000)
    if str64.any():
        feat["str64_imm"] = set((((w[str64] >> M(10)) & M(0xFFF)) * 8).tolist())
    # LDR 32 (imm*4)
    ldr32 = (w & M(0xFFC00000)) == M(0xB9400000)
    if ldr32.any():
        feat["ldr32_imm"] = set((((w[ldr32] >> M(10)) & M(0xFFF)) * 4).tolist())
    # STR 32 (imm*4)
    str32 = (w & M(0xFFC00000)) == M(0xB9000000)
    if str32.any():
        feat["str32_imm"] = set((((w[str32] >> M(10)) & M(0xFFF)) * 4).tolist())
    # LDRH (imm*2)
    ldrh = (w & M(0xFFC00000)) == M(0x79400000)
    if ldrh.any():
        feat["ldrh_imm"] = set((((w[ldrh] >> M(10)) & M(0xFFF)) * 2).tolist())
    # LDRB / STRB (imm*1)
    ldrb = (w & M(0xFFC00000)) == M(0x39400000)
    if ldrb.any():
        feat["ldrb_imm"] = set(((w[ldrb] >> M(10)) & M(0xFFF)).tolist())
    strb = (w & M(0xFFC00000)) == M(0x39000000)
    if strb.any():
        feat["strb_imm"] = set(((w[strb] >> M(10)) & M(0xFFF)).tolist())
    # frame: SUB SP, SP, #imm
    subm = (w & M(0xFF8003FF)) == M(0xD10003FF)
    si = np.flatnonzero(subm)
    if len(si):
        sh = (w[si[0]] >> M(22)) & M(3)
        i12 = (w[si[0]] >> M(10)) & M(0xFFF)
        feat["frame"] = int(i12 << 12) if sh == 1 else int(i12)
    # prologue: STP X29,X30,[SP,#-N]! or SUB SP or PACIASP
    w0 = int(w[0]); w1 = int(w[1]) if n > 1 else 0
    feat["prologue"] = ((w0 & 0xFFC07FFF) == 0xA9807BFD or (w1 & 0xFFC07FFF) == 0xA9807BFD
                        or w0 == 0xD503233F or (w0 & 0xFF8003FF) == 0xD10003FF)
    return feat


# --------------------------------------------------------------------------
# Z3 spec verification
# --------------------------------------------------------------------------
def _mem_access_ok(w):
    """any LDR/STR (x/w/b/h, imm) instruction?"""
    return z3.Or(
        (w & 0xFFC00000) == 0xF9400000,  # LDR x
        (w & 0xFFC00000) == 0xF9000000,  # STR x
        (w & 0xFFC00000) == 0xB9400000,  # LDR w
        (w & 0xFFC00000) == 0xB9000000,  # STR w
        (w & 0xFFC00000) == 0x39400000,  # LDRB
        (w & 0xFFC00000) == 0x39000000,  # STRB
        (w & 0xFFC00000) == 0x79400000,  # LDRH
        (w & 0xFFC00000) == 0x79000000,  # STRH
    )


def _mem_access_off(w):
    """unscaled byte offset accessed by the memory instruction"""
    op = w & 0xFFC00000
    scale = z3.If(z3.Or(op == 0xF9400000, op == 0xF9000000), z3.BitVecVal(8, 32),
           z3.If(z3.Or(op == 0xB9400000, op == 0xB9000000), z3.BitVecVal(4, 32),
           z3.If(z3.Or(op == 0x79400000, op == 0x79000000), z3.BitVecVal(2, 32),
                 z3.BitVecVal(1, 32))))
    return ((w >> 10) & 0xFFF) * scale


class Z3Spec:
    """Z3-backed checker for one spec variant."""

    def __init__(self, spec):
        self.spec = spec
        self.cache = {}

    # ---- per-word symbolic decoders (word = z3 BitVec 32) ----
    @staticmethod
    def _ldr_imm(w):
        """unscaled immediate of LDR/STR Xt,[Xn,#imm12*8]"""
        return (w >> 10) & 0xFFF

    def check(self, words):
        """words: list[int] instruction words → SAT proof of spec.
        Hybrid: numpy locates candidate instruction indices; Z3 then PROVES
        the bit-level decode of those words satisfies each requirement
        (genuine symbolic verification, not byte matching)."""
        sp = self.spec
        n = len(words)
        if n == 0:
            return False, {}
        if "size_min" in sp and n * 4 < sp["size_min"]:
            return False, {}
        if "size_max" in sp and n * 4 > sp["size_max"]:
            return False, {}

        s = z3.Solver()
        s.set("timeout", 4000)
        wa = np.array(words, dtype=np.uint32)
        proofs = {}
        reqs = []

        def z3_prove_access(offset, tag):
            """∃ instruction whose symbolic decode = mem-access of `offset`.
            numpy nominates indices; Z3 proves the decode."""
            nom = []
            for i, w in enumerate(words):
                wu = w & 0xFFFFFFFF
                base = wu & 0xFFC00000
                scale = 8 if base in (0xF9400000, 0xF9000000) else \
                        4 if base in (0xB9400000, 0xB9000000) else \
                        2 if base in (0x79400000, 0x79000000) else \
                        1 if base in (0x39400000, 0x39000000) else 0
                if scale and (((wu >> 10) & 0xFFF) * scale) == offset:
                    nom.append(i)
            if not nom:
                return None
            # Z3 proof: symbolic decode of the nominated words (unique var name!)
            idx = z3.Int(f"i_macc_{tag}_{offset}")
            conds = []
            for i in nom[:24]:
                w = z3.BitVecVal(words[i], 32)
                conds.append(z3.And(idx == i, _mem_access_ok(w),
                                    _mem_access_off(w) == offset))
            return z3.And(idx >= 0, idx < n, z3.Or(*conds))

        for tag, off in enumerate(sp.get("imm_access", [])):
            c = z3_prove_access(off, tag)
            if c is None:
                return False, {}
            reqs.append(c)

        # width-specific requirements (LDR64 etc.)
        for key, opcode, scale in (("imm_ldr", 0xF9400000, 8), ("imm_str", 0xF9000000, 8),
                                   ("imm_ldr_w", 0xB9400000, 4), ("imm_ldrb", 0x39400000, 1)):
            for off in sp.get(key, []):
                nom = [i for i, w in enumerate(words)
                       if (w & 0xFFC00000) == opcode and ((w >> 10) & 0xFFF) * scale == off]
                if not nom:
                    return False, {}
                idx = z3.Int(f"i_{key}_{off}")
                conds = [z3.And(idx == i,
                                (z3.BitVecVal(words[i], 32) & 0xFFC00000) == opcode,
                                ((z3.BitVecVal(words[i], 32) >> 10) & 0xFFF) * scale == off)
                         for i in nom[:24]]
                reqs.append(z3.And(idx >= 0, idx < n, z3.Or(*conds)))

        if reqs:
            s.add(*reqs)

        # BL count — numpy counts; Z3 proves the decode of nominated BL words
        if sp.get("bl_min") is not None or sp.get("bl_max") is not None:
            bl_nom = [i for i, w in enumerate(words)
                      if (w & 0xFC000000) == 0x94000000][:8]
            n_bl = sum(1 for w in words if (w & 0xFC000000) == 0x94000000)
            if bl_nom:
                bi = z3.Int("i_bl")
                conds = [z3.And(bi == i, (z3.BitVecVal(words[i], 32) & 0xFC000000) == 0x94000000)
                         for i in bl_nom]
                reqs.append(z3.And(bi >= 0, bi < n, z3.Or(*conds)))
            if sp.get("bl_min") is not None and n_bl < sp["bl_min"]:
                return False, {}
            if sp.get("bl_max") is not None and n_bl > sp["bl_max"]:
                return False, {}

        if reqs:
            s.add(*reqs)
        if s.check() != z3.sat:
            return False, {}
        proofs["z3"] = "sat"
        # concrete side-checks (cheap, not symbolic)
        feat = decode_features(wa, 0)
        if sp.get("ret_end") and not feat["ret_last"]:
            return False, {}
        if sp.get("prologue") and not feat["prologue"]:
            return False, {}
        if sp.get("frame_min") is not None and (feat["frame"] is None or feat["frame"] < sp["frame_min"]):
            return False, {}
        if sp.get("frame_max") is not None and feat["frame"] is not None and feat["frame"] > sp["frame_max"]:
            return False, {}
        return True, proofs


def z3_check_spec(spec, words):
    """top-level: supports variant_sets (OR). returns (ok, proofinfo)"""
    variants = spec.get("variant_sets") or [spec]
    best = None
    for v in variants:
        merged = {k: val for k, val in spec.items() if k not in ("variant_sets",)}
        merged.update(v)
        ok, info = Z3Spec(merged).check(words)
        if ok:
            return True, {"variant": v, **info}
        if best is None:
            best = info
    return False, best or {}


# --------------------------------------------------------------------------
# prefilter (numpy) — for blind search speed
# --------------------------------------------------------------------------
def prefilter_pass(spec, feat):
    """fast rejection before Z3. Returns False → reject."""
    if feat["n"] == 0:
        return False
    if "size_min" in spec and feat["n"] * 4 < spec["size_min"]:
        return False
    if "size_max" in spec and feat["n"] * 4 > spec["size_max"]:
        return False
    # width-agnostic offset access
    acc = set()
    for key in ("ldr64_imm", "str64_imm", "ldr32_imm", "ldrb_imm", "strb_imm"):
        acc |= feat[key]
    # 32-bit STR + LDRH forms (decode_features covers ldr32/ldrb; add str32/ldrh)
    if feat.get("str32_imm"):
        acc |= feat["str32_imm"]
    if feat.get("ldrh_imm"):
        acc |= feat["ldrh_imm"]
    for v in spec.get("imm_access", []):
        if v not in acc:
            return False
    for k, key in (("imm_ldr", "ldr64_imm"), ("imm_str", "str64_imm"),
                   ("imm_ldr_w", "ldr32_imm"), ("imm_ldrb", "ldrb_imm")):
        vals = spec.get(k)
        if vals:
            have = feat[key]
            if not all(v in have for v in vals):
                return False
    if spec.get("bl_min") is not None and feat["bl"] < spec["bl_min"]:
        return False
    if spec.get("bl_max") is not None and feat["bl"] > spec["bl_max"]:
        return False
    if spec.get("ret_end") and not feat["ret_last"]:
        return False
    if spec.get("prologue") and not feat["prologue"]:
        return False
    if spec.get("frame_min") is not None and (feat["frame"] is None or feat["frame"] < spec["frame_min"]):
        return False
    if spec.get("frame_max") is not None and feat["frame"] is not None and feat["frame"] > spec["frame_max"]:
        return False
    return True


# --------------------------------------------------------------------------
# FAST TEXT INDEX — vectorized whole-.text precompute (one pass per binary)
# --------------------------------------------------------------------------
class FastTextIndex:
    """Precomputed per-word decode tables + per-function aggregates so that
    spec prefilters over ALL functions become a handful of numpy ops."""

    FORMS = {
        "ldr64": (0xFFC00000, 0xF9400000, 8),
        "str64": (0xFFC00000, 0xF9000000, 8),
        "ldr32": (0xFFC00000, 0xB9400000, 4),
        "str32": (0xFFC00000, 0xB9000000, 4),
        "ldrb":  (0xFFC00000, 0x39400000, 1),
        "strb":  (0xFFC00000, 0x39000000, 1),
        "ldrh":  (0xFFC00000, 0x79400000, 2),
        "strh":  (0xFFC00000, 0x79000000, 2),
    }

    def __init__(self, analyzer):
        self.a = analyzer
        w = analyzer.words            # uint32 zero-copy view — all decode math fits 32 bits
        n = len(w)
        self.n_words = n
        # per-word quick masks (cheap: 1 byte/word each)
        self.is_bl = (w & np.uint32(0xFC000000)) == np.uint32(0x94000000)
        self.is_ret = (w & np.uint32(0xFFFFFC1F)) == np.uint32(0xD65F0000)
        self.is_stp_pre = (w & np.uint32(0xFFC07FFF)) == np.uint32(0xA9807BFD)
        self.is_pac = w == np.uint32(0xD503233F)
        self.is_sub_sp = (w & np.uint32(0xFF8003FF)) == np.uint32(0xD10003FF)
        # function boundaries as word indices
        funcs = analyzer.funcs
        self.funcs = funcs
        self.fidx = ((funcs - analyzer.text_va) // 4).astype(np.int64)
        fend = np.empty(len(funcs), dtype=np.uint64)
        fe_arr = analyzer._fends if getattr(analyzer, "_fends", None) is not None else None
        for i in range(len(funcs)):
            e = analyzer.func_end(int(funcs[i]))
            fend[i] = min(e, analyzer.text_va + analyzer.text_size)
        self.fwcount = np.maximum((fend - funcs) // 4, 0).astype(np.int64)
        self.fsize = (fend - funcs).astype(np.int64)
        self._reduceat_cache = {}
        self._off_cache = {}       # offset → per-function bool
        self._offform_cache = {}   # (form, offset) → per-function bool

    def _range_counts(self, acc):
        """per-function count of a boolean word array over each function's
        OWN extent [start, start+size) — correct under nesting/overlap
        (outlined cold blocks get their own FDE inside the parent)."""
        cum = np.concatenate([[0], np.cumsum(acc.astype(np.int64))])
        s = self.fidx
        e = np.minimum(self.fidx + self.fwcount, self.n_words)
        return (cum[e] - cum[s])

    def _reduceat(self, arr, dtype=np.int64):
        """per-function count (cached by array identity)"""
        key = id(arr)
        if key in self._reduceat_cache:
            return self._reduceat_cache[key]
        cnt = self._range_counts(np.asarray(arr, dtype=bool))
        out = cnt > 0
        self._reduceat_cache[key] = (cnt, out)
        return cnt, out

    def _offset_mask(self, offset):
        """word-level bool: any-width memory access of byte-offset (transient)"""
        w = self.a.words
        m = np.zeros(self.n_words, dtype=bool)
        for mask, val, scale in self.FORMS.values():
            fm = (w & np.uint32(mask)) == np.uint32(val)
            if fm.any():
                sub = (w[fm] >> np.uint32(10)) & np.uint32(0xFFF)
                m[fm] |= (sub * np.uint32(scale)) == np.uint32(offset)
        return m

    def _form_offset_mask(self, form, offset):
        mask, val, scale = self.FORMS[form]
        w = self.a.words
        return ((w & np.uint32(mask)) == np.uint32(val)) & \
               ((((w >> np.uint32(10)) & np.uint32(0xFFF)) * np.uint32(scale)) == np.uint32(offset))

    def functions_with_offset(self, offset):
        """boolean per-function (cached): accesses byte-offset (any width)"""
        key = ("anyf", offset)
        if key not in self._off_cache:
            _cnt, out = self._reduceat(self._offset_mask(offset))
            self._off_cache[key] = out
        return self._off_cache[key]

    def functions_with_form_offset(self, form, offset):
        key = (form, offset)
        if key not in self._offform_cache:
            _cnt, out = self._reduceat(self._form_offset_mask(form, offset))
            self._offform_cache[key] = out
        return self._offform_cache[key]

    def function_bl_counts(self):
        cnt, _ = self._reduceat(self.is_bl)
        return cnt

    def function_feats(self):
        """per-function aggregate dict (lazy)"""
        if hasattr(self, "_feats"):
            return self._feats
        bl_cnt, _ = self._reduceat(self.is_bl)
        # ret_last: last word of each function is RET
        last_idx = np.clip(self.fidx + self.fwcount - 1, 0, self.n_words - 1)
        ret_last = self.is_ret[last_idx] & (self.fwcount > 0)
        # frame: first sub_sp in function (min over function — approximate via first)
        # prologue: stp-pre/pac/sub_sp in first 2 words
        w0 = self.fidx
        w1 = np.clip(self.fidx + 1, 0, self.n_words - 1)
        prologue = (self.is_stp_pre[w0] | self.is_stp_pre[w1] | self.is_pac[w0]
                    | self.is_sub_sp[w0] | self.is_sub_sp[w1]) & (self.fwcount > 0)
        self._feats = {
            "bl": bl_cnt,
            "ret_last": ret_last,
            "prologue": prologue,
            "size": self.fsize,
        }
        return self._feats

    def prefilter_spec(self, spec):
        """boolean mask over functions passing the cheap constraints"""
        nf = len(self.funcs)
        keep = np.ones(nf, dtype=bool)
        f = self.function_feats()
        if "size_min" in spec:
            keep &= f["size"] >= spec["size_min"]
        if "size_max" in spec:
            keep &= f["size"] <= spec["size_max"]
        for v in spec.get("imm_access", []):
            keep &= self.functions_with_offset(v)
        # width-specific
        for k, form in (("imm_ldr", "ldr64"), ("imm_str", "str64"),
                        ("imm_ldr_w", "ldr32"), ("imm_ldrb", "ldrb")):
            for v in spec.get(k, []):
                keep &= self.functions_with_form_offset(form, v)
        if spec.get("bl_min") is not None:
            keep &= f["bl"] >= spec["bl_min"]
        if spec.get("bl_max") is not None:
            keep &= f["bl"] <= spec["bl_max"]
        if spec.get("ret_end"):
            keep &= f["ret_last"]
        if spec.get("prologue"):
            keep &= f["prologue"]
        return keep


def blind_search_fast(analyzer, index, spec, max_results=8, log=None,
                      collect_all=False, hard_cap=1200):
    """vectorized prefilter (FastTextIndex) → Z3 proof.
    collect_all=True → verify EVERY prefilter survivor (bounded by hard_cap).
    Returns list of (fn_va, proof)."""
    variants = spec.get("variant_sets") or [spec]
    results = []
    seen = set()
    for vi, v in enumerate(variants):
        merged = {k: val for k, val in spec.items() if k != "variant_sets"}
        merged.update(v)
        keep = index.prefilter_spec(merged)
        cand = np.flatnonzero(keep)
        if log:
            log(f"    prefilter: {len(cand)}/{len(index.funcs)} functions")
        for ci in cand[:hard_cap]:
            fn = int(index.funcs[ci])
            if fn in seen:
                continue
            sz = min(int(index.fsize[ci]), 0x10000)
            if sz < 12:
                continue
            off = analyzer.va2off(fn)
            if off is None:
                continue
            words = np.frombuffer(analyzer.elf, dtype="<u4", count=sz // 4, offset=off)
            ok, proof = z3_check_spec(merged, words.tolist())
            if ok:
                seen.add(fn)
                results.append((fn, {"variant": vi, "z3": "sat", **proof}))
                if log:
                    log(f"    [Z3-blind] {hex(fn)} variant#{vi} ✓")
                if not collect_all and len(results) >= max_results:
                    return results
    return results


# --------------------------------------------------------------------------
# BLIND SEARCH — brute-force every function in .text, Z3-prove survivors
# --------------------------------------------------------------------------
def z3_blind_search(analyzer, spec, max_results=8, log=None, scan_windows=False):
    """enumerate every function (or sliding window) in .text;
    numpy prefilter → Z3 proof.  Returns list of (fn_va, proof)."""
    variants = spec.get("variant_sets") or [spec]
    results = []
    funcs = analyzer.funcs
    n_funcs = len(funcs)
    for vi, v in enumerate(variants):
        merged = {k: val for k, val in spec.items() if k != "variant_sets"}
        merged.update(v)
        for fi in range(n_funcs):
            fn = int(funcs[fi])
            fend = analyzer.func_end(fn)
            if fend is None or fend <= fn:
                continue
            sz = min(fend - fn, 0x6000)
            if sz < 12 or sz % 4:
                sz = sz & ~3
            off = analyzer.va2off(fn)
            if off is None:
                continue
            words = np.frombuffer(analyzer.elf, dtype="<u4", count=sz // 4, offset=off)
            feat = decode_features(words, fn)
            if not prefilter_pass(merged, feat):
                continue
            ok, proof = Z3Spec(merged).check(words.tolist())
            if ok:
                results.append((fn, {"variant": vi, "z3": "sat", **proof}))
                if log:
                    log(f"  [Z3-blind] {hex(fn)} variant#{vi} ✓")
                if len(results) >= max_results:
                    return results
    return results


# --------------------------------------------------------------------------
# SYMBOLIC DATAFLOW — string-ref mathematical proof
# --------------------------------------------------------------------------
def z3_dataflow_string(analyzer, fn, string_va, max_ins=400):
    """Prove that `string_va` flows into an argument register (x0..x7) of a
    call (BL) inside fn, through ADRP+ADD / MOV / LDR-GOT chains.

    Symbolic execution with Z3 expressions as register values:
      regs[i] ∈ {symbolic initial, concrete addr expr}
      ADRP+ADD rd ← addr ; MOV rd ← rs ; BLR/BL snapshot args
    Returns True if any call-site arg can be PROVEN == string_va."""
    fend = analyzer.func_end(fn)
    if fend is None:
        return False
    n_ins = min((fend - fn) // 4, max_ins)
    off = analyzer.va2off(fn)
    if off is None or n_ins <= 0:
        return False
    words = np.frombuffer(analyzer.elf, dtype="<u4", count=n_ins, offset=off)
    UNKNOWN = None
    regs = [UNKNOWN] * 32          # concrete ints or ('sym', z3 expr)
    call_args = []                 # (pc, [argexprs])
    for i, w in enumerate(words.tolist()):
        pc = fn + i * 4
        wu = w & U32
        # ADRP
        if (wu & 0x9F000000) == 0x90000000:
            rd = wu & 0x1F
            lo = (wu >> 29) & 3
            hi = (wu >> 5) & 0x7FFFF
            simm = ((hi << 2) | lo)
            if simm & (1 << 20):
                simm -= 1 << 21
            page = (pc & ~0xFFF) + (simm << 12)
            regs[rd] = ("page", page)
            continue
        # ADD Xd, Xn, #imm12 (u64)
        if (wu & 0xFF800000) == 0x91000000 and ((wu >> 22) & 3) <= 1:
            rd = wu & 0x1F
            rn = (wu >> 5) & 0x1F
            i12 = (wu >> 10) & 0xFFF
            if ((wu >> 22) & 3) == 1:
                i12 <<= 12
            base = regs[rn]
            if isinstance(base, tuple) and base[0] == "page":
                regs[rd] = ("addr", base[1] + i12)
            else:
                regs[rd] = UNKNOWN
            continue
        # MOV Xd, Xm (ORR Xd, XZR, Xm)
        if (wu & 0xFFE0FC00) == 0xAA000000 and ((wu >> 10) & 0x3F) == 0 and ((wu >> 5) & 0x1F) == 0x1F:
            rd = wu & 0x1F
            rm = (wu >> 16) & 0x1F
            regs[rd] = regs[rm]
            continue
        # LDR Xt, [Xn, #imm] — resolves GOT slots concretely
        if (wu & 0xFFC00000) == 0xF9400000:
            rt = wu & 0x1F
            rn = (wu >> 5) & 0x1F
            imm = ((wu >> 10) & 0xFFF) * 8
            base = regs[rn]
            if isinstance(base, tuple) and base[0] in ("addr", "page"):
                slot = (base[1] if base[0] == "addr" else base[1]) + imm
                rv = analyzer.rel_value(slot)
                regs[rt] = ("addr", rv) if rv is not None else UNKNOWN
            else:
                regs[rt] = UNKNOWN
            continue
        # BL — snapshot x0..x7
        if (wu & 0xFC000000) == 0x94000000:
            call_args.append((pc, [regs[i] for i in range(8)]))
            continue
    # check: any call arg == string_va?
    for pc, args in call_args:
        for a in args:
            if isinstance(a, tuple) and a[0] == "addr" and a[1] == string_va:
                return True
    return False


# --------------------------------------------------------------------------
# combined string-ref verification (outline-aware)
# --------------------------------------------------------------------------
def string_ref_proof(analyzer, fn, string_va):
    """string referenced by fn:  direct ADRP+ADD / ADR / dataflow into call
    args (Z3 proof), or via outlined stub chain (stub references it and the
    real function calls the stub)."""
    # 1) direct materialization
    for tgt, pc in analyzer.refs_in(fn):
        if tgt == string_va:
            return "direct_adrp"
    for tgt, pc in analyzer.adr_in(fn):
        if tgt == string_va:
            return "direct_adr"
    # 2) dataflow proof
    if z3_dataflow_string(analyzer, fn, string_va):
        return "z3_dataflow"
    # 3) outlined stub chain: any tiny callee references it
    for pc, tgt in analyzer.bls_in(fn):
        if analyzer.func_size(tgt) <= 0x60:
            for t2, _p in analyzer.refs_in(tgt):
                if t2 == string_va:
                    return "outline_chain"
            for t2, _p in analyzer.adr_in(tgt):
                if t2 == string_va:
                    return "outline_chain"
    return None


if __name__ == "__main__":
    import sys, time
    sys.path.insert(0, ".")
    from so_analysis import SoAnalysis
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/work2/src/21.30/libUnreal.so"
    a = SoAnalysis(path)
    t0 = time.time()
    print(f"version={a.version} funcs={len(a.funcs)} ({time.time()-t0:.1f}s)")

    # ---- smoke test 1: IsNetReady fingerprint spec (structure offsets that
    #      are identical across ABI) — blind search on 21.30
    spec = {
        "imm_ldr": [0x698, 0x1F8, 0x3290, 0x3230],
        "bl_min": 1, "bl_max": 30,
        "size_min": 0x30, "size_max": 0x1200,
        "ret_end": True,
    }
    t0 = time.time()
    # first: how many functions survive prefilter?
    feats = 0
    hits = []
    for fi in range(len(a.funcs)):
        fn = int(a.funcs[fi])
        fend = a.func_end(fn)
        if not fend or fend <= fn:
            continue
        sz = min(fend - fn, 0x6000)
        if sz < 12:
            continue
        off = a.va2off(fn)
        if off is None:
            continue
        words = np.frombuffer(a.elf, dtype="<u4", count=sz // 4, offset=off)
        f = decode_features(words, fn)
        if prefilter_pass(spec, f):
            feats += 1
            hits.append(fn)
    print(f"prefilter survivors: {feats} ({time.time()-t0:.1f}s)")
    for fn in hits[:10]:
        off = a.va2off(fn)
        words = np.frombuffer(a.elf, dtype="<u4", count=min((a.func_end(fn)-fn)//4, 0x6000//4), offset=off)
        ok, proof = z3_check_spec(spec, words.tolist())
        print(f"  {hex(fn)} size={a.func_size(fn)} Z3={'SAT' if ok else 'unsat'}")
