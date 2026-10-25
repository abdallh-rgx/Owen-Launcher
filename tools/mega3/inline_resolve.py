#!/usr/bin/env python3
"""INLINE RESOLVE — resolve the 17 out-of-Finders.cpp functions on all 11 versions.
Mechanisms:
  R1 native by-name (ToDeathCause, ServerOnAttemptInteract, EquipVehicleWeapon)
  R2 reuse FindUnEquipVehicleWeapon (same target as UnequipVehicleWeaponFunc)
  R3 registrar pair-scan (name string → {name,thunk} rels pair → thunk → BL callee = impl)
  R4 tiny-fn immediate fingerprint (GetSparseClassData: ldr_x imm in tiny function)
  R5 string anchor (PostUpdate, CollectGarbageInternal)
  R6 xfp from Windows exe (AttemptDeriveFromURL)
Output: inline_<ver>.json
"""
import json, sys, os, struct, gc
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, VERSIONS, load_impls
from pattern_fix import gen_pattern_windowed
from x64tool import X64
from xfp import FpMatcher, win_field_imms

BYNAME = json.load(open('/home/z/my-project/data/db/by-name.json'))
IMPLS = load_impls()
VF = json.load(open('/home/z/re-analysis/mega/verification_full.json'))
FIX = json.load(open('/home/z/re-analysis/mega/fixcells_all.json'))
RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
TGT3B = json.load(open('/home/z/re-analysis/mega/targeted3b.json')) if os.path.exists('/home/z/re-analysis/mega/targeted3b.json') else {}

OUT = {}
p = '/home/z/re-analysis/mega/inline_all.json'
if os.path.exists(p):
    OUT = json.load(open(p))


def save():
    json.dump(OUT, open(p, 'w'), indent=1)


def try_pattern(ctx, addr, min_sz=0x18):
    if ctx.func_size(addr) < min_sz:
        return None
    r = ctx.gen_pattern(addr, max_ins=44)
    if r and r.get('ok'):
        return r
    r = gen_pattern_windowed(ctx, addr, max_span=100)
    if r and r.get('ok'):
        return r
    return None


def record(ver, name, addr, ctx, strat, extra=None):
    if OUT.get(ver, {}).get(name, {}).get('status') == 'OK':
        return True
    r = try_pattern(ctx, addr)
    if not r:
        return False
    e = {"status": "OK", "addr": hex(addr), "size": hex(ctx.func_size(addr)), "strategy": strat,
         **{k: r[k] for k in ('pattern', 'offset') if k in r}}
    if extra:
        e.update(extra)
    OUT.setdefault(ver, {})[name] = e
    print(f"  ✓ [{ver}] {name} = {hex(addr)} ({strat})", flush=True)
    return True


def verified_addr(finder, ver):
    rec = VF['per_version'].get(ver, {}).get(finder)
    if rec and rec.get('p0') == 'UNIQUE' and rec.get('resolved_addr'):
        return int(rec['resolved_addr'], 16)
    for src in (TGT3B.get(ver, {}), FIX.get(ver, {})):
        e = src.get(finder)
        if e and e.get('status') == 'OK' and e.get('addr'):
            return int(e['addr'], 16)
    e = RESOLVED.get(finder, {}).get(ver)
    if e and e.get('addr'):
        return int(e['addr'], 16)
    return None


def registrar_thunk(ctx, name):
    """Android {name,thunk} registrar pair via relocations"""
    t = ctx.tool
    occs = ctx.occurrences(name, wide=False)
    if not occs:
        return None
    str_va = occs[0][0]
    rels = t.rels
    hits = np.nonzero(rels[:, 1] == str_va)[0]
    for h in hits[:6]:
        slot = int(rels[h, 0])
        nxt = np.nonzero(rels[:, 0] == slot + 8)[0]
        if len(nxt):
            func = int(rels[nxt[0], 1])
            if t.text_va <= func < t.text_va + t.text_size:
                return func
    return None


def deoutline(ctx, fn, min_sz=0x60):
    t = ctx.tool
    cur = fn
    for _ in range(5):
        if ctx.func_size(cur) >= min_sz:
            break
        callers = set()
        for pc, tg in t.callers_of(cur):
            f = t.func_of(pc)
            if f is not None and f != cur:
                callers.add(f)
        if not callers:
            break
        cur = max(callers, key=lambda f: ctx.func_size(f))
    return cur


def resolve_version(ver):
    if ver in OUT:
        print(f"[{ver}] cached")
        return
    ctx = VersionCtx.get(ver)
    t = ctx.tool
    impl = IMPLS.get(ver, {})

    # ---- R1: natives ----
    for iname, natname in [("Inline_ToDeathCauseNative", "ToDeathCause"),
                           ("Inline_ServerOnAttemptInteract", "ServerOnAttemptInteract"),
                           ("Inline_EquipVehicleWeaponFunc", "EquipVehicleWeapon")]:
        ie = impl.get(natname) or (BYNAME.get(natname, {}).get(ver))
        if ie:
            for kind in ('orig', 'thunk'):
                a = ie.get(kind) if isinstance(ie, dict) else None
                if a and isinstance(a, int):
                    if ctx.func_size(a) >= 0x18:
                        record(ver, iname, a, ctx, f"native:{natname}:{kind}")
                        break
                elif a and isinstance(a, str):
                    a = int(a, 16)
                    if ctx.func_size(a) >= 0x18:
                        record(ver, iname, a, ctx, f"native:{natname}:{kind}")
                        break

    # ---- R2: UnequipVehicleWeapon = FindUnEquipVehicleWeapon ----
    a = verified_addr("FindUnEquipVehicleWeapon", ver)
    if a:
        record(ver, "Inline_UnequipVehicleWeaponFunc", a, ctx, "reuse:FindUnEquipVehicleWeapon")

    # ---- R3: registrar pairs → thunk → BL callee (impl) ----
    for iname, uname in [("Inline_OnPawnEnterSeatFunc", "OnPawnEnterSeat")]:
        th = registrar_thunk(ctx, uname)
        if th:
            # thunk → BL callee (the C++ impl)
            bls = [tg for pc, tg in ctx.bls_in(th)] if ctx.func_size(th) <= 0x200 else []
            if len(bls) >= 1:
                # biggest callee = impl usually
                best = max(bls, key=lambda f: ctx.func_size(f) if t.is_func(f) else 0)
                if t.is_func(best) and ctx.func_size(best) >= 0x40:
                    record(ver, iname, best, ctx, f"registrar:{uname}:thunk->impl")
                    continue
            if ctx.func_size(th) >= 0x40:
                record(ver, iname, th, ctx, f"registrar:{uname}:thunk")

    # ---- R5: string anchors ----
    for iname, s in [("Inline_PostUpdate", "ABuildingSMActor::PostUpdate() Building: %s, AltMeshIdx: %d"),
                     ("Inline_CollectGarbagePatch", "CollectGarbageInternal() is flushing async loading")]:
        fns = ctx.anchored_funcs(s)
        if fns:
            best = None
            for fn in sorted(fns.keys(), key=lambda f: ctx.func_size(f)):
                fn2 = deoutline(ctx, fn)
                if ctx.func_size(fn2) >= 0x40:
                    best = fn2
                    break
            if best:
                record(ver, iname, best, ctx, f"string:{s[:28]}")

    # ---- R4: GetSparseClassData (tiny fn: sub? ldr x0,[x0,#imm]; ret) ----
    # Windows: sub rsp,28; mov rax,[rcx+imm32]; xor r8d; mov r9,rcx → on ARM: ldr x0,[x0,#imm] tiny
    if "Inline_GetSparseClassData" not in OUT.get(ver, {}):
        # find tiny functions that are pure getters: ldr x0,[x0,#imm]; ret
        va, off, size = ctx.text_va, ctx.text_off, ctx.text_size
        words = np.frombuffer(t.elf, dtype='<u4', count=size // 4, offset=off)
        # pattern: LDR X0,[X0,#imm] (0xF9400000 | imm<<10) followed within 3 words by RET
        m = ((words & np.uint32(0xFFC0001F)) == np.uint32(0xF9400000))  # ldr x0,[x0,#imm]
        idx = np.nonzero(m)[0]
        cands = []
        for i in idx:
            # check next 4 words for RET and function small
            seg = words[i:min(i + 5, len(words))]
            has_ret = np.any((seg & np.uint32(0xFFFFFC1F)) == np.uint32(0xD65F0000))
            if not has_ret:
                continue
            pc = va + int(i) * 4
            f = t.func_of(pc)
            if f is None or ctx.func_size(f) > 0x30 or ctx.func_size(f) < 0x10:
                continue
            # ensure ldr is early in function
            if pc - f > 0x10:
                continue
            imm = ((int(words[i]) >> 10) & 0xFFF) << 3
            cands.append((f, imm))
        # GetSparseClassData: the offset matches SparseClassData offset — Windows 21.30 fn
        # gives reference imm (e.g. from winhard-style extraction); we accept candidates and
        # disambiguate via callers (called from many building funcs)
        scored = {}
        for f, imm in cands:
            n_callers = len(t.callers_of(f))
            if n_callers >= 3:
                scored[f] = (n_callers, imm)
        if scored:
            # the getter: most-called with consistent imm — take top by callers
            best = max(scored.keys(), key=lambda f: scored[f][0])
            if scored[best][0] >= 8:
                record(ver, "Inline_GetSparseClassData", best, ctx,
                       f"tiny-getter(imm={hex(scored[best][1])},callers={scored[best][0]})")

    VersionCtx._cache.pop(ver, None)
    gc.collect()


def resolve_attemptderive():
    """R6: AttemptDeriveFromURL via Windows xfp on all versions"""
    x = X64.get()
    # locate via x64 pattern from catalog_v3
    CAT3 = json.load(open('/home/z/re-analysis/mega/catalog_v3.json'))
    pats = [p for p in CAT3["Inline_AttemptDeriveFromURL"]["patterns"] if len(p) > 10]
    win_addr = None
    for p in pats:
        hits = x.scan_pattern(p, limit=6)
        for h in hits:
            fn = x.func_of(h)
            if fn == h:
                win_addr = fn
                break
        if win_addr:
            break
    if not win_addr:
        print("  × AttemptDeriveFromURL: no windows location")
        return
    print(f"  AttemptDeriveFromURL win = {hex(win_addr)} size {hex(x.func_size(win_addr))}")
    wf = win_field_imms(win_addr)
    specs = []
    for imm in sorted(set(wf.get("ldr_d", []))):
        if imm < 0x4000: specs.append(("ldr_w", imm))
    for imm in sorted(set(wf.get("ldr_q", []))):
        if imm < 0x8000: specs.append(("ldr_x", imm))
    for imm in sorted(set(wf.get("ldrb", []))):
        if imm < 0x1000: specs.append(("ldrb", imm))
    specs = specs[:12]
    print(f"  specs: {specs[:8]}")
    for ver in VERSIONS:
        if OUT.get(ver, {}).get("Inline_AttemptDeriveFromURL", {}).get('status') == 'OK':
            continue
        ctx = VersionCtx.get(ver)
        fm = FpMatcher(ctx)
        cands = fm.find_functions(specs, min_hits=max(2, (len(specs) + 1) // 2))
        wsize = x.func_size(win_addr)
        scored = []
        for f, hits in cands[:60]:
            sz = ctx.func_size(f)
            if not (0.3 * wsize <= sz <= 3.5 * wsize):
                continue
            if not ctx.tool.is_func(f):
                continue
            scored.append((hits, f))
        scored.sort(key=lambda t: -t[0])
        if scored and (len(scored) == 1 or scored[0][0] >= scored[1][0] + 2):
            record(ver, "Inline_AttemptDeriveFromURL", scored[0][1], ctx, "win-fp")
        else:
            print(f"  × [{ver}] AttemptDeriveFromURL ambig ({len(scored)})")
        VersionCtx._cache.pop(ver, None)
        gc.collect()


def main():
    for ver in VERSIONS:
        print(f"== {ver} ==", flush=True)
        resolve_version(ver)
        save()
    print("== AttemptDeriveFromURL (win-fp) ==", flush=True)
    resolve_attemptderive()
    save()
    n = sum(1 for v in OUT for k in OUT[v] if OUT[v][k].get('status') == 'OK')
    print(f"\ninline resolved cells: {n}")


if __name__ == '__main__':
    main()
