#!/usr/bin/env python3
"""MINE the knowledge base from GROUND TRUTH — per-version pass.

Usage:
  python3 mine_kb.py features <version>   # pass 1: fingerprint features
  python3 mine_kb.py validate <version>   # pass 2: Z3-blind validation of specs
  python3 mine_kb.py assemble             # final: assemble solver_kb.json

Ground truth = final_resolved_v3.json + catalog_v3.json.  One version is
loaded at a time (memory limits) and results accumulate as JSON files in
kb_stage/.
"""
import sys, os, json, time, glob
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from so_analysis import SoAnalysis
from z3verify import FastTextIndex, blind_search_fast, decode_features

VERSIONS = ["5.41", "7.40", "9.41", "13.40", "17.50", "18.40", "19.10",
            "21.30", "22.40", "23.50", "30.00"]
STAGE = "/home/z/repo/tools/solver/kb_stage"
os.makedirs(STAGE, exist_ok=True)

RESOLVED = json.load(open('/home/z/repo/erbium-finders/final_resolved_v3.json'))
CAT3 = json.load(open('/home/z/repo/tools/mega3/catalog_v3.json'))

EXTRA_NATIVES = {
    "FindClearAbility": ["ClearAbility"],
    "FindGiveAbility": ["K2_GiveAbility", "K2_GiveAbilityAndActivateOnce"],
    "FindGiveAbilityAndActivateOnce": ["K2_GiveAbilityAndActivateOnce", "K2_GiveAbility"],
    "FindKickPlayer": ["OnEndGameKickPlayers", "ServerKickPlayer", "ServerKickPlayer_2"],
    "FindSetWorld": ["K2_SetWorldLocation", "K2_SetWorldRotation", "K2_SetWorldScale"],
    "FindOnRep_ZiplineState": ["OnRep_ZiplineState"],
    "Inline_ToDeathCauseNative": ["ToDeathCause"],
    "Inline_OnDamageServerCaller": ["OnDamageServer"],
    "Inline_OnPawnEnterSeatFunc": ["OnPawnEnterSeat"],
    "Inline_K2_OnUnEquipCaller": ["K2_OnUnEquip"],
    "Inline_OnAuthorityRandomUpgradeAppliedCaller": ["OnAuthorityRandomUpgradeApplied"],
    "Inline_ClientOnPawnDiedCaller": ["ClientOnPawnDied"],
    "Inline_EquipVehicleWeaponFunc": ["EquipVehicleWeapon"],
}


def so_path(v):
    for nm in ("libUnreal.so", "libUE4.so"):
        p = f"/tmp/work2/src/{v}/{nm}"
        if os.path.exists(p):
            return p
    return None


def native_names_for(fn):
    e = CAT3.get(fn, {})
    names = list(e.get("natives", []) or [])
    names += EXTRA_NATIVES.get(fn, [])
    seen, out = set(), []
    for n in names:
        if n and n not in seen:
            seen.add(n)
            out.append(n)
    return out


def get_analyzer(v):
    t0 = time.time()
    a = SoAnalysis(so_path(v))
    print(f"  [{v}] loaded ({time.time()-t0:.1f}s, funcs={len(a.funcs)})", flush=True)
    return a


# ---------------------------------------------------------------- pass 1
def pass_features(v):
    a = get_analyzer(v)
    out = {}
    for fn, cells in RESOLVED.items():
        c = cells.get(v)
        if not (isinstance(c, dict) and c.get("addr")):
            continue
        addr = int(c["addr"], 16)
        off = a.va2off(addr)
        if off is None:
            continue
        sz = min(a.func_size(addr), 0x10000)
        if sz < 12:
            continue
        words = np.frombuffer(a.elf, dtype="<u4", count=sz // 4, offset=off)
        f = decode_features(words, addr)
        acc = set()
        for k in ("ldr64_imm", "str64_imm", "ldr32_imm", "str32_imm",
                  "ldrb_imm", "strb_imm", "ldrh_imm"):
            acc |= f[k]
        out[fn] = {
            "addr": hex(addr),
            "size": a.func_size(addr),
            "bl": f["bl"],
            "accessed": sorted(x for x in acc if x >= 0x18),
            "strings": sorted(a.strings_of(addr).keys()),
        }
    # vtable runs containing known functions (for relation mining)
    known = {fn: int(c["addr"], 16) for fn, c in
             ((fn, RESOLVED[fn].get(v)) for fn in RESOLVED)
             if isinstance(c, dict) and c.get("addr")}
    vt = {}
    for fn, addr in known.items():
        try:
            for run, idx in a.vtable_of_func(addr):
                vt[fn] = {"base": hex(run["base"]), "idx": idx, "len": len(run["slots"])}
                break
        except Exception:
            pass
    json.dump({"features": out, "vtables": vt},
              open(f"{STAGE}/feat_{v}.json", "w"))
    print(f"  [{v}] features: {len(out)} functions, vtable members: {len(vt)}", flush=True)


# ---------------------------------------------------------------- spec building
REF_VERSION = "21.30"   # rarity reference (mid-range)

def offset_rarity():
    """offset → number of functions accessing it on the reference binary.
    Rare offsets make discriminative fingerprints."""
    p = f"{STAGE}/rarity.json"
    if os.path.exists(p):
        return json.load(open(p))
    a = get_analyzer(REF_VERSION)
    ftx = FastTextIndex(a)
    # collect all offsets used by ground-truth functions' specs
    feats = {}
    for v in VERSIONS:
        fp = f"{STAGE}/feat_{v}.json"
        if os.path.exists(fp):
            feats.update(json.load(open(fp))["features"])
    cand = set()
    for f in feats.values():
        cand.update(f["accessed"])
    rarity = {}
    for o in sorted(cand):
        rarity[str(o)] = int(ftx.functions_with_offset(o).sum())
    json.dump(rarity, open(p, "w"))
    return rarity


def build_specs_from_features():
    """intersect per-version features → candidate specs (rarity-weighted)"""
    feats = {}
    for v in VERSIONS:
        p = f"{STAGE}/feat_{v}.json"
        if os.path.exists(p):
            d = json.load(open(p))
            feats[v] = d["features"]
    rarity = offset_rarity()
    specs = {}
    for fn in RESOLVED:
        per_ver = {v: d[fn] for v, d in feats.items() if fn in d}
        if not per_ver:
            continue
        acc_sets = [set(f["accessed"]) for f in per_ver.values()]
        inv = set.intersection(*acc_sets) if acc_sets else set()
        inv = {o for o in inv if o >= 0x18}
        sizes = [f["size"] for f in per_ver.values()]
        bls = [f["bl"] for f in per_ver.values()]
        if not inv:
            continue
        # rarity-weighted pick: rarest invariant offsets first (≤4)
        ranked = sorted(inv, key=lambda o: rarity.get(str(o), 10**9))
        chosen = []
        for o in ranked:
            if len(chosen) >= 4:
                break
            if rarity.get(str(o), 10**9) <= 3000:   # skip ultra-common
                chosen.append(o)
        if not chosen:                              # fallback: rarest anyway
            chosen = ranked[:2]
        spec = {
            "imm_access": sorted(chosen),
            "size_min": max(0x20, int(min(sizes) * 0.55)),
            "size_max": int(max(sizes) * 1.7) + 0x60,
            "bl_min": max(0, min(bls) - 3),
            "bl_max": max(bls) + 6,
            "n_gt_versions": len(per_ver),
        }
        strong = (len(per_ver) >= 3 and len(chosen) >= 2) or (len(chosen) >= 3)
        if strong:
            specs[fn] = spec
    return specs


# ---------------------------------------------------------------- pass 2
def pass_validate(v):
    spec_file = f"{STAGE}/specs.json"
    specs = json.load(open(spec_file)) if os.path.exists(spec_file) else build_specs_from_features()
    if not os.path.exists(spec_file):
        json.dump(specs, open(spec_file, "w"), indent=1)
    a = get_analyzer(v)
    ftx = FastTextIndex(a)
    out = {}
    for fn, spec in specs.items():
        c = RESOLVED.get(fn, {}).get(v)
        if not (isinstance(c, dict) and c.get("addr")):
            continue
        known = int(c["addr"], 16)
        core = {k: val for k, val in spec.items() if k != "n_gt_versions"}
        pre = int(ftx.prefilter_spec(core).sum())
        rec = {"known": hex(known), "prefilter": pre}
        # 1) ground truth must be Z3-SAT for this spec
        off = a.va2off(known)
        if off is not None and a.func_size(known) >= 12:
            words = np.frombuffer(a.elf, dtype="<u4",
                                  count=min(a.func_size(known), 0x10000) // 4, offset=off)
            ok, _proof = __import__("z3verify").z3_check_spec(core, words.tolist())
            rec["gt_sat"] = bool(ok)
        else:
            rec["gt_sat"] = False
        # 2) full blind search when cheap enough
        rec["found"] = False
        rec["unique"] = False
        rec["n_hits"] = None
        blind_max = int(os.environ.get("KB_BLIND_MAX", "300"))
        if pre <= blind_max and rec.get("gt_sat"):
            hits = blind_search_fast(a, ftx, core, max_results=6, collect_all=True)
            rec["found"] = any(h == known for h, _ in hits)
            rec["unique"] = len(hits) == 1 and hits[0][0] == known
            rec["n_hits"] = len(hits)
            rec["hits"] = [hex(h) for h, _ in hits]
        out[fn] = rec
    json.dump(out, open(f"{STAGE}/val_{v}.json", "w"), indent=1)
    ok = [r["found"] for r in out.values() if r["n_hits"] is not None]
    sat = [r["gt_sat"] for r in out.values()]
    print(f"  [{v}] validated {len(out)} specs — gt_sat {sum(sat)}/{len(sat)}, "
          f"blind-found {sum(ok)}/{len(ok)}", flush=True)


# ---------------------------------------------------------------- relations
def mine_relations():
    """vtable {anchor→target delta} across versions from staged features"""
    vt = {}
    for v in VERSIONS:
        p = f"{STAGE}/feat_{v}.json"
        if os.path.exists(p):
            vt[v] = json.load(open(p))["vtables"]
    pairs = {}
    for v, members in vt.items():
        by_base = {}
        for fn, info in members.items():
            by_base.setdefault(info["base"], []).append((fn, info["idx"]))
        for base, lst in by_base.items():
            if len(lst) < 2:
                continue
            for fa, ia in lst:
                for fb, ib in lst:
                    if fa >= fb:
                        continue
                    pairs.setdefault((fa, fb), {})[v] = ib - ia
    rel = []
    for (fa, fb), dv in pairs.items():
        vals = set(dv.values())
        if len(vals) == 1 and len(dv) >= 2:
            rel.append({"anchor": fa, "target": fb, "delta": vals.pop(),
                        "n_versions": len(dv)})
    rel.sort(key=lambda r: -r["n_versions"])
    return rel


# ---------------------------------------------------------------- assemble
def assemble():
    specs = json.load(open(f"{STAGE}/specs.json")) if os.path.exists(f"{STAGE}/specs.json") else {}
    rel = mine_relations()
    validations = {}
    for v in VERSIONS:
        p = f"{STAGE}/val_{v}.json"
        if os.path.exists(p):
            validations[v] = json.load(open(p))
    KB = {"targets": {}, "vtable_relations": rel,
          "generated": time.strftime("%Y-%m-%d %H:%M")}
    # spec quality per function: gt_sat on every validated version = transferable
    spec_ok, spec_uniq, spec_pre = {}, {}, {}
    for fn in specs:
        vals = [validations[v][fn] for v in validations if fn in validations[v]]
        if not vals:
            continue
        sat = [r["gt_sat"] for r in vals]
        spec_ok[fn] = all(sat)                     # correct on ALL gt versions
        spec_pre[fn] = sum(r["prefilter"] for r in vals) / len(vals)
        spec_uniq[fn] = all(r.get("unique") for r in vals if r.get("n_hits") is not None) and \
            any(r.get("n_hits") is not None for r in vals)
    for fn in RESOLVED:
        e = {
            "strings": CAT3.get(fn, {}).get("strings", []),
            "natives": native_names_for(fn),
            "kind": "global" if fn in ("FindGIsServer", "FindGIsClient", "FindGetNamePool") else "function",
        }
        if fn in specs and spec_ok.get(fn):
            e["z3_spec"] = {k: val for k, val in specs[fn].items() if k != "n_gt_versions"}
            e["z3_unique"] = spec_uniq.get(fn, False)
            e["z3_avg_prefilter"] = round(spec_pre.get(fn, -1))
            e["z3_usable_blind"] = spec_pre.get(fn, 10**9) < 500
            e["z3_validation"] = {v: {"found": validations[v][fn]["found"],
                                      "unique": validations[v][fn]["unique"]}
                                  for v in validations if fn in validations[v]}
        pats = {}
        for v, cell in RESOLVED[fn].items():
            if isinstance(cell, dict) and cell.get("pattern"):
                pats[v] = cell["pattern"]
        if pats:
            e["patterns"] = pats
        addrs = {}
        for v, cell in RESOLVED[fn].items():
            if isinstance(cell, dict):
                if cell.get("addr"):
                    addrs[v] = cell["addr"]
                elif cell.get("global"):
                    addrs[v] = "G:" + cell["global"]
                elif cell.get("vtable_index") is not None:
                    addrs[v] = f"VT:{cell['vtable_index']}"
        e["ground_truth"] = addrs
        KB["targets"][fn] = e
    out = "/home/z/repo/tools/solver/solver_kb.json"
    json.dump(KB, open(out, "w"), indent=1)
    nspec = sum(1 for fn, e in KB["targets"].items() if "z3_spec" in e)
    nuniq = sum(1 for fn, e in KB["targets"].items() if e.get("z3_unique"))
    print(f"KB assembled: {out}")
    print(f"  targets: {len(KB['targets'])}")
    print(f"  Z3 specs (100% re-find): {nspec}  (unique on all versions: {nuniq})")
    print(f"  vtable relations (stable ≥2 versions): {len(rel)}")
    for r in rel[:20]:
        print(f"    {r['target']} = {r['anchor']} %+d ({r['n_versions']} vers)")


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "features":
        pass_features(sys.argv[2])
    elif mode == "validate":
        pass_validate(sys.argv[2])
    elif mode == "specs":
        s = build_specs_from_features()
        json.dump(s, open(f"{STAGE}/specs.json", "w"), indent=1)
        print(f"specs built: {len(s)}")
    elif mode == "assemble":
        assemble()
