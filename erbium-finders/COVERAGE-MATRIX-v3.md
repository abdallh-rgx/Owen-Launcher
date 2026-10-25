# Erbium → Android ARM64 — التغطية النهائية v3 (83 + 17 دالة × 11 نسخة)

> كل خلية ✓ خضعت للتحقق الثلاثي: P0 (النمط يطابق مرة واحدة بالضبط) + P1 (cross-arch مقابل ويندوز) + P2 (native-identity)

| الدالة | 5.41 | 7.40 | 9.41 | 13.40 | 17.50 | 18.40 | 19.10 | 21.30 | 22.40 | 23.50 | 30.00 | المجموع |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `FindActivatePhase` | · | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 7/11 |
| `FindApplyCharacterCustomization` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 9/11 |
| `FindCallPreReplication` | · | · | · | · | · | · | · | ✓ | ✓ | ✓ | ✓ | 4/11 |
| `FindCanAffordToPlaceBuildableClass` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindCantBuild` | ✓ | ✓ | · | ✓ | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 9/11 |
| `FindClearAbility` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindClientHasInitializedLevelFor` | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `FindCloseActorChannel` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindConstructAbilitySpec` | ✓ | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | · | ✓ | ✓ | 9/11 |
| `FindCreateChannel` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindCreateNetDriverWorldContext` | · | · | · | · | · | ✓ | ✓ | ✓ | ✓ | · | ✓ | 5/11 |
| `FindEnterAircraft` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindFinishWorldInitialization` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindFinishedTargetSpline` | · | · | · | · | · | · | · | ✓ | ✓ | · | ✓ | 3/11 |
| `FindFlushDormancy` | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `FindGIsClient` | ⚠️ | ⚠️ | ⚠️ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⚠️ | ✓ | 11/11 |
| `FindGIsServer` | ⚠️ | ⚠️ | ⚠️ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ⚠️ | ✓ | 11/11 |
| `FindGameSessionPatch` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindGetMaxTickRate` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindGetNamePool` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindGetNetMode` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindGetPlayerViewPoint` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindGetWorldContext` | · | · | · | · | · | · | · | ✓ | ✓ | · | ✓ | 3/11 |
| `FindGiveAbility` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindGiveAbilityAndActivateOnce` | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 9/11 |
| `FindHandleMatchHasStarted` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindHandlePostSafeZonePhaseChanged` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindInitListen` | · | · | · | ✓ | · | ✓ | ✓ | ✓ | ✓ | ✓ | · | 6/11 |
| `FindInitializeBuildingActor` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindInitializeBuildingActor_next` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindInitializeFlightPath` | · | · | · | · | · | · | · | · | ✓ | ✓ | · | 2/11 |
| `FindInitializePlayerGameplayAbilities` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindInternalTryActivateAbility` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindIsNetReady` | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `FindIsNetRelevantForVft` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 9/11 |
| `FindKickPlayer` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindListenCall` | ✓ | ✓ | ✓ | · | · | · | · | · | · | · | · | 3/11 |
| `FindLoadPlayset` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindNotifyGameMemberAdded` | · | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | · | · | 5/11 |
| `FindOnItemInstanceAddedVft` | · | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 6/11 |
| `FindOnRep_ZiplineState` | · | ✓ | ✓ | ✓ | · | · | · | ✓ | ✓ | ✓ | ✓ | 7/11 |
| `FindPayBuildableClassPlacementCost` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindPickSupplyDropLocation` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindPickTeam` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindPostInitializeSpawnedBuildingActor` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindPreSendUpdate` | · | · | · | · | · | · | · | · | · | · | ✓ | 1/11 |
| `FindQueueStatEvent` | · | · | · | · | · | · | · | ✓ | ✓ | ✓ | · | 3/11 |
| `FindRemoveFromAlivePlayers` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | · | 9/11 |
| `FindRemoveInventoryItem` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindRemoveInventoryStateValue` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindReplaceBuildingActor` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindReplicateActor` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindReset` | ✓ | ✓ | · | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | 7/11 |
| `FindSelectAndSetupMyBuildingLevel` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindSendClientAdjustment` | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `FindSendDestructionInfo` | · | · | · | ✓ | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | 7/11 |
| `FindSendRequestNow` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindServerReplicateActors` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 7/11 |
| `FindSetChannelActor` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindSetChannelActorForDestroy` | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 9/11 |
| `FindSetGamePhase` | · | · | · | · | · | · | · | · | · | · | ✓ | 1/11 |
| `FindSetInventoryStateValue` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindSetPickupItems` | ✓ | ✓ | ✓ | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `FindSetPickupTarget` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindSetState` | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 9/11 |
| `FindSetWorld` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 7/11 |
| `FindShouldAllowServerSpawnDecoVft` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindSpawnDecoVft` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindSpawnInitialSafeZone` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 7/11 |
| `FindSpawnLoot` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindStartAircraftPhase` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindStartBecomingDormant` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `FindStartStreamingAdditionalPlaylistLevel` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindStreamInMyBuilding` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `FindTickFlush` | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 8/11 |
| `FindUnEquipVehicleWeapon` | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 9/11 |
| `FindUpdateSafeZonesPhase` | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `Inline_AttemptDeriveFromURL` *(inline)* | · | · | · | · | · | · | · | ✓ | · | · | · | 1/11 |
| `Inline_CanGrappleToComponent` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_ClientOnPawnDiedCaller` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_CollectGarbagePatch` *(inline)* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | · | 10/11 |
| `Inline_EquipVehicleWeaponFunc` *(inline)* | · | · | ✓ | · | · | · | · | · | · | · | · | 1/11 |
| `Inline_GetSparseClassData` *(inline)* | · | · | · | · | ✓ | · | · | · | · | · | · | 1/11 |
| `Inline_IrisBool` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_K2_OnUnEquipCaller` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_OnAuthorityRandomUpgradeAppliedCaller` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_OnDamageServerCaller` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_OnPawnEnterSeatFunc` *(inline)* | · | · | · | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 6/11 |
| `Inline_PostUpdate` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_ServerOnAttemptInteract` *(inline)* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 11/11 |
| `Inline_SetIsInsideSafeZoneBase` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_SetOwningInventory` *(inline)* | · | · | · | · | · | · | · | · | · | · | · | 0/11 |
| `Inline_ToDeathCauseNative` *(inline)* | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 10/11 |
| `Inline_UnequipVehicleWeaponFunc` *(inline)* | · | · | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 9/11 |

**مغطاة بالكامل: 19/94** | **الخلايا: 668/1034 (64%)**

✓ = نمط/عنوان/فهرس مُتحقق آلياً · ⚠️ = global يحتاج تحققاً على الجهاز · ❓ = نمط بلا عنوان مُتحقق · · = غير محلول

## التحقق (v3) — ما تغيّر عن v2
- **422 خلية** كانت في v2 بأنماط من الجلسة الأولى بلا تحقق — فُحصت كلها الآن: الناجي منها اعْتُمد بعنوانه الفعلي، والفاشل (95 خلية) أُعيد حله أو وُثّق
- **627 نمطاً** مُتحققاً بالمطابقة الواحدة بالضبط + 11 فهرس vtable محسوب + 30 global
- تحقق cross-arch: مقارنة سمات كل دالة (نصوص مستدعاة/natives/حجم) مع FortniteClient-Win64-Shipping.exe
- تحقق native-identity: 44+ خلية تطابق عناوين natives من dumps المستخرج مباشرة