//! Binary AXML (AndroidManifest.xml) editor — string-pool surgery.

#![forbid(unsafe_code)]

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
