// =============================================================================
// ogscompat.cpp — implementation of the Ogs compatibility layer
// (see cpp/OgsCompat.h).  No Windows headers, no MinHook, no MSVC.
// =============================================================================

#include "OgsCompat.h"

#include <cstdarg>
#include <type_traits>
#include <utility>
#include <mutex>
#include <fstream>
#include <sstream>

// ===========================================================================
// Config storage + tiny flat JSON reader (~150 lines, NO dependency).
// Handles the ogsen.json shape:
//   {"features":{...bools/numbers...},"vt_indices":{...},"game":"..."}
// Strings, nested arrays and unknown members are skipped robustly.
// ===========================================================================
namespace
{
        Ogs::Cfg g_Cfg;
        bool g_CfgInit = false;
        std::mutex g_LogMutex;
        bool g_LogFileStarted = false;

        // --- minimal JSON tokenizer/scanner -------------------------------------
        struct JsonScan
        {
                const char* p;
                bool bad = false;

                void ws()
                {
                        while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n')
                                ++p;
                }

                bool lit(const char* s)
                {
                        size_t n = std::strlen(s);
                        if (std::strncmp(p, s, n) == 0)
                        {
                                p += n;
                                return true;
                        }
                        return false;
                }

                // parse a JSON string into out (handles \uXXXX for ASCII plane; all
                // config keys in scope are ASCII)
                void str(std::string& out)
                {
                        out.clear();
                        ws();
                        if (*p != '"') { bad = true; return; }
                        ++p;
                        while (*p && *p != '"')
                        {
                                if (*p == '\\' && p[1])
                                {
                                        ++p;
                                        switch (*p)
                                        {
                                        case 'n': out += '\n'; break;
                                        case 't': out += '\t'; break;
                                        case 'r': out += '\r'; break;
                                        case 'b': out += '\b'; break;
                                        case 'f': out += '\f'; break;
                                        case 'u':
                                        {
                                                // \uXXXX — encode as UTF-8 (enough for keys)
                                                if (p[1] && p[2] && p[3] && p[4])
                                                {
                                                        unsigned v = 0;
                                                        for (int i = 1; i <= 4; ++i)
                                                        {
                                                                char c = p[i];
                                                                v <<= 4;
                                                                if (c >= '0' && c <= '9') v |= unsigned(c - '0');
                                                                else if (c >= 'a' && c <= 'f') v |= unsigned(c - 'a' + 10);
                                                                else if (c >= 'A' && c <= 'F') v |= unsigned(c - 'A' + 10);
                                                                else { bad = true; return; }
                                                        }
                                                        if (v < 0x80) out += char(v);
                                                        else if (v < 0x800)
                                                        {
                                                                out += char(0xC0 | (v >> 6));
                                                                out += char(0x80 | (v & 0x3F));
                                                        }
                                                        else
                                                        {
                                                                out += char(0xE0 | (v >> 12));
                                                                out += char(0x80 | ((v >> 6) & 0x3F));
                                                                out += char(0x80 | (v & 0x3F));
                                                        }
                                                        p += 4;
                                                }
                                                else { bad = true; return; }
                                                break;
                                        }
                                        default: out += *p; break;
                                        }
                                        ++p;
                                }
                                else
                                {
                                        out += *p++;
                                }
                        }
                        if (*p != '"') { bad = true; return; }
                        ++p;
                }

                // skip one JSON value (object/array/string/number/literal)
                void skipValue()
                {
                        ws();
                        if (*p == '{' || *p == '[')
                        {
                                char open = *p, close = (open == '{') ? '}' : ']';
                                int depth = 0;
                                while (*p)
                                {
                                        if (*p == '"') { std::string tmp; str(tmp); if (bad) return; continue; }
                                        if (*p == open) ++depth;
                                        else if (*p == close)
                                        {
                                                --depth;
                                                if (depth == 0) { ++p; return; }
                                        }
                                        ++p;
                                }
                                bad = true;
                                return;
                        }
                        if (*p == '"') { std::string tmp; str(tmp); return; }
                        // number / true / false / null
                        while (*p && *p != ',' && *p != '}' && *p != ']')
                                ++p;
                }
        };

        void ParseFeaturesObject(JsonScan& s, Ogs::Cfg& cfg)
        {
                s.ws();
                if (!s.lit("{")) { s.bad = true; return; }
                s.ws();
                if (s.lit("}")) return;
                for (;;)
                {
                        std::string key;
                        s.str(key);
                        if (s.bad) return;
                        s.ws();
                        if (!s.lit(":")) { s.bad = true; return; }
                        s.ws();
                        if (s.lit("true"))       cfg.Features[key] = true;
                        else if (s.lit("false")) cfg.Features[key] = false;
                        else if (*s.p == '-' || (*s.p >= '0' && *s.p <= '9'))
                        {
                                char* end = nullptr;
                                long long v = std::strtoll(s.p, &end, 10);
                                if (end == s.p) { s.bad = true; return; }
                                cfg.Numbers[key] = v;
                                s.p = end;
                        }
                        else
                        {
                                s.skipValue();
                        }
                        s.ws();
                        if (s.lit(",")) continue;
                        if (s.lit("}")) return;
                        s.bad = true;
                        return;
                }
        }

        void ParseVtObject(JsonScan& s, Ogs::Cfg& cfg)
        {
                s.ws();
                if (!s.lit("{")) { s.bad = true; return; }
                s.ws();
                if (s.lit("}")) return;
                for (;;)
                {
                        std::string key;
                        s.str(key);
                        if (s.bad) return;
                        s.ws();
                        if (!s.lit(":")) { s.bad = true; return; }
                        s.ws();
                        if (*s.p == '-' || (*s.p >= '0' && *s.p <= '9'))
                        {
                                char* end = nullptr;
                                long long v = std::strtoll(s.p, &end, 10);
                                if (end == s.p) { s.bad = true; return; }
                                cfg.VtIndices[key] = v;
                                s.p = end;
                        }
                        else
                        {
                                s.skipValue();
                        }
                        s.ws();
                        if (s.lit(",")) continue;
                        if (s.lit("}")) return;
                        s.bad = true;
                        return;
                }
        }

        void ParseTop(JsonScan& s, Ogs::Cfg& cfg)
        {
                s.ws();
                if (!s.lit("{")) return; // not an object → keep defaults
                s.ws();
                if (s.lit("}")) return;
                for (;;)
                {
                        std::string key;
                        s.str(key);
                        if (s.bad) return;
                        s.ws();
                        if (!s.lit(":")) { s.bad = true; return; }
                        if (key == "features")
                                ParseFeaturesObject(s, cfg);
                        else if (key == "vt_indices")
                                ParseVtObject(s, cfg);
                        else if (key == "game")
                                s.str(cfg.Game);
                        else
                                s.skipValue();
                        if (s.bad) return;
                        s.ws();
                        if (s.lit(",")) continue;
                        if (s.lit("}")) return;
                        return;
                }
        }
} // namespace

namespace Ogs
{
        void Init(const char* configJson)
        {
                if (g_CfgInit)
                        return;
                g_CfgInit = true;
                g_Cfg = Cfg{};
                if (!configJson)
                        return;
                JsonScan s{configJson};
                ParseTop(s, g_Cfg);
                if (s.bad)
                        Log("ogsen.json parse error — keeping defaults");
        }

        Cfg& CfgRef()
        {
                if (!g_CfgInit)
                        Init(nullptr);
                return g_Cfg;
        }

        // =========================================================================
        // Logging — logcat on Android, stderr on the host, ogsen_log.txt always
        // (best effort, mirrors the original OGS_log.txt behaviour).
        // =========================================================================
        void LogFile(const char* line)
        {
                std::lock_guard<std::mutex> lk(g_LogMutex);
                // game files dir, same derivation as the Rust side (ogsen.json lives
                // next to this file): /sdcard/Android/data/<pkg>/files/
                std::string dir;
                {
                        std::ifstream cmd("/proc/self/cmdline", std::ios::binary);
                        std::string pkg((std::istreambuf_iterator<char>(cmd)), std::istreambuf_iterator<char>());
                        std::string::size_type z = pkg.find('\0');
                        if (z != std::string::npos)
                                pkg.resize(z);
                        if (!pkg.empty())
                                dir = "/sdcard/Android/data/" + pkg + "/files";
                }
                if (dir.empty())
                        return;
                std::string path = dir + "/ogsen_log.txt";
                if (!g_LogFileStarted)
                {
                        g_LogFileStarted = true;
                        std::ofstream f(path, std::ios::trunc);
                        if (f.is_open())
                                f << "[OGS]: Log file initialized!\n";
                }
                std::ofstream f(path, std::ios::app);
                if (f.is_open())
                        f << "[OGS]: " << line << "\n";
        }

        void Log(const char* fmt, ...)
        {
                char buf[1024];
                va_list ap;
                va_start(ap, fmt);
                vsnprintf(buf, sizeof(buf), fmt, ap);
                va_end(ap);

#if OGS_ON_ANDROID
                __android_log_print(ANDROID_LOG_INFO, "OwenOGS", "%s", buf);
#else
                std::fprintf(stderr, "[I/OwenOGS] %s\n", buf);
#endif
                LogFile(buf);
        }

        // =========================================================================
        // Module base / memory helpers
        // =========================================================================
#if OGS_ON_ANDROID
        namespace
        {
                struct PhdrCtx
                {
                        uintptr_t Base = 0;
                        uintptr_t Span = 0;
                };

                // Total mapped span of a phdr set — the shim injection gives the
                // tiny loader shim the "libUE4.so" slot while the 268MB engine
                // lives at "libUE4real.so"; both match "libUE4*" and the REAL
                // engine must win (every baked offset is relative to it).
                static uintptr_t PhdrSpan(dl_phdr_info* info)
                {
                        uintptr_t lo = ~uintptr_t(0), hi = 0;
                        for (int i = 0; i < info->dlpi_phnum; ++i)
                        {
                                const ElfW(Phdr)* p = info->dlpi_phdr + i;
                                if (p->p_type != PT_LOAD)
                                        continue;
                                uintptr_t s = info->dlpi_addr + p->p_vaddr;
                                uintptr_t e = s + p->p_memsz;
                                if (s < lo) lo = s;
                                if (e > hi) hi = e;
                        }
                        return hi > lo ? (hi - lo) : 0;
                }

                int PhdrCb(dl_phdr_info* info, size_t, void* data)
                {
                        auto* ctx = static_cast<PhdrCtx*>(data);
                        const char* name = info->dlpi_name ? info->dlpi_name : "";
                        const bool ue4 = std::strstr(name, "libUE4") != nullptr;
                        const bool unreal = std::strstr(name, "libUnreal") != nullptr;
                        if (!ue4 && !unreal)
                                return 0;
                        // "libUE4real.so" (the shim-renamed engine) beats everything;
                        // otherwise the LARGEST libUE4*/libUnreal* module wins over
                        // any same-named loader shim.
                        uintptr_t span = PhdrSpan(info);
                        bool wins;
                        if (std::strstr(name, "libUE4real") != nullptr || std::strstr(name, "libUnreal") != nullptr)
                                wins = ctx->Span == 0 || span >= ctx->Span || ctx->Base == 0;
                        else
                                wins = ctx->Base == 0;
                        if (wins)
                        {
                                ctx->Base = reinterpret_cast<uintptr_t>(info->dlpi_addr);
                                ctx->Span = span;
                        }
                        return 0; // keep iterating: a later, larger engine must still win
                }
        }

        uintptr_t Base()
        {
                static uintptr_t cached = 0;
                static bool tried = false;
                if (!tried)
                {
                        tried = true;
                        PhdrCtx ctx;
                        dl_iterate_phdr(PhdrCb, &ctx);
                        cached = ctx.Base;
                        if (cached)
                                Log("Ogs::Base: libUE4 base = 0x%llx", (unsigned long long)cached);
                        else
                                Log("Ogs::Base: libUE4 module NOT found (offsets disabled)");
                }
                return cached;
        }

        void SleepMs(int ms)
        {
                usleep(static_cast<useconds_t>(ms) * 1000u);
        }

        bool MProtectRW(void* ptr, size_t len)
        {
                if (!ptr || len == 0)
                        return false;
                const size_t kPage = 4096;
                uintptr_t start = reinterpret_cast<uintptr_t>(ptr) & ~(kPage - 1);
                uintptr_t end = (reinterpret_cast<uintptr_t>(ptr) + len + kPage - 1) & ~(kPage - 1);
                if (mprotect(reinterpret_cast<void*>(start), end - start, PROT_READ | PROT_WRITE) != 0)
                        return false;
                return true;
        }
#else
        // Host: dummies so the syntax check and any host tooling stay inert.
        uintptr_t Base()
        {
                static uintptr_t dummy = 0x1000;
                return dummy;
        }
        void SleepMs(int ms)
        {
                std::this_thread::sleep_for(std::chrono::milliseconds(ms));
        }
        bool MProtectRW(void*, size_t)
        {
                return true;
        }
#endif

        // =========================================================================
        // Offset registry
        // =========================================================================
        const OffRegistry& Off()
        {
                static OffRegistry reg;
                return reg;
        }

        // =========================================================================
        // Ogs::Text — UTF-8 → SDK::FString, generic over the SDK element type.
        // =========================================================================
        SDK::FString Text(const char* utf8)
        {
                // Element type of the SDK's FString container (wchar_t on the
                // Windows dump, char16_t on the Android dump) — deduced from
                // operator[] so the code compiles against both.
                using Elem = std::remove_reference_t<decltype(std::declval<SDK::FString&>()[0])>;

                SDK::FString out;
                if (!utf8)
                {
                        out.Add(Elem(0));
                        return out;
                }

                const unsigned char* s = reinterpret_cast<const unsigned char*>(utf8);
                auto pushCp = [&](unsigned long cp)
                {
                        if constexpr (sizeof(Elem) >= 4)
                        {
                                out.Add(static_cast<Elem>(cp));
                        }
                        else if constexpr (sizeof(Elem) == 2)
                        {
                                if (cp < 0x10000)
                                {
                                        out.Add(static_cast<Elem>(cp));
                                }
                                else
                                {
                                        cp -= 0x10000;
                                        out.Add(static_cast<Elem>(0xD800 | (cp >> 10)));
                                        out.Add(static_cast<Elem>(0xDC00 | (cp & 0x3FF)));
                                }
                        }
                        else
                        {
                                out.Add(static_cast<Elem>(cp & 0xFF));
                        }
                };

                while (*s)
                {
                        if (*s < 0x80)
                        {
                                pushCp(*s++);
                        }
                        else if ((*s & 0xE0) == 0xC0 && (s[1] & 0xC0) == 0x80)
                        {
                                pushCp(((unsigned long)(*s & 0x1F) << 6) | (s[1] & 0x3F));
                                s += 2;
                        }
                        else if ((*s & 0xF0) == 0xE0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80)
                        {
                                pushCp(((unsigned long)(*s & 0x0F) << 12) | ((s[1] & 0x3F) << 6) | (s[2] & 0x3F));
                                s += 3;
                        }
                        else if ((*s & 0xF8) == 0xF0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80 && (s[3] & 0xC0) == 0x80)
                        {
                                pushCp(((unsigned long)(*s & 0x07) << 18) | ((s[1] & 0x3F) << 12) | ((s[2] & 0x3F) << 6) | (s[3] & 0x3F));
                                s += 4;
                        }
                        else
                        {
                                pushCp(0xFFFD); // invalid byte — replacement char
                                ++s;
                        }
                }
                out.Add(Elem(0)); // null terminator (FString counts it in Num())
                return out;
        }
} // namespace Ogs
