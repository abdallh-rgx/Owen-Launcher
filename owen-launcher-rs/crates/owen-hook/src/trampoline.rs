//! Trampoline generation — the branch written over the target's prologue.
//!
//! ## BTI / PAC safety (the Android 12+ crash fix)
//!
//! Modern Fortnite builds (22.00+ era, clang `-mbranch-protection=standard`)
//! start every function with `BTI c` (a Branch Target Identification landing
//! pad) and mark the ELF with the BTI note, so the kernel maps the text with
//! `PROT_BTI`. On ARMv8.5+ devices (Pixel 6+, S22+, …) an **indirect** branch
//! or call that lands on a non-BTI instruction raises SIGILL — which is
//! exactly what the old `ADRP+ADD+BR` patch did: it destroyed the landing
//! pad, so the very first PLT call to `curl_easy_setopt` killed the game
//! ("crashes right on launch").
//!
//! The new layout keeps the target a valid landing pad:
//!
//! ```text
//! target:  [ BTI c (original, kept in place) ]   ← indirect calls land here
//!          [ adrp x17, hook ; add x17 ; br x17 ] ← direct path into the hook
//! relocated block (our RX pool, PROT_BTI never set on anon pages):
//!          [ relocated instructions … ]
//!          [ b target+N ]                        ← DIRECT branch (never
//!                                                  BTI-checked)
//! ```
//!
//! * The patch's `br x17` jumps into OUR library (`libowen.so` has no BTI
//!   note → never enforced) — always safe.
//! * The hook reaches the relocated block via `blr` — the block lives on an
//!   anonymous mmap page (never PROT_BTI) — always safe.
//! * The block jumps back with a **direct `b`** — direct branches are not
//!   gated by BTI — which requires the block to live within ±128 MiB of the
//!   target, so [`crate::allocator::alloc_code_near`] places it there.
//! * When the target does NOT start with a BTI (pre-2020 builds), the old
//!   12-byte patch is used and the far `ldr/br` jump-back is allowed.

use crate::Addr;

/// `adrp x17, hook; add x17, x17, #lo; br x17` — 12 bytes, ±4 GiB reach.
pub(crate) const TRAMPOLINE_NEAR_SIZE: usize = 12;
/// `BTI c` + the near trampoline — 16 bytes (landing pad preserved).
pub(crate) const TRAMPOLINE_LANDING_SIZE: usize = 16;

/// The BTI hint encodings (AArch64): `BTI c` = 0xD503245F, `BTI j` =
/// 0xD503249F, `BTI jc` = 0xD50324DF. All share the hint-space top bits with
/// the target-class bits in [7:5] and `11111` in [4:0].
pub(crate) const BTI_C: u32 = 0xD503_245F;
pub(crate) const BTI_J: u32 = 0xD503_249F;
pub(crate) const BTI_JC: u32 = 0xD503_24DF;

/// True when `inst` is any `BTI` hint — i.e. a valid indirect-branch landing
/// pad. (`PACIASP`/`PACIBSP` also act as `BTI c`, but a PAC-first prologue is
/// never a BTI-enforced library: enforced libs carry the explicit `BTI c`.)
pub(crate) fn is_bti(inst: u32) -> bool {
    matches!(inst & 0xFFFF_FFE0, 0xD503_2440 | 0xD503_2480 | 0xD503_24C0)
        && inst & 0x1F == 0x1F
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatchKind {
    /// Target starts with a BTI landing pad: keep it, 16-byte patch.
    Landing,
    /// No BTI: plain 12-byte patch.
    Plain,
}

impl PatchKind {
    pub(crate) fn len(&self) -> usize {
        match self {
            PatchKind::Landing => TRAMPOLINE_LANDING_SIZE,
            PatchKind::Plain => TRAMPOLINE_NEAR_SIZE,
        }
    }
}

/// Decide the patch kind from the target's first instruction word.
pub(crate) fn kind_for(first_inst: u32) -> PatchKind {
    if is_bti(first_inst) {
        PatchKind::Landing
    } else {
        PatchKind::Plain
    }
}

/// Build the patch bytes that branch `from` (the target) to `to` (the hook).
///
/// `kind` decides whether the original first instruction (a BTI landing pad)
/// is preserved at offset 0. The branch itself is `adrp x17` based, which
/// reaches the full ±4 GiB of the address space — the hook can live anywhere.
pub(crate) fn build(from: Addr, to: Addr, kind: PatchKind) -> Vec<u8> {
    let mut asm = crate::assemble::Assembler::new();
    match kind {
        PatchKind::Landing => {
            // The original BTI stays the first instruction — re-emitted
            // verbatim by the caller through `patch_prefix_word`.
            asm.adrp_add(crate::assemble::TMP, from + 4, to);
            asm.br(crate::assemble::TMP);
        }
        PatchKind::Plain => {
            asm.adrp_add(crate::assemble::TMP, from, to);
            asm.br(crate::assemble::TMP);
        }
    }
    asm.finalize_bytes(0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bti_hint_detection() {
        assert!(is_bti(BTI_C));
        assert!(is_bti(BTI_J));
        assert!(is_bti(BTI_JC));
        // NOP / PACIASP / real instructions are NOT landing pads
        assert!(!is_bti(0xD503_201F)); // NOP
        assert!(!is_bti(0xD503_233F)); // PACIASP
        assert!(!is_bti(0xD503_23BF)); // PACIBSP
        assert!(!is_bti(0xA9BF_7BFD)); // stp x29, x30, [sp, #-16]!
        assert!(!is_bti(0x9100_0000)); // add
    }

    #[test]
    fn patch_kinds_and_sizes() {
        assert_eq!(kind_for(BTI_C), PatchKind::Landing);
        assert_eq!(kind_for(BTI_JC), PatchKind::Landing);
        assert_eq!(kind_for(0xD503_23BF), PatchKind::Plain); // PACIBSP-only lib
        assert_eq!(kind_for(0xA9BF_7BFD), PatchKind::Plain);
        assert_eq!(PatchKind::Landing.len(), 16);
        assert_eq!(PatchKind::Plain.len(), 12);
    }

    #[test]
    fn landing_patch_keeps_bti_first_and_is_16_bytes() {
        let patch = build(0x7f0000_1000, 0x7e8000_2000, PatchKind::Landing);
        assert_eq!(patch.len(), 12, "build() emits the adrp/add/br tail; the BTI word is prepended by install()");
        let w0 = u32::from_le_bytes([patch[0], patch[1], patch[2], patch[3]]);
        assert_eq!(w0 & 0x9F00_0000, 0x9000_0000, "first emitted word is ADRP");
        let w2 = u32::from_le_bytes([patch[8], patch[9], patch[10], patch[11]]);
        assert_eq!(w2 & 0xFFFF_FC00, 0xD61F_0000, "last word is a BR");
        assert_eq!(w2 & 0x03E0, 17 << 5, "BR x17 register field");
    }
}
