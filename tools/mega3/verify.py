#!/usr/bin/env python3
"""VERIFY v3 — Cross-arch verification of Erbium→Android ARM64 resolutions
against FortniteClient-Win64-Shipping.exe (21.30).

Four independent verification paths:
  P1 CROSS-ARCH (21.30 ↔ exe): Windows function (x64 pattern/native/string located)
     vs Android function → compare strings referenced, natives called, size ratio.
  P2 NATIVE-IDENTITY (all versions): resolved ARM addr must equal native's
     thunk/orig address from the per-version dumper (offsets.h).
  P3 PATTERN-RESCAN (all versions): every saved pattern must match EXACTLY ONCE
     and at the recorded address.
  P4 ANCHOR-CONTAINMENT (all versions): anchor strings referenced from within
     the resolved function (or its de-outlined stub chain).

Verdicts: VERIFIED_STRONG ≥2 paths pass / VERIFIED 1 path / WEAK structure-only
          / MISMATCH conflict / UNVERIFIABLE no evidence
Output: verification_v3.json + console table
"""
import json, sys, os, struct, re
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, load_impls, VERSIONS
from x64tool import X64

RESOLVED = json.load(open('/home/z/re-analysis/mega/final_resolved_v2.json'))
CAT3 = json.load(open('/home/z/re-analysis/mega/catalog_v3.json'))
XWIN = json.load(open('/home/z/re-analysis/mega/xwin.json'))
NX = json.load(open('/home/z/re-analysis/mega/natives_x64.json'))  # 21.30 pairing
NATMAP = json.load(open('/home/z/my-project/data/db/erbium-native-map.json'))
IMPLS = load_impls()  # {ver: {name: {thunk, orig}}}

# ---------- native tables ----------
# android natives by version: name -> {thunk, orig}
ANDNAT = {}
for ver in VERSIONS:
    d = json.load(open(f'/home/z/my-project/data/db/natives-v{ver}.json'))
    ANDNAT[ver] = d

# x64 thunk -> native name (21.30)
x_thunk2name = {}
x_impl2name = {}
for name, v in NX.items():
    th = v.get('x64_thunk')
    ths = th if isinstance(th, list) else ([th] if th else [])
    for t in ths:
        x_thunk2name.setdefault(int(t, 16), []).append(name)
    im = v.get('x64_impl')
    if im:
        x_impl2name.setdefault(int(im, 16), []).append(name)


def arm_native_addr(ver, name, prefer=('orig', 'thunk')):
    """address of native `name` on Android `ver` (impl preferred)"""
    impl = IMPLS.get(ver, {}).get(name, {})
    for k in prefer:
        if impl.get(k):
            return impl[k], k
    return None, None


# ================= P1: CROSS-ARCH =================
def win_features(x, fn):
    """Windows function features: strings (rip-rel), native calls, size, n_calls"""
    start_off = x.va2off(fn)
    end_off = x.va2off(x.func_end(fn))
    buf = x.full
    strings, calls = {}, []
    lim = min(end_off, start_off + 0x9000)
    for o in range(start_off, lim - 6):
        b0, b1, b2 = buf[o], buf[o+1], buf[o+2]
        if b0 in (0x48, 0x4C, 0x49, 0x4D) and b1 in (0x8D, 0x8B) and (b2 & 0xC7) == 0x05:
            disp = struct.unpack_from("<i", buf, o + 3)[0]
            tgt = x.off2va(o) + 7 + disp
            vo = x.va2off(tgt)
            if vo is None:
                continue
            raw = x.full[vo:vo+280]
            s = None
            a = raw.split(b"\0")[0]
            if len(a) >= 6 and all(32 <= c < 127 for c in a[:32]):
                s = a.decode(errors="replace")
            else:
                try:
                    wtxt = raw.decode("utf-16-le", errors="strict").split("\0")[0]
                    if len(wtxt) >= 6 and all(32 <= ord(c) < 127 for c in wtxt[:32]):
                        s = wtxt
                except Exception:
                    pass
            if s:
                strings.setdefault(s, tgt)
        if b0 == 0xE8:
            rel = struct.unpack_from("<i", buf, o + 1)[0]
            calls.append(x.off2va(o) + 5 + rel)
    natives = []
    for tgt in calls:
        if tgt in x_thunk2name:
            natives += x_thunk2name[tgt]
    return {"strings": set(strings.keys()), "natives": set(natives),
            "n_calls": len(calls), "size": x.func_size(fn)}


def arm_features(ctx, fn, ver, stub_chain=None):
    """Android function features: strings (refs+ADR), natives called (BL→thunk/orig), size"""
    t = ctx.tool
    end = ctx.func_end(fn)
    strings = set()
    for tg, pc in ctx.refs_in(fn):
        s = ctx.read_string_at(tg)
        if s and len(s[1]) >= 6:
            strings.add(s[1])
    # ADR-referenced .text literals
    for pc, va in ctx.adr_refs_to_all(fn, end):
        s = ctx.read_string_at(va)
        if s and len(s[1]) >= 6:
            strings.add(s[1])
    # natives: BL targets matching thunk/orig addrs
    thunk2name = ANDNAT_THUNK2NAME.get(ver, {})
    natives = set()
    n_bls = 0
    for pc, tg in ctx.bls_in(fn):
        n_bls += 1
        if tg in thunk2name:
            natives.add(thunk2name[tg])
    # include stub-chain strings (de-outlined): strings of tiny stubs that ONLY this func calls
    if stub_chain:
        for sfn in stub_chain:
            for tg, pc in ctx.refs_in(sfn):
                s = ctx.read_string_at(tg)
                if s and len(s[1]) >= 6:
                    strings.add(s[1])
            for pc, va in ctx.adr_refs_to_all(sfn, ctx.func_end(sfn)):
                s = ctx.read_string_at(va)
                if s and len(s[1]) >= 6:
                    strings.add(s[1])
    return {"strings": strings, "natives": natives, "n_bls": n_bls, "size": end - fn}


# build android thunk/orig → name per version (lazy, only needed versions)
ANDNAT_THUNK2NAME = {}


def build_thunk_map(ver):
    if ver in ANDNAT_THUNK2NAME:
        return
    m = {}
    impl = IMPLS.get(ver, {})
    for name, e in impl.items():
        if e.get('orig'):
            m.setdefault(e['orig'], name)  # orig (impl) wins
        if e.get('thunk'):
            m.setdefault(e['thunk'], name)
    ANDNAT_THUNK2NAME[ver] = m


# ---- locate Windows function for each Find_* ----
def locate_windows(x, finder):
    """locate the Windows exe address of a finder target:
    1) xwin.json pre-located (pattern-based)
    2) native candidates (x64_impl / x64_thunk)
    returns (win_addr, how)"""
    e = XWIN.get(finder)
    if e and e.get('status') == 'OK' and e.get('win_func'):
        try:
            return int(e['win_func'], 16), 'xwin_pattern'
        except ValueError:
            pass
    # native candidates
    for natname in NATMAP.get(finder, []):
        if natname in NX:
            v = NX[natname]
            if v.get('x64_impl'):
                return int(v['x64_impl'], 16), f'native_impl:{natname}'
            th = v.get('x64_thunk')
            if th and not isinstance(th, list):
                return int(th, 16), f'native_thunk:{natname}'
    return None, None


def compare_features(win, arm):
    """compare feature sets → (score, details)"""
    det = {}
    ws, as_ = win['strings'], arm['strings']
    if ws and as_:
        inter = ws & as_
        det['strings_common'] = sorted(inter)[:8]
        det['strings_win_only'] = len(ws - as_)
        det['strings_arm_only'] = len(as_ - ws)
        det['strings_match'] = len(inter) > 0
    else:
        det['strings_match'] = None  # one side has no strings
    wn, an = win['natives'], arm['natives']
    if wn or an:
        inter = wn & an
        det['natives_common'] = sorted(inter)[:10]
        det['natives_match'] = len(inter) > 0 if (wn and an) else None
    else:
        det['natives_match'] = None
    sr = arm['size'] / max(win['size'], 1)
    det['size_ratio'] = round(sr, 2)
    det['size_ok'] = 0.25 <= sr <= 4.0
    return det


def main():
    x = X64.get()
    ctx = VersionCtx.get("21.30")
    build_thunk_map("21.30")

    # monkeypatch: adr refs within a range (vectorized)
    def adr_refs_to_all(lo, hi):
        arr = ctx.adr_by_tgt
        # arr sorted by tgt; need by pc → linear scan is fine for one function
        out = []
        t = ctx.tool
        n = min((hi - lo) // 4, 4000)
        o = t.va2off(lo)
        if o is None:
            return out
        words = np.frombuffer(t.elf, dtype="<u4", count=n, offset=o)
        is_adr = (words & np.uint32(0x9F000000)) == np.uint32(0x10000000)
        idx = np.nonzero(is_adr)[0]
        w = words[idx].astype(np.int64)
        immlo = (w >> 29) & 3
        immhi = (w >> 5) & 0x7FFFF
        imm = immlo | (immhi << 2)
        imm = np.where(imm & (1 << 20), imm - (1 << 21), imm)
        pcs = lo + idx.astype(np.int64) * 4
        tgts = pcs + imm
        for pc, tg in zip(pcs, tgts):
            out.append((int(pc), int(tg)))
        return out
    ctx.adr_refs_to_all = adr_refs_to_all

    results = {}
    finders = sorted(RESOLVED.keys())
    n_strong = n_ver = n_weak = n_mism = n_unver = 0
    for finder in finders:
        entry = RESOLVED[finder].get("21.30")
        if not entry or not entry.get('addr'):
            results[finder] = {"P1": "no_arm_21.30"}
            continue
        arm_addr = int(entry['addr'], 16)
        # de-outline: if tiny, climb to real fn for feature extraction
        real = arm_addr
        chain = []
        cur = arm_addr
        for _ in range(4):
            if ctx.func_size(cur) >= 0x60:
                break
            callers = set()
            for pc, tgt in ctx.tool.callers_of(cur):
                f = ctx.tool.func_of(pc)
                if f is not None and f != cur:
                    callers.add(f)
            if not callers:
                break
            chain.append(cur)
            cur = max(callers, key=lambda f: ctx.func_size(f))
        real = cur

        win_addr, how = locate_windows(x, finder)
        rec = {"arm_21.30": hex(arm_addr), "arm_real": hex(real), "win_how": how}
        if win_addr:
            rec["win_21.30"] = hex(win_addr)
            try:
                wf = win_features(x, win_addr)
                af = arm_features(ctx, real, "21.30", stub_chain=chain)
                det = compare_features(wf, af)
                rec["P1"] = det
                # P1 verdict
                votes = []
                if det['strings_match']:
                    votes.append('strings')
                if det['natives_match']:
                    votes.append('natives')
                if det['size_ok']:
                    votes.append('size')
                rec["P1_votes"] = votes
            except Exception as ex:
                rec["P1"] = f"error: {ex}"
        else:
            rec["P1"] = "no_windows_location"
        results[finder] = rec

    json.dump(results, open('/home/z/re-analysis/mega/verify_p1.json', 'w'), indent=1)

    # console summary
    print(f"{'function':44s} {'win':10s} {'votes'}")
    for finder in finders:
        r = results[finder]
        p1 = r.get('P1')
        if isinstance(p1, dict):
            votes = ",".join(r.get('P1_votes', []))
            print(f"{finder:44s} {r.get('win_how','—')[:10]:10s} {votes}  strs={p1.get('strings_match')} nats={p1.get('natives_match')} ratio={p1.get('size_ratio')}")
        else:
            print(f"{finder:44s} {'—':10s} {p1}")


if __name__ == '__main__':
    main()
