#![forbid(unsafe_code)]
//! Runtime direct-APK-URL overrides — the small seam the E2E pipeline (and
//! anyone who wants to pin a build to a mirror) uses instead of the Uptodown
//! scrape for a given version.
//!
//! The generated [`super::version_catalog`] bakes `direct_apk_url` in at
//! compile time (every real entry ships `""`, so the installer falls back to
//! the `UptodownResolver` HTML scrape — brittle behind Cloudflare and useless
//! when CI needs a *pinned* APK). This module layers a file-driven override
//! on top of the static catalog, without touching the generated list:
//!
//! * the app reads `owen-direct.json` from its data root (Android:
//!   `/data/data/com.abdallhrgx.owenlauncher/files/owen-direct.json`) and,
//!   on Android, also from the external files dir
//!   (`/sdcard/Android/data/com.abdallhrgx.owenlauncher/files/owen-direct.json`
//!   — reachable with a file manager on non-rooted devices);
//! * entries can **override the URL of an existing build** (matched on
//!   `version` + `cl`) or **add a brand-new build** to a season (the catalog
//!   has no 12.41 entry at all, for instance);
//! * [`by_season_merged`] returns the same `&'static OgVersion` the UI already
//!   consumes, so `details_screen` only swaps its two `by_season` calls.
//!
//! File format (one object, `direct` list; unknown fields ignored):
//!
//! ```json
//! {
//!   "direct": [
//!     {"season": 13, "version": "13.00", "cl": 13649278,
//!      "url": "https://github.com/.../Fortnite+13.00-CL-13649278-Android.apk"},
//!     {"season": 12, "version": "12.41", "cl": 12910313,
//!      "url": "https://github.com/.../Fortnite+12.41-CL-12910313-Android.apk"}
//!   ]
//! }
//! ```
//!
//! Safety notes:
//! * pure safe Rust — the only allocation trick is `Box::leak` for the few
//!   `&'static str` fields of runtime-added entries (the catalog type is all
//!   `&'static`). The leak is bounded by the file size and happens once per
//!   process (results are cached per season), so it cannot grow unboundedly.
//! * a malformed file, a missing file or an unreadable external dir all
//!   degrade to "no overrides" — the static catalog is always the base.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::version_catalog::{self, OgVersion};

/// The override file name (both candidate locations).
const OVERRIDE_FILE: &str = "owen-direct.json";
/// The launcher's own package (same literal as `install::helper::RESULT_ACTION`
/// uses; repeated here so this module stays self-contained).
#[allow(dead_code)] // used by the android branch of candidate_dirs
const PACKAGE: &str = "com.abdallhrgx.owenlauncher";

/* --------------------------------- parsing -------------------------------- */

/// One `direct` entry of the override file.
#[derive(Debug, Clone)]
pub struct DirectEntry {
    pub season: u32,
    pub version: String,
    pub cl: i64,
    pub url: String,
}

impl DirectEntry {
    /// Parse one element of the `direct` array; `None` when it is malformed
    /// or incomplete (missing/empty url, non-numeric cl, …).
    fn from_json(v: &serde_json::Value) -> Option<Self> {
        let season = v.get("season").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let version = v.get("version")?.as_str()?.trim().to_string();
        let cl = v.get("cl").and_then(|x| x.as_i64()).unwrap_or(0);
        let url = v.get("url")?.as_str()?.trim().to_string();
        if version.is_empty() || url.is_empty() || !url.starts_with("http") || cl <= 0 {
            return None;
        }
        Some(DirectEntry { season, version, cl, url })
    }
}

/// Read + parse `owen-direct.json` from `dir`. `None` when absent/bad.
fn parse_file(dir: &Path) -> Option<Vec<DirectEntry>> {
    let path = dir.join(OVERRIDE_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let root: serde_json::Value = serde_json::from_str(&text).ok()?;
    let list = root.get("direct")?.as_array()?;
    let out: Vec<DirectEntry> = list.iter().filter_map(DirectEntry::from_json).collect();
    if out.is_empty() {
        return None;
    }
    Some(out)
}

/// The candidate directories, in priority order: the app data root first
/// (Android internal files dir — what CI pushes with `adb root`), then the
/// external files dir (Android only, usable without root).
fn candidate_dirs(data_root: &Path) -> Vec<PathBuf> {
    #[allow(unused_mut)] // android pushes a second candidate
    let mut dirs = vec![data_root.to_path_buf()];
    #[cfg(target_os = "android")]
    {
        dirs.push(PathBuf::from("/sdcard/Android/data").join(PACKAGE).join("files"));
    }
    dirs
}

/* ------------------------------ merged catalog ----------------------------- */

/// Cached merged lists, keyed by (data root, season) — loaded once per
/// process per root. The app has a single data root; tests use several.
static CACHE: Mutex<Option<HashMap<(PathBuf, u32), Vec<&'static OgVersion>>>> =
    Mutex::new(None);

fn cache_get(data_root: &Path, season: u32) -> Option<Vec<&'static OgVersion>> {
    CACHE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|m| m.get(&(data_root.to_path_buf(), season)).cloned()))
}

fn cache_put(data_root: &Path, season: u32, list: Vec<&'static OgVersion>) {
    if let Ok(mut guard) = CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert((data_root.to_path_buf(), season), list);
    }
}

/// Load the overrides for every season at once (first merged query wins; the
/// file is tiny so there is no reason to keep the parsed form around).
fn load_all(data_root: &Path) -> Vec<DirectEntry> {
    for dir in candidate_dirs(data_root) {
        if let Some(entries) = parse_file(&dir) {
            return entries;
        }
    }
    Vec::new()
}

/// `Box::leak` a String into the `&'static str` the catalog type wants.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Build the `&'static OgVersion` for one override entry (leaked once, cached
/// with the merged list — bounded by the override file size, per season).
fn override_version(e: &DirectEntry) -> &'static OgVersion {
    // The release-key shape the backend manager logs and LawinConfig matches
    // on (only used for tagging; manifest_hash stays None so apply_for_version
    // keeps the current manifest).
    let release_key = leak(format!("++Fortnite+Release-{}-CL-{}", e.version, e.cl));
    let built = OgVersion {
        season: e.season,
        version: leak(e.version.clone()),
        cl: e.cl,
        release_key,
        manifest_hash: None,
        // No Uptodown id on override entries: if the URL is ever missing the
        // installer must fail loudly instead of silently scraping Uptodown.
        uptodown_id: None,
        is_dev: false,
        direct_apk_url: leak(e.url.clone()),
        fixed_package: "",
    };
    Box::leak(Box::new(built))
}

/// The static entry for `(version, cl)` with `direct_apk_url` swapped for the
/// override URL (a copy — the generated list is never mutated).
fn patched_static(base: &'static OgVersion, url: &str) -> &'static OgVersion {
    Box::leak(Box::new(OgVersion {
        direct_apk_url: leak(url.to_string()),
        ..base.clone()
    }))
}

/// `version_catalog::by_season(season)` + the `owen-direct.json` overrides:
/// static entries with their URL replaced where an override matches
/// `(version, cl)`, then brand-new override entries appended, the whole list
/// sorted by `cl` descending exactly like `by_season`.
///
/// The result is cached per season; the first call parses the file.
pub fn by_season_merged(data_root: &Path, season: u32) -> Vec<&'static OgVersion> {
    if let Some(cached) = cache_get(data_root, season) {
        return cached;
    }
    let overrides = load_all(data_root);
    let for_season: Vec<&DirectEntry> = overrides.iter().filter(|e| e.season == season).collect();

    let mut out: Vec<&'static OgVersion> = version_catalog::by_season(season);
    if !for_season.is_empty() {
        // 1) swap URLs of existing entries — matched on exact (version, cl).
        //    An override whose (version, cl) matches nothing is simply appended
        //    as a new build below, never silently re-pinned onto another entry.
        let swapped: Vec<&'static OgVersion> = out
            .iter()
            .map(|base| {
                let hit = for_season
                    .iter()
                    .find(|e| e.version == base.version && e.cl == base.cl);
                match hit {
                    Some(e) => patched_static(base, &e.url),
                    None => *base,
                }
            })
            .collect();
        out = swapped;
        // 2) append entries that do not exist in the static catalog.
        for e in for_season {
            let exists = out
                .iter()
                .any(|v| v.version == e.version && v.cl == e.cl);
            if !exists {
                out.push(override_version(e));
            }
        }
        out.sort_by(|a, b| b.cl.cmp(&a.cl));
    }
    cache_put(data_root, season, out.clone());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("owen_direct_test_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_overrides(dir: &Path, json: &str) {
        std::fs::write(dir.join(OVERRIDE_FILE), json).unwrap();
    }

    #[test]
    fn no_file_means_static_catalog() {
        let root = tmp_root("absent");
        let s13 = by_season_merged(&root, 13);
        assert_eq!(s13.len(), version_catalog::by_season(13).len());
        assert!(s13.iter().all(|v| v.direct_apk_url.is_empty()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn url_override_and_new_entry() {
        let root = tmp_root("mixed");
        write_overrides(
            &root,
            r#"{"direct":[
                {"season":13,"version":"13.00","cl":13649278,"url":"https://ci.example/1300.apk"},
                {"season":12,"version":"12.41","cl":12910313,"url":"https://ci.example/1241.apk"}
            ]}"#,
        );
        // 13: the existing 13649278 entry is URL-swapped, count unchanged.
        let s13 = by_season_merged(&root, 13);
        assert_eq!(s13.len(), version_catalog::by_season(13).len());
        let v = s13.iter().find(|v| v.cl == 13649278).unwrap();
        assert_eq!(v.direct_apk_url, "https://ci.example/1300.apk");
        assert_eq!(v.version, "13.00");
        assert_eq!(v.season, 13);
        assert!(v.uptodown_id.is_some()); // static entry keeps its id
        // sorted by cl desc like by_season
        let cls: Vec<i64> = s13.iter().map(|v| v.cl).collect();
        let mut sorted = cls.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(cls, sorted);
        // 12: 12.41 does not exist statically → appended
        let s12 = by_season_merged(&root, 12);
        assert!(s12.iter().any(|v| v.version == "12.41" && v.cl == 12910313));
        let v41 = s12.iter().find(|v| v.version == "12.41").unwrap();
        assert_eq!(v41.direct_apk_url, "https://ci.example/1241.apk");
        assert!(v41.uptodown_id.is_none());
        assert_eq!(v41.season, 12);
        assert_eq!(v41.release_key, "++Fortnite+Release-12.41-CL-12910313");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_file_degrades_to_static() {
        let root = tmp_root("bad");
        write_overrides(&root, "{ not json at all");
        let s13 = by_season_merged(&root, 13);
        assert_eq!(s13.len(), version_catalog::by_season(13).len());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bad_entries_are_dropped() {
        let root = tmp_root("partial");
        write_overrides(
            &root,
            r#"{"direct":[
                {"season":13,"version":"13.00","cl":0,"url":"https://x/y.apk"},
                {"season":13,"version":"","cl":1,"url":"https://x/z.apk"},
                {"season":13,"version":"13.00","cl":2,"url":"notaurl"}
            ]}"#,
        );
        let s13 = by_season_merged(&root, 13);
        assert_eq!(s13.len(), version_catalog::by_season(13).len());
        let _ = std::fs::remove_dir_all(&root);
    }
}
