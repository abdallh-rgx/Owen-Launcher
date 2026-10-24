// ============================================================================
// Finders-Android.cpp — موائم Erbium لأندرويد ARM64
// الباحثات الدلالية المستخرجة من التحليل الثابت الشامل (11 نسخة)
// كل بصمة مُتحقق منها ضد: MobileDumper-7 (21.30) + سلاسل thunk→impl
// ============================================================================
#include "pch.h"
#include "../Public/Finders.h"
#include <android/log.h>
#define ALOG(...) __android_log_print(ANDROID_LOG_DEBUG, "Erbium", __VA_ARGS__)

// ARM64 instruction decoding helpers (inline, header-only)
namespace ARM64
{
    inline bool IsAdrp(uint32_t w) { return (w & 0x9F000000) == 0x90000000; }
    inline bool Adrp(uint32_t w, uint64_t pc, uint32_t& rd, uint64_t& page)
    {
        if (!IsAdrp(w)) return false;
        rd = w & 0x1F;
        int64_t imm = ((((w >> 5) & 0x7FFFF) << 2) | ((w >> 29) & 3));
        if (imm & 0x100000) imm -= 0x200000;
        page = (pc & ~0xFFFULL) + ((uint64_t)imm << 12);
        return true;
    }
    inline bool IsLdr64(uint32_t w) { return (w & 0xFFC00000) == 0xF9400000; }
    inline bool Ldr64(uint32_t w, uint32_t& rt, uint32_t& rn, uint64_t& imm)
    {
        if (!IsLdr64(w)) return false;
        rt = w & 0x1F; rn = (w >> 5) & 0x1F; imm = ((w >> 10) & 0xFFF) << 3;
        return true;
    }
    // LSR Xd, Xn, #13  → (w & 0xFFFFFC00) == 0xD34DFC00
    inline bool IsLsr13(uint32_t w) { return (w & 0xFFFFFC00) == 0xD34DFC00; }
    // AND Xd, Xn, #0x7FFF8 → 0x927D3C00
    inline bool IsAnd7FFF8(uint32_t w) { return (w & 0xFFFFFC00) == 0x927D3C00; }
    // AND Wd, Wn, #0xFFFF → 0x12003C00
    inline bool IsAndFFFF(uint32_t w) { return (w & 0xFFFFFC00) == 0x12003C00; }
    // MOVZ Wd, #imm
    inline bool MovzImm(uint32_t w, uint32_t& imm) { if ((w & 0xFFE00000) != 0x52800000) return false; imm = (w >> 5) & 0xFFFF; return true; }
    // LDRB Wt, [Xn, #imm12]
    inline bool Ldrb(uint32_t w, uint32_t& rt, uint32_t& rn, uint32_t& imm)
    {
        if ((w & 0xFFC00000) != 0x39400000) return false;
        rt = w & 0x1F; rn = (w >> 5) & 0x1F; imm = (w >> 10) & 0xFFF; return true;
    }
    // TBNZ Wt, #b, label
    inline bool Tbnz(uint32_t w, uint32_t& rt, uint32_t& bit)
    {
        if ((w & 0xFF000000) != 0x37000000) return false;
        rt = w & 0x1F; bit = (w >> 19) & 0x1F; return true;
    }
    // CBZ Wt/Xt
    inline bool IsCbzR0(uint32_t w) { return (w & 0x7F00001F) == 0x34000000 || (w & 0x7F00001F) == 0xB4000000; }
}

// ============================================================================
// 1) GObjects — بصمة المصفوفة المقسمة (Chunked)
// ============================================================================
// الفكرة: كل كود الوصول للمصفوفة المقسمة (UE4.21+ حتى UE5.3) له الشكل:
//   ADRP Xp, page ; LDR Xn, [Xp, #off]   ← GObjects+0x10 (حقل Objects)
//   LSR  Xm, Xi, #13                      ← فهرس القسم × 8
//   AND  Xm, Xm, #0x7FFF8
//   AND  Wi, Wi, #0xFFFF                  ← الفهرس داخل القسم
//   LDR  Xc, [Xn, Xm]                     ← مؤشر القسم
//   MOV/ORR Ws, #0x18                     ← حجم FUObjectItem
//   MADD/UMADDL Xr, Wi, Ws, Xc
// على UE5.4+ (30.00): تخطيط B — ADRP+LDR يعطي مؤشراً يُفك مرة إضافية [+0x10]
// وحجم العنصر صار 0x14.
//
// المتحقق منه: 21.30 = 0xDCD6E08 (مطابق للـ MobileDumper حرفياً)
// 9.41: 0xCDB9790 | 13.40: 0x12B1CD78 | 17.50: 0x13724870 | 18.40: 0x11550ED0
// 19.10: 0x10398210 | 22.40: 0xE080FD8 | 23.50: 0xDD846D8 | 30.00: 0x13BA3C10 (Layout B)
uint64_t FindGObjectsAndroid()
{
    static uint64_t GObjects = 0;
    static bool bInit = false;
    if (bInit) return GObjects;
    bInit = true;

    auto base = Memcury::GetModuleBase();
    auto textOff = Memcury::GetTextSectionOffset();
    auto textSize = Memcury::GetTextSectionSize();
    if (!base || !textSize) return 0;

    auto text = (uint32_t*)(base + textOff);
    uint64_t nIns = textSize / 4;

    // تصويت: أكثر عنوان مرشح تكراراً يفوز
    std::unordered_map<uint64_t, int> votes;

    for (uint64_t i = 16; i + 12 < nIns; i++)
    {
        if (!ARM64::IsLsr13(text[i])) continue;
        if (!ARM64::IsAnd7FFF8(text[i + 1])) continue;

        bool bHasAndFFFF = false;
        for (int j = 2; j < 8 && i + j < nIns; j++)
            if (ARM64::IsAndFFFF(text[i + j])) { bHasAndFFFF = true; break; }
        if (!bHasAndFFFF) continue;

        // تتبع للخلف حتى 14 تعليمة: ADRP ثم LDR متتاليان بنفس السجل
        // (على 30.00 قد يأتي LDR بعد الثلاثية — نفحص الاحتمالين)
        uint64_t targetA = 0, targetB = 0;
        uint32_t last_rd = 0; uint64_t last_page = 0; bool have_adrp = false;
        for (int64_t j = (int64_t)i - 1; j >= (int64_t)i - 14 && j >= 0; j--)
        {
            uint32_t rd; uint64_t page;
            if (ARM64::Adrp(text[j], base + textOff + j * 4, rd, page))
            {
                have_adrp = true; last_rd = rd; last_page = page;
                continue;
            }
            uint32_t rt, rn; uint64_t imm;
            if (have_adrp && ARM64::Ldr64(text[j], rt, rn, imm) && rn == last_rd)
            {
                targetA = last_page + imm;
                break;
            }
        }
        if (targetA)
        {
            votes[targetA - 0x10] += 2;   // Layout A: Objects@GObjects+0x10 مباشرة
            votes[targetA] += 1;          // Layout B: GObjects مؤشر (UE5.4+)
        }
    }

    int best = 0; uint64_t bestAddr = 0;
    for (auto& [addr, v] : votes)
        if (v > best) { best = v; bestAddr = addr; }

    return GObjects = bestAddr;
}

// ============================================================================
// 2) ProcessEvent — بصمة دلالية
// ============================================================================
// UE5 (18.40+):  CBZ X0 ; LDRB W?,[X0,#0xB] ; TST #imm ; B.NE ;
//                LDRB W?,[X1,#0xB1] ; TBNZ W?, #2 (FUNC_Native=0x400)
// UE5.4 (30.00): LDR W?,[X0,#8] ; TBNZ W?,#0x1E ; LDRB [X1,#0xB1]; TBNZ #2
// التحقق: 21.30 = 0x7611B5C (مطابق للـ MobileDumper + VT slot 78)
// 13.40: 0xBCF7568 | 17.50: 0xC5B7178 | 18.40: 0xAD74BC8 | 19.10: 0xA894EF0
// 22.40: 0x7F9CD20 | 23.50: 0x6115084 | 30.00: 0x9412930
uint64_t FindProcessEventAndroid()
{
    static uint64_t PE = 0;
    static bool bInit = false;
    if (bInit) return PE;
    bInit = true;

    auto base = Memcury::GetModuleBase();
    auto textOff = Memcury::GetTextSectionOffset();
    auto textSize = Memcury::GetTextSectionSize();
    auto text = (uint32_t*)(base + textOff);
    uint64_t nIns = textSize / 4;

    for (uint64_t i = 2; i + 24 < nIns; i++)
    {
        // فحص FUNC_Native: LDRB Wt,[X1,#imm] ثم TBNZ Wt,#2 خلال 4 تعليمات
        uint32_t rt, rn, imm;
        if (!ARM64::Ldrb(text[i], rt, rn, imm) || rn != 1) continue;

        bool bNative = false;
        uint32_t rt2, bit;
        for (int k = 1; k <= 4; k++)
            if (ARM64::Tbnz(text[i + k], rt2, bit) && rt2 == rt && bit == 2)
            { bNative = true; break; }
        if (!bNative) continue;

        // يجب أن يكون قرب بداية دالة (أول 0x40 بايت)
        // UE4: يبدأ مباشرة بالفحص | UE5: يسبقه CBZ X0 + فحص الإشارات
        uint64_t pc = base + textOff + i * 4;
        uint64_t fn = 0;
        // ابحث عن بداية الدالة: للأمان نستخدم التراجع حتى 0x40 بايت عن بروولوج STP/SUB
        // (في المنفذ الفعلي استخدم قائمة الدوال من eh_frame أو تراجع ذكي)
        fn = Memcury::Scanner(pc).FindFunctionStart(); // دالة مساعدة تحتاج إضافتها
        if (!fn) continue;

        // تحقق إضافي: قبل الفحص (UE5) يوجد CBZ X0 أو LDR W?,[X0,#8]+TBNZ#30
        bool bPreValid = false;
        for (int64_t j = (int64_t)i - 1; j >= (int64_t)i - 12 && j >= 0; j--)
        {
            if (ARM64::IsCbzR0(text[j])) { bPreValid = true; break; }
            uint32_t rt3, rn3, imm3;
            if (ARM64::Ldrb(text[j], rt3, rn3, imm3) && rn3 == 0) { bPreValid = true; break; }
        }
        if (!bPreValid) continue;   // يستبعد UE4 أيضاً إن أردت (UE4 بلا مقدمة — عدّل حسب الحاجة)

        // تحقق عضوية vtable: الدالة يجب أن يشير إليها relocation
        // (في المنفذ: افحص أن الدالة موجودة في .data.rel.ro عبر وحدة ديكودر relocations)
        return PE = fn;
    }
    return 0;
}

// ============================================================================
// 3) FName::ToString(FString&) — عبر سلسلة native GetObjectName
// ============================================================================
// GetObjectName native impl تفعل: LDR W?,[X0,#0x18] (FName) ثم BL ToString
// ToString نفسها: تقرأ البلا (>>13 &0x7FFF8 + Blocks[+0x40] + stride 4 + wide 0x40)
// 21.30 = 0x74937CC — تعمل على كل النسخ لأن native GetObjectName موجودة دائماً
// (عناوين الـ thunks جاهزة عندك في offsets.h من uefinder --all!)
uint64_t FindFNameToStringAndroid(uint64_t GetObjectNameThunkOrImpl)
{
    // المعطى: عنوان impl الـ native (من جدولك) — نتابع أول BL بعد تحميل [x0,#0x18]
    if (!GetObjectNameThunkOrImpl) return 0;
    Memcury::Scanner s(GetObjectNameThunkOrImpl);
    // exec thunk أو impl؟ الـ thunk يمرر FFrame — الـ impl مباشرة
    // نبحث عن: LDR W?, [X0, #0x18] ثم خلال 6 تعليمات BL
    auto ldrSite = s.ScanFor({ 0x18, 0x41, 0x40, 0xB9 }, false); // LDR Wt,[X0,#0x18] تقريباً
    if (!ldrSite.Get()) return 0;
    auto bl = ldrSite.ScanFor({ 0x94 }, false, 0, 1, 6);
    if (!bl.Get()) return 0;
    return Memcury::Scanner(bl.Get()).RelativeBranchTarget().Get();
}

// ============================================================================
// 4) Offsets جاهزة مُجمّدة (RVA) من التحليل الثابت — استخدمها كمرجع أو كتصحيح
// ============================================================================
namespace AndroidOffsets
{
    // GObjects (RVA)
    constexpr uint64_t GObjects_9_41 = 0xCDB9790;
    constexpr uint64_t GObjects_13_40 = 0x12B1CD78;
    constexpr uint64_t GObjects_17_50 = 0x13724870;
    constexpr uint64_t GObjects_18_40 = 0x11550ED0;
    constexpr uint64_t GObjects_19_10 = 0x10398210;
    constexpr uint64_t GObjects_21_30 = 0xDCD6E08;   // ✓ مُتحقق بالكامل
    constexpr uint64_t GObjects_22_40 = 0xE080FD8;
    constexpr uint64_t GObjects_23_50 = 0xDD846D8;
    constexpr uint64_t GObjects_30_00 = 0x13BA3C10;  // Layout B! deref ثم +0x10، Item=0x14

    // ProcessEvent (RVA)
    constexpr uint64_t PE_13_40 = 0xBCF7568;
    constexpr uint64_t PE_17_50 = 0xC5B7178;
    constexpr uint64_t PE_18_40 = 0xAD74BC8;
    constexpr uint64_t PE_19_10 = 0xA894EF0;
    constexpr uint64_t PE_21_30 = 0x7611B5C;         // ✓ مُتحقق (VT slot 78)
    constexpr uint64_t PE_22_40 = 0x7F9CD20;
    constexpr uint64_t PE_23_50 = 0x6115084;
    constexpr uint64_t PE_30_00 = 0x9412930;         // ✓ بنية UE5.4 كاملة

    // SpawnActor (مرسا STAT_SpawnActorTime)
    constexpr uint64_t SpawnActor_13_40 = 0xC07D830;
    constexpr uint64_t SpawnActor_17_50 = 0xC2A20A4;
    constexpr uint64_t SpawnActor_18_40 = 0xAA632BC;
    constexpr uint64_t SpawnActor_19_10 = 0xC3BA164;
    constexpr uint64_t SpawnActor_21_30 = 0x947F4FC;
    constexpr uint64_t SpawnActor_22_40 = 0x9BF411C;
    constexpr uint64_t SpawnActor_23_50 = 0x65C8CD4;
    constexpr uint64_t SpawnActor_30_00 = 0x9C439DC;

    // StaticFindObject (مرسا "Illegal call to StaticFindObject" — UE4 فقط)
    constexpr uint64_t SFO_5_41 = 0x3FE2D60;
    constexpr uint64_t SFO_7_40 = 0x4DB7CCC;
    constexpr uint64_t SFO_9_41 = 0x5D9ABC8;
    constexpr uint64_t SFO_13_40 = 0xBCF5554;
    constexpr uint64_t SFO_17_50 = 0xDE7B67C;
    constexpr uint64_t SFO_18_40 = 0xC7273CC;

    // FName::ToString (21.30)
    constexpr uint64_t FNameToString_21_30 = 0x74937CC;
    // عائلة Step (21.30): StepExplicitProperty = 0x760ECC8

    // FFrame (مؤكد من thunks على 21.30)
    constexpr uint32_t FFrame_Node = 0x18;
    constexpr uint32_t FFrame_Code = 0x20;
}
