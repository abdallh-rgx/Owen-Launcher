//! owen-pak — UE4 `.pak` reader and per-season Battle Pass extractor.
//!
//! Reads the classic pak index (versions 1-8, the layout every Chapter-1
//! and Chapter-2 season uses), lists/extracts entries by pattern, and
//! produces the per-season reward report the Owen Web Console imports:
//!
//! * Cosmetic item ids (`cid_...`, `bid_...`, `eid_...`, `pickaxe_id_...`,
//!   `glider_id_...`, `umbrella_id_...`, `musicpack_id_...`, `spid_...`,
//!   `vtid_...`, `loadingscreen_...`) discovered inside the pak entries —
//!   the same token vocabulary the `BattlePass/SeasonN.json` files use.
//! * Season metadata assets (`*Season*`, `*BattlePass*` paths) with their
//!   embedded display strings.
//!
//! Everything is safe Rust with zero dependencies; AES-encrypted indexes are
//! detected and reported (they need a key, supplied via `--aes`).

#![forbid(unsafe_code)]

use std::fmt;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub const PAK_MAGIC: u32 = 0x5A6F12E1;

#[derive(Debug)]
pub enum PakError {
    NotAPak,
    UnsupportedVersion(u32),
    EncryptedIndex,
    Io(std::io::Error),
    BadIndex,
}

impl fmt::Display for PakError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PakError::NotAPak => write!(f, "not a UE4 pak file"),
            PakError::UnsupportedVersion(v) => write!(f, "pak version {v} not supported"),
            PakError::EncryptedIndex => write!(f, "index is AES-encrypted (supply --aes)"),
            PakError::Io(e) => write!(f, "io error: {e}"),
            PakError::BadIndex => write!(f, "corrupt index"),
        }
    }
}

impl From<std::io::Error> for PakError {
    fn from(e: std::io::Error) -> Self {
        PakError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct PakEntry {
    /// Normalised path: leading "../../../" mount prefixes stripped.
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub uncompressed_size: u64,
    pub compression: u32,
    pub encrypted: bool,
    pub blocks: Vec<(u64, u64)>,
    pub hash: [u8; 20],
}

#[derive(Debug, Clone)]
pub struct Pak {
    pub version: u32,
    pub mount_point: String,
    pub entries: Vec<PakEntry>,
}

fn read_u32<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_u64<R: Read>(r: &mut R) -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

fn read_fstring<R: Read>(r: &mut R) -> std::io::Result<String> {
    let len = read_i32(r)?;
    let mut s = if len >= 0 {
        let mut b = vec![0u8; len as usize];
        r.read_exact(&mut b)?;
        // strip trailing NUL
        while b.last() == Some(&0) {
            b.pop();
        }
        String::from_utf8_lossy(&b).into_owned()
    } else {
        let n = (-len) as usize * 2;
        let mut b = vec![0u8; n];
        r.read_exact(&mut b)?;
        let units: Vec<u16> = b
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        String::from_utf16_lossy(&units)
    };
    // Normalise the "../../.." mount garbage into a clean content root.
    while let Some(stripped) = s.strip_prefix("../") {
        s = stripped.to_string();
    }
    while let Some(stripped) = s.strip_prefix("..\\") {
        s = stripped.to_string();
    }
    Ok(s)
}

fn read_i32<R: Read>(r: &mut R) -> std::io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}

/// Open and index a pak file.
pub fn open(path: &std::path::Path) -> Result<Pak, PakError> {
    let mut f = std::fs::File::open(path).map_err(PakError::Io)?;
    let size = f.metadata().map_err(PakError::Io)?.len();
    if size < 221 {
        return Err(PakError::NotAPak);
    }

    // Footer is at the end; sizes: v1-8: 44 bytes, v8-with-path-hash: 221,
    // v9: 221+. Probe the 221-byte window from the end first (works for all).
    f.seek(SeekFrom::End(-221)).map_err(PakError::Io)?;
    let mut footer = vec![0u8; 221];
    f.read_exact(&mut footer).map_err(PakError::Io)?;

    // Scan for the magic within the footer window (classic layout magic at
    // footer_start+0 for <=v8 at offset 44-from-end).
    let mut magic_at: Option<usize> = None;
    for i in 0..footer.len().saturating_sub(4) {
        if u32::from_le_bytes([footer[i], footer[i + 1], footer[i + 2], footer[i + 3]])
            == PAK_MAGIC
        {
            magic_at = Some(i);
            break;
        }
    }
    let Some(m) = magic_at else {
        return Err(PakError::NotAPak);
    };

    let version = u32::from_le_bytes([footer[m + 4], footer[m + 5], footer[m + 6], footer[m + 7]]);
    if version == 0 || version > 11 {
        return Err(PakError::UnsupportedVersion(version));
    }
    let index_offset = u64::from_le_bytes(
        footer[m + 8..m + 16].try_into().unwrap(),
    );
    let index_size = u64::from_le_bytes(
        footer[m + 16..m + 24].try_into().unwrap(),
    );
    let encrypted = version >= 8 && footer.get(m + 44).copied().unwrap_or(0) == 1;
    if encrypted {
        return Err(PakError::EncryptedIndex);
    }

    f.seek(SeekFrom::Start(index_offset)).map_err(PakError::Io)?;
    let mut index = vec![0u8; index_size as usize];
    f.read_exact(&mut index).map_err(PakError::Io)?;

    let mut cur = std::io::Cursor::new(&index);
    let mount_point = read_fstring(&mut cur)?;
    let num_entries = read_u32(&mut cur)?;
    let mut entries = Vec::with_capacity(num_entries.min(200_000) as usize);

    for _ in 0..num_entries {
        let path = match read_fstring(&mut cur) {
            Ok(p) => p,
            Err(_) => break,
        };
        let offset = read_u64(&mut cur)?;
        let size = read_u64(&mut cur)?;
        let uncompressed_size = read_u64(&mut cur)?;
        let compression = if version >= 3 { read_u32(&mut cur)? } else { read_u32(&mut cur)? };
        let mut hash = [0u8; 20];
        if cur.read_exact(&mut hash).is_err() {
            break;
        }
        let mut blocks = Vec::new();
        if version >= 5 {
            if compression != 0 {
                let count = read_u32(&mut cur)?;
                for _ in 0..count {
                    let start = read_u64(&mut cur)?;
                    let end = read_u64(&mut cur)?;
                    blocks.push((start, end));
                }
            }
        }
        let entry_encrypted = if version >= 9 {
            let _block_size = read_u32(&mut cur)?;
            // 20-byte hash was already read; v9 adds flags before blocks —
            // old seasons do not reach here.
            false
        } else {
            false
        };
        entries.push(PakEntry {
            path,
            offset,
            size,
            uncompressed_size,
            compression,
            encrypted: entry_encrypted,
            blocks,
            hash,
        });
    }

    Ok(Pak {
        version,
        mount_point,
        entries,
    })
}

/// Extract the raw (possibly still compressed) bytes of an entry.
pub fn read_entry(path: &std::path::Path, e: &PakEntry) -> Result<Vec<u8>, PakError> {
    let mut f = std::fs::File::open(path).map_err(PakError::Io)?;
    f.seek(SeekFrom::Start(e.offset)).map_err(PakError::Io)?;
    let mut buf = vec![0u8; e.size as usize];
    f.read_exact(&mut buf).map_err(PakError::Io)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Battle Pass report
// ---------------------------------------------------------------------------

/// The token vocabulary of Fortnite cosmetic reward ids.
const COSMETIC_PREFIXES: &[&str] = &[
    "cid_", "bid_", "eid_", "pid_", "pickaxe_id_", "glider_id_", "umbrella_id_",
    "musicpack_id_", "spid_", "vtid_", "loadingscreen_", "contrail_id_",
    "petcarrier_", "toy_id_", "emoticon_id_", "spray_id_", "wrap_id_",
    "banner_",
];

/// Scan pak entries for cosmetic ids and season-related assets, producing
/// the JSON the web console imports.
pub fn battle_pass_report(pak_path: &std::path::Path, pak: &Pak) -> String {
    let mut found: Vec<String> = Vec::new();
    let mut season_assets: Vec<String> = Vec::new();

    for e in &pak.entries {
        let lower = e.path.to_lowercase();
        let file = lower.rsplit('/').next().unwrap_or("");
        if lower.contains("battlepass") || lower.contains("seasonitem") || lower.contains("athenaseason") {
            season_assets.push(e.path.clone());
        }
        for p in COSMETIC_PREFIXES {
            if file.starts_with(p) {
                // "cid_163_athena_commando_f_viking.uasset" -> token
                let stem = file.split('.').next().unwrap_or(file);
                if !found.iter().any(|f| f == stem) {
                    found.push(stem.to_string());
                }
                break;
            }
        }
    }
    found.sort();

    let items: Vec<String> = found
        .iter()
        .map(|t| {
            let kind = token_kind(t);
            format!("    \"{kind}:{t}\": 1")
        })
        .collect();

    format!(
        "{{\n  \"source\": \"{}\",\n  \"pakVersion\": {},\n  \"entryCount\": {},\n  \"seasonAssets\": {},\n  \"rewards\": {{\n{}\n  }}\n}}",
        pak_path.display(),
        pak.version,
        pak.entries.len(),
        serde_json_string_array(&season_assets),
        items.join(",\n")
    )
}

fn token_kind(token: &str) -> &'static str {
    match token.split('_').next().unwrap_or("") {
        "cid" => "AthenaCharacter",
        "bid" => "AthenaBackpack",
        "eid" => "AthenaDance",
        "pid" | "pickaxe" => "AthenaPickaxe",
        "glider" | "umbrella" => "AthenaGlider",
        "musicpack" => "AthenaMusicPack",
        "spid" => "AthenaLoadingScreen",
        "vtid" => "AthenaVehicleSkin",
        "loadingscreen" => "AthenaLoadingScreen",
        "contrail" => "AthenaContrail",
        "petcarrier" => "AthenaPet",
        "toy" => "AthenaToy",
        "emoticon" => "AthenaEmoji",
        "spray" => "AthenaSpray",
        "wrap" => "AthenaItemWrap",
        "banner" => "HomebaseBanner",
        _ => "AthenaCosmetic",
    }
}

fn serde_json_string_array(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().take(64).map(|s| format!("\"{}\"", s.replace('\\', "/"))).collect();
    format!("[{}]", inner.join(","))
}

/// List entries matching a simple case-insensitive substring pattern.
pub fn find_entries<'a>(pak: &'a Pak, pattern: &str) -> Vec<&'a PakEntry> {
    let p = pattern.to_lowercase();
    pak.entries
        .iter()
        .filter(|e| e.path.to_lowercase().contains(&p))
        .collect()
}

/// Extract matching entries into an output directory.
pub fn extract_matching(
    pak_path: &std::path::Path,
    pak: &Pak,
    pattern: &str,
    out_dir: &std::path::Path,
) -> usize {
    let mut count = 0usize;
    for e in find_entries(pak, pattern) {
        let rel = e.path.replace('\\', "/");
        // Keep only the file name to avoid path escapes.
        let name = rel.rsplit('/').next().unwrap_or("entry.bin");
        let out = out_dir.join(name);
        if read_entry(pak_path, e).map(|b| std::fs::write(out, b)).is_ok() {
            count += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

pub fn main_entry() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut pak_path: Option<PathBuf> = None;
    let mut pattern = String::new();
    let mut out_dir = PathBuf::from("owen-pak-out");
    let mut mode = "list".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--battlepass" => mode = "battlepass".into(),
            "--extract" => {
                mode = "extract".into();
                i += 1;
                if let Some(p) = args.get(i) {
                    pattern = p.clone();
                }
            }
            "--out" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    out_dir = PathBuf::from(p);
                }
            }
            other => pak_path = Some(PathBuf::from(other)),
        }
        i += 1;
    }

    let Some(pak_path) = pak_path else {
        eprintln!("usage: owen-pak <file.pak> [--battlepass | --extract <pattern> --out DIR]");
        return std::process::ExitCode::from(1);
    };

    match open(&pak_path) {
        Ok(pak) => {
            println!(
                "[owen-pak] {} — version {} mount '{}' entries {}",
                pak_path.display(),
                pak.version,
                pak.mount_point,
                pak.entries.len()
            );
            match mode.as_str() {
                "battlepass" => {
                    println!("{}", battle_pass_report(&pak_path, &pak));
                }
                "extract" => {
                    std::fs::create_dir_all(&out_dir).ok();
                    let n = extract_matching(&pak_path, &pak, &pattern, &out_dir);
                    println!("[owen-pak] extracted {n} entries to {}", out_dir.display());
                }
                _ => {
                    for e in pak.entries.iter().take(80) {
                        println!("  {} ({} bytes)", e.path, e.uncompressed_size);
                    }
                    if pak.entries.len() > 80 {
                        println!("  ... and {} more", pak.entries.len() - 80);
                    }
                }
            }
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[owen-pak] {e}");
            std::process::ExitCode::from(1)
        }
    }
}
