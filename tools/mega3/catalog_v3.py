#!/usr/bin/env python3
"""CATALOG V3 — Complete Erbium dynamic-resolution inventory.
83 Finders.cpp functions + ALL inline resolutions OUTSIDE Finders.cpp
(pattern-based, FindStringRef-based, FindFunctionCall-based, vtable-ScanFor).

Sources verified by reading the actual Erbium source at /tmp/Erbium.
Every entry: file:line, x64 patterns (verbatim), version conditions, mechanism, purpose.
Output: catalog_v3.json
"""
import json, re

cat83 = json.load(open('/home/z/re-analysis/mega/catalog.json'))

# ============ INLINE FUNCTIONS OUTSIDE Finders.cpp ============
# (verified against source at /tmp/Erbium)
INLINE = {
    # ---- A. Pattern-based call/callers ----
    "Inline_GetSparseClassData": {
        "file": "FortniteGame/Private/BuildingSMActor.cpp", "line": 591,
        "target_var": "GetSparseClassData_",
        "patterns": [
            "48 83 EC ? 48 8B 81 ? ? ? ? 45 33 C0 4C 8B C9",
            "48 83 EC ? 48 8B 81 ? ? ? ? 48 85 C0 74 ? 48 83 C4 ? C3",
            "48 83 EC ? 48 8B 81 ? ? ? ? 45 33 C0 48 85 C0 75",
        ],
        "condition": "!GetDefaultObj()->HasBuildingResourceAmountOverride()",
        "mechanism": "FindPattern", "purpose": "call-target",
        "semantic": "USparseClassData getter: sub rsp,X; mov rax,[rcx+imm32]; xor r8d,r8d; mov r9,rcx — tiny function returning SparseClassData ptr at fixed class offset",
        "note": "hooked? no — stored and CALLED directly",
    },
    "Inline_SetOwningInventory": {
        "file": "FortniteGame/Private/FortInventory.cpp", "line": 678,
        "target_var": "SetOwningInventory",
        "patterns": [
            "48 85 D2 74 ? 80 BA ? ? ? ? ? 75 ? 48 89 91",
            "48 83 EC ? 48 85 D2 74 ? 80 BA ? ? ? ? ? 75 ? 48 81 C1",
            "48 85 D2 74 ? 48 89 5C 24 ? 57 48 83 EC ? 48 8B F9 48 8B DA 48 8B CA E8 ? ? ? ? 83 F8 ? 75",
        ],
        "condition": "always", "mechanism": "FindPattern", "purpose": "vtable-slot-finder",
        "semantic": "test rdx,rdx; jz; cmp byte [rdx+imm32],?; jne; mov [rcx+imm32],rdx — sets OwningInventory field w/ temporary-item check",
        "note": "pattern finds a CALLER/inlined-site; then vtable of UFortWorldItem is scanned for the function that matches; actually: pattern = the real function? NO — used to find SetOwningInventory vtable index by comparing function bytes",
    },
    "Inline_ToDeathCauseNative": {
        "file": "FortniteGame/Private/FortPlayerControllerAthena.cpp", "line": 1059,
        "target_var": "ToDeathCauseNative",
        "patterns": [
            "48 89 5C 24 ? 48 89 6C 24 ? 48 89 74 24 ? 57 48 83 EC 20 41 0F B6 F8 48 8B DA 48 8B F1 E8 ? ? ? ? 33 ED",  # UE4.19
            "48 89 5C 24 ? 48 89 74 24 ? 57 48 83 EC 20 0F B6 FA 48 8B D9 E8 ? ? ? ? 33 F6 48 89 74 24",  # UE4.20
            "48 89 5C 24 ? 48 89 74 24 ? 57 48 83 EC 20 0F B6 FA 48 8B D9 E8 ? ? ? ? 33",  # UE4.21
        ],
        "condition": "EngineVersion >= 4.19 (and < 4.22 native path)", "mechanism": "FindPattern", "purpose": "call-target",
        "semantic": "ToDeathCause(TagContainer, bDBNO) native impl — prologue saves rbx/rsi/rdi, movzx edi,r8b (bDBNO), mov rdx/rdx? rdx=Tags, call GetTagCount…",
        "note": "Android versions available are UE4.21+ (5.41=UE4.21) so only pattern 3 relevant for 5.41; 7.40/9.41 are UE4.21/4.22/4.23 era",
    },
    "Inline_SetIsInsideSafeZoneBase": {
        "file": "FortniteGame/Private/FortPlayerPawnAthena.cpp", "line": 642,
        "target_var": "SetIsInsideSafeZoneBase",
        "patterns": [
            "74 ? 33 D2 48 8B ? E8 ? ? ? ? 48 8B ? 41 8A ? 48 8B ? FF 90",  # mov dl,rXXb
            "74 ? 33 D2 48 8B ? E8 ? ? ? ? 48 8B ? B2 01 48 8B ? FF 90",  # mov dl,1
            "0F 84 ? ? ? ? 33 D2 48 8B ? E8 ? ? ? ? 48 8B ? B2 01 48 8B ? FF 90",  # imm32 jz
        ],
        "condition": "always", "mechanism": "FindPattern", "purpose": "patch-site(virtual-call-slot)",
        "semantic": "jz; xor edx,edx; mov rax,[?]; call IsInsideSafeZone-ish; mov rax,[rax]; mov cl/dl,[r?+..]; mov rax,[rax]; call [rax+vt] — inlined safe-zone containment check with virtual call",
        "note": "the site is INSIDE a function (not a function start) — patch semantics",
    },
    "Inline_OnPawnEnterSeatFunc": {
        "file": "FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp", "line": 134,
        "target_var": "OnPawnEnterSeatFunc",
        "patterns": [
            "48 89 5C 24 ? 48 89 6C 24 ? 48 89 74 24 ? 48 89 7C 24 ? 41 56 48 83 EC ? 48 8B 99 ? ? ? ? 41 8B F0",
            "48 8B C4 48 89 58 ? 48 89 68 ? 48 89 70 ? 48 89 78 ? 41 56 48 83 EC ? 48 63 A9 ? ? ? ? 41 8B F0",
        ],
        "condition": "always", "mechanism": "FindPattern", "purpose": "vtable-index-extraction",
        "semantic": "OnPawnEnterSeat(this, Pawn, ?) — saves regs, mov rbx,[rcx+imm32] (SeatComponent->Vehicle), mov esi,r8d; TWO virtual calls FF 90 used to read GetMountedWeaponOperatorSeatIndexIdx & GetPlayerSlotsIdx",
        "note": "after match: ScanFor FF 90 ×2 → vtable offsets /8",
    },
    "Inline_EquipVehicleWeaponFunc": {
        "file": "FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp", "line": 150,
        "target_var": "EquipVehicleWeaponFunc",
        "patterns": [
            "48 89 5C 24 ? 57 48 83 EC ? 41 80 78 ? ? 48 8B DA 48 8B F9",
        ],
        "condition": "always", "mechanism": "FindPattern", "purpose": "call-target",
        "semantic": "cmp byte [r8+?],? early check then mov rbx,rdx; mov rdi,rcx — EquipVehicleWeapon-ish",
    },
    "Inline_UnequipVehicleWeaponFunc": {
        "file": "FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp", "line": 156,
        "target_var": "UnequipVehicleWeaponFunc",
        "patterns": [
            "48 89 5C 24 ? 48 89 6C 24 ? 48 89 74 24 ? 57 41 54 41 55 41 56 41 57 48 83 EC ? 45 33 F6 45 8A E9",
            "48 8B C4 48 89 58 ? 48 89 68 ? 48 89 70 ? 44 88 48 ? 57 41 54 41 55 41 56 41 57 48 83 EC ? 48 8B 81",
            "48 8B C4 48 89 58 ? 48 89 68 ? 48 89 70 ? 44 88 48 ? 57 41 54 41 55 41 56 41 57 48 83 EC ? 4D 8B E8",
        ],
        "condition": "always", "mechanism": "FindPattern", "purpose": "call-target",
        "semantic": "UnequipVehicleWeapon(this, bool) — big prologue, xor r14d, mov r13b,r9b",
        "note": "pattern3 is for newest; related FindUnEquipVehicleWeapon() in Finders.cpp",
    },
    "Inline_AttemptDeriveFromURL": {
        "file": "Erbium/Private/Misc.cpp", "line": 530,
        "target_var": "AttemptDeriveFromURL",
        "patterns": [
            "48 89 5C 24 ? 55 56 57 41 54 41 55 41 56 41 57 48 81 EC ? ? ? ? 4C 8B C1",
            "48 89 5C 24 ? 55 56 57 41 54 41 55 41 56 41 57 48 81 EC ? ? ? ? 48 8B D1",
            "48 89 5C 24 ? 55 56 57 41 54 41 55 41 56 41 57 48 81 EC ? ? ? ? 4C 8B D1",
            "48 89 5C 24 ? 48 89 6C 24 ? 48 89 74 24 ? 57 41 54 41 55 41 56 41 57 48 81 EC ? ? ? ? 4C 8B D1",
        ],
        "condition": "GetNetMode hook setup", "mechanism": "FindPattern", "purpose": "patch-site(caller-of-GetNetMode)",
        "semantic": "UEngine::AttemptDeriveFromURL — full-frame prologue 8 regs + mov r8,rcx / mov rdx,rcx — the CALLER of GetNetMode that gets patched for netmode fix",
        "note": "hook: the call to GetNetMode inside this func is patched to return NM_DedicatedServer",
    },
    "Inline_CollectGarbagePatch": {
        "file": "FortniteGame/Private/FortGameMode.cpp", "line": 838,
        "target_var": "pattern",
        "patterns": [
            "E8 ? ? ? ? EB 26 40 38 3D ? ? ? ?",     # FN > 3.2
            "E8 ? ? ? ? F0 FF 0D ? ? ? ? 0F B6 C3",  # FN <= 3.2
        ],
        "condition": "EngineVersion <= 4.20", "mechanism": "FindPattern+RelativeOffset(1)", "purpose": "patch-site(call→ret)",
        "semantic": "call CollectGarbage; jmp+0x26; cmp [rip+X],spl — the E8 call target gets patched to 0xC3 (ret) to disable GC during startup",
        "note": "only ancient versions; Android 5.41 is UE4.21 — likely N/A for our versions (verify)",
    },
    "Inline_IrisBool": {
        "file": "ErbiumClient/Private/dllmain.cpp", "line": 56,
        "target_var": "IrisBool",
        "patterns": [
            "83 3D ? ? ? ? ? 0F 8E ? ? ? ? 49 8B B9",
            "44 39 25 ? ? ? ? 0F 9F C0 45 84 FF",
        ],
        "condition": "Iris replication versions", "mechanism": "FindPattern+RelativeOffset(2/3)", "purpose": "global-bool-patch",
        "semantic": "cmp dword [rip+imm32],?; jle — FReplicationFlags/iris-enabled global bool forced to 1",
        "note": "ACTIVE in ErbiumClient/dllmain.cpp (commented in main dllmain.cpp); used in VerifyIris()",
    },
    # ---- B. FindFunctionCall / FindNameRef based (UFunction name → ref → caller) ----
    "Inline_OnDamageServerCaller": {
        "file": "FortniteGame/Private/BuildingSMActor.cpp", "line": 605,
        "ufunc_name": "OnDamageServer",
        "prologue_variants": {"4.16": "4C 89 4C", "4.19||>=4.27": "48 8B C4", "else": "40 55"},
        "mechanism": "FindFunctionCall", "purpose": "hook-target",
        "semantic": "the function that CALLS OnDamageServer UFunction (via ProcessEvent ref) — hooked with OnDamageServer OG trampoline",
        "note": "OnDamageServer is a UFunction of ABuildingSMActor — find its native exec + its callers",
    },
    "Inline_ClientOnPawnDiedCaller": {
        "file": "FortniteGame/Private/FortPlayerControllerAthena.cpp", "line": 3615,
        "ufunc_name": "ClientOnPawnDied",
        "prologue_variants": {"4.16": "48 89 54", ">=24<25": "48 8B C4", "else": "48 89 5C"},
        "mechanism": "FindFunctionCall", "purpose": "hook-target",
        "semantic": "the server function that calls ClientOnPawnDied (Client RPC sender) — hooked",
    },
    "Inline_K2_OnUnEquipCaller": {
        "file": "FortniteGame/Private/FortPlayerControllerAthena.cpp", "line": 3638,
        "ufunc_name": "K2_OnUnEquip",
        "prologue_variants": {"else": "48 89 5C"},
        "condition": "FortniteVersion >= 14.00", "mechanism": "FindFunctionCall", "purpose": "hook-target",
        "semantic": "caller of K2_OnUnEquip UFunction (FortWeaponComponent) — hooked with OnUnEquip",
    },
    "Inline_OnAuthorityRandomUpgradeAppliedCaller": {
        "file": "FortniteGame/Private/FortLootPackage.cpp", "line": 690,
        "ufunc_name": "OnAuthorityRandomUpgradeApplied",
        "prologue_variants": {"else": "48 89 5C"},
        "mechanism": "FindFunctionCall", "purpose": "hook-target",
        "semantic": "caller of OnAuthorityRandomUpgradeApplied — hooked for loot upgrade logic",
    },
    # ---- C. StringRef-based ----
    "Inline_PostUpdate": {
        "file": "FortniteGame/Private/FortLootPackage.cpp", "line": 679,
        "target_var": "PostUpdate_",
        "strings": ["ABuildingSMActor::PostUpdate() Building: %s, AltMeshIdx: %d"],
        "wide": [True], "wide_arg": True,
        "condition": "FortniteVersion >= 19", "mechanism": "FindStringRef+backscan", "purpose": "call-target",
        "semantic": "ABuildingSMActor::PostUpdate — logs building/altmeshidx then updates mesh",
    },
    "Inline_ServerOnAttemptInteract": {
        "file": "FortniteGame/Private/FortLootPackage.cpp", "line": 696,
        "target_var": "ServerOnAttemptInteractRef",
        "strings": ["ABuildingContainer::ServerOnAttemptInteract %s failed for %s"],
        "wide": [True],
        "mechanism": "FindStringRef+ScanFor(48 89 5C backscan)", "purpose": "hook-target",
        "semantic": "ABuildingContainer::ServerOnAttemptInteract — the real interact handler",
    },
    # ---- D. vtable-ScanFor based ----
    "Inline_CanGrappleToComponent": {
        "file": "FortniteGame/Private/FortPhysicsPawn.cpp", "line": 241,
        "target_var": "CanGrappleToComponent_",
        "anchor_vfunc": "FortOctopusTowhookAttachableProjectile:OnRep_ReplicatedAttachedInfo",
        "mechanism": "vtable-impl+ScanFor(FF 90)+backscan(48 8B+E8/FF 90)", "purpose": "call-target",
        "semantic": "from OnRep_ReplicatedAttachedInfo impl, find the virtual call slot, then scan back for mov rax,[..]; call/jmp → CanGrappleToComponent function or vtable slot",
    },
}

cat3 = dict(cat83)
for k, v in INLINE.items():
    v["inline"] = True
    cat3[k] = v

json.dump(cat3, open('/home/z/re-analysis/mega/catalog_v3.json', 'w'), indent=1)
print(f"catalog_v3: {len(cat3)} entries (83 Finders + {len(INLINE)} inline)")
for k in INLINE:
    print(" +", k)
