//! Unreal Engine pak file index parser (safe Rust).
//!
//! Supports the pak versions Fortnite mobile shipped with:
//!
//! * **v3 - v9 (legacy)**: mount point + entry count + entries with inline
//!   filenames.
//! * **v10+ (PathHashIndex era)**: primary index (mount point, seed,
//!   PathHashIndex / FullDirectoryIndex pointers, `EncodedPakEntries`,
//!   decoded `Files` array) + FullDirectoryIndex directory tree with
//!   `FPakEntryLocation` references.
//!
//! Indexes encrypted with AES-256-ECB (every Fortnite pak from ~v5 on)
//! are decrypted when a key is supplied — the format follows the
//! reference implementations (UEViewer / UE4 `PakFile.cpp`):
//!
//! ```text
//! Encoded entry:  u32 bitfield | [u32 block_size] | offset(u32/u64) |
//!                 uncompressed(u32/u64) | size(u32/u64, compressed only) |
//!                 blocks(16B each)
//! bitfield:  0x8000_0000 offset-32   0x4000_0000 usz-32
//!            0x2000_0000 size-32     bits 23..29 method
//!            bit 22 encrypted        bits 6..22 block count
//!            bits 0..6 block-size/2048 (0x3f = explicit u32)
//! ```

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub const PAK_MAGIC: u32 = 0x5A6F_12E1;

/// Compression method names used by Fortnite paks.
pub const COMPRESSION_METHODS: [&str; 5] = ["None", "Zlib", "Gzip", "Custom", "Oodle"];

const MAX_ENTRIES: i64 = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakInfo {
    pub version: u32,
    pub index_offset: u64,
    pub index_size: u64,
    #[serde(serialize_with = "hex20", default)]
    pub index_hash: [u8; 20],
}

fn hex20<S: serde::Serializer>(h: &[u8; 20], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&h.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakEntry {
    /// Full in-pak path: mount + directory + filename.
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub uncompressed_size: u64,
    pub compression_method: u32,
    pub compression_method_name: String,
    pub encrypted: bool,
    pub compression_blocks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakIndex {
    pub mount_point: String,
    pub version: u32,
    pub entries: Vec<PakEntry>,
    pub encrypted: bool,
    pub decrypted: bool,
}

#[derive(Debug)]
pub enum PakError {
    Io(std::io::Error),
    NotPak,
    UnsupportedVersion(u32),
    EncryptedIndex,
    BadIndex(&'static str),
    BadKey,
}

impl std::fmt::Display for PakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PakError::Io(e) => write!(f, "io error: {e}"),
            PakError::NotPak => write!(f, "not a pak file (bad magic)"),
            PakError::UnsupportedVersion(v) => write!(f, "unsupported pak version {v}"),
            PakError::EncryptedIndex => write!(f, "pak index is AES encrypted (supply the season key)"),
            PakError::BadIndex(w) => write!(f, "corrupt index: {w}"),
            PakError::BadKey => write!(f, "decryption with the supplied key failed (wrong key?)"),
        }
    }
}

impl From<std::io::Error> for PakError {
    fn from(e: std::io::Error) -> Self {
        PakError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// AES-256-ECB decryption (safe API over the `aes` crate block cipher)
// ---------------------------------------------------------------------------

fn aes256_ecb_decrypt(data: &mut [u8], key: &[u8; 32]) -> Result<(), PakError> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockDecrypt, KeyInit};
    if data.len() % 16 != 0 {
        return Err(PakError::BadKey);
    }
    let cipher = aes::Aes256::new(GenericArray::from_slice(key));
    for chunk in data.chunks_exact_mut(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        chunk.copy_from_slice(&block);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reader over the pak file
// ---------------------------------------------------------------------------

struct Cursor<R: Read> {
    inner: R,
}

impl<R: Read> Cursor<R> {
    fn bytes(&mut self, n: usize) -> Result<Vec<u8>, PakError> {
        let mut b = vec![0u8; n];
        self.inner.read_exact(&mut b).map_err(PakError::Io)?;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, PakError> {
        Ok(self.bytes(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, PakError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, PakError> {
        Ok(self.u32()? as i32)
    }
    fn u64(&mut self) -> Result<u64, PakError> {
        Ok(u64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, PakError> {
        Ok(self.u64()? as i64)
    }
    fn guid20(&mut self) -> Result<[u8; 20], PakError> {
        let b = self.bytes(20)?;
        Ok(b.try_into().unwrap())
    }
    /// FString: i32 length (negative = UTF-16), then bytes.
    fn fstring(&mut self) -> Result<String, PakError> {
        let len = self.i32()?;
        if len == 0 {
            return Ok(String::new());
        }
        if len < 0 {
            let n = (-len) as usize * 2;
            let b = self.bytes(n)?;
            let units: Vec<u16> = b
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&u| u != 0)
                .collect();
            return Ok(String::from_utf16_lossy(&units));
        }
        let b = self.bytes(len as usize)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        if !b[..end].is_ascii() {
            // Likely binary garbage: the index is encrypted or the parse is
            // misaligned.
            return Err(PakError::BadIndex("non-ascii filename"));
        }
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }
}

/// Read the FPakInfo trailer. The footer size differs across versions and
/// Fortnite builds (204/205/221/225/44/53 bytes); instead of guessing, the
/// magic is scanned backwards from the end of the file.
pub fn read_pak_info<R: Read + Seek>(r: &mut R) -> Result<PakInfo, PakError> {
    let file_len = r.seek(SeekFrom::End(0))?;
    if file_len < 64 {
        return Err(PakError::NotPak);
    }
    let window = (file_len.min(512)) as usize;
    r.seek(SeekFrom::Start(file_len - window as u64))?;
    let tail = {
        let mut buf = vec![0u8; window];
        r.read_exact(&mut buf).map_err(PakError::Io)?;
        buf
    };
    // Scan backwards on 4-byte alignment for the magic.
    let base = file_len as usize - window;
    let mut magic_at = None;
    let start = window.saturating_sub(4);
    let mut i = (start / 4) * 4;
    loop {
        if i + 4 <= window
            && u32::from_le_bytes([tail[i], tail[i + 1], tail[i + 2], tail[i + 3]]) == PAK_MAGIC
        {
            magic_at = Some(base + i);
            break;
        }
        if i < 4 {
            break;
        }
        i -= 4;
    }
    let at = magic_at.ok_or(PakError::NotPak)?;
    r.seek(SeekFrom::Start(at as u64))?;
    let mut c = Cursor { inner: r };
    let _magic = c.u32()?;
    let version = c.u32()?;
    let index_offset = c.u64()?;
    let index_size = c.u64()?;
    let index_hash = c.guid20()?;
    if !(1..=20).contains(&version) {
        return Err(PakError::UnsupportedVersion(version));
    }
    if index_offset == 0 || index_size == 0 || index_offset + index_size > file_len + 1024 {
        return Err(PakError::BadIndex("implausible index bounds"));
    }
    Ok(PakInfo {
        version,
        index_offset,
        index_size,
        index_hash,
    })
}

/// Parse the full index of a pak file, decrypting with `key` when needed.
pub fn read_pak_index<R: Read + Seek>(
    r: &mut R,
    key: Option<&[u8; 32]>,
) -> Result<PakIndex, PakError> {
    let info = read_pak_info(r)?;
    r.seek(SeekFrom::Start(info.index_offset))?;
    let mut raw = {
        let mut buf = vec![0u8; info.index_size as usize];
        r.read_exact(&mut buf).map_err(PakError::Io)?;
        buf
    };

    if info.version < 10 {
        // Legacy index: try plaintext, then decrypted.
        if let Ok(idx) = parse_legacy_index(&raw, info.version) {
            return Ok(idx);
        }
        if let Some(k) = key {
            aes256_ecb_decrypt(&mut raw, k).map_err(|_| PakError::BadKey)?;
            return parse_legacy_index(&raw, info.version);
        }
        return Err(PakError::EncryptedIndex);
    }

    // v10+: primary index (plaintext first, then decrypted).
    let mut decrypted = false;
    let mut primary = match parse_primary_index(&raw) {
        Ok(p) => p,
        Err(_) => {
            let Some(k) = key else {
                return Err(PakError::EncryptedIndex);
            };
            aes256_ecb_decrypt(&mut raw, k).map_err(|_| PakError::BadKey)?;
            decrypted = true;
            parse_primary_index(&raw)?
        }
    };

    // FullDirectoryIndex.
    let fdi_offset = primary.fdi_offset;
    let fdi_size = primary.fdi_size;
    if fdi_offset == 0 || fdi_size == 0 {
        return Err(PakError::BadIndex("no full directory index"));
    }
    r.seek(SeekFrom::Start(fdi_offset))?;
    let mut fdi_raw = {
        let mut buf = vec![0u8; fdi_size as usize];
        r.read_exact(&mut buf).map_err(PakError::Io)?;
        buf
    };
    let fdi = match parse_fdi(&fdi_raw, &primary.mount_point) {
        Ok(f) => f,
        Err(_) => {
            let Some(k) = key else {
                return Err(PakError::EncryptedIndex);
            };
            aes256_ecb_decrypt(&mut fdi_raw, k).map_err(|_| PakError::BadKey)?;
            parse_fdi(&fdi_raw, &primary.mount_point)?
        }
    };

    // Resolve entries.
    let mut entries = Vec::with_capacity(fdi.len());
    for (path, location) in fdi {
        let entry = if location >= 0 {
            decode_entry_at(&primary.encoded_entries, location as usize, &path)?
        } else {
            let idx = (-(location as i64) - 1) as usize;
            primary
                .files
                .get(idx)
                .ok_or(PakError::BadIndex("file index out of range"))?
                .to_entry(&path)
        };
        entries.push(entry);
    }

    Ok(PakIndex {
        mount_point: primary.mount_point.clone(),
        version: info.version,
        entries,
        encrypted: true,
        decrypted,
    })
}

// ---------------------------------------------------------------------------
// v10+ primary index
// ---------------------------------------------------------------------------

struct PrimaryIndex {
    mount_point: String,
    #[allow(dead_code)]
    seed: u64,
    fdi_offset: u64,
    fdi_size: u64,
    encoded_entries: Vec<u8>,
    files: Vec<LegacyEntry>,
}

struct LegacyEntry {
    offset: u64,
    size: u64,
    uncompressed_size: u64,
    compression_method: u32,
    encrypted: bool,
    blocks: u32,
}

impl LegacyEntry {
    fn to_entry(&self, name: &str) -> PakEntry {
        PakEntry {
            name: name.to_string(),
            offset: self.offset,
            size: self.size,
            uncompressed_size: self.uncompressed_size,
            compression_method: self.compression_method,
            compression_method_name: method_name(self.compression_method).to_string(),
            encrypted: self.encrypted,
            compression_blocks: self.blocks,
        }
    }
}

fn method_name(m: u32) -> &'static str {
    match m {
        0 => "None",
        1 => "Zlib",
        2 => "Gzip",
        3 => "Custom",
        4 => "Oodle",
        _ => "Unknown",
    }
}

fn parse_primary_index(data: &[u8]) -> Result<PrimaryIndex, PakError> {
    let mut c = Cursor {
        inner: std::io::Cursor::new(data),
    };
    let mount_point = c.fstring()?;
    if mount_point.len() > 260 || mount_point.contains('\u{fffd}') {
        return Err(PakError::BadIndex("implausible mount point"));
    }
    let seed = c.u64()?;
    // Fortnite builds carry 4 extra zero bytes between the seed and the
    // first flag. Accept both alignments.
    let mut probe = c.i32()?;
    if probe == 0 {
        probe = c.i32()?; // re-align past the licensee pad
    }
    let mut fdi_offset = 0u64;
    let mut fdi_size = 0u64;
    // PathHashIndex block
    if probe != 0 {
        let _phi_off = c.u64()?;
        let _phi_size = c.u64()?;
        let _phi_hash = c.guid20()?;
    }
    // FullDirectoryIndex block
    let has_fdi = c.i32()?;
    if has_fdi != 0 {
        fdi_offset = c.u64()?;
        fdi_size = c.u64()?;
        let _fdi_hash = c.guid20()?;
    }
    // EncodedPakEntries: TArray<u8>
    let n = c.i32()?;
    if n < 0 || n as usize > data.len() {
        return Err(PakError::BadIndex("bad EncodedPakEntries size"));
    }
    let encoded_entries = c.bytes(n as usize)?;
    // Files: TArray<FPakEntry> (legacy serialization)
    let file_count = c.i32()?;
    if file_count < 0 || file_count as i64 > MAX_ENTRIES {
        return Err(PakError::BadIndex("bad Files count"));
    }
    let mut files = Vec::with_capacity(file_count.min(100_000) as usize);
    for _ in 0..file_count {
        let offset = c.u64()?;
        let size = c.u64()?;
        let usz = c.u64()?;
        let method = c.u32()?;
        let _hash = c.guid20()?;
        let mut blocks = 0u32;
        if method != 0 {
            let n = c.i32()?;
            if n < 0 || n > 100_000 {
                return Err(PakError::BadIndex("bad block count"));
            }
            for _ in 0..n {
                c.u64()?;
                c.u64()?;
            }
            blocks = n as u32;
        }
        let encrypted = c.u8()? != 0;
        let _block_size = c.u32()?;
        files.push(LegacyEntry {
            offset,
            size,
            uncompressed_size: usz,
            compression_method: method,
            encrypted,
            blocks,
        });
    }
    Ok(PrimaryIndex {
        mount_point,
        seed,
        fdi_offset,
        fdi_size,
        encoded_entries,
        files,
    })
}

type DirListing = Vec<(String, i32)>;

fn parse_fdi(data: &[u8], mount_point: &str) -> Result<DirListing, PakError> {
    let mut c = Cursor {
        inner: std::io::Cursor::new(data),
    };
    let dir_count = c.i32()?;
    if dir_count < 0 || dir_count > 100_000 {
        return Err(PakError::BadIndex("bad directory count"));
    }
    let mut out: DirListing = Vec::new();
    let mount = mount_point.trim_end_matches('/');
    for _ in 0..dir_count {
        let dir = c.fstring()?;
        let file_count = c.i32()?;
        if file_count < 0 || file_count as i64 > MAX_ENTRIES {
            return Err(PakError::BadIndex("bad file count"));
        }
        for _ in 0..file_count {
            let file = c.fstring()?;
            let location = c.i32()?;
            if location == i32::MAX || location == i32::MIN {
                continue;
            }
            let full = normalize_pak_path(&format!("{mount}/{dir}/{file}"));
            out.push((full, location));
        }
    }
    Ok(out)
}

/// Collapse `../../../`, duplicate slashes and dot segments.
pub fn normalize_pak_path(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

fn decode_entry_at(buf: &[u8], loc: usize, name: &str) -> Result<PakEntry, PakError> {
    fn rd_u32(buf: &[u8], off: usize) -> Option<u32> {
        Some(u32::from_le_bytes(buf.get(off..off + 4)?.try_into().ok()?))
    }
    fn rd_u64(buf: &[u8], off: usize) -> Option<u64> {
        Some(u64::from_le_bytes(buf.get(off..off + 8)?.try_into().ok()?))
    }
    let bitfield = rd_u32(buf, loc).ok_or(PakError::BadIndex("entry out of range"))?;
    let mut q = loc + 4;
    let block_size = if bitfield & 0x3f != 0x3f {
        (bitfield & 0x3f) << 11
    } else {
        let v = rd_u32(buf, q).ok_or(PakError::BadIndex("truncated entry"))?;
        q += 4;
        v
    };
    let method = (bitfield >> 23) & 0x3f;
    let offset = if bitfield & 0x8000_0000 != 0 {
        let v = rd_u32(buf, q).ok_or(PakError::BadIndex("truncated entry"))? as u64;
        q += 4;
        v
    } else {
        let v = rd_u64(buf, q).ok_or(PakError::BadIndex("truncated entry"))?;
        q += 8;
        v
    };
    let usz = if bitfield & 0x4000_0000 != 0 {
        let v = rd_u32(buf, q).ok_or(PakError::BadIndex("truncated entry"))? as u64;
        q += 4;
        v
    } else {
        let v = rd_u64(buf, q).ok_or(PakError::BadIndex("truncated entry"))?;
        q += 8;
        v
    };
    let (size, blocks) = if method != 0 {
        let size = if bitfield & 0x2000_0000 != 0 {
            let v = rd_u32(buf, q).ok_or(PakError::BadIndex("truncated entry"))? as u64;
            q += 4;
            v
        } else {
            let v = rd_u64(buf, q).ok_or(PakError::BadIndex("truncated entry"))?;
            q += 8;
            v
        };
        let blocks = (bitfield >> 6) & 0xFFFF;
        q += blocks as usize * 16;
        (size, blocks)
    } else {
        (usz, 0)
    };
    let encrypted = (bitfield >> 22) & 1 != 0;
    let _ = (q, block_size);
    Ok(PakEntry {
        name: name.to_string(),
        offset,
        size,
        uncompressed_size: usz,
        compression_method: method,
        compression_method_name: method_name(method).to_string(),
        encrypted,
        compression_blocks: blocks,
    })
}

// ---------------------------------------------------------------------------
// Legacy index (v3-v9)
// ---------------------------------------------------------------------------

fn parse_legacy_index(data: &[u8], version: u32) -> Result<PakIndex, PakError> {
    let mut c = Cursor {
        inner: std::io::Cursor::new(data),
    };
    let mount_point = c.fstring()?;
    if mount_point.is_empty() || mount_point.len() > 260 {
        return Err(PakError::BadIndex("implausible mount point"));
    }
    let count = c.i32()?;
    if count < 0 || count as i64 > MAX_ENTRIES {
        return Err(PakError::BadIndex("bad entry count"));
    }
    let mut entries = Vec::with_capacity(count.min(500_000) as usize);
    for _ in 0..count {
        let offset = c.u64()?;
        let size = c.u64()?;
        let usz = c.u64()?;
        let method = c.u32()?;
        let _hash = c.guid20()?;
        let mut blocks = 0u32;
        if version >= 3 && method != 0 {
            let n = c.i32()?;
            if n < 0 || n > 100_000 {
                return Err(PakError::BadIndex("bad block count"));
            }
            for _ in 0..n {
                c.u64()?;
                c.u64()?;
            }
            blocks = n as u32;
        }
        let encrypted = if version >= 3 { c.u8()? != 0 } else { false };
        if version >= 3 {
            let _block_size = c.u32()?;
        }
        let name = c.fstring()?;
        entries.push(PakEntry {
            name: format!("{}{}", mount_point.trim_end_matches('/'), name),
            offset,
            size,
            uncompressed_size: usz,
            compression_method: method,
            compression_method_name: method_name(method).to_string(),
            encrypted,
            compression_blocks: blocks,
        });
    }
    Ok(PakIndex {
        mount_point,
        version,
        entries,
        encrypted: false,
        decrypted: false,
    })
}

/// Convenience: parse a pak file from disk.
pub fn parse_path(path: &Path) -> Result<PakIndex, PakError> {
    parse_path_with_key(path, None)
}

/// Parse a pak from disk with an optional AES-256 key (hex or raw bytes).
pub fn parse_path_with_key(path: &Path, key: Option<&[u8; 32]>) -> Result<PakIndex, PakError> {
    let mut f = std::fs::File::open(path).map_err(PakError::Io)?;
    read_pak_index(&mut f, key)
}
