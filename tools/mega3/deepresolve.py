#!/usr/bin/env python3
"""DEEP RESOLVE — fix the remaining ~28 hard cells.
Methodologies:
  S4 VTABLE-DELTA: F broken on version X but resolved on 21.30.
     Build ARM vtable runs on 21.30 (rels dump). Find F's run position.
     Find sibling S in the same run resolved on X too → delta = pos(F)-pos(S).
     On X: locate S's run → candidate = run[pos(S@X)+delta]. Majority vote.
  S5 XFP FINGERPRINT: Windows function (xanchors/catalog patterns) → immediates
     → Android functions containing all (op,imm) pairs → size filter.
  S6 CALLER-CRAWL: F is called by a resolved caller (e.g. CallPreReplication
     inside ReplicateActor) → BL targets of caller filtered by fingerprint.
Output: deepcells_<ver>.json
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, load_impls, VERSIONS
from pattern_fix import gen_pattern_windowed
from vtable_x64 import arm_vtable_runs

RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
VF = json.load(open('/home/z/re-analysis/mega/verification_full.json'))
FIX = json.load(open('/home/z/re-analysis/mega/fixcells_all.json'))
XANCHORS = json.load(open('/home/z/re-analysis/mega/xanchors.json'))

VERIDX = {v: i for i, v in enumerate(VERSIONS)}


def verified_addr(finder, ver):
    """best-known address of finder on ver (verified cells > fixcells)"""
    rec = VF['per_version'].get(ver, {}).get(finder)
    if rec and rec.get('p0') == 'UNIQUE' and rec.get('resolved_addr'):
        return int(rec['resolved_addr'], 16)
    f = FIX.get(ver, {}).get(finder)
    if f and f.get('status') == 'OK' and f.get('addr'):
        return int(f['addr'], 16)
    e = RESOLVED.get(finder, {}).get(ver)
    if e and e.get('addr') and not (rec and rec.get('p0') in ('NO_MATCH',) or str(rec.get('p0', '')).startswith('BROKEN')):
        return int(e['addr'], 16)
    return None


def run_index(ctx, runs):
    """func → [(run_idx, pos)]"""
    idx = {}
    for ri, run in enumerate(runs):
        for pos, (slot, f) in enumerate(run):
            idx.setdefault(f, []).append((ri, pos))
    return idx


def try_pattern(ctx, addr, min_sz=0x20):
    if ctx.func_size(addr) < min_sz:
        return None
    r = ctx.gen_pattern(addr, max_ins=48)
    if r and r.get('ok'):
        return r
    r = gen_pattern_windowed(ctx, addr, max_span=120)
    if r and r.get('ok'):
        return r
    return None


def resolve_cell(ver, finder, ctx2130, runs2130, idx2130, known2130):
    """returns {status, addr, pattern, strategy} or {status: UNRESOLVED}"""
    # current verified addresses on `ver` (for sibling lookup)
    target_addr = verified_addr(finder, ver)
    if target_addr:
        return {"status": "already"}
    a2130 = verified_addr(finder, "21.30")
    if not a2130:
        return {"status": "no_2130_anchor"}

    ctx = VersionCtx.get(ver)
    t = ctx.tool

    # ---- S4: vtable delta ----
    fpos = idx2130.get(a2130, [])
    if fpos:
        runs_ver = arm_vtable_runs(ctx)
        idx_ver = run_index(ctx, runs_ver)
        # siblings: functions in the same 21.30 runs that are resolved on ver
        votes = {}
        for ri, fpos_i in fpos:
            run = runs2130[ri]
            for pos, (slot, sf) in enumerate(run):
                if pos == fpos_i:
                    continue
                sib_ver = verified_addr(None, ver)  # placeholder
                # find which finder sf corresponds to (reverse map)
                sib_finder = known2130.get(sf)
                if not sib_finder:
                    continue
                sv = verified_addr(sib_finder, ver)
                if not sv:
                    continue
                delta = fpos_i - pos
                for vri, vpos in idx_ver.get(sv, []):
                    tp = vpos + delta
                    if 0 <= tp < len(runs_ver[vri]):
                        cand = runs_ver[vri][tp][1]
                        votes[cand] = votes.get(cand, 0) + 1
        if votes:
            best, nvotes = max(votes.items(), key=lambda x: x[1])
            if nvotes >= 2 or (nvotes >= 1 and t.is_func(best)):
                sz2130 = ctx2130.func_size(a2130)
                szv = ctx.func_size(best)
                ratio = szv / max(sz2130, 1)
                if 0.3 <= ratio <= 3.5 and t.is_func(best):
                    r = try_pattern(ctx, best)
                    if r:
                        return {"status": "OK", "strategy": f"vtable_delta(votes={nvotes})",
                                "addr": hex(best), "size": hex(szv), "size_ratio": round(ratio, 2),
                                **{k: r[k] for k in ('pattern', 'offset') if k in r}}
                # single vote or bad size — record for fingerprint fallback
                votes_best = (best, nvotes, ratio)
            else:
                votes_best = (best, nvotes, ctx.func_size(best) / max(ctx2130.func_size(a2130), 1))
        else:
            votes_best = None
    else:
        votes_best = None

    # ---- S5: xfp fingerprint from 21.30 (same-version cross-check via structure) ----
    # extract fingerprint of a2130 on 21.30: immediates from disasm
    from xfp import FpMatcher
    imms2130 = {}
    t2130 = ctx2130.tool
    o = t2130.va2off(a2130)
    n = min(ctx2130.func_size(a2130) // 4, 800)
    code = t2130.elf[o:o + n * 4]
    for ins in t2130.cs.disasm(code, a2130):
        w = struct.unpack_from("<I", t2130.elf, t2130.va2off(ins.address))[0]
        # LDR Wt,[Xn,#imm] 0xB9400000 | LDR Xt 0xF9400000 | LDRB 0x39400000 | CMP imm 0x71000000
        if (w & 0xFFC00000) == 0xB9400000:
            imm = ((w >> 10) & 0xFFF) << 2
            if imm >= 0x40: imms2130.setdefault('ldr_w', set()).add(imm)
        elif (w & 0xFFC00000) == 0xF9400000:
            imm = ((w >> 10) & 0xFFF) << 3
            if imm >= 0x40: imms2130.setdefault('ldr_x', set()).add(imm)
        elif (w & 0xFFC00000) == 0x39400000:
            imm = ((w >> 10) & 0xFFF)
            if imm >= 0x40: imms2130.setdefault('ldrb', set()).add(imm)
        elif (w & 0xFF800000) == 0x71000000:
            imm = ((w >> 10) & 0xFFF) << (2 if (w & 0x400000) else 0)
            if imm >= 0x100: imms2130.setdefault('cmp', set()).add(imm)

    specs = []
    for k in ('ldr_w', 'ldr_x', 'ldrb', 'cmp'):
        for imm in sorted(imms2130.get(k, set()))[:8]:
            specs.append((k, imm))
    # keep distinctive ones (≥4 specs → very selective; 2-3 → may be ambiguous)
    if len(specs) >= 2:
        fm = FpMatcher(ctx)
        cands = fm.find_functions(specs[:8], min_hits=max(2, len(specs) - 2))
        # prioritize vtable-delta candidate agreement
        out_c = []
        for f, hits in cands[:20]:
            szv = ctx.func_size(f)
            ratio = szv / max(ctx2130.func_size(a2130), 1)
            if not (0.3 <= ratio <= 3.5):
                continue
            if not t.is_func(f):
                continue
            agree = 2 if votes_best and votes_best[0] == f else 0
            out_c.append((hits + agree, f, ratio))
        if out_c:
            out_c.sort(reverse=True)
            top = out_c[0]
            # require clear winner or vtable agreement
            if top[0] >= 3 or (votes_best and votes_best[0] == top[1] and top[0] >= 2):
                r = try_pattern(ctx, top[1])
                if r:
                    return {"status": "OK", "strategy": f"fp2130(hits={top[0]})",
                            "addr": hex(top[1]), "size": hex(ctx.func_size(top[1])),
                            **{k: r[k] for k in ('pattern', 'offset') if k in r}}
    return {"status": "UNRESOLVED", "vtable_hint": (hex(votes_best[0]), votes_best[1]) if votes_best else None}


def main():
    ver = sys.argv[1] if len(sys.argv) > 1 else None
    # remaining unresolved from fixcells
    todo = []
    for v in VERSIONS:
        for finder, rec in FIX.get(v, {}).items():
            if rec.get('status') != 'OK':
                todo.append((v, finder))
        for b in VF['broken']:
            if b[0] == v and FIX.get(v, {}).get(b[1]) is None:
                todo.append((v, b[1]))
    todo = sorted(set(todo))
    if ver:
        todo = [(v, f) for (v, f) in todo if v == ver]
    print(f"deep-resolving {len(todo)} cells (ver={ver or 'ALL'})", flush=True)

    ctx2130 = VersionCtx.get("21.30")
    runs2130 = arm_vtable_runs(ctx2130)
    idx2130 = run_index(ctx2130, runs2130)
    # map addr→finder on 21.30
    known2130 = {}
    for f in set(RESOLVED.keys()):
        a = verified_addr(f, "21.30")
        if a:
            known2130[a] = f
    print(f"21.30: {len(runs2130)} vtable runs, {len(known2130)} known finders", flush=True)

    out_all = {}
    for v in VERSIONS:
        if ver and v != ver:
            continue
        cells = [f for (vv, f) in todo if vv == v]
        if not cells:
            continue
        ctx = VersionCtx.get(v)
        res = {}
        for finder in cells:
            r = resolve_cell(v, finder, ctx2130, runs2130, idx2130, known2130)
            res[finder] = r
            mark = "✓" if r.get('status') == 'OK' else ("·" if r.get('status') == 'already' else "×")
            print(f"[{v}] {finder:48s} {mark} {r.get('addr','')} {r.get('strategy', r.get('status'))}", flush=True)
        json.dump(res, open(f'/home/z/re-analysis/mega/deepcells_{v}.json', 'w'), indent=1)
        out_all[v] = res
        VersionCtx._cache.pop(v, None)
        gc.collect()

    json.dump(out_all, open('/home/z/re-analysis/mega/deepcells_all.json', 'w'), indent=1)
    n_ok = sum(1 for vv in out_all for f, r in out_all[vv].items() if r.get('status') == 'OK')
    print(f"\ndeep-resolved {n_ok}/{len(todo)}", flush=True)


if __name__ == '__main__':
    main()
