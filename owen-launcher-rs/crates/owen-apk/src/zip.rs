//! Minimal zip (APK) reader/writer — STORED entries only, which is exactly
//! what an APK with `android:extractNativeLibs="false"` requires and what
//! the original pipeline produced via RawZip + zipalign.

#![forbid(unsafe_code)]

use std::io::{Read, Seek, SeekFrom};

pub const LF_SIGNATURE: u32 = 0x0403_4b50;
pub const CD_SIGNATURE: u32 = 0x0201_4b50;
pub const EOCD_SIGNATURE: u32 = 0x0605_4b50;

pub struct Entry {
    pub name: String,
    pub offset: u64,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

/// List entries by scanning local file headers (robust for APKs).
pub fn list<R: Read + Seek>(r: &mut R) -> std::io::Result<Vec<Entry>> {
    let len = r.seek(SeekFrom::End(0))?;
    let mut pos = 0u64;
    let mut out = Vec::new();
    while pos + 30 <= len {
        r.seek(SeekFrom::Start(pos))?;
        let mut hdr = [0u8; 30];
        if r.read_exact(&mut hdr).is_err() {
            break;
        }
        let sig = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        if sig != LF_SIGNATURE {
            break;
        }
        let name_len = u16::from_le_bytes([hdr[26], hdr[27]]) as usize;
        let extra_len = u16::from_le_bytes([hdr[28], hdr[29]]) as usize;
        let csize = u32::from_le_bytes([hdr[18], hdr[19], hdr[20], hdr[21]]) as u64;
        let usize_ = u32::from_le_bytes([hdr[22], hdr[23], hdr[24], hdr[25]]) as u64;
        let mut name = vec![0u8; name_len];
        r.read_exact(&mut name)?;
        out.push(Entry {
            name: String::from_utf8_lossy(&name).into_owned(),
            offset: pos + 30 + name_len as u64 + extra_len as u64,
            compressed_size: csize,
            uncompressed_size: usize_,
        });
        pos = pos + 30 + name_len as u64 + extra_len as u64 + csize;
    }
    Ok(out)
}

/// Extract one stored entry.
pub fn extract<R: Read + Seek>(r: &mut R, e: &Entry) -> std::io::Result<Vec<u8>> {
    r.seek(SeekFrom::Start(e.offset))?;
    let mut buf = vec![0u8; e.compressed_size as usize];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

pub struct ZipBuilder {
    files: Vec<(String, Vec<u8>)>,
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

impl ZipBuilder {
    pub fn new() -> ZipBuilder {
        ZipBuilder { files: Vec::new() }
    }

    pub fn add(&mut self, name: &str, data: Vec<u8>) {
        self.files.push((name.to_string(), data));
    }

    /// Serialise into a valid zip with STORED, 4-byte-aligned entries
    /// (zipalign semantics for native libs).
    pub fn build_aligned(mut self) -> Vec<u8> {
        // Sort for stable output.
        self.files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let _pad = |out: &mut Vec<u8>, to: usize| {
            while out.len() % to != 0 {
                out.push(0);
            }
        };

        for (name, data) in &self.files {
            // Alignment: entry data must start at a 4-byte boundary.
            let header_len = 30 + name.len();
            let cur = out.len();
            let _data_start = cur + header_len;
            while (out.len() + header_len) % 4 != 0 {
                out.push(0);
            }
            let offset = out.len() as u32;
            let crc = crc32(data);
            let mut lfh = Vec::with_capacity(header_len);
            lfh.extend_from_slice(&LF_SIGNATURE.to_le_bytes());
            lfh.extend_from_slice(&20u16.to_le_bytes()); // version
            lfh.extend_from_slice(&0u16.to_le_bytes()); // flags
            lfh.extend_from_slice(&0u16.to_le_bytes()); // method: STORED
            lfh.extend_from_slice(&0u16.to_le_bytes()); // time
            lfh.extend_from_slice(&0u32.to_le_bytes()); // date
            lfh.extend_from_slice(&crc.to_le_bytes());
            lfh.extend_from_slice(&(data.len() as u32).to_le_bytes());
            lfh.extend_from_slice(&(data.len() as u32).to_le_bytes());
            lfh.extend_from_slice(&(name.len() as u16).to_le_bytes());
            lfh.extend_from_slice(&0u16.to_le_bytes()); // extra len
            lfh.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&lfh);
            out.extend_from_slice(data);

            central.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&0u16.to_le_bytes()); // flags
            central.extend_from_slice(&0u16.to_le_bytes()); // method
            central.extend_from_slice(&0u16.to_le_bytes()); // time
            central.extend_from_slice(&0u16.to_le_bytes()); // date
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra
            central.extend_from_slice(&0u16.to_le_bytes()); // comment
            central.extend_from_slice(&0u16.to_le_bytes()); // disk
            central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }

        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        out.extend_from_slice(&central);
        let count = self.files.len() as u16;
        out.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // start disk
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out
    }
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}
