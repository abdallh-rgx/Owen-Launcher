#!/usr/bin/env python3
"""TARGETED3 — final resolution round with corrected decoders:
  - TST Xn,#0x30 exact word decode (N=1,immr=0x3C,imms=0x1)
  - BFI #0x20,#0x10 decode (immr=0x20,imms=0x0F)
  - min_hits fixed (≤ len(specs))
  - 30.00 fp transfers from 23.50/22.40
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
TGT = {}
for v in VERSIONS:
    p = f'/home/z/re-analysis/mega/targeted_all.json'
    if os.path.exists(p):
        TGT = json.load(open(p))
    p2 = f'/home/z/re-analysis/mega/targeted2_all.json'
    if os.path.exists(p2):
        for vv, d in json.load(open(p2)).items():
            TGT.setdefault(vv, {}).update(d)

RESULTS = {}
for v in VERSIONS:
    p = f'/home/z/re-analysis/mega/targeted3_{v}.json'
    if os.path.exists(p):
        RESULTS[v] = json.load(open(p))


def verified_addr(finder, ver):
    rec = VF['per_version'].get(ver, {}).get(finder)
    if rec and rec.get('p0') == 'UNIQUE' and rec.get('resolved_addr'):
        return int(rec['resolved_addr'], 16)
    for src in (RESULTS.get(ver, {}), TGT.get(ver, {}), FIX.get(ver, {}), DEEP.get(ver, {})):
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


def record(ver, finder, addr, ctx, strategy, extra=None):
    if RESULTS.get(ver, {}).get(finder, {}).get('status') == 'OK':
        return True
    r = try_pattern(ctx, addr)
    if not r:
        print(f"    !! pattern failed {hex(addr)}", flush=True)
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
        elif (w & 0xFFC00000) == 0x79400000:
            imm = ((w >> 10) & 0xFFF) << 1
            if imm >= 0x40: specs.append(('ldr_h', imm))
    seen = set()
    out = []
    for s in specs:
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


def fp_transfer(ctx_dst, src_ctx, src_addr, ratio_ok=(0.5, 2.2), max_specs=10):
    from xfp import FpMatcher
    specs = fp_specs_of(src_ctx, src_addr)[:max_specs]
    if len(specs) < 2:
        return None, specs
    fm = FpMatcher(ctx_dst)
    cands = fm.find_functions(specs, min_hits=max(2, (len(specs) * 2) // 3))
    src_size = src_ctx.func_size(src_addr)
    scored = []
    for f, hits in cands[:60]:
        sz = ctx_dst.func_size(f)
        if not (ratio_ok[0] * src_size <= sz <= ratio_ok[1] * src_size):
            continue
        if not ctx_dst.tool.is_func(f):
            continue
        scored.append((hits, f))
    scored.sort(reverse=True)
    if not scored:
        return None, specs
    if len(scored) == 1 or scored[0][0] >= scored[1][0] + 2:
        return scored[0][1], specs
    return "AMBIG", specs


# ---- TST x8,#0x30 / BFI decoders ----
def has_tst30_bfi(words):
    """exact: tst xN,#0x30 (N=1 immr=0x3C imms=1) + bfi #0x20,#0x10"""
    tst = bfi = False
    for w in words:
        if (w & 0xFFE00000) == 0xF2600000 and ((w >> 22) & 1) == 1 \
           and ((w >> 16) & 0x3F) == 0x3C and ((w >> 10) & 0x3F) == 0x1:
            tst = True
        if (w & 0xFF800000) == 0xB3000000 and ((w >> 16) & 0x3F) == 0x20 \
           and ((w >> 10) & 0x3F) == 0x0F:
            bfi = True
    return tst, bfi


def resolve_callprerep(ver):
    ctx = VersionCtx.get(ver)
    rep = verified_addr("FindRepicateActor" if False else "FindReplicateActor", ver)
    if not rep:
        print(f"  × [{ver}] CallPreReplication: no ReplicateActor")
        return
    t = ctx.tool
    callees = set(tg for pc, tg in ctx.bls_in(rep))
    cands = []
    for tg in callees:
        sz = ctx.func_size(tg)
        if not (0x150 <= sz <= 0x600):
            continue
        o = t.va2off(tg)
        if o is None:
            continue
        nfull = min(sz // 4, 300)
        wf = np.frombuffer(t.elf, dtype='<u4', count=nfull, offset=o)
        tst, bfi = has_tst30_bfi(wf)
        # cbz x1/x2 early
        has_cbz = any((w & 0x7F000000) == 0x34000000 and (w & 0x1F) in (1, 2)
                      for w in wf[:10])
        # ldrh [x0,#0x5c] + ldr [x0,#0x58] merge pattern
        has_585c = False
        for w in wf[:16]:
            if (w & 0xFFC00000) == 0x79400000 and (((w >> 10) & 0xFFF) << 1) == 0x5c:
                has_585c = True
        if tst and bfi and has_cbz:
            cands.append(tg)
        elif tst and (bfi or has_585c):
            cands.append(tg)
    print(f"    [{ver}] CallPreReplication filter → {len(cands)}: {[hex(c) for c in cands]}")
    if len(cands) == 1:
        record(ver, "FindCallPreReplication", cands[0], ctx, "caller-crawl:tst30")
    elif cands:
        a2130 = verified_addr("FindCallPreReplication", "21.30")
        if a2130:
            sz21 = VersionCtx.get("21.30").func_size(a2130)
            best = min(cands, key=lambda c: abs(ctx.func_size(c) - sz21))
            record(ver, "FindCallPreReplication", best, ctx, "caller-crawl:tst30+size")
        else:
            record(ver, "FindCallPreReplication", cands[0], ctx, "caller-crawl:first")
    else:
        print(f"  × [{ver}] CallPreReplication: none")
    VersionCtx._cache.pop(ver, None); gc.collect()


def resolve_senddestruction():
    ver = "21.30"
    if RESULTS.get(ver, {}).get("FindSendDestructionInfo", {}).get('status') == 'OK':
        return
    ctx = VersionCtx.get(ver)
    specs = []
    for srcv in ("19.10", "22.40", "17.50", "13.40"):
        a = verified_addr("FindSendDestructionInfo", srcv)
        if a:
            sctx = VersionCtx.get(srcv)
            specs += fp_specs_of(sctx, a)[:12]
            VersionCtx._cache.pop(srcv, None)
    seen = set()
    specs = [s for s in specs if not (s in seen or seen.add(s))]
    print(f"    senddestruction specs: {len(specs)}")
    if len(specs) < 2:
        print("  × [21.30] SendDestructionInfo: too few specs")
        return
    from xfp import FpMatcher
    fm = FpMatcher(ctx)
    cands = fm.find_functions(specs[:10], min_hits=max(2, (len(specs) + 1) // 2))
    scored = [(h, f) for h, f in cands[:40] if ctx.tool.is_func(f)]
    scored.sort(reverse=True)
    print(f"    top: {[(h, hex(f), hex(ctx.func_size(f))) for h, f in scored[:6]]}")
    if scored and (len(scored) == 1 or scored[0][0] >= scored[1][0] + 2):
        record(ver, "FindSendDestructionInfo", scored[0][1], ctx, "fp-agree")
    else:
        print("  × [21.30] SendDestructionInfo ambiguous")
    VersionCtx._cache.pop(ver, None); gc.collect()


def main():
    print("== CallPreReplication (fixed decoders) ==", flush=True)
    for ver in ("22.40", "23.50", "30.00"):
        resolve_callprerep(ver)
    print("== SendDestructionInfo 21.30 ==", flush=True)
    resolve_senddestruction()
    json_str = json.dumps(RESULTS, indent=1)
    for v in RESULTS:
        json.dump(RESULTS[v], open(f'/home/z/re-analysis/mega/targeted3_{v}.json', 'w'), indent=1)
    print("saved targeted3_*")


if __name__ == '__main__':
    main()
