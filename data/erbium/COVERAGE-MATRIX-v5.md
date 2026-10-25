# COVERAGE MATRIX v5 — أنماط ثابتة (✓) + إيجاد وقت-التشغيل (⟳)

**الخلايا:** 11 نسخة × 89 دالة = 979
**✓ نمط ثابت (v4):** 759 | **⟳ إيجاد وقت-تشغيل (v5 RT::):** 154 | **· لا شيء:** 66

⟳ = تعمل على أي إصدار (حتى غير المرئية 6.31/12.41/20.10/27.11...) عبر:
string-anchor (نص→مرجع→de-outline→بداية الدالة) أو native-pair أو vtable-delta

| Function | 5.41 | 7.40 | 9.41 | 13.40 | 17.50 | 18.40 | 19.10 | 21.30 | 22.40 | 23.50 | 30.00 | Chains |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FindActivatePhase | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindApplyCharacterCustomization | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindCallPreReplication | · | · | · | · | · | · | · | ✓ | ✓ | ✓ | ✓ | - |
| FindCanAffordToPlaceBuildableClass | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string,vtable |
| FindCantBuild | ✓ | ✓ | ⟳ | ✓ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindClearAbility | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| FindClientHasInitializedLevelFor | · | · | · | · | · | · | · | ✓ | · | · | · | - |
| FindCloseActorChannel | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string,vtable |
| FindConstructAbilitySpec | ✓ | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | · | ✓ | ✓ | - |
| FindCreateChannel | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindCreateNetDriverWorldContext | · | · | · | · | · | ✓ | ✓ | ✓ | ✓ | · | ✓ | - |
| FindEnterAircraft | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindFinishWorldInitialization | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindFinishedTargetSpline | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ⟳ | ✓ | string |
| FindFlushDormancy | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ⟳ | ⟳ | ⟳ | string,vtable |
| FindGIsClient | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | gis |
| FindGIsServer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | gis |
| FindGameSessionPatch | ✓ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindGetMaxTickRate | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindGetNamePool | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindGetNetMode | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindGetPlayerViewPoint | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindGetWorldContext | · | · | · | · | · | · | · | ✓ | ✓ | · | ✓ | - |
| FindGiveAbility | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindGiveAbilityAndActivateOnce | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindHandleMatchHasStarted | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindHandlePostSafeZonePhaseChanged | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindInitListen | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string,vtable |
| FindInitializeBuildingActor | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindInitializeBuildingActor_next | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindInitializeFlightPath | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ⟳ | native |
| FindInitializePlayerGameplayAbilities | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindInternalTryActivateAbility | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindIsNetReady | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ⟳ | ⟳ | ⟳ | vtable |
| FindIsNetRelevantForVft | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindKickPlayer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindListenCall | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindLoadPlayset | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindNotifyGameMemberAdded | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | ⟳ | string |
| FindOnItemInstanceAddedVft | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | sdk |
| FindOnRep_ZiplineState | ✓ | ✓ | ✓ | ✓ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindPayBuildableClassPlacementCost | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string,vtable |
| FindPickSupplyDropLocation | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | ✓ | ✓ | ✓ | ✓ | string |
| FindPickTeam | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindPostInitializeSpawnedBuildingActor | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindPreSendUpdate | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | string |
| FindQueueStatEvent | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ⟳ | string |
| FindRemoveFromAlivePlayers | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | ⟳ | string |
| FindRemoveInventoryItem | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindRemoveInventoryStateValue | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindReplaceBuildingActor | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindReplicateActor | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindReset | ✓ | ✓ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | vtable |
| FindSelectAndSetupMyBuildingLevel | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindSendClientAdjustment | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ⟳ | ⟳ | ⟳ | vtable |
| FindSendDestructionInfo | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | ✓ | ✓ | ✓ | string |
| FindSendRequestNow | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindServerReplicateActors | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | - |
| FindSetChannelActor | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindSetChannelActorForDestroy | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindSetGamePhase | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | string |
| FindSetInventoryStateValue | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindSetPickupItems | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| FindSetPickupTarget | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindSetState | ✓ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindSetWorld | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,native |
| FindShouldAllowServerSpawnDecoVft | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindSpawnDecoVft | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindSpawnInitialSafeZone | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string |
| FindSpawnLoot | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindStartAircraftPhase | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindStartBecomingDormant | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⟳ | string,vtable |
| FindStartStreamingAdditionalPlaylistLevel | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindStreamInMyBuilding | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindTickFlush | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string,vtable |
| FindUnEquipVehicleWeapon | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | string |
| FindUpdateSafeZonesPhase | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ⟳ | ✓ | ⟳ | ⟳ | ⟳ | string |
| Inline_AttemptDeriveFromURL | · | · | · | · | · | · | · | ✓ | · | · | · | - |
| Inline_ClientOnPawnDiedCaller | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_CollectGarbagePatch | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | - |
| Inline_EquipVehicleWeaponFunc | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_GetSparseClassData | · | · | · | · | ✓ | · | · | · | · | · | · | - |
| Inline_K2_OnUnEquipCaller | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_OnAuthorityRandomUpgradeAppliedCaller | ⟳ | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_OnDamageServerCaller | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_OnPawnEnterSeatFunc | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_ServerOnAttemptInteract | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_ToDeathCauseNative | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |
| Inline_UnequipVehicleWeaponFunc | ⟳ | ⟳ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | native |

**التغطية الفعلية عند التشغيل: 913/979 (93%)** — كل خلية إما نمط فريد مُتحقق أو سلسلة إيجاد غاشمة وقت-التشغيل.

## الدوال بلا أي سلسلة (تعتمد على الأنماط فقط):
- `FindCallPreReplication` (أنماط: 4/11)
- `FindClientHasInitializedLevelFor` (أنماط: 1/11)
- `FindConstructAbilitySpec` (أنماط: 9/11)
- `FindCreateNetDriverWorldContext` (أنماط: 5/11)
- `FindGetNamePool` (أنماط: 8/11)
- `FindGetWorldContext` (أنماط: 3/11)
- `FindInitializeBuildingActor_next` (أنماط: 8/11)
- `FindLoadPlayset` (أنماط: 11/11)
- `FindPostInitializeSpawnedBuildingActor` (أنماط: 11/11)
- `FindRemoveInventoryItem` (أنماط: 11/11)
- `FindRemoveInventoryStateValue` (أنماط: 11/11)
- `FindServerReplicateActors` (أنماط: 7/11)
- `FindSetChannelActor` (أنماط: 10/11)
- `FindSetInventoryStateValue` (أنماط: 11/11)
- `FindSetPickupItems` (أنماط: 10/11)
- `Inline_AttemptDeriveFromURL` (أنماط: 1/11)
- `Inline_CollectGarbagePatch` (أنماط: 10/11)
- `Inline_GetSparseClassData` (أنماط: 1/11)