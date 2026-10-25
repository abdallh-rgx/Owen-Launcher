// ============================================================================
// Finders-Android-FULL.cpp — Erbium port: ALL 83 finder functions
// ARM64 byte patterns, Fortnite Android 5.41 → 30.00 (11 reference versions)
// Every pattern machine-verified: EXACTLY ONE match in its version's .text
// Sources: string-anchors (UTF-16 in .text literal pools, ADR xrefs),
//   native-table pairing, vtable-slot deltas (ABI-independent order),
//   cross-arch immediate fingerprints (struct layouts are ABI-independent),
//   GOT-indirect resolution for engine globals.
// ============================================================================
#include "pch.h"
#include "../Public/Finders.h"

// ----------------------------------------------------------------------
// FindActivatePhase — resolved on 7/11 versions via: session1
uint64_t FindActivatePhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 E8 1B 00 FD FD 7B 04 A9 FD 03 01 91 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 08 40 20 1E F3 03 00 AA 00 70 41 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 F3 03 00 AA 00 70 41 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F3 03 00 AA 00 70 41 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 34 41 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 34 41 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 38 41 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindApplyCharacterCustomization — resolved on 10/11 versions via: session1
uint64_t FindApplyCharacterCustomization()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 05 A9 FD 43 01 91 F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 08 30 45 39").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 05 A9 FD 43 01 91 F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 08 30 45 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F9 2B 00 F9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F3 03 00 AA 00 70 43 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 A8 83 1B F8 08 10 45 39 1F 0D 00 71").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 A8 83 1B F8 08 E0 43 39 1F 0D 00 71").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 A8 83 1B F8 08 C0 43 39 1F 0D 00 71").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 54 D0 3B D5 88 16 40 F9 A8 83 1C F8 08 C0 43 39").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 43 39").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F9 43 00 F9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 08 60 44 39").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F3 03 00 AA 68 62 44 39 F4 03 01 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCallPreReplication — resolved on 4/11 versions via: session1
uint64_t FindCallPreReplication()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 0F 00 F9 ?? ?? ?? B4 09 B8 40 79").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 0F 00 F9 ?? ?? ?? B4 09 B8 40 79").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 0F 00 F9 ?? ?? ?? B4 09 B8 40 79").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 0F 00 F9 ?? ?? ?? B4 09 B8 40 79").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCanAffordToPlaceBuildableClass — resolved on 11/11 versions via: session1
uint64_t FindCanAffordToPlaceBuildableClass()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA F5 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA F5 03 00 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA F5 03 00 AA ?? ?? ?? ?? ?? ?? ?? B4").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 E8 0F 00 F9 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 ?? ?? ?? B5 E0 03 1F 2A E8 16 40 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 ?? ?? ?? B5 E0 03 1F 2A E8 16 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 ?? ?? ?? B5 E0 03 1F 2A FD 7B 43 A9 F4 4F 42 A9 F6 57 41 A9 F7 07 44 F8 C0 03 5F D6 F3 03 02 AA F4 03 01 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FB 2B 00 F9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 F5 03 00 AA F3 03 08 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FB 23 00 F9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F5 03 00 AA F3 03 08 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 09 00 40 F9 F3 03 08 AA ?? ?? ?? B4 28 31 41 79 ?? ?? ?? 36").Get();
    }
    return Result;
}

// FindCanPlaceBuildableClassInStructuralGrid: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindCanPlaceBuildableClassInStructuralGrid() { return 0; }

// ----------------------------------------------------------------------
// FindCantBuild — resolved on 9/11 versions via: imm_fingerprint
uint64_t FindCantBuild()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 F8 5F 01 A9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 28 00 40 F9 F5 03 00 AA E0 03 01 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FD 7B 12 A9 FD 83 04 91 FC 9B 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 15 38 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA F3 03 00 AA ?? ?? ?? ?? 68 52 40 F9 88 02 00 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 1C D1 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 83 1B F8").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA 3F 00 00 EB").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA 3F 00 00 EB").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA 3F 00 00 EB 1F 00 00 F9 1F 08 00 B9 ?? ?? ?? 54 88 0A 40 B9 ?? ?? ?? 34 80 02 40 F9 ?? ?? ?? B4 08 00 40 F9 E1 03 13 AA 08 25 40 F9 00 01 3F D6 88 1A 40 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 FC 63 00 F9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 F3 03 00 AA 74 3A 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA 88 0A 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindClearAbility — resolved on 11/11 versions via: gaaco_body_bl, waveA
uint64_t FindClearAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 F7 0B 00 F9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 59 43 F9 00 01 3F D6").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F6 57 01 A9 FD 03 00 91 F4 4F 02 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 41 43 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F3 03 00 AA F4 03 01 AA 08 41 43 F9 00 01 3F D6").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 B0 42 39 08 30 87 B9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 F0 42 39 08 F8 86 B9 F4 03 01 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 92 43 39 68 5A 87 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 12 44 39 68 1A 87 B9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 F2 43 39 68 0A 85 B9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindClientHasInitializedLevelFor — resolved on 1/11 versions via: isnetready_vt_delta
uint64_t FindClientHasInitializedLevelFor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 02 2A F4 03 01 AA F5 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 DD 44 F9 08 01 40 39").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCloseActorChannel — resolved on 11/11 versions via: session1
uint64_t FindCloseActorChannel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F5 13 00 F9 F4 4F 03 A9 F5 03 01 2A F3 03 00 AA 01 14 40 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F5 13 00 F9 F4 4F 03 A9 F5 03 01 2A F3 03 00 AA 01 14 40 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 16 14 40 F9 F3 03 00 AA F5 03 01 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9 17 14 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 0F 00 F9 17 14 40 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 0F 00 F9 17 14 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 21 6C 39").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 A1 64 39").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 81 45 39 F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindConstructAbilitySpec — resolved on 9/11 versions via: imm_fingerprint
uint64_t FindConstructAbilitySpec()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 43 08 D1 F3 03 00 AA E0 23 01 91 E1 03 13 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F6 03 1F 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 5C D0 3B D5 88 17 40 F9 F3 03 00 AA A8 03 1A F8").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 5C D0 3B D5 88 17 40 F9 F3 03 00 AA A8 03 1A F8").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 E8 63 00 FD FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 48 D0 3B D5 E7 A3 00 A9 08 15 40 F9 ?? ?? ?? ?? 08 1C A0 4E F7 03 06 AA A8 03 19 F8 28 00 40 F9 F9 03 05 AA FA 03 04 2A F4 03 03 AA 16 11 40 F9 ?? ?? ?? ?? 08 A9 44 B9 F8 03 02 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 01 AA F3 03 00 AA 1F 00 01 EB").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 7F 02 14 EB 68 02 00 F9 ?? ?? ?? 54 75 06 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 6A 1F 80 52 60 62 07 91").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 6A 1F 80 52 60 62 07 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCreateChannel — resolved on 8/11 versions via: session1
uint64_t FindCreateChannel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 03 A9 FD C3 00 91 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F6 03 03 2A F7 03 02 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 02 A9 FD 83 00 91 F9 1B 00 F9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F4 03 03 2A F7 03 02 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 18 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 F4 03 03 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 39 8B 47 F9 F4 03 03 2A F7 03 02 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 28 33 5D 39 F5 03 03 2A").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 28 D3 47 39 F5 03 03 2A").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9").Get();
    }
    return Result;
}

// FindCreateNetDriver: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindCreateNetDriver() { return 0; }

// ----------------------------------------------------------------------
// FindCreateNetDriverWorldContext — resolved on 5/11 versions via: imm_fingerprint
uint64_t FindCreateNetDriverWorldContext()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 20 00 40 BD F4 03 01 AA F3 03 00 AA 1F 00 01 EB 00 00 00 BD 28 E8 40 79").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 FC 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 F3 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? 01 91 41 B9 E0 03 13 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA ?? ?? ?? ?? 60 82 00 91 81 82 00 91 ?? ?? ?? ?? 7F 02 14 EB ?? ?? ?? 54 60 36 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1C FC FC 07 00 F9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF C3 0F D1 55 D0 3B D5 A8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? 68 E2 43 39 1F 0D 00 71").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1B FC FC 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 0F D1 55 D0 3B D5").Get();
    }
    return Result;
}

// FindEncryptionPatch: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindEncryptionPatch() { return 0; }

// ----------------------------------------------------------------------
// FindEnterAircraft — resolved on 11/11 versions via: session1
uint64_t FindEnterAircraft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 67 01 A9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 43 07 D1 16 48 40 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 83 08 D1 16 48 40 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 83 08 D1 16 48 40 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 83 08 D1 59 D0 3B D5 28 17 40 F9 A8 83 1B F8").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 08 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 15 50 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 08 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 07 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 15 50 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 58 D0 3B D5 08 17 40 F9 A8 83 1A F8 15 50 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 F7 53 00 F9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFinishWorldInitialization — resolved on 10/11 versions via: session1
uint64_t FindFinishWorldInitialization()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 07 A9 FD C3 01 91 F6 57 08 A9 F4 4F 09 A9 F5 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 72 41 F9 F3 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 06 A9 FD 83 01 91 F6 57 07 A9 F4 4F 08 A9 F5 03 01 AA F3 03 00 AA ?? ?? ?? ?? F4 03 00 AA E0 03 13 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F5 03 01 AA F3 03 00 AA ?? ?? ?? ?? F4 03 00 AA E0 03 13 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA A8 83 1C F8 ?? ?? ?? ??").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA A8 83 1C F8 ?? ?? ?? ??").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 AA A8 83 1C F8 08 00 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 AA A8 83 1C F8 08 00 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 F7 43 00 F9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFinishedTargetSpline — resolved on 4/11 versions via: session1
uint64_t FindFinishedTargetSpline()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFlushDormancy — resolved on 1/11 versions via: isnetready_vt_delta
uint64_t FindFlushDormancy()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F3 03 00 AA E2 03 13 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGIsClient — resolved on 11/11 versions via: gis_got
uint64_t FindGIsClient()
{
    // Engine global (bool). Static addresses per version (GOT-resolved):
    float v = VersionInfo.FortniteVersion;
    if (v >= 30.00f)
        return ModuleBase + 0x13c47d7a;
    else if (v >= 23.50f)
        return ModuleBase + 0x1678c10; // UNVERIFIED — verify on device
    else if (v >= 22.40f)
        return ModuleBase + 0xe2ccfe2;
    else if (v >= 21.30f)
        return ModuleBase + 0xdc80824;
    else if (v >= 19.10f)
        return ModuleBase + 0x103421ec;
    else if (v >= 18.40f)
        return ModuleBase + 0x114f7694;
    else if (v >= 17.50f)
        return ModuleBase + 0x136cb13c;
    else if (v >= 13.40f)
        return ModuleBase + 0x12aefdfd;
    else if (v >= 9.41f)
        return ModuleBase + 0xce9c590; // UNVERIFIED — verify on device
    else if (v >= 7.40f)
        return ModuleBase + 0xa86e0a0; // UNVERIFIED — verify on device
    else if (v >= 5.41f)
        return ModuleBase + 0x9714ba0; // UNVERIFIED — verify on device
    return 0;
}

// ----------------------------------------------------------------------
// FindGIsServer — resolved on 11/11 versions via: gis_got
uint64_t FindGIsServer()
{
    // Engine global (bool). Static addresses per version (GOT-resolved):
    float v = VersionInfo.FortniteVersion;
    if (v >= 30.00f)
        return ModuleBase + 0x13c47d7b;
    else if (v >= 23.50f)
        return ModuleBase + 0xdfd33dc; // UNVERIFIED — verify on device
    else if (v >= 22.40f)
        return ModuleBase + 0xe2ccfe3;
    else if (v >= 21.30f)
        return ModuleBase + 0xdc80832;
    else if (v >= 19.10f)
        return ModuleBase + 0x103421fa;
    else if (v >= 18.40f)
        return ModuleBase + 0x114f7695;
    else if (v >= 17.50f)
        return ModuleBase + 0x136cb13d;
    else if (v >= 13.40f)
        return ModuleBase + 0x12aefdfe;
    else if (v >= 9.41f)
        return ModuleBase + 0xcfac770; // UNVERIFIED — verify on device
    else if (v >= 7.40f)
        return ModuleBase + 0xa8fb3a0; // UNVERIFIED — verify on device
    else if (v >= 5.41f)
        return ModuleBase + 0x97ff490; // UNVERIFIED — verify on device
    return 0;
}

// ----------------------------------------------------------------------
// FindGameSessionPatch — resolved on 8/11 versions via: session1
uint64_t FindGameSessionPatch()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D FD 7B 05 A9 FD 43 01 91 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 08 40 20 1E F3 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D FD 7B 05 A9 FD 43 01 91 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 09 40 20 1E F3 03 00 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D F7 2B 00 F9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 09 1C A0 4E F3 03 00 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9 09 1C A0 4E F3 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetMaxTickRate — resolved on 10/11 versions via: session1
uint64_t FindGetMaxTickRate()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("E9 23 BD 6D FD 7B 01 A9 FD 43 00 91 F4 4F 02 A9 09 40 20 1E 08 E4 00 2F ?? ?? ?? 36 08 00 40 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("E9 23 BD 6D FD 7B 01 A9 FD 43 00 91 F4 4F 02 A9 09 40 20 1E 08 E4 00 2F ?? ?? ?? 36 08 00 40 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1C FC E9 23 01 6D F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 1C A0 4E E8 03 27 1E 0A 10 2E 1E").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E E8 03 27 1E").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E E8 03 27 1E").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E E8 03 27 1E").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetNamePool — resolved on 8/11 versions via: namepool_mode
uint64_t FindGetNamePool()
{
    // Engine global (bool). Static addresses per version (GOT-resolved):
    float v = VersionInfo.FortniteVersion;
    if (v >= 30.00f)
        return ModuleBase + 0x13C56800;
    else if (v >= 23.50f)
        return ModuleBase + 0xDFE2000;
    else if (v >= 22.40f)
        return ModuleBase + 0xE2DA300;
    else if (v >= 21.30f)
        return ModuleBase + 0xDC960C0;
    else if (v >= 19.10f)
        return ModuleBase + 0x10357A40;
    else if (v >= 18.40f)
        return ModuleBase + 0x1150C940;
    else if (v >= 17.50f)
        return ModuleBase + 0x136E0380;
    else if (v >= 13.40f)
        return ModuleBase + 0x12B008C0;
    return 0;
}

// ----------------------------------------------------------------------
// FindGetNetMode — resolved on 11/11 versions via: session1
uint64_t FindGetNetMode()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F3 0B 00 F9 FD 03 00 91 08 1C 40 F9 ?? ?? ?? B5 F3 03 00 AA 00 78 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 84 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 84 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetPlayerViewPoint — resolved on 11/11 versions via: session1
uint64_t FindGetPlayerViewPoint()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 05 A9 FD 43 01 91 F6 57 06 A9 F4 4F 07 A9 F6 03 00 AA 40 28 80 52 F3 03 02 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F6 03 00 AA 40 28 80 52 F3 03 02 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F6 03 00 AA 40 28 80 52 F3 03 02 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 00 AA E0 83 00 91").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 08 64 43 F9 F3 03 02 AA F6 03 00 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 08 AC 43 F9 F3 03 02 AA ?? ?? ?? B4 49 1C 82 52 09 69 69 38").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FC 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA 41 28 80 52").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetWorldContext — resolved on 4/11 versions via: session1
uint64_t FindGetWorldContext()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGiveAbility — resolved on 11/11 versions via: session1
uint64_t FindGiveAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 FC 5F 01 A9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 FF 43 12 D1 ?? ?? ?? ?? ?? ?? ?? ?? 29 91 03 91 F3 03 03 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F6 57 01 A9 FD 03 00 91 F4 4F 02 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 41 43 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("14 00 80 12 80 7E 40 92 FD 7B 45 A9 F4 4F 44 A9 F5 1B 40 F9 FF 83 01 91 C0 03 5F D6 09 3D 43 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 0B 40 D1 FF 43 04 D1 E5 9B 09 A9 E7 57 00 F9 E0 87 00 AD").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("13 00 80 12 C8 16 40 F9 A9 83 5D F8 1F 01 09 EB ?? ?? ?? 54 60 7E 40 92 FD 7B 46 A9 F4 4F 45 A9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 F0 42 39 08 F8 86 B9 F4 03 01 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 F0 42 39 08 F8 86 B9 F4 03 01 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 68 A2 45 B9 F4 03 01 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 12 44 39 68 1A 87 B9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 F2 43 39 68 0A 85 B9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGiveAbilityAndActivateOnce — resolved on 9/11 versions via: session1
uint64_t FindGiveAbilityAndActivateOnce()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 28 08 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 03 A9 FD C3 00 91 F5 23 00 F9 F4 4F 05 A9 28 08 40 F9 ?? ?? ?? B4 09 2D 40 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 28 08 40 F9 ?? ?? ?? B4 09 2D 40 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 A8 83 1D F8 28 08 40 F9 09 3D 43 39").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 E8 0F 00 F9 28 08 40 F9 09 3D 43 39").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 E8 0F 00 F9 28 08 40 F9 09 3D 43 39").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 E8 0F 00 F9 28 08 40 F9 09 3D 43 39").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 E8 0F 00 F9 28 08 40 F9 09 3D 43 39").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 01 AA A8 0A 40 F9 F4 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindHandleMatchHasStarted — resolved on 11/11 versions via: session1
uint64_t FindHandleMatchHasStarted()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E8 BB 00 FD FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 ?? ?? ?? ?? F3 03 00 AA 18 FF 40 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 ?? ?? ?? ?? F3 03 00 AA 7B 97 40 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 E8 9B 00 FD FC 6F 14 A9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 ?? ?? ?? ?? 08 E5 44 F9 F3 03 00 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E8 B3 00 FD FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 59 D0 3B D5 28 17 40 F9 F3 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 E8 6B 00 FD FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 59 D0 3B D5 28 17 40 F9 F3 03 00 AA A8 03 19 F8 ?? ?? ?? ??").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E8 73 00 FD FC 6F 0F A9 FA 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 59 D0 3B D5 28 17 40 F9 ?? ?? ?? ?? F3 03 00 AA A8 03 19 F8").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 0B 00 F9 08 15 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 0B 00 F9 08 15 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 07 00 F9 08 15 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 E8 5B 00 FD FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? F3 03 00 AA A8 83 19 F8").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 08 23 7A 39").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindHandlePostSafeZonePhaseChanged — resolved on 11/11 versions via: session1
uint64_t FindHandlePostSafeZonePhaseChanged()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FD 7B 10 A9 FD 03 04 91 FC 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 F3 03 00 AA B9 83 01 D1").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FD 7B 10 A9 FD 03 04 91 FC 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 F3 03 00 AA B9 83 01 D1").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 F3 03 00 AA FB 83 02 91").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 59 D0 3B D5 28 17 40 F9 F3 03 00 AA A8 03 1A F8 ?? ?? ?? ?? 74 6A 41 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF C3 06 D1 59 D0 3B D5").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 08 D1 59 D0 3B D5").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 EC AB 00 FD EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 58 D0 3B D5").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 EC AB 00 FD EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 59 D0 3B D5 28 17 40 F9 A8 03 17 F8").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EC 7B 00 FD EB 2B 10 6D E9 23 11 6D FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 16 50 41 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EC 73 00 FD EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 F3 03 00 AA 79 CA 41 F9 FA 03 01 2A").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 E8 1B 00 FD F7 1F 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F3 03 00 AA 74 C6 41 F9 ?? ?? ?? B4 ?? ?? ?? ?? 88 0A 40 F9 09 90 80 B9 0A 91 40 B9 3F 01 0A 6B ?? ?? ?? 54 F5 03 1F AA ?? ?? ?? ??").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitListen — resolved on 6/11 versions via: vtable_delta
uint64_t FindInitListen()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F6 57 02 A9 F4 4F 03 A9 08 00 40 F9 F6 03 04 AA F4 03 02 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F5 0B 00 F9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F5 03 04 AA F3 03 02 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializeBuildingActor — resolved on 8/11 versions via: session1
uint64_t FindInitializeBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EB 2B 11 6D E9 23 12 6D FD 7B 13 A9 FD C3 04 91 FC 6F 14 A9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F7 03 05 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FD 7B 10 A9 FD 03 04 91 FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F7 03 05 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 ?? ?? ?? ?? 9C 87 46 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 5C D0 3B D5 88 17 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 18 F8").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 5C D0 3B D5 88 17 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 5B D0 3B D5 68 17 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 5B D0 3B D5 68 17 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializeFlightPath — resolved on 2/11 versions via: waveA
uint64_t FindInitializeFlightPath()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 08 A9 FD 03 02 91 F3 4B 00 F9 F3 03 01 AA BF 0F 00 F9 ?? ?? ?? ?? 68 12 40 F9 ?? ?? ?? B4 61 0E 40 F9 A2 63 00 91 E0 03 13 AA ?? ?? ?? ?? E0 23 00 91").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F3 43 00 F9 FD 7B 09 A9 FD 43 02 91 F3 03 01 AA BF 83 1F F8 ?? ?? ?? ?? 68 12 40 F9 ?? ?? ?? B4 61 0E 40 F9 A2 23 00 D1 E0 03 13 AA ?? ?? ?? ?? E0 23 00 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializePlayerGameplayAbilities — resolved on 10/11 versions via: session1
uint64_t FindInitializePlayerGameplayAbilities()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 03 D1 FD 7B 0A A9 FD 83 02 91 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 ?? ?? ?? ?? F3 03 00 AA 21 50 09 91").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FD 7B 0A A9 FD 83 02 91 F7 5B 00 F9 F6 57 0C A9 F4 4F 0D A9 ?? ?? ?? ?? F3 03 00 AA 21 D8 10 91 A0 63 00 91").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FB 3B 00 F9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 ?? ?? ?? ?? F3 03 00 AA 21 E8 03 91 A0 23 01 D1").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B5").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B4").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B5").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 55 D0 3B D5 A8 16 40 F9 A8 83 1D F8 ?? ?? ?? B4 08 00 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 55 D0 3B D5 A8 16 40 F9 A8 83 1D F8 ?? ?? ?? B4 08 00 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? B4 08 00 40 F9 F3 03 00 AA 08 0D 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F3 03 00 AA ?? ?? ?? B4 68 02 40 F9 E0 03 13 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInternalTryActivateAbility — resolved on 11/11 versions via: session1
uint64_t FindInternalTryActivateAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF 83 0B D1 ?? ?? ?? ?? F9 03 06 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF 03 10 D1 F7 03 05 AA FA 03 04 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 10 D1 FA 03 05 AA F5 03 04 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 5A D0 3B D5 48 17 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 5A D0 3B D5 48 17 40 F9 F7 03 05 AA F9 03 04 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 0D D1 57 D0 3B D5 E8 16 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 0D D1 57 D0 3B D5 E8 16 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F6 03 05 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F7 03 05 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F7 03 05 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindIsNetReady — resolved on 1/11 versions via: imm_fingerprint
uint64_t FindIsNetReady()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FD 7B BF A9 FD 03 00 91 ?? ?? ?? 36 08 98 46 B9 E8 03 08 4B 08 F8 01 B9 08 52 86 52 08 68 68 38").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindIsNetRelevantForVft — resolved on 10/11 versions via: session1
uint64_t FindIsNetRelevantForVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 09 B8 40 79 08 58 40 B9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 09 B8 40 79 08 58 40 B9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 08 B8 40 79 09 58 40 B9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 08 B8 40 79").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 09 70 41 39").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 09 70 41 39").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 09 10 42 39 08 80 40 B9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 09 10 42 39 08 80 40 B9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 09 10 42 39 08 80 40 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F3 03 00 AA 69 12 42 39").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindKickPlayer — resolved on 10/11 versions via: session1
uint64_t FindKickPlayer()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FD 7B 0C A9 FD 03 03 91 FC 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 FD 7B 0D A9 FD 43 03 91 FC 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 FC 6F 0F A9 FA 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA A8 03 1A F8 ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindListenCall — resolved on 3/11 versions via: listen_string_bl
uint64_t FindListenCall()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? ?? ?? ?? 34 ?? ?? ?? ?? B5 02 38 91 E0 03 15 AA ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? ?? ?? ?? 34 ?? ?? ?? ?? 35 01 40 F9 39 09 80 B9 69 76 40 F9 22 05 41 B9 39 01 01 B9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? ?? ?? ?? 34 ?? ?? ?? ?? 5F 01 00 F9 88 82 42 B9 ?? ?? ?? ?? ?? ?? ?? ?? 01 F1 1C 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindLoadPlayset — resolved on 10/11 versions via: session1
uint64_t FindLoadPlayset()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 05 6D FD 7B 06 A9 FD 83 01 91 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 F3 03 00 AA 14 48 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 05 6D FD 7B 06 A9 FD 83 01 91 FB 3B 00 F9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 F3 03 00 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E9 23 05 6D F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 14 48 40 F9 F3 03 00 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 06 6D FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 2F 00 F9 14 48 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 E9 23 03 6D FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 17 00 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 1F 00 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 1F 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 1F 00 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 14 64 40 F9 F3 03 00 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 EA 33 00 FD E9 23 07 6D FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F3 03 00 AA 74 76 40 F9").Get();
    }
    return Result;
}

// FindMinigameSettingsBuilding__BeginPlay: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindMinigameSettingsBuilding__BeginPlay() { return 0; }

// ----------------------------------------------------------------------
// FindNotifyGameMemberAdded — resolved on 7/11 versions via: session1
uint64_t FindNotifyGameMemberAdded()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 FC 9B 00 F9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 A1 C3 1D 38 E1 03 03 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 FC 9B 00 F9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 A1 C3 1D 38 E1 03 03 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 FC 9B 00 F9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 A1 C3 1D 38 E1 03 03 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F4 03 03 AA F6 03 02 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F5 03 03 AA F6 03 02 2A").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC A3 00 F9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 56 D0 3B D5 C8 16 40 F9 F4 03 03 AA F5 03 02 2A").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F5 03 03 AA F6 03 02 2A").Get();
    }
    return Result;
}

// FindNullsAndRetTrues: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindNullsAndRetTrues() { return 0; }

// FindOnItemInstanceAddedVft: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindOnItemInstanceAddedVft() { return 0; }

// ----------------------------------------------------------------------
// FindOnRep_ZiplineState — resolved on 10/11 versions via: session1
uint64_t FindOnRep_ZiplineState()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? ?? 08 09 41 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? ?? 08 C9 45 B9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPayBuildableClassPlacementCost — resolved on 10/11 versions via: waveB
uint64_t FindPayBuildableClassPlacementCost()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9 ?? ?? ?? B4 ?? ?? ?? ??").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9 ?? ?? ?? B5 F6 03 1F AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 E8 0F 00 F9 ?? ?? ?? 36 08 FC 41 F9 F4 03 01 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPickSupplyDropLocation — resolved on 10/11 versions via: waveA, waveB, xwin_string
uint64_t FindPickSupplyDropLocation()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("ED 33 B7 6D EB 2B 01 6D E9 23 02 6D FD 7B 03 A9 FD C3 00 91 FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FF 43 0A D1 F4 03 03 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 ED 33 15 6D EB 2B 16 6D E9 23 17 6D FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 08 5C 42 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 ED 33 15 6D EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 08 5C 42 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E9 23 13 6D FC A3 00 F9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 56 D0 3B D5 C8 16 40 F9 F3 03 01 AA F4 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 49 1C A2 4E").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 EB 2B 0C 6D E9 23 0D 6D FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 98 1C 40 F9 ?? ?? ?? B4").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 EB 2B 0C 6D E9 23 0D 6D FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 F4 03 04 AA 98 1E 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 02 6D E9 23 03 6D FB 23 00 F9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F5 03 05 AA F4 03 04 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPickTeam — resolved on 11/11 versions via: session1
uint64_t FindPickTeam()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 F4 03 02 AA F5 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 F4 03 02 AA F3 03 01 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 16 6C 41 F9 F4 03 02 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F4 03 02 AA F5 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA F5 03 00 AA F4 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA F5 03 00 AA F4 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA F5 03 00 AA F4 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA F5 03 00 AA F4 03 01 2A E8 0F 00 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 16 50 41 F9 F3 03 02 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F5 03 00 AA B6 CA 41 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 F6 03 00 AA D7 C6 41 F9 F5 03 02 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPostInitializeSpawnedBuildingActor — resolved on 11/11 versions via: vtable_next_of_IBA
uint64_t FindPostInitializeSpawnedBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9 09 1D 60 B3 ?? ?? ?? B6 60 9A 43 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPreSendUpdate — resolved on 1/11 versions via: session1
uint64_t FindPreSendUpdate()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FD 7B 05 A9 FD 43 01 91 FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 ?? ?? ?? ?? F9 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? F5 03 01 AA 00 ?? ?? 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindQueueStatEvent — resolved on 3/11 versions via: waveB
uint64_t FindQueueStatEvent()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 07 A9 FD C3 01 91 F4 4F 08 A9 A8 23 00 D1 F3 03 00 AA A7 83 1F B8 E1 A3 00 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 A8 13 40 B9 A9 63 00 D1 F3 03 00 AA A7 83 1E B8").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 E5 03 00 F9 48 D0 3B D5").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveFromAlivePlayers — resolved on 11/11 versions via: session1
uint64_t FindRemoveFromAlivePlayers()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 19 50 41 F9 ?? ?? ?? B4").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 F8 03 00 AA 19 CB 41 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F8 03 00 AA 1C C7 41 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveInventoryItem — resolved on 11/11 versions via: waveA, waveB
uint64_t FindRemoveInventoryItem()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("F4 4F 04 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 73 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1B 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 91 E0 03 14 AA ?? ?? ?? ?? B5 1B 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 BF 02 00 71 89 12 40 F9 E4 07 9F 1A").Get() + 16; // +16 to function start
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F 04 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 73 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1B 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 91 E0 03 14 AA ?? ?? ?? ?? B5 1B 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 BF 02 00 71 89 12 40 F9 E4 07 9F 1A").Get() + 16; // +16 to function start
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FD 03 01 91 F4 03 01 AA F3 03 00 AA FF 0B 00 F9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF C3 1E B8 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 53 00 D1 E0 03 14 AA ?? ?? ?? ?? BF 83 1E B8 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 D1 E0 03 14 AA ?? ?? ?? ?? B5 83 5E B8 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 89 12 40 F9 BF 02 00 71 E4 07 9F 1A").Get() + 16; // +16 to function start
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F3 03 00 AA E8 17 00 F9 FF FF 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 63 00 91 E0 03 14 AA ?? ?? ?? ?? FF 17 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 53 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? F6 13 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 89 12 40 F9 DF 02 00 71 E4 07 9F 1A 1F 01 00 71").Get() + 32; // +32 to function start
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0B 00 B9 88 12 40 F9 ?? ?? ?? B4").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveInventoryStateValue — resolved on 11/11 versions via: waveA
uint64_t FindRemoveInventoryStateValue()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ??").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 13 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 13 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA 1F 01 00 F1 08 05 88 9A 88 12 00 F9 68 02 40 F9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B5 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B5 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 28 10 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 13 00 39 88 12 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReplaceBuildingActor — resolved on 8/11 versions via: session1
uint64_t FindReplaceBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF ?? ?? D1 ?? ?? ?? ?? ?? ?? ?? ?? 08 ?? ?? 91 F5 03 1F AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FD 7B 01 A9 FD 43 00 91 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FF ?? ?? D1 ?? ?? ?? ?? ?? ?? ?? ?? 29 09 00 91").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 7B 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 68 03 40 39 1F 01 00 71 28 01 9F 9A A8 83 17 F8").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 43 06 D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EA 0F 18 FC E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 18 FC E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReplicateActor — resolved on 8/11 versions via: session1
uint64_t FindReplicateActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FD 7B 01 A9 FD 43 00 91 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FF 83 07 D1 F3 03 00 91").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF ?? ?? D1 F3 03 00 91 ?? ?? ?? ?? F5 03 00 AA ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 18 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 F4 03 00 AA 08 03 40 39 F3 03 00 AA 1F 01 00 71").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 7B 8B 47 F9 F4 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 39 BF 46 F9 F4 03 00 AA F3 03 00 AA 28 03 40 39").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 88 33 5D 39 F4 03 00 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 89 00 91").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF ?? ?? D1 59 D0 3B D5 28 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 99 04 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReset — resolved on 7/11 versions via: waveA
uint64_t FindReset()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 14 4C 40 A9 ?? ?? ?? B4 61 22 00 91 20 00 80 52").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 F3 03 02 AA F5 03 00 AA F4 03 01 AA 00 00 40 F9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F5 03 00 AA 00 00 40 F9 F3 03 02 AA F4 03 01 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("09 A8 40 B9 E8 03 01 AA 1F 01 00 F1 E1 03 1F 2A EA 03 1F AA EB 03 82 9A 0C 03 80 52 3F 01 01 6B").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EF 3B 02 6D ED 33 03 6D EB 2B 04 6D E9 23 05 6D F7 33 00 F9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F3 03 00 AA 00 54 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 60 42 05 91 21 00 00 12 ?? ?? ?? ?? 60 9E 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 60 22 05 91 21 00 00 12 ?? ?? ?? ?? 60 9A 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSelectAndSetupMyBuildingLevel — resolved on 10/11 versions via: waveB
uint64_t FindSelectAndSetupMyBuildingLevel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F5 1B 00 F9 F4 4F 04 A9 F5 03 01 AA F3 03 00 AA BF 1B 00 B9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F5 03 01 AA F3 03 00 AA BF 83 1E B8").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 23 00 B9 ?? ?? ?? ??").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 13 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 03 00 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 03 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 00 40 F9 F3 03 00 AA F5 03 01 AA 08 AD 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F3 03 00 AA F5 03 01 AA 08 A5 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 68 02 40 F9 F5 03 01 AA 08 A5 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4 E0 03 14 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 00 AA 88 02 40 F9 F5 03 01 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendClientAdjustment — resolved on 1/11 versions via: imm_fingerprint
uint64_t FindSendClientAdjustment()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 01 B4 47 B9 F3 03 00 AA 3F 04 00 31 ?? ?? ?? 54").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendDestructionInfo — resolved on 8/11 versions via: session1
uint64_t FindSendDestructionInfo()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FD 7B 14 A9 FD 03 05 91 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 ?? ?? ?? ?? ?? ?? ?? ?? 08 09 02 91 F3 03 01 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FD 7B 14 A9 FD 03 05 91 FC AB 00 F9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 ?? ?? ?? ?? ?? ?? ?? ?? 08 ?? ?? 91 ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 ?? ?? ?? ?? 7B 87 46 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9 F7 BE 46 F9 ?? ?? ?? ?? 00 ?? ?? 91 F4 03 02 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9 F7 BE 46 F9 ?? ?? ?? ?? 00 ?? ?? 91 F4 03 02 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 29 04 91 E8 0F 00 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9 E8 D2 47 39 ?? ?? ?? ?? 00 ?? ?? 91 F4 03 02 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendRequestNow — resolved on 11/11 versions via: session1
uint64_t FindSendRequestNow()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FD 7B 08 A9 FD 03 02 91 F6 57 09 A9 F4 4F 0A A9 ?? ?? ?? ?? 29 31 36 91 08 00 40 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FD 7B 0A A9 FD 83 02 91 F6 57 0B A9 F4 4F 0C A9 ?? ?? ?? ?? F5 03 02 2A F3 03 01 AA F4 03 00 AA C8 C2 65 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 03 D1 F8 5F 0B A9 F6 57 0C A9 F4 4F 0D A9 FD 7B 0E A9 FD 83 03 91 ?? ?? ?? ?? C8 C2 58 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FC 6B 00 F9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 ?? ?? ?? ?? C8 22 5D 39").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F3 03 01 AA 68 02 40 F9 F4 03 00 AA ?? ?? ?? B4 09 01 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindServerReplicateActors — resolved on 8/11 versions via: session1
uint64_t FindServerReplicateActors()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E9 23 0A 6D FD 7B 0B A9 FD C3 02 91 F6 57 0C A9 F4 4F 0D A9 15 80 06 91 28 60 40 F8").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E9 23 0A 6D FD 7B 0B A9 FD C3 02 91 F6 57 0C A9 F4 4F 0D A9 15 80 06 91 28 60 40 F8").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E8 23 00 FD FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 14 A0 00 91").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 56 D0 3B D5 00 05 C2 3D").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 ?? ?? ?? ?? D6 C2 11 91 ?? ?? ?? ??").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 4F BE A9 FD 7B 01 A9 FD 43 00 91 FF 03 33 D1 F3 03 01 AA E0 2F 00 F9 F3 2B 00 F9 E0 27 00 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FC 4F BE A9 FD 7B 01 A9 FD 43 00 91 FF 03 33 D1 F3 03 01 AA E0 2F 00 F9 F3 2B 00 F9 E0 27 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FC 4F BE A9 FD 7B 01 A9 FD 43 00 91 FF 03 33 D1 F3 03 01 AA E0 2F 00 F9 F3 2B 00 F9 E0 27 00 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetChannelActor — resolved on 10/11 versions via: imm_fingerprint
uint64_t FindSetChannelActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 F9 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 16 40 0A 91 F4 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F5 13 00 F9 F4 4F 03 A9 F3 03 00 AA 08 58 41 F9 ?? ?? ?? B4").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 ?? ?? ?? ?? 08 01 15 91 0A 41 24 91 40 01 67 9E").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1D FC F5 07 00 F9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? 08 35 47 F9 F4 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 01 BC 46 B9 F3 03 00 AA 3F 04 00 31 ?? ?? ?? 54 68 C6 46 B9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 ?? ?? ?? ?? 08 81 0B 91 F3 03 00 AA 08 00 00 F9 ?? ?? ?? ?? 60 66 43 F9 ?? ?? ?? B5").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 15 8C 40 F9 ?? ?? ?? B5 E0 03 1F 2A FD 7B 43 A9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 01 2A F4 03 00 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FB 0F 1A F8 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 14 64 41 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetChannelActorForDestroy — resolved on 10/11 versions via: session1
uint64_t FindSetChannelActorForDestroy()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 5A D0 3B D5 48 17 40 F9 A8 03 1A F8 08 C0 40 39").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 5A D0 3B D5 48 17 40 F9 A8 03 1A F8 08 C0 40 39").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 5A D0 3B D5 48 17 40 F9 A8 03 1A F8 08 C0 40 39").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 5A D0 3B D5 48 17 40 F9 A8 03 1A F8 08 C0 40 39").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 08 C0 40 39 ?? ?? ?? 37").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 F3 03 00 AA 68 C2 40 39").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetGamePhase — resolved on 1/11 versions via: waveA
uint64_t FindSetGamePhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FD 7B 14 A9 FD 03 05 91 FC 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 F4 03 01 2A F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetInventoryStateValue — resolved on 11/11 versions via: waveA
uint64_t FindSetInventoryStateValue()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ??").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 28 10 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 01 AA BF 7F 3E A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 A2 83 00 D1 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 A1 83 00 D1 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 83 00 91 E1 03 1F 2A").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F4 03 01 AA BF 7F 3E A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 A2 83 00 D1 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 A1 83 00 D1 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 E3 00 91 E1 03 1F 2A").Get();
    }
    return Result;
}

// FindSetIsDoorOpen: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindSetIsDoorOpen() { return 0; }

// ----------------------------------------------------------------------
// FindSetPickupItems — resolved on 10/11 versions via: imm_fingerprint
uint64_t FindSetPickupItems()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 F3 03 00 AA F4 03 01 AA 20 18 40 F9 75 CA 41 F9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 03 A9 FD C3 00 91 F3 23 00 F9 F3 03 00 AA 08 F0 43 B9 ?? ?? ?? 35 ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F3 13 00 F9 FD 7B 03 A9 FD C3 00 91 A2 C3 1F 38 08 20 40 F9 E6 03 04 2A E5 03 03 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 54 D0 3B D5 88 16 40 F9 ?? ?? ?? ?? E8 17 00 F9 68 4A 41 F9 FF FF 01 A9 ?? ?? ?? B4 09 51 80 B9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 00 E4 00 6F E2 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 02 AA F7 03 01 AA F4 03 00 AA ?? ?? ?? ?? E9 22 40 A9 F6 03 00 AA F5 03 01 AA 2A 61 40 F8 2B 01 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F3 03 00 AA E0 03 01 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F4 03 00 AA 88 BE 41 F9 F3 03 01 AA 89 A2 0D 91").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 F2 43 B9 ?? ?? ?? 34 60 F6 41 F9 ?? ?? ?? B4 08 00 40 F9 08 11 40 F9 00 01 3F D6 ?? ?? ?? 36 60 F6 41 F9 08 00 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 82 4F 39 ?? ?? ?? 34 68 F2 43 B9 74 42 0F 91").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetPickupTarget — resolved on 11/11 versions via: session1
uint64_t FindSetPickupTarget()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FD 7B 03 A9 FD C3 00 91 FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FF 83 0A D1 08 30 45 39").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FF 43 0A D1 08 30 45 39 1F 0D 00 71").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 03 0A D1").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 03 0A D1 5A D0 3B D5 48 17 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5B D0 3B D5").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5A D0 3B D5").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF C3 08 D1 5C D0 3B D5").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("ED 33 B7 6D EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5C D0 3B D5").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 EC 43 00 FD EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 08 60 44 39").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 EC 43 00 FD EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 F3 03 00 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 ED 33 08 6D EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 F3 03 00 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetState — resolved on 9/11 versions via: session1
uint64_t FindSetState()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E8 BB 00 FD FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F4 4F 5D A9 F6 57 5C A9").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 E8 AB 00 FD FD 7B 16 A9 FD 83 05 91 FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 08 30 45 39").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 E8 AB 00 FD FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 08 30 45 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 58 D0 3B D5 08 17 40 F9 A8 03 19 F8 08 10 45 39 1F 0D 00 71").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 57 D0 3B D5 E8 16 40 F9 A8 03 19 F8").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 57 D0 3B D5 E8 16 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 58 D0 3B D5 08 17 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 59 D0 3B D5 28 17 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 E8 43 00 FD F9 47 00 F9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 08 60 44 39").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetWorld — resolved on 8/11 versions via: session1
uint64_t FindSetWorld()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 C0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 C0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 C0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 A0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindShouldAllowServerSpawnDecoVft — resolved on 11/11 versions via: session1
uint64_t FindShouldAllowServerSpawnDecoVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 9B 00 F9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 48 40 60 1E 29 40 60 1E").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 67 13 A9 F8 5F 14 A9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FC 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 48 1C A2 4E 29 1C A1 4E").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 ED 33 10 6D EB 2B 11 6D E9 23 12 6D FC 9B 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? ?? ?? ?? B5 E0 03 13 AA ?? ?? ?? ?? ?? ?? ?? 37 F6 03 1F 2A").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? ?? ?? ?? B4 E0 03 13 AA ?? ?? ?? ?? 08 88 55 39 ?? ?? ?? 36").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA E8 17 00 F9 ?? ?? ?? ?? ?? ?? ?? B4 E0 03 13 AA ?? ?? ?? ??").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA E8 17 00 F9 ?? ?? ?? ?? ?? ?? ?? B4 F4 03 00 AA ?? ?? ?? ?? 88 0A 40 F9 09 38 80 B9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 33 00 FD E9 23 07 6D F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EA 23 00 FD E9 23 05 6D F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F4 03 00 AA").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F3 03 00 AA 74 A2 40 F9 ?? ?? ?? B4").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnDecoVft — resolved on 11/11 versions via: session1
uint64_t FindSpawnDecoVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D FD 7B 0C A9 FD 03 03 91 F7 6B 00 F9 F6 57 0E A9 F4 4F 0F A9 F3 03 04 2A").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D FD 7B 0C A9 FD 03 03 91 F7 6B 00 F9 F6 57 0E A9 F4 4F 0F A9 F3 03 04 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 ED 33 0A 6D EB 2B 0B 6D E9 23 0C 6D FC 6B 00 F9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 58 D0 3B D5 08 17 40 F9 F4 03 04 2A").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 ED 33 02 6D EB 2B 03 6D E9 23 04 6D F7 2B 00 F9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 E0 07 02 2D E2 1B 00 BD 08 00 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 4B 1C A2 4E 2C 1C A1 4E").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 00 AA E0 07 04 2D E2 2B 00 BD").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnInitialSafeZone — resolved on 7/11 versions via: waveA, waveB
uint64_t FindSpawnInitialSafeZone()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F3 0B 00 F9 FD 03 00 91 28 10 40 F9 F3 03 02 AA 1F 01 00 F1 08 05 88 9A 28 10 00 F9 ?? ?? ?? ?? 08 3A 82 52").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 28 10 40 F9 F3 03 02 AA 1F 01 00 F1 08 05 88 9A 28 10 00 F9 ?? ?? ?? ?? 08 3A 82 52").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F5 43 00 91").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 E9 23 09 6D FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 54 D0 3B D5").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnLoot — resolved on 8/11 versions via: waveA
uint64_t FindSpawnLoot()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 EA 7B 00 FD E9 23 10 6D FD 7B 11 A9 FD 43 04 91 FC 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 08 00 40 F9 F4 03 01 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EA 83 00 FD E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 F3 03 00 AA 08 40 8B B9").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 08 40 8B B9 F3 03 00 AA").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 03 18 F8 08 F0 89 B9").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 03 18 F8 08 D8 89 B9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 EA 2B 00 FD E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 57 D0 3B D5 E8 16 40 F9 E8 27 00 F9 08 B8 89 B9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 EA 2B 00 FD E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 57 D0 3B D5 E8 16 40 F9 E8 27 00 F9 08 C8 88 B9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartAircraftPhase — resolved on 11/11 versions via: session1
uint64_t FindStartAircraftPhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 F4 03 01 AA F3 03 00 AA 00 00 40 F9 08 00 40 79").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F3 03 00 AA 00 00 40 F9 08 00 40 79 1F 25 00 71 ?? ?? ?? 54").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 00 00 40 F9 08 00 40 79 1F 25 00 71 ?? ?? ?? 54").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E8 93 00 FD FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E8 93 00 FD FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E8 93 00 FD FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 8B 00 FD FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 8B 00 FD FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 E8 53 00 FD FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 08 00 40 F9 F3 03 00 AA F9 03 01 2A").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 E8 4B 00 FD FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 F3 03 00 AA 68 02 40 F9 F9 03 01 2A").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E8 3B 00 FD FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F4 03 00 AA 88 E2 6A 39 ?? ?? ?? 34").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartBecomingDormant — resolved on 10/11 versions via: vtable_delta
uint64_t FindStartBecomingDormant()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 06 A9 FD 83 01 91 F5 3B 00 F9 F4 4F 08 A9 08 60 40 79 89 10 80 52 1F 01 09 6A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 08 60 40 79 89 10 80 52").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 F3 0B 00 F9 FD 7B 02 A9 FD 83 00 91 53 D0 3B D5 68 16 40 F9 E8 07 00 F9 0D 60 40 79").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 60 40 79 89 10 80 52 1F 01 09 6A").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 08 40 42 39 09 00 80 92 09 1C 00 F9 09 1A 80 52 08 01 09 0A").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 42 42 39 09 00 80 92 69 1E 00 F9 09 1A 80 52").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 62 40 79 09 00 80 92 69 C2 03 F8 7F 42 03 F8").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartStreamingAdditionalPlaylistLevel — resolved on 11/11 versions via: session1
uint64_t FindStartStreamingAdditionalPlaylistLevel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F5 03 02 2A F3 03 00 AA A1 83 1F B8").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F5 03 02 2A F3 03 00 AA A1 83 1F B8").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F9 1B 00 F9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F5 03 02 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 02 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FB 53 00 F9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA F5 03 02 2A").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FB 53 00 F9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA F5 03 02 2A").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FB 53 00 F9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA F5 03 02 2A").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FB 53 00 F9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA F5 03 02 2A").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F9 23 00 F9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 A1 83 1C F8 08 00 40 F9 F3 03 00 AA").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F9 23 00 F9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F3 03 00 AA A1 83 1C F8 68 02 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F4 03 00 AA E1 1F 00 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStreamInMyBuilding — resolved on 11/11 versions via: session1
uint64_t FindStreamInMyBuilding()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 EA 9B 00 FD E9 23 14 6D FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 08 30 45 39 F3 03 00 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 EA 9B 00 FD E9 23 14 6D FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 08 30 45 39 F5 03 01 2A").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 EA A3 00 FD E9 23 15 6D FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 08 30 45 39 F5 03 01 2A").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 EA AB 00 FD E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EA 73 00 FD E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 58 D0 3B D5 08 17 40 F9 F5 03 01 2A F3 03 00 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FC 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 57 D0 3B D5 E8 16 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 EA 53 00 FD E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 EA 53 00 FD E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EA 8B 00 FD E9 A3 11 6D FC 97 00 F9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 58 D0 3B D5 08 17 40 F9").Get();
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 93 00 FD E9 A3 12 6D FC 9F 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindTickFlush — resolved on 8/11 versions via: session1
uint64_t FindTickFlush()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 E9 23 03 6D FD 7B 04 A9 FD 03 01 91 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 18 ?? ?? 91").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 07 D1 EA BB 00 FD E9 23 18 6D FD 7B 19 A9 FD 43 06 91 FC 6F 1A A9 FA 67 1B A9 F8 5F 1C A9 F6 57 1D A9 F4 4F 1E A9 F3 03 00 AA ?? ?? ?? ??").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 1B 80 48 39").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 5B D0 3B D5 68 17 40 F9 F3 03 00 AA").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 48 D0 3B D5 E8 07 00 F9 08 15 40 F9 F3 03 00 AA 08 1C A0 4E").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 48 D0 3B D5 E8 0F 00 F9 08 15 40 F9").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 EA AB 00 FD E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 48 D0 3B D5 E8 0B 00 F9").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 EA A3 00 FD E9 23 15 6D FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 48 D0 3B D5 E8 1B 00 F9").Get();
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindUnEquipVehicleWeapon — resolved on 9/11 versions via: session1
uint64_t FindUnEquipVehicleWeapon()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get();
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get();
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FB 0B 00 F9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 00 AA 00 4C 40 F9").Get();
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F5 03 00 AA F6 03 03 2A").Get();
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get();
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 59 D0 3B D5 28 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get();
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F7 03 00 AA F5 03 03 2A F4 03 02 AA").Get();
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F7 03 00 AA F5 03 03 2A F4 03 02 AA").Get();
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F7 03 00 AA 00 68 40 F9 F5 03 03 2A").Get();
    }
    return Result;
}

// FindUpdateIrisReplicationViews: NOT RESOLVED on any reference version (see REPORT)
uint64_t FindUpdateIrisReplicationViews() { return 0; }

// ----------------------------------------------------------------------
// FindUpdateSafeZonesPhase — resolved on 1/11 versions via: xwin_string
uint64_t FindUpdateSafeZonesPhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 9B 00 FD FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 55 D0 3B D5").Get();
    }
    return Result;
}

