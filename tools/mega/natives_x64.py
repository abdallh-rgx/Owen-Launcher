#!/usr/bin/env python3
"""Extract native UFunction thunks+impls from the Windows exe, paired with Android offsets.h.
Method: {char* name → void* thunk} qword pairs in .rdata (FNativeFunctionRegistrar arrays).
Vectorized: one pass over .rdata/.data as u64 arrays.
Output: natives_x64.json {name: {x64_thunk, x64_impl, arm_thunk, arm_orig}}
"""
import sys, os, json, struct, re
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from x64tool import X64

def main():
    x = X64.get()
    full = x.full
    # 1) Android natives
    txt = open('/tmp/work2/src/21.30/offsets.h').read()
    arm = {}
    for m in re.finditer(r'#define UF_(\w+)_THUNK_RVA\s+(0x[0-9A-F]+)', txt):
        arm.setdefault(m.group(1), {})['thunk'] = int(m.group(2), 16)
    for m in re.finditer(r'#define UF_(\w+)_ORIG_RVA\s+(0x[0-9A-F]+)', txt):
        arm.setdefault(m.group(1), {})['orig'] = int(m.group(2), 16)
    print(f"android natives: {len(arm)}")

    # 2) strings in .rdata/.data: name → VAs (single pass)
    str_map = {}
    for sec in ('.rdata', '.data'):
        vaddr, vsize, roff, rsize = x.sections[sec]
        data = full[roff:roff + rsize]
        # walk C strings
        start = None
        for i in range(len(data)):
            c = data[i]
            if c == 0:
                if start is not None:
                    L = i - start
                    if 4 <= L <= 80:
                        s = data[start:i]
                        # must be identifier-like
                        ok = all(65 <= c <= 90 or 97 <= c <= 122 or 48 <= c <= 57 or c == 95 for c in s)
                        if ok:
                            str_map.setdefault(s, []).append(x.base + vaddr + start)
                    start = None
            elif start is None:
                start = i
    print(f"identifier strings in exe rdata/data: {len(str_map)}")

    # 3) qword refs: build sorted u64 arrays of .rdata/.data
    secs_u64 = []
    for sec in ('.rdata', '.data'):
        vaddr, vsize, roff, rsize = x.sections[sec]
        arr = np.frombuffer(full, dtype=np.uint64, count=rsize // 8, offset=roff)
        base_va = x.base + vaddr
        secs_u64.append((arr, base_va))
    all_vals = np.concatenate([a for a, b in secs_u64])
    order = np.argsort(all_vals, kind="stable")
    sorted_vals = all_vals[order]

    text_lo, text_hi = x.text_va, x.text_va + len(x.text)

    def qword_positions(va):
        """positions (array index) where u64 == va"""
        s = np.searchsorted(sorted_vals, va, "left")
        e = np.searchsorted(sorted_vals, va, "right")
        return order[s:e]

    def idx_to_va(arr_idx):
        # find which section
        for arr, base in secs_u64:
            if arr_idx < len(arr):
                return base + arr_idx * 8, arr[arr_idx]
            arr_idx -= len(arr)
        return None, None

    # 4) VECTORIZED pairing: one pass — positions where qword == str_va and next qword in .text
    str_va_arr = np.unique(np.array([va for vals in str_map.values() for va in vals], dtype=np.uint64))
    thunks_by_strva = {}
    for arr, base in secs_u64:
        n = len(arr)
        if n < 2:
            continue
        v = arr.astype(np.uint64)
        # match positions: v[i] in str_va_arr and v[i+1] in text range
        pos = np.searchsorted(str_va_arr, v[:n-1], "left")
        posc = np.searchsorted(str_va_arr, v[:n-1], "right")
        is_str = pos != posc
        nxt_in_text = (v[1:n] >= text_lo) & (v[1:n] < text_hi)
        hit = np.flatnonzero(is_str & nxt_in_text)
        for i in hit:
            sva = int(v[i])
            tgt = int(v[i + 1])
            thunks_by_strva.setdefault(sva, set()).add(tgt)
    print(f"str→thunk map entries: {len(thunks_by_strva)}")

    pairs = {}
    ambig = 0
    missing = 0
    for name, ainfo in arm.items():
        nb = name.encode()
        if nb not in str_map:
            missing += 1
            continue
        thunks = set()
        for sva in str_map[nb][:4]:
            thunks |= thunks_by_strva.get(sva, set())
        thunks = [t for t in thunks if x.func_of(t) == t]
        if len(thunks) == 1:
            pairs[name] = {'x64_thunk': hex(thunks[0]),
                           'arm_thunk': hex(ainfo.get('thunk', 0)),
                           'arm_orig': hex(ainfo['orig']) if ainfo.get('orig') else None}
        elif len(thunks) > 1:
            pairs[name] = {'x64_thunk': [hex(t) for t in sorted(thunks)],
                           'arm_thunk': hex(ainfo.get('thunk', 0)),
                           'arm_orig': hex(ainfo['orig']) if ainfo.get('orig') else None}
            ambig += 1
    print(f"paired: {len(pairs)} (ambiguous: {ambig}, missing string: {missing})")
    json.dump(pairs, open('/home/z/re-analysis/mega/natives_x64.json', 'w'))

    # 5) derive impls for unambiguous thunks: analyze call AND tail-jmp targets
    from capstone import Cs, CS_ARCH_X86, CS_MODE_64
    cs = Cs(CS_ARCH_X86, CS_MODE_64)
    # global call-target frequency across thunks
    freq = {}
    thunk_calls = {}
    for name, p in pairs.items():
        th = p['x64_thunk']
        if isinstance(th, list):
            continue
        th = int(th, 16)
        o = x.va2off(th)
        end = x.func_end(th)
        code = full[o:o + (end - th)]
        tgts = []
        for ins in cs.disasm(code, th):
            if ins.mnemonic in ('call', 'jmp'):
                op = ins.op_str.strip()
                if op.startswith('0x'):
                    try:
                        t = int(op, 16)
                        if t != ins.address and x.func_of(t) == t:
                            tgts.append((t, ins.mnemonic))
                    except Exception:
                        pass
        thunk_calls[name] = tgts
        for t, _ in tgts:
            freq[t] = freq.get(t, 0) + 1
    # impl = largest low-freq target (not shared helper); prefer tail-jmp
    n_impl = 0
    for name, tgts in thunk_calls.items():
        cands = [(t, m) for t, m in tgts if freq.get(t, 0) <= 3]
        if not cands:
            continue
        # tail-jmp targets first (classic exec thunk → impl), then size
        cands.sort(key=lambda tm: (0 if tm[1] == 'jmp' else 1, -(x.func_end(tm[0]) - tm[0])))
        pairs[name]['x64_impl'] = hex(cands[0][0])
        n_impl += 1
    print(f"x64 impls derived: {n_impl}")
    json.dump(pairs, open('/home/z/re-analysis/mega/natives_x64.json', 'w'), indent=0)

if __name__ == '__main__':
    main()
