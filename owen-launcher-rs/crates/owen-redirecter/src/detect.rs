//! Runtime discovery of `curl_easy_setopt` in the loaded game libraries.
//!
//! Two strategies, matching the Web Console configuration:
//!
//! 1. `symbols` — `dlsym` on the loaded library (works on libUE4.so-era
//!    builds where curl is dynamically exported).
//! 2. `signature` — the owen-detect v2 scan over the mapped image of a
//!    **stripped** library (libUnreal.so 13.40+ and libEOSSDK.so 22.00+).
//!
//! `auto` tries symbols first and falls back to the signature scan. The
//! signature path is what removes the per-version offset tables from the
//! original Owen.c entirely: 10.31 through 33.00+ all resolve automatically.

use crate::config::DetectStrategy;
use crate::maps::Module;
use owen_detect::Candidate;

extern "C" {
    fn dlsym(handle: *mut core::ffi::c_void, symbol: *const u8) -> *mut core::ffi::c_void;
    fn dlopen(filename: *const u8, flags: i32) -> *mut core::ffi::c_void;
}

const RTLD_DEFAULT: i32 = 0;
const RTLD_NOW: i32 = 2;
const RTLD_NOLOAD: i32 = 4;

/// Resolve `curl_easy_setopt` by symbol lookup in the given module.
pub fn by_symbol(lib_name: &str) -> Option<usize> {
    let sym = b"curl_easy_setopt\0";
    // SAFETY: dlsym(RTLD_DEFAULT, ...) searches the caller's namespace —
    // our library is loaded into the same namespace as the game libraries.
    unsafe {
        let p = dlsym(core::ptr::null_mut(), sym.as_ptr());
        if !p.is_null() {
            return Some(p as usize);
        }
    }
    let name_c: Vec<u8> = lib_name.bytes().chain(std::iter::once(0)).collect();
    // SAFETY: dlopen with RTLD_NOLOAD returns the existing handle or null
    // without loading anything new.
    unsafe {
        let h = dlopen(name_c.as_ptr(), RTLD_NOW | RTLD_NOLOAD);
        if h.is_null() {
            return None;
        }
        let p = dlsym(h, sym.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(p as usize)
        }
    }
}

/// Locate `curl_easy_setopt` in the mapped image of `module` using the
/// owen-detect v2 signature scan.
///
/// Returns the best candidate above `min_score` together with its details.
pub fn by_signature(module: &Module, min_score: u32) -> Option<(usize, Candidate)> {
    // The ELF header of a mapped shared library sits exactly at `base`
    // (the first PT_LOAD maps file offset 0 at the load bias).
    let base = module.base as *const u8;
    // SAFETY: reading the 64-byte ELF header at a mapped module base. The
    // module table derived this address from /proc/self/maps, and every
    // mapped library starts with a readable ELF header.
    let header = unsafe { core::slice::from_raw_parts(base, 64) };
    if header.get(0..4) != Some(b"\x7fELF") {
        return None;
    }

    // Determine the extent we may scan: up to the end of the executable
    // region (covers program headers, rodata and text; excludes the
    // mutable data segment, which would only produce noise).
    let extent = module.exec_end.saturating_sub(module.base);
    if extent < 4096 {
        return None;
    }
    // SAFETY: [base, base+extent) is fully mapped and readable (r--, r-x)
    // per /proc/self/maps.
    let image = unsafe { core::slice::from_raw_parts(base, extent) };

    let report = owen_detect::scan_elf_image(image)?;
    let best = report.candidates.first()?;
    if best.score < min_score {
        return None;
    }
    // Scanner reports virtual addresses in the library's own vaddr space;
    // the module base is the load bias (first PT_LOAD has vaddr 0).
    let va = owen_detect::off_to_va(&report.segments, best.hook_off())?;
    Some((module.base + va as usize, best.clone()))
}

/// Combined strategy entry point.
pub fn locate(
    lib_name: &str,
    module: &Module,
    strategy: DetectStrategy,
    min_score: u32,
) -> Option<(usize, &'static str)> {
    match strategy {
        DetectStrategy::SymbolsOnly => by_symbol(lib_name).map(|a| (a, "dlsym")),
        DetectStrategy::SignatureOnly => {
            by_signature(module, min_score).map(|(a, _)| (a, "signature-v2"))
        }
        DetectStrategy::Auto => by_symbol(lib_name)
            .map(|a| (a, "dlsym"))
            .or_else(|| by_signature(module, min_score).map(|(a, _)| (a, "signature-v2"))),
    }
}

/// Extract the `++Fortnite+Release-...` build string from a mapped module.
pub fn detect_version(module: &Module) -> Option<String> {
    let needle = b"++Fortnite+Release-";
    let extent = module.exec_end.saturating_sub(module.base);
    if extent < 4096 {
        return None;
    }
    // SAFETY: readable mapped range as above.
    let image = unsafe { core::slice::from_raw_parts(module.base as *const u8, extent) };
    let mut i = 0usize;
    while i + needle.len() < image.len() {
        if &image[i..i + needle.len()] == needle {
            let start = i;
            let mut end = i;
            while end < image.len() && end - start < 64 {
                let c = image[end];
                if c == 0 || !(0x20..0x7f).contains(&c) {
                    break;
                }
                end += 1;
            }
            if end > start {
                return Some(String::from_utf8_lossy(&image[start..end]).into_owned());
            }
        }
        i += 1;
    }
    None
}
