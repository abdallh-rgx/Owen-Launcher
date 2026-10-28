//! DT_NEEDED injection for ELF64 shared objects.
//!
//! Adds `libOwen.so` to the dynamic dependency list of a native library so
//! the Android linker loads it at process start — the smali-free way to
//! get the redirecter into the game.
//!
//! Strategy (in order of preference):
//! 1. **Padding inside `.dynstr`** — a NUL run in the string table's own
//!    file range. Zero collateral.
//! 2. **Inter-section alignment gaps** — verified against the section
//!    header table: the bytes between the end of one section and the
//!    start of the next. Overwriting alignment padding never touches
//!    live data. The DT_NEEDED value points past `DT_STRSZ` — bionic's
//!    linker does not bounds-check string offsets (verified against real
//!    libUE4.so/libUnreal.so layouts).
//! 3. Fail loudly when neither exists.
//!
//! The new entry reuses the first spare `DT_NULL` slot of `PT_DYNAMIC`
//! (the dynamic array always keeps trailing NULLs for exactly this).
//!
//! Everything is plain byte surgery on a `Vec<u8>` — safe Rust.

use owen_core::elf::{ElfImage, DT_NEEDED, DT_NULL, DT_STRTAB, DT_STRSZ};

/// Result of an injection attempt.
pub struct InjectResult {
    /// The (possibly modified) library image.
    pub data: Vec<u8>,
    /// True when the string was newly added.
    pub injected: bool,
}

/// Inject `lib_name` into the DT_NEEDED list. Returns the image unchanged
/// with `injected=false` when the dependency already exists.
pub fn inject_needed(lib: &[u8], lib_name: &str) -> Result<(Vec<u8>, bool), String> {
    let elf = ElfImage::parse(lib).map_err(|e| e.to_string())?;

    // Already needed?
    if elf.needed_libraries_from(lib).iter().any(|n| n == lib_name) {
        return Ok((lib.to_vec(), false));
    }

    let mut data = lib.to_vec();

    // ---- locate PT_DYNAMIC through the program headers ----
    let phoff = rd_u64(&data, 0x20).ok_or("bad phoff".to_string())? as usize;
    let phentsize = rd_u16(&data, 0x36).ok_or("bad phentsize".to_string())? as usize;
    let phnum = rd_u16(&data, 0x38).ok_or("bad phnum".to_string())? as usize;
    let mut dyn_off = None;
    let mut dyn_size = 0usize;
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if ph + 56 > data.len() {
            break;
        }
        let p_type = rd_u32(&data, ph).ok_or("bad ph".to_string())?;
        if p_type == 2 {
            // PT_DYNAMIC: p_offset at +8, p_filesz at +32.
            dyn_off = Some(rd_u64(&data, ph + 8).ok_or("bad dyn off".to_string())? as usize);
            dyn_size = rd_u64(&data, ph + 32).unwrap_or(0) as usize;
            break;
        }
    }
    let dyn_off = dyn_off.ok_or("no PT_DYNAMIC")?;

    // ---- DT_STRTAB / DT_STRSZ / spare DT_NULL ----
    // (collected first so the strsz slot can be updated when the string
    // lands in a section gap past the original strtab end)
    let mut strtab_va = None;
    let mut strsz = 0u64;
    let mut null_slot = None;
    let mut strsz_slot = None;
    let entries = dyn_size / 16;
    for i in 0..entries {
        let off = dyn_off + i * 16;
        if off + 16 > data.len() {
            break;
        }
        let tag = rd_u64(&data, off).ok_or("bad dyn tag".to_string())? as i64;
        let val = rd_u64(&data, off + 8).ok_or("bad dyn val".to_string())?;
        match tag {
            DT_STRTAB => strtab_va = Some(val),
            DT_STRSZ => {
                strsz = val;
                strsz_slot = Some(off + 8);
            }
            DT_NULL => {
                null_slot = Some(off);
                break;
            }
            _ => {}
        }
    }
    let strtab_va = strtab_va.ok_or("no DT_STRTAB")?;
    let strtab_off = elf
        .va_to_off(strtab_va)
        .ok_or("DT_STRTAB outside PT_LOAD".to_string())? as usize;
    let strtab_end = strtab_off + strsz as usize;

    let name_bytes = lib_name.as_bytes();
    let need = name_bytes.len() + 1;

    // ---- placement: 1) padding inside the strtab range ----
    let placement = find_nul_run(&data[strtab_off..strtab_end], need).map(|rel| {
        let at = strtab_off + rel;
        (at, at as u64 - strtab_off as u64)
    });

    // ---- 2) inter-section alignment gaps after the strtab ----
    let placement = placement.or_else(|| {
        let gaps = section_gaps_after(&data, strtab_end);
        for (start, end) in gaps {
            if end > start && end - start >= need {
                let at = start;
                // The gap must be inside a PT_LOAD (so it is mapped) and
                // its VA must be resolvable.
                if let Some(va) = elf.off_to_va(at as u64) {
                    if va >= strtab_va + strsz {
                        return Some((at, va - strtab_va));
                    }
                }
            }
        }
        None
    });

    let Some((at, strtab_delta)) = placement else {
        return Err("no safe padding found for the dependency name (strtab and section gaps are full)".into());
    };

    // ---- write the string ----
    data[at..at + name_bytes.len()].copy_from_slice(name_bytes);
    data[at + name_bytes.len()] = 0;

    // ---- extend DT_STRSZ so the name is inside the declared strtab ----
    // (bionic bounds-checks string offsets on modern Android; the strtab
    // merely grows to cover the alignment gap it now spans)
    let new_strsz = strtab_delta + need as u64;
    if new_strsz > strsz {
        if let Some(slot) = strsz_slot {
            write_u64(&mut data, slot, new_strsz);
        }
    }

    // ---- rewrite the first DT_NULL as DT_NEEDED ----
    let null_off = null_slot.ok_or("no spare DT_NULL slot in PT_DYNAMIC")?;
    write_u64(&mut data, null_off, DT_NEEDED as u64);
    write_u64(&mut data, null_off + 8, strtab_delta);

    // ---- verification ----
    let check = ElfImage::parse(&data).map_err(|e| format!("patched image broken: {e}"))?;
    if !check.needed_libraries_from(&data).iter().any(|n| n == lib_name) {
        return Err("verification failed: name not visible after patch".into());
    }

    Ok((data, true))
}

/// Compute the alignment gaps between sections that start after
/// `after_off`. Only gaps between two adjacent non-NOBITS sections count.
fn section_gaps_after(data: &[u8], after_off: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let Some(e_shoff) = rd_u64(data, 0x28).map(|v| v as usize) else {
        return out;
    };
    let (Some(e_shentsize), Some(e_shnum)) = (rd_u16(data, 0x3A), rd_u16(data, 0x3C)) else {
        return out;
    };
    let mut sections: Vec<(usize, usize)> = Vec::new(); // (offset, offset+size)
    for i in 0..e_shnum as usize {
        let sh = e_shoff + i * e_shentsize as usize;
        if sh + 64 > data.len() {
            break;
        }
        let sh_type = rd_u32(data, sh + 4).unwrap_or(0);
        let sh_offset = rd_u64(data, sh + 24).unwrap_or(0) as usize;
        let sh_size = rd_u64(data, sh + 32).unwrap_or(0) as usize;
        if sh_type == 8 || sh_offset == 0 {
            continue; // SHT_NOBITS / empty
        }
        if sh_offset + sh_size <= data.len() {
            sections.push((sh_offset, sh_offset + sh_size));
        }
    }
    sections.sort();
    for w in sections.windows(2) {
        let (_a_start, a_end) = w[0];
        let (b_start, _) = w[1];
        if a_end > after_off && b_start > a_end {
            out.push((a_end.max(after_off), b_start));
        }
    }
    out
}

fn find_nul_run(buf: &[u8], len: usize) -> Option<usize> {
    if buf.len() < len {
        return None;
    }
    let mut run = 0usize;
    for (i, &b) in buf.iter().enumerate().rev() {
        if b == 0 {
            run += 1;
            if run >= len {
                return Some(i);
            }
        } else {
            run = 0;
        }
    }
    None
}

fn rd_u16(b: &[u8], off: usize) -> Option<u16> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn rd_u32(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn rd_u64(b: &[u8], off: usize) -> Option<u64> {
    b.get(off..off + 8)
        .map(|s| u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
}

fn write_u64(b: &mut [u8], off: usize, v: u64) {
    b[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
