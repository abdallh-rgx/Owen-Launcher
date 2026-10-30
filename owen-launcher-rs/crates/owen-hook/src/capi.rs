//! C ABI surface of the hook engine — the seam the OGS in-game mod
//! (`libOwenOGS.so`, C++) uses to install AArch64 inline hooks without
//! duplicating a second hooking engine in C++.
//!
//! Contract (mirrors `hook::install`):
//! * `owen_hook_install(target, replacement, &out_original)` hooks the native
//!   function at `target`; every call lands in `replacement`. On success it
//!   stores the relocated-original address in `*out_original` (call it from
//!   the detour with the hooked function's own signature) and returns `0`.
//! * Error codes: `-1` null target, `-2` null replacement, `-3` already
//!   hooked, `-4` hook table full, `-5` trampoline allocation failed,
//!   `-6` relocation failed, `-7` code patch failed, `-8` bad out pointer.
//! * `owen_hook_restore_all()` un-hooks everything (best effort) and returns
//!   the number of restored hooks.
//! * `owen_hook_trampoline(target)` — pure query: returns the address the
//!   engine *would* hand out for a hook at `target` without installing
//!   anything. Currently always 0 (reserved); C++ side must not rely on it.
//!
//! The C++ declaration (shipped in `crates/owen-ogs/cpp/ogshook.h`):
//! ```c
//! extern "C" int  owen_hook_install(unsigned long target,
//!                                    unsigned long replacement,
//!                                    unsigned long* out_original);
//! extern "C" int  owen_hook_restore_all(void);
//! ```

use crate::hook;
use crate::HookError;

/// Map a [`HookError`] onto the stable C error codes documented above.
fn code(err: HookError) -> i32 {
    match err {
        HookError::NullTarget => -1,
        HookError::NullReplacement => -2,
        HookError::AlreadyHooked => -3,
        HookError::TooManyHooks => -4,
        HookError::TrampolineAllocFailed => -5,
        HookError::RelocationFailed => -6,
        HookError::PatchFailed => -7,
    }
}

/// Install an inline hook from C. See the module docs for the contract.
#[no_mangle]
pub extern "C" fn owen_hook_install(
    target: usize,
    replacement: usize,
    out_original: *mut usize,
) -> i32 {
    if out_original.is_null() {
        return -8;
    }
    match hook::install(target, replacement) {
        Ok(installed) => {
            unsafe { *out_original = installed.original.as_addr() };
            0
        }
        Err(e) => code(e),
    }
}

/// Best-effort removal of every hook installed by this process.
#[no_mangle]
pub extern "C" fn owen_hook_restore_all() -> i32 {
    // usize hook-count → i32 C return (saturating; counts are tiny).
    hook::restore_all().min(i32::MAX as usize) as i32
}

/// Reserved no-op query (see module docs). Always returns 0.
#[no_mangle]
pub extern "C" fn owen_hook_trampoline(_target: usize) -> usize {
    0
}
