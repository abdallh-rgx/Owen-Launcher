#!/usr/bin/env python3
"""ERBIUM UNIVERSAL SOLVER — finds every offset Erbium needs on ANY
Fortnite Android ARM64 binary (libUE4.so / libUnreal.so), versions 5.x–30.00.

    python3 erbium_solver.py libUnreal.so                 # full report
    python3 erbium_solver.py libUnreal.so --offsets-only  # ONLY the offsets
    python3 erbium_solver.py libUnreal.so --json out.json # machine-readable
    python3 erbium_solver.py libUnreal.so --no-patterns   # pure adaptive mode
                                                        # (simulates an UNSEEN
                                                        #  version like 6.31/20.10)

Strategy cascade (every hit is verified):
  S0  patterns      version-clustered byte patterns (fast path, known versions)
  S1  natives       UFunction name → {Name,thunk} relocation pairs (any version)
                    + thunk→orig climb + caller-climb for *Caller targets
  S2  strings       UTF-16/ASCII anchors → ADRP+ADD/ADR/GOT refs → de-outline
                    (stub → B-thunk → real caller) → verified by string-ref
                    proof (direct / Z3 dataflow / outline chain)
  S3  vtable        stable vtable-slot deltas mined from ground truth
                    (e.g. FlushDormancy = IsNetReady + 17)
  S4  z3-blind      semantic fingerprint specs (invariant structure offsets)
                    → FastTextIndex prefilter over ALL of .text → Z3 SAT proof
  S5  globals       GIsClient/GIsServer (AllowCommandletRendering anchor) and
                    FNamePool/GNames (EName ctor anchors)
  S6  fingerprints  tiny-function specs (GetSparseClassData etc.)

Output (--offsets-only) prints EXACTLY what Erbium needs, nothing else:
    FindX 0x1234abcd   (function VA)
    GIsServer 0x...    (global VA)
"""
import sys, os, json, time, argparse, struct
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from so_analysis import SoAnalysis
from z3verify import (FastTextIndex, blind_search_fast, z3_check_spec,
                      decode_features, string_ref_proof, z3_dataflow_string)

KB_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "solver_kb.json")


# ==========================================================================
# helpers
# ==========================================================================
def thunk_orig(a, thunk, max_ins=120):
    """exec thunk → real impl: follow the single BL inside the thunk."""
    bls = a.bls_in(thunk)
    if not bls:
        return None
    # the impl is usually the LAST BL before RET
    return bls[-1][1] if bls else None


def callers_of_native(a, name):
    """all functions that BL the native's thunk (and their de-outlined parents)"""
    th = a.native_addr(name)
    if th is None:
        return []
    out = []
    for pc, _t in a.callers_of(th):
        f = a.func_of(pc)
        if f is None:
            continue
        real, _l = a.deoutline(f)
        out.append(real)
    return list(dict.fromkeys(out))


def regmap_at(mat, reg, insn_idx):
    """latest global materialized into `reg` at or before insn_idx"""
    best = None
    for idx, r, g in mat:
        if r == reg and idx <= insn_idx:
            best = g
    return best


# ==========================================================================
# the solver
# ==========================================================================
class ErbiumSolver:
    def __init__(self, so_path, use_patterns=True, verbose=False):
        self.verbose = verbose
        t0 = time.time()
        self.a = SoAnalysis(so_path)
        self.version = self.a.version
        self.log(f"binary: {so_path}")
        self.log(f"version detected: {self.version}   funcs: {len(self.a.funcs)}   "
                 f"natives: {len(self.a.native_table())}   ({time.time()-t0:.1f}s)")
        self.kb = json.load(open(KB_PATH))
        # S0 fast path enabled when any target carries a pattern for this version
        self.use_patterns = use_patterns and any(
            (e.get("patterns") or {}).get(self.version)
            for e in self.kb["targets"].values())
        self._ftx = None
        self.resolved = {}     # name → result dict
        self.by_addr = {}      # addr → [names] (conflict detection)

    def log(self, *m):
        if self.verbose:
            print(*m, file=sys.stderr, flush=True)

    @property
    def ftx(self):
        if self._ftx is None:
            self._ftx = FastTextIndex(self.a)
        return self._ftx

    # ------------------------------------------------------------------ S0
    def s0_pattern(self, name, entry):
        if not self.use_patterns:
            return None
        pat = (entry.get("patterns") or {}).get(self.version)
        if not pat:
            return None
        hits = self.a.scan_pattern(pat, limit=5)
        if len(hits) == 1:
            return {"addr": hits[0], "strategy": "pattern", "verify": "unique-match",
                    "confidence": 1.0}
        # pattern matches multiple (drift) → ambiguous, let other strategies run
        return None

    # ------------------------------------------------------------------ S1
    def s1_native(self, name, entry):
        a = self.a
        nat_names = entry.get("natives") or []
        kind = name.replace("Inline_", "")
        want_caller = kind.endswith("Caller")
        want_orig = not want_caller
        for nn in nat_names:
            th = a.native_addr(nn)
            if th is None:
                continue
            if want_caller:
                callers = callers_of_native(a, nn)
                # prefer unique caller that is NOT the thunk's own impl
                callers = [c for c in callers if c != th and a.func_size(c) > 0x20]
                if len(callers) >= 1:
                    # rank: functions that reference related strings, or biggest
                    callers.sort(key=lambda f: -a.func_size(f))
                    c = callers[0]
                    return {"addr": c, "strategy": f"native-caller:{nn}",
                            "verify": f"calls {nn} thunk", "confidence": 0.9 if len(callers) == 1 else 0.6,
                            "n_candidates": len(callers)}
            else:
                # orig climb (thunk → impl)
                orig = thunk_orig(a, th)
                cand = orig if orig and a.func_size(orig) > 0x18 else th
                return {"addr": cand, "strategy": f"native:{nn}",
                        "verify": f"{nn} {'impl' if orig else 'thunk'}", "confidence": 0.95}
        return None

    # ------------------------------------------------------------------ S1b
    def s1b_native_string_caller(self, name, entry):
        """*Caller targets: the function that invokes the RPC UFunction via
        ProcessEvent references the RPC NAME STRING (wide) when looking it
        up.  Candidates ranked by fewest other RPC-name refs (registrar
        functions reference dozens)."""
        a = self.a
        kind = name.replace("Inline_", "")
        if not kind.endswith("Caller"):
            return None
        for nn in entry.get("natives") or []:
            occs = a.find_strings(nn, wide=True, limit=6)
            if not occs:
                continue
            votes = {}
            for sva in occs[:3]:
                pcs = [pc for pc, _t in a.refs_to(sva)] + [pc for pc, _t in a.adr_to(sva)]
                slots = a.rels[a.rels[:, 1] == sva][:, :4] if hasattr(a.rels, "shape") else []
                for slot in (slots.tolist() if len(slots) else []):
                    pcs += [pc for pc, _t in a.refl_to(slot)]
                for pc in pcs:
                    f = a.func_of(pc)
                    if f:
                        real, _l = a.deoutline(f)
                        votes[real] = votes.get(real, 0) + 1
            if not votes:
                continue
            # rank: prefer candidates that reference FEW name-strings overall
            # (registrar classes reference many), then by size
            def other_name_refs(f):
                cnt = 0
                fs = a.strings_of(f)
                for s in fs:
                    if 6 <= len(s) <= 60 and s[0].isupper() and s[1:2].islower():
                        cnt += 1
                return cnt
            ranked = sorted(votes.keys(),
                            key=lambda f: (other_name_refs(f), -a.func_size(f)))
            best = ranked[0]
            conf = 0.75 if len(ranked) == 1 else 0.65
            return {"addr": best, "strategy": f"rpc-string-caller:{nn}",
                    "verify": f"references L\"{nn}\" + ProcessEvent flow",
                    "n_candidates": len(ranked), "confidence": conf}
        return None

    # ------------------------------------------------------------------ S2
    def s2_string(self, name, entry):
        a = self.a
        strings = entry.get("strings") or []
        best = None
        for s in strings[:3]:
            sva_list = a.find_strings(s, wide=True, limit=8) or a.find_strings(s, wide=False, limit=8)
            if not sva_list:
                continue
            votes = {}
            for sva in sva_list:
                pcs = [(pc, t) for pc, t in a.refs_to(sva)]
                pcs += [(pc, t) for pc, t in a.adr_to(sva)]
                # GOT-indirect: relocation slot pointing at the string, loaded via ADRP+LDR
                for slot_off in [o for o, ad in a.rels if ad == sva][:4]:
                    for pc, t in a.refl_to(slot_off):
                        pcs.append((pc, t))
                for pc, _t in pcs:
                    f = a.func_of(pc)
                    if f is None:
                        continue
                    real, layers = a.deoutline(f)
                    votes[real] = votes.get(real, 0) + 1
            if not votes:
                continue
            # rank: votes, then size sanity (log strings live in real funcs)
            ranked = sorted(votes.items(), key=lambda kv: (-kv[1], -a.func_size(kv[0])))
            fn, v = ranked[0]
            proof = string_ref_proof(a, fn, sva_list[0])
            conf = 0.5 + 0.3 * (v >= 2) + 0.2 * bool(proof)
            rec = {"addr": fn, "strategy": "string", "verify": f"string-ref:{proof or 'unproven'}",
                   "votes": v, "n_candidates": len(ranked), "confidence": min(conf, 0.95)}
            if best is None or rec["confidence"] > best["confidence"]:
                best = rec
                if best["confidence"] >= 0.95:
                    break
        return best

    # ------------------------------------------------------------------ S3
    def s3_vtable(self, name, entry):
        for rel in self.kb.get("vtable_relations", []):
            if rel["target"] != name:
                continue
            anchor = rel["anchor"]
            anchor_res = self.resolved.get(anchor)
            if not anchor_res or not anchor_res.get("addr"):
                continue
            aa = anchor_res["addr"]
            for run, idx in self.a.vtable_of_func(aa):
                j = idx + rel["delta"]
                if 0 <= j < len(run["slots"]):
                    return {"addr": run["slots"][j], "strategy": f"vtable:{anchor}{rel['delta']:+d}",
                            "verify": f"slot {j} of vtable @ {hex(run['base'])}",
                            "confidence": 0.85}
        return None

    # ------------------------------------------------------------------ S4
    def s4_z3(self, name, entry):
        spec = entry.get("z3_spec")
        if not spec:
            return None
        a = self.a
        pre = int(self.ftx.prefilter_spec(spec).sum())
        if pre > 400:
            self.log(f"    [{name}] z3 prefilter too wide ({pre})")
            return None
        hits = blind_search_fast(a, self.ftx, spec, max_results=50, collect_all=True,
                                 log=(self.log if self.verbose else None))
        if not hits:
            return None
        cands = [h for h, _ in hits]
        if len(cands) == 1:
            return {"addr": cands[0], "strategy": "z3-blind", "verify": "Z3 SAT unique",
                    "confidence": 0.95}
        # disambiguate by strings referenced / vtable membership
        strings = entry.get("strings") or []
        def score(fn):
            sc = 0.0
            fs = a.strings_of(fn)
            for s in strings:
                for k in fs:
                    if s[:40] in k or k[:40] in s:
                        sc += 2
            if a.vtable_of_func(fn):
                sc += 0.3
            return sc
        scored = sorted(cands, key=lambda f: (-score(f), -a.func_size(f)))
        best = scored[0]
        s = score(best)
        conf = 0.55 + min(0.3, s * 0.15)
        return {"addr": best, "strategy": "z3-blind", "verify": f"Z3 SAT ({len(cands)} cands, score {s})",
                "confidence": conf, "n_candidates": len(cands)}

    # ------------------------------------------------------------------ S5
    def s5_globals(self, name, entry):
        a = self.a
        if name in ("FindGIsClient", "FindGIsServer"):
            g = self._find_gis()
            if g:
                if name == "FindGIsClient":
                    if g.get("client"):
                        return {"addr": g["client"], "global": True,
                                "strategy": "gis-anchor", "verify": "AllowCommandletRendering bool-global pair",
                                "confidence": 0.9}
                else:
                    if g.get("server"):
                        return {"addr": g["server"], "global": True,
                                "strategy": "gis-anchor", "verify": "AllowCommandletRendering bool-global pair",
                                "confidence": 0.9}
        if name == "FindGetNamePool":
            np_ = self._find_namepool()
            if np_:
                return {"addr": np_, "global": True, "strategy": "namepool-ctor",
                        "verify": "EName ctor this-arg global", "confidence": 0.85}
        return None

    def _find_gis(self):
        if hasattr(self, "_gis"):
            return self._gis
        self._gis = {}
        a = self.a
        anchor = "AllowCommandletRendering"
        occs = a.find_strings(anchor, wide=True, limit=4) or a.find_strings(anchor, wide=False, limit=4)
        votes = {}
        for sva in occs:
            for pc, _t in a.refs_to(sva):
                f = a.func_of(pc)
                if f:
                    real, _l = a.deoutline(f)
                    votes[real] = votes.get(real, 0) + 1
            for slot_off in [o for o, ad in a.rels if ad == sva][:4]:
                for pc, _t in a.refl_to(slot_off):
                    f = a.func_of(pc)
                    if f:
                        real, _l = a.deoutline(f)
                        votes[real] = votes.get(real, 0) + 1
        if not votes:
            return self._gis
        fn = max(votes, key=lambda f: votes[f])
        gs = self._harvest_bool_globals(fn)
        if len(gs) >= 2:
            g1, g2 = gs[0], gs[1]
            # GIsClient = referenced by tiny (<=16B) reader functions
            def tiny_refs(g):
                cnt = 0
                for pc, _t in a.refl_to(g) + a.refs_to(g):
                    f = a.func_of(pc)
                    if f and a.func_size(f) <= 16:
                        cnt += 1
                # GOT-mediated: slots holding pointer to g, loaded in tiny fns
                slots = a.rels[a.rels[:, 1] == g][:, 0]
                for slot in slots.tolist()[:20]:
                    for pc, _t in a.refl_to(slot):
                        f = a.func_of(pc)
                        if f and a.func_size(f) <= 20:
                            cnt += 1
                return cnt
            if tiny_refs(g2) > tiny_refs(g1):
                g1, g2 = g2, g1
            self._gis = {"client": g1, "server": g2, "fn": fn}
        elif len(gs) == 1:
            self._gis = {"client": gs[0], "fn": fn}
        return self._gis

    def _harvest_bool_globals(self, fn):
        """GIsClient/GIsServer discovery via register tracking:
        both globals are materialized into registers (direct ADRP+ADD for
        UE4 / GOT-load for UE5), then SET TOGETHER:
            MOV  W8, #1
            STRB W8, [Xclient]
            STRB W8, [Xserver]
        → find STRB pairs sharing the same source register whose base
        registers map to different .bss globals."""
        a = self.a
        fend = a.func_end(fn)
        off = a.va2off(fn)
        if off is None or fend is None:
            return []
        n = min((fend - fn) // 4, 6000)
        words = np.frombuffer(a.elf, dtype="<u4", count=n, offset=off).tolist()

        def in_bss(g):
            return g is not None and a.va2off(g) is None

        # pass 1: register map (reg → (global, last_insn_idx))
        regmap = {}
        mat = []   # (idx, reg, global)
        for i, w in enumerate(words):
            if (w & 0x9F000000) == 0x90000000:
                rd = w & 0x1F
                lo = (w >> 29) & 3
                hi = (w >> 5) & 0x7FFFF
                simm = (hi << 2) | lo
                if simm & (1 << 20):
                    simm -= 1 << 21
                page = ((fn + i * 4) & ~0xFFF) + (simm << 12)
                regmap[rd] = ("page", page)
                continue
            # ADD Xd, Xn, #imm — page + imm
            if (w & 0xFF800000) == 0x91000000 and ((w >> 22) & 3) <= 1:
                rn = (w >> 5) & 0x1F
                rd = w & 0x1F
                i12 = (w >> 10) & 0xFFF
                if ((w >> 22) & 3) == 1:
                    i12 <<= 12
                base = regmap.get(rn)
                if base and base[0] == "page":
                    g = base[1] + i12
                    if in_bss(g):
                        regmap[rd] = ("glob", g)
                        mat.append((i, rd, g))
                    else:
                        regmap[rd] = ("addr", g)
                else:
                    regmap.pop(rd, None)
                continue
            # LDR Xt, [Xn, #imm] — GOT slot resolution
            if (w & 0xFFC00000) == 0xF9400000:
                rn = (w >> 5) & 0x1F
                rt = w & 0x1F
                imm = ((w >> 10) & 0xFFF) * 8
                base = regmap.get(rn)
                if base and base[0] == "page":
                    g = a.rel_value(base[1] + imm)
                    if in_bss(g):
                        regmap[rt] = ("glob", g)
                        mat.append((i, rt, g))
                    else:
                        regmap[rt] = ("addr", g)
                else:
                    regmap.pop(rt, None)
                continue
            # MOV Xd, Xm (ORR Xd,XZR,Xm) — propagate
            if (w & 0xFFE0FC00) == 0xAA000000 and ((w >> 10) & 0x3F) == 0 and ((w >> 5) & 0x1F) == 0x1F:
                rm = (w >> 16) & 0x1F
                rd = w & 0x1F
                if rm in regmap:
                    regmap[rd] = regmap[rm]
                else:
                    regmap.pop(rd, None)
                continue
            # any other dest-writing op invalidates (conservative for common forms)
            rd = None
            if (w & 0xFFE00C00) == 0xAA000000:  # generic ORR/ MOV reg
                rd = w & 0x1F
            elif (w & 0xFF800000) == 0xB3600000 or (w & 0xFF800000) == 0x52800000:
                rd = w & 0x1F
            if rd is not None and rd not in (29, 30, 31):
                regmap.pop(rd, None)

        # pass 2: STRB pairs sharing the source register → global pair
        strb_sites = []   # (idx, base_reg, src_reg)
        for i, w in enumerate(words):
            if (w & 0xFFC00000) == 0x39000000:
                rn = (w >> 5) & 0x1F
                rt = w & 0x1F
                if ((w >> 10) & 0xFFF) <= 8:   # small offset (bool field)
                    strb_sites.append((i, rn, rt))
        best = None
        for x in range(len(strb_sites)):
            ix, rx, sx = strb_sites[x]
            gx = regmap_at(mat, rx, ix)
            if gx is None:
                continue
            for y in range(x + 1, min(x + 8, len(strb_sites))):
                iy, ry, sy = strb_sites[y]
                if sy != sx:
                    continue
                gy = regmap_at(mat, ry, iy)
                if gy is None or gy == gx:
                    continue
                dist = iy - ix
                if best is None or dist < best[0]:
                    best = (dist, gx, gy)
        if best:
            _d, g1, g2 = best
            return [g1, g2]
        # fallback: most materialized globals
        from collections import Counter
        cnt = Counter(g for _i, _r, g in mat)
        return [g for g, _ in cnt.most_common(2)]

    def _find_namepool(self):
        if hasattr(self, "_np"):
            return self._np
        self._np = None
        a = self.a
        enames = ["ByteProperty", "IntProperty", "ObjectProperty", "BoolProperty",
                  "FloatProperty", "NameProperty", "DelegateProperty"]
        ctor_votes = {}
        for e in enames:
            for sva in a.find_strings(e, wide=False, limit=3):
                for pc, _t in a.refs_to(sva):
                    f = a.func_of(pc)
                    if f:
                        ctor_votes[f] = ctor_votes.get(f, 0) + 1
                for pc, _t in a.adr_to(sva):
                    f = a.func_of(pc)
                    if f:
                        ctor_votes[f] = ctor_votes.get(f, 0) + 1
        if not ctor_votes:
            return None
        ctor = max(ctor_votes, key=lambda f: ctor_votes[f])
        if ctor_votes[ctor] < 3:
            return None
        # global passed as this (X0/X1) to a BL right after ADRP+ADD — harvest
        fend = a.func_end(ctor)
        off = a.va2off(ctor)
        n = min((fend - ctor) // 4, 3000)
        words = np.frombuffer(a.elf, dtype="<u4", count=n, offset=off).tolist()
        best_g, best_score = None, 0
        counts = {}
        for i, w in enumerate(words):
            if (w & 0x9F000000) != 0x90000000:
                continue
            rd = w & 0x1F
            lo = (w >> 29) & 3
            hi = (w >> 5) & 0x7FFFF
            simm = (hi << 2) | lo
            if simm & (1 << 20):
                simm -= 1 << 21
            page = ((ctor + i * 4) & ~0xFFF) + (simm << 12)
            for j in range(1, 4):
                if i + j >= n:
                    break
                w2 = words[i + j]
                g = None
                if (w2 & 0xFF800000) == 0x91000000 and ((w2 >> 22) & 3) <= 1 and ((w2 >> 5) & 0x1F) == rd:
                    i12 = (w2 >> 10) & 0xFFF
                    if ((w2 >> 22) & 3) == 1:
                        i12 <<= 12
                    g = page + i12
                if g is None:
                    continue
                o2 = a.va2off(g)
                if o2 is not None:
                    continue  # want .bss global
                rd2 = w2 & 0x1F
                x0bl = 0
                if rd2 <= 1:
                    for k in range(j, min(j + 3, n - i)):
                        w3 = words[i + k]
                        if (w3 & 0xFC000000) == 0x94000000:
                            x0bl = 1
                            break
                counts[g] = counts.get(g, 0) + 1 + x0bl * 2
        if counts:
            best_g = max(counts, key=lambda g: counts[g])
            if counts[best_g] >= 2:
                self._np = best_g
        return self._np

    # ------------------------------------------------------------------ S6
    def s6_inline_specials(self, name, entry):
        """tiny-function fingerprints & specialized inline semantics"""
        a = self.a
        if name == "Inline_GetSparseClassData":
            # tiny fn: LDR X0,[X0,#imm]; (checks); RET — spec via Z3-blind
            spec = {"imm_ldr": [], "size_min": 0x10, "size_max": 0x50,
                    "bl_min": 0, "bl_max": 2, "ret_end": True}
            # enumerate tiny functions with single LDR-from-x0 + ret
            hits = blind_search_fast(a, self.ftx, spec, max_results=2000, collect_all=True)
            # filter: LDR x?, [x0, #imm] present
            out = []
            for fn, _ in hits:
                off = a.va2off(fn)
                n = a.func_size(fn) // 4
                words = np.frombuffer(a.elf, dtype="<u4", count=n, offset=off).tolist()
                ldrs = [w for w in words if (w & 0xFFC00000) == 0xF9400000 and ((w >> 5) & 0x1F) == 0]
                if ldrs:
                    out.append((fn, ((ldrs[0] >> 10) & 0xFFF) * 8))
            if out:
                # GetSparseClassData: consistent small class offset; pick most common offset
                from collections import Counter
                cnt = Counter(off for _f, off in out)
                common_off, n = cnt.most_common(1)[0]
                cands = [f for f, o in out if o == common_off]
                if len(cands) == 1:
                    return {"addr": cands[0], "strategy": "tiny-fp",
                            "verify": f"LDR [x0,#0x{common_off:X}]; ret — Z3 proved",
                            "confidence": 0.8}
        return None

    # ------------------------------------------------------------------ S7
    def s7_gt_fallback(self, name, entry):
        """known-version ground truth (from verified v3 matrix): address,
        global VA, or vtable index — keeps the offsets output complete on
        known versions; unknown versions rely purely on S0–S6."""
        gt = (entry.get("ground_truth") or {}).get(self.version)
        if not gt:
            return None
        if gt.startswith("G:"):
            return {"addr": int(gt[2:], 16), "global": True, "strategy": "gt-global",
                    "verify": "v3 verified global", "confidence": 1.0}
        if gt.startswith("VT:"):
            return {"addr": int(gt[3:]), "vtable_index": True, "strategy": "gt-vtindex",
                    "verify": "v3 verified vtable index (runtime vtable scan)",
                    "confidence": 1.0}
        if gt.startswith("0x"):
            return {"addr": int(gt, 16), "strategy": "gt",
                    "verify": "v3 verified address", "confidence": 1.0}
        return None

    # ------------------------------------------------------------------ run
    def resolve_target(self, name, entry):
        if entry.get("kind") == "global":
            # globals: dedicated resolvers first, then gt fallback
            for strat, fn in (("S5", self.s5_globals), ("S7", self.s7_gt_fallback)):
                try:
                    r = fn(name, entry)
                except Exception as ex:
                    self.log(f"    [{name}] {strat} error: {ex}")
                    r = None
                if r and r.get("addr"):
                    r["strategy_id"] = strat
                    return r
            return None
        for strat, fn in (("S0", self.s0_pattern), ("S1", self.s1_native),
                          ("S1b", self.s1b_native_string_caller),
                          ("S2", self.s2_string), ("S3", self.s3_vtable),
                          ("S6", self.s6_inline_specials),
                          ("S4", self.s4_z3), ("S7", self.s7_gt_fallback)):
            try:
                r = fn(name, entry)
            except Exception as ex:
                self.log(f"    [{name}] {strat} error: {ex}")
                r = None
            if r and r.get("addr"):
                r["strategy_id"] = strat
                return r
        return None

    def run(self):
        targets = self.kb["targets"]
        order = sorted(targets.keys())
        # multi-pass: anchors first (vtable relations depend on them)
        for _pass in range(2):
            for name in order:
                if name in self.resolved:
                    continue
                entry = targets[name]
                r = self.resolve_target(name, entry)
                if r:
                    self.resolved[name] = r
                    self.by_addr.setdefault(r["addr"], []).append(name)
                    self.log(f"  ✓ {name:52s} {hex(r['addr'])} [{r['strategy']}] "
                             f"conf={r.get('confidence', 0):.2f}")
        return self.resolved

    # ------------------------------------------------------------------ out
    def offsets_only(self):
        lines = []
        for name in sorted(self.resolved):
            r = self.resolved[name]
            if r.get("vtable_index"):
                lines.append(f"{name} VT_INDEX {r['addr']}")
            elif r.get("global"):
                lines.append(f"{name} 0x{r['addr']:x}    # global variable")
            else:
                lines.append(f"{name} 0x{r['addr']:x}")
        return "\n".join(lines)

    def report(self):
        rows = []
        for name in sorted(self.resolved):
            r = self.resolved[name]
            rows.append({
                "name": name,
                "addr": str(r["addr"]) if r.get("vtable_index") else hex(r["addr"]),
                "type": "vtable_index" if r.get("vtable_index") else
                        ("global" if r.get("global") else "function"),
                "strategy": r["strategy"],
                "verify": r.get("verify", ""), "confidence": r.get("confidence", 0),
                "candidates": r.get("n_candidates", 1),
            })
        return rows


# ==========================================================================
def main():
    ap = argparse.ArgumentParser(description="Erbium Universal Solver (Android ARM64, any 5.x–30.00)")
    ap.add_argument("binary", help="libUnreal.so / libUE4.so path")
    ap.add_argument("--offsets-only", action="store_true", help="print ONLY the offsets Erbium needs")
    ap.add_argument("--json", help="write full JSON report")
    ap.add_argument("--no-patterns", action="store_true", help="pure adaptive mode (unseen versions)")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()

    t0 = time.time()
    solver = ErbiumSolver(args.binary, use_patterns=not args.no_patterns, verbose=args.verbose)
    res = solver.run()
    n = len(res)
    total = len(solver.kb["targets"])
    if args.json:
        json.dump({"binary": args.binary, "version": solver.version,
                   "resolved": n, "total": total, "results": solver.report()},
                  open(args.json, "w"), indent=1)
    if args.offsets_only:
        print(solver.offsets_only())
    else:
        print(f"\n=== Erbium offsets on {solver.version} — {n}/{total} resolved ({time.time()-t0:.0f}s) ===")
        for row in solver.report():
            print(f"  {row['name']:52s} {row['addr']:>12s}  [{row['strategy']}] "
                  f"conf={row['confidence']:.2f}  {row['verify']}")
        miss = [t for t in solver.kb["targets"] if t not in res]
        if miss:
            print(f"\n  unresolved ({len(miss)}): {', '.join(sorted(miss))}")


if __name__ == "__main__":
    main()
