//! Binary AXML (AndroidManifest.xml) editor — string-pool surgery.
//!
//! ## Kotlin source map
//!
//! | Kotlin (Owen-Launcher)                                    | Rust (here)                      |
//! |-----------------------------------------------------------|----------------------------------|
//! | `ManifestRenamer.Pool` + `parsePool`                      | [`StringPool::parse_pool`]       |
//! | `ManifestRenamerStrings.parse(buf, chunkOff, chunkSize)`  | [`strings_at`]                   |
//! | `ManifestRenamer.encodeString` / `var8` / `readVar8`      | [`encode_string`]                |
//! | `ManifestRenamer.UTF8_FLAG` / `RES_STRING_POOL` / …       | [`UTF8_FLAG`] / [`RES_STRING_POOL_TYPE`] / … |
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/{ManifestRenamer.kt,ManifestRenamerStrings.kt}

#![forbid(unsafe_code)]

/// `RES_XML_TYPE` — chunk type of the AXML file header (`ManifestRenamer.kt`).
pub const RES_XML_TYPE: u16 = 3;
/// `RES_STRING_POOL` — chunk type of the string pool (`ManifestRenamer.kt`).
pub const RES_STRING_POOL_TYPE: u16 = 1;
/// `RES_XML_START_ELEMENT` — chunk type of `<element>` nodes (`ManifestRenamer.kt`).
pub const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
/// `UTF8_FLAG` — string-pool flag selecting UTF-8 over UTF-16 (`ManifestRenamer.kt`).
pub const UTF8_FLAG: u32 = 0x100;

/// A parsed AXML string pool (the Rust port of `ManifestRenamer.Pool`).
///
/// All offsets (`size`, `strings_start`, `styles_start`, `offsets[i]`) are
/// relative to the START of the pool chunk, exactly like the Kotlin fields.
#[derive(Debug, Clone)]
pub struct StringPool {
    pub header_size: usize,
    pub size: usize,
    pub string_count: usize,
    pub style_count: usize,
    pub flags: u32,
    pub strings_start: usize,
    pub styles_start: usize,
    pub offsets: Vec<u32>,
    pub strings: Vec<String>,
}

impl StringPool {
    /// `(flags and UTF8_FLAG) != 0` — mirrors `Pool.utf8`.
    pub fn is_utf8(&self) -> bool {
        self.flags & UTF8_FLAG != 0
    }

    /// Port of `ManifestRenamer.parsePool(pool: ByteArray)`.
    ///
    /// `pool` must be exactly the string-pool chunk bytes (from chunk type
    /// through the end of the styles blob). Out-of-bounds reads that Kotlin
    /// turned into exceptions (caught by the callers) return `None` here.
    pub fn parse_pool(pool: &[u8]) -> Option<StringPool> {
        if u16_at(pool, 0)? != RES_STRING_POOL_TYPE {
            return None;
        }
        let header_size = u16_at(pool, 2)? as usize;
        let size = u32_at(pool, 4)? as usize;
        let string_count = u32_at(pool, 8)? as usize;
        let style_count = u32_at(pool, 12)? as usize;
        let flags = u32_at(pool, 16)?;
        let strings_start = u32_at(pool, 20)? as usize;
        let styles_start = u32_at(pool, 24)? as usize;
        let utf8 = flags & UTF8_FLAG != 0;
        // Kotlin: if (stringCount < 0 || 28 + stringCount * 4 > pool.size) return null
        if 28u64.checked_add(string_count as u64 * 4)? > pool.len() as u64 {
            return None;
        }
        let mut offsets = Vec::with_capacity(string_count);
        for i in 0..string_count {
            offsets.push(u32_at(pool, 28 + i * 4)?);
        }
        let mut strings = Vec::with_capacity(string_count);
        for i in 0..string_count {
            let at = offsets[i] as usize + strings_start;
            // Kotlin: if (at < 0 || at >= pool.size) { strings.add(""); continue }
            if at >= pool.len() {
                strings.push(String::new());
                continue;
            }
            if utf8 {
                // readVar8 twice: charLen (ignored), then byteLen.
                let (_char_len, p1) = read_var8(pool, at)?;
                let (byte_len, p2) = read_var8(pool, p1)?;
                let end = (p2 + byte_len).min(pool.len());
                if byte_len > 0 && p2 < pool.len() {
                    let take = end.saturating_sub(p2);
                    strings.push(String::from_utf8_lossy(&pool[p2..p2 + take]).into_owned());
                } else {
                    strings.push(String::new());
                }
            } else {
                let mut len = u16_at(pool, at)? as usize;
                let mut p = at + 2;
                if len & 0x8000 != 0 {
                    len = ((len & 0x7FFF) << 16) | u16_at(pool, p)? as usize;
                    p = at + 4;
                }
                let mut units = Vec::with_capacity(len.min(pool.len()));
                for j in 0..len {
                    let char_at = p + j * 2;
                    // Kotlin: if (charAt + 1 >= pool.size) break
                    if char_at + 1 >= pool.len() {
                        break;
                    }
                    units.push(u16_at(pool, char_at)?);
                }
                strings.push(String::from_utf16_lossy(&units));
            }
        }
        Some(StringPool {
            header_size,
            size,
            string_count,
            style_count,
            flags,
            strings_start,
            styles_start,
            offsets,
            strings,
        })
    }
}

/// Port of `ManifestRenamerStrings.parse(buf, chunkOff, chunkSize)` — parse the
/// string pool located at absolute offset `chunk_off` inside the WHOLE buffer
/// `buf` (bounds checks run against `buf.len()`, like the Kotlin original).
pub fn strings_at(buf: &[u8], chunk_off: usize) -> Option<Vec<String>> {
    let count = u32_at(buf, chunk_off + 8)? as usize;
    let flags = u32_at(buf, chunk_off + 16)?;
    let strings_start = u32_at(buf, chunk_off + 20)? as usize;
    let utf8 = flags & 0x100 != 0;
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let at = chunk_off + strings_start + u32_at(buf, chunk_off + 28 + i * 4)? as usize;
        // Kotlin: if (at < chunkOff || at >= buf.size) { result.add(""); continue }
        if at < chunk_off || at >= buf.len() {
            result.push(String::new());
            continue;
        }
        if utf8 {
            let first = *buf.get(at)?;
            let mut p = at + if first & 0x80 != 0 { 2 } else { 1 };
            let byte_len_mark = *buf.get(p)?;
            let byte_len = if byte_len_mark & 0x80 != 0 {
                (((byte_len_mark & 0x7F) as usize) << 8) | (*buf.get(p + 1)? as usize)
            } else {
                byte_len_mark as usize
            };
            p += if byte_len_mark & 0x80 != 0 { 2 } else { 1 };
            let end = (p + byte_len).min(buf.len());
            if byte_len > 0 && p < buf.len() {
                let take = end.saturating_sub(p);
                result.push(String::from_utf8_lossy(&buf[p..p + take]).into_owned());
            } else {
                result.push(String::new());
            }
        } else {
            let mut len = u16_at(buf, at)? as usize;
            let mut p = at + 2;
            if len & 0x8000 != 0 {
                len = ((len & 0x7FFF) << 16) | u16_at(buf, p)? as usize;
                p = at + 4;
            }
            let mut units = Vec::with_capacity(len.min(buf.len()));
            for j in 0..len {
                let pos = j * 2 + p;
                // Kotlin: if (pos + 1 >= buf.size) break
                if pos + 1 >= buf.len() {
                    break;
                }
                units.push(u16_at(buf, pos)?);
            }
            result.push(String::from_utf16_lossy(&units));
        }
    }
    Some(result)
}

/// Port of `ManifestRenamer.readVar8` — (value, next_offset).
fn read_var8(b: &[u8], o: usize) -> Option<(usize, usize)> {
    let first = *b.get(o)? as usize;
    if first & 0x80 != 0 {
        Some((((first & 0x7F) << 8) | *b.get(o + 1)? as usize, o + 2))
    } else {
        Some((first, o + 1))
    }
}

/// Port of `ManifestRenamer.var8` — the modified-UTF8 length prefix.
fn var8(v: usize) -> [u8; 2] {
    debug_assert!(v < 0x8000);
    if v < 128 {
        [v as u8, 0]
    } else {
        [(0x80 | (v >> 8)) as u8, (v & 0xFF) as u8]
    }
}

/// Port of `ManifestRenamer.encodeString(value, utf8)` — re-encode a string
/// exactly the way the pool stores it (so it can be appended verbatim).
///
/// * UTF-8:  `var8(utf16_len) var8(byte_len) bytes 0x00`
/// * UTF-16: `u16 len` + UTF-16LE bytes + 2 zero bytes (NUL terminator)
pub fn encode_string(value: &str, utf8: bool) -> Vec<u8> {
    if utf8 {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len() + 4);
        // Kotlin uses String.length = UTF-16 code units.
        let units = value.encode_utf16().count();
        let l1 = var8(units);
        if units < 128 {
            out.push(l1[0]);
        } else {
            out.extend_from_slice(&l1);
        }
        let l2 = var8(bytes.len());
        if bytes.len() < 128 {
            out.push(l2[0]);
        } else {
            out.extend_from_slice(&l2);
        }
        out.extend_from_slice(bytes);
        out.push(0);
        out
    } else {
        let units: Vec<u16> = value.encode_utf16().collect();
        let mut out = vec![0u8; units.len() * 2 + 4];
        out[0..2].copy_from_slice(&(units.len() as u16).to_le_bytes());
        for (i, unit) in units.iter().enumerate() {
            out[2 + i * 2..4 + i * 2].copy_from_slice(&unit.to_le_bytes());
        }
        out
    }
}

/// A parsed AXML string pool entry we can inspect.
pub struct AxmlStrings {
    /// Raw file bytes.
    pub data: Vec<u8>,
    /// (offset_in_file, byte_len) of every pool string (UTF-16 or UTF-8).
    pub entries: Vec<(usize, usize)>,
}

fn u16_at(d: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(off)?, *d.get(off + 1)?]))
}
fn u32_at(d: &[u8], off: usize) -> Option<u32> {
    let b = d.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Parse the string pool of an AXML file.
pub fn parse(data: Vec<u8>) -> Option<AxmlStrings> {
    // Header: magic u16 0x0003, size u16, then chunks.
    // Find the RES_STRING_POOL_TYPE chunk (type 0x0001).
    let mut off = 8usize; // skip file header
    while off + 8 <= data.len() {
        let ctype = u16_at(&data, off)?;
        let csize = u32_at(&data, off + 4)? as usize;
        if ctype == 0x0001 {
            // StringPool header:
            //   chunkType u16, headerSize u16, chunkSize u32,
            //   stringCount u32, styleCount u32, flags u32,
            //   stringsStart u32, stylesStart u32
            let count = u32_at(&data, off + 8)? as usize;
            let flags = u32_at(&data, off + 16)?;
            let strings_start = u32_at(&data, off + 20)? as usize;
            let utf8 = flags & (1 << 8) != 0;
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let idx_off = off + 28 + i * 4;
                let str_off = u32_at(&data, idx_off)? as usize;
                let p = off + strings_start + str_off;
                if utf8 {
                    // u16 char len, u8 byte len, bytes, NUL
                    let _chars = u16_at(&data, p)? as usize;
                    let blen = *data.get(p + 2)? as usize;
                    entries.push((p + 3, blen));
                } else {
                    // u16 len, UTF-16LE bytes, NUL
                    let len = u16_at(&data, p)? as usize;
                    entries.push((p + 2, len * 2));
                }
            }
            return Some(AxmlStrings { data, entries });
        }
        if csize == 0 {
            break;
        }
        off += csize;
    }
    None
}

impl AxmlStrings {
    /// Read pool string `i` as a Rust String.
    pub fn get(&self, i: usize) -> Option<String> {
        let (off, len) = *self.entries.get(i)?;
        let b = self.data.get(off..off + len)?;
        let u16s: Vec<u16> = b
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)]))
            .collect();
        Some(String::from_utf16_lossy(&u16s))
    }

    /// Replace pool string `i` with `new`, **in place**. Works when the new
    /// value fits in the old slot (shorter or equal, NUL-padded); returns
    /// false otherwise (the caller then rebuilds the pool instead).
    pub fn replace_in_place(&mut self, i: usize, new: &str) -> bool {
        let Some(&(off, len)) = self.entries.get(i) else {
            return false;
        };
        let units: Vec<u16> = new.encode_utf16().collect();
        let needed = units.len() * 2;
        if needed > len {
            return false;
        }
        for (k, unit) in units.iter().enumerate() {
            let p = off + k * 2;
            self.data[p] = (*unit & 0xFF) as u8;
            self.data[p + 1] = (*unit >> 8) as u8;
        }
        // NUL-pad the remainder.
        for p in (off + needed)..(off + len) {
            self.data[p] = 0;
        }
        // Keep the terminating NUL (the byte after the slot) intact.
        true
    }

    /// The manifest package name is the string whose value equals the
    /// attribute at the manifest element — practically: the first pool
    /// string that looks like a Java package (`a.b.c`).
    pub fn find_package_string(&self) -> Option<usize> {
        for i in 0..self.entries.len() {
            let s = self.get(i)?;
            let looks_like_pkg = s.contains('.')
                && s.split('.').count() >= 2
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                && !s.contains(' ');
            if looks_like_pkg {
                return Some(i);
            }
        }
        None
    }

    /// Rename the package (in place when the new name fits).
    pub fn rename_package(&mut self, new_package: &str) -> bool {
        match self.find_package_string() {
            Some(i) => self.replace_in_place(i, new_package),
            None => false,
        }
    }
}
