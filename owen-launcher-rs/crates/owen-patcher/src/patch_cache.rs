//! Patched-APK cache for fast re-patching — the literal Rust port of
//! `PatchCache.kt`, plus a JSON index (in memory + file) that tracks the
//! cached builds.
//!
//! ## Kotlin source map
//!
//! | Kotlin (`PatchCache.kt`)                     | Rust (here)                        |
//! |----------------------------------------------|------------------------------------|
//! | `CACHE_VERSION = 2`                          | [`CACHE_VERSION`]                  |
//! | `MIN_VALID_BYTES = 2_000_000L`               | [`MIN_VALID_BYTES`]                |
//! | `key(version, cl, isDev, backendUrl, loginUrl, isPartyHub)` | [`key`]           |
//! | `keyFor(version: OgVersion, …)`              | [`key_for`]                        |
//! | `lookup(context, cacheKey)`                  | [`PatchCache::lookup`]             |
//! | `store(context, cacheKey, signedApk)`        | [`PatchCache::store`]              |
//! | `clear(context)`                             | [`PatchCache::clear`]              |
//! | `dir(context)` = filesDir/patch_cache        | [`PatchCache::open`] base dir      |
//! | `context.getSharedPreferences` (implicit)    | JSON index `index.json` (memory + file) |
//!
//! The Kotlin "cache" was purely file-name based; the Rust port additionally
//! remembers `(apk, size, package)` per key in `index.json` so callers can
//! enumerate cached builds without scanning the directory.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/PatchCache.kt

use crate::owen_manager::OgVersion;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CACHE_VERSION: u32 = 2;
pub const MIN_VALID_BYTES: u64 = 2_000_000;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// One cached patched APK (the JSON index entry).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// File name of the cached APK (`owen_<cacheKey>.apk`).
    pub apk: String,
    /// Size in bytes at store time.
    pub bytes: u64,
    /// Package name the patched manifest carries (when known).
    pub package: Option<String>,
}

/// `PatchCache` — anchored at `<base>/patch_cache`, index at
/// `<base>/patch_cache/index.json`.
pub struct PatchCache {
    dir: PathBuf,
    index: BTreeMap<String, CacheEntry>,
}

impl PatchCache {
    /// Open (and load) the cache rooted at `base` (`context.filesDir` in the
    /// Kotlin original).
    pub fn open(base: &Path) -> Self {
        let dir = base.join("patch_cache");
        let index = std::fs::read_to_string(dir.join("index.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        PatchCache { dir, index }
    }

    /// The cache directory (created on demand), mirroring `dir(context)`.
    fn dir(&self) -> Result<&Path, String> {
        if !self.dir.is_dir() {
            std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        }
        Ok(&self.dir)
    }

    fn apk_path(&self, cache_key: &str) -> PathBuf {
        self.dir.join(format!("owen_{cache_key}.apk"))
    }

    /// `PatchCache.lookup` — `Some(path)` only when the cached APK still
    /// exists and is at least [`MIN_VALID_BYTES`] long.
    pub fn lookup(&self, cache_key: &str) -> Option<PathBuf> {
        let apk = self.apk_path(cache_key);
        let len = std::fs::metadata(&apk).ok()?.len();
        if len < MIN_VALID_BYTES {
            return None;
        }
        Some(apk)
    }

    /// `PatchCache.store` — move (or copy) `signed_apk` into the cache under
    /// `cache_key` and record it in the JSON index.
    pub fn store(&mut self, cache_key: &str, signed_apk: &Path) -> Result<PathBuf, String> {
        let dir = self.dir()?;
        let target = dir.join(format!("owen_{cache_key}.apk"));
        if target == signed_apk {
            return Ok(target);
        }
        if target.is_file() {
            std::fs::remove_file(&target).map_err(|e| e.to_string())?;
        }
        if std::fs::rename(signed_apk, &target).is_err() {
            std::fs::copy(signed_apk, &target).map_err(|e| e.to_string())?;
            std::fs::remove_file(signed_apk).ok();
        }
        let bytes = std::fs::metadata(&target).map_err(|e| e.to_string())?.len();
        let package = crate::manifest_renamer::read_package_name_from_apk(&target);
        self.index.insert(
            cache_key.to_string(),
            CacheEntry {
                apk: target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                bytes,
                package,
            },
        );
        self.save_index()?;
        Ok(target)
    }

    /// `PatchCache.clear` — wipe the cache directory.
    pub fn clear(&mut self) -> Result<(), String> {
        self.index.clear();
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Entries of the in-memory JSON index.
    pub fn entries(&self) -> &BTreeMap<String, CacheEntry> {
        &self.index
    }

    fn save_index(&self) -> Result<(), String> {
        let dir = self.dir()?;
        let text = serde_json::to_string_pretty(&self.index).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("index.json"), text).map_err(|e| e.to_string())
    }
}

/// `PatchCache.key(version, cl, isDev, backendUrl, loginUrl, isPartyHub)` —
/// SHA-256 over the pipe-joined parts, hex, first 16 chars.
pub fn key(
    version: &str,
    cl: i64,
    is_dev: bool,
    backend_url: &str,
    login_url: &str,
    is_party_hub: bool,
) -> String {
    let effective_login = if is_party_hub { login_url } else { "" };
    let joined = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        CACHE_VERSION, version, cl, is_dev, backend_url, effective_login, is_party_hub
    );
    let digest = Sha256::digest(joined.as_bytes());
    let mut sb = String::with_capacity(32);
    for b in digest {
        sb.push(HEX[(b >> 4) as usize] as char);
        sb.push(HEX[(b & 0xF) as usize] as char);
    }
    sb.chars().take(16).collect()
}

/// `PatchCache.keyFor(version: OgVersion, backendUrl, loginUrl)` — the
/// PartyHub flag is derived from the major version (seasons 12–15).
pub fn key_for(version: &OgVersion, backend_url: &str, login_url: &str) -> String {
    let major = version
        .version
        .split('.')
        .next()
        .and_then(|m| m.parse::<i64>().ok())
        .unwrap_or(0);
    key(
        &version.version,
        version.cl,
        version.is_dev,
        backend_url,
        login_url,
        (12..=15).contains(&major),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_stable_and_16_hex_chars() {
        let a = key("12.41", 123456, false, "http://127.0.0.1:3551", "http://x/login", true);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, key("12.41", 123456, false, "http://127.0.0.1:3551", "http://x/login", true));
        // login only counts for party-hub builds
        assert_ne!(a, key("12.41", 123456, false, "http://127.0.0.1:3551", "http://x/login", false));
        // cache version bumps change the key
        assert_ne!(a, key("12.41", 123456, true, "http://127.0.0.1:3551", "http://x/login", true));
    }

    #[test]
    fn key_for_derives_partyhub_from_major() {
        let v = OgVersion {
            season: 12,
            version: "12.41".into(),
            cl: 14009477,
            is_dev: false,
            fixed_package: String::new(),
        };
        let a = key_for(&v, "http://127.0.0.1:3551", "http://x/login");
        let expected = key("12.41", 14009477, false, "http://127.0.0.1:3551", "http://x/login", true);
        assert_eq!(a, expected);
    }

    #[test]
    fn store_lookup_round_trip_respects_min_size() {
        let dir = std::env::temp_dir().join(format!("owen-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = PatchCache::open(&dir);

        // A tiny APK: stored, but below MIN_VALID_BYTES → lookup misses.
        let small = dir.join("small.apk");
        std::fs::write(&small, vec![0u8; 1024]).unwrap();
        let target = cache.store("k1", &small).unwrap();
        assert!(!small.exists(), "store renames the source when possible");
        assert!(target.is_file());
        assert!(cache.lookup("k1").is_none(), "below MIN_VALID_BYTES");

        // A large APK: stored + lookup hit + index entry.
        let big = dir.join("big.apk");
        std::fs::write(&big, vec![7u8; MIN_VALID_BYTES as usize]).unwrap();
        let target = cache.store("k2", &big).unwrap();
        assert_eq!(cache.lookup("k2"), Some(target));
        let entry = cache.entries().get("k2").unwrap();
        assert_eq!(entry.bytes, MIN_VALID_BYTES);

        // clear wipes everything.
        cache.clear().unwrap();
        assert!(cache.entries().is_empty());
        assert!(cache.lookup("k2").is_none());

        // The index survives reopen (JSON file).
        let mut cache2 = PatchCache::open(&dir);
        assert!(cache2.entries().is_empty());
        let big2 = dir.join("big2.apk");
        std::fs::write(&big2, vec![9u8; MIN_VALID_BYTES as usize]).unwrap();
        cache2.store("k3", &big2).unwrap();
        let cache3 = PatchCache::open(&dir);
        assert!(cache3.entries().contains_key("k3"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
