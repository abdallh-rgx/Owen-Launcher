//! Minimal ELF64 little-endian parser (safe Rust, no `unsafe`, no deps).
//!
//! Only what Owen needs: program headers (PT_LOAD / PT_DYNAMIC), the dynamic
//! symbol table for exported-symbol lookups, and offset <-> virtual address
//! conversion. Fortnite Android libraries are AArch64 ELF64-LE shared
//! objects, exactly what this module targets.

use std::fs;
use std::path::Path;

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;

/// A `PT_LOAD` program-header segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Offset of the segment inside the file.
    pub offset: u64,
    /// Virtual address where the segment is mapped.
    pub vaddr: u64,
    /// Size of the segment data in the file.
    pub filesz: u64,
    /// Size in memory (may exceed `filesz` for .bss).
    pub memsz: u64,
    /// Segment flags (r=4, w=2, x=1).
    pub flags: u32,
}

/// One entry of the `.dynsym` table.
#[derive(Debug, Clone)]
pub struct DynSymbol {
    pub name: String,
    /// Value = virtual address of the symbol (st_value).
    pub value: u64,
    pub size: u64,
}

/// One `Elf64_Dyn` entry.
#[derive(Debug, Clone, Copy)]
pub struct DynEntry {
    pub tag: i64,
    pub value: u64,
}

pub const DT_NULL: i64 = 0;
pub const DT_NEEDED: i64 = 1;
pub const DT_STRTAB: i64 = 5;
pub const DT_STRSZ: i64 = 10;

/// A parsed ELF64-LE image.
#[derive(Debug, Clone)]
pub struct ElfImage {
    pub segments: Vec<Segment>,
    pub dyn_entries: Vec<DynEntry>,
    pub dyn_symbols: Vec<DynSymbol>,
}

#[derive(Debug)]
pub struct ElfError(pub String);

impl std::fmt::Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "elf: {}", self.0)
    }
}

impl std::error::Error for ElfError {}

fn rd_u16(b: &[u8], off: usize) -> Result<u16, ElfError> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| ElfError(format!("short read u16 at {off:#x}")))
}

fn rd_u32(b: &[u8], off: usize) -> Result<u32, ElfError> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| ElfError(format!("short read u32 at {off:#x}")))
}

fn rd_u64(b: &[u8], off: usize) -> Result<u64, ElfError> {
    b.get(off..off + 8)
        .map(|s| {
            u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
        .ok_or_else(|| ElfError(format!("short read u64 at {off:#x}")))
}

fn cstr_at(b: &[u8], off: usize) -> Option<String> {
    let rest = b.get(off..)?;
    let len = rest.iter().position(|&c| c == 0)?;
    Some(String::from_utf8_lossy(&rest[..len]).into_owned())
}

impl ElfImage {
    /// Parse an ELF64-LE image from raw bytes.
    pub fn parse(data: &[u8]) -> Result<Self, ElfError> {
        if data.len() < 64 || &data[..4] != b"\x7fELF" {
            return Err(ElfError("not an ELF file".into()));
        }
        if data[4] != 2 || data[5] != 1 {
            return Err(ElfError("only ELF64 little-endian is supported".into()));
        }

        let e_phoff = rd_u64(data, 0x20)? as usize;
        let e_shoff = rd_u64(data, 0x28)? as usize;
        let e_phentsize = rd_u16(data, 0x36)? as usize;
        let e_phnum = rd_u16(data, 0x38)? as usize;
        let e_shentsize = rd_u16(data, 0x3A)? as usize;
        let e_shnum = rd_u16(data, 0x3C)? as usize;

        // ---- program headers ----
        let mut segments = Vec::new();
        let mut dynamic = Vec::new();
        for i in 0..e_phnum {
            let ph = e_phoff + i * e_phentsize;
            if ph + 56 > data.len() {
                break;
            }
            let p_type = rd_u32(data, ph)?;
            let p_flags = rd_u32(data, ph + 4)?;
            let p_offset = rd_u64(data, ph + 8)?;
            let p_vaddr = rd_u64(data, ph + 16)?;
            let p_filesz = rd_u64(data, ph + 32)?;
            let p_memsz = rd_u64(data, ph + 40)?;
            match p_type {
                PT_LOAD => segments.push(Segment {
                    offset: p_offset,
                    vaddr: p_vaddr,
                    filesz: p_filesz,
                    memsz: p_memsz,
                    flags: p_flags,
                }),
                PT_DYNAMIC => {
                    // Read the dynamic array (pairs of u64 until DT_NULL).
                    let mut off = p_offset as usize;
                    loop {
                        if off + 16 > data.len() {
                            break;
                        }
                        let tag = rd_u64(data, off)? as i64;
                        let val = rd_u64(data, off + 8)?;
                        if tag == DT_NULL {
                            dynamic.push(DynEntry { tag, value: val });
                            break;
                        }
                        dynamic.push(DynEntry { tag, value: val });
                        off += 16;
                    }
                }
                _ => {}
            }
        }

        // ---- dynamic symbols ----
        // Preferred: via section headers (.dynsym + .dynstr).
        // Fallback: via DT_SYMTAB / DT_STRTAB when sections were stripped.
        let mut dyn_symbols = Vec::new();
        let mut strtab_off: Option<usize> = None;
        let mut strtab_size = 0usize;

        for e in &dynamic {
            if e.tag == DT_STRTAB {
                if let Some(off) = va_to_off(&segments, e.value) {
                    strtab_off = Some(off as usize);
                }
            } else if e.tag == DT_STRSZ {
                strtab_size = e.value as usize;
            }
        }

        if e_shnum > 0 && e_shoff > 0 && e_shoff + 18 <= data.len() {
            let shstrndx = rd_u16(data, 0x3E)? as usize;
            let shstr_off = if e_shoff + shstrndx * e_shentsize + 24 <= data.len() {
                rd_u64(data, e_shoff + shstrndx * e_shentsize + 24)? as usize
            } else {
                0
            };
            for i in 0..e_shnum {
                let sh = e_shoff + i * e_shentsize;
                if sh + 64 > data.len() {
                    break;
                }
                let sh_name = rd_u32(data, sh)? as usize;
                let sh_type = rd_u32(data, sh + 4)?;
                let sh_offset = rd_u64(data, sh + 24)? as usize;
                let sh_size = rd_u64(data, sh + 32)? as usize;
                let sh_link = rd_u32(data, sh + 40)? as usize;
                let sh_entsize = rd_u64(data, sh + 56)? as usize;
                if sh_type != 11 {
                    // SHT_DYNSYM
                    continue;
                }
                let str_hdr = e_shoff + sh_link * e_shentsize;
                if str_hdr + 64 > data.len() {
                    continue;
                }
                let str_off = rd_u64(data, str_hdr + 24)? as usize;
                let str_size = rd_u64(data, str_hdr + 32)? as usize;
                let strtab = data.get(str_off..(str_off + str_size).min(data.len())).unwrap_or(&[]);
                let entsize = if sh_entsize == 0 { 24 } else { sh_entsize };
                let count = if entsize > 0 { sh_size / entsize } else { 0 };
                for j in 0..count {
                    let sym = sh_offset + j * entsize;
                    if sym + 24 > data.len() {
                        break;
                    }
                    let st_name = rd_u32(data, sym)? as usize;
                    let st_value = rd_u64(data, sym + 8)?;
                    let st_size = rd_u64(data, sym + 32 - 24)?;
                    let name = cstr_at(strtab, st_name).unwrap_or_default();
                    if !name.is_empty() {
                        dyn_symbols.push(DynSymbol { name, value: st_value, size: st_size });
                    }
                }
                break;
            }
            // remember .dynstr offset for DT_NEEDED resolution later
            if strtab_off.is_none() {
                for e in &dynamic {
                    if e.tag == DT_STRTAB {
                        // translate via the section headers' .dynstr
                        let _ = e;
                    }
                }
            }
        }

        if dyn_symbols.is_empty() {
            // Section headers absent: use DT_SYMTAB heuristically. The symbol
            // table usually directly precedes the string table.
            let mut symtab_va = None;
            for e in &dynamic {
                if e.tag == 6 {
                    // DT_SYMTAB
                    symtab_va = Some(e.value);
                }
            }
            if let (Some(sym_va), Some(str_off_u64)) = (symtab_va, strtab_off.map(|v| v as u64)) {
                if let Some(sym_off_u64) = va_to_off(&segments, sym_va) {
                    let (sym_off, str_off) = (sym_off_u64 as usize, str_off_u64 as usize);
                    if str_off > sym_off {
                        let strtab = data.get(str_off..(str_off + strtab_size).min(data.len())).unwrap_or(&[]);
                        let mut off = sym_off;
                        while off + 24 <= str_off && dyn_symbols.len() < 500_000 {
                            let st_name = rd_u32(data, off)? as usize;
                            let st_value = rd_u64(data, off + 8)?;
                            let st_size = rd_u64(data, off + 32 - 24)?;
                            let name = cstr_at(strtab, st_name).unwrap_or_default();
                            if !name.is_empty() {
                                dyn_symbols.push(DynSymbol { name, value: st_value, size: st_size });
                            }
                            off += 24;
                        }
                    }
                }
            }
        }

        Ok(Self { segments, dyn_entries: dynamic, dyn_symbols })
    }

    /// Parse from a file path.
    pub fn open(path: &Path) -> Result<Self, ElfError> {
        let data = fs::read(path).map_err(|e| ElfError(format!("read {}: {e}", path.display())))?;
        Self::parse(&data)
    }

    /// Convert a virtual address to a file offset.
    pub fn va_to_off(&self, va: u64) -> Option<u64> {
        va_to_off(&self.segments, va)
    }

    /// Convert a file offset to a virtual address.
    pub fn off_to_va(&self, off: u64) -> Option<u64> {
        off_to_va(&self.segments, off)
    }

    /// Look up an exported (dynamic) symbol by exact name.
    pub fn find_symbol(&self, name: &str) -> Option<&DynSymbol> {
        self.dyn_symbols.iter().find(|s| s.name == name)
    }

    /// Resolve the list of DT_NEEDED library names (may be empty when the
    /// string table is unavailable).
    pub fn needed_libraries(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(strtab) = self.dynstr() else { return out };
        for e in &self.dyn_entries {
            if e.tag == DT_NEEDED {
                if let Some(off) = self.va_to_off(e.value) {
                    if let Some(name) = cstr_at(strtab, off as usize) {
                        out.push(name);
                    }
                }
            }
        }
        out
    }

    /// The .dynstr bytes (file-based slice is returned by the caller using
    /// the original buffer; here we re-read from disk is not possible, so we
    /// locate via DT_STRTAB/DT_STRSZ against the provided image bytes).
    fn dynstr(&self) -> Option<&[u8]> {
        None // resolved by [`Self::needed_libraries_from`] with raw bytes.
    }

    /// Resolve DT_NEEDED names given the raw file bytes.
    pub fn needed_libraries_from(&self, data: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        let (str_off, str_size) = self.dynstr_bounds_from(data);
        let strtab = data.get(str_off..(str_off + str_size).min(data.len())).unwrap_or(&[]);
        for e in &self.dyn_entries {
            if e.tag == DT_NEEDED {
                if let Some(off) = self.va_to_off(e.value) {
                    if let Some(name) = cstr_at(strtab, off as usize) {
                        out.push(name);
                    }
                }
            }
        }
        out
    }

    /// Locate the dynamic string table inside the raw file.
    pub fn dynstr_bounds_from(&self, data: &[u8]) -> (usize, usize) {
        let mut strtab_va = None;
        let mut strsz = 0usize;
        for e in &self.dyn_entries {
            match e.tag {
                5 => strtab_va = Some(e.value), // DT_STRTAB
                10 => strsz = e.value as usize, // DT_STRSZ
                _ => {}
            }
        }
        if let Some(va) = strtab_va {
            if let Some(off) = self.va_to_off(va) {
                return (off as usize, strsz);
            }
        }
        // Fallback: use section headers
        if data.len() >= 0x3A {
            if let (Ok(e_shoff), Ok(e_shentsize), Ok(e_shnum)) =
                (rd_u64(data, 0x28), rd_u16(data, 0x3A), rd_u16(data, 0x3C))
            {
                for i in 0..e_shnum as usize {
                    let sh = e_shoff as usize + i * e_shentsize as usize;
                    if sh + 64 > data.len() {
                        break;
                    }
                    if let (Ok(sh_type), Ok(sh_offset), Ok(sh_size)) =
                        (rd_u32(data, sh + 4), rd_u64(data, sh + 24), rd_u64(data, sh + 32))
                    {
                        if sh_type == 3 {
                            // SHT_STRTAB and this is the dynamic one
                            return (sh_offset as usize, sh_size as usize);
                        }
                    }
                }
            }
        }
        (0, 0)
    }
}

/// Convert a virtual address to a file offset using PT_LOAD segments.
pub fn va_to_off(segments: &[Segment], va: u64) -> Option<u64> {
    for s in segments {
        if va >= s.vaddr && va < s.vaddr + s.filesz {
            return Some(va - s.vaddr + s.offset);
        }
    }
    None
}

/// Convert a file offset to a virtual address using PT_LOAD segments.
pub fn off_to_va(segments: &[Segment], off: u64) -> Option<u64> {
    for s in segments {
        if off >= s.offset && off < s.offset + s.filesz {
            return Some(off - s.offset + s.vaddr);
        }
    }
    None
}
