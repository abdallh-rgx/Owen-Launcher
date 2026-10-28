//! Zero-smali loader injection: add `libowen.so` to the dynamic section of
//! the engine library (libUE4.so / libUnreal.so) as a `DT_NEEDED`
//! dependency. The Android linker loads Owen before the engine runs —
//! no dex, no smali, no bytecode modification at all.

#![forbid(unsafe_code)]

pub const DT_NULL: u64 = 0;
pub const DT_NEEDED: u64 = 1;

fn u16_at(d: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(off)?, *d.get(off + 1)?]))
}
fn u32_at(d: &[u8], off: usize) -> Option<u32> {
    let b = d.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn u64_at(d: &[u8], off: usize) -> Option<u64> {
    let b = d.get(off..off + 8)?;
    let mut v = [0u8; 8];
    v.copy_from_slice(b);
    Some(u64::from_le_bytes(v))
}

/// ELF64 section header locations relevant to us.
struct Layout {
    dyn_off: usize,
    dyn_len: usize,
    dynstr_off: usize,
    dynstr_len: usize,
}

fn find_dynamic(image: &[u8]) -> Option<Layout> {
    if image.get(0..4) != Some(b"\x7fELF") || image.get(4) != Some(&2) {
        return None;
    }
    let shoff = u64_at(image, 0x28)? as usize;
    let shentsize = u16_at(image, 0x3a)? as usize;
    let shnum = u16_at(image, 0x3c)? as usize;
    let shstrndx = u16_at(image, 0x3e)? as usize;

    let sh = |i: usize| -> Option<(u64, u32, u64, u64)> {
        // (name, type, offset, size)
        let b = shoff.checked_add(i * shentsize)?;
        let name = u32_at(image, b)? as u64;
        let ty = u32_at(image, b + 4)?;
        let off = u64_at(image, b + 24)?;
        let size = u64_at(image, b + 32)?;
        Some((name, ty, off, size))
    };
    let strtab_off = sh(shstrndx)?.2 as usize;

    let name_of = |section: (u64, u32, u64, u64)| -> Option<String> {
        let start = strtab_off + section.0 as usize;
        let end = image[start..].iter().position(|&b| b == 0)? + start;
        Some(String::from_utf8_lossy(&image[start..end]).into_owned())
    };

    let mut dynamic = None;
    let mut dynstr = None;
    for i in 0..shnum {
        let s = sh(i)?;
        if s.1 == 6 {
            // SHT_DYNAMIC
            dynamic = Some((s.2 as usize, s.3 as usize));
        }
        if s.1 == 3 {
            // SHT_STRTAB
            if let (Some(_), Some(name)) = (dynamic, name_of(s)) {
                if name == ".dynstr" {
                    dynstr = Some((s.2 as usize, s.3 as usize));
                    break;
                }
            }
        }
    }
    // .dynstr may come before .dynamic in section order; second pass.
    if dynstr.is_none() {
        for i in 0..shnum {
            let s = sh(i)?;
            if s.1 == 3 && name_of(s).as_deref() == Some(".dynstr") {
                dynstr = Some((s.2 as usize, s.3 as usize));
            }
        }
    }
    let (dyn_off, dyn_len) = dynamic?;
    let (dynstr_off, dynstr_len) = dynstr?;
    Some(Layout {
        dyn_off,
        dyn_len,
        dynstr_off,
        dynstr_len,
    })
}

/// Check whether the image already depends on `lib`.
pub fn has_needed(image: &[u8], lib: &str) -> Option<bool> {
    let l = find_dynamic(image)?;
    let mut off = l.dyn_off;
    let end = l.dyn_off + l.dyn_len;
    while off + 16 <= end {
        let tag = u64_at(image, off)?;
        let val = u64_at(image, off + 8)?;
        if tag == DT_NULL {
            break;
        }
        if tag == DT_NEEDED {
            let s = l.dynstr_off + val as usize;
            let e = image[s..].iter().position(|&b| b == 0)? + s;
            if &image[s..e] == lib.as_bytes() {
                return Some(true);
            }
        }
        off += 16;
    }
    Some(false)
}

/// The injection result.
pub enum InjectOutcome {
    /// Dependency written; returns the new image bytes.
    Injected(Vec<u8>),
    /// Already present, nothing to do.
    AlreadyPresent,
    /// No spare DT_NULL slot in .dynamic.
    NoDynamicSlot,
    /// No room in .dynstr for the name.
    NoStringRoom,
    NotAnElf,
}

/// Inject `DT_NEEDED lib` into a copy of the image.
///
/// Strategy (the classic zero-relocation trick):
/// 1. Write `lib` into the slack at the end of `.dynstr` (the section is
///    almost always padded to alignment, and even when it is not, the
///    linker never reads past the last referenced string's NUL).
/// 2. Convert the **first** `DT_NULL` terminator of `.dynamic` into
///    `DT_NEEDED` pointing at the new string. Terminating padding entries
///    usually follow, so the array remains properly terminated.
pub fn inject_needed(image: &[u8], lib: &str) -> InjectOutcome {
    if image.get(0..4) != Some(b"\x7fELF") {
        return InjectOutcome::NotAnElf;
    }
    if matches!(has_needed(image, lib), Some(true)) {
        return InjectOutcome::AlreadyPresent;
    }
    let Some(l) = find_dynamic(image) else {
        return InjectOutcome::NotAnElf;
    };

    // 1. Room in .dynstr?
    let mut out = image.to_vec();
    let name_off = {
        // Find the end of used strings: first NUL after each string is its
        // terminator; the section length minus the longest string run is the
        // slack. Conservative: use the trailing region after the last
        // string's NUL.
        let region = &image[l.dynstr_off..l.dynstr_off + l.dynstr_len];
        let Some(last_nul) = region.iter().rposition(|&b| b == 0) else {
            return InjectOutcome::NoStringRoom;
        };
        let slack_start = l.dynstr_off + last_nul + 1;
        let slack = l.dynstr_off + l.dynstr_len - slack_start;
        if lib.len() + 1 > slack {
            return InjectOutcome::NoStringRoom;
        }
        slack_start
    };
    for (i, b) in lib.bytes().chain(std::iter::once(0)).enumerate() {
        out[name_off + i] = b;
    }
    let str_index = (name_off - l.dynstr_off) as u64;

    // 2. Convert the first DT_NULL into DT_NEEDED.
    let end = l.dyn_off + l.dyn_len;
    let mut off = l.dyn_off;
    let mut patched = false;
    while off + 16 <= end {
        let tag = u64_at(&out, off).unwrap_or(u64::MAX);
        if tag == DT_NULL {
            // Safety guard: a second DT_NULL (or the section end) must
            // follow, otherwise converting this one would leave the array
            // unterminated.
            let next_tag = if off + 32 <= end {
                u64_at(&out, off + 16).unwrap_or(u64::MAX)
            } else {
                DT_NULL
            };
            if off + 32 <= end && next_tag != DT_NULL {
                return InjectOutcome::NoDynamicSlot;
            }
            out[off..off + 8].copy_from_slice(&DT_NEEDED.to_le_bytes());
            out[off + 8..off + 16].copy_from_slice(&str_index.to_le_bytes());
            patched = true;
            break;
        }
        off += 16;
    }
    if !patched {
        return InjectOutcome::NoDynamicSlot;
    }
    InjectOutcome::Injected(out)
}

/// Parse helpers used by tests and the CLI to report the layout.
pub fn describe(image: &[u8]) -> Option<String> {
    let l = find_dynamic(image)?;
    let mut needed = Vec::new();
    let mut off = l.dyn_off;
    let end = l.dyn_off + l.dyn_len;
    while off + 16 <= end {
        let tag = u64_at(image, off)?;
        let val = u64_at(image, off + 8)?;
        if tag == DT_NULL {
            break;
        }
        if tag == DT_NEEDED {
            let s = l.dynstr_off + val as usize;
            let e = image[s..].iter().position(|&b| b == 0)? + s;
            needed.push(String::from_utf8_lossy(&image[s..e]).into_owned());
        }
        off += 16;
    }
    Some(format!(
        ".dynamic @ 0x{:X} ({}), .dynstr @ 0x{:X} ({}), deps: {}",
        l.dyn_off,
        l.dyn_len,
        l.dynstr_off,
        l.dynstr_len,
        needed.join(", ")
    ))
}
