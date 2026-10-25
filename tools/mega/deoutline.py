#!/usr/bin/env python3
"""DE-OUTLINE pass: Android 13.40+ machine-outlined log sequences into tiny stubs.
Real function = the CALLER of the stub. Fix wave1/wave3 entries whose anchored
function is tiny (<0x60), then regenerate their patterns."""
import sys, os, json, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx, VERSIONS
from pattern_fix import gen_pattern_windowed

STUB_MAX = 0x60

def is_b_thunk(ctx, fn):
    """size ≤ 8 and first instruction = unconditional B"""
    sz = ctx.func_size(fn)
    if sz > 8 or sz < 4:
        return False
    o = ctx.tool.va2off(fn)
    w = struct.unpack_from("<I", ctx.tool.elf, o)[0]
    return (w & 0xFC000000) == 0x14000000


def deoutline(ctx, fn):
    """follow stub/thunk layers up to real function. returns (real, layers)"""
    cur = fn
    layers = 0
    for _ in range(4):
        sz = ctx.func_size(cur)
        if sz >= STUB_MAX:
            break
        # B-thunk: follow its callers
        if is_b_thunk(ctx, cur) or sz < STUB_MAX:
            callers = set()
            for pc, tgt in ctx.tool.callers_of(cur):
                f = ctx.tool.func_of(pc)
                if f is not None and f != cur:
                    callers.add(f)
            if not callers:
                break
            if len(callers) == 1:
                cur = next(iter(callers))
                layers += 1
                continue
            # multiple callers: largest (real functions are big)
            nxt = max(callers, key=lambda f: ctx.func_size(f))
            if ctx.func_size(nxt) <= sz:
                break
            cur = nxt
            layers += 1
    return cur, layers


def process_wave1():
    path = '/home/z/re-analysis/mega/wave1.json'
    data = json.load(open(path))
    ctx_cache = {}
    n_fix = 0
    for fname, vers in data.items():
        for ver, r in vers.items():
            if r.get('status') != 'ok' or not r.get('addr'):
                continue
            if ver not in ctx_cache:
                ctx_cache[ver] = VersionCtx(ver)
            ctx = ctx_cache[ver]
            a = int(r['addr'], 16)
            sz = r.get('size') or ctx.func_size(a)
            r['size'] = sz
            if sz >= STUB_MAX:
                continue
            real, nc = deoutline(ctx, a)
            if real != a:
                old = r['addr']
                r['addr'] = hex(real)
                r['outlined_stub'] = old
                r['size'] = ctx.func_size(real)
                r['stub_callers'] = nc
                # regenerate pattern
                p = ctx.gen_pattern(real)
                if p and not p.get('icf_stub'):
                    r['pattern'] = p['pattern']
                    r['unique'] = p['ok']
                    r['pat_offset'] = 0
                    if not p['ok']:
                        p2 = gen_pattern_windowed(ctx, real)
                        if p2 and p2.get('ok'):
                            r['pattern'] = p2['pattern']
                            r['unique'] = True
                            r['pat_offset'] = p2['offset']
                n_fix += 1
    json.dump(data, open(path, 'w'), indent=1)
    for c in ctx_cache.values():
        c.tool.close()
    return n_fix


def process_wave3():
    path = '/home/z/re-analysis/mega/wave3.json'
    data = json.load(open(path))
    ctx_cache = {}
    n_fix = 0
    for fname, vers in data.items():
        for ver, r in vers.items():
            if not isinstance(r, dict) or not r.get('addr'):
                continue
            if ver not in ctx_cache:
                try:
                    ctx_cache[ver] = VersionCtx(ver)
                except Exception:
                    continue
            ctx = ctx_cache[ver]
            a = int(r['addr'], 16)
            sz = ctx.func_size(a)
            if sz >= STUB_MAX:
                continue
            real, nc = deoutline(ctx, a)
            if real != a:
                r['outlined_stub'] = r['addr']
                r['addr'] = hex(real)
                r['stub_callers'] = nc
                p = ctx.gen_pattern(real)
                if p and not p.get('icf_stub'):
                    r['pattern'] = p['pattern']
                    r['unique'] = p['ok']
                    if not p['ok']:
                        p2 = gen_pattern_windowed(ctx, real)
                        if p2 and p2.get('ok'):
                            r['pattern'] = p2['pattern']
                            r['unique'] = True
                            r['pat_offset'] = p2['offset']
                n_fix += 1
    json.dump(data, open(path, 'w'), indent=1)
    for c in ctx_cache.values():
        c.tool.close()
    return n_fix


if __name__ == '__main__':
    n1 = process_wave1()
    print(f"wave1 de-outlined: {n1}")
    n3 = process_wave3()
    print(f"wave3 de-outlined: {n3}")
