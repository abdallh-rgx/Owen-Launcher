#!/usr/bin/env python3
"""Extract per-function anchors from Erbium Finders.cpp:
- FindStringRef strings (wide/ascii)
- native names (from our natives map)
Outputs erbium-anchors.json used by the v5 generator."""
import json
import re

SRC = '/home/z/erbium-src/Erbium/Erbium/Private/Finders.cpp'

src = open(SRC, encoding='utf-8', errors='replace').read()
funcs = {}
for m in re.finditer(r'uint(?:64_t|64|ptr_t)\s+(Find\w+)\s*\(\s*\)\s*\{', src):
    name, start = m.group(1), m.end()
    depth, i = 1, start
    while i < len(src) and depth:
        if src[i] == '{':
            depth += 1
        elif src[i] == '}':
            depth -= 1
        i += 1
    body = src[start:i]
    strs = re.findall(r'FindStringRef\(\s*(L?)\s*"((?:[^"\\]|\\.)*)"', body)
    funcs[name] = {'strings': [{'wide': w == 'L', 's': s.replace('\\"', '"')} for w, s in strs]}

# Android native anchors from the v4 resolution strategies
NAT = {
    'FindClearAbility': 'ClearAbility',
    'FindGiveAbility': 'K2_GiveAbility',
    'FindGiveAbilityAndActivateOnce': 'K2_GiveAbilityAndActivateOnce',
    'FindKickPlayer': 'ServerKickPlayer',
    'FindOnRep_ZiplineState': 'OnRep_ZiplineState',
    'FindSetWorld': 'K2_SetWorldLocation',
    'Inline_ToDeathCauseNative': 'ToDeathCause',
    'Inline_ServerOnAttemptInteract': 'ServerOnAttemptInteract',
    'Inline_OnPawnEnterSeatFunc': 'OnPawnEnterSeat',
    'Inline_K2_OnUnEquipCaller': 'K2_OnUnEquip',
    'Inline_OnDamageServerCaller': 'OnDamageServer',
    'Inline_ClientOnPawnDiedCaller': 'ClientOnPawnDied',
    'Inline_OnAuthorityRandomUpgradeAppliedCaller': 'OnAuthorityRandomUpgradeApplied',
    'Inline_EquipVehicleWeaponFunc': 'EquipVehicleWeapon',
    'Inline_UnequipVehicleWeaponFunc': 'UnEquipVehicleWeapon',
    'FindActivatePhase': 'ActivatePhase',
}
for fn, nat in NAT.items():
    if fn in funcs:
        funcs[fn]['native'] = nat
    else:
        funcs[fn] = {'strings': [], 'native': nat}

# String anchors discovered for the hard functions (Android UTF-16 literal pools / .rodata)
EXTRA_STRINGS = {
    'FindFlushDormancy': [{'wide': True, 's': 'FlushDormancy: %s. Connection: %s'}],
    'FindStartBecomingDormant': [{'wide': True, 's': 'StartBecomingDormant: %s'}],
    'FindEnterAircraft': [{'wide': True, 's': 'EnterAircraft: [%s] is attempting to enter aircraft after having already exited.'}],
    'FindUpdateSafeZonesPhase': [{'wide': True, 's': 'AFortGameModeAthena::UpdateSafeZonesPhase'}],
    'FindPreSendUpdate': [{'wide': True, 's': 'ReplicationSystem_PreSendUpdate'}],
    'FindSpawnInitialSafeZone': [{'wide': True, 's': 'FortGameModeAthena::SpawnInitialSafeZone bShouldSpawnSafeZoneIndicator == false, skipping spawning Safe Zone Indicator'}],
    'FindHandleMatchHasStarted': [{'wide': True, 's': 'AFortGameModeAthena::HandleMatchHasStarted: NumPlayers: %i'}],
    'FindSelectAndSetupMyBuildingLevel': [{'wide': True, 's': 'ABuildingFoundation::SelectAndSetupMyBuildingLevel'}],
    'FindQueueStatEvent': [{'wide': True, 's': 'QueueStatEvent: %s'}],
    'FindPickSupplyDropLocation': [{'wide': False, 's': 'PickSupplyDropLocation'}],
    'FindSetGamePhase': [{'wide': True, 's': 'SetGamePhase'}],
    'FindFinishedTargetSpline': [{'wide': False, 's': 'FinishedTargetSpline'}],
    'FindCantBuild': [{'wide': True, 's': 'Cannot build; hitting build limit'}],
    'FindGameSessionPatch': [{'wide': True, 's': 'GameSession'}],
    'FindNotifyGameMemberAdded': [{'wide': True, 's': 'NotifyGameMemberAdded'}],
}
for fn, ss in EXTRA_STRINGS.items():
    funcs.setdefault(fn, {'strings': []})['strings'].extend(ss)

# vtable deltas PROVEN (Android 11/11 documented + Win 27.11 verified)
VT_DELTAS = {
    'FindIsNetReady': {'anchor': 'FindFlushDormancy', 'delta': -17, 'note': 'proven 11/11 Android + Win 27.11 (5 vtables agree)'},
    'FindFlushDormancy': {'anchor': 'FindIsNetReady', 'delta': +17, 'note': 'reverse of above'},
    'FindStartBecomingDormant': {'anchor': 'FindCloseActorChannel', 'delta': +11, 'note': 'documented Android 10 versions'},
    'FindCloseActorChannel': {'anchor': 'FindStartBecomingDormant', 'delta': -11, 'note': 'reverse'},
    'FindSendClientAdjustment': {'anchor': 'FindReset', 'delta': +276, 'note': 'Win 27.11 (276 in 4 vtables); per-version check needed'},
    'FindReset': {'anchor': 'FindSendClientAdjustment', 'delta': -276, 'note': 'reverse'},
    'FindCanAffordToPlaceBuildableClass': {'anchor': 'FindSendClientAdjustment', 'delta': +385, 'note': 'Win 27.11'},
    'FindInitListen': {'anchor': 'FindTickFlush', 'delta': -15, 'note': 'documented Android 6 versions'},
    'FindTickFlush': {'anchor': 'FindInitListen', 'delta': +15, 'note': 'reverse'},
}
funcs['__VT_DELTAS__'] = VT_DELTAS

# vtable indices documented (Android-computed, version clustered)
VT_INDICES = {
    'FindOnItemInstanceAddedVft': {'17.50': 143, '18.40': 143, '19.10': 143, '21.30': 143, '22.40': 143, '23.50': 143, '30.00': 144, '13.40': 143, '9.41': 143, '7.40': 143, '5.41': 143},
    'FindIsNetRelevantForVft': {'5.41': 135, '7.40': 135, '9.41': 135, '13.40': 139, '17.50': 143, '18.40': 143, '19.10': 152, '21.30': 155, '22.40': 156, '23.50': 157, '30.00': 158},
    'FindSpawnDecoVft': {'_default': 395},
    'FindCloseActorChannel': {'22.40': 90, '_default': 90},
    'FindSetChannelActor': {'27.11win': 36, '_default': 36},
}

with open('/home/z/re-analysis/out/erbium-anchors.json', 'w') as f:
    json.dump(funcs, f, indent=1)
print('saved with', len([k for k in funcs if not k.startswith('__')]), 'functions')
for fn, info in funcs.items():
    if fn.startswith('__'):
        continue
    ss = [('#' if x['wide'] else '') + x['s'][:45] for x in info['strings']]
    if ss or 'native' in info:
        print(f'{fn}: native={info.get("native","-")} strings={ss[:3]}')
