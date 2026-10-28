//! Instruction rewriter — makes the first N instructions of the hook target
//! position-independent so they can be copied into the "relocated" block.
//!
//! Faithful port of the `relocate()` pass in the original `Owen.c`:
//!
//! * `b` / `bl`            -> `ldr x17, =target; br x17` / `...; blr x17`
//! * `ldr xN, [pc, #imm]`  -> `mov x17, #dst; ldr xN, [x17]` (w or x by opc)
//! * `adr` / `adrp`        -> `mov xD, #dst`
//! * `b.cond` / `cbz/cbnz` / `tbz/tbnz`
//!                         -> inverted branch over a `ldr x17,=dst; br x17`
//!
//! Everything else is copied verbatim.

use crate::assemble::Assembler;
use crate::Addr;

const UNCONDITIONAL_BRANCH_FIXED_MASK: u32 = 0x7C00_0000;
const UNCONDITIONAL_BRANCH_FIXED: u32 = 0x1400_0000;
const UNCONDITIONAL_BRANCH_MASK: u32 = 0xFC00_0000;
const B_INST: u32 = 0x1400_0000;
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

fn bits(obj: u32, st: u32, en: u32) -> u32 {
    let mask = if en >= 31 {
        u32::MAX
    } else {
        (1u32 << (en + 1)) - 1
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

/// Reads `len` instructions starting at `origin` and appends relocated
/// equivalents to `as_`. Returns the number of source bytes consumed.
///
/// SAFETY (caller): `origin..origin+len` must be readable executable memory.
pub(crate) unsafe fn relocate(
    as_: &mut Assembler,
    origin: Addr,
    len: usize,
    branch_back_to: Option<Addr>,
) -> Result<usize, ()> {
    let mut offset = 0usize;
    while offset < len {
        // SAFETY: caller guaranteed readability; 4-byte aligned reads.
        let inst = unsafe { core::ptr::read_unaligned((origin + offset) as *const u32) };
        let pc = origin + offset;

        if inst_is_b_bl(inst) {
            let dst = (pc as i64 + decode_imm26(inst)) as u64;
            let is_bl = (inst & UNCONDITIONAL_BRANCH_MASK) == BL_INST;
            as_.ldr_label(crate::assemble::TMP, dst);
            if is_bl {
                as_.blr(crate::assemble::TMP);
            } else {
                as_.br(crate::assemble::TMP);
            }
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
            let mut branch_inst = inst;
            let cond = bits(inst, 0, 3) ^ 1;
            branch_inst = (branch_inst & !0xF) | cond;
            // branch +3 instructions forward (over ldr+br)
            let new_off = 4 * 3;
            let imm19 = ((new_off >> 2) as u32) & 0x7FFFF;
            branch_inst = (branch_inst & !(0x7FFFF << 5)) | (imm19 << 5);
            // raw emit of modified instruction
            emit_raw(as_, branch_inst);
            as_.ldr_label(crate::assemble::TMP, dst);
            as_.br(crate::assemble::TMP);
        } else if inst_is_compare_b(inst) {
            let dst = (pc as i64 + decode_imm19(inst)) as u64;
            let mut branch_inst = inst;
            let op = bits(inst, 24, 24) ^ 1;
            branch_inst = (branch_inst & !(1 << 24)) | (op << 24);
            let new_off = 4 * 3;
            let imm19 = ((new_off >> 2) as u32) & 0x7FFFF;
            branch_inst = (branch_inst & !(0x7FFFF << 5)) | (imm19 << 5);
            emit_raw(as_, branch_inst);
            as_.ldr_label(crate::assemble::TMP, dst);
            as_.br(crate::assemble::TMP);
        } else if inst_is_test_b(inst) {
            let dst = (pc as i64 + decode_imm14(inst)) as u64;
            let mut branch_inst = inst;
            let op = bits(inst, 24, 24) ^ 1;
            branch_inst = (branch_inst & !(1 << 24)) | (op << 24);
            let new_off = 4 * 3;
            let imm14 = ((new_off >> 2) as u32) & 0x3FFF;
            branch_inst = (branch_inst & !(0x3FFF << 5)) | (imm14 << 5);
            emit_raw(as_, branch_inst);
            as_.ldr_label(crate::assemble::TMP, dst);
            as_.br(crate::assemble::TMP);
        } else {
            emit_raw(as_, inst);
        }
        offset += 4;
    }

    if let Some(back) = branch_back_to {
        as_.literal_ldr_branch(back);
    }
    Ok(offset)
}

fn emit_raw(as_: &mut Assembler, inst: u32) {
    as_.push_word(inst);
}
