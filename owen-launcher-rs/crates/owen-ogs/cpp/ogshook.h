// =============================================================================
// ogshook.h — hook facade for the OGS Android port.
//
// The Windows original used MinHook (MH_CreateHook + MH_EnableHook) and two
// ad-hoc vtable patchers (HookVTable / ExecHook).  On Android every hook goes
// through the audited Rust `owen-hook` engine, exposed as a plain C ABI:
//
//   extern "C" int  owen_hook_install(unsigned long target,
//                                     unsigned long replacement,
//                                     unsigned long* out_original); // 0 = ok
//   extern "C" int  owen_hook_restore_all(void);
//
// (declared in crates/owen-hook/src/capi.rs; libOwenOGS.so links owen-hook
// statically so the symbols resolve at the final link.)
//
// Facade API used by the ported mod:
//   OgsHook::Native(addr, detour, orig)
//       inline hook at an absolute address.  NOTE: unlike MinHook there is
//       no separate "enable" step — owen_hook_install installs ACTIVE
//       immediately (the original's MH_EnableHook(MH_ALL_HOOKS) step is
//       dropped).
//   OgsHook::NativeOff(key, detour, orig)
//       Native(Ogs::Off().<key> resolved via Base()+rva); missing offset →
//       logs "feature off (offset missing)" and returns false.  Use the
//       OGS_HOOK_NATIVE_OFF(key, Detour, OrigVar) macro so every raw hook
//       is fail-safe by construction.
//   OgsHook::ByExec(cdo, className, funcName, detour, orig)
//       replaces the original HookVTable: resolves the reflected UFunction
//       via cdo->Class->GetFunction(className, funcName), takes its
//       ExecFunction pointer and scans the CDO's vtable (up to 2048 slots)
//       for the slot holding that exec thunk; the slot is then patched
//       (MProtectRW + write).  Not found → log + false, NEVER crash.
//   OgsHook::ByIndex(cdo, index, detour, orig)
//       direct vtable slot patch for non-reflected virtuals; the index comes
//       from the config "vt_indices" map by name.
//   OgsHook::ExecSwap(fullFuncPath, detour, orig)
//       replaces the original ExecHook: finds the UFunction by full name
//       (SDK::UObject::FindObject) and overwrites its ExecFunction member
//       (MProtectRW first).
//   OgsHook::RestoreAll()
//       best-effort unhook of everything (owen_hook_restore_all).
// =============================================================================

#pragma once

#include "OgsCompat.h"

// --- C ABI of the Rust hook engine (crates/owen-hook/src/capi.rs) -----------
extern "C" int owen_hook_install(unsigned long target, unsigned long replacement, unsigned long* out_original);
extern "C" int owen_hook_restore_all(void);

namespace OgsHook
{
	/// Inline hook at an absolute address; installs ACTIVE immediately.
	bool Native(uintptr_t addr, void* detour, void** orig);

	/// Inline hook via the offset registry; missing offset → log + false.
	bool NativeOff(const char* key, uintptr_t rva, bool found, void* detour, void** orig);

	/// Vtable hook by reflected function name (the HookVTable replacement).
	bool ByExec(SDK::UObject* cdo, const char* className, const char* funcName, void* detour, void** orig);

	/// Direct vtable slot patch (non-reflected virtuals).
	bool ByIndex(SDK::UObject* cdo, int index, void* detour, void** orig);

	/// Overwrite a UFunction's ExecFunction (the ExecHook replacement).
	bool ExecSwap(const char* fullFuncPath, void* detour, void** orig);

	/// Best-effort unhook of everything.
	bool RestoreAll();
}

/// Fail-safe raw hook: every MH_CreateHook(ImageBase+0x..., Detour, &OG)
/// became OGS_HOOK_NATIVE_OFF(key, Detour, OGVar).  Evaluates the registry
/// entry once so the log names the feature, not just the key.
#define OGS_HOOK_NATIVE_OFF(RegEntry, Detour, OrigVar)                            \
	do                                                                             \
	{                                                                              \
		const ::Ogs::OffEntry& _ogs_he_ = (RegEntry);                              \
		if (!_ogs_he_.Found)                                                       \
			::Ogs::Log("feature off (offset missing): %s", #RegEntry);             \
		else                                                                       \
			::OgsHook::Native(::Ogs::Resolve(_ogs_he_.Rva), (Detour), (OrigVar));  \
	} while (0)
