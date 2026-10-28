//! AArch64 instruction decoding helpers used by the scanner.
//!
//! Two generations of markers are provided:
//!
//! * The reference set (`is_function_start`) — identical to the original
//!   `find_curl_easy_setopt.py`: `sub sp,sp,#imm`, `stp x29,x30,[sp,#-N]!`,
//!   `pacibsp`, `bti`.
//! * The v2 set (`is_prologue_marker_v2`) — widens the STP pre-index marker to
//!   any register pair (`stp Xt1,Xt2,[sp,#-N]!`), which is how clang actually
//!   emits the prologue of `curl_easy_setopt` on Android (`stp x20,x19` first).
//!
//! v2 also knows the variadic signature of `curl_easy_setopt` (the `va_start`
//! register spill sequence: `stp q0,q1 / q2,q3 / ...` and `stp x4,x5 /
//! x6,x7`), which is used as a scoring bonus and is near-impossible to hit by
//! accident in any other function shape.

#![forbid(unsafe_code)]

/// `mov w0, #0x2b` — CURLE_BAD_FUNCTION_ARGUMENT (43) returned by
/// curl_easy_setopt when the handle is NULL.
pub const MOV_W0_2B_BYTES: [u8; 4] = [0x60, 0x05, 0x80, 0x52];

/// High byte of a `CBZ` instruction (sf=1, i.e. X registers).
pub const CBZ_X0_HIGH_BYTE: u8 = 0xB4;
/// Mask for the Rt field of CBZ/TBZ-family encodings.
pub const CBZ_X0_RT_MASK: u32 = 0x1F;
/// Inclusive range of the high byte of a `BL` instruction.
pub const BL_HIGH_BYTE_MIN: u8 = 0x94;
pub const BL_HIGH_BYTE_MAX: u8 = 0x97;

fn sign_extend(value: u32, bits: u32) -> i64 {
    let sign = 1u32 << (bits - 1);
    ((value ^ sign) as i64) - (sign as i64)
}

/// Decode the branch target of a `CBZ X0, ...` instruction.
/// Returns `None` when the word is not a CBZ on X0.
pub fn cbz_x0_target(word: u32, pc: usize) -> Option<usize> {
    if ((word >> 24) & 0xFF) != CBZ_X0_HIGH_BYTE as u32 {
        return None;
    }
    if (word & CBZ_X0_RT_MASK) != 0 {
        return None;
    }
    let imm19 = (word >> 5) & 0x7FFFF;
    let offset = sign_extend(imm19 << 2, 2 + 19);
    Some((pc as i64 + offset) as usize)
}

/// Decode the branch target of a `BL` instruction.
pub fn bl_target(word: u32, pc: usize) -> Option<usize> {
    if (word >> 26) & 0x3F != 0b100101 {
        return None;
    }
    let imm26 = word & 0x03FF_FFFF;
    let offset = sign_extend(imm26 << 2, 2 + 26);
    Some((pc as i64 + offset) as usize)
}

// ---------------------------------------------------------------------------
// Prologue markers
// ---------------------------------------------------------------------------

/// `sub sp, sp, #imm`
pub fn is_sub_sp_sp(word: u32) -> bool {
    (word & 0xFF80_03FF) == 0xD100_03FF
}

/// `stp x29, x30, [sp, #-N]!` (pre-index writeback) — reference marker.
pub fn is_stp_fp_lr_preindex(word: u32) -> bool {
    is_stp_preindex_sp(word) && (word & 0x1F) == 29 && ((word >> 10) & 0x1F) == 30
}

/// Any `stp Xt1, Xt2, [sp, #-N]!` — v2 marker. This is the encoding clang
/// uses for the first prologue instruction of most Epic/curl functions
/// (e.g. `stp x20, x19, [sp, #-0x20]!` = 0xA9BE4FF4).
pub fn is_stp_preindex_sp(word: u32) -> bool {
    (word & 0xFFC0_0000) == 0xA980_0000 && ((word >> 5) & 0x1F) == 31
}

/// `stp qQt1, qQt2, [sp, #imm]` — 128-bit SIMD pair store, the FP half of the
/// AArch64 variadic (`va_start`) spill sequence.
pub fn is_stp_q_pair(word: u32) -> bool {
    // signed-offset form 0xAD000000 and pre-index form 0xAD800000
    let base = word & 0xFFC0_0000;
    base == 0xAD00_0000 || base == 0xAD80_0000
}

/// `stp Xt1, Xt2, [sp, #imm]` (signed offset) — used to detect the integer
/// varargs spill (`stp x4,x5` / `stp x6,x7`) with any base register.
pub fn is_stp_any_64bit(word: u32) -> bool {
    (word & 0xFFC0_0000) == 0xA900_0000
}

/// `pacibsp` pointer-authentication prologue.
pub fn is_pacibsp(word: u32) -> bool {
    word == 0xD503_237F
}

/// Branch target identification markers (`bti`, `bti c`, `bti j`, `bti jc`).
pub fn is_bti(word: u32) -> bool {
    matches!(
        word,
        0xD503_241F | 0xD503_245F | 0xD503_249F | 0xD503_24DF
    )
}

/// Reference marker set (identical to the Python tool).
pub fn is_function_start(word: u32) -> bool {
    is_sub_sp_sp(word) || is_stp_fp_lr_preindex(word) || is_pacibsp(word) || is_bti(word)
}

/// v2 marker set: the reference set plus any `stp Xt1,Xt2,[sp,#-N]!`.
pub fn is_prologue_marker_v2(word: u32) -> bool {
    is_function_start(word) || is_stp_preindex_sp(word)
}

// ---------------------------------------------------------------------------
// Function-end markers (used by the v2 predecessor sanity check)
// ---------------------------------------------------------------------------

pub const RET: u32 = 0xD65F_03C0;
pub const NOP: u32 = 0xD503_201F;

/// `ret`
pub fn is_ret(word: u32) -> bool {
    (word & 0xFFFF_FFE0) == RET
}

/// Unconditional `b imm`
pub fn is_b(word: u32) -> bool {
    (word & 0xFC00_0000) == 0x1400_0000
}

/// `bl imm`
pub fn is_bl(word: u32) -> bool {
    (word & 0xFC00_0000) == 0x9400_0000
}

/// Conditional branch `b.cond`
pub fn is_b_cond(word: u32) -> bool {
    (word & 0xFF00_0010) == 0x5400_0000
}

/// `ldp Xt1, Xt2, [sp], #imm` post-index epilogue.
pub fn is_ldp_postindex_sp(word: u32) -> bool {
    (word & 0xFFC0_0000) == 0xA8C0_0000 && ((word >> 5) & 0x1F) == 31
}

/// A word that can plausibly terminate the previous function or act as
/// inter-function padding. If the instruction right before a candidate
/// prologue is one of these, the prologue is very likely a real function
/// start rather than a mid-body `sub sp, sp` or stray STP.
pub fn is_function_end(word: u32) -> bool {
    is_ret(word)
        || is_b(word)
        || is_bl(word)
        || is_b_cond(word)
        || is_ldp_postindex_sp(word)
        || word == NOP
        || word == 0xD420_0000 // brk
}

// ---------------------------------------------------------------------------
// Variadic (va_start) signature detection — the curl_easy_setopt fingerprint
// ---------------------------------------------------------------------------

/// Count the FP/integer varargs spill instructions between two offsets.
/// `curl_easy_setopt` is variadic; its prologue spills q0-q7 (and typically
/// x2-x7) right after the stack setup, before the `cbz x0` NULL-handle check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VariadicSpill {
    pub q_pairs: u32,
    pub x_pairs: u32,
}

impl VariadicSpill {
    pub fn is_variadic(&self) -> bool {
        // Two or more SIMD pair spills is the va_start fingerprint.
        self.q_pairs >= 2
    }
}

/// Inspect `[from, to)` and count spill instructions.
pub fn count_variadic_spill(data: &[u8], from: usize, to: usize) -> VariadicSpill {
    let mut v = VariadicSpill::default();
    let mut off = from & !3;
    while off + 4 <= to {
        let word = word_at(data, off);
        if is_stp_q_pair(word) {
            v.q_pairs += 1;
        } else if is_stp_any_64bit(word) {
            // integer pair spill with a negative offset from x29 or a plain
            // [sp] store — count x4,x5/x6,x7 style spills conservatively
            let rt = word & 0x1F;
            let rt2 = (word >> 10) & 0x1F;
            if rt >= 4 && rt2 == rt + 1 {
                v.x_pairs += 1;
            }
        }
        off += 4;
    }
    v
}

/// Read a little-endian 32-bit instruction at `off`. Returns 0 when out of
/// bounds (the scanner treats out-of-range reads as non-matching).
#[inline]
pub fn word_at(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() {
        return 0;
    }
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}
