#!/usr/bin/env python3
"""Extract the master catalog of ALL Find* functions from Erbium source.
Output: catalog.json (features) + bodies.txt (full source per function)"""
import re, json, os

ROOT = '/tmp/Erbium/Erbium'
CPPS = []
for d in ['Erbium/Private', 'FortniteGame/Private', 'Engine/Private']:
    p = os.path.join(ROOT, d)
    if os.path.isdir(p):
        for f in sorted(os.listdir(p)):
            if f.endswith('.cpp'):
                CPPS.append(os.path.join(p, f))

DEF_RE = re.compile(
    r'^[ \t]*(?:uint64|uintptr_t|uint64_t|int32_t|uint32_t|uint32|int32|DWORD64|__int64|void|auto|static\s+uint64|inline\s+uint64)\s+'
    r'(Find\w+)\s*\([^;{]*\)\s*(?:\n[ \t]*)?\{', re.M)

def extract_functions(text):
    funcs = {}
    for m in DEF_RE.finditer(text):
        name = m.group(1)
        start = m.end() - 1  # position of '{'
        depth = 0
        i = start
        in_str = False
        while i < len(text):
            c = text[i]
            if c == '"' and text[i-1] != '\\':
                in_str = not in_str
            elif not in_str:
                if c == '{':
                    depth += 1
                elif c == '}':
                    depth -= 1
                    if depth == 0:
                        break
            i += 1
        body = text[m.start():i+1]
        funcs[name] = {'body': body, 'line': text[:m.start()].count('\n') + 1}
    return funcs

all_funcs = {}
for path in CPPS:
    text = open(path, encoding='utf-8', errors='replace').read()
    for name, info in extract_functions(text).items():
        info['file'] = os.path.basename(path)
        all_funcs[name] = info

# ---- feature extraction ----
catalog = {}
bodies = []
for name in sorted(all_funcs):
    info = all_funcs[name]
    body = info['body']
    strings = re.findall(r'StringRef\(\s*(L?)\s*"([^"]+)"', body)
    pats = re.findall(r'(?:FindPattern|Pattern)\(\s*"([^"]+)"', body)
    branches = re.findall(r'FortniteVersion\s*>=\s*([\d.]+)', body)
    branches = sorted({float(b) for b in branches}, reverse=True)
    uses = sorted(set(re.findall(r'\b(Find\w+)\s*\(', body)) - {name})
    hooks = re.findall(r'Hooking::(\w+)', body)
    catalog[name] = {
        'file': info['file'], 'line': info['line'],
        'strings': [s[1] for s in strings], 'wide': [s[0] == 'L' for s in strings],
        'patterns': pats, 'branches': branches, 'uses': uses, 'hooks': hooks,
        'n_lines': body.count('\n'),
        'vft': ('Vft' in name) or ('Vft' in ''.join(uses)),
    }
    bodies.append(f"// ===== {name}  ({info['file']}:{info['line']}) =====\n{body}\n")

with open('/home/z/re-analysis/mega/catalog.json', 'w') as f:
    json.dump(catalog, f, indent=1)
with open('/home/z/re-analysis/mega/bodies.txt', 'w') as f:
    f.write('\n'.join(bodies))

# summary print
n_str = sum(1 for v in catalog.values() if v['strings'])
n_pat = sum(1 for v in catalog.values() if v['patterns'])
n_vft = sum(1 for v in catalog.values() if v['vft'])
print(f"total: {len(catalog)} functions")
print(f"  with string anchors: {n_str}")
print(f"  with x64 patterns:   {n_pat}")
print(f"  vtable-based:        {n_vft}")
print(f"  neither (pure logic/derivations): {sum(1 for v in catalog.values() if not v['strings'] and not v['patterns'])}")
for name, v in catalog.items():
    tag = []
    if v['strings']: tag.append('S:' + '|'.join(v['strings'])[:60])
    if v['patterns']: tag.append(f'P:{len(v["patterns"])}')
    if v['vft']: tag.append('VFT')
    if v['uses']: tag.append('U:' + ','.join(u.replace('Find', '') for u in v['uses'])[:50])
    if v['hooks']: tag.append('H:' + ','.join(set(v['hooks']))[:40])
    print(f"  {name:45s} br={str(v['branches'])[:40]:42s} {' '.join(tag)}")
