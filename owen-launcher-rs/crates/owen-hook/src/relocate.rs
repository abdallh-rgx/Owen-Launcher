//! Instruction rewriter — makes the first N instructions of the hook target
//! position-independent so they can be copied into the "relocated" block.
//!
//! ## BTI-safe branch re-encoding (the Android 12+ crash fix)
//!
//! The relocated block is entered with `blr` from the hook and left with a
//! **direct `b`** back into the original function. To keep the block itself
//! free of **indirect** branches (which would need BTI landing pads at their
//! destinations — mid-function targets have none, and BTI-enforced libraries
//! would SIGILL), every PC-relative branch is re-encoded as the SAME direct
//! branch form with a recomputed displacement:
//!
//! * `b` / `bl`       -> `b` / `bl` to the same absolute destination (±128 MiB)
//! * `b.cond`         -> `b.cond` to the same destination (±1 MiB)
//! * `cbz/cbnz`       -> `cbz/cbnz` recomputed (±1 MiB)
//! * `tbz/tbnz`       -> `tbz/tbnz` recomputed (±32 KiB)
//! * `ldr xN, [pc,#]` -> `movz/movk x17, =dst; ldr xN, [x17]`
//! * `adr` / `adrp`   -> `movz/movk xD, =dst` (page-absolute for adrp)
//!
//! A displacement that no longer fits its immediate field is a hard
//! relocation error — the hook is then **refused** (`HookError::
//! RelocationFailed`), never installed as a broken trampoline. Graceful
//! degradation beats crashing the game.

use crate::assemble::Assembler;
use crate::Addr;

const UNCONDITIONAL_BRANCH_FIXED_MASK: u32 = 0x7C00_0000;
const UNCONDITIONAL_BRANCH_FIXED: u32 = 0x1400_0000;
const UNCONDITIONAL_BRANCH_MASK: u32 = 0xFC00_0000;
const BL_INST: u32 = 0x9400_0000;

const LOAD_REG_LITERAL_FIXED_MASK: u32 = 0x3B00_0000;
const LOAD_REG_LITERAL_FIXED: u32 = 0x1800_0000;
const LOAD_REG_LITERAL_MASK: u32 = 0xFF00_0000;

const PC_REL_ADDRESSING_FIXED_MASK: u32 = 0x1F00_0000;
const PC_REL_ADDRESSING_FIXED: u32 = 0x1000_0000;
const PC_REL_ADDRESSING_MASK: u32 = 0x9F00_0000;
const ADR: u32 = 0x1000_0000;
const ADRP: u32 = 0x9000_0000;

const COND_BRANCH_FIXED_MASK: u32 = 0xFE00_0000;
const COND_BRANCH_FIXED: u32 = 0x5400_0000;

const COMPARE_BRANCH_FIXED_MASK: u32 = 0x7E00_0000;
const COMPARE_BRANCH_FIXED: u32 = 0x3400_0000;

const TEST_BRANCH_FIXED_MASK: u32 = 0x7E00_0000;
const TEST_BRANCH_FIXED: u32 = 0x3600_0000;

#[inline]
fn sign_extend(x: u32, bits: u32) -> i64 {
    let sign = 1u32 << (bits - 1);
    ((x ^ sign) as i64) - (sign as i64)
}

/// Bitfield extract for `obj[st..=en]`. The mask covers EXACTLY the field
/// width — a wider mask leaks the opcode bits above `en` into every
/// displacement decode (the historical bug: `bits(i,5,23)` returned 24 bits
/// for the 19-bit imm19 field, so `b.cond`/`cbz`/`adrp` relocations jumped
/// to wildly wrong addresses).
fn bits(obj: u32, st: u32, en: u32) -> u32 {
    let width = en.saturating_sub(st) + 1;
    let mask = if width >= 32 {
        u32::MAX
    } else {
        (1u32 << width) - 1
    };
    (obj >> st) & mask
}

#[inline]
fn inst_is_b_bl(i: u32) -> bool {
    (i & UNCONDITIONAL_BRANCH_FIXED_MASK) == UNCONDITIONAL_BRANCH_FIXED
}
#[inline]
fn inst_is_ldr_literal(i: u32) -> bool {
    (i & LOAD_REG_LITERAL_MASK) == LOAD_REG_LITERAL_FIXED
        && (i & LOAD_REG_LITERAL_FIXED_MASK) == LOAD_REG_LITERAL_FIXED
}
#[inline]
fn inst_is_adr(i: u32) -> bool {
    (i & PC_REL_ADDRESSING_FIXED_MASK) == PC_REL_ADDRESSING_FIXED
        && (i & PC_REL_ADDRESSING_MASK) == ADR
}
#[inline]
fn inst_is_adrp(i: u32) -> bool {
    (i & PC_REL_ADDRESSING_FIXED_MASK) == PC_REL_ADDRESSING_FIXED
        && (i & PC_REL_ADDRESSING_MASK) == ADRP
}
#[inline]
fn inst_is_b_cond(i: u32) -> bool {
    (i & COND_BRANCH_FIXED_MASK) == COND_BRANCH_FIXED
}
#[inline]
fn inst_is_compare_b(i: u32) -> bool {
    (i & COMPARE_BRANCH_FIXED_MASK) == COMPARE_BRANCH_FIXED
}
#[inline]
fn inst_is_test_b(i: u32) -> bool {
    (i & TEST_BRANCH_FIXED_MASK) == TEST_BRANCH_FIXED
}

fn decode_imm19(i: u32) -> i64 {
    sign_extend(bits(i, 5, 23) << 2, 2 + 19)
}
fn decode_imm14(i: u32) -> i64 {
    sign_extend(bits(i, 5, 18) << 2, 2 + 14)
}
fn decode_imm26(i: u32) -> i64 {
    sign_extend(bits(i, 0, 25) << 2, 2 + 26)
}
fn decode_immhi_immlo(i: u32) -> i64 {
    let immlo = bits(i, 29, 30) as i64;
    let immhi = bits(i, 5, 23) as i64;
    sign_extend((immlo | (immhi << 2)) as u32, 2 + 19)
}

/// How the relocated block returns to the original function.
#[derive(Debug, Clone, Copy)]
pub(crate) enum BackRef {
    /// `b back` — a DIRECT branch (never BTI-gated; needs the block near
    /// the target; validated when `finalize_bytes` gets the real base).
    Near(Addr),
    /// `ldr x17, =back; br x17` — indirect, legacy (non-BTI) libraries only.
    Far(Addr),
}

/// Reads `len` bytes (a multiple of 4) starting at `origin` and appends
/// relocated equivalents to `as_`. Direct-branch displacements are recorded
/// absolute and patched in `Assembler::finalize_bytes` once the block's
/// final address is known. Returns the number of source bytes consumed.
///
/// SAFETY (caller): `origin..origin+len` must be readable executable memory.
pub(crate) unsafe fn relocate(
    as_: &mut Assembler,
    origin: Addr,
    len: usize,
    branch_back: Option<BackRef>,
) -> Result<usize, ()> {
    let mut offset = 0usize;
    while offset < len {
        // SAFETY: caller guaranteed readability; 4-byte aligned reads.
        let inst = unsafe { core::ptr::read_unaligned((origin + offset) as *const u32) };
        let pc = origin + offset;

        if inst_is_b_bl(inst) {
            let dst = (pc as i64 + decode_imm26(inst)) as u64;
            let is_bl = (inst & UNCONDITIONAL_BRANCH_MASK) == BL_INST;
            // Re-encode as the SAME direct branch form, displacement
            // recomputed for the block — patched once the base is final.
            as_.direct_branch(is_bl, dst, 26);
        } else if inst_is_ldr_literal(inst) {
            let dst = (pc as i64 + decode_imm19(inst)) as u64;
            let rt = bits(inst, 0, 4);
            let opc = bits(inst, 30, 31);
            as_.mov_imm64(crate::assemble::TMP, dst);
            if opc == 0b00 {
                as_.ldr_unsigned_w(rt, crate::assemble::TMP, 0);
            } else {
                as_.ldr_unsigned_x(rt, crate::assemble::TMP, 0);
            }
        } else if inst_is_adr(inst) {
            let dst = (pc as i64 + decode_immhi_immlo(inst)) as u64;
            let rd = bits(inst, 0, 4);
            as_.mov_imm64(rd, dst);
        } else if inst_is_adrp(inst) {
            let dst = (pc as i64 + (decode_immhi_immlo(inst) << 12)) as u64;
            let rd = bits(inst, 0, 4);
            as_.mov_imm64(rd, dst & !0xFFF);
        } else if inst_is_b_cond(inst) {
            let dst = (pc as i64 + decode_imm19(inst)) as u64;
            as_.direct_cond_branch(inst, dst);
        } else if inst_is_compare_b(inst) {
            let dst = (pc as i64 + decode_imm19(inst)) as u64;
            as_.direct_compare_branch(inst, dst);
        } else if inst_is_test_b(inst) {
            let dst = (pc as i64 + decode_imm14(inst)) as u64;
            as_.direct_test_branch(inst, dst);
        } else {
            // Everything else (including BTI/PAC hints, stp, mov, …) is
            // copied verbatim — register semantics are preserved.
            as_.push_word(inst);
        }
        offset += 4;
    }

    match branch_back {
        Some(BackRef::Near(back)) => as_.direct_branch(false, back as u64, 26),
        Some(BackRef::Far(back)) => as_.literal_ldr_branch(back),
        None => {}
    }
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trampoline;

    #[test]
    fn bti_and_pac_hints_are_verbatim() {
        // A BTI-c-first prologue (BTI c + PACIBSP + stp) must relocate to
        // exactly those three words — no re-encoding, no clobbering.
        let mut asm = Assembler::new();
        let words: [u32; 3] = [trampoline::BTI_C, 0xD503_23BF, 0xA9BF_7BFD];
        let mut buf = Vec::new();
        for w in words {
            buf.extend_from_slice(&w.to_le_bytes());
        }
        // SAFETY: host test buffer, readable for the length passed.
        unsafe {
            relocate(
                &mut asm,
                buf.as_ptr() as Addr,
                12,
                Some(BackRef::Far(0x2000)),
            )
        }
        .unwrap();
        let bytes = asm.finalize_bytes(0x1000).unwrap();
        assert_eq!(bytes.len() % 4, 0);
        let got: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(got[0], trampoline::BTI_C, "BTI kept verbatim");
        assert_eq!(got[1], 0xD503_23BF, "PACIBSP kept verbatim");
        assert_eq!(got[2], 0xA9BF_7BFD, "STP kept verbatim");
        // the far jump-back: ldr x17, [pc,#8]; br x17; .quad 0x2000
        assert_eq!(got[3] & 0xFF00_0000, 0x5800_0000, "ldr x17 literal");
        assert_eq!(got[4] & 0xFFFF_FC00, 0xD61F_0000, "br opcode");
        assert_eq!(got[4] & 0x03E0, 17 << 5, "br x17 register field");
    }

    #[test]
    fn direct_b_displacement_is_recomputed_for_the_block_base() {
        let mut asm = Assembler::new();
        // a direct b at block offset 0 to absolute 0x10010 (block base 0x10000)
        asm.direct_branch(false, 0x10010, 26);
        let bytes = asm.finalize_bytes(0x10000).unwrap();
        let w0 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(w0 & 0xFC00_0000, 0x1400_0000, "still a direct b");
        assert_eq!(w0 & 0x03FF_FFFF, 0x10 >> 2, "displacement = dst - (base+0)");
    }

    #[test]
    fn out_of_range_direct_b_is_a_hard_error() {
        let mut asm = Assembler::new();
        // 256 MiB away — beyond the ±128 MiB imm26 reach
        asm.direct_branch(false, 0x1000_0000 + 0x1000_0000, 26);
        assert!(asm.finalize_bytes(0x1000).is_err());
    }

    #[test]
    fn cond_branch_kinds_survive_reencoding() {
        // b.eq +0x20
        let mut asm = Assembler::new();
        let beq = 0x5400_0000u32 | (((0x20 >> 2) as u32) << 5) | 0; // cond=eq(0)
        let mut buf = Vec::new();
        buf.extend_from_slice(&beq.to_le_bytes());
        // SAFETY: host test buffer.
        let origin = buf.as_ptr() as Addr;
        unsafe {
            relocate(
                &mut asm,
                origin,
                4,
                None,
            )
        }
        .unwrap();
        // finalize with the block placed at the same address as the buffer so
        // the displacement is preserved
        let base = buf.as_ptr() as Addr;
        let bytes = asm.finalize_bytes(base).unwrap();
        let w0 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(w0 & 0xFF00_0000, 0x5400_0000, "still b.cond");
        assert_eq!(w0 & 0xF, 0, "cond preserved");
        assert_eq!(
            (w0 >> 5) & 0x7FFFF,
            0x20 >> 2,
            "same displacement (block at the origin)"
        );
    }
}
