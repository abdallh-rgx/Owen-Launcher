//! Minimal ZIP reader/writer for APK surgery.
//!
//! Design goals:
//! * Read entries (stored + deflate).
//! * Copy untouched entries **byte-identical** (raw compressed data is
//!   streamed from the source file).
//! * Add or replace entries (stored for native libs, deflate otherwise).
//! * Produce a valid ZIP with correct central directory and EOCD.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct ZipEntryInfo {
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub method: u16,
    pub local_header_offset: u64,
}

pub struct ZipArchive {
    path: std::path::PathBuf,
    pub entries: Vec<ZipEntryInfo>,
    pub overrides: BTreeMap<String, Vec<u8>>,
    pub additions: BTreeMap<String, Vec<u8>>,
}

fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn rd_u64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7],
    ])
}

impl ZipArchive {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let size = f.metadata().map_err(|e| e.to_string())?.len();
        let read_back = (size).min(65_536 + 22);
        f.seek(SeekFrom::Start(size - read_back))
            .map_err(|e| e.to_string())?;
        let mut tail = vec![0u8; read_back as usize];
        f.read_exact(&mut tail).map_err(|e| e.to_string())?;

        // Find EOCD.
        let mut eocd = None;
        if tail.len() >= 22 {
            for i in (0..=tail.len() - 22).rev() {
                if rd_u32(&tail, i) == 0x0605_4b50 {
                    eocd = Some(i);
                    break;
                }
            }
        }
        let eocd = eocd.ok_or("no EOCD found")?;
        let cd_offset = rd_u32(&tail, eocd + 16) as u64;
        let cd_count = rd_u16(&tail, eocd + 10) as usize;

        f.seek(SeekFrom::Start(cd_offset)).map_err(|e| e.to_string())?;
        let mut cd = vec![0u8; (size - cd_offset).min(16 * 1024 * 1024) as usize];
        let cd_len = f.read(&mut cd).map_err(|e| e.to_string())?;
        cd.truncate(cd_len);

        let mut entries = Vec::new();
        let mut off = 0usize;
        for _ in 0..cd_count {
            if off + 46 > cd.len() || rd_u32(&cd, off) != 0x0201_4b50 {
                break;
            }
            let method = rd_u16(&cd, off + 10);
            let comp = rd_u32(&cd, off + 20) as u64;
            let uncomp = rd_u32(&cd, off + 24) as u64;
            let name_len = rd_u16(&cd, off + 28) as usize;
            let extra_len = rd_u16(&cd, off + 30) as usize;
            let comment_len = rd_u16(&cd, off + 32) as usize;
            let lho = rd_u32(&cd, off + 42) as u64;
            if off + 46 + name_len > cd.len() {
                break;
            }
            let name = String::from_utf8_lossy(&cd[off + 46..off + 46 + name_len]).into_owned();
            entries.push(ZipEntryInfo {
                name,
                compressed_size: comp,
                uncompressed_size: uncomp,
                method,
                local_header_offset: lho,
            });
            off += 46 + name_len + extra_len + comment_len;
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
            overrides: BTreeMap::new(),
            additions: BTreeMap::new(),
        })
    }

    pub fn list(&self) -> &[ZipEntryInfo] {
        &self.entries
    }

    pub fn has(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    /// Read and decompress one entry.
    pub fn read_entry(&self, name: &str) -> Option<Vec<u8>> {
        if let Some(v) = self.overrides.get(name) {
            return Some(v.clone());
        }
        if let Some(v) = self.additions.get(name) {
            return Some(v.clone());
        }
        let e = self.entries.iter().find(|e| e.name == name)?;
        let mut f = std::fs::File::open(&self.path).ok()?;
        f.seek(SeekFrom::Start(e.local_header_offset)).ok()?;
        let mut lh = [0u8; 30];
        f.read_exact(&mut lh).ok()?;
        if rd_u32(&lh, 0) != 0x0403_4b50 {
            return None;
        }
        let name_len = rd_u16(&lh, 26) as usize;
        let extra_len = rd_u16(&lh, 28) as usize;
        f.seek(SeekFrom::Current((name_len + extra_len) as i64))
            .ok()?;
        let mut raw = vec![0u8; e.compressed_size as usize];
        f.read_exact(&mut raw).ok()?;
        match e.method {
            0 => Some(raw),
            8 => {
                let mut out = Vec::with_capacity(e.uncompressed_size as usize);
                let mut dec = flate2::read::DeflateDecoder::new(&raw[..]);
                dec.read_to_end(&mut out).ok()?;
                Some(out)
            }
            _ => None,
        }
    }

    /// Queue a replacement for an existing entry (native libs are stored
    /// uncompressed and 16-byte aligned so Android can mmap them).
    pub fn replace_entry(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        if !self.has(name) {
            return Err(format!("no such entry: {name}"));
        }
        self.overrides.insert(name.to_string(), data.to_vec());
        Ok(())
    }

    /// Queue a new entry.
    pub fn add_entry(&mut self, name: &str, data: &[u8]) {
        self.additions.insert(name.to_string(), data.to_vec());
    }

    /// Write the final APK.
    pub fn write_to(&self, out_path: &Path) -> Result<(), String> {
        let mut out = std::io::BufWriter::new(
            std::fs::File::create(out_path).map_err(|e| e.to_string())?,
        );
        let mut src = std::fs::File::open(&self.path).map_err(|e| e.to_string())?;
        let mut central: Vec<u8> = Vec::new();
        let mut count = 0u16;

        let mut offset: u64 = 0;
        for e in &self.entries {
            let data: Option<Cow> = if let Some(new) = self.overrides.get(&e.name) {
                Some(if e.name.ends_with(".so") {
                    Cow::Stored(new.clone())
                } else {
                    let mut enc =
                        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
                    std::io::Write::write_all(&mut enc, new).map_err(|x| x.to_string())?;
                    Cow::Deflated(enc.finish().map_err(|x| x.to_string())?, new.len() as u32)
                })
            } else {
                None
            };

            match data {
                Some(payload) => {
                    let (raw, method, crc, usize_) = match &payload {
                        Cow::Stored(v) => (v.clone(), 0u16, crc32(v), v.len() as u32),
                        Cow::Deflated(v, ulen) => (v.clone(), 8u16, crc32(v), *ulen),
                    };
                    let name = e.name.as_bytes();
                    // zipalign-style 4096 padding for stored entries (native
                    // libs) so the APK installs with extractNativeLibs=false.
                    let extra_pad = if method == 0 {
                        let hdr = 30u64 + name.len() as u64;
                        let rem = (offset + hdr) % 4096;
                        if rem == 0 { 0 } else { (4096 - rem) as usize }
                    } else {
                        0
                    };
                    let mut lh = local_header(offset, method, crc, raw.len() as u32, usize_, name.len() as u16);
                    lh[28..30].copy_from_slice(&(extra_pad as u16).to_le_bytes());
                    let pad = vec![0u8; extra_pad];
                    write_all(&mut out, &lh)?;
                    write_all(&mut out, name)?;
                    write_all(&mut out, &pad)?;
                    write_all(&mut out, &raw)?;
                    central.extend_from_slice(&central_header(
                        method, crc, raw.len() as u32, usize_, name.len() as u16, offset,
                    ));
                    central.extend_from_slice(name);
                    offset += lh.len() as u64 + name.len() as u64 + pad.len() as u64 + raw.len() as u64;
                }
                None => {
                    // Raw copy from source.
                    src.seek(SeekFrom::Start(e.local_header_offset))
                        .map_err(|x| x.to_string())?;
                    let mut lh = [0u8; 30];
                    src.read_exact(&mut lh).map_err(|x| x.to_string())?;
                    if rd_u32(&lh, 0) != 0x0403_4b50 {
                        return Err(format!("bad local header for {}", e.name));
                    }
                    let name_len = rd_u16(&lh, 26) as usize;
                    let extra_len = rd_u16(&lh, 28) as usize;
                    let mut name_buf = vec![0u8; name_len];
                    src.read_exact(&mut name_buf).map_err(|x| x.to_string())?;
                    let mut extra = vec![0u8; extra_len];
                    src.read_exact(&mut extra).map_err(|x| x.to_string())?;
                    let mut raw = vec![0u8; e.compressed_size as usize];
                    src.read_exact(&mut raw).map_err(|x| x.to_string())?;

                    write_all(&mut out, &lh)?;
                    write_all(&mut out, &name_buf)?;
                    write_all(&mut out, &extra)?;
                    write_all(&mut out, &raw)?;

                    let crc = rd_u32(&lh, 14);
                    central.extend_from_slice(&central_header(
                        e.method,
                        crc,
                        e.compressed_size as u32,
                        e.uncompressed_size as u32,
                        name_len as u16,
                        offset,
                    ));
                    central.extend_from_slice(&name_buf);
                    offset += 30 + name_len as u64 + extra_len as u64 + raw.len() as u64;
                }
            }
            count += 1;
        }

        // Additions.
        for (name, data) in &self.additions {
            let (raw, method): (Vec<u8>, u16) = if name.ends_with(".so") {
                (data.clone(), 0)
            } else {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
                std::io::Write::write_all(&mut enc, data).map_err(|x| x.to_string())?;
                (enc.finish().map_err(|x| x.to_string())?, 8)
            };
            let name_b = name.as_bytes();
            // zipalign-style: align stored .so data to 4096 via the local
            // extra field so the entry stays mmap-friendly.
            let extra_pad = if method == 0 {
                let hdr = 30u64 + name_b.len() as u64;
                let rem = (offset + hdr) % 4096;
                if rem == 0 {
                    0
                } else {
                    (4096 - rem) as usize
                }
            } else {
                0
            };
            let lh = local_header(
                offset,
                method,
                crc32(&raw),
                raw.len() as u32,
                data.len() as u32,
                name_b.len() as u16,
            );
            // patch extra-field length
            let mut lh = lh;
            lh[28..30].copy_from_slice(&(extra_pad as u16).to_le_bytes());
            let mut pad = vec![0u8; extra_pad];
            write_all(&mut out, &lh)?;
            write_all(&mut out, name_b)?;
            write_all(&mut out, &pad)?;
            write_all(&mut out, &raw)?;
            central.extend_from_slice(&central_header(
                method,
                crc32(&raw),
                raw.len() as u32,
                data.len() as u32,
                name_b.len() as u16,
                offset,
            ));
            central.extend_from_slice(name_b);
            offset += lh.len() as u64 + name_b.len() as u64 + pad.len() as u64 + raw.len() as u64;
            count += 1;
        }

        let cd_offset = offset;
        write_all(&mut out, &central)?;
        let cd_size = central.len() as u64;
        let mut eocd = [0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        eocd[8..10].copy_from_slice(&count.to_le_bytes());
        eocd[10..12].copy_from_slice(&count.to_le_bytes());
        eocd[12..16].copy_from_slice(&(cd_size as u32).to_le_bytes());
        eocd[16..20].copy_from_slice(&(cd_offset as u32).to_le_bytes());
        write_all(&mut out, &eocd)?;
        out.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

enum Cow {
    Stored(Vec<u8>),
    Deflated(Vec<u8>, u32),
}

fn local_header(offset: u64, method: u16, crc: u32, csize: u32, usize_: u32, name_len: u16) -> Vec<u8> {
    let mut h = vec![0u8; 30];
    h[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
    h[4..6].copy_from_slice(&20u16.to_le_bytes()); // version needed
    h[6..8].copy_from_slice(&0u16.to_le_bytes()); // flags
    h[8..10].copy_from_slice(&method.to_le_bytes());
    h[10..12].copy_from_slice(&0u16.to_le_bytes()); // time
    h[12..14].copy_from_slice(&0x2100u16.to_le_bytes()); // date (2000-01-01)
    h[14..18].copy_from_slice(&crc.to_le_bytes());
    h[18..22].copy_from_slice(&csize.to_le_bytes());
    h[22..26].copy_from_slice(&usize_.to_le_bytes());
    h[26..28].copy_from_slice(&name_len.to_le_bytes());
    h[28..30].copy_from_slice(&0u16.to_le_bytes());
    let _ = offset;
    h
}

fn central_header(method: u16, crc: u32, csize: u32, usize_: u32, name_len: u16, offset: u64) -> Vec<u8> {
    let mut h = vec![0u8; 46];
    h[0..4].copy_from_slice(&0x0201_4b50u32.to_le_bytes());
    h[4..6].copy_from_slice(&20u16.to_le_bytes()); // version made by
    h[6..8].copy_from_slice(&20u16.to_le_bytes()); // version needed
    h[8..10].copy_from_slice(&0u16.to_le_bytes());
    h[10..12].copy_from_slice(&method.to_le_bytes());
    h[12..14].copy_from_slice(&0u16.to_le_bytes());
    h[14..16].copy_from_slice(&0x2100u16.to_le_bytes());
    h[16..20].copy_from_slice(&crc.to_le_bytes());
    h[20..24].copy_from_slice(&csize.to_le_bytes());
    h[24..28].copy_from_slice(&usize_.to_le_bytes());
    h[28..30].copy_from_slice(&name_len.to_le_bytes());
    h[30..32].copy_from_slice(&0u16.to_le_bytes()); // extra len
    h[32..34].copy_from_slice(&0u16.to_le_bytes()); // comment len
    h[34..36].copy_from_slice(&0u16.to_le_bytes()); // disk
    h[36..38].copy_from_slice(&0u16.to_le_bytes()); // internal attrs
    h[38..42].copy_from_slice(&0u32.to_le_bytes()); // external attrs
    h[42..46].copy_from_slice(&(offset as u32).to_le_bytes());
    h
}

fn write_all<W: std::io::Write>(w: &mut W, data: &[u8]) -> Result<(), String> {
    w.write_all(data).map_err(|e| e.to_string())
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}
