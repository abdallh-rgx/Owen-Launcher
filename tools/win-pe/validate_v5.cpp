// Syntax validation harness for Finders-Android-FULL-v5.cpp
// Stubs the Erbium/Memcury environment so the file can compile standalone.
#include <cstdint>
#include <cstring>
#include <cwchar>
#include <cstdio>
#include <vector>

typedef uint64_t uintptr_t;

// --- Memcury stub ---
namespace Memcury
{
    struct Scanner
    {
        uintptr_t addr = 0;
        static Scanner FindPattern(const char* /*pat*/, bool /*b*/ = true)
        {
            return Scanner{};
        }
        uintptr_t Get() const { return addr; }
        bool IsValid() const { return addr != 0; }
        Scanner RelativeOffset(int32_t /*off*/, bool /*b*/ = false) const { return *this; }
        Scanner ScanFor(std::vector<uint8_t> /*sig*/, bool /*b*/ = true) const { return *this; }
        static uintptr_t FindStringRef(const wchar_t* /*s*/, bool = true, int = 0, bool = false)
        {
            return 0;
        }
    };
} // namespace Memcury

static uintptr_t ModuleBase = 0x100000;

struct FVersionInfoStub
{
    double EngineVersion = 0;
    double FortniteVersion = 0;
};
static FVersionInfoStub VersionInfo;

// pull in the generated file
#include "/home/z/my-project/data/erbium/Finders-Android-FULL-v5.cpp"

int main()
{
    // touch every finder to force instantiation
    printf("%llu\n", (unsigned long long)FindActivatePhase());
    printf("%llu\n", (unsigned long long)FindGIsClient());
    printf("%llu\n", (unsigned long long)FindGIsServer());
    printf("%u\n", FindOnItemInstanceAddedVft());
    printf("%llu\n", (unsigned long long)FindIsNetRelevantForVft());
    printf("%llu\n", (unsigned long long)RT::GIsFromAnchor(1));
    printf("%llu\n", (unsigned long long)RT::VtNeighbor(0x1234, -17));
    return 0;
}
