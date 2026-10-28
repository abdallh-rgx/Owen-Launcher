//! Trampoline generation — the branch written over the target's prologue.

use crate::Addr;

/// `adrp x17, hook; add x17, x17, #lo; br x17` — 12 bytes, usable when the
/// hook lives within +/-4 GB of the target (always true inside one process).
pub(crate) const TRAMPOLINE_NEAR_SIZE: usize = 12;
/// `ldr x17, [pc, #8]; br x17; .quad hook` — 16 bytes.
pub(crate) const TRAMPOLINE_FAR_SIZE: usize = 16;

/// Build the patch bytes that branch `from` to `to`.
///
/// The original engine prefers the ADRP+ADD+BR form when the distance fits
/// in the ADRP range; otherwise it uses a literal pool load. We allocate the
/// replacement function inside our own RX pools, which are always within
/// +/-4 GB on Android (single address space), so the near form is the norm.
pub(crate) fn build(from: Addr, to: Addr) -> Vec<u8> {
    // ADRP immediate range: +/-4 GB (2^33 bytes), signed.
    let delta = (to as i64) - (from as i64);
    if delta.unsigned_abs() < (1u64 << 33) {
        let mut asm = crate::assemble::Assembler::new();
        asm.adrp_add(crate::assemble::TMP, from, to);
        asm.br(crate::assemble::TMP);
        asm.finalize_bytes()
    } else {
        let mut asm = crate::assemble::Assembler::new();
        asm.literal_ldr_branch(to);
        asm.finalize_bytes()
    }
}
