#!/usr/bin/env python3
"""TARGETED2 — corrected per-case resolution:
  1. CallPreReplication (22.40/23.50/30.00): caller-crawl from ReplicateActor with
     CORRECT fingerprint (size 0x250-0x500, cbz x1, tst #0x30 64-bit, bfi #0x20)
  2. OnRep_ZiplineState (17.50/18.40/19.10): pick among 'ZiplineState'-anchored
     candidates via fingerprint agreement from 13.40 + 21.30
  3. SendDestructionInfo (21.30): fingerprint agreement from 19.10 + 22.40
  4. remaining 30.00 fp-transfers (CanAfford/ApplyChar/HandlePostSafeZone/...)
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, VERSIONS
from pattern_fix import gen_pattern_windowed

RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
VF = json.load(open('/home/z/re-analysis/mega/verification_full.json'))
FIX = json.load(open('/home/z/re-analysis/mega/fixcells_all.json'))
DEEP = {}
for v in VERSIONS:
    p = f'/home/z/re-analysis/mega/deepcells_{v}.json'
    if os.path.exists(p):
        DEEP[v] = json.load(open(p))
try:
    TGT1 = json.load(open('/home/z/re-analysis/mega/targeted_all.json'))
except Exception:
    TGT1 = {}


def verified_addr(finder, ver):
    rec = VF['per_version'].get(ver, {}).get(finder)
    if rec and rec.get('p0') == 'UNIQUE' and rec.get('resolved_addr'):
        return int(rec['resolved_addr'], 16)
    for src in (TGT1.get(ver, {}), FIX.get(ver, {}), DEEP.get(ver, {})):
        e = src.get(finder)
        if e and e.get('status') == 'OK' and e.get('addr'):
            return int(e['addr'], 16)
    e = RESOLVED.get(finder, {}).get(ver)
    if e and e.get('addr'):
        rec2 = VF['per_version'].get(ver, {}).get(finder, {})
        if rec2.get('p0') != 'NO_MATCH' and not str(rec2.get('p0', '')).startswith('BROKEN'):
            return int(e['addr'], 16)
    return None


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


RESULTS = {}
try:
    RESULTS = json.load(open('/home/z/re-analysis/mega/targeted2_all.json'))
except Exception:
    pass


def record(ver, finder, addr, ctx, strategy, extra=None):
    r = try_pattern(ctx, addr)
    if not r:
        print(f"    !! pattern failed {hex(addr)}")
        return False
    rec = {"status": "OK", "strategy": strategy, "addr": hex(addr),
           "size": hex(ctx.func_size(addr)),
           **{k: r[k] for k in ('pattern', 'offset') if k in r}}
    if extra:
        rec.update(extra)
    RESULTS.setdefault(ver, {})[finder] = rec
    print(f"  ✓ [{ver}] {finder} = {hex(addr)} ({strategy})", flush=True)
    return True


def fp_specs_of(ctx, fn):
    t = ctx.tool
    o = t.va2off(fn)
    n = min(ctx.func_size(fn) // 4, 800)
    code = t.elf[o:o + n * 4]
    specs = []
    for ins in t.cs.disasm(code, fn):
        w = struct.unpack_from("<I", t.elf, t.va2off(ins.address))[0]
        if (w & 0xFFC00000) == 0xB9400000:
            imm = ((w >> 10) & 0xFFF) << 2
            if imm >= 0x40: specs.append(('ldr_w', imm))
        elif (w & 0xFFC00000) == 0xF9400000:
            imm = ((w >> 10) & 0xFFF) << 3
            if imm >= 0x40: specs.append(('ldr_x', imm))
        elif (w & 0xFFC00000) == 0x39400000:
            imm = ((w >> 10) & 0xFFF)
            if imm >= 0x40: specs.append(('ldrb', imm))
        elif (w & 0xFFC00000) == 0x79400000:  # LDRH
            imm = ((w >> 10) & 0xFFF) << 1
            if imm >= 0x40: specs.append(('ldr_h', imm))
    seen = set()
    out = []
    for s in specs:
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


def fp_score(ctx, fn, specs):
    """how many specs present in fn"""
    t = ctx.tool
    o = t.va2off(fn)
    n = min(ctx.func_size(fn) // 4, 900)
    code = t.elf[o:o + n * 4]
    words = set()
    for ins in t.cs.disasm(code, fn):
        w = struct.unpack_from("<I", t.elf, t.va2off(ins.address))[0]
        if (w & 0xFFC00000) == 0xB9400000:
            words.add(('ldr_w', ((w >> 10) & 0xFFF) << 2))
        elif (w & 0xFFC00000) == 0xF9400000:
            words.add(('ldr_x', ((w >> 10) & 0xFFF) << 3))
        elif (w & 0xFFC00000) == 0x39400000:
            words.add(('ldrb', ((w >> 10) & 0xFFF)))
        elif (w & 0xFFC00000) == 0x79400000:
            words.add(('ldr_h', ((w >> 10) & 0xFFF) << 1))
    return sum(1 for s in specs if s in words)


# ============ 1. CallPreReplication ============
def resolve_callprerep(ver):
    ctx = VersionCtx.get(ver)
    rep = verified_addr("FindReplicateActor", ver)
    if not rep:
        print(f"  × [{ver}] CallPreReplication: no ReplicateActor anchor")
        return
    t = ctx.tool
    callees = set(tg for pc, tg in ctx.bls_in(rep))
    cands = []
    for tg in callees:
        sz = ctx.func_size(tg)
        if not (0x250 <= sz <= 0x560):
            continue
        o = t.va2off(tg)
        if o is None:
            continue
        n = min(sz // 4, 24)
        words = np.frombuffer(t.elf, dtype='<u4', count=n, offset=o)
        # cbz x1/x2 in first 10
        has_cbz = any((w & 0x7F000000) == 0x34000000 and ((w) & 0x1F) in (1, 2)
                      for w in words[:10])
        # mrs tpidr_el0 in first 10 (stack cookie)
        has_mrs = any((w & 0xFFFFFFE0) == 0xD53BD040 for w in words[:10])
        if not (has_cbz and has_mrs):
            continue
        # full-body scan for: tst x8,#0x30 (ANDS 64 imm) or bfi #0x20,#0x10
        nfull = min(sz // 4, 250)
        wf = np.frombuffer(t.elf, dtype='<u4', count=nfull, offset=o)
        has_tst = any((w & 0xFF800000) == 0xF2000000 and ((w >> 10) & 0xFFF) == 0x18C
                      for w in wf)  # N=1,immr... approximate
        has_bfi = any((w & 0xFF800000) == 0xB3000000 and ((w >> 10) & 0x3F) == 0x2F - 0x10 + 0x10
                      for w in wf)
        # simpler: decode via disasm text
        txt = t.disasm(tg, min(sz // 4, 120))
        ok_tst = 'tst' in txt and '#0x30' in txt
        ok_bfi = 'bfi' in txt
        if ok_tst and ok_bfi:
            cands.append(tg)
    if len(cands) == 1:
        record(ver, "FindCallPreReplication", cands[0], ctx, "caller-crawl:tst30+bfi")
    elif len(cands) > 1:
        # compare with 21.30 size
        a2130 = verified_addr("FindCallPreReplication", "21.30")
        if a2130:
            sz21 = VersionCtx.get("21.30").func_size(a2130)
            best = min(cands, key=lambda c: abs(ctx.func_size(c) - sz21))
            record(ver, "FindCallPreReplication", best, ctx, "caller-crawl:tst30+bfi+size")
        else:
            record(ver, "FindCallPreReplication", cands[0], ctx, "caller-crawl:first")
    else:
        print(f"  × [{ver}] CallPreReplication: {len(cands)} candidates after filter")
    VersionCtx._cache.pop(ver, None); gc.collect()


# ============ 2. OnRep_ZiplineState ============
def resolve_zipline(ver):
    ctx = VersionCtx.get(ver)
    cands = list(ctx.anchored_funcs("ZiplineState").keys())
    if not cands:
        print(f"  × [{ver}] OnRep_ZiplineState: no ZiplineState anchors")
        return
    # fingerprints from resolved neighbors
    specs = []
    for srcv in ("13.40", "21.30", "22.40"):
        a = verified_addr("FindOnRep_ZiplineState", srcv)
        if a:
            sctx = VersionCtx.get(srcv)
            specs += fp_specs_of(sctx, a)[:12]
            VersionCtx._cache.pop(srcv, None)
    seen = set()
    specs = [s for s in specs if not (s in seen or seen.add(s))]
    scored = [(fp_score(ctx, c, specs), c) for c in cands]
    scored.sort(reverse=True)
    print(f"    zipline candidates: {[(hex(c), s, hex(ctx.func_size(c))) for s, c in scored]}")
    if scored and scored[0][0] >= max(3, len(specs) // 3):
        record(ver, "FindOnRep_ZiplineState", scored[0][1], ctx, "ZiplineState-anchored+fp")
    else:
        print(f"  × [{ver}] OnRep_ZiplineState: fp weak")
    VersionCtx._cache.pop(ver, None); gc.collect()


# ============ 3. SendDestructionInfo 21.30 ============
def resolve_senddestruction():
    ver = "21.30"
    ctx = VersionCtx.get(ver)
    specs = []
    for srcv in ("19.10", "22.40", "17.50"):
        a = verified_addr("FindSendDestructionInfo", srcv)
        if a:
            sctx = VersionCtx.get(srcv)
            specs += fp_specs_of(sctx, a)[:14]
            VersionCtx._cache.pop(srcv, None)
    if not specs:
        print("  × [21.30] SendDestructionInfo: no anchors")
        return
    seen = set()
    specs = [s for s in specs if not (s in seen or seen.add(s))]
    # candidate pool: callees of CloseActorChannel + SendDestructionInfo callers?
    # broad: all functions 0x80-0x2000 with ≥60% specs — use xfp matcher
    from xfp import FpMatcher
    fm = FpMatcher(ctx)
    cands = fm.find_functions(specs[:10], min_hits=max(3, len(specs) // 2))
    scored = []
    for f, hits in cands[:30]:
        if not ctx.tool.is_func(f):
            continue
        scored.append((hits, f))
    scored.sort(reverse=True)
    print(f"    senddestruction top: {[(h, hex(f), hex(ctx.func_size(f))) for h, f in scored[:6]]}")
    if scored and (len(scored) == 1 or scored[0][0] >= scored[1][0] + 2):
        record(ver, "FindSendDestructionInfo", scored[0][1], ctx, "fp-agree:19.10+22.40")
    else:
        print("  × [21.30] SendDestructionInfo ambiguous")
    VersionCtx._cache.pop(ver, None); gc.collect()


def main():
    print("== 1. CallPreReplication ==", flush=True)
    for ver in ("22.40", "23.50", "30.00"):
        resolve_callprerep(ver)
    print("== 2. OnRep_ZiplineState ==", flush=True)
    for ver in ("17.50", "18.40", "19.10"):
        resolve_zipline(ver)
    print("== 3. SendDestructionInfo ==", flush=True)
    resolve_senddestruction()
    json.dump(RESULTS, open('/home/z/re-analysis/mega/targeted2_all.json', 'w'), indent=1)
    n = sum(1 for v in RESULTS for f, r in RESULTS[v].items() if r.get('status') == 'OK')
    print(f"\ntargeted2 resolved: {n}")


if __name__ == '__main__':
    main()
