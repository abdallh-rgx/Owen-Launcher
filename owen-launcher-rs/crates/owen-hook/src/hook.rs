//! Hook installation — the `DobbyHook` equivalent, BTI/PAC-safe.
//!
//! Layout produced for `install(target, replacement)`:
//!
//! ```text
//! target (BTI lib):   [ BTI c (original, kept) ]     ← indirect calls land
//!                     [ adrp x17, hook; add; br x17 ]  (12 bytes)
//! target (legacy lib):[ adrp x17, hook; add; br x17 ]  (12 bytes)
//! relocated block:    [ relocated first N instructions of the target ]
//!                     [ b target+N ]                   ← DIRECT branch —
//!                                                      never BTI-gated
//! ```
//!
//! Why the dance (the "game crashes right on launch" fix): Fortnite builds
//! from ~2021 on are compiled with `-mbranch-protection=standard` — every
//! function starts with `BTI c` and the ELF carries the BTI note, so
//! ARMv8.5+ kernels map the text with `PROT_BTI`. An indirect branch onto a
//! non-BTI instruction (what the old plain trampoline produced, both at the
//! target and at the jump-back) raises SIGILL. The new design:
//!
//! 1. the target keeps a valid landing pad (its own `BTI c` stays word 0),
//! 2. the patch jumps into **our** library (`br x17` — libowen.so has no
//!    BTI note, so its pages are never PROT_BTI),
//! 3. the hook reaches the relocated block with `blr` (anonymous mmap page —
//!    never PROT_BTI),
//! 4. the block returns into the original function with a **direct `b`**
//!    (direct branches are not BTI-gated), which is why the block is
//!    allocated **near** the target ([`crate::allocator::alloc_code_near`]).
//!
//! When the block cannot be placed within direct-branch reach and the target
//! carries a BTI landing pad, the hook is **refused**
//! (`HookError::TrampolineAllocFailed`) — a dead redirecter is recoverable,
//! a SIGILL in the game is not.

use crate::allocator::{alloc_code, alloc_code_near, write_code};
use crate::assemble::Assembler;
use crate::relocate;
use crate::trampoline::{self, PatchKind};
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

/// The relocated-block build (the sizing pass and the final pass share it).
enum JumpBack {
    /// `b target+N` — direct, BTI-safe, requires the near allocator.
    Near,
    /// `ldr x17,=target+N; br x17` — indirect, legacy libraries only.
    Far,
}

fn build_block(
    target: Addr,
    kind: PatchKind,
    back: Addr,
    jump: JumpBack,
) -> Result<Assembler, HookError> {
    let reloc_origin = target + if matches!(kind, PatchKind::Landing) { 4 } else { 0 };
    let reloc_len = trampoline::TRAMPOLINE_NEAR_SIZE; // always the 3 words after the pad
    let mut asm = Assembler::new();
    let back_ref = match jump {
        JumpBack::Near => relocate::BackRef::Near(back),
        JumpBack::Far => relocate::BackRef::Far(back),
    };
    // SAFETY: the target is a mapped, readable code address in this process
    // (validated by the caller — the redirecter only passes addresses it
    // derived from dlsym or the signature scanner, both of which point at
    // real executable mappings), and we read exactly 12 bytes of it.
    unsafe {
        relocate::relocate(&mut asm, reloc_origin, reloc_len, Some(back_ref))
            .map_err(|_| HookError::RelocationFailed)?;
    }
    Ok(asm)
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

    // ---- 1. Read the target prologue and classify it (BTI or legacy) -----
    let mut first = [0u8; 16];
    // SAFETY: reading 16 bytes at a mapped executable target (validated by
    // the caller — dlsym or the signature scanner).
    unsafe {
        core::ptr::copy_nonoverlapping(target as *const u8, first.as_mut_ptr(), 16);
    }
    let inst0 = u32::from_le_bytes([first[0], first[1], first[2], first[3]]);
    let kind = trampoline::kind_for(inst0);
    let patch_len = kind.len();
    let back = target + patch_len;

    // ---- 2. Build the patch: [landing pad] + adrp/add/br into the hook ---
    let mut patch = Vec::with_capacity(patch_len);
    if matches!(kind, PatchKind::Landing) {
        patch.extend_from_slice(&inst0.to_le_bytes());
    }
    patch.extend(trampoline::build(target, replacement, kind));
    if patch.len() != patch_len {
        return Err(HookError::TrampolineAllocFailed);
    }

    // ---- 3. Relocated block, near the target (BTI-safe jump-back) --------
    let near = build_block(target, kind, back, JumpBack::Near)?;
    let size = near.size_hint();
    let mut asm = near;
    let placed = if let Some(block) = alloc_code_near(size, target) {
        Some((block, JumpBack::Near))
    } else if matches!(kind, PatchKind::Plain) {
        // Legacy library (no BTI note → no PROT_BTI): the indirect far
        // jump-back is safe anywhere.
        let far = build_block(target, kind, back, JumpBack::Far)?;
        let size = far.size_hint();
        asm = far;
        alloc_code(size).map(|block| (block, JumpBack::Far))
    } else {
        None // BTI library + no near page: refuse, never crash the game
    };
    let Some((block, _jump)) = placed else {
        return Err(HookError::TrampolineAllocFailed);
    };
    let relocated_bytes = asm
        .finalize_bytes(block)
        .map_err(|_| HookError::RelocationFailed)?;
    if relocated_bytes.is_empty() {
        return Err(HookError::RelocationFailed);
    }
    let relocated_len = relocated_bytes.len();
    // SAFETY: `block` is a page from our own RX pool.
    if !unsafe { write_code(block, &relocated_bytes) } {
        return Err(HookError::PatchFailed);
    }

    // ---- 4. Save the original bytes and patch the target -----------------
    let mut original = [0u8; 16];
    // SAFETY: reading `patch_len <= 16` bytes at the mapped target.
    unsafe {
        core::ptr::copy_nonoverlapping(target as *const u8, original.as_mut_ptr(), patch_len);
    }
    // SAFETY: the target is an executable mapping we are deliberately
    // patching — that is the entire purpose of this crate.
    if !unsafe { write_code(target, &patch) } {
        return Err(HookError::PatchFailed);
    }

    entries.push(Entry {
        target,
        replacement,
        relocated: block,
        relocated_len,
        original,
        original_len: patch_len,
    });

    Ok(Installed {
        original: OriginalFn { addr: block },
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
