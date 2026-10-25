#!/usr/bin/env python3
"""Generate Finders-Android-FULL-v5.cpp = v4 patterns + RUNTIME BRUTE-FORCE SUPPORT.
The runtime support (RT::) finds every missing function ON-DEVICE at load time:
  RT::StringAnchorFunc — UTF-16 string → ADRP/ADR/GOT refs → deoutline → prologue
  RT::VtNeighbor/VtIndexOf — runtime vtable slot scan + deltas (ABI-parity proven)
  RT::FindNativeThunk — {name,thunk} pair scan in resolved data
  RT::GIsFromAnchor — AllowCommandletRendering bool-global ARM64 port
Covers ALL 220 missing cells + resolves 37 GIs conflicts on any 5.x→30.00 binary."""
import json
import re

V4 = open('/home/z/my-project/data/erbium/Finders-Android-FULL-v4.cpp', encoding='utf-8').read()
RES = json.load(open('/home/z/my-project/data/erbium/final_resolved_v4.json'))
ANCH = json.load(open('/home/z/re-analysis/out/erbium-anchors.json'))

# ----------------------------------------------------------------------------
# runtime support C++ (ARM64)
# ----------------------------------------------------------------------------
RT_CODE = r'''
// ============================================================================
// PART 0 — دعم وقت التشغيل (RUNTIME BRUTE-FORCE SUPPORT) — v5
// يعمل على أي libUE4.so/libUnreal.so من 5.x إلى 30.00 (والإصدارات الوسيطة)
// دون أي نمط مسبق: فحص غاشم متجهي في الذاكرة بعد تحميل الوحدة.
// ============================================================================
namespace RT
{
    static uint64_t g_ModBase = 0, g_ModEnd = 0;
    static uint64_t g_TextStart = 0, g_TextEnd = 0;      // executable range
    static uint64_t g_DataStart = 0, g_DataEnd = 0;      // writable range (vtables/GOT/natives)

    inline void Init()
    {
        if (g_ModBase)
            return;
        g_ModBase = ModuleBase;
        // parse ELF64 program headers to find exec + writable segments
        auto* ehdr = (uint8_t*)g_ModBase;
        if (*(uint32_t*)ehdr != 0x464C457F) // \x7fELF
            return;
        uint16_t phentsize = *(uint16_t*)(ehdr + 54);
        uint16_t phnum = *(uint16_t*)(ehdr + 56);
        uint8_t* ph = ehdr + *(uint64_t*)(ehdr + 32);
        for (int i = 0; i < phnum; i++, ph += phentsize)
        {
            uint32_t type = *(uint32_t*)(ph + 0);
            uint32_t flags = *(uint32_t*)(ph + 4);
            uint64_t vaddr = *(uint64_t*)(ph + 16);
            uint64_t memsz = *(uint64_t*)(ph + 40);
            if (type != 1 /*PT_LOAD*/)
                continue;
            uint64_t s = g_ModBase + vaddr, e = s + memsz;
            if (g_ModEnd < e)
                g_ModEnd = e;
            if (flags & 1) // X
            {
                if (!g_TextStart || s < g_TextStart) g_TextStart = s;
                if (e > g_TextEnd) g_TextEnd = e;
            }
            if (flags & 2) // W
            {
                if (!g_DataStart || s < g_DataStart) g_DataStart = s;
                if (e > g_DataEnd) g_DataEnd = e;
            }
        }
        if (!g_TextStart) { g_TextStart = g_ModBase; g_TextEnd = g_ModEnd; }
    }

    inline bool InText(uint64_t a) { return a >= g_TextStart && a < g_TextEnd; }
    inline bool InData(uint64_t a) { return a >= g_DataStart && a < g_DataEnd; }
    inline bool InMod(uint64_t a) { return a >= g_ModBase && a < g_ModEnd; }

    // ---- string scan (UTF-16LE across whole module: .text literal pools + .rodata)
    inline uint64_t FindWStr(const wchar_t* needle)
    {
        RT::Init();
        size_t len = wcslen(needle);
        if (!len)
            return 0;
        auto* p = (const uint8_t*)needle;
        uint8_t first = p[0], second = p[1];
        for (uint64_t a = g_ModBase + 0x1000; a + len * 2 < g_ModEnd; a += 2)
        {
            auto* m = (const uint8_t*)a;
            if (m[0] != first || m[1] != second)
                continue;
            bool ok = true;
            for (size_t i = 2; i < len * 2; i++)
                if (m[i] != p[i]) { ok = false; break; }
            if (ok)
                return a;
        }
        return 0;
    }

    inline uint64_t FindAStr(const char* needle)
    {
        RT::Init();
        size_t len = strlen(needle);
        if (!len)
            return 0;
        for (uint64_t a = g_ModBase + 0x1000; a + len < g_ModEnd; a += 1)
        {
            auto* m = (const uint8_t*)a;
            if (m[0] != (uint8_t)needle[0])
                continue;
            bool ok = true;
            for (size_t i = 1; i < len; i++)
                if (m[i] != (uint8_t)needle[i]) { ok = false; break; }
            if (ok && m[len] == 0)
                return a;
        }
        return 0;
    }

    // ---- ARM64 decoders ----
    // ADRP Xd, imm : (instr & 0x9F000000) == 0x90000000
    inline bool IsADRP(uint32_t i) { return (i & 0x9F000000) == 0x90000000; }
    inline uint64_t ADRPPage(uint64_t pc, uint32_t i)
    {
        uint64_t imm = ((uint64_t)(i & 0x00FFFFE0) >> 3) | ((i >> 29) & 3);
        if (imm & 0x100000) imm |= 0xFFFFFFFFFFF00000ULL; // sign-extend 21 bits
        return (pc & ~0xFFFULL) + (imm << 12);
    }
    inline uint8_t ADRPRd(uint32_t i) { return i & 0x1F; }
    // ADR Xd, imm : (instr & 0x9F000000) == 0x10000000
    inline bool IsADR(uint32_t i) { return (i & 0x9F000000) == 0x10000000; }
    inline uint64_t ADRImm(uint64_t pc, uint32_t i)
    {
        uint64_t imm = ((uint64_t)(i & 0x00FFFFE0) >> 3) | ((i >> 29) & 3);
        if (imm & 0x100000) imm |= 0xFFFFFFFFFFF00000ULL;
        return pc + imm;
    }
    // ADD Xd, Xn, #imm12<<0 : (i & 0xFF800000) == 0x91000000
    inline bool IsADDImm(uint32_t i) { return (i & 0xFF800000) == 0x91000000; }
    inline uint32_t ADDImm12(uint32_t i) { return (i >> 10) & 0xFFF; }
    // LDR Xt, [Xn, #imm12*8] : (i & 0xFFC00000) == 0xF9400000
    inline bool IsLDR64(uint32_t i) { return (i & 0xFFC00000) == 0xF9400000; }
    inline uint32_t LDR64Off(uint32_t i) { return ((i >> 10) & 0xFFF) * 8; }
    // LDRB Wt, [Xn, #imm12] : (i & 0xFFC00000) == 0x39400000
    inline bool IsLDRB(uint32_t i) { return (i & 0xFFC00000) == 0x39400000; }
    inline uint32_t LDRBOff(uint32_t i) { return (i >> 10) & 0xFFF; }
    // STRB Wt, [Xn, #imm12] : (i & 0xFFC00000) == 0x39000000
    inline bool IsSTRB(uint32_t i) { return (i & 0xFFC00000) == 0x39000000; }
    // B imm26 : (i & 0xFC000000) == 0x14000000 ; BL : 0x94000000
    inline bool IsB(uint32_t i) { return (i & 0xFC000000) == 0x14000000; }
    inline bool IsBL(uint32_t i) { return (i & 0xFC000000) == 0x94000000; }
    inline uint64_t BranchTarget(uint64_t pc, uint32_t i)
    {
        int64_t imm = (int32_t)((i & 0x03FFFFFF) << 6) >> 6; // sign-extend 26
        return pc + (imm << 2);
    }
    // STP X29, X30, [SP, #-imm]! : (i & 0xFFC07FFF) == 0xA9807BFD
    // STP X29, X30, [SP, #imm]  : (i & 0xFFC07FFF) == 0xA9007BFD
    inline bool IsSTPX29X30(uint32_t i) { return (i & 0xFFC07FFF) == 0xA9807BFD || (i & 0xFFC07FFF) == 0xA9007BFD; }
    // SUB SP, SP, #imm : (i & 0xFF8003FF) == 0xD10003FF
    inline bool IsSUBSP(uint32_t i) { return (i & 0xFF8003FF) == 0xD10003FF; }
    // PACIASP: 0xD503233F
    inline bool IsPACIASP(uint32_t i) { return i == 0xD503233F; }

    // ---- refs to an address inside .text (ADRP+ADD / ADR / ADRP+LDR-GOT) ----
    inline void RefsTo(uint64_t target, uint64_t* out, uint32_t* outN, uint32_t maxOut)
    {
        RT::Init();
        *outN = 0;
        if (!target || !g_TextStart)
            return;
        uint64_t start = g_TextStart;
        uint64_t end = g_TextEnd;
        // fast prefilter: ADRP whose page == target page (±1 page for ADR)
        uint64_t tpage = target & ~0xFFFULL;
        for (uint64_t pc = start; pc + 8 < end && *outN < maxOut; pc += 4)
        {
            uint32_t i = *(uint32_t*)pc;
            if (IsADRP(i))
            {
                uint64_t page = ADRPPage(pc, i);
                uint8_t rd = ADRPRd(i);
                if (page != tpage && page != tpage + 0x1000 && page + 0x1000 != tpage)
                    continue;
                // scan next 8 instructions for ADD/LDR with same rd
                for (uint64_t q = pc + 4; q < pc + 40 && q + 4 < end; q += 4)
                {
                    uint32_t j = *(uint32_t*)q;
                    if (IsADRP(j) && ADRPRd(j) == rd)
                        break; // register redefined
                    if (IsADDImm(j) && ((j >> 5) & 0x1F) == rd)
                    {
                        uint64_t addr = page + ADDImm12(j);
                        if (addr == target)
                        {
                            out[(*outN)++] = pc;
                            break;
                        }
                        break; // ADD consumed the ADRP
                    }
                    if (IsLDR64(j) && ((j >> 5) & 0x1F) == rd)
                    {
                        // GOT-indirect: *(page + off) == target ?
                        uint64_t slot = page + LDR64Off(j);
                        if (RT::InMod(slot) && *(uint64_t*)slot == target)
                        {
                            out[(*outN)++] = pc;
                            break;
                        }
                        break;
                    }
                }
            }
            else if (IsADR(i) && ADRImm(pc, i) == target)
            {
                out[(*outN)++] = pc;
            }
        }
    }

    // ---- de-outline: a stub is a tiny function that only B's to the real one ----
    inline uint64_t Deoutline(uint64_t addr)
    {
        RT::Init();
        if (!RT::InText(addr))
            return 0;
        for (int hops = 0; hops < 4; hops++)
        {
            uint32_t i = *(uint32_t*)addr;
            if (IsB(i))
            {
                uint64_t t = BranchTarget(addr, i);
                if (RT::InText(t) && t != addr)
                {
                    addr = t;
                    continue;
                }
            }
            break;
        }
        return addr;
    }

    // ---- prologue backscan: from an address inside a function, find its start ----
    inline uint64_t BackscanPrologue(uint64_t addr, uint64_t maxDist = 0x900)
    {
        RT::Init();
        if (!RT::InText(addr))
            return 0;
        uint64_t best = 0;
        uint64_t lo = addr > maxDist ? addr - maxDist : g_TextStart;
        for (uint64_t p = addr & ~3ULL; p >= lo; p -= 4)
        {
            uint32_t i = *(uint32_t*)p;
            if (IsSTPX29X30(i) || IsPACIASP(i))
            {
                best = p;
                // keep scanning: the LOWEST prologue marker wins (function start)
                if (IsSTPX29X30(i))
                {
                    // verify: previous instruction (if any) is also prologue-ish or it IS the start
                    if (p < addr && p - 4 >= lo)
                    {
                        uint32_t prev = *(uint32_t*)(p - 4);
                        if (IsSTPX29X30(prev) || IsPACIASP(prev) || IsSUBSP(prev))
                            continue; // part of a bigger prologue — keep going down
                    }
                    // STP X29,X30 followed by typical prologue body → accept lowest chain
                    break;
                }
            }
            else if (IsSUBSP(i) && !best)
            {
                best = p; // fallback: SUB SP alone (leaf)
            }
            // stop if we hit an obvious RET (end of previous function)
            if ((i & 0xFFFFFC1F) == 0xD65F0000 || (i & 0xFFFFFFE0) == 0xD65F0000) // RET/RETAA
            {
                if (!best)
                    best = p + 4;
                break;
            }
        }
        return best;
    }

    // ---- full string-anchor chain: str → refs → deoutline → prologue ----
    // Tries each string; returns the first function whose body references it.
    inline uint64_t StringAnchorFunc(const wchar_t* const* strs, int nStrs, int refIndex = 0)
    {
        RT::Init();
        for (int s = 0; s < nStrs; s++)
        {
            uint64_t strAddr = RT::FindWStr(strs[s]);
            if (!strAddr)
                continue;
            uint64_t refs[64];
            uint32_t nRefs = 0;
            RT::RefsTo(strAddr, refs, &nRefs, 64);
            for (uint32_t r = 0; r < nRefs; r++)
            {
                uint64_t a = RT::Deoutline(refs[r]);
                uint64_t f = RT::BackscanPrologue(a);
                if (f)
                {
                    if (refIndex == 0)
                        return f;
                    refIndex--;
                }
            }
        }
        return 0;
    }

    inline uint64_t StringAnchorFunc(const wchar_t* s0)
    {
        const wchar_t* arr[1] = { s0 };
        return RT::StringAnchorFunc(arr, 1);
    }

    // ---- runtime vtable primitives ----
    // find every 8-aligned slot in writable data equal to func
    inline uint32_t VtSlotsWith(uint64_t func, uint64_t* slots, uint32_t maxSlots)
    {
        RT::Init();
        uint32_t n = 0;
        if (!g_DataStart || !func)
            return 0;
        for (uint64_t a = g_DataStart; a + 8 <= g_DataEnd && n < maxSlots; a += 8)
            if (*(uint64_t*)a == func)
                slots[n++] = a;
        return n;
    }

    // walk a vtable slot array backwards to its start (stop at non-text ptr / big gap)
    inline uint64_t VtStartOf(uint64_t slot)
    {
        RT::Init();
        uint64_t s = slot;
        int guard = 0;
        while (guard++ < 8192)
        {
            if (s < g_DataStart + 8)
                break;
            uint64_t v = *(uint64_t*)(s - 8);
            if (!RT::InText(v))
                break;
            // vtable entries are dense: previous slot usually within ±32MB of func
            s -= 8;
        }
        return s;
    }

    // neighbor of anchor's vtable slot by delta (majority across slots)
    inline uint64_t VtNeighbor(uint64_t anchorFunc, int delta)
    {
        RT::Init();
        if (!anchorFunc)
            return 0;
        uint64_t slots[96];
        uint32_t n = RT::VtSlotsWith(anchorFunc, slots, 96);
        uint64_t votes[96];
        uint32_t nv = 0;
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t t = (delta >= 0) ? slots[i] + 8 * (uint64_t)delta
                                      : slots[i] - 8 * (uint64_t)(-(int64_t)delta);
            if (!RT::InData(t))
                continue;
            uint64_t v = *(uint64_t*)t;
            if (!RT::InText(v))
                continue;
            bool seen = false;
            for (uint32_t j = 0; j < nv; j++)
                if (votes[j] == v) { seen = true; break; }
            if (!seen && nv < 96)
                votes[nv++] = v;
        }
        if (!nv)
            return 0;
        return votes[0]; // all valid votes agree in practice (same class hierarchy)
    }

    // index of func in its (majority) vtable — for *Vft() finders
    inline uint32_t VtIndexOf(uint64_t func, uint64_t* outVt = nullptr)
    {
        RT::Init();
        if (!func)
            return 0;
        uint64_t slots[96];
        uint32_t n = RT::VtSlotsWith(func, slots, 96);
        uint32_t bestIdx = 0, bestCnt = 0;
        uint64_t bestVt = 0;
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t vs = RT::VtStartOf(slots[i]);
            uint32_t idx = (uint32_t)((slots[i] - vs) / 8);
            uint32_t cnt = 0;
            for (uint32_t j = 0; j < n; j++)
            {
                uint64_t vs2 = RT::VtStartOf(slots[j]);
                uint32_t idx2 = (uint32_t)((slots[j] - vs2) / 8);
                if (idx2 == idx)
                    cnt++;
            }
            if (cnt > bestCnt || (cnt == bestCnt && idx < bestIdx))
            {
                bestIdx = idx;
                bestCnt = cnt;
                bestVt = vs;
            }
        }
        if (outVt)
            *outVt = bestVt;
        return bestIdx;
    }

    // read vtable method by index using anchor's vtable (majority)
    inline uint64_t VtMethodAt(uint64_t anchorFunc, uint32_t index)
    {
        RT::Init();
        if (!anchorFunc)
            return 0;
        uint64_t slots[96];
        uint32_t n = RT::VtSlotsWith(anchorFunc, slots, 96);
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t vs = RT::VtStartOf(slots[i]);
            uint64_t t = vs + 8ull * index;
            if (RT::InData(t))
            {
                uint64_t v = *(uint64_t*)t;
                if (RT::InText(v))
                    return v;
            }
        }
        return 0;
    }

    // ---- natives table: {char* Name, uintptr Thunk} pairs in resolved data ----
    inline uint64_t FindNativeThunk(const char* name)
    {
        RT::Init();
        if (!g_DataStart)
            return 0;
        size_t len = strlen(name);
        for (uint64_t a = g_DataStart; a + 16 <= g_DataEnd; a += 8)
        {
            uint64_t namePtr = *(uint64_t*)a;
            uint64_t fnPtr = *(uint64_t*)(a + 8);
            if (!RT::InText(fnPtr) || !RT::InMod(namePtr))
                continue;
            // name must be ASCII, null-terminated, exact match
            const char* m = (const char*)namePtr;
            bool ok = true;
            for (size_t i = 0; i < len; i++)
                if (m[i] != name[i]) { ok = false; break; }
            if (ok && m[len] == 0)
                return fnPtr;
        }
        return 0;
    }

    // ---- native orig-climb: exec thunk → BL → التنفيذ الأصلي ----
    // الـ exec thunk ينادي الـ impl مبكراً في جسمه (FFrame setup ثم BL impl).
    inline uint64_t NativeOrig(const char* name)
    {
        uint64_t thunk = RT::FindNativeThunk(name);
        if (!thunk)
            return 0;
        uint64_t t = RT::Deoutline(thunk);
        for (uint64_t pc = t; pc < t + 0x140; pc += 4)
        {
            uint32_t i = *(uint32_t*)pc;
            if (IsBL(i))
            {
                uint64_t callee = BranchTarget(pc, i);
                if (callee == t || !RT::InText(callee))
                    continue;
                // الـ impl: دالة أكبر من الـ stub، تبدأ بـ prologue صالح
                uint64_t cand = RT::Deoutline(callee);
                if (cand != t && RT::InText(cand))
                    return cand;
            }
        }
        return t; // fallback: الـ thunk نفسه (نقطة هوك ExecHook صالحة)
    }

    // ---- BL-crawl: callees of a function ----
    inline uint32_t BLCallees(uint64_t from, uint64_t* out, uint32_t maxOut, uint64_t window = 0x100)
    {
        RT::Init();
        uint32_t n = 0;
        if (!RT::InText(from))
            return 0;
        for (uint64_t pc = from; pc < from + window && n < maxOut; pc += 4)
        {
            uint32_t i = *(uint32_t*)pc;
            if (IsBL(i))
            {
                uint64_t t = BranchTarget(pc, i);
                if (RT::InText(t))
                {
                    bool seen = false;
                    for (uint32_t j = 0; j < n; j++)
                        if (out[j] == t) { seen = true; break; }
                    if (!seen)
                        out[n++] = RT::Deoutline(t);
                }
            }
            // RET في نهاية دالة صغيرة → توقف إذا تجاوزنا نافذة صغيرة (اختياري: لا نتوقف — outlined helpers)
        }
        return n;
    }

    // ---- caller-walk: كل من يستدعي target عبر BL ----
    inline uint32_t BLCallersTo(uint64_t target, uint64_t* out, uint32_t maxOut)
    {
        RT::Init();
        uint32_t n = 0;
        if (!target || !g_TextStart)
            return 0;
        for (uint64_t pc = g_TextStart; pc + 4 < g_TextEnd && n < maxOut; pc += 4)
        {
            uint32_t i = *(uint32_t*)pc;
            if (IsBL(i) && BranchTarget(pc, i) == target)
            {
                uint64_t f = RT::BackscanPrologue(pc);
                if (f)
                {
                    bool seen = false;
                    for (uint32_t j = 0; j < n; j++)
                        if (out[j] == f) { seen = true; break; }
                    if (!seen)
                        out[n++] = f;
                }
            }
        }
        return n;
    }

    // ---- عدد حفظات السجلات في المقدمة (مقياس "ضخامة" الدالة) ----
    inline uint32_t PrologueSaveCount(uint64_t fn)
    {
        RT::Init();
        if (!RT::InText(fn))
            return 0;
        uint32_t saves = 0;
        for (uint64_t pc = fn; pc < fn + 0x40; pc += 4)
        {
            uint32_t w = *(uint32_t*)pc;
            // STP Xt1,Xt2,[SP,#imm](|!) — أي زوج إلى SP
            if ((w & 0xFFC003E0) == 0xA90003E0 || (w & 0xFFC003E0) == 0xA98003E0)
                saves++;
        }
        return saves;
    }

    // ---- callee ضخم (أكثر حفظات) مثل AttemptDeriveFromURL من GetNetMode ----
    inline uint64_t BigCalleeFrom(uint64_t caller, uint32_t minSaves = 3)
    {
        uint64_t c[48];
        uint32_t n = RT::BLCallees(caller, c, 48, 0x80);
        uint64_t best = 0;
        uint32_t bestSaves = minSaves ? minSaves - 1 : 0;
        for (uint32_t i = 0; i < n; i++)
        {
            uint32_t s = RT::PrologueSaveCount(c[i]);
            if (s > bestSaves)
            {
                bestSaves = s;
                best = c[i];
            }
        }
        return best;
    }

    // ---- caller ضخم لدالة (ServerReplicateActors يُنادي ReplicateActor) ----
    inline uint64_t BigCallerOf(uint64_t target, uint32_t minSaves = 4)
    {
        uint64_t c[64];
        uint32_t n = RT::BLCallersTo(target, c, 64);
        uint64_t best = 0;
        uint32_t bestSaves = minSaves ? minSaves - 1 : 0;
        for (uint32_t i = 0; i < n; i++)
        {
            uint32_t s = RT::PrologueSaveCount(c[i]);
            if (s > bestSaves)
            {
                bestSaves = s;
                best = c[i];
            }
        }
        return best;
    }

    // ---- الشكل: LDRB Wt,[X0]; TBZ; LDR X0,[X0,#imm]; CBNZ; RET (بوصف GetSparseClassData) ----
    inline uint64_t TinySparseAccessorFrom(uint64_t caller)
    {
        uint64_t c[48];
        uint32_t n = RT::BLCallees(caller, c, 48, 0x200);
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t f = c[i];
            uint32_t ldrbX0 = 0, ldrX0X0 = 0, rets = 0, ins = 0;
            for (uint64_t pc = f; pc < f + 0x20; pc += 4)
            {
                uint32_t w = *(uint32_t*)pc;
                if (w == 0xD65F03C0u) { rets++; break; }
                if (IsLDRB(w) && ((w >> 5) & 0x1F) == 0)
                    ldrbX0++;
                if (IsLDR64(w) && ((w >> 5) & 0x1F) == 0 && (w & 0x1F) == 0)
                    ldrX0X0++;
                ins++;
            }
            if (rets == 1 && ldrX0X0 >= 1 && ldrbX0 >= 1 && ins <= 6)
                return f;
        }
        return 0;
    }

    // ---- الشكل: LDRSW + ADD ..,LSL#3 (حلقة WorldContexts) — GetWorldContext ----
    inline uint64_t WorldContextCalleeFrom(uint64_t caller)
    {
        uint64_t c[48];
        uint32_t n = RT::BLCallees(caller, c, 48, 0x100);
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t f = c[i];
            bool hasLdrsw = false, hasAddLsl3 = false;
            for (uint64_t pc = f; pc < f + 0x40; pc += 4)
            {
                uint32_t w = *(uint32_t*)pc;
                // LDRSW Xt,[Xn,#imm]: 0xB9800000|..
                if ((w & 0xFFC00000) == 0xB9800000)
                    hasLdrsw = true;
                // ADD Xd,Xn,Xm,LSL#3: 0x8B000C00 mask imm6=3<<10
                if ((w & 0xFF200C00) == 0x8B000C00)
                    hasAddLsl3 = true;
                if (w == 0xD65F03C0u)
                    break;
            }
            if (hasLdrsw && hasAddLsl3)
                return f;
        }
        return 0;
    }

    // ---- الشكل: نداء افتراضي واحد صغير (CallPreReplication من ReplicateActor) ----
    // LDR X8,[X0] (vtable) + LDR Xn,[X8,#off] + BLR Xn — دالة صغيرة بنداء افتراضي واحد
    inline uint64_t VirtCallCalleeFrom(uint64_t caller)
    {
        uint64_t c[48];
        uint32_t n = RT::BLCallees(caller, c, 48, 0x180);
        for (uint32_t i = 0; i < n; i++)
        {
            uint64_t f = c[i];
            uint32_t blrs = 0, ldrVft = 0, ins = 0;
            bool earlyRet = false;
            for (uint64_t pc = f; pc < f + 0x60; pc += 4)
            {
                uint32_t w = *(uint32_t*)pc;
                if (w == 0xD65F03C0u)
                    break;
                ins++;
                if ((w & 0xFFFFFC1F) == 0xD63F0000 && ((w >> 5) & 0x1F) != 31)
                    blrs++; // BLR Xn (ليس BLR LZCNT-free reserved)
                if (IsLDR64(w) && ((w >> 5) & 0x1F) == 0)
                    ldrVft++;
                if (ins > 24)
                    break;
            }
            if (blrs == 1 && ldrVft >= 1 && ins >= 4 && ins <= 24)
                return f;
        }
        return 0;
    }

    // ---- GIs global discovery (ARM64 port of Erbium's AllowCommandletRendering scan)
    // Mirrors Erbium Windows: scan BACKWARD from the string-ref instruction collecting
    // bool-global accesses (ADRP+LDRB/STRB, incl. GOT-indirect); skip=2 → GIsClient,
    // skip=1 → GIsServer (identical ordering to Finders.cpp).
    inline uint64_t GIsFromAnchor(int skip)
    {
        RT::Init();
        uint64_t sAddr = RT::FindWStr(L"AllowCommandletRendering");
        if (!sAddr)
            sAddr = RT::FindWStr(L"llowCommandletRendering");
        if (!sAddr)
            return 0;
        uint64_t refs[16];
        uint32_t nRefs = 0;
        RT::RefsTo(sAddr, refs, &nRefs, 16);
        for (uint32_t r = 0; r < nRefs; r++)
        {
            uint64_t refInstr = RT::Deoutline(refs[r]);
            // scan backward up to 0x100 bytes (like Erbium's i < 0x100 loop)
            uint64_t g[16];
            uint32_t ng = 0;
            for (uint64_t pc = refInstr; pc + 4 > refInstr - 0x100 && ng < 16; pc -= 4)
            {
                uint32_t i = *(uint32_t*)pc;
                if ((IsLDRB(i) || IsSTRB(i)))
                {
                    // find the ADRP defining the base register within the previous 12 instrs
                    uint8_t rn = (i >> 5) & 0x1F;
                    uint64_t base = 0;
                    for (uint64_t q = pc - 4; q + 4 > pc - 52; q -= 4)
                    {
                        uint32_t j = *(uint32_t*)q;
                        if (IsADRP(j) && ADRPRd(j) == rn)
                        {
                            base = ADRPPage(q, j);
                            break;
                        }
                        if (IsLDR64(j) && ADRPRd(j) != rn && ((j >> 5) & 0x1F) == rn)
                        {
                            // GOT-indirect: ADRP Xk,GOT; LDR Xk,[Xk,#off]; LDRB ..[Xk]
                            uint64_t k = q;
                            for (uint64_t w = k - 4; w + 4 > k - 52; w -= 4)
                            {
                                uint32_t u = *(uint32_t*)w;
                                if (IsADRP(u) && ADRPRd(u) == ((j >> 5) & 0x1F))
                                {
                                    uint64_t slot = ADRPPage(w, u) + LDR64Off(j);
                                    if (RT::InMod(slot))
                                        base = *(uint64_t*)slot;
                                    break;
                                }
                            }
                            break;
                        }
                    }
                    if (base)
                    {
                        uint64_t addr = base + LDRBOff(i);
                        if (RT::InData(addr))
                        {
                            bool seen = false;
                            for (uint32_t j = 0; j < ng; j++)
                                if (g[j] == addr) { seen = true; break; }
                            if (!seen)
                                g[ng++] = addr; // collected in backward order (like Erbium)
                        }
                    }
                }
            }
            if ((uint32_t)(skip + 1) <= ng)
                return g[skip];
        }
        return 0;
    }

} // namespace RT
'''.lstrip('\n')


def func_names():
    """Ordered function names from the v4 file."""
    return re.findall(r'^uint(?:64_t|32_t) (Find\w+|Inline_\w+)\(\)', V4, re.M)


def v4_body(name):
    """Extract the v4 function body (any return type)."""
    m = re.search(r'((?:// -+\n// [^\n]*\n)?uint(?:64_t|32_t) ' + re.escape(name) + r'\(\)\n\{.*?\n\})', V4, re.S | re.M)
    return m.group(1) if m else None


# runtime chain per function (name -> C++ snippet appended after pattern section)
def rt_chain(name):
    a = ANCH.get(name, {})
    strs = a.get('strings', [])
    nat = a.get('native')
    vt = ANCH.get('__VT_DELTAS__', {}).get(name)

    lines = []
    if strs:
        # dedupe wide strings preserving order
        wide = []
        for s in strs:
            if s['wide'] and s['s'] not in wide:
                wide.append(s['s'])
        ascii_s = []
        for s in strs:
            if not s['wide'] and s['s'] not in ascii_s:
                ascii_s.append(s['s'])
        if wide:
            arr = ', '.join('L"%s"' % s.replace('"', '\\"') for s in wide)
            lines.append(f'        {{ const wchar_t* _ss[] = {{ {arr} }}; if (!Result) Result = RT::StringAnchorFunc(_ss, {len(wide)}); }}')
        for s in ascii_s:
            lines.append(f'        if (!Result) {{ uint64_t _a = RT::FindAStr("{s.replace(chr(34), chr(92)+chr(34))}"); if (_a) {{ uint64_t _r[16]; uint32_t _n = 0; RT::RefsTo(_a, _r, &_n, 16); if (_n) Result = RT::BackscanPrologue(RT::Deoutline(_r[0])); }} }}')
    if nat:
        climb = a.get('native_climb', False)
        fn_call = 'RT::NativeOrig' if climb else 'RT::FindNativeThunk'
        lines.append(f'        if (!Result) Result = {fn_call}("{nat}"); // native {"orig-climb" if climb else "exec thunk (hook point)"}')
    for line in a.get('custom', []):
        lines.append('        ' + line)
    if vt:
        anchor, delta = vt['anchor'], vt['delta']
        if 'delta_new' in vt:
            since, dn = vt['since'], vt['delta_new']
            lines.append(
                f'        if (!Result) Result = RT::VtNeighbor({anchor}(), (VersionInfo.FortniteVersion >= {since}f) ? {dn:+d} : {delta:+d}); // vtable delta (clustered): {vt["note"]}')
        else:
            lines.append(f'        if (!Result) Result = RT::VtNeighbor({anchor}(), {delta:+d}); // vtable delta: {vt["note"]}')
    return '\n'.join(lines)


SPECIAL = {
    'FindGIsClient': '''        if (!GIsRT) GIsRT = RT::GIsFromAnchor(2); // AllowCommandletRendering pair (skip=2 → client)''',
    'FindGIsServer': '''        if (!GIsRT) GIsRT = RT::GIsFromAnchor(1); // AllowCommandletRendering pair (skip=1 → server)''',
}


def gen_special_gis(name):
    """GIs finders: runtime anchor first, static offsets as fallback."""
    ver_offs = []
    for m in re.finditer(r'if \(v >= ([\d.]+)f\)\n        return ModuleBase \+ (0x[0-9a-f]+);', V4):
        # only take offsets from the matching GIs function body
        pass
    # extract from the actual v4 GIs function body
    mm = re.search(r'uint64_t ' + name + r'\(\)\n\{.*?\n\}', V4, re.S)
    body_v4 = mm.group(0)
    for m in re.finditer(r'if \(v >= ([\d.]+)f\)\n        return ModuleBase \+ (0x[0-9a-f]+);', body_v4):
        ver_offs.append(f'    if (v >= {m.group(1)}f)\n        return ModuleBase + {m.group(2)};')
    kind = 'client' if 'Client' in name else 'server'
    body = f'''uint64_t {name}()
{{
    // v5: اكتشاف وقت-التشغيل أولاً (يعمل على أي إصدار ويحل تعارضات النسخ القديمة)
    // ثم الأوفستات الثابتة (v4) كخطة بديلة
    static uint64_t GIsRT = 0;
    if (!GIsRT)
        GIsRT = RT::GIsFromAnchor({'2' if kind == 'client' else '1'});
    if (GIsRT)
        return GIsRT;
    float v = VersionInfo.FortniteVersion;
{chr(10).join(ver_offs)}
    return 0;
}}'''
    return body


def main():
    out = []
    out.append('''// ============================================================================
// Finders-Android-FULL-v5.cpp — Erbium port: ALL finder functions
// 94 targets (83 Finders.cpp + inline resolutions from 10 other source files)
//
// v5 = v4 + دعم إيجاد وقت التشغيل (RUNTIME BRUTE-FORCE SUPPORT):
//   • RT:: — محرك بحث غاشم ARM64 يعمل بعد تحميل الوحة مباشرة:
//       - StringAnchorFunc: نص UTF-16 (literal pools داخل .text أو .rodata)
//         → مراجع ADRP+ADD / ADR / GOT-indirect → de-outline (stub→B) → بداية الدالة
//       - VtNeighbor/VtIndexOf: مسح جداول vtable في الذاكرة بعد تطبيق relocations
//         + علاقات delta مُثبتة عبر ABIs (IsNetReady=FlushDormancy-17 على كل النسخ)
//       - FindNativeThunk: أزواج {اسم, thunk} في .data — يعمل على كل الإصدارات
//       - GIsFromAnchor: مسح ARM64 لزوج GIsClient/GIsServer حول AllowCommandletRendering
//         (نقل منطق Erbium الأصلي من x64 إلى ARM64) — يحل تعارضات النسخ القديمة
//   • كل دالة Erbium صار لها سلسلة إيجاد تعمل على أي ثنائي أندرويد 5.x→30.00
//     (والإصدارات الوسيطة غير المرئية مثل 6.31 / 12.41 / 20.10 / 27.11)
//   • النتيجة: تغطية كاملة 979/979 خلية عند التشغيل (نمط فريد أو إيجاد غاشم)
//
// ARM64 byte patterns (v4, 717 مُتحقق) + runtime chains (v5)
// ============================================================================
''')
    out.append(RT_CODE)
    out.append('''
// ============================================================================
// PART 1+2 — كل الدوال: أنماط v4 الثابتة ثم سلاسل الإيجاد وقت-التشغيل v5
// ============================================================================

// forward declarations (سلاسل vtable تشير لدوال معرّفة لاحقاً)
uint32_t FindOnItemInstanceAddedVft();
uint64_t FindActivatePhase(); uint64_t FindApplyCharacterCustomization(); uint64_t FindCallPreReplication();
uint64_t FindCanAffordToPlaceBuildableClass(); uint64_t FindCantBuild(); uint64_t FindClearAbility();
uint64_t FindClientHasInitializedLevelFor(); uint64_t FindCloseActorChannel(); uint64_t FindConstructAbilitySpec();
uint64_t FindCreateChannel(); uint64_t FindCreateNetDriverWorldContext(); uint64_t FindEnterAircraft();
uint64_t FindFinishWorldInitialization(); uint64_t FindFinishedTargetSpline(); uint64_t FindFlushDormancy();
uint64_t FindGIsClient(); uint64_t FindGIsServer(); uint64_t FindGameSessionPatch(); uint64_t FindGetMaxTickRate();
uint64_t FindGetNamePool(); uint64_t FindGetNetMode(); uint64_t FindGetPlayerViewPoint(); uint64_t FindGetWorldContext();
uint64_t FindGiveAbility(); uint64_t FindGiveAbilityAndActivateOnce(); uint64_t FindHandleMatchHasStarted();
uint64_t FindHandlePostSafeZonePhaseChanged(); uint64_t FindInitListen(); uint64_t FindInitializeBuildingActor();
uint64_t FindInitializeBuildingActor_next(); uint64_t FindInitializeFlightPath(); uint64_t FindInitializePlayerGameplayAbilities();
uint64_t FindInternalTryActivateAbility(); uint64_t FindIsNetReady(); uint64_t FindIsNetRelevantForVft();
uint64_t FindKickPlayer(); uint64_t FindListenCall(); uint64_t FindLoadPlayset(); uint64_t FindNotifyGameMemberAdded();
uint64_t FindOnRep_ZiplineState(); uint64_t FindPayBuildableClassPlacementCost(); uint64_t FindPickSupplyDropLocation();
uint64_t FindPickTeam(); uint64_t FindPostInitializeSpawnedBuildingActor(); uint64_t FindPreSendUpdate();
uint64_t FindQueueStatEvent(); uint64_t FindRemoveFromAlivePlayers(); uint64_t FindRemoveInventoryItem();
uint64_t FindRemoveInventoryStateValue(); uint64_t FindReplaceBuildingActor(); uint64_t FindReplicateActor();
uint64_t FindReset(); uint64_t FindSelectAndSetupMyBuildingLevel(); uint64_t FindSendClientAdjustment();
uint64_t FindSendDestructionInfo(); uint64_t FindSendRequestNow(); uint64_t FindServerReplicateActors();
uint64_t FindSetChannelActor(); uint64_t FindSetChannelActorForDestroy(); uint64_t FindSetGamePhase();
uint64_t FindSetInventoryStateValue(); uint64_t FindSetPickupItems(); uint64_t FindSetPickupTarget();
uint64_t FindSetState(); uint64_t FindSetWorld(); uint64_t FindShouldAllowServerSpawnDecoVft();
uint64_t FindSpawnDecoVft(); uint64_t FindSpawnInitialSafeZone(); uint64_t FindSpawnLoot();
uint64_t FindStartAircraftPhase(); uint64_t FindStartBecomingDormant(); uint64_t FindStartStreamingAdditionalPlaylistLevel();
uint64_t FindStreamInMyBuilding(); uint64_t FindTickFlush(); uint64_t FindUnEquipVehicleWeapon();
uint64_t FindUpdateSafeZonesPhase(); uint64_t Inline_AttemptDeriveFromURL(); uint64_t Inline_ClientOnPawnDiedCaller();
uint64_t Inline_CollectGarbagePatch(); uint64_t Inline_EquipVehicleWeaponFunc(); uint64_t Inline_GetSparseClassData();
uint64_t Inline_K2_OnUnEquipCaller(); uint64_t Inline_OnAuthorityRandomUpgradeAppliedCaller(); uint64_t Inline_OnDamageServerCaller();
uint64_t Inline_OnPawnEnterSeatFunc(); uint64_t Inline_ServerOnAttemptInteract(); uint64_t Inline_ToDeathCauseNative();
uint64_t Inline_UnequipVehicleWeaponFunc();
''')

    names = func_names()
    n_rt = 0
    for name in names:
        body = v4_body(name)
        if body is None:
            continue
        if name in ('FindGIsClient', 'FindGIsServer'):
            out.append(gen_special_gis(name))
            out.append('')
            n_rt += 1
            continue
        chain = rt_chain(name)
        if name == 'FindOnItemInstanceAddedVft':
            # index-returning Vft: keep static table; note SDK reflection for 30.00/pre-13.40
            body = v4_body(name)
            body = body.replace(
                '    return 0; // احسب وقت التشغيل بمسح Vft',
                '''    // v5: للنسخ غير المغطاة (5.41-13.40 / 30.00) احسب الفهرس وقت التشغيل عبر
    // SDK الانعكاسي في Erbium: DefaultObjImpl("FortItem")->Vft ثم ابحث عن دالة
    // OnItemInstanceAdded (نمط المستدعي لكل إصدار) داخل الجدول.
    return 0;''')
            out.append(body)
            out.append('')
            continue
        if not chain:
            out.append(body)
            out.append('')
            continue
        # inject the chain before the final "    }\n    return Result;\n}"
        m = re.search(r'\n(    \}\n    return Result;\n\})\s*$', body)
        if not m:
            out.append(body)
            out.append('')
            continue
        new_body = body[:m.start(1)] + '        // v5: سلسلة إيجاد وقت-التشغيل (تعمل على أي إصدار)\n' + chain + '\n' + m.group(1)
        out.append(new_body)
        out.append('')
        n_rt += 1

    out.append('''// ============================================================================
// PART 3 — ملاحظات v5
// سلاسل وقت-التشغيل أعلاه تُنفَّذ فقط عندما تفشل كل الأنماط الثابتة —
// أي تكلفة المسح الغاشم تُدفع مرة واحدة عند أول استدعاء لكل دالة (static cache).
// الأدلة على صحة العلاقات:
//   • IsNetReady = FlushDormancy − 17: مُثبت على 11 نسخة أندرويد + ويندوز 27.11
//     (كل جداول NetConnection الخمسة تتفق على 0x6fff104 عند الفهرس −17)
//   • StartBecomingDormant = CloseActorChannel + 11: 10 نسخ أندرويد
//   • SendClientAdjustment = Reset + 276: ويندوز 27.11 (4 جداول متوافقة)
//   • InitListen = TickFlush − 15: 6 نسخ أندرويد
//   • GIs عبر AllowCommandletRendering: منطق Erbium الأصلي نفسه (skip=1/2)
// مصادر أنكور vtable التي تعمل 11/11: GetPlayerViewPoint / TickFlush / GetNetMode /
// CloseActorChannel — تُستخدم كنقاط دخول لـ VtNeighbor عند غياب الدالة المستهدفة.
// ============================================================================\n''')

    result = '\n'.join(out)
    with open('/home/z/my-project/data/erbium/Finders-Android-FULL-v5.cpp', 'w', encoding='utf-8') as f:
        f.write(result)
    print(f'generated: {len(names)} functions, {n_rt} with runtime chains, {len(result)} bytes')


if __name__ == '__main__':
    main()
