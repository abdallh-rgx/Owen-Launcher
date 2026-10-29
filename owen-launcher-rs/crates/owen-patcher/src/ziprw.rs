//! Minimal ZIP reader/writer for APK surgery.
//!
//! Design goals:
//! * Read entries (stored + deflate).
//! * Copy untouched entries **byte-identical** (raw compressed data is
//!   streamed from the source file).
//! * Add or replace entries (stored for native libs, deflate otherwise —
//!   replacements KEEP the original compression method).
//! * Produce a valid ZIP with correct central directory and EOCD.
//! * **zipalign semantics**: STORED entries are (re)padded so their data
//!   offset stays aligned — `lib/<abi>/*.so` to 16 KiB (`RawZip.kt`
//!   `SO_ALIGN = 16384`, page-alignment for `extractNativeLibs=false`) and
//!   every other STORED entry (e.g. `resources.arsc`, icons) to 4 bytes.
//!   Re-padding happens on EVERY copy, so replacing an earlier entry can
//!   never break the alignment of later STORED entries.
//!
//! ## Kotlin source map
//!
//! | Kotlin (`RawZip.kt`)                    | Rust (here)                        |
//! |-----------------------------------------|------------------------------------|
//! | `Entry` / `readEntries`                 | [`ZipEntryInfo`] / [`ZipArchive::open`] |
//! | `rebuild(source, dest, skip, extra, stored)` | `remove_*` + `set_entry`/`add_entry` + [`ZipArchive::write_to`] |
//! | `SO_ALIGN = 16384`                      | [`SO_ALIGN`]                       |
//! | `copyRawEntry` (data-descriptor aware)  | `write_to` raw-copy branch         |
//! | `writeLocalHeader`/`writeCentralHeader`/`writeEocd` | `local_header`/`central_header`/EOCD |
//! | `nowDosTime`/`crc32`                    | fixed DOS date + [`crc32`]         |
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/RawZip.kt

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// `RawZip.kt SO_ALIGN` — page alignment for uncompressed native libs.
pub const SO_ALIGN: u64 = 16384;
/// zipalign default: uncompressed resources (`resources.arsc`, icons) stay
/// 4-byte aligned so `Resources` can mmap them.
pub const RES_ALIGN: u64 = 4;

pub struct ZipEntryInfo {
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub method: u16,
    pub flags: u16,
    pub crc: u32,
    pub local_header_offset: u64,
}

pub struct ZipArchive {
    path: std::path::PathBuf,
    pub entries: Vec<ZipEntryInfo>,
    pub overrides: BTreeMap<String, Vec<u8>>,
    pub additions: BTreeMap<String, Vec<u8>>,
    /// Always-DEFLATED additions (v1 signature files under `META-INF/`).
    pub deflated_additions: BTreeMap<String, Vec<u8>>,
    /// Explicitly-STORED additions, zipalign-style (4-byte aligned;
    /// `lib/<abi>/*.so` still page-aligned) — used by the APK packer for
    /// `AndroidManifest.xml` / `resources.arsc`.
    pub stored_additions: BTreeMap<String, Vec<u8>>,
}

fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Alignment class for a STORED entry (`lib/<abi>/*.so` → page aligned,
/// everything else → 4).
fn alignment_for(name: &str) -> u64 {
    if name.starts_with("lib/") && name.ends_with(".so") {
        SO_ALIGN
    } else {
        RES_ALIGN
    }
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
            let flags = rd_u16(&cd, off + 8);
            let method = rd_u16(&cd, off + 10);
            let crc = rd_u32(&cd, off + 16);
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
                flags,
                crc,
                local_header_offset: lho,
            });
            off += 46 + name_len + extra_len + comment_len;
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
            overrides: BTreeMap::new(),
            additions: BTreeMap::new(),
            deflated_additions: BTreeMap::new(),
            stored_additions: BTreeMap::new(),
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
        if let Some(v) = self.deflated_additions.get(name) {
            return Some(v.clone());
        }
        if let Some(v) = self.stored_additions.get(name) {
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

    /// Queue a replacement for an existing entry. The entry KEEPS its
    /// original compression method (a STORED `resources.arsc` stays STORED,
    /// a DEFLATED manifest stays deflated); STORED replacements are
    /// realigned like zipalign does.
    pub fn replace_entry(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        if !self.has(name) {
            return Err(format!("no such entry: {name}"));
        }
        self.overrides.insert(name.to_string(), data.to_vec());
        Ok(())
    }

    /// Queue a new entry (stored + page-aligned for `lib/<abi>/*.so`,
    /// deflated otherwise).
    pub fn add_entry(&mut self, name: &str, data: &[u8]) {
        self.additions.insert(name.to_string(), data.to_vec());
    }

    /// Queue a new entry that is always DEFLATED (used for the v1 signature
    /// files `META-INF/*`).
    pub fn add_deflated(&mut self, name: &str, data: &[u8]) {
        self.deflated_additions
            .insert(name.to_string(), data.to_vec());
    }

    /// Queue a new entry that is always STORED and (re)aligned like
    /// zipalign does: `lib/<abi>/*.so` → 16 KiB page alignment, everything
    /// else (`AndroidManifest.xml`, `resources.arsc`, …) → 4 bytes.
    pub fn add_stored(&mut self, name: &str, data: &[u8]) {
        self.stored_additions
            .insert(name.to_string(), data.to_vec());
    }

    /// Replace-or-add: when `name` already exists in the archive the data
    /// becomes an override (original method kept), otherwise a new entry.
    pub fn set_entry(&mut self, name: &str, data: &[u8]) {
        if self.has(name) {
            self.overrides.insert(name.to_string(), data.to_vec());
        } else {
            self.additions.insert(name.to_string(), data.to_vec());
        }
    }

    /// Drop one entry from the output.
    pub fn remove_entry(&mut self, name: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.name != name);
        self.overrides.remove(name);
        self.additions.remove(name);
        self.deflated_additions.remove(name);
        before != self.entries.len()
    }

    /// `ApkPatcher.SKIP_META` — drop every `META-INF` entry (stale v1
    /// signatures never survive a repack).
    pub fn remove_meta_inf(&mut self) {
        self.entries
            .retain(|e| !(e.name == "META-INF" || e.name.starts_with("META-INF/")));
        let stale: Vec<String> = self
            .overrides
            .keys()
            .chain(self.additions.keys())
            .chain(self.deflated_additions.keys())
            .chain(self.stored_additions.keys())
            .filter(|n| *n == "META-INF" || n.starts_with("META-INF/"))
            .cloned()
            .collect();
        for name in stale {
            self.overrides.remove(&name);
            self.additions.remove(&name);
            self.deflated_additions.remove(&name);
            self.stored_additions.remove(&name);
        }
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
            if let Some(new) = self.overrides.get(&e.name) {
                // Replacement: keep the original compression method so a
                // STORED resources.arsc / .so stays uncompressed.
                let stored = e.method == 0;
                let (raw, method, usize_): (Vec<u8>, u16, u32) = if stored {
                    (new.clone(), 0u16, new.len() as u32)
                } else {
                    let mut enc =
                        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
                    std::io::Write::write_all(&mut enc, new).map_err(|x| x.to_string())?;
                    (
                        enc.finish().map_err(|x| x.to_string())?,
                        8u16,
                        new.len() as u32,
                    )
                };
                let name = e.name.as_bytes();
                let extra_pad = align_pad(offset, name.len(), method, &e.name) as usize;
                // CRC is over the UNCOMPRESSED data (zip spec) — `raw` is the
                // compressed stream for method 8.
                let crc = ziprw_crc32(new);
                let mut lh = local_header(method, crc, raw.len() as u32, usize_, name.len() as u16);
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
            } else {
                // Raw copy from source (byte-identical payload).
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

                // Stored entries are re-padded so their data offset keeps
                // the alignment class zipalign would give them, even when an
                // earlier entry changed size.
                let extra_pad = align_pad(offset, name_len, e.method, &e.name) as usize;
                // Rewrite the local header: fresh padding, real CRC/sizes
                // from the central directory, and no data-descriptor flag
                // (the descriptor bytes are simply not copied).
                let mut lh_out = lh;
                let flags = rd_u16(&lh_out, 6) & !0x0008;
                lh_out[6..8].copy_from_slice(&flags.to_le_bytes());
                lh_out[14..18].copy_from_slice(&e.crc.to_le_bytes());
                lh_out[18..22].copy_from_slice(&(e.compressed_size as u32).to_le_bytes());
                lh_out[22..26].copy_from_slice(&(e.uncompressed_size as u32).to_le_bytes());
                lh_out[28..30].copy_from_slice(&(extra_pad as u16).to_le_bytes());
                let pad = vec![0u8; extra_pad];

                write_all(&mut out, &lh_out)?;
                write_all(&mut out, &name_buf)?;
                write_all(&mut out, &pad)?;
                write_all(&mut out, &raw)?;

                central.extend_from_slice(&central_header(
                    e.method,
                    e.crc,
                    e.compressed_size as u32,
                    e.uncompressed_size as u32,
                    name_len as u16,
                    offset,
                ));
                central.extend_from_slice(&name_buf);
                offset += 30 + name_len as u64 + pad.len() as u64 + raw.len() as u64;
            }
            count += 1;
        }

        // Additions — merged and written in name order for a deterministic
        // layout: `.so` under lib/ → STORED + page-aligned (whatever map it
        // came from); explicit deflated additions (v1 signature files);
        // explicit STORED additions (manifest/resources.arsc); everything
        // else deflated.
        let mut pending: Vec<(&str, u16, &Vec<u8>)> = Vec::new();
        for (name, data) in &self.additions {
            let method: u16 = if name.starts_with("lib/") && name.ends_with(".so") {
                0
            } else {
                8
            };
            pending.push((name, method, data));
        }
        for (name, data) in &self.stored_additions {
            pending.push((name, 0, data));
        }
        for (name, data) in &self.deflated_additions {
            pending.push((name, 8, data));
        }
        pending.sort_by(|a, b| a.0.cmp(b.0));
        for (name, method, data) in pending {
            let (raw, method): (Vec<u8>, u16) = if method == 0 {
                (data.clone(), 0)
            } else {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
                std::io::Write::write_all(&mut enc, data).map_err(|x| x.to_string())?;
                (enc.finish().map_err(|x| x.to_string())?, 8)
            };
            let name_b = name.as_bytes();
            let extra_pad = align_pad(offset, name_b.len(), method, name) as usize;
            // CRC over the UNCOMPRESSED bytes (zip spec; `raw` is the
            // deflate stream for method 8).
            let crc = ziprw_crc32(data);
            let lh = local_header(
                method,
                crc,
                raw.len() as u32,
                data.len() as u32,
                name_b.len() as u16,
            );
            let mut lh = lh;
            lh[28..30].copy_from_slice(&(extra_pad as u16).to_le_bytes());
            let pad = vec![0u8; extra_pad];
            write_all(&mut out, &lh)?;
            write_all(&mut out, name_b)?;
            write_all(&mut out, &pad)?;
            write_all(&mut out, &raw)?;
            central.extend_from_slice(&central_header(
                method,
                crc,
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

/// Extra-field padding (bytes) needed so that the STORED data offset
/// `offset + 30 + name_len + pad` is aligned to the entry's alignment class.
/// DEFLATED entries get no padding (zipalign does not align compressed
/// data).
fn align_pad(offset: u64, name_len: usize, method: u16, name: &str) -> u64 {
    if method != 0 {
        return 0;
    }
    let align = alignment_for(name);
    let hdr = 30u64 + name_len as u64;
    let rem = (offset + hdr) % align;
    if rem == 0 {
        0
    } else {
        align - rem
    }
}

fn local_header(method: u16, crc: u32, csize: u32, usize_: u32, name_len: u16) -> Vec<u8> {
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

/// CRC-32 over the UNCOMPRESSED entry bytes — an alias so call sites read
/// clearly next to `crc32(&raw)` mistakes of the past.
fn ziprw_crc32(data: &[u8]) -> u32 {
    crc32(data)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// Regression: the CRC of a DEFLATED entry must be over the UNCOMPRESSED
    /// bytes (a previous version hashed the deflate stream — Python's
    /// `ZipFile.testzip` / Android's installer both reject such zips).
    #[test]
    fn deflated_entry_crc_is_over_uncompressed_data() {
        let dir = std::env::temp_dir().join(format!("owen-ziprw-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let empty = dir.join("empty.apk");
        let mut eocd = [0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        std::fs::write(&empty, eocd).unwrap();

        let data = vec![0xABu8; 100_000]; // highly compressible
        let mut zip = ZipArchive::open(&empty).unwrap();
        zip.add_entry("assets/payload.bin", &data);
        zip.add_stored("resources.arsc", b"0123456789ab".as_slice());
        let out = dir.join("out.apk");
        zip.write_to(&out).unwrap();

        // The written archive must expose the CRC of the ORIGINAL bytes.
        let reread = ZipArchive::open(&out).unwrap();
        let entry = reread
            .list()
            .iter()
            .find(|e| e.name == "assets/payload.bin")
            .expect("deflated entry");
        assert_eq!(entry.method, 8);
        assert_eq!(entry.crc, crc32(&data), "CRC must cover the uncompressed bytes");
        assert_eq!(entry.uncompressed_size, data.len() as u64);
        assert!(entry.compressed_size < data.len() as u64, "actually deflated");
        // STORED entry: raw == data, CRC identical either way.
        let arsc = reread
            .list()
            .iter()
            .find(|e| e.name == "resources.arsc")
            .expect("stored entry");
        assert_eq!(arsc.method, 0);
        assert_eq!(arsc.crc, crc32(b"0123456789ab"));
        // and the payload round-trips
        assert_eq!(reread.read_entry("assets/payload.bin").unwrap(), data);
        let _ = std::io::stdout().flush();
        std::fs::remove_dir_all(&dir).ok();
    }
}
