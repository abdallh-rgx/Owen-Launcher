//! owen-detect — universal `curl_easy_setopt` locator for stripped AArch64 ELF
//! shared libraries (libUE4.so / libUnreal.so / libEOSSDK.so and any other
//! ELF64-LE image).
//!
//! This crate is a faithful, dependency-free, `unsafe`-free port of the
//! original `find_curl_easy_setopt.py` tool. It is used in two places:
//!
//! 1. As a CLI / web tool to locate the function in a `.so` file ahead of time.
//! 2. Embedded inside `libowen.so` at runtime, where the same four-layer
//!    strategy locates the hook target automatically on every game version
//!    (10.31 through 33.00+ and beyond) with no per-version offset tables.
//!
//! Strategy (four layers, identical to the reference implementation):
//! 1. `MOV W0, #0x2b`   — CURLE_OK (43) is returned at the function epilogue.
//! 2. `CBZ X0 -> same address` — validate the branch target by decoding imm19.
//! 3. `BL between CBZ and MOV` — the inner call to `Curl_vsetopt`.
//! 4. Back-walk from CBZ — locate the real function prologue (sub sp / stp /
//!    pacibsp / bti) so the returned address is the function start, not a
//!    mid-function estimate.
//!
//! Every function in this crate is safe; the crate compiles with
//! `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]

pub mod arm64;
pub mod elf;
pub mod scan;

pub use elf::{off_to_va, Segment};
pub use scan::{scan, scan_v2, Candidate};

/// Combined result of scanning one image.
#[derive(Debug, Clone)]
pub struct ScanReport {
    /// ELF PT_LOAD segments discovered in the image.
    pub segments: Vec<Segment>,
    /// All candidates, sorted by score (best first) then by offset.
    pub candidates: Vec<Candidate>,
}

/// Scan a full ELF image (file bytes or an in-memory copy of the mapped
/// library) with the improved v2 strategy (backward walk, widened prologue
/// markers, variadic fingerprint bonus). Returns segments plus ranked
/// candidates.
pub fn scan_elf_image(data: &[u8]) -> Option<ScanReport> {
    let segments = elf::read_elf_segments(data)?;
    let mut candidates = scan_v2(data);
    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.cbz_off.cmp(&b.cbz_off))
    });
    Some(ScanReport {
        segments,
        candidates,
    })
}

/// Reference scan (byte-for-byte parity with the original Python tool),
/// kept for auditing and cross-checking.
pub fn scan_elf_image_legacy(data: &[u8]) -> Option<ScanReport> {
    let segments = elf::read_elf_segments(data)?;
    let mut candidates = scan(data);
    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.cbz_off.cmp(&b.cbz_off))
    });
    Some(ScanReport {
        segments,
        candidates,
    })
}

/// Best-guess hook address (virtual address) for the image, if any candidate
/// was found. This mirrors the `--quiet` mode of the reference tool.
pub fn best_va(data: &[u8]) -> Option<u64> {
    let report = scan_elf_image(data)?;
    let best = report.candidates.first()?;
    let target = best
        .func_start
        .unwrap_or(best.cbz_off);
    off_to_va(&report.segments, target)
}
