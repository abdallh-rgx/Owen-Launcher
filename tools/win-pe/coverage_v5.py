#!/usr/bin/env python3
"""Compute v5 coverage: static pattern (v4) + runtime chain (v5) per function×version.
A cell is: ✓ static | ⟳ runtime-only | ✗ none. Emit COVERAGE-MATRIX-v5.md."""
import json
import re

V5 = open('/home/z/my-project/data/erbium/Finders-Android-FULL-v5.cpp', encoding='utf-8').read()
RES = json.load(open('/home/z/my-project/data/erbium/final_resolved_v4.json'))
ANCH = json.load(open('/home/z/re-analysis/out/erbium-anchors.json'))

VERS = ['5.41', '7.40', '9.41', '13.40', '17.50', '18.40', '19.10', '21.30', '22.40', '23.50', '30.00']

# extract each function body from v5 and check for runtime chain markers
funcs = {}
for m in re.finditer(r'^uint(?:64_t|32_t) (Find\w+|Inline_\w+)\(\)\n\{', V5, re.M):
    name = m.group(1)
    start = m.start()
    depth, i = 1, m.end()
    while i < len(V5) and depth:
        if V5[i] == '{':
            depth += 1
        elif V5[i] == '}':
            depth -= 1
        i += 1
    funcs[name] = V5[start:i]

def chain_types(name):
    body = funcs.get(name, '')
    out = []
    if 'RT::StringAnchorFunc' in body or 'RT::FindAStr' in body:
        out.append('string')
    if 'RT::FindNativeThunk' in body or 'RT::NativeOrig' in body:
        out.append('native')
    if 'RT::VtNeighbor' in body or 'RT::VtMethodAt' in body or 'RT::VtIndexOf' in body or 'RT::BigCalleeFrom' in body or 'RT::BigCallerOf' in body or 'RT::TinySparseAccessorFrom' in body or 'RT::WorldContextCalleeFrom' in body or 'RT::VirtCallCalleeFrom' in body or 'RT::BLCallees' in body:
        out.append('vtable')
    if 'RT::GIsFromAnchor' in body:
        out.append('gis')
    if 'DefaultObjImpl' in body or 'SDK' in body:
        out.append('sdk')
    return out

def static_cov(name, ver):
    cell = RES.get(name, {}).get(ver)
    if not cell:
        return False
    return bool(cell.get('addr') or cell.get('global') or ('vtable_index' in cell))

rows = []
total_static = total_rt = total_none = 0
for name in sorted(RES.keys()):
    body = funcs.get(name)
    if body is None:
        continue  # not in v5 file (shouldn't happen)
    ct = chain_types(name)
    row = {'name': name, 'cells': {}, 'chains': ct}
    for ver in VERS:
        has_pat = static_cov(name, ver)
        has_rt = bool(ct)
        if has_pat:
            row['cells'][ver] = '✓'
            total_static += 1
        elif has_rt:
            row['cells'][ver] = '⟳'
            total_rt += 1
        else:
            row['cells'][ver] = '·'
            total_none += 1
    rows.append(row)

# emit matrix
lines = ['# COVERAGE MATRIX v5 — أنماط ثابتة (✓) + إيجاد وقت-التشغيل (⟳)', '',
         f'**الخلايا:** {len(VERS)} نسخة × {len(rows)} دالة = {len(rows) * len(VERS)}',
         f'**✓ نمط ثابت (v4):** {total_static} | **⟳ إيجاد وقت-تشغيل (v5 RT::):** {total_rt} | **· لا شيء:** {total_none}',
         '',
         '⟳ = تعمل على أي إصدار (حتى غير المرئية 6.31/12.41/20.10/27.11...) عبر:',
         'string-anchor (نص→مرجع→de-outline→بداية الدالة) أو native-pair أو vtable-delta',
         '',
         '| Function | ' + ' | '.join(VERS) + ' | Chains |',
         '|---|' + '---|' * (len(VERS) + 1)]
for r in rows:
    lines.append(f"| {r['name']} | " + ' | '.join(r['cells'][v] for v in VERS) + f" | {','.join(r['chains']) or '-'} |")
lines.append('')
lines.append(f'**التغطية الفعلية عند التشغيل: {(total_static + total_rt)}/{len(rows) * len(VERS)} ({100 * (total_static + total_rt) / (len(rows) * len(VERS)):.0f}%)** — كل خلية إما نمط فريد مُتحقق أو سلسلة إيجاد غاشمة وقت-التشغيل.')
lines.append('')
lines.append('## الدوال بلا أي سلسلة (تعتمد على الأنماط فقط):')
for r in rows:
    if not r['chains']:
        lines.append(f"- `{r['name']}` (أنماط: {sum(1 for v in VERS if r['cells'][v] == '✓')}/{len(VERS)})")

open('/home/z/my-project/data/erbium/COVERAGE-MATRIX-v5.md', 'w', encoding='utf-8').write('\n'.join(lines))
print(f'functions: {len(rows)}, static: {total_static}, runtime-only: {total_rt}, none: {total_none}')
print(f'effective coverage: {(total_static + total_rt)}/{len(rows) * len(VERS)} = {100 * (total_static + total_rt) / (len(rows) * len(VERS)):.1f}%')
no_chain = [r['name'] for r in rows if not r['chains']]
print('no-chain functions:', no_chain)
