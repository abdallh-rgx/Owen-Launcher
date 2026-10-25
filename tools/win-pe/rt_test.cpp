// RUNTIME FUNCTIONAL TEST for the RT:: engine in Finders-Android-FULL-v5.cpp
// Builds a synthetic ARM64 ELF module in memory (text/rodata/data), then verifies:
//   FindWStr, FindAStr, ADRP+ADD / ADR / GOT refs, Deoutline, BackscanPrologue,
//   StringAnchorFunc, VtSlotsWith/VtNeighbor/VtMethodAt, FindNativeThunk, GIsFromAnchor.
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <cwchar>
#include <sys/mman.h>

static uintptr_t ModuleBase = 0;

// --- stubs for the Erbium environment ---
namespace Memcury
{
    struct Scanner
    {
        uintptr_t addr = 0;
        static Scanner FindPattern(const char*, bool = true) { return Scanner{}; }
        uintptr_t Get() const { return addr; }
        bool IsValid() const { return addr != 0; }
    };
} // namespace Memcury
struct FVersionInfoStub { double EngineVersion = 0; double FortniteVersion = 0; };
static FVersionInfoStub VersionInfo;

#include "/home/z/my-project/data/erbium/Finders-Android-FULL-v5.cpp"

// ---- synthetic module layout ----
// [0x0000..0x1000)  : ELF header + phdrs
// [0x1000..0x2000)  : .text (RX)
// [0x2000..0x3000)  : .rodata (R)  — strings
// [0x3000..0x4000)  : .got (RW)
// [0x4000..0x6000)  : .data (RW)   — vtables, natives, globals
// [0x6000..0x8000)  : .bss (RW|anonymous)

static uint32_t MakeADRP(uint8_t rd, uint64_t pc, uint64_t target)
{
    uint64_t page = target & ~0xFFFULL;
    uint64_t pcPage = pc & ~0xFFFULL;
    uint64_t imm = (page - pcPage) >> 12;
    uint32_t lo = imm & 3;
    uint64_t hi = (imm >> 2) & 0x7FFFF;
    return 0x90000000u | (lo << 29) | (hi << 5) | rd;
}
static uint32_t MakeADD(uint8_t rd, uint8_t rn, uint32_t imm12)
{
    return 0x91000000u | ((imm12 & 0xFFF) << 10) | (rn << 5) | rd;
}
static uint32_t MakeADR(uint8_t rd, uint64_t pc, uint64_t target)
{
    uint64_t off = target - pc;
    uint32_t lo = off & 3;
    uint64_t hi = (off >> 2) & 0x7FFFF;
    return 0x10000000u | (lo << 29) | (hi << 5) | rd;
}
static uint32_t MakeLDR64(uint8_t rt, uint8_t rn, uint32_t imm12_8)
{
    return 0xF9400000u | ((imm12_8 / 8) << 10) | (rn << 5) | rt;
}
static uint32_t MakeLDRB(uint8_t rt, uint8_t rn, uint32_t imm12)
{
    return 0x39400000u | ((imm12 & 0xFFF) << 10) | (rn << 5) | rt;
}
static uint32_t MakeB(uint64_t pc, uint64_t target)
{
    uint32_t imm26 = (uint32_t)(((target - pc) >> 2) & 0x3FFFFFF);
    return 0x14000000u | imm26;
}
static uint32_t MakeSTPX29()
{
    return 0xA9BF7BFDu; // stp x29, x30, [sp, #-0x10]!
}

static int g_Fail = 0;
#define CHECK(cond, name)                                                                                              \
    do                                                                                                                 \
    {                                                                                                                  \
        if (cond)                                                                                                      \
            printf("  [PASS] %s\n", name);                                                                             \
        else                                                                                                           \
        {                                                                                                              \
            printf("  [FAIL] %s\n", name);                                                                             \
            g_Fail++;                                                                                                  \
        }                                                                                                              \
    } while (0)

int main()
{
    // ---- build the synthetic module ----
    size_t MOD = 0x8000;
    uint8_t* m = (uint8_t*)mmap((void*)0x500000000000ULL, MOD, PROT_READ | PROT_WRITE | PROT_EXEC,
                                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (m == MAP_FAILED)
    {
        printf("mmap failed\n");
        return 1;
    }
    memset(m, 0, MOD);
    ModuleBase = (uintptr_t)m;

    // ELF64 header
    uint32_t magic = 0x464C457F;
    memcpy(m, &magic, 4);
    uint8_t cls = 2;
    memcpy(m + 4, &cls, 1); // ELFCLASS64
    *(uint16_t*)(m + 54) = 56; // phentsize
    *(uint16_t*)(m + 56) = 4;  // phnum
    *(uint64_t*)(m + 32) = 64; // phoff
    struct Phdr
    {
        uint32_t type, flags;
        uint64_t offset, vaddr, paddr, filesz, memsz, align;
    };
    Phdr ph[4] = {
        {1, 5, 0, 0, 0, 0x2000, 0x2000, 0x1000},      // RX: text 0x0-0x2000
        {1, 4, 0x2000, 0x2000, 0x2000, 0x1000, 0x1000, 0x1000}, // R: rodata
        {1, 6, 0x3000, 0x3000, 0x3000, 0x3000, 0x5000, 0x1000}, // RW: got+data+bss 0x3000-0x8000
        {0, 0, 0, 0, 0, 0, 0, 0},
    };
    memcpy(m + 64, ph, sizeof(ph));

    uint8_t* text = m + 0x1000;
    uint8_t* rodata = m + 0x2000;
    uint8_t* got = m + 0x3000;
    uint8_t* data = m + 0x4000;

    // 1) UTF-16 string in rodata
    const wchar_t* WSTR = L"FlushDormancy: %s. Connection: %s";
    wchar_t* wstrAddr = (wchar_t*)rodata;
    wcsncpy((wchar_t*)rodata, WSTR, 64);

    // 2) ASCII string in rodata (after the wide one)
    char* astrAddr = (char*)(rodata + 0x200);
    strcpy((char*)(rodata + 0x200), "ReplicationSystem_PreSendUpdate");

    // 3) FUNCTION A @ text+0x100: prologue + ADRP+ADD (ref to wstr) + ret
    uint64_t fnA = (uintptr_t)(text + 0x100);
    uint32_t* t = (uint32_t*)fnA;
    t[0] = MakeSTPX29();
    t[1] = 0xD10043FFu; // sub sp, sp, #0x10
    t[2] = MakeADRP(8, (uintptr_t)&t[2], (uintptr_t)wstrAddr);
    t[3] = MakeADD(8, 8, ((uintptr_t)wstrAddr) & 0xFFF);
    t[4] = 0xD65F03C0u; // ret

    // 4) FUNCTION B @ text+0x200: prologue + ADR (direct, within ±1MB) ref to astr + ret
    uint64_t fnB = (uintptr_t)(text + 0x200);
    t = (uint32_t*)fnB;
    t[0] = MakeSTPX29();
    t[1] = MakeADR(9, (uintptr_t)&t[1], (uintptr_t)astrAddr);
    t[2] = 0xD65F03C0u;

    // 5) GOT-indirect ref: FUNCTION C @ text+0x300 with ADRP X10, got-page; LDR X10,[X10,#0]; (got slot 0 -> wstrAddr)
    uint64_t fnC = (uintptr_t)(text + 0x300);
    t = (uint32_t*)fnC;
    t[0] = MakeSTPX29();
    t[1] = MakeADRP(10, (uintptr_t)&t[1], (uintptr_t)got);
    t[2] = MakeLDR64(10, 10, 0);
    t[3] = 0xD65F03C0u;
    *(uint64_t*)got = (uint64_t)wstrAddr; // GOT slot resolves to the wide string

    // 6) stub (outliner): text+0x400: B fnD_real ; real fnD at text+0x500 with prologue+ADRP/ADD to wstr
    uint64_t stubD = (uintptr_t)(text + 0x400);
    uint64_t fnD = (uintptr_t)(text + 0x500);
    *(uint32_t*)stubD = MakeB(stubD, fnD);
    t = (uint32_t*)fnD;
    t[0] = MakeSTPX29();
    t[1] = MakeADRP(11, (uintptr_t)&t[1], (uintptr_t)wstrAddr);
    t[2] = MakeADD(11, 11, ((uintptr_t)wstrAddr) & 0xFFF);
    t[3] = 0xD65F03C0u;

    // 7) vtable in data: vtable_start, 4 entries: [0]=fnA [1]=fnB [2]=fnC [3]=fnD
    uint64_t* vt = (uint64_t*)(data + 0x100);
    vt[0] = fnA;
    vt[1] = fnB;
    vt[2] = fnC;
    vt[3] = fnD;
    uint64_t vtBase = (uintptr_t)vt;

    // 8) natives pair in data: {char* "ServerKickPlayer", thunk}
    char* natName = (char*)(data + 0x400);
    strcpy(natName, "ServerKickPlayer");
    uint64_t* natPair = (uint64_t*)(data + 0x600);
    natPair[0] = (uint64_t)natName;
    natPair[1] = fnA; // thunk "points to" fnA

    // 9) GIs globals in data + function with LDRB refs near an AllowCommandletRendering ref
    // Erbium semantics (Finders.cpp): matches collected BACKWARD from the string ref;
    // skip=1 → 2nd match = GIsServer, skip=2 → 3rd match = GIsClient.
    // Natural code order (far→near): GIsClient load, GIsServer load, GAllowCommandletRendering load, string ref.
    uint64_t gisS = (uintptr_t)(data + 0x700);
    uint64_t gisC = (uintptr_t)(data + 0x710);
    uint64_t gAllow = (uintptr_t)(data + 0x720);
    const wchar_t* ACR = L"AllowCommandletRendering";
    wchar_t* acrAddr = (wchar_t*)(rodata + 0x400);
    wcsncpy(acrAddr, ACR, 64);
    uint64_t fnG = (uintptr_t)(text + 0x600);
    t = (uint32_t*)fnG;
    int k = 0;
    t[k++] = MakeSTPX29();
    // farthest: GIsClient load
    t[k] = MakeADRP(8, (uintptr_t)&t[k], gisC);
    k++;
    t[k++] = MakeLDRB(9, 8, gisC & 0xFFF);
    // middle: GIsServer load
    t[k] = MakeADRP(12, (uintptr_t)&t[k], gisS);
    k++;
    t[k++] = MakeLDRB(13, 12, gisS & 0xFFF);
    // nearest: GAllowCommandletRendering load
    t[k] = MakeADRP(14, (uintptr_t)&t[k], gAllow);
    k++;
    t[k++] = MakeLDRB(15, 14, gAllow & 0xFFF);
    // string ref
    t[k] = MakeADRP(0, (uintptr_t)&t[k], (uintptr_t)acrAddr);
    k++;
    t[k++] = MakeADD(0, 0, ((uintptr_t)acrAddr) & 0xFFF);
    t[k++] = 0xD65F03C0u;

    printf("== RT:: engine functional test (synthetic ARM64 module) ==\n");

    // FindWStr
    uint64_t w = RT::FindWStr(WSTR);
    CHECK(w == (uintptr_t)wstrAddr, "FindWStr locates UTF-16 string in rodata");

    // FindAStr
    uint64_t a = RT::FindAStr("ReplicationSystem_PreSendUpdate");
    CHECK(a == (uintptr_t)astrAddr, "FindAStr locates ASCII string");

    // RefsTo: ADRP+ADD (fnA)
    uint64_t refs[64];
    uint32_t n = 0;
    RT::RefsTo((uintptr_t)wstrAddr, refs, &n, 64);
    bool foundA = false, foundC = false, foundD = false;
    for (uint32_t i = 0; i < n; i++)
    {
        if (refs[i] == fnA + 8)
            foundA = true;
        if (refs[i] == fnC + 4)
            foundC = true;
        if (refs[i] == fnD + 4)
            foundD = true;
    }
    CHECK(foundA, "RefsTo finds ADRP+ADD ref (fnA)");
    CHECK(foundC, "RefsTo finds GOT-indirect ref (fnC)");
    CHECK(foundD, "RefsTo finds deoutline-needed ref (fnD via stub->real)");

    // RefsTo: ADR (fnB -> astr)
    n = 0;
    RT::RefsTo((uintptr_t)astrAddr, refs, &n, 64);
    bool foundB = false;
    for (uint32_t i = 0; i < n; i++)
        if (refs[i] == fnB + 4)
            foundB = true;
    CHECK(foundB, "RefsTo finds direct ADR ref (fnB)");

    // Deoutline
    CHECK(RT::Deoutline(stubD) == fnD, "Deoutline follows B stub to real function");

    // BackscanPrologue
    CHECK(RT::BackscanPrologue(fnA + 8) == fnA, "BackscanPrologue finds fnA start");
    CHECK(RT::BackscanPrologue(fnG + 0x18) == fnG, "BackscanPrologue finds fnG start");

    // StringAnchorFunc: first ref of wstr is fnA (refs ordered by address: fnA < fnC < fnD)
    uint64_t fA = RT::StringAnchorFunc(WSTR);
    CHECK(fA == fnA, "StringAnchorFunc resolves containing function (fnA)");

    // VtSlotsWith / VtNeighbor
    uint64_t slots[96];
    uint32_t ns = RT::VtSlotsWith(fnA, slots, 96);
    CHECK(ns >= 1 && slots[0] == vtBase + 0, "VtSlotsWith finds fnA's vtable slot");
    CHECK(RT::VtNeighbor(fnA, +3) == fnD, "VtNeighbor(+3) = fnD (delta within vtable)");
    CHECK(RT::VtNeighbor(fnD, -3) == fnA, "VtNeighbor(-3) = fnA");
    CHECK(RT::VtNeighbor(fnB, +2) == fnD, "VtNeighbor(+2) from fnB = fnD");
    CHECK(RT::VtMethodAt(fnA, 2) == fnC, "VtMethodAt(anchor=fnA, index=2) = fnC");
    uint64_t vtOut = 0;
    uint32_t idx = RT::VtIndexOf(fnD, &vtOut);
    CHECK(idx == 3 && vtOut == vtBase, "VtIndexOf(fnD) = 3 with correct vtable start");

    // FindNativeThunk
    CHECK(RT::FindNativeThunk("ServerKickPlayer") == fnA, "FindNativeThunk resolves {name,thunk} pair");

    // GIsFromAnchor: backward collection → [0]=gisS (nearest), [1]=gisC → skip=1 → server, skip=2 → client
    uint64_t gs = RT::GIsFromAnchor(1);
    uint64_t gc = RT::GIsFromAnchor(2);
    CHECK(gs == gisS, "GIsFromAnchor(1) = GIsServer global");
    CHECK(gc == gisC, "GIsFromAnchor(2) = GIsClient global");

    printf(g_Fail ? "== %d FAILURES ==\n" : "== ALL TESTS PASSED ==\n", g_Fail);
    return g_Fail ? 1 : 0;
}
