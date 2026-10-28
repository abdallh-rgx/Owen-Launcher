//! Automatic `curl_easy_setopt` detection for stripped AArch64 ELF images.
//!
//! This is a faithful Rust port of the reference Python tool
//! (`find_curl_easy_setopt.py`). The strategy has four layers:
//!
//! 1. `MOV W0, #0x2b` — libcurl returns `CURLE_OK` (43) on success.
//! 2. `CBZ X0, ...` — the `if (!handle)` null check jumps to the same spot.
//! 3. A `BL` between the `CBZ` and the `MOV` — the call into
//!    `Curl_vsetopt`.
//! 4. A backwards walk from the `CBZ` to the real function prologue
//!    (`SUB SP`, `STP X29, X30, [SP, #-N]!`, `PACIBSP`, or `BTI`).
//!
//! Works on libUE4.so, libUnreal.so and libEOSSDK.so alike because it never
//! relies on symbols — only on instruction semantics.

use crate::elf::{off_to_va, ElfImage, Segment};
use std::path::Path;

// ---------------------------------------------------------------------------
// AArch64 signature constants (identical to the Python reference)
// ---------------------------------------------------------------------------

/// `mov w0, #0x2b`  (little-endian bytes `60 05 80 52`)
pub const MOV_W0_2B_BYTES: [u8; 4] = [0x60, 0x05, 0x80, 0x52];
const CBZ_X0_HIGH_BYTE: u8 = 0xB4;
const CBZ_X0_RT_MASK: u32 = 0x1F;
const BL_HIGH_BYTE_MIN: u8 = 0x94;
const BL_HIGH_BYTE_MAX: u8 = 0x97;

const BL_MAX_GAP: u32 = 0x200;
const CBZ_MOV_GAP_MIN: u32 = 0x30;
const CBZ_MOV_GAP_MAX: u32 = 0xC0;
const FUNC_BACK_WINDOW: u32 = 0x100;

// ---------------------------------------------------------------------------
// Candidate model
// ---------------------------------------------------------------------------

/// One candidate location for `curl_easy_setopt`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Candidate {
    /// Score of the candidate (0-10), higher is better.
    pub score: u32,
    /// File offset of the detected function start (when found).
    pub function_start: Option<u64>,
    /// Virtual address of the detected function start.
    pub function_start_va: Option<u64>,
    /// File offset of the `CBZ X0` instruction.
    pub cbz_offset: u64,
    pub cbz_va: Option<u64>,
    /// File offset of the `BL` instruction.
    pub bl_offset: u64,
    pub bl_va: Option<u64>,
    /// Target of the BL (the `Curl_vsetopt` callee), file offset.
    pub bl_target_offset: Option<u64>,
    pub bl_target_va: Option<u64>,
    /// File offset of the `MOV W0, #0x2b` instruction.
    pub mov_offset: u64,
    pub mov_va: Option<u64>,
    /// Gap between the CBZ and the MOV, in bytes.
    pub gap_cbz_to_mov: u32,
}

/// Result of scanning one image.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanReport {
    pub candidates: Vec<Candidate>,
    /// Exported `curl_easy_setopt` symbol if present (dynamic symbol table).
    pub exported_symbol_va: Option<u64>,
    /// Best function-start virtual address (exported symbol wins over scan).
    pub best_va: Option<u64>,
    pub best_file_offset: Option<u64>,
    pub version: Option<String>,
    /// Parse error, if the image was not a valid ELF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _error: Option<String>,
}

// ---------------------------------------------------------------------------
// AArch64 decoding helpers
// ---------------------------------------------------------------------------

fn sign_extend(value: u32, bits: u32) -> i64 {
    let sign = 1i64 << (bits - 1);
    ((value as i64) ^ sign) - sign
}

fn read_u32(data: &[u8], off: usize) -> Option<u32> {
    let s = data.get(off..off + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Decode the branch target of `CBZ X0`.
fn cbz_x0_target(word: u32, pc: u64) -> Option<u64> {
    if ((word >> 24) & 0xFF) as u8 != CBZ_X0_HIGH_BYTE {
        return None;
    }
    if (word & CBZ_X0_RT_MASK) != 0 {
        return None;
    }
    let imm19 = sign_extend((word >> 5) & 0x7FFFF, 19);
    Some((pc as i64 + (imm19 << 2)) as u64)
}

/// Decode the branch target of `BL`.
fn bl_target(word: u32, pc: u64) -> Option<u64> {
    if (word >> 26) & 0x3F != 0b100101 {
        return None;
    }
    let imm26 = sign_extend(word & 0x03FF_FFFF, 26);
    Some((pc as i64 + (imm26 << 2)) as u64)
}

fn is_sub_sp_sp(word: u32) -> bool {
    (word & 0xFF8003FF) == 0xD10003FF
}

fn is_stp_fp_lr_preindex(word: u32) -> bool {
    if (word & 0xFFC00000) != 0xA9800000 {
        return false;
    }
    let rt = word & 0x1F;
    let rt2 = (word >> 10) & 0x1F;
    let rn = (word >> 5) & 0x1F;
    rt == 29 && rt2 == 30 && rn == 31
}

fn is_pacibsp(word: u32) -> bool {
    word == 0xD503237F
}

fn is_bti(word: u32) -> bool {
    matches!(word, 0xD503241F | 0xD503245F | 0xD503249F | 0xD50324DF)
}

fn is_function_start(word: u32) -> bool {
    is_sub_sp_sp(word) || is_stp_fp_lr_preindex(word) || is_pacibsp(word) || is_bti(word)
}

/// Walk backwards from `cbz_off` looking for a function prologue. The
/// **nearest** prologue wins — between the window start and the CBZ there
/// can be several complete small functions, and the one that owns the CBZ
/// is the closest one.
fn find_function_start(data: &[u8], cbz_off: usize, window: u32) -> Option<usize> {
    let start = cbz_off.saturating_sub(window as usize);
    let start = (start + 3) & !3;
    let mut off = (cbz_off - 4) & !3;
    while off >= start {
        if let Some(word) = read_u32(data, off) {
            if is_function_start(word) {
                return Some(off);
            }
        }
        if off < 4 {
            break;
        }
        off -= 4;
    }
    None
}

/// Index of every `CBZ X0` keyed by its branch target.
fn index_cbz_x0(data: &[u8]) -> std::collections::HashMap<usize, Vec<usize>> {
    let mut index: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let n = data.len();
    let mut cursor = 0usize;
    while let Some(rel) = data[cursor.min(n)..].iter().position(|&b| b == CBZ_X0_HIGH_BYTE) {
        let hit = cursor + rel;
        if hit < 3 || hit + 1 >= n {
            break;
        }
        cursor = hit + 1;
        let off = hit - 3;
        if off % 4 != 0 || off + 4 > n {
            continue;
        }
        if (data[off] as u32) & CBZ_X0_RT_MASK != 0 {
            continue;
        }
        if let Some(word) = read_u32(data, off) {
            if let Some(target) = cbz_x0_target(word, off as u64) {
                if let Some(t) = u64_to_usize(target) {
                    index.entry(t).or_default().push(off);
                }
            }
        }
    }
    index
}

fn u64_to_usize(v: u64) -> Option<usize> {
    usize::try_from(v).ok()
}

fn score(c: &Candidate) -> u32 {
    let mut s = 0u32;
    if (CBZ_MOV_GAP_MIN..=CBZ_MOV_GAP_MAX).contains(&c.gap_cbz_to_mov) {
        s += 2;
    }
    if let Some(fs) = c.function_start {
        s += 3;
        let dist = (c.cbz_offset - fs) as u32;
        if (0x10..=0x80).contains(&dist) {
            s += 2;
        }
    }
    if c.bl_target_offset.is_some() {
        s += 2;
        if (c.mov_offset.saturating_sub(c.bl_offset)) < 0x40 {
            s += 1;
        }
    }
    s
}

/// Core scan over raw image bytes. Returns candidates sorted by score
/// (descending), then by offset.
///
/// When `segments` are supplied, file offsets are additionally translated to
/// virtual addresses.
pub fn scan_image(data: &[u8], segments: Option<&[Segment]>) -> Vec<Candidate> {
    let cbz_index = index_cbz_x0(data);
    let mut out: Vec<Candidate> = Vec::new();
    let va = |off: u64| -> Option<u64> {
        segments.and_then(|segs| off_to_va(segs, off))
    };

    let mut cursor = 0usize;
    while let Some(rel) = data[cursor.min(data.len())..]
        .windows(4)
        .position(|w| w == MOV_W0_2B_BYTES)
    {
        let mov_off = cursor + rel;
        cursor = mov_off + 1;
        if mov_off % 4 != 0 || mov_off + 4 > data.len() {
            continue;
        }

        // The MOV is the branch target of the leading CBZ X0.
        let mut chosen: Option<Candidate> = None;
        for cbz_off in cbz_index.get(&mov_off).cloned().unwrap_or_default() {
            // Signed gap: backwards branches produce negative gaps and are
            // rejected exactly like the Python reference (`gap <= 0`).
            let gap_i = mov_off as i64 - cbz_off as i64;
            if gap_i <= 0 || gap_i > BL_MAX_GAP as i64 {
                continue;
            }
            let gap = gap_i as u32;

            // Find the BL between CBZ and MOV.
            let mut bl_off: i64 = -1;
            let mut bl_dest: Option<u64> = None;
            let mut j = cbz_off;
            while j + 4 <= mov_off {
                if (BL_HIGH_BYTE_MIN..=BL_HIGH_BYTE_MAX).contains(&data[j + 3]) {
                    if let Some(word) = read_u32(data, j) {
                        bl_off = j as i64;
                        bl_dest = bl_target(word, j as u64);
                        break;
                    }
                }
                j += 4;
            }
            if bl_off < 0 {
                continue;
            }
            let bl_off = bl_off as usize;

            let func_start = find_function_start(data, cbz_off, FUNC_BACK_WINDOW);

            let mut c = Candidate {
                score: 0,
                function_start: func_start.map(|v| v as u64),
                function_start_va: func_start.map(|v| va(v as u64)).flatten(),
                cbz_offset: cbz_off as u64,
                cbz_va: va(cbz_off as u64),
                bl_offset: bl_off as u64,
                bl_va: va(bl_off as u64),
                bl_target_offset: bl_dest.and_then(u64_to_usize).map(|v| v as u64),
                bl_target_va: bl_dest.and_then(|t| va(t)),
                mov_offset: mov_off as u64,
                mov_va: va(mov_off as u64),
                gap_cbz_to_mov: gap,
            };
            c.score = score(&c);
            chosen = Some(c);
            break;
        }
        if let Some(c) = chosen {
            out.push(c);
        }
    }

    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.cbz_offset.cmp(&b.cbz_offset)));
    out
}

/// Scan an ELF file from disk. Reads the whole file (Fortnite native
/// libraries are 20-310 MB) and runs the pattern scan.
pub fn scan_path(path: &Path) -> Result<ScanReport, String> {
    let data = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let data: &[u8] = &data;

    let elf = ElfImage::parse(data).map_err(|e| e.to_string())?;
    let segments: Vec<Segment> = elf.segments.clone();

    // Exported symbol fast-path.
    let exported_symbol_va = elf
        .find_symbol("curl_easy_setopt")
        .filter(|s| s.value != 0)
        .map(|s| s.value);

    let mut candidates = scan_image(data, Some(&segments));

    // Prefer candidates whose function start is inside an executable segment.
    for c in &mut candidates {
        if let Some(fs) = c.function_start_va {
            let exec = segments
                .iter()
                .any(|s| s.flags & 1 != 0 && fs >= s.vaddr && fs < s.vaddr + s.filesz);
            if !exec {
                c.score = c.score.saturating_sub(1);
            }
        }
    }
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then(a.cbz_offset.cmp(&b.cbz_offset)));

    let version = crate::version::detect_from_bytes(data).map(|v| v.to_string());

    let (best_va, best_file_offset) = if let Some(sym) = exported_symbol_va {
        (Some(sym), elf.va_to_off(sym))
    } else {
        let best = candidates.first().and_then(|c| c.function_start_va.or(c.cbz_va));
        let best_off = candidates.first().and_then(|c| c.function_start.or(Some(c.cbz_offset)));
        (best, best_off)
    };

    Ok(ScanReport {
        candidates,
        exported_symbol_va,
        best_va,
        best_file_offset,
        version,
        _error: None,
    })
}

/// Scan an in-memory ELF image (e.g. an uploaded file). Equivalent to
/// [`scan_path`] but without touching the filesystem.
pub fn scan_image_data(data: &[u8]) -> ScanReport {
    let elf = match ElfImage::parse(data) {
        Ok(e) => e,
        Err(e) => {
            return ScanReport {
                candidates: Vec::new(),
                exported_symbol_va: None,
                best_va: None,
                best_file_offset: None,
                version: None,
                _error: Some(e.to_string()),
            }
        }
    };
    let segments = elf.segments.clone();
    let exported_symbol_va = elf
        .find_symbol("curl_easy_setopt")
        .filter(|s| s.value != 0)
        .map(|s| s.value);

    let mut candidates = scan_image(data, Some(&segments));
    for c in &mut candidates {
        if let Some(fs) = c.function_start_va {
            let exec = segments
                .iter()
                .any(|s| s.flags & 1 != 0 && fs >= s.vaddr && fs < s.vaddr + s.filesz);
            if !exec {
                c.score = c.score.saturating_sub(1);
            }
        }
    }
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then(a.cbz_offset.cmp(&b.cbz_offset)));

    let version = crate::version::detect_from_bytes(data).map(|v| v.to_string());
    let (best_va, best_file_offset) = if let Some(sym) = exported_symbol_va {
        (Some(sym), elf.va_to_off(sym))
    } else {
        (
            candidates.first().and_then(|c| c.function_start_va.or(c.cbz_va)),
            candidates.first().and_then(|c| c.function_start.or(Some(c.cbz_offset))),
        )
    };

    ScanReport {
        candidates,
        exported_symbol_va,
        best_va,
        best_file_offset,
        version,
        _error: None,
    }
}
