//! The candidate scan — two generations.
//!
//! `scan` reproduces the reference Python tool one-to-one (kept for parity
//! and auditability; the Rust and Python binaries produce identical output
//! on every image tested).
//!
//! `scan_v2` fixes the one weakness of the reference algorithm discovered
//! while validating against real game libraries:
//!
//! * The reference back-walk scans **forward** from the window start and
//!   returns the *earliest* prologue marker. On libUE4.so (12.50) that
//!   selects a `sub sp,sp,#0x10` inside the *previous* function
//!   (curl_easy_init), 0x68 bytes before the real curl_easy_setopt, because
//!   the true prologue `stp x20,x19,[sp,#-0x20]!` is not in the reference
//!   marker set at all.
//!
//! v2 changes, in order of importance:
//! 1. Backward scan from the CBZ — the closest marker wins.
//! 2. Widened marker set: any `stp Xt1,Xt2,[sp,#-N]!` counts as a prologue.
//! 3. Predecessor sanity: the instruction before a true function start is
//!    almost always a `ret` / `b` / `bl` / `b.cond` / `ldp ...post-index` /
//!    `nop`. Markers preceded by ordinary arithmetic are skipped.
//! 4. Variadic scoring: `curl_easy_setopt` is variadic, and its prologue
//!    spills q0-q7 (and x2-x7) before the `cbz x0`. Candidates carrying the
//!    `va_start` fingerprint gain +3 score, which makes them outrank any
//!    accidental CBZ/MOV chain.
//!
//! On the validation set:
//!   libUE4.so    12.50 : v1 0x0AE64C04 (wrong by 0x68) -> v2 0x0AE64C6C
//!                        (= exported dynsym symbol, exact)
//!   libUnreal.so 23.00 : v1 = v2 = 0x0657E4A4 (verified by disassembly)
//!   libEOSSDK.so 23.00 : v1 = v2 = 0x0178AD00 (verified by disassembly)

#![forbid(unsafe_code)]

use crate::arm64::{
    bl_target, cbz_x0_target, count_variadic_spill, is_function_end,
    is_function_start, is_prologue_marker_v2, word_at, BL_HIGH_BYTE_MAX,
    BL_HIGH_BYTE_MIN, CBZ_X0_HIGH_BYTE, CBZ_X0_RT_MASK, MOV_W0_2B_BYTES,
};
use std::collections::HashMap;

/// Maximum distance between CBZ and MOV for the pair to be considered.
pub const BL_MAX_GAP: usize = 0x200;
/// Sensible gap range between CBZ and MOV (used by the scorer).
pub const CBZ_MOV_GAP_MIN: usize = 0x30;
pub const CBZ_MOV_GAP_MAX: usize = 0xC0;
/// Backwards search window when locating the function prologue.
pub const FUNC_BACK_WINDOW: usize = 0x100;

/// One scored `curl_easy_setopt` candidate.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// File offset of the function prologue, when located.
    pub func_start: Option<usize>,
    /// File offset of the `CBZ X0` instruction.
    pub cbz_off: usize,
    /// File offset of the `BL` (Curl_vsetopt call).
    pub bl_off: usize,
    /// File offset of the BL target (Curl_vsetopt), when decodable.
    pub bl_target: Option<usize>,
    /// File offset of the `MOV W0, #0x2b`.
    pub mov_off: usize,
    /// Distance in bytes between CBZ and MOV.
    pub gap: usize,
    /// Confidence score, 0-10 (reference weights) plus the v2 variadic bonus.
    pub score: u32,
    /// The `va_start` spill fingerprint, when detected (v2 only).
    pub variadic: bool,
}

impl Candidate {
    /// The recommended hook address (function start when known, otherwise the
    /// CBZ address as an estimate).
    pub fn hook_off(&self) -> usize {
        self.func_start.unwrap_or(self.cbz_off)
    }
}

/// Index every `CBZ X0` in the image as `{target_offset: [cbz_off, ...]}`.
fn index_cbz_x0(data: &[u8]) -> HashMap<usize, Vec<usize>> {
    let mut index: HashMap<usize, Vec<usize>> = HashMap::new();
    let n = data.len();
    if n < 4 {
        return index;
    }
    let mut cursor = 0usize;
    while let Some(hit) = memchr(data, 0xB4, cursor) {
        cursor = hit + 1;
        if hit < 3 {
            continue;
        }
        let off = hit - 3;
        if off % 4 != 0 || off + 4 > n {
            continue;
        }
        if data[off] & CBZ_X0_RT_MASK as u8 != 0 {
            continue;
        }
        let word = word_at(data, off);
        if (word >> 24) & 0xFF != CBZ_X0_HIGH_BYTE as u32 {
            continue;
        }
        if let Some(target) = cbz_x0_target(word, off) {
            index.entry(target).or_default().push(off);
        }
    }
    index
}

fn memchr(data: &[u8], byte: u8, from: usize) -> Option<usize> {
    if from >= data.len() {
        return None;
    }
    data[from..]
        .iter()
        .position(|&b| b == byte)
        .map(|p| p + from)
}

/// Build candidates around each MOV W0,#0x2B occurrence using the supplied
/// function-start resolver and scorer flavour.
fn scan_with(
    data: &[u8],
    v2: bool,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    let cbz_index = index_cbz_x0(data);

    let needle: &[u8] = &MOV_W0_2B_BYTES;
    let mut cursor = 0usize;
    while let Some(mov_off) = find(data, needle, cursor) {
        cursor = mov_off + 1;
        if mov_off % 4 != 0 {
            continue;
        }
        let Some(cbzs) = cbz_index.get(&mov_off) else {
            continue;
        };
        for &cbz_off in cbzs {
            let gap = mov_off.saturating_sub(cbz_off);
            if cbz_off >= mov_off || gap == 0 || gap > BL_MAX_GAP {
                continue;
            }

            // Layer 3: first BL between CBZ and MOV.
            let mut bl_off: Option<usize> = None;
            let mut bl_dst: Option<usize> = None;
            let mut j = cbz_off;
            while j + 4 <= mov_off {
                if let Some(&b3) = data.get(j + 3) {
                    if (BL_HIGH_BYTE_MIN..=BL_HIGH_BYTE_MAX).contains(&b3) {
                        let word = word_at(data, j);
                        if let Some(dst) = bl_target(word, j) {
                            bl_off = Some(j);
                            bl_dst = Some(dst);
                            break;
                        }
                    }
                }
                j += 4;
            }
            let Some(bl_off) = bl_off else { continue };

            // Layer 4: locate the prologue.
            let func_start = if v2 {
                find_function_start_v2(data, cbz_off)
            } else {
                find_function_start_legacy(data, cbz_off, FUNC_BACK_WINDOW)
            };

            // v2: the va_start fingerprint.
            let variadic = if v2 {
                let from = func_start.unwrap_or(cbz_off.saturating_sub(0x40));
                count_variadic_spill(data, from, cbz_off).is_variadic()
            } else {
                false
            };

            let mut c = Candidate {
                func_start,
                cbz_off,
                bl_off,
                bl_target: bl_dst,
                mov_off,
                gap,
                score: 0,
                variadic,
            };
            c.score = score(&c);
            out.push(c);
            break;
        }
    }
    out
}

/// Reference (Python-identical) scan.
pub fn scan(data: &[u8]) -> Vec<Candidate> {
    scan_with(data, false)
}

/// Improved scan (backward walk, widened markers, variadic bonus).
pub fn scan_v2(data: &[u8]) -> Vec<Candidate> {
    scan_with(data, true)
}

/// Reference back-walk: forward scan from the window start, earliest marker.
pub fn find_function_start_legacy(data: &[u8], cbz_off: usize, window: usize) -> Option<usize> {
    let start = cbz_off.saturating_sub(window);
    let start = (start + 3) & !3;
    let mut off = start;
    while off + 4 <= cbz_off {
        let word = word_at(data, off);
        if is_function_start(word) {
            return Some(off);
        }
        off += 4;
    }
    None
}

/// v2 back-walk: scan backwards from the CBZ. The closest marker whose
/// predecessor looks like the end of the previous function wins. If no
/// marker passes the predecessor check, the closest raw marker is used.
pub fn find_function_start_v2(data: &[u8], cbz_off: usize) -> Option<usize> {
    let lo = cbz_off.saturating_sub(FUNC_BACK_WINDOW) & !3;
    let mut off = (cbz_off - 4) & !3;
    let mut fallback: Option<usize> = None;
    while off + 4 <= cbz_off && off >= lo {
        let word = word_at(data, off);
        if is_prologue_marker_v2(word) {
            if fallback.is_none() {
                fallback = Some(off);
            }
            // Predecessor sanity: instruction before a true start is a
            // function end or padding.
            if off >= 4 {
                let prev = word_at(data, off - 4);
                if is_function_end(prev) {
                    return Some(off);
                }
            } else {
                return Some(off); // image start
            }
        }
        if off < 4 {
            break;
        }
        off -= 4;
    }
    fallback
}

/// Reference scoring (0-10) plus the v2 variadic bonus (+3).
fn score(c: &Candidate) -> u32 {
    let mut s = 0u32;
    if (CBZ_MOV_GAP_MIN..=CBZ_MOV_GAP_MAX).contains(&c.gap) {
        s += 2;
    }
    if let Some(fs) = c.func_start {
        s += 3;
        let dist = c.cbz_off.saturating_sub(fs);
        if (0x10..=0x80).contains(&dist) {
            s += 2;
        }
    }
    if c.bl_target.is_some() {
        s += 2;
        if c.mov_off.abs_diff(c.bl_off) < 0x40 {
            s += 1;
        }
    }
    if c.variadic {
        s += 3;
    }
    s
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}
