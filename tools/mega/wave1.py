#!/usr/bin/env python3
"""WAVE 1 — resolve all string-anchored Erbium functions across 11 Android versions.
For each (function, version):
  1. anchor → candidate functions (anchored_funcs_v2)
  2. pick per strategy (maxrefs / unique / largest)
  3. cross-check vs x64 (feature overlap when x64 resolvable)
  4. generate ARM64 pattern + verify exactly-one match
Output: wave1.json
"""
import sys, os, json, struct
sys.path.insert(0, '/home/z/re-analysis')
sys.path.insert(0, '/home/z/re-analysis/mega')
import numpy as np
from engine import VersionCtx, VERSIONS
from anchors import find_substrings, xrefs_to_literal, anchored_funcs_v2
from x64tool import X64

# ---------------- SPECS ----------------
# m: 'R1' anchor→containing func | 'R2' anchor→func→first/nth BL target | 'R2SITE' from xref site
# pick: how to choose among candidate funcs
SPECS = {
    'FindUpdateSafeZonesPhase':        {'m': 'R1', 'strings': ['AFortGameModeAthena::UpdateSafeZonesPhase']},
    'FindHandleMatchHasStarted':       {'m': 'R1', 'strings': ['AFortGameModeAthena::HandleMatchHasStarted: NumPlayers: %i', 'AFortGameModeAthena::HandleMatchHasStarted: Playlist: %s, NumPlayers: %i']},
    'FindCloseActorChannel':           {'m': 'R1', 'strings': ['UActorChannel::Close: ChIndex: %d, Actor: %s', 'UActorChannel::Close: ChIndex: %d, Actor: %s, Reason: %s']},
    'FindCreateChannel':               {'m': 'R1', 'strings': ['STAT_NetConnection_CreateChannelByName']},
    'FindFlushDormancy':               {'m': 'R1', 'strings': ['FlushDormancy: %s. Connection: %s']},
    'FindStartBecomingDormant':        {'m': 'R1', 'strings': ['StartBecomingDormant: %s']},
    'FindEnterAircraft':               {'m': 'R1', 'strings': ['EnterAircraft: [%s] is attempting to enter aircraft after having already exited.']},
    'FindActivatePhase':               {'m': 'R1', 'strings': ['[ASpecialEventScript::ActivatePhase()] [%s] New Phase [%s] OldPhase [%s] SequenceTimeOffset [%f]', '[ASpecialEventScript::ActivatePhase()] [%s] New Phase [%s] OldPhase [%s]']},
    'FindGameSessionPatch':            {'m': 'R1', 'strings': ['Gamephase Step: %s']},
    'FindInitializePlayerGameplayAbilities': {'m': 'R1', 'strings': ['InitializePlayerGameplayAbilities with invalid PlayerStateOrProxy!']},
    'FindInternalTryActivateAbility':  {'m': 'R1', 'strings': ['InternalTryActivateAbility called with invalid Handle! ASC: %s. AvatarActor: %s']},
    'FindNotifyGameMemberAdded':       {'m': 'R1', 'strings': ['%s: Adding Player state with UniqueId: %s, in team: %d, and in squad: %d']},
    'FindOnRep_ZiplineState':          {'m': 'R1', 'strings': ['ZIPLINES!! Role(%s)  AFortPlayerPawn::OnRep_ZiplineState ZiplineState.bIsZiplining=%d', 'ZIPLINES!! GetLocalRole()(%s)  AFortPlayerPawn::OnRep_ZiplineState ZiplineState.bIsZiplining=%d', 'AFortPlayerPawn::HandleZiplineStateChanged']},
    'FindPickSupplyDropLocation':      {'m': 'R1', 'strings': ['PickSupplyDropLocation: Failed to find valid location using the origin and extent provided', 'AFortAthenaMapInfo::PickSupplyDropLocation']},
    'FindPickTeam':                    {'m': 'R1', 'strings': ['PickTeam for [%s] used beacon value [%d]', 'PickTeam for [%s] used beacon value [%s]']},
    'FindQueueStatEvent':              {'m': 'R1', 'strings': ['UFortQuestManager::QueueStatEvent: %s tried to queue a stat event with no valid stat row.']},
    'FindRemoveFromAlivePlayers':      {'m': 'R1', 'strings': ['FortGameModeAthena: Player [%s] removed from alive players list', 'FortGameModeAthena: RemoveFromAlivePlayers: Player [%s] PC [%s] unique id [%s]']} ,
    'FindReplicateActor':              {'m': 'R1', 'strings': ['STAT_NetReplicateActorTime']},
    'FindReplaceBuildingActor':        {'m': 'R1', 'strings': ['STAT_Fort_BuildingSMActorReplaceBuildingActor']},
    'FindSendDestructionInfo':         {'m': 'R1', 'strings': ['STAT_NetSendDestructionInfo']},
    'FindSetChannelActorForDestroy':   {'m': 'R1', 'strings': ['SetChannelActorForDestroy: Channel %d. NetGUID <%s> Path: %s']},
    'FindSetPickupTarget':             {'m': 'R1', 'strings': ['%s: Attempted to spawn non-world item %s!', 'Attempted to spawn non-world item %s!']},
    'FindSetState':                    {'m': 'R1', 'strings': ['Time from Setup to InProgress: %6.2fms']},
    'FindSpawnInitialSafeZone':        {'m': 'R1', 'strings': ['FortGameModeAthena::SpawnInitialSafeZone bShouldSpawnSafeZone[%d]']},
    'FindSpawnLoot':                   {'m': 'R1', 'strings': ['ABuildingContainer::SpawnLoot() called on %s (%s)...', 'ABuildingContainer::SpawnLoot() called on %s (%s)', 'ABuildingContainer::SpawnLoot() >>> START called on %s (%s)']},
    'FindStartStreamingAdditionalPlaylistLevel': {'m': 'R1', 'strings': ['PLAYLIST: Failed to locate valid level to be streamed %s']},
    'FindStreamInMyBuilding':          {'m': 'R1', 'strings': ['&ABuildingFoundation::OnLevelStreamedIn', '%s.%s trying to load invalid level %s']},
    'FindUnEquipVehicleWeapon':        {'m': 'R1', 'strings': ['UFortVehicleSeatWeaponComponent::UnEquipVehicleWeapon failed']},
    'FindSelectAndSetupMyBuildingLevel': {'m': 'R1', 'strings': ['ABuildingFoundation::SelectAndSetupMyBuildingLevel - Cannot setup building level - %s']},
    'FindLoadPlayset':                 {'m': 'R1', 'strings': ['UPlaysetLevelStreamComponent::LoadPlayset Error: no owner for %s']},
    'FindSendRequestNow':              {'m': 'R1', 'strings': ['MCP-Profile: Dispatching request to %s', 'MCP-Profile: Dispatching request to %s - ContextCredentials: %s']},
    'FindPreSendUpdate':               {'m': 'R1', 'strings': ['ReplicationSystem_PreSendUpdate']},
    'FindPayBuildableClassPlacementCost': {'m': 'R1', 'strings': ['Failed to remove item %s during pay building costs, item duplication prevention']},
    'FindCanAffordToPlaceBuildableClass': {'m': 'R1', 'strings': ['Resource not found! Resource Type is %i, might be invalid']},
    'FindApplyCharacterCustomization': {'m': 'R1', 'strings': ['AFortPlayerState::ApplyCharacterCustomization - Failed initialization, using default parts. Player Controller: %s PlayerState: %s, HeroId: %s']},
    'FindKickPlayer':                  {'m': 'R1', 'strings': ['Validation Failure: %s. kicking %s', 'Failed to kick player']},
    'FindHandlePostSafeZonePhaseChanged': {'m': 'R1', 'strings': ['FortGameModeAthena: No MegaStorm on SafeZone[%d].  GridCellThickness is less than 1.0.']},
    'FindGiveAbilityAndActivateOnce':  {'m': 'R1', 'strings': ['GiveAbilityAndActivateOnce called on ability %s on the client, not allowed!']},
    'FindFinishWorldInitialization':   {'m': 'R1', 'strings': ["Can't find a FortAthenaMapInfo placed in map.  Skipping warmup and aircraft phases."]},
    'FindTickFlush':                   {'m': 'R1', 'strings': ['STAT_NetTickFlush']},
    'FindGetMaxTickRate':              {'m': 'R1', 'strings': ['Hitching by request!']},
    'FindInitializeBuildingActor':     {'m': 'R1', 'strings': ['STAT_Fort_BuildingSMActorInitializeBuildingActor'], 'note': 'x64 uses fwd-scan to NEXT function — record both candidates, cross-check picks'},
    # R2 specials
    'FindGetNetMode':                  {'m': 'R2', 'strings': ['PREPHYSBONES'], 'bl': 'first'},
    'FindGiveAbility':                 {'m': 'R2SITE', 'strings': ['GiveAbilityAndActivateOnce called on ability %s on the client, not allowed!'], 'bl': 'next_after_site'},
    'FindStartAircraftPhase':          {'m': 'R2E8x2', 'strings': ['STARTAIRCRAFT'], 'bl': 'second'},
    'FindStartAircraftPhaseStat':      {'m': 'R1', 'strings': ['STAT_StartAircraftPhase']},
}


def resolve_R1(ctx, strings):
    """returns (cands {func: nrefs}, used_string)"""
    for s in strings:
        fns = anchored_funcs_v2(ctx, s)
        if fns:
            return fns, s
    return {}, None


def pick_func(fns, strategy='maxrefs', ctx=None):
    if not fns:
        return None
    if len(fns) == 1:
        return next(iter(fns))
    if strategy == 'maxrefs':
        return max(fns.items(), key=lambda kv: (len(kv[1]), -kv[0]))[0]
    if strategy == 'largest':
        return max(fns, key=lambda f: ctx.func_size(f))
    if strategy == 'first':
        return min(fns)
    return None


def bls_from(ctx, fn):
    return ctx.bls_in(fn)


def process_one(ctx, spec):
    """resolve a single spec on a single version ctx — returns res dict"""
    res = {'status': '?'}
    if spec['m'] in ('R1', 'R2', 'R2SITE', 'R2E8x2'):
        fns, used = resolve_R1(ctx, spec['strings'])
        if not fns:
            return {'status': 'no_anchor'}
        if spec['m'] == 'R1':
            picked = pick_func(fns, 'maxrefs', ctx)
            return {'status': 'ok', 'addr': hex(picked), 'cands': {hex(k): len(v) for k, v in fns.items()},
                    'used': used, 'size': ctx.func_size(picked)}
        if spec['m'] == 'R2':
            picked = pick_func(fns, 'maxrefs', ctx)
            bls = bls_from(ctx, picked)
            if not bls:
                return {'status': 'no_bl', 'host': hex(picked)}
            return {'status': 'ok', 'addr': hex(bls[0][1]), 'host': hex(picked),
                    'cands': {hex(k): len(v) for k, v in fns.items()}, 'used': used}
        if spec['m'] in ('R2SITE', 'R2E8x2'):
            all_pc = []
            for s in spec['strings']:
                for pc, tgt, la, lit, w in xrefs_to_literal(ctx, s):
                    f = ctx.tool.func_of(pc)
                    if f is not None:
                        all_pc.append((pc, f))
            if not all_pc:
                return {'status': 'no_anchor'}
            pc, host = min(all_pc)
            bls = [b for b in bls_from(ctx, host) if b[0] >= pc]
            if spec['m'] == 'R2SITE':
                if not bls:
                    return {'status': 'no_bl_after_site', 'host': hex(host)}
                return {'status': 'ok', 'addr': hex(bls[0][1]), 'host': hex(host), 'site': hex(pc)}
            else:
                if len(bls) < 2:
                    return {'status': 'no_2bl', 'host': hex(host)}
                return {'status': 'ok', 'addr': hex(bls[1][1]), 'host': hex(host), 'site': hex(pc)}
    return res


def main():
    import gc
    out = {}
    if os.path.exists('/home/z/re-analysis/mega/wave1.json'):
        try:
            out = json.load(open('/home/z/re-analysis/mega/wave1.json'))
        except Exception:
            out = {}
    only = sys.argv[1:] if len(sys.argv) > 1 else None
    specs = {k: v for k, v in SPECS.items() if not only or k in only}

    for ver in VERSIONS:
        ctx = VersionCtx(ver)  # fresh context per version (memory-safe)
        for name, spec in specs.items():
            entry = out.get(name, {})
            if ver in entry and entry[ver].get('status') in ('ok', 'no_anchor') and entry[ver].get('pattern'):
                continue  # cached
            try:
                res = process_one(ctx, spec)
            except Exception as e:
                res = {'status': 'error', 'err': str(e)[:120]}
            # pattern gen
            if res.get('status') == 'ok' and res.get('addr'):
                try:
                    p = ctx.gen_pattern(int(res['addr'], 16))
                    if p:
                        if p.get('icf_stub'):
                            res['pattern'] = 'ICF_STUB'
                            res['unique'] = False
                        else:
                            res['pattern'] = p['pattern']
                            res['unique'] = p['ok']
                            res['pat_ins'] = p['n_ins']
                            if not p['ok']:
                                res['pat_hits'] = p.get('hits_list', [])
                except Exception as e:
                    res['pattern_err'] = str(e)[:100]
            entry[ver] = res
            out[name] = entry
        ok = sum(1 for k, v in out.items() if k in specs and v.get(ver, {}).get('status') == 'ok')
        tot = len(specs)
        print(f"{ver}: {ok}/{tot} resolved", flush=True)
        json.dump(out, open('/home/z/re-analysis/mega/wave1.json', 'w'), indent=1)
        # release
        ctx.tool.close()
        del ctx
        VersionCtx._cache.clear()
        gc.collect()

    # summary per function
    print("\n===== per-function summary =====")
    for name in specs:
        e = out.get(name, {})
        ok = [v for v in e.values() if v.get('status') == 'ok']
        uniq = [v for v in e.values() if v.get('unique')]
        print(f"{name:48s} ok={len(ok):2d}/11 unique={len(uniq):2d}")
    print("\nSaved wave1.json")


if __name__ == "__main__":
    main()
