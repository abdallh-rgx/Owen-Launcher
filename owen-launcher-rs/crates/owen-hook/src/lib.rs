//! owen-hook — AArch64 inline hooking engine for Android, ported to Rust from
//! the battle-tested Dobby core embedded in the original `Owen.c`.
//!
//! Safety model
//! ============
//! Owen Launcher's policy is "100% safe Rust for 100% of the project logic".
//! Raw memory interop cannot be expressed in safe Rust, so it is isolated in
//! exactly two audited cores (`owen-hook` and `owen-art`). This crate keeps
//! every `unsafe` block small, documented and reviewed, and exposes a safe
//! API:
//!
//! ```ignore
//! let original = owen_hook::install(target_addr, replacement_addr)?;
//! // later, from inside the replacement:
//! let code = unsafe { original.call::<unsafe extern "C" fn(_, _, _) -> _>(a, b, c) };
//! ```
//!
//! What it does (identical to the original engine):
//! 1. Allocate a trampoline near (or far from) the target.
//! 2. Relocate the first N instructions of the target into an executable
//!    block, rewriting every PC-relative instruction (B/BL, LDR-literal,
//!    ADR/ADRP, B.cond, CBZ/CBZ, TBZ/TBZ) to be position-independent.
//! 3. Patch the target with a branch to the replacement.
//! 4. Hand out a callable pointer to (relocated prologue + jump back).
//!
//! The engine is deliberately dependency-free: the handful of libc entry
//! points (mmap/mprotect/munmap/sysconf) are declared directly.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

mod allocator;
mod assemble;
mod capi;
mod hook;
mod relocate;
mod trampoline;

pub use capi::{
    owen_hook_install, owen_hook_restore_all, owen_hook_trampoline,
};
pub use hook::{install, restore_all, Installed, OriginalFn};

use core::fmt;

/// Errors produced by the hook engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookError {
    NullTarget,
    NullReplacement,
    AlreadyHooked,
    TooManyHooks,
    TrampolineAllocFailed,
    RelocationFailed,
    PatchFailed,
}

impl fmt::Display for HookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            HookError::NullTarget => "target address is null",
            HookError::NullReplacement => "replacement address is null",
            HookError::AlreadyHooked => "target is already hooked",
            HookError::TooManyHooks => "hook table full",
            HookError::TrampolineAllocFailed => "trampoline allocation failed",
            HookError::RelocationFailed => "instruction relocation failed",
            HookError::PatchFailed => "code patch failed",
        };
        f.write_str(s)
    }
}

impl std::error::Error for HookError {}

pub(crate) type Addr = usize;
