#!/usr/bin/env python3
"""Extract all x64 FindPattern strings from Erbium Finders.cpp and locate each function
on a Windows exe. Produces a ground-truth map {FuncName: RVA} per exe."""
import json
import re
import sys

sys.path.insert(0, '/home/z/re-analysis/tools')
from xref2 import PE


def parse_finders(path):
    """Parse Finders.cpp: {funcName: [patterns...]} preserving order."""
    src = open(path, encoding='utf-8', errors='replace').read()
    funcs = {}
    # match uint64_t FindX() { ... } blocks (also uint64 / uintptr_t)
    for m in re.finditer(r'uint(?:64_t|64|ptr_t)\s+(Find\w+)\s*\(\s*\)\s*\{', src):
        name = m.group(1)
        start = m.end()
        # find matching closing brace at depth 0
        depth, i = 1, start
        while i < len(src) and depth:
            c = src[i]
            if c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
            i += 1
        body = src[start:i]
        # extract all FindPattern("...") literals
        pats = re.findall(r'FindPattern\(\s*"((?:[^"\\]|\\.)*)"\s*(?:,\s*(false|true))?\s*\)', body)
        pats = [(p.replace('\\\\"', '"').replace('\\\\', '\\'), raw == 'false') for p, raw in pats]
        if pats:
            funcs[name] = pats
    return funcs


def pat_to_bytes(p):
    """IDA-style pattern -> (bytes, mask) where ?? = wildcard."""
    toks = p.split()
    b, m = bytearray(), bytearray()
    for t in toks:
        if t in ('?', '??', '? ?'):
            b.append(0)
            m.append(0)
        else:
            b.append(int(t, 16))
            m.append(0xFF)
    return bytes(b), bytes(m)


def find_pattern(d, pat, start=0, single=True):
    """Numpy-vectorized IDA pattern search over mmap-backed .text buffer.
    d: np.uint8 array view. Returns list of offsets (into d)."""
    import numpy as np
    b, m = pat_to_bytes(pat)
    n = len(b)
    if n == 0:
        return []
    arr = d if isinstance(d, np.ndarray) else np.frombuffer(d, dtype=np.uint8)
    # pick up to 6 non-wildcard probe positions
    fixed = [i for i in range(n) if m[i]][:6]
    if not fixed:
        return []
    cand = np.ones(len(arr) - n + 1, dtype=bool)
    for fp in fixed:
        cand &= arr[fp:fp + len(cand)] == b[fp]
        if not cand.any():
            return []
    idx = np.nonzero(cand)[0]
    # verify full pattern for candidates
    out = []
    for i in idx:
        ok = True
        for j in range(n):
            if m[j] and arr[i + j] != b[j]:
                ok = False
                break
        if ok:
            out.append(int(i))
            if single:
                return out
    return out


if __name__ == '__main__':
    finders_path = sys.argv[1] if len(sys.argv) > 1 else '/home/z/erbium-src/Erbium/Erbium/Private/Finders.cpp'
    funcs = parse_finders(finders_path)
    print(f'parsed {len(funcs)} functions with patterns')
    exes = sys.argv[2:] or ['/home/z/re-analysis/binaries/win/FortniteClient-27.11.exe']
    for exe in exes:
        pe = PE(exe)
        t = pe.sections['.text']
        import numpy as np
        text = np.frombuffer(pe.d, dtype=np.uint8, count=t["rsize"], offset=t["raw"]).copy()
        ver_off = pe.d.find(b'Fortnite+Release-')
        ver = pe.d[ver_off:ver_off + 40].split(b'\x00')[0].decode().split('-')[1]
        found, multi, miss = {}, {}, []
        for name, pats in funcs.items():
            hit = None
            tried = []
            for pat, raw in pats:
                offs = find_pattern(text, pat)
                tried.append((pat[:40], len(offs)))
                if len(offs) == 1:
                    hit = (offs[0] + t['va'], pat, 'unique')
                    break
                if len(offs) > 1:
                    multi.setdefault(name, []).append((len(offs), pat[:50]))
            if hit:
                found[name] = {'rva': hex(hit[0]), 'pat': hit[1][:80], 'how': hit[2]}
            else:
                miss.append({'name': name, 'tried': tried})
        print(f'[{exe.split("/")[-1]} v{ver}] unique: {len(found)}, multi: {len(multi)}, miss: {len(miss)}')
        out = {'exe': exe, 'version': ver, 'found': found,
               'multi': {k: v[:3] for k, v in multi.items()},
               'miss': [m['name'] for m in miss]}
        fn = f'/home/z/re-analysis/out/win-x64pat-{ver}.json'
        with open(fn, 'w') as f:
            json.dump(out, f, indent=1)
        print('saved', fn)
        for name, info in list(found.items())[:10]:
            print(f'  {name} @ {info["rva"]}')
        print('  missed:', [m['name'] for m in miss][:25])
