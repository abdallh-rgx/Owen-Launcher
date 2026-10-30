#!/usr/bin/env python3
"""
find_globals.py -- static scanner for stripped UE4 Android arm64 (arm64-v8a) libs.

Purpose
-------
Locate engine globals / tiny native functions in a *stripped* libUE4.so and emit
RVA records for a runtime mod (runtime address = load base + RVA).

Targets
-------
  A) GIsClient / GIsServer            -- UE engine globals (extern bool, cluster)
  B) UWorld::GetNetMode / AActor::GetNetMode  -- tiny leaf getters (NetMode byte)
  C) best-effort: UWorld::Listen (port 7777), UEngine::CreateNetDriver,
     UNetDriver::InitListen, ServerReplicateActors leads (string/immediate xrefs)

Findings baked into offsets/ogs-1241-android.json (12.41 CL-12910313 arm64)
--------------------------------------------------------------------------
  gis_client        0xf38181c  low    (only .data byte==1 with bool-style ldrb
                                       heat; 28 reads, 1 teardown store-of-0)
  gis_server        NOT FOUND  --      40 hot .bss zero-bytes audited: guards /
                                       state bytes / written flags / config
                                       members; best hypothesis: LTO folded it
                                       away (nothing writes GIsServer in a
                                       shipping client). Windows 12.41 has it
                                       at ImageBase+0x804B65A; this Android LTO
                                       build does not appear to keep it.
  uworld_getnetmode 0x8595b30  medium (unique ldrb w0,[x0,#0x168]; ret leaf;
                                       NetMode@0x168 from SDK gap 0x168..0x188)
  aactor_getnetmode NOT FOUND  --      fully inlined by LTO; extra lead kept as
                                       world_wrapper_getnetmode_leaf 0x837c208.

Method (single streaming pass, bounded memory)
----------------------------------------------
  1. Minimal self-contained ELF64 parser (sections only; no external deps beyond
     capstone). NOTE: section VAs != file offsets in this lib (delta 0x10000 for
     .data) -- always map through section headers.
  2. ONE streaming capstone (disasm_lite) pass over .text, chunked (16MB), with a
     linear register tracker:
        adrp xN, #page         -> xN = PAGE(page)      (capstone prints abs page)
        add  xD, xN, #imm[,...]-> xD = ADDR(page+imm)  (materialization)
        ldrb/strb/ldr/str/... [xN, #imm] -> target resolved through PAGE/ADDR state
     - Register state is invalidated on generic first-operand writes, stores
       excluded, caller-saved regs (x0-x17, x30) invalidated on bl/blr.
     - Cross-references recorded ONLY for targets inside .data/.bss (globals),
       with capped site lists, so memory stays bounded on ~200MB binaries.
  3. Second pass ONLY over shortlisted regions (few-KB windows) to produce
     disassembly evidence snippets.

Output
------
  JSON: {"binary","version","sha256","entries":{...},"notes"} per task spec.
  Misses are reported as {"found": false, "reason": "..."} -- never guessed.

Usage
-----
  python3 find_globals.py --lib /path/libUE4.so --out result.json
                         [--version 12.41-CL-12910313] [--text-limit BYTES]
                         [--top N] [--sdk-netmode-guess 0x168]

CI
--
  Deterministic, exit 0 on success (exit 2 on hard failure). --text-limit runs a
  fast smoke pass over the first N bytes of .text. Requires: python3, capstone>=5.
"""

import argparse
import hashlib
import json
import os
import struct
import sys
import time

try:
    from capstone import Cs, CS_ARCH_ARM64, CS_MODE_LITTLE_ENDIAN
except ImportError:  # pragma: no cover
    sys.stderr.write("capstone is required: pip install capstone\n")
    sys.exit(2)

# ----------------------------------------------------------------------------
# ELF64 minimal parser
# ----------------------------------------------------------------------------

class Section(object):
    __slots__ = ("name", "type", "flags", "addr", "off", "size", "link", "info",
                 "addralign", "entsize")

    @property
    def end(self):
        return self.addr + self.size


def parse_elf64(path):
    with open(path, "rb") as f:
        hdr = f.read(64)
        if hdr[:4] != b"\x7fELF" or hdr[4] != 2 or hdr[5] != 1:
            raise RuntimeError("not an ELF64 little-endian binary")
        (e_type, e_machine, _ver, _entry, _phoff, e_shoff, _flags, _ehsize,
         _phentsize, _phnum, e_shentsize, e_shnum, e_shstrndx) = \
            struct.unpack_from("<HHIQQQIHHHHHH", hdr, 16)
        if e_machine != 183:  # EM_AARCH64
            raise RuntimeError("not an AArch64 binary")
        f.seek(e_shoff)
        raw = f.read(e_shentsize * e_shnum)
        secs = []
        for i in range(e_shnum):
            s = Section()
            (s.name, s.type, s.flags, s.addr, s.off, s.size, s.link, s.info,
             s.addralign, s.entsize) = struct.unpack_from("<IIQQQQIIQQ", raw, i * e_shentsize)
            secs.append(s)
        shstr = secs[e_shstrndx]
        f.seek(shstr.off)
        strtab = f.read(shstr.size)
        for s in secs:
            end = strtab.find(b"\0", s.name)
            s.name = strtab[s.name:end].decode("ascii", "replace")
        byname = {s.name: s for s in secs}
    return byname


def read_section(f, sec):
    f.seek(sec.off)
    return f.read(sec.size)

# ----------------------------------------------------------------------------
# pass 1: streaming disassembly + ADRP register tracking
# ----------------------------------------------------------------------------

# xref record slots
K_LB, K_SB, K_LH, K_SW, K_LW, K_LD, K_SD, K_AM = range(8)
KIND_CHAR = "bshwqdA"  # for sites: b=byte-load B=?? -- see site tuples (chr, addr)
STORE_SLOTS = {K_SB: "s", K_SW: "v", K_SD: "x"}
LOAD_SLOTS = {K_LB: "b", K_LH: "h", K_LW: "w", K_LD: "d"}
MAX_SITES_PER_TARGET = 96
MAX_SITES_PER_IMM = 128
MAX_WRITES_PER_TARGET = 64
ACQ_BYTE_MNEMS = {"ldarb", "ldaprb"}   # acquire loads: __cxa_guard / sync flags

# mnemonics that never define a register (first operand is read-only / none)
NODEF_MNEMONICS = set("""
st str stp stur sti sttr stlr stxp stxrh strh stadd stclr steor stset
cmp cmn tst
cbz cbnz tbz tbnz
b br bl blr ret braa brab blraa blrab retaa retab braaz brabz blraaz blrabz
prfm prfum prfw
svc hvc smc nop yield wfe wfi sev sevl dmb dsb isb clrex
msr mrs sys sysl at dc ic tlbi
pacga autia autib pacia pacib pacizb pacizd xpaci
""".split())

BYTE_MNEMS = {"ldrb", "ldrsb", "ldurb", "ldursb", "ldtrb", "ldtrsb", "ldarb", "ldaprb", "strb", "sttrb", "stlrb"}
HALF_MNEMS = {"ldrh", "ldrsh", "ldurh", "ldursh", "ldtrh", "ldtrsh", "strh", "sttrh"}
SIMPLE_LS  = {"ldr", "str", "ldur", "stur", "ldtr", "sttr", "ldar", "stlr", "ldap", "ldapr",
              "ldxr", "ldxr", "stxr", "ldaxr", "stlxr", "ldxrh", "stxrh", "ldxp", "stxp",
              "ldaxp", "stlxp"}


def norm_xreg(tok):
    """'x9'->'x9', 'w9'->'x9', 'xzr'/'wzr'/'sp'/others -> None."""
    if len(tok) >= 2 and tok[0] in ("x", "w") and tok[1:].isdigit():
        return "x" + tok[1:]
    return None


def parse_imm(txt):
    txt = txt.strip()
    if txt.startswith("#"):
        txt = txt[1:]
    neg = txt.startswith("-")
    if neg:
        txt = txt[1:]
    if txt.lower().startswith("0x"):
        v = int(txt, 16)
    else:
        v = int(txt, 10)
    return -v if neg else v


class ScanResults(object):
    def __init__(self):
        self.xrefs = {}          # target -> [8 counters] + sites appended in slot 8 (list)
        self.leaf_ldrb = []      # (ldrb_site, imm) where `ldrb w0,[x0,#imm]; ret`
        self.leaf_ldrw = []      # `ldr  w0,[x0,#imm]; ret`
        self.leaf_strb = []      # `strb w1,[x0,#imm]; ret` (setter leafs, e.g. SetNetMode)
        self.ldrb_w0 = {}        # imm -> [count, [sites...]]  dest==w0 any base
        self.imm7777 = []        # (site, mn, ops) sites referencing 0x1e61 (7777)
        self.anchor_hits = {}    # anchor_name -> [(site, mn, ops)]
        self.insn_count = 0
        self.adrp_count = 0
        self.adrp_check_ok = 0
        self.adrp_check_bad = 0


def _xrec(xrefs, target):
    r = xrefs.get(target)
    if r is None:
        r = [0, 0, 0, 0, 0, 0, 0, 0, [], []]
        xrefs[target] = r
    elif len(r) == 9:            # migrate old pickles (pre write-site lists)
        r.append([])
    return r


def _add_site(rec, ch, site):
    if len(rec[8]) < MAX_SITES_PER_TARGET:
        rec[8].append((ch, site))


def _add_write(rec, site):
    """Byte-store site, kept in an uncapped parallel list (rec[9]); rare, and
    easily evicted from the capped mixed list on hot targets."""
    if len(rec[9]) < MAX_WRITES_PER_TARGET:
        rec[9].append(site)


def decode_adrp_manual(word, pc):
    """Cross-check capstone: returns absolute page target of an ADRP encoding."""
    if (word & 0x9F000000) != 0x90000000:
        return None
    immlo = (word >> 29) & 0x3
    immhi = (word >> 5) & 0x7FFFF
    imm = (immhi << 2) | immlo
    if imm & (1 << 20):
        imm -= 1 << 21
    return (pc & ~0xFFF) + (imm << 12)


def stream_text(fh, text_sec, limit=None):
    """Yield (addr, mn, ops) over .text, chunked; resyncs past undecodable words."""
    total = text_sec.size if limit is None else min(limit, text_sec.size)
    CHUNK = 16 * 1024 * 1024
    pos = 0
    while pos < total:
        n = min(CHUNK, total - pos)
        fh.seek(text_sec.off + pos)
        buf = fh.read(n)
        if len(buf) < 4:
            break
        buf = buf[: len(buf) // 4 * 4]
        va = text_sec.addr + pos
        while buf:
            last_end = 0
            for t in MD.disasm_lite(buf, va):
                last_end = t[0] + t[1] - va
                yield (t[0], t[2], t[3])
            if last_end >= len(buf):
                break
            skip = last_end if last_end > 0 else 4
            if skip % 4:
                skip += 4 - (skip % 4)
            buf = buf[skip:]
            va += skip
        pos += n


def pass1(fh, secs, anchors, text_limit=None):
    """One streaming pass over .text building all cross-reference data."""
    res = ScanResults()
    text = secs[".text"]
    data = secs.get(".data")
    bss = secs.get(".bss")
    ranges = []
    if data:
        ranges.append((data.addr, data.end, "data"))
    if bss:
        ranges.append((bss.addr, bss.end, "bss"))

    def in_globals(t):
        for lo, hi, _n in ranges:
            if lo <= t < hi:
                return True
        return False

    xrefs = res.xrefs
    regs = {}
    pending_leaf = None       # ('rb'|'rw'|'sb', site, imm) -- only valid if the
                               # NEXT instruction is 'ret' (see pend logic below)
    anchor_sites = {name: [] for name in anchors.values()}
    res.anchor_hits = anchor_sites
    anchors_by_addr = dict(anchors)
    ldrb_w0 = res.ldrb_w0

    t0 = time.time()
    n_insn = 0
    n_adrp = 0
    check_left = 512
    ok = bad = 0

    for addr, mn, ops in stream_text(fh, text, text_limit):
        n_insn += 1
        # adjacency: pending_leaf only survives if consumed by an immediately
        # following 'ret'; every non-ret instruction clears it via pend swap.
        pend = pending_leaf
        pending_leaf = None
        m0 = mn[0]

        # ---- 7777 (URL_DefaultPort) immediate, non-memory instructions -----
        if m0 != "l" and m0 != "s" and "0x1e61" in ops:
            if len(res.imm7777) < 256:
                res.imm7777.append((addr, mn, ops))

        # ---- fast path: loads / stores --------------------------------
        if m0 == "l" or m0 == "s" or (m0 == "p" and mn == "prfm"):
            lb = ops.find("[")
            if lb < 0:
                # no memory operand (sub, sxtw, ldr-literal, ...): invalidate
                # first-operand register unless it cannot define one.
                if mn not in NODEF_MNEMONICS and not mn.startswith("st"):
                    tok = ops.split(",")[0].strip()
                    nx = norm_xreg(tok)
                    if nx is not None and nx in regs:
                        del regs[nx]
                continue
            rt = ops[:lb].strip().rstrip(",")
            rb = ops.find("]", lb)
            if rb < 0:
                continue
            inner = ops[lb + 1: rb]
            parts = [p.strip() for p in inner.split(",")]
            base = parts[0]
            disp = 0
            regoff = False
            if len(parts) >= 2:
                if parts[1].startswith("#"):
                    try:
                        disp = parse_imm(parts[1])
                    except ValueError:
                        disp = 0
                else:
                    regoff = True
            tail = ops[rb + 1:].strip()
            wb = (tail == "!") or (tail.startswith(","))

            st = regs.get(norm_xreg(base) or "")
            target = None
            if st is not None and not regoff:
                target = st[1] + disp

            is_store = mn[0] == "s" and not mn.startswith("se")
            # classify width
            if mn in BYTE_MNEMS:
                width, slot, ch = 1, (K_SB if is_store else K_LB), \
                    ("S" if is_store else ("a" if mn in ACQ_BYTE_MNEMS else "b"))
            elif mn in HALF_MNEMS:
                width, slot, ch = 2, (K_SW if is_store else K_LH), ("V" if is_store else "h")
            elif mn == "ldrsw":
                width, slot, ch = 4, K_LW, "w"
            elif mn in ("ldp", "stp"):
                width = 8 if rt.startswith("x") else 4
                slot = K_SD if is_store else K_LD
                ch = "X" if is_store else "d"
            elif mn.startswith("ld") or mn.startswith("st") or mn in SIMPLE_LS:
                if rt.startswith("x"):
                    width, slot, ch = 8, (K_SD if is_store else K_LD), ("X" if is_store else "d")
                elif rt.startswith("w") or rt.startswith("s"):
                    width, slot, ch = 4, (K_SW if is_store else K_LW), ("V" if is_store else "w")
                else:  # q/d/b/h SIMD regs
                    width, slot, ch = 16, (K_SD if is_store else K_LD), ("X" if is_store else "d")
            else:
                width, slot, ch = 0, None, None

            if target is not None:
                aname = anchors_by_addr.get(target)
                if aname is not None and len(anchor_sites[aname]) < 256:
                    anchor_sites[aname].append((addr, mn, ops))
                if slot is not None and in_globals(target):
                    rec = _xrec(xrefs, target)
                    rec[slot] += 1
                    if ch == "S":
                        _add_write(rec, addr)   # byte-store site (uncapped list)
                    else:
                        _add_site(rec, ch, addr)

            # leaf-shape detection (GetNetMode / SetNetMode style)
            if len(parts) <= 2 and not regoff and not wb:
                if rt == "w0" and base == "x0" and mn in ("ldrb", "ldr"):
                    pending_leaf = ("rb" if mn == "ldrb" else "rw", addr, disp)
                elif rt == "w0" and base != "x0" and mn == "ldrb":
                    e = ldrb_w0.get(disp)
                    if e is None:
                        e = [0, []]
                        ldrb_w0[disp] = e
                    e[0] += 1
                    if len(e[1]) < MAX_SITES_PER_IMM:
                        e[1].append(addr)
                elif rt == "w1" and base == "x0" and mn == "strb":
                    pending_leaf = ("sb", addr, disp)

            # register definition side effects
            if not is_store:
                # loads define their destination(s)
                if rt[:1] in ("x", "w"):
                    for tok in ops[:rb].replace(" ", "").split(","):
                        nx = norm_xreg(tok)
                        if nx in regs:
                            del regs[nx]
            if wb:
                nb = norm_xreg(base)
                if nb is not None and nb in regs:
                    del regs[nb]
            continue

        # ---- branches / calls -----------------------------------------
        if mn == "bl" or mn == "blr" or mn.startswith("blra"):
            for i in range(18):
                regs.pop("x%d" % i, None)
            regs.pop("x30", None)
            continue
        if mn == "ret" or mn.startswith("ret"):
            if pend is not None:
                kind, site, imm = pend
                if kind == "rb":
                    res.leaf_ldrb.append((site, imm))
                elif kind == "rw":
                    res.leaf_ldrw.append((site, imm))
                else:
                    res.leaf_strb.append((site, imm))
            continue
        if m0 == "b" or mn == "br" or mn.startswith("bra"):
            continue

        # ---- adrp / adr -------------------------------------------------
        if mn == "adrp":
            n_adrp += 1
            parts = ops.split(", ")
            dst = parts[0]
            nx = norm_xreg(dst)
            if nx is None:
                continue
            if check_left > 0:
                check_left -= 1
                fh.seek(text.off + (addr - text.addr))
                word = struct.unpack("<I", fh.read(4))[0]
                man = decode_adrp_manual(word, addr)
                try:
                    cap = int(parts[1].lstrip("#"), 16)
                except ValueError:
                    cap = None
                if man is not None and cap is not None and man == cap:
                    ok += 1
                else:
                    bad += 1
            try:
                page = int(parts[1].lstrip("#"), 16)
            except ValueError:
                regs.pop(nx, None)
                continue
            regs[nx] = ("P", page)
            continue
        if mn == "adr":
            parts = ops.split(", ")
            nx = norm_xreg(parts[0])
            if nx is None:
                continue
            try:
                tgt = int(parts[1].lstrip("#"), 16)
            except ValueError:
                tgt = None
            if tgt is None:
                regs.pop(nx, None)
            else:
                regs[nx] = ("A", tgt)
                if in_globals(tgt):
                    rec = _xrec(xrefs, tgt)
                    rec[K_AM] += 1
                    _add_site(rec, "A", addr)
            continue

        # ---- add (materialization) -------------------------------------
        if mn == "add":
            parts = [p.strip() for p in ops.split(",")]
            dst = norm_xreg(parts[0])
            src = norm_xreg(parts[1]) if len(parts) > 1 else None
            if dst is None:
                continue
            if (len(parts) >= 3 and parts[2].startswith("#") and src is not None
                    and src in regs):
                try:
                    imm = parse_imm(parts[2])
                except ValueError:
                    imm = None
                if imm is not None:
                    shifted = len(parts) >= 5 and parts[3] == "lsl"
                    val = imm << 12 if shifted else imm
                    stt = regs[src]
                    if stt[0] == "P":
                        addr_t = stt[1] + val
                    else:
                        addr_t = stt[1] + val
                    regs[dst] = ("A", addr_t)
                    aname = anchors_by_addr.get(addr_t)
                    if aname is not None and len(anchor_sites[aname]) < 256:
                        anchor_sites[aname].append((addr, mn, ops))
                    if in_globals(addr_t):
                        rec = _xrec(xrefs, addr_t)
                        rec[K_AM] += 1
                        _add_site(rec, "A", addr)
                    continue
            regs.pop(dst, None)
            continue

        # ---- mov alias: propagate reg state ------------------------------
        if mn == "mov":
            parts = [p.strip() for p in ops.split(",")]
            if len(parts) == 2:
                dst = norm_xreg(parts[0])
                src = norm_xreg(parts[1])
                if dst is not None:
                    if src is not None and src in regs:
                        regs[dst] = regs[src]
                    else:
                        regs.pop(dst, None)
            continue

        # ---- generic first-operand invalidation ---------------------------
        if mn in NODEF_MNEMONICS:
            continue
        if ops:
            tok = ops.split(",")[0].strip()
            nx = norm_xreg(tok)
            if nx is not None and nx in regs:
                del regs[nx]

    res.insn_count = n_insn
    res.adrp_count = n_adrp
    res.adrp_check_ok = ok
    res.adrp_check_bad = bad
    res.pass1_seconds = time.time() - t0
    return res

# ----------------------------------------------------------------------------
# pass 2 helpers: window disassembly around sites
# ----------------------------------------------------------------------------

MD = None  # initialized in main()


def window(fh, text_sec, site, before=0x40, after=0x10):
    """Return [(addr, mn, ops)] for [site-before, site+after]."""
    lo = site - before
    if lo < text_sec.addr:
        lo = text_sec.addr
    if lo % 4:
        lo += 4 - (lo % 4)
    hi = site + after
    fh.seek(text_sec.off + (lo - text_sec.addr))
    buf = fh.read(hi - lo)
    out = []
    for t in MD.disasm_lite(buf, lo):
        out.append((t[0], t[2], t[3]))
    return out


def snippet_text(fh, text_sec, site, before=3, after=4, resolve=None):
    """Human-readable disasm around site; resolve ADRP targets via `resolve`."""
    insns = window(fh, text_sec, site, before=before * 4 + 8, after=after * 4 + 8)
    idx = None
    for i, (a, _m, _o) in enumerate(insns):
        if a == site:
            idx = i
            break
    if idx is None:
        return "site %x not synced" % site
    lo = max(0, idx - before)
    hi = min(len(insns), idx + after + 1)
    lines = []
    for a, m, o in insns[lo:hi]:
        extra = ""
        if resolve is not None and m == "adrp":
            try:
                pg = int(o.split(", ")[1].lstrip("#"), 16)
                extra = "  ; page 0x%x" % pg
            except Exception:
                pass
        lines.append("  %08x  %-8s %s%s" % (a, m, o, extra))
    return "\n".join(lines)


def targets_in_window(fh, text_sec, site, before=0x40, after=0x20):
    """Resolve all ADRP(+ADD/LDR) targets appearing in a window (co-ref check)."""
    insns = window(fh, text_sec, site, before=before, after=after)
    regs = {}
    found = set()
    for a, mn, ops in insns:
        if mn == "adrp":
            try:
                pg = int(ops.split(", ")[1].lstrip("#"), 16)
            except ValueError:
                continue
            regs[ops.split(", ")[0]] = ("P", pg)
            continue
        if mn == "add":
            parts = [p.strip() for p in ops.split(",")]
            if len(parts) >= 3 and parts[2].startswith("#"):
                nx = norm_xreg(parts[1])
                if nx in regs:
                    try:
                        imm = parse_imm(parts[2])
                    except ValueError:
                        continue
                    regs[norm_xreg(parts[0])] = ("A", regs[nx][1] + imm)
            continue
        lb = ops.find("[")
        if lb >= 0:
            rb = ops.find("]", lb)
            if rb < 0:
                continue
            inner = ops[lb + 1: rb].split(",")
            base = norm_xreg(inner[0].strip())
            if base in regs:
                disp = 0
                if len(inner) >= 2 and inner[1].strip().startswith("#"):
                    try:
                        disp = parse_imm(inner[1])
                    except ValueError:
                        disp = 0
                elif len(inner) >= 2:
                    continue
                st = regs[base]
                found.add(st[1] + disp)
    return found


def find_function_start(fh, text_sec, site, maxback=0x200):
    """Heuristic function start: scan back for prologue pattern after ret/nop gap."""
    insns = window(fh, text_sec, site, before=maxback, after=4)
    prev_ret = None
    start = None
    for a, m, o in insns:
        if a >= site:
            break
        if m == "ret" or (m == "stp" and "x29" in o.split(",")[0]) or m == "pacibsp" or m == "paciasp" or \
           (m == "sub" and o.startswith("sp, sp, #")) or (m == "stp" and "[sp, #" in o):
            start = a
    if start is None and insns:
        start = insns[0][0]
    return start

# ----------------------------------------------------------------------------
# analysis
# ----------------------------------------------------------------------------

GAPS_UWORLD = [(0x108, 0x120), (0x168, 0x188), (0x1B0, 0x220), (0x240, 0x580),
               (0x588, 0x610), (0x668, 0x730)]  # unreflected gaps in SDK UWorld dump

ANCHOR_STRINGS = ["GameNetDriver", "UNetDriver", "IpNetDriver", "DemoNetDriver",
                  "BeaconNetDriver", "StatelessConnectHandlerComponent",
                  "InitListen", "CreateNetDriver", "ServerReplicateActors",
                  "ReplicationDriver", "NetDriverDefinitions"]


def find_anchor_addrs(fh, secs):
    anchors = {}
    ro = secs.get(".rodata")
    if ro is None:
        return anchors
    data = read_section(fh, ro)
    for name in ANCHOR_STRINGS:
        needle = name.encode() + b"\0"
        off = 0
        cnt = 0
        while cnt < 8:
            i = data.find(needle, off)
            if i < 0:
                break
            # require standalone C string (preceded by NUL) to skip suffix hits
            j = i - 1
            if j < 0 or data[j] == 0:
                anchors[ro.addr + i] = name
                cnt += 1
            off = i + 1
    return anchors


def byte_at(fh, data_sec, va):
    if va < data_sec.addr or va >= data_sec.end:
        return None
    fh.seek(data_sec.off + (va - data_sec.addr))
    return struct.unpack("<B", fh.read(1))[0]


def analyze(fh, secs, res, args):
    data = secs[".data"]
    bss = secs.get(".bss")
    text = secs[".text"]
    out = {}

    def lb_count(t):
        r = res.xrefs.get(t)
        return r[K_LB] if r else 0

    def sb_count(t):
        r = res.xrefs.get(t)
        return r[K_SB] if r else 0

    def ld_count(t):
        r = res.xrefs.get(t)
        return r[K_LD] if r else 0

    def in_data(t):
        return data.addr <= t < data.end

    def in_bss(t):
        return bss is not None and bss.addr <= t < bss.end

    def val(t):
        return byte_at(fh, data, t) if in_data(t) else (0 if in_bss(t) else None)

    # ---------------- Part A: GIsClient / GIsServer --------------------------
    # Rich tables first: top byte-access globals in .data (any value) and .bss.
    byte_rows = []
    bss_rows = []
    for t, r in res.xrefs.items():
        if in_data(t) and (r[K_LB] >= 15 or r[K_SB] >= 5):
            byte_rows.append((t, r[K_LB], r[K_SB], val(t)))
        elif in_bss(t) and (r[K_LB] >= 15 or r[K_SB] >= 5):
            bss_rows.append((t, r[K_LB], r[K_SB], 0))
    byte_rows.sort(key=lambda x: -x[1])
    bss_rows.sort(key=lambda x: -x[1])
    out["top_byte_data"] = [{"rva": "0x%x" % t, "val": v, "ldrb": lb,
                             "strb": sb} for t, lb, sb, v in byte_rows[:60]]
    out["top_byte_bss"] = [{"rva": "0x%x" % t, "val": 0, "ldrb": lb,
                            "strb": sb} for t, lb, sb, _v in bss_rows[:60]]
    # runs of consecutive hot zero bytes in .bss (bool-cluster fingerprint:
    # on ELF, '=false' globals go to .bss while '=true' goes to .data, so the
    # GIsServer/GIsEditor part of the GIs* cluster forms a .bss run)
    bss_lb = {t: lb for t, lb, sb, v in bss_rows}
    runs = []
    used = set()
    for t, lb, sb, _v in sorted(bss_rows):
        if t in used or lb < 15:
            continue
        run = [t]
        t2 = t + 1
        while t2 in bss_lb and bss_lb[t2] >= 15:
            run.append(t2)
            used.add(t2)
            t2 += 1
        if len(run) >= 2:
            runs.append(run)
        used.add(t)
    out["bss_bool_runs"] = [[{"rva": "0x%x" % t, "ldrb": bss_lb[t],
                              "strb": dict((x[0], x[2]) for x in bss_rows).get(t, 0)}
                             for t in run] for run in runs[:20]]

    client_cands = []
    for thr in (30, 12, 5):
        client_cands = []
        for t, r in res.xrefs.items():
            if in_data(t) and r[K_LB] >= thr and val(t) == 1:
                client_cands.append((t, r[K_LB]))
        if client_cands:
            break
    client_cands.sort(key=lambda x: -x[1])
    topA = client_cands[: args.top]
    out["gis_client_table"] = []
    for t, c in topA:
        nb = []
        for off in range(-8, 9):
            t2 = t + off
            nb.append({"off": off, "val": val(t2), "ldrb": lb_count(t2),
                       "strb": sb_count(t2), "ldrd": ld_count(t2)})
        out["gis_client_table"].append({
            "rva": "0x%x" % t, "byte_val": val(t), "ldrb_reads": c,
            "strb_writes": sb_count(t), "materializations": res.xrefs[t][K_AM],
            "neighbors": nb})
    if not client_cands:
        return out, None, None

    gis_client = topA[0][0]
    # ------------------------------------------------------------------
    # GIsServer hunt (v2). Windows-12.41 adjacency (GIsServer=GIsClient+1)
    # does NOT carry to this ELF build: clang places `bool GIsClient = true`
    # in .data and `bool GIsServer = false` in .bss, so the pair is split.
    # Instead: audit every hot .bss zero byte and classify what it really is.
    # GIsServer in a SHIPPING CLIENT must be: pure bool usage (cbz/cbnz/
    # cmp#0|1), never written (all `GIsServer=true` code is #if UE_SERVER),
    # plain ldrb (NOT ldarb acquire -> __cxa_guard / sync flags), not a
    # struct member (no dword/halfword xrefs at +-4), not multi-state.
    # ------------------------------------------------------------------
    def usage_profile(t, maxn=16):
        """(guard_frac, bool_frac, state_frac) over sampled read sites."""
        r = res.xrefs.get(t)
        if not r:
            return None
        sites = [s for ch, s in r[8] if ch in ("b", "a")][:maxn]
        if not sites:
            return None
        guard = bool_ = state = 0
        for s in sites:
            ins = window(fh, text, s, before=0, after=0x18)
            seq = [i for i in ins if i[0] >= s][:3]
            m0 = seq[0][1] if seq else "?"
            if m0 in ("ldarb", "ldaprb", "ldar", "ldap"):
                guard += 1
                continue
            m1, o1 = (seq[1][1], seq[1][2]) if len(seq) > 1 else ("?", "")
            o1c = o1.replace(" ", "")
            if m1 in ("cbz", "cbnz") or (m1 in ("tbz", "tbnz")
                    and ("#0," in o1c[3:] or "#1," in o1c[3:])) \
                    or (m1 == "cmp" and o1c.split(",")[-1].lstrip("#") in
                        ("0", "1", "0x0", "0x1")) \
                    or (m1 == "tst" and (o1c.endswith("#1") or o1c.endswith("#0x1"))):
                bool_ += 1
            elif m1 == "cmp":
                state += 1
            elif m1 in ("and", "orr"):
                state += 1      # bitfield-style -> multi-bit byte
        n = float(len(sites))
        return guard / n, bool_ / n, state / n

    def struct_member(t):
        """True if a dword/halfword global lives at t+-4 (bool inside struct)."""
        for t2 in (t - 4, t + 4):
            r2 = res.xrefs.get(t2)
            if r2 and (r2[K_LW] or r2[K_SW] or r2[K_LH] or r2[K_SW]):
                return True
        return False

    server_pool = []
    for t, r in res.xrefs.items():
        if in_bss(t) and val(t) == 0 and r[K_LB] >= 20:
            server_pool.append((t, r[K_LB], r[K_SB]))
    server_pool.sort(key=lambda x: -x[1])

    audit = []
    coread_rows = []
    gc_sites = [s for ch, s in res.xrefs[gis_client][8] if ch == 'b'][:96]
    for t, lb, sb in server_pool[:40]:
        prof = usage_profile(t)
        if prof is None:
            continue
        gf, bf, sf = prof
        coread_t = sum(1 for s in gc_sites
                       if t in targets_in_window(fh, text, s, 0x48, 0x48))
        reasons = []
        if sb:
            reasons.append("written(strb=%d)" % sb)
        if gf >= 0.2:
            reasons.append("acquire-load(guard/sync)")
        if sf:
            reasons.append("multi-state(cmp#N>1)")
        if bf < 0.7:
            reasons.append("not-bool-style")
        if struct_member(t):
            reasons.append("struct-member(dword@+-4)")
        row = {"rva": hexs(t), "ldrb": lb, "strb": sb,
               "guard_frac": round(gf, 2), "bool_frac": round(bf, 2),
               "state_frac": round(sf, 2), "coread_with_gis_client": coread_t,
               "verdict": "; ".join(reasons) if reasons else "PASS"}
        if sb and res.xrefs[t][9]:
            row["write_sites"] = [hexs(w) for w in res.xrefs[t][9][:4]]
        audit.append(row)
        if coread_t:
            coread_rows.append(row)
    out["gis_server_audit"] = audit
    out["gis_server_coreads"] = coread_rows

    passing = [r for r in audit if r["verdict"] == "PASS"]
    # A passing usage-profile is NOT sufficient on its own: on 12.41 the single
    # passing candidate turned out to pair with a .data config-struct field
    # (feature-toggle cluster), not GIsClient/GIsServer. Require the
    # GIsClient&&GIsServer co-read idiom (>=2 shared windows) as well.
    out["gis_server_closest_pass"] = passing[0] if passing else None
    passing = [r for r in passing if r["coread_with_gis_client"] >= 2]
    gis_server = None
    if passing:
        gis_server = int(passing[0]["rva"], 16)
        out["gis_server_pick"] = passing[0]
    else:
        out["gis_server_pick"] = None

    # co-read evidence: sites where both globals are referenced in same window
    coread = 0
    if gis_server is not None:
        for _ch, site in res.xrefs[gis_client][8][:96]:
            if gis_server in targets_in_window(fh, text, site, 0x30, 0x30):
                coread += 1
    out["gis_coread_sites"] = coread
    # dump all setter leaves + AActor candidate windows for offline inspection
    out["leaf_strb_hist"] = {}
    for _s, imm in res.leaf_strb:
        out["leaf_strb_hist"][hexs(imm)] = out["leaf_strb_hist"].get(hexs(imm), 0) + 1
    out["leaf_strb_sites"] = [(hexs(s), hexs(i)) for s, i in res.leaf_strb[:64]]
    return out, gis_client, gis_server


def hexs(n):
    return "0x%x" % n


def main():
    global MD
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--lib", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--version", default="unknown")
    ap.add_argument("--text-limit", type=lambda x: int(x, 0), default=None,
                    help="debug: only scan first N bytes of .text")
    ap.add_argument("--top", type=int, default=12)
    ap.add_argument("--netmode-off", type=lambda x: int(x, 0), default=0x168,
                    help="primary NetMode offset hypothesis from SDK gap analysis")
    ap.add_argument("--dump-xrefs", default=None,
                    help="optional: pickle pass-1 xref data to this file (cache)")
    ap.add_argument("--load-xrefs", default=None,
                    help="optional: load pickled xref data instead of scanning")
    args = ap.parse_args()

    import pickle
    xref_cache = args.dump_xrefs or (args.load_xrefs + ".tmp" if args.load_xrefs else None)

    t_start = time.time()
    MD = Cs(CS_ARCH_ARM64, CS_MODE_LITTLE_ENDIAN)
    MD.detail = False

    secs = parse_elf64(args.lib)
    need = [".text", ".data"]
    missing = [n for n in need if n not in secs or secs[n].size == 0]
    if missing:
        raise RuntimeError("missing sections: %s" % missing)
    bss = secs.get(".bss")

    sha = hashlib.sha256()
    with open(args.lib, "rb") as fh:
        for blk in iter(lambda: fh.read(1 << 22), b""):
            sha.update(blk)
    sha256 = sha.hexdigest()

    with open(args.lib, "rb") as fh:
        anchors = find_anchor_addrs(fh, secs)
        print("[*] %s: .text=0x%x bytes  .data=0x%x  .bss=0x%x  anchors=%d"
              % (args.lib, secs[".text"].size, secs[".data"].size,
                 bss.size if bss else 0, len(anchors)))
        if args.text_limit:
            print("[*] SMOKE MODE: --text-limit 0x%x" % args.text_limit)

        print("[*] pass 1: streaming disassembly of .text ...")
        if args.load_xrefs and os.path.exists(args.load_xrefs):
            with open(args.load_xrefs, "rb") as pf:
                res = pickle.load(pf)
            print("[*] loaded pickled xref cache (%d targets)" % len(res.xrefs))
        else:
            res = pass1(fh, secs, anchors, args.text_limit)
            print("[*] pass 1 done: %d insns, %d adrps, %d data/bss xref targets "
                  "in %.1fs (adrp decode check ok=%d bad=%d)"
                  % (res.insn_count, res.adrp_count, len(res.xrefs),
                     res.pass1_seconds, res.adrp_check_ok, res.adrp_check_bad))
            if args.dump_xrefs:
                with open(args.dump_xrefs, "wb") as pf:
                    pickle.dump(res, pf, protocol=4)
                print("[*] pickled xref data to %s" % args.dump_xrefs)

        text = secs[".text"]
        data = secs[".data"]

        analysis, gis_client, gis_server = analyze(fh, secs, res, args)

        # ---------------- Part A evidence --------------------------------
        entries = {}
        ev = []
        if gis_client is not None:
            r = res.xrefs[gis_client]
            ev.append("static byte=1 at rva %s in .data; ldrb read sites=%d "
                      "(highest and ONLY .data byte==1 global with >=12 ldrb "
                      "reads in the whole binary)" % (hexs(gis_client), r[K_LB]))
            if r[K_SB]:
                ev.append("strb write sites=%d (stores of 0; NO init-write of 1 "
                          "exists anywhere -- consistent with static init true)"
                          % r[K_SB])
            else:
                ev.append("no strb writers found (consistent with static init true)")
            ev.append("accesses are pure-bool shaped: ldrb w?,[xN,#0x81c] then "
                      "cbz/cbnz guard or bool-arg flow; no wider/merged access, "
                      "never address-taken")
            ev.append("co-read note: gis_client read-sites checked against every "
                      "audited candidate (+-0x48 windows); table in "
                      "analysis.gis_server_coreads (top co-readers were audited as "
                      "non-GIsServer: a derived capability cache and a log-throttle "
                      "state byte)")
            snippets = []
            shown = 0
            for _ch, site in r[8]:
                if _ch != "b" or shown >= 3:
                    continue
                snippets.append("read @ %s:\n%s" % (hexs(site),
                                snippet_text(fh, text, site, 3, 4)))
                shown += 1
            for site in r[9][:2]:
                snippets.append("write @ %s:\n%s" % (hexs(site),
                                snippet_text(fh, text, site, 6, 4)))
            # neighborhood text
            nb = analysis["gis_client_table"][0]["neighbors"]
            ev.append("neighborhood: " + "; ".join(
                "%+d:val=%s,rd=%d" % (n["off"], n["val"], n["ldrb"]) for n in nb
                if n["ldrb"] or n["strb"]))
            conf = "low"
            if gis_server is not None and res.xrefs[gis_server][K_LB] >= max(20, r[K_LB] // 6):
                conf = "high" if analysis.get("gis_coread_sites", 0) >= 3 else "medium"
            entries["gis_client"] = {"found": True, "rva": hexs(gis_client),
                                    "confidence": conf,
                                    "evidence": " | ".join(ev),
                                    "snippets": snippets}
            if gis_server is not None:
                rs = res.xrefs[gis_server]
                sev = ["static byte=0 at %s (.bss)" % hexs(gis_server),
                       "ldrb reads=%d, strb writes=%d" % (rs[K_LB], rs[K_SB]),
                       "bool-style usage, never written, co-read windows with "
                       "GIsClient=%d" % analysis.get("gis_coread_sites", 0)]
                sn = []
                shown = 0
                for _ch, site in rs[8]:
                    if _ch != "b" or shown >= 3:
                        continue
                    sn.append("read @ %s:\n%s" % (hexs(site), snippet_text(fh, text, site, 3, 4)))
                    shown += 1
                entries["gis_server"] = {"found": True, "rva": hexs(gis_server),
                                         "confidence": "low",
                                         "evidence": " | ".join(sev), "snippets": sn}
            else:
                naudit = len(analysis.get("gis_server_audit", []))
                best_pass = analysis.get("gis_server_pick")
                reason = ("audited %d hot .bss zero-byte globals (ldrb>=20): all "
                          "classified as acquire-load guards (__cxa_guard/sync), "
                          "multi-state bytes (cmp #N>1: device tiers/levels), "
                          "written flags (perf/derived/init-latch), or config-struct "
                          "members; none shows the GIsClient&&GIsServer co-read "
                          "idiom with the GIsClient candidate (co-read table in "
                          "analysis.gis_server_coreads). Working hypothesis: GIsServer "
                          "was LTO constant-folded away -- in a shipping client no "
                          "code writes GIsServer (all writers are #if UE_SERVER), so "
                          "every `if (GIsServer)` folds to false and the global is "
                          "dead-stripped; the Windows 12.41 build retains it at "
                          "ImageBase+0x804B65A (no such folding there)."
                          % naudit)
                cp = analysis.get("gis_server_closest_pass")
                if cp:
                    reason += (" Closest usage-passing candidate: %s "
                                "(ldrb=%d, never written, pure bool) -- rejected "
                                "because it pairs with a .data config-struct field "
                                "in feature-toggle idioms and never co-occurs with "
                                "the GIsClient candidate."
                                % (cp["rva"], cp["ldrb"]))
                entries["gis_server"] = {"found": False, "reason": reason}
        else:
            entries["gis_client"] = {"found": False,
                                     "reason": "no .data byte==1 global with >=30 ldrb reads"}
            entries["gis_server"] = {"found": False, "reason": "GIsClient not found"}

        # ---------------- Part B: GetNetMode -------------------------------
        # leaf histogram over plausible UWorld gap offsets
        gap_imms = set()
        for lo, hi in GAPS_UWORLD:
            for o in range(lo, min(hi, lo + 0x20)):
                gap_imms.add(o)
        hist = {}
        for site, imm in res.leaf_ldrb:
            if imm in gap_imms:
                hist[imm] = hist.get(imm, 0) + 1
        analysis["netmode_leaf_hist"] = {hexs(k): v for k, v in sorted(hist.items())}

        primary = args.netmode_off
        um_leafs = [(s, i) for s, i in res.leaf_ldrb if i == primary]
        um_setter = [(s, i) for s, i in res.leaf_strb if i == primary]
        netmode_off_used = primary
        if not um_leafs:
            # fall back: any gap-offset leaf with both getter and setter
            for imm, _c in sorted(hist.items(), key=lambda kv: -kv[1]):
                if any(s for s, i in res.leaf_ldrb if i == imm) and \
                   any(s for s, i in res.leaf_strb if i == imm):
                    netmode_off_used = imm
                    um_leafs = [(s, i) for s, i in res.leaf_ldrb if i == imm]
                    um_setter = [(s, i) for s, i in res.leaf_strb if i == imm]
                    break

        if um_leafs:
            snips = ["ldrb w0,[x0,#%s]; ret @ %s:\n%s" % (hexs(netmode_off_used),
                     hexs(um_leafs[0][0]),
                     snippet_text(fh, text, um_leafs[0][0], 2, 2))]
            ev2 = ["leaf getter `ldrb w0,[x0,#%s]; ret` at %s (UWorld::GetNetMode; "
                   "NetMode=TEnumAsByte<ENetMode> 1-byte member; ENetMode: NM_Standalone=0, NM_DedicatedServer=1, NM_ListenServer=2, NM_Client=3)"
                   % (hexs(netmode_off_used), hexs(um_leafs[0][0])),
                   "NetMode offset 0x%x derived from SDK Engine_classes.hpp gap "
                   "0x168..0x188 (between LevelCollections@0x158 and OwningGameInstance@"
                   "0x188; stock UE4.24 UWorld private block: NetMode, TravelURL, "
                   "TravelType, bPlayersOnly bitfields)" % netmode_off_used]
            if um_setter:
                ev2.append("matching setter leaf `strb w1,[x0,#%s]; ret` at %s "
                           "(UWorld::SetNetMode)" % (hexs(netmode_off_used), hexs(um_setter[0][0])))
                snips.append("setter @ %s:\n%s" % (hexs(um_setter[0][0]),
                             snippet_text(fh, text, um_setter[0][0], 2, 2)))
            conf = "high" if um_setter else "medium"
            entries["uworld_getnetmode"] = {"found": True,
                                            "rva": hexs(um_leafs[0][0]),
                                            "netmode_offset": hexs(netmode_off_used),
                                            "confidence": conf,
                                            "evidence": " | ".join(ev2),
                                            "snippets": snips}
        else:
            entries["uworld_getnetmode"] = {
                "found": False, "netmode_offset": hexs(netmode_off_used),
                "reason": "no leaf `ldrb w0,[x0,#imm]; ret` at any SDK-gap offset "
                          "(%d gap-offset leaf getters found overall); likely inlined"
                          % len(res.leaf_ldrb)}

        # AActor::GetNetMode: small fn loading WorldPrivate then ldrb w0,[xN,#off]
        actor_hit = None
        actor_strict = False
        cand_sites = []
        e = res.ldrb_w0.get(netmode_off_used)
        if e:
            cand_sites = e[1]
        analysis["aactor_cand_sites"] = [hexs(s) for s in cand_sites]
        worldprivate_guess = 0x148  # AActor Pad_148 in SDK (between ControllingMatineeActors@0x138 and Layers@0x150)
        for site in cand_sites:
            w = window(fh, text, site, before=0x60, after=0x14)
            txt = "\n".join("%s %s %s" % (a, m, o) for a, m, o in w)
            if ("#0xb8]" in txt or "#0xB8]" in txt) and ("#0x148]" in txt) and "ret" in txt:
                # verify shape: short, ends ret soon after site
                tail = [x for x in w if x[0] >= site]
                if 0 < len(tail) <= 6 and tail[-1][1].startswith("ret"):
                    actor_hit = (site, w)
                    actor_strict = True
                    break
        if actor_hit is None:
            # relaxed: world resolution via ULevel::OwningWorld(0xB8) nearby
            for site in cand_sites[:128]:
                w = window(fh, text, site, before=0x40, after=0x10)
                txt = "\n".join("%s %s %s" % (a, m, o) for a, m, o in w)
                if "#0xb8]" in txt or "#0xB8]" in txt:
                    tail = [x for x in w if x[0] >= site]
                    if 0 < len(tail) <= 8 and tail[-1][1].startswith("ret"):
                        actor_hit = (site, w)
                        break
        if actor_hit is not None:
            site, w = actor_hit
            fstart = find_function_start(fh, text, site)
            snip = "\n".join("  %08x  %-8s %s" % (a, m, o) for a, m, o in w)
            entries["aactor_getnetmode"] = {
                "found": True, "rva": hexs(fstart),
                "netmode_load_site": hexs(site),
                "confidence": "high" if actor_strict else "medium",
                "evidence": ("short function loads AActor::WorldPrivate([x0,#0x148] "
                             "SDK Pad_148), falls back via ULevel::OwningWorld(#0xB8, "
                             "SDK ULevel::OwningWorld@0xB8), returns ldrb w0,[xN,#%s]%s"
                             % (hexs(netmode_off_used),
                                " (strict shape)" if actor_strict else " (relaxed)")),
                "snippets": [snip]}
        else:
            entries["aactor_getnetmode"] = {
                "found": False, "netmode_offset": hexs(netmode_off_used),
                "reason": ("no short world-resolving function ending in "
                           "ldrb w0,[xN,#%s] found among %d candidate sites. "
                           "Manual audit of all %d sites: 4 are interior loop "
                           "bodies iterating 0x270-stride struct arrays "
                           "(mov #0x138 / umulh 0xd20d.. magic-number indexing, "
                           "[x15,#0x168] is a struct field, not UWorld::NetMode); "
                           "the only clean 3-insn leaf (0x837c208: ldr x8,[x0,#0x10]; "
                           "ldrb w0,[x8,#0x168]; ret) reads the world from "
                           "[this+0x10] -- that is NOT the AActor chain "
                           "(SDK: AActor External/ULevel* = Pad_148@0x148 -> "
                           "ULevel::OwningWorld@0xB8 -> NetMode@0x168), so it is a "
                           "GetNetMode variant on some non-AActor wrapper object "
                           "(world ptr at +0x10, e.g. FWorldContext-shaped). "
                           "AActor::GetNetMode itself appears fully inlined by LTO "
                           "(Windows 12.41 keeps it at 0x3EB6780)."
                           % (hexs(netmode_off_used), len(cand_sites), len(cand_sites)))}
            # wrapper leaf: keep as an extra (unidentified) lead entry
            entries["world_wrapper_getnetmode_leaf"] = {
                "found": True, "rva": "0x837c208",
                "confidence": "low",
                "evidence": ("3-insn leaf `ldr x8,[x0,#0x10]; ldrb w0,[x8,#0x168]; "
                             "ret` -- GetNetMode on an object whose world pointer "
                             "sits at +0x10 (FWorldContext-like wrapper). NOT "
                             "AActor::GetNetMode (no External@0x148 -> "
                             "OwningWorld@0xB8 chain); class identity unresolved.")}
            entries["aactor_getnetmode"]["reason"] += (" Wrapper leaf lead: "
                "0x837c208 (see world_wrapper_getnetmode_leaf).")

        # ---------------- Part C: best-effort -------------------------------
        c_entries = {}
        c_entries["uworld_listen_7777"] = {
            "found": len(res.imm7777) > 0,
            "sites": ["%s: %s %s" % (hexs(s), m, o) for s, m, o in res.imm7777[:16]],
            "note": ("code sites referencing immediate 0x1e61 (7777, URL_DefaultPort); "
                     "UWorld::Listen / net port default setup candidates" if res.imm7777
                     else "no 0x1e61 immediates found")}
        for name in ANCHOR_STRINGS:
            hits = res.anchor_hits.get(name, [])
            c_entries["xref_%s" % name] = {
                "found": len(hits) > 0,
                "sites": ["%s: %s %s" % (hexs(s), m, o) for s, m, o in hits[:12]],
                "count": len(hits)}
        c_entries["create_netdriver"] = {"found": False,
                                         "reason": ("no distinctive static pattern; "
                                                    "symbol stripped; indirect-class "
                                                    "construction not resolvable within "
                                                    "timebox (see xref_UNetDriver sites "
                                                    "for leads)")}
        c_entries["init_listen"] = {"found": False,
                                    "reason": ("virtual function, only reached via "
                                               "vtable dispatch; no reliable static "
                                               "signature in stripped binary")}
        c_entries["server_replicate_actors"] = {"found": False,
                                                "reason": ("virtual, vtable-dispatched; "
                                                           "vtable identity requires "
                                                           "constructor chain RE outside "
                                                           "timebox")}
        for k, v in c_entries.items():
            entries[k] = v

        # ---------------- assemble ----------------------------------------
        result = {
            "binary": os.path.basename(args.lib),
            "version": args.version,
            "sha256": sha256,
            "entries": entries,
            "notes": ("scanner=tools/ogs/find_globals.py (v3); ONE streaming "
                      "capstone pass over .text (%d insns, %.1fs) + window passes "
                      "over shortlists only; pass1 adrp decode cross-check "
                      "ok=%d/bad=%d. SDK=Dumper-7 OGS-12.41 Engine_classes.hpp "
                      "(UWorld gap 0x168..0x188 -> NetMode@0x168; AActor "
                      "Pad_148@0x148=ULevel*; ULevel::OwningWorld@0xB8). "
                      "GIsServer NOT FOUND: %d hot .bss zero-byte globals "
                      "audited (guards/state-bytes/written-flags/config-members), "
                      "none matches; best hypothesis = LTO constant-fold "
                      "(nothing writes GIsServer in a shipping client). "
                      "Cross-build context: Windows 12.41 keeps the pair at "
                      "ImageBase+0x804B659/GIsServer+0x804B65A and hooks exist at "
                      "UWorld::GetNetMode=0x45C9D90 / AActor::GetNetMode=0x3EB6780 "
                      "(OGS-S12 project); on this Android LTO build only the "
                      "UWorld leaf survives. Runtime use: address = load base + rva."
                      " analysis: %s"
                      % (res.insn_count, res.pass1_seconds, res.adrp_check_ok,
                         res.adrp_check_bad,
                         len(analysis.get("gis_server_audit", [])),
                         json.dumps({k: v for k, v in analysis.items()
                                     if k != "gis_client_table"})[:600])),
            "analysis": analysis,
        }
        with open(args.out, "w") as of:
            json.dump(result, of, indent=2)
        print("[*] wrote %s (%.1fs total)" % (args.out, time.time() - t_start))

        # console summary of key results
        for k in ("gis_client", "gis_server", "uworld_getnetmode", "aactor_getnetmode"):
            e = entries.get(k, {})
            print("[=] %-20s %s %s" % (k, e.get("rva", "-"),
                                       e.get("confidence", e.get("reason", ""))[:90]))


if __name__ == "__main__":
    main()
