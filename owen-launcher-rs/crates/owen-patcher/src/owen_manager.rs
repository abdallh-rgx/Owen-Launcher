//! Owen injection manager — the literal Rust port of `OwenManager.kt`
//! (the patch decision core; on-device APK installation stays with the
//! Android launcher, which calls this crate through `owen-patcher-jni`).
//!
//! ## Kotlin source map
//!
//! | Kotlin (`OwenManager.kt`)                        | Rust (here)                            |
//! |--------------------------------------------------|----------------------------------------|
//! | `InjectionResult` (sealed class)                 | [`InjectionResult`]                    |
//! | `PREFS`/`KEY_PATCHED`/`KEY_URL` (SharedPreferences) | JSON marks file (memory + file), [`OwenManager::mark_patched`] etc. |
//! | `isPatchedByUs` / `getPatchedUrl`                | [`OwenManager::is_patched_by_us`] / [`OwenManager::get_patched_url`] |
//! | `markPatched` / `clearMark`                      | [`OwenManager::mark_patched`] / [`OwenManager::clear_mark`] |
//! | `packageNameFor(version)`                        | [`package_name_for`]                   |
//! | `computeRenamedPackage(version, versionName)`    | [`compute_renamed_package`]            |
//! | `isPartyHubVersion(version)`                     | [`is_party_hub_version`]               |
//! | `versionMatches(versionName, versionCode, v)`    | [`version_matches`]                    |
//! | `installedLibEquals(apkFile, ours)`              | [`installed_lib_equals`]               |
//! | `ensureInjected` decision block (marked/hasLib/libEquals) | [`OwenManager::needs_patch`]   |
//! | fast re-patch path (`ApkPatcher.swapLibAndSign`) | [`OwenManager::fast_repatch`] → [`crate::swap_lib_and_sign`] |
//! | `VersionCatalog.OgVersion`                       | [`OgVersion`]                          |
//!
//! Not ported (Android platform APIs, no Rust equivalent in this crate):
//! `findInstalledPackage`, `installedApkPath`, `uninstall`,
//! `PackageInstallHelper.install`, `reinstallFromCache` — the caller
//! (launcher/JNI) feeds the installed APK path in and installs the result.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/owen/OwenManager.kt

use crate::ziprw::ZipArchive;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The `lib/arm64-v8a/libOwen.so` entry every patched APK carries
/// (`OwenManager.OWEN_LIB_ENTRY`, identical to `ApkPatcher.OWEN_LIB_ENTRY`).
pub const OWEN_LIB_ENTRY: &str = "lib/arm64-v8a/libOwen.so";
const PREFS: &str = "owen_manager";
/// SharedPreferences key prefixes from the Kotlin original (`patched_<pkg>` /
/// `url_<pkg>`); kept for documentation — the JSON mark struct below carries
/// both values per package.
#[allow(dead_code)]
const KEY_PATCHED: &str = "patched_";
#[allow(dead_code)]
const KEY_URL: &str = "url_";

/// `VersionCatalog.OgVersion` — the fields the manager and the patch cache
/// actually consume (season, version, cl, isDev, fixedPackage).
///
/// Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/VersionCatalog.kt
#[derive(Debug, Clone, PartialEq)]
pub struct OgVersion {
    pub season: i32,
    pub version: String,
    pub cl: i64,
    pub is_dev: bool,
    pub fixed_package: String,
}

/// `OwenManager.InjectionResult` (sealed class) — the outcome of
/// [`OwenManager::needs_patch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionResult {
    /// Already injected with this backend URL.
    UpToDate,
    /// The Owen library changed — a fast re-patch (`swapLibAndSign`) is
    /// enough.
    Reinstalled,
    /// Full patch required.
    Injected(String),
    /// Patching failed.
    Error(String),
    /// Signature conflict — the game must be uninstalled first.
    NeedsCleanReinstall,
}

/// One package mark: `patched_<pkg> = true` + `url_<pkg> = backend`
/// (SharedPreferences in the Kotlin original, JSON here).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Mark {
    #[serde(default)]
    patched: bool,
    #[serde(default)]
    url: Option<String>,
}

/// The manager state, persisted as `<base>/owen_manager.json`.
pub struct OwenManager {
    state_path: PathBuf,
    marks: BTreeMap<String, Mark>,
}

impl OwenManager {
    /// Open (and load) the manager state from `<base>/owen_manager.json`.
    pub fn open(base: &Path) -> Self {
        let state_path = base.join(format!("{PREFS}.json"));
        let marks = std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        OwenManager { state_path, marks }
    }

    fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&self.marks) {
            std::fs::write(&self.state_path, text).ok();
        }
    }

    /// `OwenManager.isPatchedByUs`.
    pub fn is_patched_by_us(&self, pkg: &str) -> bool {
        self.marks.get(pkg).map(|m| m.patched).unwrap_or(false)
    }

    /// `OwenManager.getPatchedUrl`.
    pub fn get_patched_url(&self, pkg: &str) -> Option<String> {
        self.marks.get(pkg).and_then(|m| m.url.clone())
    }

    /// `OwenManager.markPatched` (`patched_<pkg>` + `url_<pkg>`).
    pub fn mark_patched(&mut self, pkg: &str, url: &str) {
        self.marks.insert(
            pkg.to_string(),
            Mark {
                patched: true,
                url: Some(url.to_string()),
            },
        );
        self.save();
    }

    /// `OwenManager.clearMark`.
    pub fn clear_mark(&mut self, pkg: &str) {
        self.marks.remove(pkg);
        self.save();
    }

    /// The `ensureInjected` decision core: is the installed APK already
    /// injected with this backend URL (UpToDate), does only the Owen library
    /// differ (Reinstalled → fast re-patch), or is a full patch needed
    /// (Injected placeholder)?
    ///
    /// `our_lib` is the freshly built `libOwen.so`
    /// (`NativeBuilder.buildOwen`); `None` mirrors the Kotlin
    /// "libOwen.so is not available" path which forces a full patch.
    pub fn needs_patch(
        &self,
        pkg: &str,
        apk: &Path,
        backend_url: &str,
        our_lib: Option<&[u8]>,
    ) -> InjectionResult {
        let marked = self.is_patched_by_us(pkg)
            && self.get_patched_url(pkg).as_deref() == Some(backend_url);
        if marked && crate::apk_has_owen_lib(apk) {
            if let Some(ours) = our_lib {
                if installed_lib_equals(apk, ours) {
                    return InjectionResult::UpToDate;
                }
                return InjectionResult::Reinstalled;
            }
        }
        InjectionResult::Injected(String::new())
    }
}

/// `OwenManager.packageNameFor`.
pub fn package_name_for(version: &OgVersion) -> String {
    if !version.fixed_package.is_empty() {
        version.fixed_package.clone()
    } else {
        crate::ORIGINAL_PACKAGE.to_string()
    }
}

/// `OwenManager.computeRenamedPackage`.
pub fn compute_renamed_package(version: &OgVersion, version_name: Option<&str>) -> Option<String> {
    if !version.fixed_package.is_empty() || version.cl <= 0 {
        return None;
    }
    // Kotlin: versionName?.substringBefore('-')?.takeIf { it.isNotBlank() } ?: version.version
    let base = version_name
        .map(|v| v.split('-').next().unwrap_or(""))
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(&version.version);
    let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    Some(format!("{}{}{}", crate::ORIGINAL_PACKAGE, digits, version.cl))
}

/// `OwenManager.isPartyHubVersion` — seasons 12–15 shipped the Kairos
/// PartyHub app.
pub fn is_party_hub_version(version: &OgVersion) -> bool {
    let major = version
        .version
        .split('.')
        .next()
        .and_then(|m| m.parse::<i64>().ok())
        .unwrap_or(0);
    (12..=15).contains(&major)
}

/// `OwenManager.versionMatches`.
pub fn version_matches(version_name: &str, version_code: i64, v: &OgVersion) -> bool {
    let before_dash = version_name.split('-').next().unwrap_or("").trim();
    let mut parts = before_dash.split('.');
    let season = parts.next().and_then(|p| p.parse::<i64>().ok());
    let Some(season) = season else {
        return false;
    };
    let minor = parts.next().and_then(|p| p.parse::<i64>().ok()).unwrap_or(0);
    let Some(version_minor) = v.version.split('.').nth(1).and_then(|p| p.parse::<i64>().ok())
    else {
        return false;
    };
    if season != v.season as i64 || minor != version_minor {
        return false;
    }
    version_code == v.cl || version_code <= 0 || v.cl == 0
}

/// `OwenManager.installedLibEquals` — byte-compare the APK's embedded
/// `libOwen.so` with the freshly built one.
pub fn installed_lib_equals(apk: &Path, ours: &[u8]) -> bool {
    let Ok(zip) = ZipArchive::open(apk) else {
        return false;
    };
    match zip.read_entry(OWEN_LIB_ENTRY) {
        Some(installed) => installed == ours,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(version: &str, cl: i64, fixed: &str) -> OgVersion {
        OgVersion {
            season: version.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0),
            version: version.into(),
            cl,
            is_dev: false,
            fixed_package: fixed.into(),
        }
    }

    #[test]
    fn package_names() {
        assert_eq!(package_name_for(&v("12.41", 1, "")), "com.epicgames.fortnite");
        assert_eq!(package_name_for(&v("12.41", 1, "com.fixed.pkg")), "com.fixed.pkg");
    }

    #[test]
    fn renamed_packages() {
        let ver = v("12.41", 14009477, "");
        assert_eq!(
            compute_renamed_package(&ver, Some("12.41-CL-14009477")).as_deref(),
            Some("com.epicgames.fortnite124114009477")
        );
        // versionName missing → digits come from the version string
        assert_eq!(
            compute_renamed_package(&ver, None).as_deref(),
            Some("com.epicgames.fortnite124114009477")
        );
        // fixed package or cl <= 0 → None
        assert_eq!(compute_renamed_package(&v("12.41", 1, "com.fixed.pkg"), None), None);
        assert_eq!(compute_renamed_package(&v("12.41", 0, ""), None), None);
    }

    #[test]
    fn party_hub_versions() {
        assert!(is_party_hub_version(&v("12.41", 1, "")));
        assert!(is_party_hub_version(&v("15.30", 1, "")));
        assert!(!is_party_hub_version(&v("11.31", 1, "")));
        assert!(!is_party_hub_version(&v("16.00", 1, "")));
    }

    #[test]
    fn version_matching() {
        let ver = v("12.41", 14009477, "");
        assert!(version_matches("12.41-CL-14009477", 14009477, &ver));
        assert!(version_matches("12.41", 0, &ver), "unknown versionCode tolerated");
        assert!(!version_matches("12.40-CL-1", 14009477, &ver));
        assert!(!version_matches("13.41", 14009477, &ver));
        assert!(!version_matches("garbage", 14009477, &ver));
    }

    #[test]
    fn marks_persist_across_open() {
        let dir = std::env::temp_dir().join(format!("owen-mgr-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut mgr = OwenManager::open(&dir);
        assert!(!mgr.is_patched_by_us("com.epicgames.fortnite124"));
        mgr.mark_patched("com.epicgames.fortnite124", "http://127.0.0.1:3551");
        assert!(mgr.is_patched_by_us("com.epicgames.fortnite124"));
        assert_eq!(
            mgr.get_patched_url("com.epicgames.fortnite124").as_deref(),
            Some("http://127.0.0.1:3551")
        );

        let mgr2 = OwenManager::open(&dir);
        assert!(mgr2.is_patched_by_us("com.epicgames.fortnite124"));

        let mut mgr3 = OwenManager::open(&dir);
        mgr3.clear_mark("com.epicgames.fortnite124");
        assert!(!mgr3.is_patched_by_us("com.epicgames.fortnite124"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
