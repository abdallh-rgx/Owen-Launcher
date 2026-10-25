// ============================================================================
// Finders-Android-FULL-v3.cpp — Erbium port: ALL finder functions
// 83 Finders.cpp functions + 17 inline resolutions from OTHER source files
// (BuildingSMActor / FortInventory / FortPlayerControllerAthena / FortPlayerPawnAthena /
//  FortVehicleSeatWeaponComponent / Misc.cpp / FortGameMode / FortLootPackage /
//  FortPhysicsPawn.cpp / ErbiumClient)
//
// ARM64 byte patterns, Fortnite Android 5.41 → 30.00 (11 reference versions)
// كل نمط مُتحقق آلياً: مطابقة واحدة بالضبط في .text الخاص بنسخته (P0)
// + تحقق cross-arch مقابل FortniteClient-Win64-Shipping.exe (P1)
// + تحقق native-identity مقابل dumps المُستخرج (P2)
//
// الجديد في v3:
//  1. تصحيح شامل: 422 خلية من v2 كانت أنماطاً غير مُتحققة — أعيد فحصها كلها
//     (513 ✓ / 95 معطوبة وُصلحت أو وُثّقت)
//  2. دوال خارج Finders.cpp (17 دالة inline)
//  3. فهارس vtable محسوبة من ثنائيات أندرويد فعلياً (ليست تخميناً)
//  4. fallbacks انعكاسية وقت التشغيل للدوال المسجلة كـ UFunction
// ============================================================================
#include "pch.h"
#include "../Public/Finders.h"

// ---------------------------------------------------------------------------
// Runtime reflection fallback: للدوال المسجلة كـ UFunction natives، يمكن
// الحصول على العنوان من نظام الانعكاس مباشرة (أضمن من أي نمط ثابت):
//   auto Fn = DefaultObj->GetFunction("Name"); if (Fn) return Fn->Func;
// ---------------------------------------------------------------------------

// ============================================================================
// PART 1 — الدوال الـ83 الأصلية (Finders.cpp)
// ============================================================================

// ----------------------------------------------------------------------
// FindActivatePhase — 7/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:3076
uint64_t FindActivatePhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 E8 1B 00 FD FD 7B 04 A9 FD 03 01 91 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 08 40 20 1E F3 03 00 AA 00 70 41 F9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 F3 03 00 AA 00 70 41 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F3 03 00 AA 00 70 41 F9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 34 41 F9").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 34 41 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 17 00 F9 00 38 41 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindApplyCharacterCustomization — 9/11 نسخة | استراتيجيات: P0-verified-session1, repick-size-matched | المصدر: Finders.cpp:810
uint64_t FindApplyCharacterCustomization()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 05 A9 FD 43 01 91 F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 08 30 45 39").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F9 2B 00 F9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F3 03 00 AA 00 70 43 F9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? 08 4D 40 F9 ?? ?? ?? ?? E7 03 02 AA E6 03 01 AA 84 F0 37 91 02 11 00 91 63 00 80 52").Get(); // repick-size-matched
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 A8 83 1B F8 08 E0 43 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 A8 83 1B F8 08 C0 43 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 54 D0 3B D5 88 16 40 F9 A8 83 1C F8 08 C0 43 39").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 43 39").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F9 43 00 F9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 08 60 44 39").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F3 03 00 AA 68 62 44 39 F4 03 01 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCallPreReplication — 4/11 نسخة | استراتيجيات: P0-verified-session1, tst30+bfi+crawl | المصدر: Finders.cpp:1732
uint64_t FindCallPreReplication()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? B4 FD 7B BB A9 F9 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 09 B8 40 79").Get(); // tst30+bfi+crawl
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 01 A9 FD 43 00 91 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 ?? ?? ?? B4 09 B8 40 79 F3 03 01 AA").Get(); // tst30+bfi+crawl
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? B4 09 B8 40 79 08 58 40 B9").Get(); // tst30+bfi+crawl
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 0F 00 F9 ?? ?? ?? B4 09 B8 40 79").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCanAffordToPlaceBuildableClass — 10/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2619
uint64_t FindCanAffordToPlaceBuildableClass()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA F5 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA F5 03 00 AA ?? ?? ?? ?? ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 E8 0F 00 F9 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 ?? ?? ?? B5 E0 03 1F 2A E8 16 40 F9").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 ?? ?? ?? B5 E0 03 1F 2A E8 16 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 ?? ?? ?? B5 E0 03 1F 2A FD 7B 43 A9 F4 4F 42 A9 F6 57 41 A9 F7 07 44 F8 C0 03 5F D6 F3 03 02 AA F4 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 ?? ?? ?? B4 F3 03 02 AA F4 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FB 2B 00 F9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 F5 03 00 AA F3 03 08 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FB 23 00 F9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F5 03 00 AA F3 03 08 AA").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 09 00 40 F9 F3 03 08 AA ?? ?? ?? B4 28 31 41 79 ?? ?? ?? 36").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCantBuild — 9/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:1112
uint64_t FindCantBuild()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 F8 5F 01 A9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 28 00 40 F9 F5 03 00 AA E0 03 01 AA").Get(); // imm_fingerprint
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FD 7B 12 A9 FD 83 04 91 FC 9B 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 15 38 40 F9 ?? ?? ?? B4").Get(); // imm_fingerprint
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA F3 03 00 AA ?? ?? ?? ?? 68 52 40 F9 88 02 00 F9").Get(); // imm_fingerprint
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 1C D1 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 83 1B F8").Get(); // imm_fingerprint
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA 3F 00 00 EB").Get(); // imm_fingerprint
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA 3F 00 00 EB").Get(); // imm_fingerprint
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA 3F 00 00 EB 1F 00 00 F9 1F 08 00 B9 ?? ?? ?? 54 88 0A 40 B9 ?? ?? ?? 34 80 02 40 F9 ?? ?? ?? B4 08 00 40 F9 E1 03 13 AA 08 25 40 F9 00 01 3F D6 88 1A 40 B9").Get(); // imm_fingerprint
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 FC 63 00 F9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 F3 03 00 AA 74 3A 40 F9").Get(); // imm_fingerprint
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA 88 0A 40 F9").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindClearAbility — 11/11 نسخة | استراتيجيات: gaaco_body_bl, waveA | المصدر: Finders.cpp:1513
uint64_t FindClearAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 F7 0B 00 F9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 59 43 F9 00 01 3F D6").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F6 57 01 A9 FD 03 00 91 F4 4F 02 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 41 43 F9").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F3 03 00 AA F4 03 01 AA 08 41 43 F9 00 01 3F D6").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 B0 42 39 08 30 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // gaaco_body_bl
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // gaaco_body_bl
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // gaaco_body_bl
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // gaaco_body_bl
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 F0 42 39 08 F8 86 B9 F4 03 01 AA").Get(); // gaaco_body_bl
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 92 43 39 68 5A 87 B9").Get(); // gaaco_body_bl
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 12 44 39 68 1A 87 B9").Get(); // gaaco_body_bl
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 F2 43 39 68 0A 85 B9").Get(); // gaaco_body_bl
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindClientHasInitializedLevelFor — 1/11 نسخة | استراتيجيات: isnetready_vt_delta | المصدر: Finders.cpp:2017
uint64_t FindClientHasInitializedLevelFor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 02 2A F4 03 01 AA F5 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 DD 44 F9 08 01 40 39").Get(); // isnetready_vt_delta
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCloseActorChannel — 10/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40, transfer:21.30 | المصدر: Finders.cpp:1986
uint64_t FindCloseActorChannel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F5 13 00 F9 F4 4F 03 A9 F5 03 01 2A F3 03 00 AA 01 14 40 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 16 14 40 F9 F3 03 00 AA F5 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9 17 14 40 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9 17 14 40 F9").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 0F 00 F9 17 14 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F4 03 01 2A E8 0F 00 F9 17 14 40 F9").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 2A E8 0F 00 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 21 6C 39").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 A1 64 39").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? ?? 08 81 45 39 F3 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindConstructAbilitySpec — 9/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:735
uint64_t FindConstructAbilitySpec()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 43 08 D1 F3 03 00 AA E0 23 01 91 E1 03 13 AA").Get(); // imm_fingerprint
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F6 03 1F 2A").Get(); // imm_fingerprint
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 5C D0 3B D5 88 17 40 F9 F3 03 00 AA A8 03 1A F8").Get(); // imm_fingerprint
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 5C D0 3B D5 88 17 40 F9 F3 03 00 AA A8 03 1A F8").Get(); // imm_fingerprint
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 E8 63 00 FD FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 48 D0 3B D5 E7 A3 00 A9 08 15 40 F9 ?? ?? ?? ?? 08 1C A0 4E F7 03 06 AA A8 03 19 F8 28 00 40 F9 F9 03 05 AA FA 03 04 2A F4 03 03 AA 16 11 40 F9 ?? ?? ?? ?? 08 A9 44 B9 F8 03 02 AA").Get(); // imm_fingerprint
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 01 AA F3 03 00 AA 1F 00 01 EB").Get(); // imm_fingerprint
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 7F 02 14 EB 68 02 00 F9 ?? ?? ?? 54 75 06 40 F9").Get(); // imm_fingerprint
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 6A 1F 80 52 60 62 07 91").Get(); // imm_fingerprint
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 AA 88 02 40 F9 F3 03 00 AA 6A 1F 80 52 60 62 07 91").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCreateChannel — 8/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:1918
uint64_t FindCreateChannel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 03 A9 FD C3 00 91 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F6 03 03 2A F7 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 02 A9 FD 83 00 91 F9 1B 00 F9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F4 03 03 2A F7 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 18 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 F4 03 03 2A").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 39 8B 47 F9 F4 03 03 2A F7 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 28 33 5D 39 F5 03 03 2A").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9 28 D3 47 39 F5 03 03 2A").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FB 1B 00 F9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 17 00 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindCreateNetDriverWorldContext — 5/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:241
uint64_t FindCreateNetDriverWorldContext()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 20 00 40 BD F4 03 01 AA F3 03 00 AA 1F 00 01 EB 00 00 00 BD 28 E8 40 79").Get(); // imm_fingerprint
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 FC 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 F3 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? 01 91 41 B9 E0 03 13 AA").Get(); // imm_fingerprint
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA ?? ?? ?? ?? 60 82 00 91 81 82 00 91 ?? ?? ?? ?? 7F 02 14 EB ?? ?? ?? 54 60 36 40 F9").Get(); // imm_fingerprint
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1C FC FC 07 00 F9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF C3 0F D1 55 D0 3B D5 A8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? 68 E2 43 39 1F 0D 00 71").Get(); // imm_fingerprint
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1B FC FC 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 0F D1 55 D0 3B D5").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindEnterAircraft — 11/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2099
uint64_t FindEnterAircraft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 67 01 A9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 43 07 D1 16 48 40 F9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 83 08 D1 16 48 40 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 83 08 D1 16 48 40 F9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 83 08 D1 59 D0 3B D5 28 17 40 F9 A8 83 1B F8").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 08 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 15 50 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 08 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 07 D1 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 15 50 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 58 D0 3B D5 08 17 40 F9 A8 83 1A F8 15 50 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 F7 53 00 F9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 F4 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFinishWorldInitialization — 10/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: Finders.cpp:2978
uint64_t FindFinishWorldInitialization()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 07 A9 FD C3 01 91 F6 57 08 A9 F4 4F 09 A9 F5 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 72 41 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 06 A9 FD 83 01 91 F6 57 07 A9 F4 4F 08 A9 F5 03 01 AA F3 03 00 AA ?? ?? ?? ?? F4 03 00 AA E0 03 13 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F5 03 01 AA F3 03 00 AA ?? ?? ?? ?? F4 03 00 AA E0 03 13 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA A8 83 1C F8 ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA A8 83 1C F8 ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA A8 83 1C F8 ?? ?? ?? ??").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 AA A8 83 1C F8 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 F7 43 00 F9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F5 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFinishedTargetSpline — 3/11 نسخة | استراتيجيات: P0-verified-session1, string:FinishedTargetSpline | المصدر: Finders.cpp:944
uint64_t FindFinishedTargetSpline()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 03 D1 E8 5B 00 FD FD FB 0B A9 FD E3 02 91 F7 67 00 F9 F6 57 0D A9 F4 4F 0E A9 16 A0 13 91").Get(); // string:FinishedTargetSpline
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EF 3B B9 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F4 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindFlushDormancy — 1/11 نسخة | استراتيجيات: isnetready_vt_delta | المصدر: Finders.cpp:2073
uint64_t FindFlushDormancy()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F3 03 00 AA E2 03 13 AA").Get(); // isnetready_vt_delta
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGIsClient — 11/11 نسخة | استراتيجيات: gis_got | المصدر: Finders.cpp:5
uint64_t FindGIsClient()
{
    // Engine global — عناوين ثابتة لكل نسخة (مُتحققة عبر GOT relocations):
    float v = VersionInfo.FortniteVersion;
    if (v >= 30.00f)
        return ModuleBase + 0x13c47d7a;
    else if (v >= 23.50f)
        return ModuleBase + 0x1678c10; // ⚠️ يحتاج تحققاً على الجهاز
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
        return ModuleBase + 0xce9c590; // ⚠️ يحتاج تحققاً على الجهاز
    else if (v >= 7.40f)
        return ModuleBase + 0xa86e0a0; // ⚠️ يحتاج تحققاً على الجهاز
    else if (v >= 5.41f)
        return ModuleBase + 0x9714ba0; // ⚠️ يحتاج تحققاً على الجهاز
    return 0;
}

// ----------------------------------------------------------------------
// FindGIsServer — 11/11 نسخة | استراتيجيات: gis_got | المصدر: Finders.cpp:65
uint64_t FindGIsServer()
{
    // Engine global — عناوين ثابتة لكل نسخة (مُتحققة عبر GOT relocations):
    float v = VersionInfo.FortniteVersion;
    if (v >= 30.00f)
        return ModuleBase + 0x13c47d7b;
    else if (v >= 23.50f)
        return ModuleBase + 0xdfd33dc; // ⚠️ يحتاج تحققاً على الجهاز
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
        return ModuleBase + 0xcfac770; // ⚠️ يحتاج تحققاً على الجهاز
    else if (v >= 7.40f)
        return ModuleBase + 0xa8fb3a0; // ⚠️ يحتاج تحققاً على الجهاز
    else if (v >= 5.41f)
        return ModuleBase + 0x97ff490; // ⚠️ يحتاج تحققاً على الجهاز
    return 0;
}

// ----------------------------------------------------------------------
// FindGameSessionPatch — 8/11 نسخة | استراتيجيات: P0-verified-session1, transfer:21.30 | المصدر: Finders.cpp:1544
uint64_t FindGameSessionPatch()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D FD 7B 05 A9 FD 43 01 91 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 08 40 20 1E F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D FD 7B 05 A9 FD 43 01 91 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 09 40 20 1E F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EB 2B 03 6D E9 23 04 6D F7 2B 00 F9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 09 1C A0 4E F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9 09 1C A0 4E F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9 09 1C A0 4E F3 03 00 AA").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9 09 1C A0 4E F3 03 00 AA").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9 09 1C A0 4E F3 03 00 AA").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 56 D0 3B D5 C8 16 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetMaxTickRate — 10/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40, transfer:22.40 | المصدر: Finders.cpp:651
uint64_t FindGetMaxTickRate()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("E9 23 BD 6D FD 7B 01 A9 FD 43 00 91 F4 4F 02 A9 09 40 20 1E 08 E4 00 2F ?? ?? ?? 36 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("E9 23 BD 6D FD 7B 01 A9 FD 43 00 91 F4 4F 02 A9 09 40 20 1E 08 E4 00 2F ?? ?? ?? 36 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1C FC E9 23 01 6D F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 1C A0 4E E8 03 27 1E 0A 10 2E 1E").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1C FC E9 23 01 6D F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 1C A0 4E E8 03 27 1E 0A 10 2E 1E").Get(); // transfer:22.40
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1C FC E9 23 01 6D F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 1C A0 4E E8 03 27 1E 0A 10 2E 1E").Get(); // transfer:22.40
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("EB 2B BA 6D E9 23 01 6D F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 09 1C A0 4E").Get(); // transfer:18.40
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E E8 03 27 1E").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 1B FC E9 23 01 6D F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 09 1C A0 4E F3 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetNamePool — 8/11 نسخة | استراتيجيات: namepool_mode | المصدر: Finders.cpp:2243
uint64_t FindGetNamePool()
{
    // Engine global — عناوين ثابتة لكل نسخة (مُتحققة عبر GOT relocations):
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
// FindGetNetMode — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:22.40, transfer:9.41 | المصدر: Finders.cpp:125
uint64_t FindGetNetMode()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F3 0B 00 F9 FD 03 00 91 08 1C 40 F9 ?? ?? ?? B5 F3 03 00 AA 00 78 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // transfer:22.40
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // transfer:22.40
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // transfer:22.40
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B5 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ?? ?? ?? ?? ?? 60 00 80 52").Get(); // transfer:22.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 60 00 80 52 C0 03 5F D6 08 78 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 70 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get(); // transfer:9.41
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("08 1C 40 F9 ?? ?? ?? B4 E0 07 00 32 C0 03 5F D6 08 84 40 F9 ?? ?? ?? B4 E0 03 08 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetPlayerViewPoint — 11/11 نسخة | استراتيجيات: P0-verified-session1, native:GetPlayerViewPoint:thunk | المصدر: Finders.cpp:2153
uint64_t FindGetPlayerViewPoint()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 05 A9 FD 43 01 91 F6 57 06 A9 F4 4F 07 A9 F6 03 00 AA 40 28 80 52 F3 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 03 A9 FD C3 00 91 F6 57 04 A9 F4 4F 05 A9 F5 03 01 AA F3 03 01 AA F4 03 00 AA BF 8E 03 F8 A8 82 5E F8 BF 06 00 F9 ?? ?? ?? B4 61 0E 40 F9 E2 63 00 91 E0 03 13 AA ?? ?? ?? ?? 68 12 40 F9 76 1E 40 F9 BF 7E 00 A9 ?? ?? ?? B4 61 0E 40 F9 E2 03 00 91 E0 03 13 AA ?? ?? ?? ?? 68 1E 40 F9 DF 02 00 F1 6A 12 40 F9 E9 63 00 91 21 01 96 9A E9 03 00 91 1F 01 00 F1 E0 03 14 AA 22 01 88 9A 5F 01 00 F1 48 05 8A 9A 68 12 00 F9 88 02 40 F9 08 E5 43 F9").Get(); // native:GetPlayerViewPoint:thunk
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F6 03 00 AA 40 28 80 52 F3 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 01 AA F4 03 01 AA F3 03 00 AA A8 83 1D F8 BF 8E 03 F8 A8 82 5E F8 BF 06 00 F9 ?? ?? ?? B4 81 0E 40 F9 E2 83 00 91 E0 03 14 AA ?? ?? ?? ?? 88 1E 40 F9 BF 7E 00 A9 89 12 40 F9 EA 83 00 91 1F 01 00 F1 55 01 88 9A ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? 88 1E 40 F9 89 12 40 F9 EA 23 00 91 E0 03 13 AA 1F 01 00 F1 42 01 88 9A 3F 01 00 F1 28 05 89 9A 88 12 00 F9 68 02 40 F9 E1 03 15 AA 08 DD 43 F9").Get(); // native:GetPlayerViewPoint:thunk
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 00 AA E0 83 00 91").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 F6 03 00 AA 40 28 80 52").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 08 64 43 F9 F3 03 02 AA F6 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 08 AC 43 F9 F3 03 02 AA ?? ?? ?? B4 49 1C 82 52 09 69 69 38").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FC 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA 41 28 80 52").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGetWorldContext — 3/11 نسخة | استراتيجيات: P0-verified-session1, string:GetWorldContext | المصدر: Finders.cpp:169
uint64_t FindGetWorldContext()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E8 3B 00 FD FD 7B 08 A9 FD 03 02 91 FC 6F 09 A9 FA 67 0A A9 F8 5F 0B A9 F6 57 0C A9 F4 4F 0D A9 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? 00 C0 3F 91").Get(); // string:GetWorldContext
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? 08 04 00 51 1F 05 00 71").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGiveAbility — 11/11 نسخة | استراتيجيات: P0-verified-session1, native:K2_GiveAbility:orig, transfer:18.40 | المصدر: Finders.cpp:699
uint64_t FindGiveAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BC A9 FC 5F 01 A9 FD 03 00 91 F6 57 02 A9 F4 4F 03 A9 FF 43 12 D1 ?? ?? ?? ?? ?? ?? ?? ?? 29 91 03 91 F3 03 03 2A").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F6 57 01 A9 FD 03 00 91 F4 4F 02 A9 08 00 40 F9 F4 03 01 AA F3 03 00 AA 08 41 43 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FC 4F 0F A9 FD 7B 10 A9 FD 03 04 91 08 00 40 F9 F3 03 00 AA 09 F9 43 F9 E8 23 00 91 20 01 3F D6 E8 0F 40 F9 ?? ?? ?? B4 08 2D 40 39 1F 05 1B 72 ?? ?? ?? 54 E1 23 00 91 E0 03 13 AA ?? ?? ?? ?? 13 7C 40 92").Get(); // native:K2_GiveAbility:orig
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 0B 40 D1 FF 43 04 D1 E5 9B 09 A9 E7 57 00 F9 E0 87 00 AD").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // transfer:18.40
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 09 F0 42 39 08 58 87 B9 F4 03 01 AA F3 03 00 AA").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 09 F0 42 39 08 F8 86 B9 F4 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 68 A2 45 B9 F4 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 12 44 39 68 1A 87 B9").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 69 F2 43 39 68 0A 85 B9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindGiveAbilityAndActivateOnce — 9/11 نسخة | استراتيجيات: P0-verified-session1, transfer:21.30 | المصدر: Finders.cpp:1478
uint64_t FindGiveAbilityAndActivateOnce()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 28 08 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 03 A9 FD C3 00 91 F5 23 00 F9 F4 4F 05 A9 28 08 40 F9 ?? ?? ?? B4 09 2D 40 39").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 28 08 40 F9 ?? ?? ?? B4 09 2D 40 39").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 A8 83 1D F8 28 08 40 F9 09 3D 43 39").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 A8 83 1D F8 28 08 40 F9 09 3D 43 39").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 A8 83 1D F8 28 08 40 F9 09 3D 43 39").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 56 D0 3B D5 C8 16 40 F9 A8 83 1D F8 28 08 40 F9 09 3D 43 39").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 E8 0F 00 F9 28 08 40 F9 09 3D 43 39").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 01 AA A8 0A 40 F9 F4 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindHandleMatchHasStarted — 11/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2412
uint64_t FindHandleMatchHasStarted()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E8 BB 00 FD FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 ?? ?? ?? ?? F3 03 00 AA 18 FF 40 F9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 ?? ?? ?? ?? F3 03 00 AA 7B 97 40 F9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 E8 9B 00 FD FC 6F 14 A9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 ?? ?? ?? ?? 08 E5 44 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E8 B3 00 FD FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 59 D0 3B D5 28 17 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 E8 6B 00 FD FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 59 D0 3B D5 28 17 40 F9 F3 03 00 AA A8 03 19 F8 ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E8 73 00 FD FC 6F 0F A9 FA 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 59 D0 3B D5 28 17 40 F9 ?? ?? ?? ?? F3 03 00 AA A8 03 19 F8").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 0B 00 F9 08 15 40 F9").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 0B 00 F9 08 15 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 48 D0 3B D5 E8 07 00 F9 08 15 40 F9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 E8 5B 00 FD FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? F3 03 00 AA A8 83 19 F8").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 08 23 7A 39").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindHandlePostSafeZonePhaseChanged — 10/11 نسخة | استراتيجيات: P0-verified-session1, repick-size-matched | المصدر: Finders.cpp:869
uint64_t FindHandlePostSafeZonePhaseChanged()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FD 7B 10 A9 FD 03 04 91 FC 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 F3 03 00 AA B9 83 01 D1").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 F3 03 00 AA FB 83 02 91").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? 08 D9 47 F9 ?? ?? ?? ?? 84 08 3F 91 A3 00 80 52 02 11 00 91 E5 03 00 2A E0 03 1F AA").Get(); // repick-size-matched
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF C3 06 D1 59 D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 08 D1 59 D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 EC AB 00 FD EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 58 D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 EC AB 00 FD EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 59 D0 3B D5 28 17 40 F9 A8 03 17 F8").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EC 7B 00 FD EB 2B 10 6D E9 23 11 6D FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 16 50 41 F9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EC 73 00 FD EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 F3 03 00 AA 79 CA 41 F9 FA 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 E8 1B 00 FD F7 1F 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F3 03 00 AA 74 C6 41 F9 ?? ?? ?? B4 ?? ?? ?? ?? 88 0A 40 F9 09 90 80 B9 0A 91 40 B9 3F 01 0A 6B ?? ?? ?? 54 F5 03 1F AA ?? ?? ?? ??").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitListen — 6/11 نسخة | استراتيجيات: vtable_delta | المصدر: Finders.cpp:312
uint64_t FindInitListen()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F6 57 02 A9 F4 4F 03 A9 08 00 40 F9 F6 03 04 AA F4 03 02 AA").Get(); // vtable_delta
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F5 0B 00 F9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F5 03 04 AA F3 03 02 AA").Get(); // vtable_delta
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get(); // vtable_delta
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get(); // vtable_delta
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA F3 03 02 AA E2 03 01 AA").Get(); // vtable_delta
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 04 AA").Get(); // vtable_delta
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializeBuildingActor — 8/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: Finders.cpp:2444
uint64_t FindInitializeBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EB 2B 11 6D E9 23 12 6D FD 7B 13 A9 FD C3 04 91 FC 6F 14 A9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F7 03 05 2A").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FD 7B 10 A9 FD 03 04 91 FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 F7 03 05 2A").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 ?? ?? ?? ?? 9C 87 46 F9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 5C D0 3B D5 88 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 18 F8").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 5C D0 3B D5 88 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 EB 2B 0F 6D E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 5C D0 3B D5 88 17 40 F9").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EB 2B 0E 6D E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 5B D0 3B D5 68 17 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializeBuildingActor_next — 8/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: 
uint64_t FindInitializeBuildingActor_next()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E8 43 00 FD FD FB 08 A9 FD 23 02 91 F5 4F 00 F9 F4 4F 0A A9 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F3 03 00 AA 3F 1C 00 72 01 18 1F 39").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E8 3B 00 FD FD FB 07 A9 FD E3 01 91 F7 47 00 F9 F6 57 09 A9 F4 4F 0A A9 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F3 03 00 AA 3F 1C 00 72").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 E8 33 00 FD F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F4 03 00 AA 3F 1C 00 72 01 1C 23 39").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E8 3B 00 FD F7 3F 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 A8 83 1C F8 08 10 45 39 1F 0D 00 71 ?? ?? ?? 54 F4 03 00 AA 3F 1C 00 72 01 80 22 39").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 A5 00 00 12 F4 03 04 AA F3 03 00 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 FC 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializeFlightPath — 2/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:2495
uint64_t FindInitializeFlightPath()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 08 A9 FD 03 02 91 F3 4B 00 F9 F3 03 01 AA BF 0F 00 F9 ?? ?? ?? ?? 68 12 40 F9 ?? ?? ?? B4 61 0E 40 F9 A2 63 00 91 E0 03 13 AA ?? ?? ?? ?? E0 23 00 91").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F3 43 00 F9 FD 7B 09 A9 FD 43 02 91 F3 03 01 AA BF 83 1F F8 ?? ?? ?? ?? 68 12 40 F9 ?? ?? ?? B4 61 0E 40 F9 A2 23 00 D1 E0 03 13 AA ?? ?? ?? ?? E0 23 00 91").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInitializePlayerGameplayAbilities — 10/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: Finders.cpp:2900
uint64_t FindInitializePlayerGameplayAbilities()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 03 D1 FD 7B 0A A9 FD 83 02 91 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 ?? ?? ?? ?? F3 03 00 AA 21 50 09 91").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FD 7B 0A A9 FD 83 02 91 F7 5B 00 F9 F6 57 0C A9 F4 4F 0D A9 ?? ?? ?? ?? F3 03 00 AA 21 D8 10 91 A0 63 00 91").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FB 3B 00 F9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 ?? ?? ?? ?? F3 03 00 AA 21 E8 03 91 A0 23 01 D1").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B5").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B5").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FB 43 00 F9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 56 D0 3B D5 C8 16 40 F9 A8 83 1A F8 ?? ?? ?? B5").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 55 D0 3B D5 A8 16 40 F9 A8 83 1D F8 ?? ?? ?? B4 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 ?? ?? ?? B4 08 00 40 F9 F3 03 00 AA 08 0D 40 F9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F3 03 00 AA ?? ?? ?? B4 68 02 40 F9 E0 03 13 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindInternalTryActivateAbility — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:9.41 | المصدر: Finders.cpp:783
uint64_t FindInternalTryActivateAbility()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF 83 0B D1 ?? ?? ?? ?? F9 03 06 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF 03 10 D1 F7 03 05 AA FA 03 04 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 10 D1 FA 03 05 AA F5 03 04 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 5A D0 3B D5 48 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 5A D0 3B D5 48 17 40 F9 F7 03 05 AA F9 03 04 AA").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 83 10 D1 58 D0 3B D5 08 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 0D D1 57 D0 3B D5 E8 16 40 F9").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF 43 0D D1 57 D0 3B D5 E8 16 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F6 03 05 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F6 03 05 AA").Get(); // transfer:9.41
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 08 ?? ?? 91 08 FD DF 08 F7 03 05 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindIsNetReady — 1/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:2261
uint64_t FindIsNetReady()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FD 7B BF A9 FD 03 00 91 ?? ?? ?? 36 08 98 46 B9 E8 03 08 4B 08 F8 01 B9 08 52 86 52 08 68 68 38").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindIsNetRelevantForVft — 9/11 نسخة | استراتيجيات: P0-verified-session1, computed_vtable_index, transfer:18.40, transfer:21.30 | المصدر: Finders.cpp:492
uint64_t FindIsNetRelevantForVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 08 B8 40 79").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 08 B8 40 79").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 09 70 41 39").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 E8 17 00 F9 09 70 41 39").Get(); // transfer:18.40
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindKickPlayer — 10/11 نسخة | استراتيجيات: P0-verified-session1, native:ServerKickPlayer:thunk | المصدر: Finders.cpp:1166
uint64_t FindKickPlayer()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FD 7B 0C A9 FD 03 03 91 FC 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 FD 7B 0D A9 FD 43 03 91 FC 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 FC 6F 0F A9 FA 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? E0 03 14 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA A8 03 1A F8 ?? ?? ?? ?? E0 03 14 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 04 D1 FC 6F 0D A9 FA 67 0E A9 F8 5F 0F A9 F6 57 10 A9 F4 4F 11 A9 FD 7B 12 A9 FD 83 04 91 5A D0 3B D5 48 17 40 F9 F3 03 01 AA F4 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 55 D0 3B D5 A8 16 40 F9 ?? ?? ?? ?? D6 82 0F 91 F4 03 01 AA F3 03 00 AA A8 83 1D F8 FF 7F 01 A9 F6 07 00 F9 FF 7F 02 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4").Get(); // native:ServerKickPlayer:thunk
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 55 D0 3B D5 A8 16 40 F9 ?? ?? ?? ?? D6 42 1C 91 F4 03 01 AA A8 83 1D F8 FF 7F 01 A9 F6 07 00 F9 FF 7F 02 A9 28 10 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 C3 00 91").Get(); // native:ServerKickPlayer:thunk
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 88 02 40 F9 E0 03 14 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindListenCall — 3/11 نسخة | استراتيجيات: string:LoadMap: failed to Listen(%s) | المصدر: Finders.cpp:2924
uint64_t FindListenCall()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 07 D1 EA BB 00 FD E9 23 18 6D FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 FD 7B 1E A9 FD 83 07 91 53 D0 3B D5 68 16 40 F9").Get(); // string:LoadMap: failed to Listen(%s)
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("E9 23 B9 6D FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 07 40 D1 FF 43 22 D1").Get(); // string:LoadMap: failed to Listen(%s)
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("E9 23 B9 6D FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 0B 40 D1 FF C3 02 D1").Get(); // string:LoadMap: failed to Listen(%s)
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindLoadPlayset — 10/11 نسخة | استراتيجيات: P0-verified-session1, transfer:19.10 | المصدر: Finders.cpp:2680
uint64_t FindLoadPlayset()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 05 6D FD 7B 06 A9 FD 83 01 91 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 F3 03 00 AA 14 48 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 05 6D FD 7B 06 A9 FD 83 01 91 FB 3B 00 F9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E9 23 05 6D F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 14 48 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 E9 23 06 6D FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 2F 00 F9 14 48 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 E9 23 03 6D FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 17 00 F9").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 E9 23 03 6D FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 17 00 F9").Get(); // transfer:19.10
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 E9 23 03 6D FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 17 00 F9").Get(); // transfer:19.10
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA E8 1F 00 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 23 00 FD E9 23 05 6D FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 14 64 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 EA 33 00 FD E9 23 07 6D FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F3 03 00 AA 74 76 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindNotifyGameMemberAdded — 5/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2528
uint64_t FindNotifyGameMemberAdded()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 FC 9B 00 F9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 A1 C3 1D 38 E1 03 03 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F4 03 03 AA F6 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F5 03 03 AA F6 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC A3 00 F9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 56 D0 3B D5 C8 16 40 F9 F4 03 03 AA F5 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 57 D0 3B D5 E8 16 40 F9 F5 03 03 AA F6 03 02 2A").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindOnItemInstanceAddedVft — 6/11 نسخة | استراتيجيات: win_abi_parity | المصدر: Finders.cpp:2197
uint32_t FindOnItemInstanceAddedVft()
{
    // فهرس vtable محسوب من ثنائيات أندرويد (مواضع الدالة في runs الفعلي):
    float v = VersionInfo.FortniteVersion;
    if (v >= 23.50f) return 143; // win_abi_parity
    else if (v >= 22.40f) return 143; // win_abi_parity
    else if (v >= 21.30f) return 143; // win_abi_parity
    else if (v >= 19.10f) return 143; // win_abi_parity
    else if (v >= 18.40f) return 143; // win_abi_parity
    else if (v >= 17.50f) return 143; // win_abi_parity
    return 0; // احسب وقت التشغيل: امسح Vft للكلاس عن دالة مطابقة
}

// ----------------------------------------------------------------------
// FindOnRep_ZiplineState — 7/11 نسخة | استراتيجيات: P0-verified-session1, native:OnRep_ZiplineState:orig | المصدر: Finders.cpp:1444
// fallback انعكاسي (للنسخ غير المغطاة): GetFunction("OnRep_ZiplineState")->Func على FortPlayerPawnAthena
uint64_t FindOnRep_ZiplineState()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 FD 7B 06 A9 FD 83 01 91 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 08 CE 84 52 F4 03 01 AA").Get(); // native:OnRep_ZiplineState:orig
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 08 27 84 52 F5 03 00 AA 14 00 08 8B F3 03 01 AA").Get(); // native:OnRep_ZiplineState:orig
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 08 0F 84 52 F3 03 00 AA 00 00 08 8B F4 03 01 AA ?? ?? ?? ??").Get(); // native:OnRep_ZiplineState:orig
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("08 E3 83 52 29 60 40 39 08 68 68 38 3F 01 08 6B ?? ?? ?? 54 C0 03 5F D6 00 E8 5D F9 ?? ?? ?? 34").Get(); // native:OnRep_ZiplineState:orig
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA E8 0F 00 F9 ?? ?? ?? ?? 08 E5 4D B9 ?? ?? ?? 34").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? ?? 08 09 41 B9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 ?? ?? ?? ?? 08 C9 45 B9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPayBuildableClassPlacementCost — 10/11 نسخة | استراتيجيات: waveB | المصدر: Finders.cpp:2587
uint64_t FindPayBuildableClassPlacementCost()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get(); // waveB
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get(); // waveB
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9 ?? ?? ?? B4 ?? ?? ?? ??").Get(); // waveB
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 58 D0 3B D5 08 17 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9").Get(); // waveB
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9 ?? ?? ?? B5 F6 03 1F AA").Get(); // waveB
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F5 03 01 AA F3 03 00 AA E8 0F 00 F9").Get(); // waveB
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 E8 0F 00 F9 ?? ?? ?? 36 08 FC 41 F9 F4 03 01 AA").Get(); // waveB
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA").Get(); // waveB
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get(); // waveB
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F7 0B 00 F9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 02 AA F5 03 01 AA F3 03 00 AA ?? ?? ?? B4").Get(); // waveB
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPickSupplyDropLocation — 10/11 نسخة | استراتيجيات: waveA, waveB, xwin_string | المصدر: Finders.cpp:2856
uint64_t FindPickSupplyDropLocation()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("ED 33 B7 6D EB 2B 01 6D E9 23 02 6D FD 7B 03 A9 FD C3 00 91 FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FF 43 0A D1 F4 03 03 2A").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 ED 33 15 6D EB 2B 16 6D E9 23 17 6D FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 08 5C 42 F9").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 ED 33 15 6D EB 2B 16 6D E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 08 5C 42 F9").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E9 23 13 6D FC A3 00 F9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 56 D0 3B D5 C8 16 40 F9 F3 03 01 AA F4 03 00 AA").Get(); // xwin_string
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5").Get(); // waveB
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5").Get(); // waveB
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 ED 33 04 6D EB 2B 05 6D E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 49 1C A2 4E").Get(); // waveB
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 EB 2B 0C 6D E9 23 0D 6D FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 98 1C 40 F9 ?? ?? ?? B4").Get(); // waveB
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 05 D1 EB 2B 0C 6D E9 23 0D 6D FC 6F 0E A9 FA 67 0F A9 F8 5F 10 A9 F6 57 11 A9 F4 4F 12 A9 FD 7B 13 A9 FD C3 04 91 F4 03 04 AA 98 1E 40 F9").Get(); // waveB
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EB 2B 02 6D E9 23 03 6D FB 23 00 F9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F5 03 05 AA F4 03 04 AA").Get(); // waveB
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPickTeam — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:21.30 | المصدر: Finders.cpp:1076
uint64_t FindPickTeam()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 F4 03 02 AA F5 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 F4 03 02 AA F3 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 16 6C 41 F9 F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F4 03 02 AA F5 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F4 03 02 AA F5 03 00 AA").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F4 03 02 AA F5 03 00 AA").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F4 03 02 AA F5 03 00 AA").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 59 D0 3B D5 28 17 40 F9 F3 03 02 AA F5 03 00 AA F4 03 01 2A E8 0F 00 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 16 50 41 F9 F3 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F5 03 00 AA B6 CA 41 F9").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F9 33 00 F9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 F6 03 00 AA D7 C6 41 F9 F5 03 02 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPostInitializeSpawnedBuildingActor — 11/11 نسخة | استراتيجيات: vtable_next_of_IBA | المصدر: Finders.cpp:2464
uint64_t FindPostInitializeSpawnedBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 BA 40 79 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 72 41 39 69 5A 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F4 03 01 2A F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9").Get(); // vtable_next_of_IBA
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA ?? ?? ?? ?? 68 12 42 39 69 82 40 B9 09 1D 60 B3 ?? ?? ?? B6 60 9A 43 F9").Get(); // vtable_next_of_IBA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindPreSendUpdate — 1/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2384
uint64_t FindPreSendUpdate()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FD 7B 05 A9 FD 43 01 91 FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 ?? ?? ?? ?? F9 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? F5 03 01 AA 00 ?? ?? 91").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindQueueStatEvent — 3/11 نسخة | استراتيجيات: waveB | المصدر: Finders.cpp:2955
uint64_t FindQueueStatEvent()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 07 A9 FD C3 01 91 F4 4F 08 A9 A8 23 00 D1 F3 03 00 AA A7 83 1F B8 E1 A3 00 39").Get(); // waveB
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 A8 13 40 B9 A9 63 00 D1 F3 03 00 AA A7 83 1E B8").Get(); // waveB
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 E5 03 00 F9 48 D0 3B D5").Get(); // waveB
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveFromAlivePlayers — 9/11 نسخة | استراتيجيات: P0-verified-session1, repick-size-matched, string:FortGameModeAthena::RemoveFromAlivePlaye | المصدر: Finders.cpp:1588
uint64_t FindRemoveFromAlivePlayers()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 19 6C 41 F9 ?? ?? ?? B4").Get(); // string:FortGameModeAthena::RemoveFromAlivePlaye
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 FD 7B 02 A9 FD 83 00 91 E5 13 00 B9 E4 0B 00 B9 ?? ?? ?? ?? 4A D9 47 F9 E9 03 00 AA ?? ?? ?? ?? E8 03 03 2A E7 03 02 2A E6 03 01 AA").Get(); // repick-size-matched
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 E8 27 00 F9 E1 1F 00 F9").Get(); // string:FortGameModeAthena::RemoveFromAlivePlaye
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 E8 27 00 F9 E1 23 00 F9").Get(); // string:FortGameModeAthena::RemoveFromAlivePlaye
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 59 D0 3B D5 28 17 40 F9 E8 27 00 F9 E1 23 00 F9").Get(); // string:FortGameModeAthena::RemoveFromAlivePlaye
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 5B D0 3B D5 68 17 40 F9 A8 83 1A F8 19 40 41 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 19 50 41 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 FC 6F 06 A9 FA 67 07 A9 F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91 F8 03 00 AA 19 CB 41 F9").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 F8 03 00 AA 1C C7 41 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveInventoryItem — 11/11 نسخة | استراتيجيات: native:ServerRemoveInventoryItem:thunk, waveA | المصدر: Finders.cpp:1300
uint64_t FindRemoveInventoryItem()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F5 1B 00 F9 F4 4F 04 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 73 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1B 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 91 E0 03 14 AA ?? ?? ?? ?? B5 1B 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 BF 02 00 71 89 12 40 F9 E4 07 9F 1A").Get(); // native:ServerRemoveInventoryItem:thunk
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F5 1B 00 F9 F4 4F 04 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 73 00 91 E0 03 14 AA ?? ?? ?? ?? BF 1B 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 91 E0 03 14 AA ?? ?? ?? ?? B5 1B 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 BF 02 00 71 89 12 40 F9 E4 07 9F 1A").Get(); // native:ServerRemoveInventoryItem:thunk
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FF D7 01 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA F3 03 00 AA FF 0B 00 F9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? BF C3 1E B8 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 53 00 D1 E0 03 14 AA ?? ?? ?? ?? BF 83 1E B8 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 A2 63 00 D1 E0 03 14 AA ?? ?? ?? ?? B5 83 5E B8 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 89 12 40 F9 BF 02 00 71 E4 07 9F 1A").Get(); // native:ServerRemoveInventoryItem:thunk
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 63 00 91 E0 03 14 AA ?? ?? ?? ?? FF 17 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 53 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? F6 13 40 B9 FF 0F 00 B9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? E8 0F 40 B9 89 12 40 F9 DF 02 00 71 E4 07 9F 1A 1F 01 00 71").Get(); // native:ServerRemoveInventoryItem:thunk
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get(); // waveA
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get(); // waveA
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get(); // waveA
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 01 A9").Get(); // waveA
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9").Get(); // waveA
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9").Get(); // waveA
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0F 00 B9 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 0B 00 B9 88 12 40 F9 ?? ?? ?? B4").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindRemoveInventoryStateValue — 11/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:1356
uint64_t FindRemoveInventoryStateValue()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ??").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? FF 33 00 39 ?? ?? ?? ??").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 13 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get(); // waveA
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 13 00 91 E0 03 14 AA ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA 1F 01 00 F1 08 05 88 9A 88 12 00 F9 68 02 40 F9").Get(); // waveA
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B5 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9").Get(); // waveA
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B5 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? FF 13 00 39 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9").Get(); // waveA
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 0F 00 F9 FF FF 00 A9 28 10 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 23 00 91 E0 03 14 AA 48 10 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 13 00 39 88 12 40 F9").Get(); // waveA
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get(); // waveA
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get(); // waveA
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA FF 7F 01 A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? FF 33 00 39 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 33 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 33 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? 88 12 40 F9 E0 03 13 AA").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReplaceBuildingActor — 8/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:1142
uint64_t FindReplaceBuildingActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF ?? ?? D1 ?? ?? ?? ?? ?? ?? ?? ?? 08 ?? ?? 91 F5 03 1F AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FD 7B 01 A9 FD 43 00 91 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FF ?? ?? D1 ?? ?? ?? ?? ?? ?? ?? ?? 29 09 00 91").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 7B 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 68 03 40 39 1F 01 00 71 28 01 9F 9A A8 83 17 F8").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 43 06 D1 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EA 0F 18 FC E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("EA 0F 18 FC E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 83 06 D1 5B D0 3B D5 68 17 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReplicateActor — 8/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:1949
uint64_t FindReplicateActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FD 7B 01 A9 FD 43 00 91 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FF 83 07 D1 F3 03 00 91").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF ?? ?? D1 F3 03 00 91 ?? ?? ?? ?? F5 03 00 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 ?? ?? ?? ?? 18 87 46 F9 ?? ?? ?? ?? 29 ?? ?? 91 F4 03 00 AA 08 03 40 39 F3 03 00 AA 1F 01 00 71").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 7B 8B 47 F9 F4 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 58 D0 3B D5 08 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 39 BF 46 F9 F4 03 00 AA F3 03 00 AA 28 03 40 39").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 A8 03 1A F8 88 33 5D 39 F4 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF ?? ?? D1 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 89 00 91").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("E8 0F 19 FC FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF ?? ?? D1 59 D0 3B D5 28 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 99 04 91").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindReset — 7/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:2513
uint64_t FindReset()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 14 4C 40 A9 ?? ?? ?? B4 61 22 00 91 20 00 80 52").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 F3 03 02 AA F5 03 00 AA F4 03 01 AA 00 00 40 F9").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F5 03 00 AA 00 00 40 F9 F3 03 02 AA F4 03 01 AA").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("09 A8 40 B9 E8 03 01 AA 1F 01 00 F1 E1 03 1F 2A EA 03 1F AA EB 03 82 9A 0C 03 80 52 3F 01 01 6B").Get(); // waveA
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 EF 3B 02 6D ED 33 03 6D EB 2B 04 6D E9 23 05 6D F7 33 00 F9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 F3 03 00 AA 00 54 40 F9").Get(); // waveA
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 60 42 05 91 21 00 00 12 ?? ?? ?? ?? 60 9E 40 F9").Get(); // waveA
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 60 22 05 91 21 00 00 12 ?? ?? ?? ?? 60 9A 40 F9").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSelectAndSetupMyBuildingLevel — 10/11 نسخة | استراتيجيات: waveB | المصدر: Finders.cpp:3228
uint64_t FindSelectAndSetupMyBuildingLevel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F5 1B 00 F9 F4 4F 04 A9 F5 03 01 AA F3 03 00 AA BF 1B 00 B9").Get(); // waveB
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F5 13 00 F9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F5 03 01 AA F3 03 00 AA BF 83 1E B8").Get(); // waveB
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 23 00 B9 ?? ?? ?? ??").Get(); // waveB
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 13 00 F9 ?? ?? ?? ?? ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4").Get(); // waveB
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 03 00 F9").Get(); // waveB
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F5 03 01 AA F3 03 00 AA E8 17 00 F9 FF 03 00 F9").Get(); // waveB
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 00 40 F9 F3 03 00 AA F5 03 01 AA 08 AD 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4").Get(); // waveB
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 00 40 F9 F3 03 00 AA F5 03 01 AA 08 A5 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA").Get(); // waveB
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 00 AA 68 02 40 F9 F5 03 01 AA 08 A5 40 F9 00 01 3F D6 ?? ?? ?? ?? F4 03 00 AA ?? ?? ?? B4 E0 03 14 AA").Get(); // waveB
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 00 AA 88 02 40 F9 F5 03 01 AA").Get(); // waveB
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendClientAdjustment — 1/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:1773
uint64_t FindSendClientAdjustment()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 01 B4 47 B9 F3 03 00 AA 3F 04 00 31 ?? ?? ?? 54").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendDestructionInfo — 7/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:1886
uint64_t FindSendDestructionInfo()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FD 7B 14 A9 FD 03 05 91 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 ?? ?? ?? ?? ?? ?? ?? ?? 08 09 02 91 F3 03 01 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FD 7B 14 A9 FD 03 05 91 FC AB 00 F9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 ?? ?? ?? ?? ?? ?? ?? ?? 08 ?? ?? 91 ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 ?? ?? ?? ?? 7B 87 46 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9 F7 BE 46 F9 ?? ?? ?? ?? 00 ?? ?? 91 F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 29 04 91 E8 0F 00 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9 E8 D2 47 39 ?? ?? ?? ?? 00 ?? ?? 91 F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F7 13 00 F9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91 E8 0F 00 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSendRequestNow — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:21.30 | المصدر: Finders.cpp:615
uint64_t FindSendRequestNow()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 FD 7B 08 A9 FD 03 02 91 F6 57 09 A9 F4 4F 0A A9 ?? ?? ?? ?? 29 31 36 91 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FD 7B 0A A9 FD 83 02 91 F6 57 0B A9 F4 4F 0C A9 ?? ?? ?? ?? F5 03 02 2A F3 03 01 AA F4 03 00 AA C8 C2 65 39").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 03 D1 F8 5F 0B A9 F6 57 0C A9 F4 4F 0D A9 FD 7B 0E A9 FD 83 03 91 ?? ?? ?? ?? C8 C2 58 39").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F9 5B 00 F9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 ?? ?? ?? ?? E8 22 59 39").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 FC 6B 00 F9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 ?? ?? ?? ?? C8 22 5D 39").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F3 03 01 AA 68 02 40 F9 F4 03 00 AA ?? ?? ?? B4 09 01 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindServerReplicateActors — 7/11 نسخة | استراتيجيات: P0-verified-session1, fp2130(hits=4) | المصدر: Finders.cpp:544
uint64_t FindServerReplicateActors()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E9 23 0A 6D FD 7B 0B A9 FD C3 02 91 F6 57 0C A9 F4 4F 0D A9 15 80 06 91 28 60 40 F8").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 E8 23 00 FD FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 14 A0 00 91").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F9 13 00 F9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 ?? ?? ?? ?? 56 D0 3B D5 00 05 C2 3D").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 ?? ?? ?? ?? D6 C2 11 91 ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 59 D0 3B D5 28 17 40 F9 A8 03 1A F8 ?? ?? ?? B4").Get(); // fp2130(hits=4)
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EF 3B B6 6D ED 33 01 6D EB 2B 02 6D E9 23 03 6D FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 FF C3 09 D1 E6 1F 00 F9 E5 2F 00 F9").Get(); // fp2130(hits=4)
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FC 4F BE A9 FD 7B 01 A9 FD 43 00 91 FF 03 33 D1 F3 03 01 AA E0 2F 00 F9 F3 2B 00 F9 E0 27 00 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetChannelActor — 10/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:1812
uint64_t FindSetChannelActor()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 F9 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 16 40 0A 91 F4 03 00 AA").Get(); // imm_fingerprint
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 01 A9 FD 43 00 91 F5 13 00 F9 F4 4F 03 A9 F3 03 00 AA 08 58 41 F9 ?? ?? ?? B4").Get(); // imm_fingerprint
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 ?? ?? ?? ?? 08 01 15 91 0A 41 24 91 40 01 67 9E").Get(); // imm_fingerprint
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1D FC F5 07 00 F9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F3 03 00 AA ?? ?? ?? ?? ?? ?? ?? ?? 08 35 47 F9 F4 03 00 AA").Get(); // imm_fingerprint
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 01 BC 46 B9 F3 03 00 AA 3F 04 00 31 ?? ?? ?? 54 68 C6 46 B9").Get(); // imm_fingerprint
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 ?? ?? ?? ?? 08 81 0B 91 F3 03 00 AA 08 00 00 F9 ?? ?? ?? ?? 60 66 43 F9 ?? ?? ?? B5").Get(); // imm_fingerprint
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 01 AA F4 03 00 AA ?? ?? ?? ?? 15 8C 40 F9 ?? ?? ?? B5 E0 03 1F 2A FD 7B 43 A9").Get(); // imm_fingerprint
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 01 2A F4 03 00 AA").Get(); // imm_fingerprint
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FB 0F 1A F8 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 14 64 41 F9").Get(); // imm_fingerprint
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 02 D1 F7 3B 00 F9 F6 57 08 A9 F4 4F 09 A9 FD 7B 0A A9 FD 83 02 91 57 D0 3B D5 E8 16 40 F9 F4 03 02 AA F3 03 00 AA").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetChannelActorForDestroy — 9/11 نسخة | استراتيجيات: P0-verified-session1, fp2130(hits=5), transfer:21.30 | المصدر: Finders.cpp:1862
uint64_t FindSetChannelActorForDestroy()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BA A9 FC 6F 01 A9 FD 03 00 91 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FF C3 12 D1 1C 20 41 29 AA 37 40 F9").Get(); // fp2130(hits=5)
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 6F BA A9 FA 67 01 A9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 FF C3 10 D1 A6 9F 38 A9 A5 03 18 F8").Get(); // fp2130(hits=5)
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 06 D1 FC A3 00 F9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 FD 7B 19 A9 FD 43 06 91 5A D0 3B D5 48 17 40 F9 A8 83 1A F8 08 C0 40 39").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 5A D0 3B D5 48 17 40 F9 A8 03 1A F8 08 C0 40 39").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 08 C0 40 39 ?? ?? ?? 37").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 F3 03 00 AA 68 C2 40 39").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetGamePhase — 1/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:2570
uint64_t FindSetGamePhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 FD 7B 14 A9 FD 03 05 91 FC 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 F4 03 01 2A F3 03 00 AA").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetInventoryStateValue — 11/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:1400
uint64_t FindSetInventoryStateValue()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 FD 7B 02 A9 FD 83 00 91 F4 4F 03 A9 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F4 03 01 AA F3 03 00 AA FF 7F 01 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ??").Get(); // waveA
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 ?? ?? ?? ?? 88 12 40 F9 ?? ?? ?? B4 81 0E 40 F9 E2 23 00 91 E0 03 14 AA ?? ?? ?? ?? E0 63 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F5 1B 00 F9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 55 D0 3B D5 A8 16 40 F9 F4 03 01 AA F3 03 00 AA E8 17 00 F9 FF FF 00 A9 28 10 40 F9").Get(); // waveA
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 FF 7F 01 A9 28 10 40 F9 F4 03 01 AA F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 E2 43 00 91 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 E1 43 00 91 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 03 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 01 AA BF 7F 3E A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 A2 83 00 D1 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 A1 83 00 D1 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 83 00 91 E1 03 1F 2A").Get(); // waveA
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F4 03 01 AA BF 7F 3E A9 88 12 40 F9 F3 03 00 AA ?? ?? ?? B4 81 0E 40 F9 A2 83 00 D1 E0 03 14 AA ?? ?? ?? ?? ?? ?? ?? ?? 82 42 40 F9 A1 83 00 D1 E0 03 14 AA 48 14 40 F9 88 42 00 F9 ?? ?? ?? ?? E0 E3 00 91 E1 03 1F 2A").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetPickupItems — 10/11 نسخة | استراتيجيات: imm_fingerprint | المصدر: Finders.cpp:1679
uint64_t FindSetPickupItems()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 FD 7B 02 A9 FD 83 00 91 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 F3 03 00 AA F4 03 01 AA 20 18 40 F9 75 CA 41 F9").Get(); // imm_fingerprint
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 01 D1 FD 7B 03 A9 FD C3 00 91 F3 23 00 F9 F3 03 00 AA 08 F0 43 B9 ?? ?? ?? 35 ?? ?? ?? ??").Get(); // imm_fingerprint
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F3 13 00 F9 FD 7B 03 A9 FD C3 00 91 A2 C3 1F 38 08 20 40 F9 E6 03 04 2A E5 03 03 2A").Get(); // imm_fingerprint
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F7 1B 00 F9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 54 D0 3B D5 88 16 40 F9 ?? ?? ?? ?? E8 17 00 F9 68 4A 41 F9 FF FF 01 A9 ?? ?? ?? B4 09 51 80 B9").Get(); // imm_fingerprint
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F7 23 00 F9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 57 D0 3B D5 E8 16 40 F9 00 E4 00 6F E2 03 00 AA").Get(); // imm_fingerprint
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F7 0F 1C F8 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 F3 03 02 AA F7 03 01 AA F4 03 00 AA ?? ?? ?? ?? E9 22 40 A9 F6 03 00 AA F5 03 01 AA 2A 61 40 F8 2B 01 40 F9").Get(); // imm_fingerprint
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 F3 03 00 AA E0 03 01 AA").Get(); // imm_fingerprint
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F6 57 BD A9 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 F4 03 00 AA 88 BE 41 F9 F3 03 01 AA 89 A2 0D 91").Get(); // imm_fingerprint
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 F2 43 B9 ?? ?? ?? 34 60 F6 41 F9 ?? ?? ?? B4 08 00 40 F9 08 11 40 F9 00 01 3F D6 ?? ?? ?? 36 60 F6 41 F9 08 00 40 F9").Get(); // imm_fingerprint
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 82 4F 39 ?? ?? ?? 34 68 F2 43 B9 74 42 0F 91").Get(); // imm_fingerprint
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetPickupTarget — 11/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2876
uint64_t FindSetPickupTarget()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FD 7B 03 A9 FD C3 00 91 FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FF 83 0A D1 08 30 45 39").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FD 7B 02 A9 FD 83 00 91 FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FF 43 0A D1 08 30 45 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 03 0A D1").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("EB 2B B8 6D E9 23 01 6D FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 FF 03 0A D1 5A D0 3B D5 48 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5B D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5A D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("EC 0F 17 FC EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF C3 08 D1 5C D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("ED 33 B7 6D EB 2B 01 6D E9 23 02 6D FC 6F 03 A9 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 FF 03 09 D1 5C D0 3B D5").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 EC 43 00 FD EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 08 60 44 39").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 EC 43 00 FD EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 ED 33 08 6D EB 2B 09 6D E9 23 0A 6D FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 F3 03 00 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetState — 9/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2831
uint64_t FindSetState()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E8 BB 00 FD FD 7B 18 A9 FD 03 06 91 FC 6F 19 A9 FA 67 1A A9 F8 5F 1B A9 F6 57 1C A9 F4 4F 1D A9 08 30 45 39 1F 0D 00 71 ?? ?? ?? 54 F4 4F 5D A9 F6 57 5C A9").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 E8 AB 00 FD FD 7B 16 A9 FD 83 05 91 FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 08 30 45 39").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 E8 AB 00 FD FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 08 30 45 39").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 58 D0 3B D5 08 17 40 F9 A8 03 19 F8 08 10 45 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 57 D0 3B D5 E8 16 40 F9 A8 03 19 F8").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 57 D0 3B D5 E8 16 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E8 7B 00 FD FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 58 D0 3B D5 08 17 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E8 83 00 FD FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 59 D0 3B D5 28 17 40 F9 A8 03 19 F8 08 C0 43 39 1F 0D 00 71").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 E8 43 00 FD F9 47 00 F9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 58 D0 3B D5 08 17 40 F9 A8 83 1B F8 08 60 44 39").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSetWorld — 7/11 نسخة | استراتيجيات: P0-verified-session1, string:AOnlineBeaconHost::InitHost failed, transfer:21.30 | المصدر: Finders.cpp:361
uint64_t FindSetWorld()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 FD 7B 0A A9 FD 83 02 91 F6 57 0B A9 F4 4F 0C A9 ?? ?? ?? ?? F3 03 00 AA 94 42 30 91").Get(); // string:AOnlineBeaconHost::InitHost failed
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 08 A0 40 F9 F4 03 00 AA F3 03 01 AA ?? ?? ?? B5").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 C0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 C0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 14 A0 0C 91 F3 03 00 AA E0 03 14 AA ?? ?? ?? ?? E0 03 14 AA").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindShouldAllowServerSpawnDecoVft — 11/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2780
uint64_t FindShouldAllowServerSpawnDecoVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 9B 00 F9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 48 40 60 1E 29 40 60 1E").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 67 13 A9 F8 5F 14 A9").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 ED 33 0F 6D EB 2B 10 6D E9 23 11 6D FC 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 48 1C A2 4E 29 1C A1 4E").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 ED 33 10 6D EB 2B 11 6D E9 23 12 6D FC 9B 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? ?? ?? ?? B5 E0 03 13 AA ?? ?? ?? ?? ?? ?? ?? 37 F6 03 1F 2A").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA A8 83 1C F8 ?? ?? ?? ?? ?? ?? ?? B4 E0 03 13 AA ?? ?? ?? ?? 08 88 55 39 ?? ?? ?? 36").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA E8 17 00 F9 ?? ?? ?? ?? ?? ?? ?? B4 E0 03 13 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA E8 17 00 F9 ?? ?? ?? ?? ?? ?? ?? B4 F4 03 00 AA ?? ?? ?? ?? 88 0A 40 F9 09 38 80 B9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 03 03 D1 EA 33 00 FD E9 23 07 6D F8 5F 08 A9 F6 57 09 A9 F4 4F 0A A9 FD 7B 0B A9 FD C3 02 91").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 EA 23 00 FD E9 23 05 6D F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F4 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F3 03 00 AA 74 A2 40 F9 ?? ?? ?? B4").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnDecoVft — 10/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:2747
uint64_t FindSpawnDecoVft()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D FD 7B 0C A9 FD 03 03 91 F7 6B 00 F9 F6 57 0E A9 F4 4F 0F A9 F3 03 04 2A").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 ED 33 09 6D EB 2B 0A 6D E9 23 0B 6D F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 ED 33 0A 6D EB 2B 0B 6D E9 23 0C 6D FC 6B 00 F9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 58 D0 3B D5 08 17 40 F9 F4 03 04 2A").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 ED 33 02 6D EB 2B 03 6D E9 23 04 6D F7 2B 00 F9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 E0 07 02 2D E2 1B 00 BD 08 00 40 F9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 ED 33 03 6D EB 2B 04 6D E9 23 05 6D F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 4B 1C A2 4E 2C 1C A1 4E").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F4 03 00 AA E0 07 04 2D E2 2B 00 BD").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnInitialSafeZone — 7/11 نسخة | استراتيجيات: waveA, waveB | المصدر: Finders.cpp:2288
uint64_t FindSpawnInitialSafeZone()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F3 0B 00 F9 FD 03 00 91 28 10 40 F9 F3 03 02 AA 1F 01 00 F1 08 05 88 9A 28 10 00 F9 ?? ?? ?? ?? 08 3A 82 52").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 28 10 40 F9 F3 03 02 AA 1F 01 00 F1 08 05 88 9A 28 10 00 F9 ?? ?? ?? ?? 08 3A 82 52").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 F7 63 00 F9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA F5 43 00 91").Get(); // waveB
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 E9 23 09 6D FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get(); // waveB
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get(); // waveB
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 56 D0 3B D5 C8 16 40 F9 F3 03 00 AA").Get(); // waveB
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 54 D0 3B D5").Get(); // waveB
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindSpawnLoot — 8/11 نسخة | استراتيجيات: waveA | المصدر: Finders.cpp:910
uint64_t FindSpawnLoot()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 EA 7B 00 FD E9 23 10 6D FD 7B 11 A9 FD 43 04 91 FC 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 08 00 40 F9 F4 03 01 AA").Get(); // waveA
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EA 83 00 FD E9 23 11 6D FD 7B 12 A9 FD 83 04 91 FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 F3 03 00 AA 08 40 8B B9").Get(); // waveA
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 08 40 8B B9 F3 03 00 AA").Get(); // waveA
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA").Get(); // waveA
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 03 18 F8 08 F0 89 B9").Get(); // waveA
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9 F3 03 00 AA F4 03 01 AA A8 03 18 F8 08 D8 89 B9").Get(); // waveA
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 EA 2B 00 FD E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 57 D0 3B D5 E8 16 40 F9 E8 27 00 F9 08 B8 89 B9").Get(); // waveA
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 03 D1 EA 2B 00 FD E9 23 06 6D FC 6F 07 A9 FA 67 08 A9 F8 5F 09 A9 F6 57 0A A9 F4 4F 0B A9 FD 7B 0C A9 FD 03 03 91 57 D0 3B D5 E8 16 40 F9 E8 27 00 F9 08 C8 88 B9").Get(); // waveA
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartAircraftPhase — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:17.50, transfer:18.40, transfer:22.40 | المصدر: Finders.cpp:1634
uint64_t FindStartAircraftPhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 F4 03 01 AA F3 03 00 AA 00 00 40 F9 08 00 40 79").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BE A9 F4 4F 01 A9 FD 03 00 91 F3 03 00 AA 00 00 40 F9 08 00 40 79 1F 25 00 71 ?? ?? ?? 54").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 00 00 40 F9 08 00 40 79 1F 25 00 71 ?? ?? ?? 54").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("F4 4F BE A9 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 00 00 40 F9 08 00 40 79 1F 25 00 71 ?? ?? ?? 54").Get(); // transfer:22.40
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 8B 00 FD FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // transfer:17.50
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E8 93 00 FD FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 E8 93 00 FD FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 5C D0 3B D5 88 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 8B 00 FD FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 5B D0 3B D5 68 17 40 F9 ?? ?? ?? ?? ?? ?? ?? ?? 29 ?? ?? 91").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 04 D1 E8 53 00 FD FC 6F 0B A9 FA 67 0C A9 F8 5F 0D A9 F6 57 0E A9 F4 4F 0F A9 FD 7B 10 A9 FD 03 04 91 08 00 40 F9 F3 03 00 AA F9 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 E8 4B 00 FD FC 6F 0A A9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 F3 03 00 AA 68 02 40 F9 F9 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 03 D1 E8 3B 00 FD FC 6F 08 A9 FA 67 09 A9 F8 5F 0A A9 F6 57 0B A9 F4 4F 0C A9 FD 7B 0D A9 FD 43 03 91 F4 03 00 AA 88 E2 6A 39 ?? ?? ?? 34").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartBecomingDormant — 10/11 نسخة | استراتيجيات: vtable_delta | المصدر: Finders.cpp:2047
uint64_t FindStartBecomingDormant()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 06 A9 FD 83 01 91 F5 3B 00 F9 F4 4F 08 A9 08 60 40 79 89 10 80 52 1F 01 09 6A").Get(); // vtable_delta
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F9 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 08 60 40 79 89 10 80 52").Get(); // vtable_delta
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get(); // vtable_delta
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 F3 0B 00 F9 FD 7B 02 A9 FD 83 00 91 53 D0 3B D5 68 16 40 F9 E8 07 00 F9 0D 60 40 79").Get(); // vtable_delta
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get(); // vtable_delta
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 54 D0 3B D5 88 16 40 F9 89 10 80 52").Get(); // vtable_delta
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("F8 5F BC A9 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 08 60 40 79 89 10 80 52 1F 01 09 6A").Get(); // vtable_delta
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 08 40 42 39 09 00 80 92 09 1C 00 F9 09 1A 80 52 08 01 09 0A").Get(); // vtable_delta
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 42 42 39 09 00 80 92 69 1E 00 F9 09 1A 80 52").Get(); // vtable_delta
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("F3 0F 1E F8 FD 7B 01 A9 FD 43 00 91 F3 03 00 AA 68 62 40 79 09 00 80 92 69 C2 03 F8 7F 42 03 F8").Get(); // vtable_delta
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStartStreamingAdditionalPlaylistLevel — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:21.30 | المصدر: Finders.cpp:3312
uint64_t FindStartStreamingAdditionalPlaylistLevel()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F5 03 02 2A F3 03 00 AA A1 83 1F B8").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FD 7B 04 A9 FD 03 01 91 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 F5 03 02 2A F3 03 00 AA A1 83 1F B8").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 F9 1B 00 F9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 F5 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 FA 67 04 A9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 57 D0 3B D5 E8 16 40 F9 F5 03 02 2A").Get(); // transfer:21.30
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 04 D1 FB 53 00 F9 FA 67 0B A9 F8 5F 0C A9 F6 57 0D A9 F4 4F 0E A9 FD 7B 0F A9 FD C3 03 91 58 D0 3B D5 08 17 40 F9 F3 03 00 AA F5 03 02 2A").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F9 23 00 F9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 A1 83 1C F8 08 00 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F9 23 00 F9 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F3 03 00 AA A1 83 1C F8 68 02 40 F9").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 02 D1 F8 5F 05 A9 F6 57 06 A9 F4 4F 07 A9 FD 7B 08 A9 FD 03 02 91 F4 03 00 AA E1 1F 00 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindStreamInMyBuilding — 11/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: Finders.cpp:3267
uint64_t FindStreamInMyBuilding()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 EA 9B 00 FD E9 23 14 6D FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 08 30 45 39 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 EA 9B 00 FD E9 23 14 6D FD 7B 15 A9 FD 43 05 91 FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 08 30 45 39 F5 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 EA A3 00 FD E9 23 15 6D FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 08 30 45 39 F5 03 01 2A").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 EA AB 00 FD E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 58 D0 3B D5 08 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 83 05 D1 EA 73 00 FD E9 23 0F 6D FC 6F 10 A9 FA 67 11 A9 F8 5F 12 A9 F6 57 13 A9 F4 4F 14 A9 FD 7B 15 A9 FD 43 05 91 58 D0 3B D5 08 17 40 F9 F5 03 01 2A F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FC 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 57 D0 3B D5 E8 16 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 05 D1 E9 23 0F 6D FC 67 10 A9 F8 5F 11 A9 F6 57 12 A9 F4 4F 13 A9 FD 7B 14 A9 FD 03 05 91 57 D0 3B D5 E8 16 40 F9").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 EA 53 00 FD E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 58 D0 3B D5 08 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FC 6F 13 A9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 58 D0 3B D5 08 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 EA 8B 00 FD E9 A3 11 6D FC 97 00 F9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 58 D0 3B D5 08 17 40 F9").Get(); // P0-verified-session1
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 93 00 FD E9 A3 12 6D FC 9F 00 F9 FA 67 14 A9 F8 5F 15 A9 F6 57 16 A9 F4 4F 17 A9 FD 7B 18 A9 FD 03 06 91 57 D0 3B D5 E8 16 40 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindTickFlush — 8/11 نسخة | استراتيجيات: P0-verified-session1 | المصدر: Finders.cpp:426
uint64_t FindTickFlush()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 E9 23 03 6D FD 7B 04 A9 FD 03 01 91 FC 6F 05 A9 FA 67 06 A9 F8 5F 07 A9 F6 57 08 A9 F4 4F 09 A9 18 ?? ?? 91").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 07 D1 EA BB 00 FD E9 23 18 6D FD 7B 19 A9 FD 43 06 91 FC 6F 1A A9 FA 67 1B A9 F8 5F 1C A9 F6 57 1D A9 F4 4F 1E A9 F3 03 00 AA ?? ?? ?? ??").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 06 D1 E8 A3 00 FD FC 6F 15 A9 FA 67 16 A9 F8 5F 17 A9 F6 57 18 A9 F4 4F 19 A9 FD 7B 1A A9 FD 83 06 91 1B 80 48 39").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 5B D0 3B D5 68 17 40 F9 F3 03 00 AA").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 48 D0 3B D5 E8 07 00 F9 08 15 40 F9 F3 03 00 AA 08 1C A0 4E").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 07 D1 E9 23 17 6D FC 6F 18 A9 FA 67 19 A9 F8 5F 1A A9 F6 57 1B A9 F4 4F 1C A9 FD 7B 1D A9 FD 43 07 91 48 D0 3B D5 E8 0F 00 F9 08 15 40 F9").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 07 D1 EA AB 00 FD E9 23 16 6D FC 6F 17 A9 FA 67 18 A9 F8 5F 19 A9 F6 57 1A A9 F4 4F 1B A9 FD 7B 1C A9 FD 03 07 91 48 D0 3B D5 E8 0B 00 F9").Get(); // P0-verified-session1
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 07 D1 EA A3 00 FD E9 23 15 6D FC 6F 16 A9 FA 67 17 A9 F8 5F 18 A9 F6 57 19 A9 F4 4F 1A A9 FD 7B 1B A9 FD C3 06 91 48 D0 3B D5 E8 1B 00 F9").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindUnEquipVehicleWeapon — 9/11 نسخة | استراتيجيات: P0-verified-session1, transfer:18.40 | المصدر: Finders.cpp:3354
uint64_t FindUnEquipVehicleWeapon()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FB 0B 00 F9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 00 AA 00 4C 40 F9").Get(); // P0-verified-session1
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F5 03 00 AA F6 03 03 2A").Get(); // P0-verified-session1
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 59 D0 3B D5 28 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 59 D0 3B D5 28 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // transfer:18.40
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F7 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // P0-verified-session1
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F7 03 00 AA 00 68 40 F9 F5 03 03 2A").Get(); // P0-verified-session1
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// FindUpdateSafeZonesPhase — 1/11 نسخة | استراتيجيات: xwin_string | المصدر: Finders.cpp:2324
uint64_t FindUpdateSafeZonesPhase()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E8 9B 00 FD FC 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 55 D0 3B D5").Get(); // xwin_string
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ============================================================================
// PART 2 — الدوال الـ17 خارج Finders.cpp (inline resolutions)
// ============================================================================

// ----------------------------------------------------------------------
// Inline_AttemptDeriveFromURL — 1/11 نسخة | استراتيجيات: win-fp | المصدر: Erbium/Private/Misc.cpp:530
uint64_t Inline_AttemptDeriveFromURL()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? 34 48 00 40 39 63 04 00 71 28 00 00 39 48 04 40 B9 28 04 00 B9 48 08 40 B9 28 08 00 B9").Get(); // win-fp
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// Inline_CanGrappleToComponent: منطق vtable-ScanFor كامل (FF 90 → BLR على ARM) — قابل للنقل حرفياً للـ runtime
uint64_t Inline_CanGrappleToComponent() { return 0; }

// Inline_ClientOnPawnDiedCaller: غير محلولة — راجع VERIFICATION-REPORT-v3.md
uint64_t Inline_ClientOnPawnDiedCaller() { return 0; }

// ----------------------------------------------------------------------
// Inline_CollectGarbagePatch — 10/11 نسخة | استراتيجيات: string:CollectGarbageInternal() is  | المصدر: FortniteGame/Private/FortGameMode.cpp:838
uint64_t Inline_CollectGarbagePatch()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF 43 06 D1 EA 8B 00 FD E9 23 12 6D FD 7B 13 A9 FD C3 04 91 FC 6F 14 A9 FA 67 15 A9 F8 5F 16 A9 F6 57 17 A9 F4 4F 18 A9 ?? ?? ?? ?? F3 03 00 2A").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 E1 17 00 B9").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 06 D1 E9 23 11 6D FC 6F 12 A9 FA 67 13 A9 F8 5F 14 A9 F6 57 15 A9 F4 4F 16 A9 FD 7B 17 A9 FD C3 05 91 48 D0 3B D5 E8 0F 00 F9 08 15 40 F9").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 05 D1 E9 23 10 6D FC 6F 11 A9 FA 67 12 A9 F8 5F 13 A9 F6 57 14 A9 F4 4F 15 A9 FD 7B 16 A9 FD 83 05 91 55 D0 3B D5").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? A8 03 19 F8 ?? ?? ?? ??").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? A8 03 19 F8 ?? ?? ?? ??").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 83 04 D1 E9 23 0B 6D FC 6F 0C A9 FA 67 0D A9 F8 5F 0E A9 F6 57 0F A9 F4 4F 10 A9 FD 7B 11 A9 FD 43 04 91 57 D0 3B D5 E8 16 40 F9 ?? ?? ?? ?? A8 03 19 F8 ?? ?? ?? ?? 08 51 53 39 ?? ?? ?? 34").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 02 D1 E9 23 03 6D FC 6F 04 A9 FA 67 05 A9 F8 5F 06 A9 F6 57 07 A9 F4 4F 08 A9 FD 7B 09 A9 FD 43 02 91 ?? ?? ?? ?? A8 46 44 F9 F3 03 01 2A").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("E9 23 B9 6D FC 6F 01 A9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 FF 07 40 D1 FF 03 06 D1").Get(); // string:CollectGarbageInternal() is 
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("E8 0F 1B FC FC 07 00 F9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 07 40 D1").Get(); // string:CollectGarbageInternal() is 
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// Inline_EquipVehicleWeaponFunc — 1/11 نسخة | استراتيجيات: native:EquipVehicleWeapon:thunk | المصدر: FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp:150
// fallback انعكاسي (للنسخ غير المغطاة): GetFunction("EquipVehicleWeapon")->Func على FortVehicleSeatWeaponComponent
uint64_t Inline_EquipVehicleWeaponFunc()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 F5 23 00 F9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 BF 83 1E F8 28 10 40 F9 F3 03 01 AA F4 03 00 AA ?? ?? ?? B4").Get(); // native:EquipVehicleWeapon:thunk
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// Inline_GetSparseClassData — 1/11 نسخة | استراتيجيات: tiny-getter(imm=0x0,callers=1078) | المصدر: FortniteGame/Private/BuildingSMActor.cpp:591
uint64_t Inline_GetSparseClassData()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("08 00 40 39 ?? ?? ?? 37 00 00 40 F9 ?? ?? ?? B5 C0 03 5F D6 ?? ?? ?? ??").Get(); // tiny-getter(imm=0x0,callers=1078)
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// Inline_IrisBool: global عبر نمط cmp [rip] — يحتاج فك GOT وقت التشغيل (نفس منهجية GIs)
uint64_t Inline_IrisBool() { return 0; }

// Inline_K2_OnUnEquipCaller: غير محلولة — راجع VERIFICATION-REPORT-v3.md
uint64_t Inline_K2_OnUnEquipCaller() { return 0; }

// Inline_OnAuthorityRandomUpgradeAppliedCaller: غير محلولة — راجع VERIFICATION-REPORT-v3.md
uint64_t Inline_OnAuthorityRandomUpgradeAppliedCaller() { return 0; }

// Inline_OnDamageServerCaller: غير محلولة — راجع VERIFICATION-REPORT-v3.md
uint64_t Inline_OnDamageServerCaller() { return 0; }

// ----------------------------------------------------------------------
// Inline_OnPawnEnterSeatFunc — 6/11 نسخة | استراتيجيات: registrar:OnPawnEnterSeat:thunk->impl | المصدر: FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp:134
uint64_t Inline_OnPawnEnterSeatFunc()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF C3 08 D1 28 58 40 A9").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BB A9 FC 0B 00 F9 FD 03 00 91 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FF 43 08 D1 28 54 40 A9").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FC 0F 1B F8 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF C3 0C D1 28 58 40 A9").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 0D D1 59 D0 3B D5").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 03 0D D1 59 D0 3B D5").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FC 67 BB A9 F8 5F 01 A9 F6 57 02 A9 F4 4F 03 A9 FD 7B 04 A9 FD 03 01 91 FF 43 0A D1 59 D0 3B D5 28 17 40 F9 F3 03 01 AA").Get(); // registrar:OnPawnEnterSeat:thunk->impl
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// Inline_PostUpdate: غير محلولة — راجع VERIFICATION-REPORT-v3.md
uint64_t Inline_PostUpdate() { return 0; }

// ----------------------------------------------------------------------
// Inline_ServerOnAttemptInteract — 11/11 نسخة | استراتيجيات: native:ServerOnAttemptInteract:orig | المصدر: FortniteGame/Private/FortLootPackage.cpp:696
// fallback انعكاسي (للنسخ غير المغطاة): GetFunction("ServerOnAttemptInteract")->Func على BuildingContainer
uint64_t Inline_ServerOnAttemptInteract()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? 00 E4 00 6F 08 5D 41 F9 1F 00 00 79 1F 40 02 F8 00 40 80 3C 00 40 81 3C 00 01 C0 3D").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("?? ?? ?? ?? 00 E4 00 6F 08 0D 47 F9 1F 00 00 79 1F 40 02 F8 00 40 80 3C 00 40 81 3C 00 01 C0 3D").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("1F 00 00 79 1F 40 02 F8 ?? ?? ?? ?? 08 21 46 F9 00 E4 00 6F 00 40 80 3C 00 40 81 3C 09 09 40 F9").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("1F 00 00 79 1F 40 02 F8 ?? ?? ?? ?? 08 21 47 F9 00 E4 00 6F 00 40 80 3C 00 40 81 3C 09 09 40 F9").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("1F 00 00 79 ?? ?? ?? ?? 29 75 40 F9 E0 E5 00 6F 08 00 80 12 00 40 80 3C 00 40 81 3C 08 40 02 F8").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("E0 E5 00 6F 08 00 80 12 ?? ?? ?? ?? 1F 00 00 79 29 41 25 91 00 40 80 3C 00 40 81 3C 08 40 02 F8").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("E0 E5 00 6F 08 00 80 12 ?? ?? ?? ?? 1F 00 00 79 29 C1 0F 91 00 40 80 3C 00 40 81 3C 08 40 02 F8").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("E0 E5 00 6F 08 00 80 12 ?? ?? ?? ?? 1F 00 00 79 29 81 2E 91 00 40 80 3C 00 40 81 3C 08 40 02 F8").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("1F 00 00 39 ?? ?? ?? ?? 29 1D 44 F9 E0 E5 00 6F 08 00 80 12 00 40 80 3C 00 40 81 3C 08 40 02 F8").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("E0 E5 00 6F 08 00 80 12 1F 00 00 39 00 40 80 3C 00 40 81 3C 08 40 02 F8 ?? ?? ?? ?? 08 75 40 F9").Get(); // native:ServerOnAttemptInteract:orig
        else if (v >= 5.41f)
            Result = Memcury::Scanner::FindPattern("E0 E5 00 6F 08 00 80 12 1F 00 00 39 00 40 80 3C 00 40 81 3C 08 40 02 F8 ?? ?? ?? ?? 08 71 43 F9").Get(); // native:ServerOnAttemptInteract:orig
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// Inline_SetIsInsideSafeZoneBase: patch-site مع virtual call — يحتاج بصمة BLR+vtable وقت التشغيل
uint64_t Inline_SetIsInsideSafeZoneBase() { return 0; }

// Inline_SetOwningInventory: vtable-slot finder — يحتاج مسح vtable وقت التشغيل (منطق قابل للنقل كما هو)
uint64_t Inline_SetOwningInventory() { return 0; }

// ----------------------------------------------------------------------
// Inline_ToDeathCauseNative — 10/11 نسخة | استراتيجيات: native:ToDeathCause:orig | المصدر: FortniteGame/Private/FortPlayerControllerAthena.cpp:1059
// fallback انعكاسي (للنسخ غير المغطاة): GetFunction("ToDeathCause")->Func
uint64_t Inline_ToDeathCauseNative()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 FD 7B 01 A9 FD 43 00 91 F4 4F 02 A9 48 06 80 52 F4 03 01 2A F3 03 00 AA A8 C3 1F 38").Get(); // native:ToDeathCause:orig
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FD 7B BD A9 F5 0B 00 F9 FD 03 00 91 F4 4F 02 A9 48 06 80 52 F4 03 01 2A F3 03 00 AA A8 73 00 39").Get(); // native:ToDeathCause:orig
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("F5 0F 1D F8 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 48 06 80 52 F4 03 01 2A F3 03 00 AA E8 33 00 39").Get(); // native:ToDeathCause:orig
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 56 D0 3B D5 C8 16 40 F9 49 06 80 52").Get(); // native:ToDeathCause:orig
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 56 D0 3B D5 C8 16 40 F9 49 06 80 52").Get(); // native:ToDeathCause:orig
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 56 D0 3B D5 C8 16 40 F9 49 06 80 52").Get(); // native:ToDeathCause:orig
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 56 D0 3B D5 C8 16 40 F9 49 06 80 52").Get(); // native:ToDeathCause:orig
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF 03 01 D1 F6 57 01 A9 F4 4F 02 A9 FD 7B 03 A9 FD C3 00 91 56 D0 3B D5 C8 16 40 F9 49 06 80 52").Get(); // native:ToDeathCause:orig
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 E8 07 1C 32 F4 03 01 2A F3 03 00 AA E8 33 00 39").Get(); // native:ToDeathCause:orig
        else if (v >= 7.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 00 D1 F4 4F 01 A9 FD 7B 02 A9 FD 83 00 91 A8 05 80 52 F4 03 01 2A F3 03 00 AA E8 33 00 39").Get(); // native:ToDeathCause:orig
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ----------------------------------------------------------------------
// Inline_UnequipVehicleWeaponFunc — 9/11 نسخة | استراتيجيات: reuse:FindUnEquipVehicleWeapon | المصدر: FortniteGame/Private/FortVehicleSeatWeaponComponent.cpp:156
uint64_t Inline_UnequipVehicleWeaponFunc()
{
    static uint64_t Result = 0;
    static bool bInit = false;
    if (!bInit)
    {
        bInit = true;
        float v = VersionInfo.FortniteVersion;
        if (v >= 30.00f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 23.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FD 7B 01 A9 FD 43 00 91 FB 13 00 F9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 F6 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 22.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FB 0B 00 F9 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 F5 03 00 AA 00 4C 40 F9").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 21.30f)
            Result = Memcury::Scanner::FindPattern("FF 03 02 D1 FC 6F 02 A9 FA 67 03 A9 F8 5F 04 A9 F6 57 05 A9 F4 4F 06 A9 FD 7B 07 A9 FD C3 01 91 5B D0 3B D5 68 17 40 F9 F5 03 00 AA F6 03 03 2A").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 19.10f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 18.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 59 D0 3B D5 28 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 17.50f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 59 D0 3B D5 28 17 40 F9 F6 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 13.40f)
            Result = Memcury::Scanner::FindPattern("FF C3 01 D1 FA 67 02 A9 F8 5F 03 A9 F6 57 04 A9 F4 4F 05 A9 FD 7B 06 A9 FD 83 01 91 5A D0 3B D5 48 17 40 F9 F7 03 00 AA F5 03 03 2A F4 03 02 AA").Get(); // reuse:FindUnEquipVehicleWeapon
        else if (v >= 9.41f)
            Result = Memcury::Scanner::FindPattern("FF 83 01 D1 F9 0B 00 F9 F8 5F 02 A9 F6 57 03 A9 F4 4F 04 A9 FD 7B 05 A9 FD 43 01 91 F7 03 00 AA 00 68 40 F9 F5 03 03 2A").Get(); // reuse:FindUnEquipVehicleWeapon
        // النسخ غير المذكورة أعلاه: غير محلولة بنمط فريد — راجع التقرير
    }
    return Result;
}

// ============================================================================
// PART 3 — منفذ FindFunctionCall لنظام أندرويد (آلية وقت التشغيل)
// يعادل FindNameRef/FindFunctionCall في Memcury.h لكن بتعليمات ARM64:
//   1. ابحث عن نص اسم الـ UFunction (UTF-16/ASCII)
//   2. الدالة المحتوية = static-init المسجِّل؛ ابحث فيها عن ADRP+ADD نحو global
//   3. من يحمّل ذلك الـ global = الدالة المستدعية (hook target)
// ملاحظة: على أندرويد يمكن الاختصار — التسجيل {char* name, void* thunk} موجود
// في .data.rel.ro مع relocations؛ البحث عن زوج المؤشرات يعطي الـ thunk مباشرة.
// ============================================================================
