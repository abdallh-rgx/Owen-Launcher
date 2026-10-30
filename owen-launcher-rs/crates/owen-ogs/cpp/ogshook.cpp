// =============================================================================
// ogshook.cpp — implementation of the hook facade (see cpp/ogshook.h).
// =============================================================================

#include "ogshook.h"

namespace
{
        constexpr int kMaxVTableScan = 2048; // ByExec scan bound (spec)

        bool VTableSlotPatch(void*** vtableLoc, int index, void* detour, void** orig)
        {
                if (!vtableLoc || !*vtableLoc)
                        return false;
                void** vtable = *vtableLoc;
                void** slot = &vtable[index];
                if (orig)
                        *orig = *slot;
                if (!Ogs::MProtectRW(slot, sizeof(void*)))
                {
                        Ogs::Log("hook: MProtectRW failed for vtable slot %d", index);
                        return false;
                }
                *slot = detour;
                return true;
        }
}

namespace OgsHook
{
        bool Native(uintptr_t addr, void* detour, void** orig)
        {
                if (!addr || !detour)
                        return false;
                unsigned long original = 0;
                int rc = owen_hook_install(static_cast<unsigned long>(addr),
                                           static_cast<unsigned long>(reinterpret_cast<uintptr_t>(detour)),
                                           &original);
                if (rc != 0)
                {
                        Ogs::Log("hook install failed @ 0x%llx rc=%d", (unsigned long long)addr, rc);
                        return false;
                }
                if (orig)
                        *orig = reinterpret_cast<void*>(static_cast<uintptr_t>(original));
                return true;
        }

        bool NativeOff(const char* key, uintptr_t rva, bool found, void* detour, void** orig)
        {
                if (!found)
                {
                        Ogs::Log("feature off (offset missing): %s", key);
                        return false;
                }
                return Native(Ogs::Resolve(rva), detour, orig);
        }

        bool ByExec(SDK::UObject* cdo, const char* className, const char* funcName, void* detour, void** orig)
        {
                if (!cdo || !cdo->Class)
                {
                        Ogs::Log("ByExec(%s.%s): null CDO/class", className, funcName);
                        return false;
                }

                SDK::UFunction* func = cdo->Class->GetFunction(className, funcName);
                if (!func)
                {
                        Ogs::Log("ByExec: UFunction %s.%s not found on CDO %s — hook skipped",
                                 className, funcName, cdo->Class->GetName().c_str());
                        return false;
                }

                auto exec = reinterpret_cast<void*>(reinterpret_cast<uintptr_t>(func->ExecFunction));
                if (!exec)
                {
                        Ogs::Log("ByExec: %s.%s has no ExecFunction", className, funcName);
                        return false;
                }

                void** vtable = *(void***)cdo;
                if (!vtable)
                {
                        Ogs::Log("ByExec: CDO %s.%s has no vtable", className, funcName);
                        return false;
                }

                for (int i = 0; i < kMaxVTableScan; ++i)
                {
                        if (vtable[i] == exec)
                                return VTableSlotPatch(reinterpret_cast<void***>(cdo), i, detour, orig);
                }

                Ogs::Log("ByExec: exec of %s.%s not present in first %d vtable slots — hook skipped",
                         className, funcName, kMaxVTableScan);
                return false;
        }

        bool ByIndex(SDK::UObject* cdo, int index, void* detour, void** orig)
        {
                if (!cdo)
                {
                        Ogs::Log("ByIndex(%d): null CDO", index);
                        return false;
                }
                if (index < 0)
                {
                        Ogs::Log("ByIndex: no vtable index configured — hook skipped");
                        return false;
                }
                return VTableSlotPatch((void***)cdo, index, detour, orig);
        }

        bool ExecSwap(const char* fullFuncPath, void* detour, void** orig)
        {
                // Find the UFunction by full name using the SDK's own helper
                // (Basic.hpp ships UObject::FindObject<T> — GObjects walk).
                SDK::UFunction* func = SDK::UObject::FindObject<SDK::UFunction>(fullFuncPath);
                if (!func)
                {
                        Ogs::Log("ExecSwap: UFunction %s not found — hook skipped", fullFuncPath);
                        return false;
                }

                if (orig)
                        *orig = reinterpret_cast<void*>(reinterpret_cast<uintptr_t>(func->ExecFunction));

                if (!Ogs::MProtectRW(&func->ExecFunction, sizeof(func->ExecFunction)))
                {
                        Ogs::Log("ExecSwap: MProtectRW failed for %s", fullFuncPath);
                        return false;
                }
                func->ExecFunction = reinterpret_cast<SDK::UFunction::FNativeFuncPtr>(detour);
                return true;
        }

        bool RestoreAll()
        {
                return owen_hook_restore_all() >= 0;
        }
} // namespace OgsHook
