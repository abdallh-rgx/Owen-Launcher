//! Hook installation — the `DobbyHook` equivalent.
//!
//! Layout produced for `install(target, replacement)`:
//!
//! ```text
//! target:          [ trampoline -> replacement ]        (patched in place)
//! relocated block: [ relocated first N bytes of target ]
//!                  [ ldr x17, =target+N; br x17 ]       (jump back)
//! ```
//!
//! The relocated block is what `OriginalFn` points at, so calling the
//! original executes the untouched semantics of the patched prologue and
//! then continues in the original function.

use crate::allocator::{alloc_code, write_code};
use crate::assemble::Assembler;
use crate::trampoline;
use crate::{Addr, HookError};
use std::sync::Mutex;

const MAX_HOOKS: usize = 256;

struct Entry {
    target: Addr,
    replacement: Addr,
    relocated: Addr,
    relocated_len: usize,
    /// Original prologue bytes, kept so `restore_all` can undo everything.
    original: [u8; 16],
    original_len: usize,
}

static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

/// A callable pointer to the relocated original function.
#[derive(Clone, Copy)]
pub struct OriginalFn {
    addr: Addr,
}

impl OriginalFn {
    /// Call the original function through its relocated prologue.
    ///
    /// SAFETY: the caller must pass a signature compatible with the hooked
    /// function, including the C ABI and variadic expectations.
    #[inline(always)]
    pub unsafe fn call<F: Copy>(&self, f: F) -> F {
        f
    }

    /// Raw address of the relocated original (for manual transmutes).
    #[inline(always)]
    pub fn as_addr(&self) -> Addr {
        self.addr
    }
}

/// Result of a successful hook installation.
pub struct Installed {
    pub original: OriginalFn,
}

/// Install an inline hook: every call to `target` lands in `replacement`.
///
/// The function is safe to call from any thread; internally it serialises on
/// the hook table. The only `unsafe` work happens on raw code pages owned by
/// this crate or at the hook target itself.
pub fn install(target: Addr, replacement: Addr) -> Result<Installed, HookError> {
    if target == 0 {
        return Err(HookError::NullTarget);
    }
    if replacement == 0 {
        return Err(HookError::NullReplacement);
    }

    let mut entries = ENTRIES.lock().map_err(|_| HookError::PatchFailed)?;
    if entries.iter().any(|e| e.target == target) {
        return Err(HookError::AlreadyHooked);
    }
    if entries.len() >= MAX_HOOKS {
        return Err(HookError::TooManyHooks);
    }

    // 1. Build the trampoline that will be written over the target.
    let patch = trampoline::build(target, replacement);
    if patch.is_empty() || patch.len() > 16 {
        return Err(HookError::TrampolineAllocFailed);
    }

    // 2. Relocate the first `patch.len()` bytes of the target into a fresh
    //    executable block that ends with a jump back to target+N.
    let mut asm = Assembler::new();
    // SAFETY: the target is a mapped, readable code address in this process
    // (validated by the caller — the redirecter only passes addresses it
    // derived from dlsym or the signature scanner, both of which point at
    // real executable mappings).
    let consumed = unsafe {
        crate::relocate::relocate(&mut asm, target, patch.len(), Some(target + patch.len()))
            .map_err(|_| HookError::RelocationFailed)?
    };
    let relocated_bytes = asm.finalize_bytes();
    if relocated_bytes.is_empty() {
        return Err(HookError::RelocationFailed);
    }
    let relocated = alloc_code(relocated_bytes.len()).ok_or(HookError::TrampolineAllocFailed)?;
    // SAFETY: `relocated` is a page from our own RX pool.
    unsafe { write_code(relocated, &relocated_bytes) }.then_some(()).ok_or(HookError::PatchFailed)?;

    // 3. Save the original bytes and patch the target.
    let mut original = [0u8; 16];
    // SAFETY: reading `patch.len() <= 16` bytes at the mapped target.
    unsafe {
        core::ptr::copy_nonoverlapping(
            target as *const u8,
            original.as_mut_ptr(),
            patch.len(),
        );
    }
    // SAFETY: the target is an executable mapping we are deliberately
    // patching — that is the entire purpose of this crate.
    let ok = unsafe { write_code(target, &patch) };
    if !ok {
        return Err(HookError::PatchFailed);
    }

    entries.push(Entry {
        target,
        replacement,
        relocated,
        relocated_len: relocated_bytes.len(),
        original,
        original_len: patch.len(),
    });
    let _ = consumed;

    Ok(Installed {
        original: OriginalFn { addr: relocated },
    })
}

/// Undo every installed hook (best effort; used by diagnostics).
pub fn restore_all() -> usize {
    let mut restored = 0usize;
    if let Ok(mut entries) = ENTRIES.lock() {
        for e in entries.drain(..) {
            // SAFETY: restoring bytes we saved from the same address.
            unsafe {
                if write_code(e.target, &e.original[..e.original_len]) {
                    restored += 1;
                }
            }
            let _ = (e.relocated, e.relocated_len);
        }
    }
    restored
}
