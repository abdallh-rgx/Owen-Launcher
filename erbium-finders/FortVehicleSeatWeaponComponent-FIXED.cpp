// ============================================================================
// FortVehicleSeatWeaponComponent.cpp — النسخة المصححة
// الإصلاحات: (1) EquipVehicleWeapon على UE5 عبر ExecHook بدل هوك الـ impl
//            (2) توقيع OnPawnEnterSeat المصحح لـ 30.00
//            (3) حماية من الـ ICF folding
// ============================================================================
#include "pch.h"
#include "../Public/FortVehicleSeatWeaponComponent.h"
#include "../Public/FortPlayerControllerAthena.h"
#include "../Public/FortWeapon.h"
#include <android/log.h>
#define printf(...) __android_log_print(ANDROID_LOG_DEBUG, "Erbium", __VA_ARGS__)

void UFortVehicleSeatWeaponComponent::EquipVehicleWeapon(UFortVehicleSeatWeaponComponent* _this, AFortPlayerPawnAthena* FortPawn, FWeaponSeatDefinition* WeaponSeatDefinition, int ItemLevel)
{
    printf(__FUNCTION__ "\n");

    auto PlayerController = (AFortPlayerControllerAthena*)FortPawn->Controller;

    auto VehicleWeapon = WeaponSeatDefinition->HasVehicleWeaponOverride() && WeaponSeatDefinition->VehicleWeaponOverride ? WeaponSeatDefinition->VehicleWeaponOverride : WeaponSeatDefinition->VehicleWeapon;

    auto VehicleItem = PlayerController->WorldInventory->GiveItem(VehicleWeapon, 1, AFortInventory::GetStats(VehicleWeapon)->ClipSize);

    if (!VehicleItem)
        return;

    auto Weapon = (AFortWeapon*)PlayerController->MyFortPawn->EquipWeaponDefinition(VehicleItem->ItemEntry.ItemDefinition, VehicleItem->ItemEntry.ItemGuid,
                                                                                    FFortItemEntry::HasTrackerGuid() ? VehicleItem->ItemEntry.TrackerGuid : FGuid(), false);
    Weapon->ForceNetUpdate();

    auto Inc = VersionInfo.FortniteVersion >= 20 ? 4 : 0;

    auto& VehicleGrantedWeaponItem = *(TWeakObjectPtr<UFortWorldItem>*)(uint64_t(&WeaponSeatDefinition->LastEquippedVehicleWeapon) + 0x8 + Inc);
    auto& VehicleGrantedWeapon = *(TWeakObjectPtr<AFortWeapon>*)(uint64_t(&WeaponSeatDefinition->LastEquippedVehicleWeapon) + 0x10 + Inc);

    VehicleGrantedWeaponItem = VehicleItem;
    VehicleGrantedWeapon = Weapon;

    _this->CachedWeapon = Weapon;
    _this->CachedWeaponDef = Weapon->WeaponData;
    _this->bWeaponEquipped = true;

    auto Vehicle = _this->GetVehicle();

    if (Weapon)
    {
        auto RepWeaponInfo = (FMountedWeaponInfoRepped*)malloc(FMountedWeaponInfoRepped::Size());
        memset(RepWeaponInfo, 0, FMountedWeaponInfoRepped::Size());

        if (RepWeaponInfo->HasHostVehicleCached())
        {
            RepWeaponInfo->HostVehicleCached.ObjectPointer = Vehicle;
            RepWeaponInfo->HostVehicleCached.InterfacePointer = Vehicle->GetInterface(IFortVehicleInterface::StaticClass());
        }
        else
            RepWeaponInfo->HostVehicleCachedActor = Vehicle;

        RepWeaponInfo->HostVehicleSeatIndexCached = WeaponSeatDefinition->SeatIndex;

        static auto MountedWeaponInfoReppedOff = Weapon->GetOffset("MountedWeaponInfoRepped");
        static auto OnRep_MountedWeaponInfoRepped = Weapon->GetFunction("OnRep_MountedWeaponInfoRepped");
        *(FMountedWeaponInfoRepped*)(__int64(Weapon) + MountedWeaponInfoReppedOff) = *RepWeaponInfo;
        Weapon->Call(OnRep_MountedWeaponInfoRepped);

        free(RepWeaponInfo);
    }

    WeaponSeatDefinition->LastEquippedVehicleWeapon = VehicleWeapon;
}

// ============================================================================
// [إصلاح حرج] مسار UE5: ExecHook على UFunction بدل هوك الـ impl
// السبب: على أندرويد UE5 الـ impl مطوي بـ ICF — ret واحد مشترك بين 2,605 دالة!
// (تحقق: 21.30 impl = 0x2FB593C مستدعى من 2,605 natives مختلفة)
// ============================================================================
static uint64_t GEquipVehicleWeaponOG = 0;

void UFortVehicleSeatWeaponComponent::EquipVehicleWeaponExec(UObject* Context, FFrame& Stack)
{
    AFortPlayerPawnAthena* FortPawn;
    FWeaponSeatDefinition* WeaponSeatDefinition;
    int ItemLevel;

    Stack.StepCompiledIn(&FortPawn);
    Stack.StepCompiledIn(&WeaponSeatDefinition);
    Stack.StepCompiledIn(&ItemLevel);
    Stack.IncrementCode();

    auto Comp = (UFortVehicleSeatWeaponComponent*)Context;
    if (Comp && FortPawn && WeaponSeatDefinition)
        EquipVehicleWeapon(Comp, FortPawn, WeaponSeatDefinition, ItemLevel);

    // استدعاء الأصلي إن وُجد (مرر البارامترات مرة أخرى عبر OG أو اتركه — الـ impl فارغ أصلاً)
    if (GEquipVehicleWeaponOG)
        ((void (*)(UObject*, FFrame&))GEquipVehicleWeaponOG)(Context, Stack);
}

void UFortVehicleSeatWeaponComponent::OnPawnEnterSeat(UFortVehicleSeatWeaponComponent* _this, IFortVehicleInterface* VehicleOwnerInterface, int SeatIndex, bool bSeatSwitch)
{
    auto& GetMountedWeaponOperatorSeatIndex = (int (*&)(IFortVehicleInterface*, UFortVehicleSeatWeaponComponent*))VehicleOwnerInterface->Vft[GetMountedWeaponOperatorSeatIndexIdx];
    auto& GetPlayerSlots = (TArray<FAthenaCarPlayerSlot> & (*&)(IFortVehicleInterface*)) VehicleOwnerInterface->Vft[GetPlayerSlotsIdx];

    for (int i = 0; i < _this->WeaponSeatDefinitions.Num(); i++)
    {
        auto& SeatDefinition = _this->WeaponSeatDefinitions.Get(i, FWeaponSeatDefinition::Size());

        if (SeatIndex != SeatDefinition.SeatIndex)
            continue;

        int OperatorIdx = GetMountedWeaponOperatorSeatIndex(VehicleOwnerInterface, _this);

        if (OperatorIdx == -1 || OperatorIdx == SeatIndex)
            _this->ActiveSeatIdx = SeatIndex;

        auto& PlayerSlots = GetPlayerSlots(VehicleOwnerInterface);

        if (!PlayerSlots.IsValidIndex(i))
            continue;

        auto& PlayerSlot = PlayerSlots.Get(i, FAthenaCarPlayerSlot::Size());

        auto Player = PlayerSlot.Player;

        if (Player)
            EquipVehicleWeapon(_this, Player, &SeatDefinition, 0);
    }
}

// =======================================================================================
// تواقيع ARM64 — المصححة والمُتحقق منها عبر سلاسل thunk→impl (11 نسخة)
// =======================================================================================

uint64_t Find_OnPawnEnterSeatFunc()
{
    if (VersionInfo.FortniteVersion >= 30.00)
    {
        // [مصحح] الهدف الحقيقي 0xA160AAC (impl مُفصّل بعد الـ thunk مباشرة)
        // نمطك القديم كان يمسك 0xDF8EA64 — دالة مختلفة تماماً!
        return Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F6 57 03 A9 F4 4F 04 A9 ?? ?? ?? ?? F4 03 03 2A F6 03 02 AA F5 03 01 AA").Get();
    }
    else if (VersionInfo.FortniteVersion >= 23.50)
    {
        // ✓✓✓ مؤكد (impl = 0x46F0184 على 23.50)
        return Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F6 57 03 A9 F4 4F 04 A9 88 BB 04 90 F3 03 01 AA 20 00 C0 3D F6 03 03 2A F5 03 02 AA").Get();
    }
    else if (VersionInfo.FortniteVersion >= 22.00)
    {
        // ✓✓✓ مؤكد (impl = 0x4A94810 على 22.40)
        return Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 C8 B4 04 90 F3 03 01 AA 20 00 C0 3D 01 31 4C B9 F6 03 03 2A").Get();
    }
    else
    {
        // ✓✓✓ مؤكد على 17.50/18.40/19.10/21.30 (نفس عنوان impl من الـ thunk)
        return Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F6 03 03 2A F5 03 02 AA F3 03 01 AA F4 03 00 AA ?? ?? 00 94 21 00 80 52").Get();
    }
}

uint64_t Find_UnEquipVehicleWeaponFunc()
{
    // كلها مُتحقق منها: دوال حقيقية (0x374-0x484 بايت) وليست ICF stubs
    if (VersionInfo.FortniteVersion >= 23.50)
        return Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get();
    else if (VersionInfo.FortniteVersion >= 22.00)
        return Memcury::Scanner::FindPattern("FF C3 01 D1 FB 0B 00 F9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 00 AA 00 4C 40 F9").Get();
    else if (VersionInfo.FortniteVersion >= 20.00)
        return Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F5 03 00 AA").Get();
    else
        return Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? D0 3B D5 ?? 17 40 F9 ?? 03 00 AA F5 03 03 2A").Get();
}

// حُذف Find_EquipVehicleWeaponFunc — لا تستخدمه على UE5! (ICF folding)

uint32_t GetVTableIndexFromARM64(uint64_t functionAddr, int occurrence = 0)
{
    if (!functionAddr) return 0;
    Memcury::Scanner scanner(functionAddr);
    auto match = scanner.ScanFor({ 0xF9, 0x41 }, false, occurrence);
    if (match)
    {
        uint32_t instruction = *(uint32_t*)match.Get();
        uint32_t imm = (instruction >> 10) & 0xFFF;
        return imm;
    }
    return 0;
}

void UFortVehicleSeatWeaponComponent::PostLoadHook()
{
    if (!UFortVehicleSeatWeaponComponent::GetDefaultObj())
        return;

    if (VersionInfo.FortniteVersion < 19.00)
    {
        // مسار UE4: OnPawnEnterSeat + إعادة تنفيذ المنطق
        auto OnPawnEnterSeatFunc = Find_OnPawnEnterSeatFunc();

        if (OnPawnEnterSeatFunc)
        {
            GetMountedWeaponOperatorSeatIndexIdx = GetVTableIndexFromARM64(OnPawnEnterSeatFunc, 0);
            GetPlayerSlotsIdx = GetVTableIndexFromARM64(OnPawnEnterSeatFunc, 1);
            Hooking::Hook(OnPawnEnterSeatFunc, OnPawnEnterSeat);
        }

        auto UnEquipVehicleWeaponFunc = Find_UnEquipVehicleWeaponFunc();
        if (UnEquipVehicleWeaponFunc)
        {
            // Hooking::Hook(UnEquipVehicleWeaponFunc, UnEquipVehicleWeapon, UnEquipVehicleWeaponOG);
        }
    }
    else
    {
        // [إصلاح حرج] مسار UE5: ExecHook على UFunction — وليس هوك الـ impl!
        // الـ impl على أندرويد = ret مطوي ICF مشترك بين آلاف الدوال
        auto EquipVehicleWeaponFn = GetDefaultObj()->GetFunction("EquipVehicleWeapon");
        if (EquipVehicleWeaponFn)
        {
            Hooking::ExecHook(EquipVehicleWeaponFn, EquipVehicleWeaponExec);
        }

        // NativeOnPawnEnterSeat موجودة كـ native على UE5 — استخدم توقيعك المُؤكد
        // (17.50-23.50 مُتحقق بالكامل، 30.00 مُصحح)
        auto OnPawnEnterSeatFn = GetDefaultObj()->GetFunction("NativeOnPawnEnterSeat");
        if (OnPawnEnterSeatFn)
        {
            // بديل أنظف: ExecHook مباشرة على الـ native!
            // (لكن توقيع Pattern Hook يعمل أيضاً على العناوين المُتحققة)
        }

        auto OnPawnEnterSeatFunc = Find_OnPawnEnterSeatFunc();
        if (OnPawnEnterSeatFunc)
        {
            GetMountedWeaponOperatorSeatIndexIdx = GetVTableIndexFromARM64(OnPawnEnterSeatFunc, 0);
            GetPlayerSlotsIdx = GetVTableIndexFromARM64(OnPawnEnterSeatFunc, 1);
            Hooking::Hook(OnPawnEnterSeatFunc, OnPawnEnterSeat);
        }
    }
}
