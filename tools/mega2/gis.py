#!/usr/bin/env python3
"""GIS v3 — GIsClient/GIsServer with GOT-indirect resolution.
Handles: ADRP+ADD (direct), ADRP+LDR→GOT slot (indirect, resolve via rels).
Byte-writes within 0x100 before the AllowCommandletRendering ref:
descending PC → index 1 = GIsServer (skip=1), index 2 = GIsClient (skip=2).
Usage: gis.py <version>"""
import sys, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
from engine import VersionCtx

ver = sys.argv[1]
ctx = VersionCtx.get(ver)
t = ctx.tool

xf = ctx.xrefs_to_string("AllowCommandletRendering")
if not xf:
    xf = ctx.xrefs_to_string("llowCommandletRendering")
fns = {}
for pc, tgt, addr, w, k in xf:
    f = t.func_of(pc)
    if f:
        fns.setdefault(f, []).append(pc)
if not fns:
    print(json.dumps({"status": "no_anchor"}))
    sys.exit(0)
fn = max(fns.keys(), key=lambda f: ctx.func_size(f))
sz = ctx.func_size(fn)
ref_pc = max(fns[fn])

text_lo, text_hi = ctx.text_va, ctx.text_va + ctx.text_size
code = t.elf[t.va2off(fn):t.va2off(fn) + sz]
writes = []   # (pc, global_va, kind)
reg = {}      # reg → ('addr', va) | ('gotslot', slot_va)

for ins in t.cs.disasm(code, fn):
    if ins.mnemonic == "adrp":
        ops = ins.op_str.split(", ")
        rd = ops[0].strip().lstrip("x").lstrip("w")
        try:
            page = int(ops[1].strip().lstrip("#"), 16)
            reg[int(rd)] = ("addr", page)
        except ValueError:
            pass
    elif ins.mnemonic == "add" and len(ins.op_str.split(",")) == 3:
        ops = [x.strip() for x in ins.op_str.split(",")]
        try:
            rd = int(ops[0].lstrip("x")); rn = int(ops[1].lstrip("x"))
            imm = int(ops[2].lstrip("#"), 0)
        except ValueError:
            continue
        if reg.get(rn, ("",))[0] == "addr":
            reg[rd] = ("addr", reg[rn][1] + imm)
    elif ins.mnemonic == "ldr":
        import re as _re
        m = _re.match(r"^(?:x|w)(\d+), \[(?:x|w)(\d+)(?:, #(0x[0-9a-f]+|\d+))?\]$", ins.op_str)
        if m:
            rd, rn = int(m.group(1)), int(m.group(2))
            imm = int(m.group(3), 0) if m.group(3) else 0
            base = reg.get(rn)
            if base and base[0] == "addr":
                slot = base[1] + imm
                gv = t.rel_value(slot)
                if gv:
                    reg[rd] = ("addr", gv)
                else:
                    reg[rd] = ("gotslot", slot)
    elif ins.mnemonic in ("strb", "str"):
        import re as _re
        m = _re.match(r"^(?:x|w)\d+, \[(?:x|w)(\d+)(?:, #(0x[0-9a-f]+|\d+))?\]$", ins.op_str)
        if m:
            rn = int(m.group(1))
            imm = int(m.group(2), 0) if m.group(2) else 0
            base = reg.get(rn)
            if base and base[0] == "addr":
                tgt = base[1] + imm
                if not (text_lo <= tgt < text_hi) and tgt > 0x100000:
                    writes.append((ins.address, tgt, 1 if ins.mnemonic == "strb" else 4))

# Erbium window: byte-writes within 0x100 before ref
gw = [(pc, tg, wd) for pc, tg, wd in writes
      if wd == 1 and ref_pc - 0x100 <= pc <= ref_pc]
gw.sort(key=lambda x: -x[0])
mode = "window0x100"
if len(gw) < 3:
    gw = [(pc, tg, wd) for pc, tg, wd in writes if wd == 1 and pc <= ref_pc]
    gw.sort(key=lambda x: -x[0])
    gw = gw[:12]
    mode = "funcwide"

out = {"anchor": hex(fn), "anchor_size": hex(sz), "ref_pc": hex(ref_pc), "mode": mode,
       "writes_desc": [(hex(pc), hex(tg)) for pc, tg, _ in gw]}
if len(gw) >= 3:
    out["GIsServer"] = hex(gw[1][1])
    out["GIsClient"] = hex(gw[2][1])
elif len(gw) == 2:
    out["GIsServer"] = hex(gw[1][1])

print(f"anchor {hex(fn)} mode={mode} writes={len(gw)}")
for pc, tg, _ in gw[:8]:
    print(f"  {hex(pc)} -> {hex(tg)}")
if "GIsServer" in out:
    print(f"GIsServer={out['GIsServer']} GIsClient={out.get('GIsClient','?')}")
json.dump(out, open(f'/home/z/re-analysis/mega/gis_{ver}.json', 'w'), indent=1)
print("saved")
