//! Minimal ELF64 little-endian program-header reader.
//!
//! Only what the scanner needs: the PT_LOAD segments so file offsets can be
//! converted to virtual addresses. No unsafe, no dependencies.

#![forbid(unsafe_code)]

use core::fmt;

/// One PT_LOAD segment mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
}

#[derive(Debug)]
pub enum ElfError {
    NotElf,
    NotElf64Le,
    Truncated,
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfError::NotElf => write!(f, "not an ELF file"),
            ElfError::NotElf64Le => {
                write!(f, "only ELF64 little-endian is supported (Android/ARM64)")
            }
            ElfError::Truncated => write!(f, "ELF headers truncated"),
        }
    }
}

fn u16_at(d: &[u8], off: usize) -> Option<u16> {
    let b = d.get(off..off + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
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

/// Read the PT_LOAD segments of an ELF64-LE image.
pub fn read_elf_segments(data: &[u8]) -> Option<Vec<Segment>> {
    let ident = data.get(0..16)?;
    if &ident[0..4] != b"\x7fELF" {
        return None;
    }
    // ei_class == 2 (ELF64), ei_data == 1 (little endian)
    if ident[4] != 2 || ident[5] != 1 {
        return None;
    }

    // ELF64 header layout:
    //   e_phoff   u64 @ 0x20
    //   e_phentsize u16 @ 0x36
    //   e_phnum   u16 @ 0x38
    let phoff = u64_at(data, 0x20)? as usize;
    let phentsize = u16_at(data, 0x36)? as usize;
    let phnum = u16_at(data, 0x38)? as usize;
    if phentsize < 56 || phnum == 0 {
        return None;
    }

    let mut segments = Vec::with_capacity(phnum.min(64));
    for i in 0..phnum {
        let base = phoff.checked_add(i * phentsize)?;
        let p_type = u32_at(data, base)?;
        if p_type != 1 {
            continue; // PT_LOAD only
        }
        // Elf64_Phdr: p_type u32, p_flags u32, p_offset u64, p_vaddr u64,
        //             p_paddr u64, p_filesz u64, p_memsz u64, p_align u64
        let offset = u64_at(data, base + 8)?;
        let vaddr = u64_at(data, base + 16)?;
        let filesz = u64_at(data, base + 32)?;
        segments.push(Segment {
            offset,
            vaddr,
            filesz,
        });
    }
    Some(segments)
}

/// Convert a file offset to a virtual address using the PT_LOAD mapping.
pub fn off_to_va(segs: &[Segment], off: usize) -> Option<u64> {
    let off = off as u64;
    for s in segs {
        if s.offset <= off && off < s.offset + s.filesz {
            return Some(off - s.offset + s.vaddr);
        }
    }
    None
}

/// Convert a virtual address back to a file offset.
pub fn va_to_off(segs: &[Segment], va: u64) -> Option<usize> {
    for s in segs {
        if s.vaddr <= va && va < s.vaddr + s.filesz {
            return Some((va - s.vaddr + s.offset) as usize);
        }
    }
    None
}
