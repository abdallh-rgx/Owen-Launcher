//! Auto manifest redirecter: `cloudcontent.json` ingestion and local
//! manifest generation.
//!
//! Fortnite Android reads `assets/Cloud/cloudcontent.json` to learn which
//! content manifest to download:
//!
//! ```json
//! {
//!   "AppName": "FortniteContentBuilds",
//!   "BuildVersion": "++Fortnite+Release-13.40-CL-14009477",
//!   "Platform": "Android",
//!   "ManifestPath": "Builds/Fortnite/Content/CloudDir/cZ80wF54SdibOCVxa0GPXH4IDBoUzA.manifest",
//!   "ManifestHash": "dfa24b53324c3e106718dcac5d9e787fc12b4ba9"
//! }
//! ```
//!
//! The game then asks the launcher API
//! `/launcher/api/public/assets/:platform/:label/:appName/:buildVersion`
//! (on `ol.epicgames.com`, which Owen redirects) for the manifest URL.
//! This module builds the answer: either a live-CDN passthrough or a
//! locally-served manifest.

use crate::version::FortniteVersion;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const LIVE_CDN_BASE: &str = "https://download.akamaized.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudContent {
    #[serde(rename = "AppName", default)]
    pub app_name: String,
    #[serde(rename = "BuildVersion", default)]
    pub build_version: String,
    #[serde(rename = "Platform", default)]
    pub platform: String,
    #[serde(rename = "ManifestPath", default)]
    pub manifest_path: String,
    #[serde(rename = "ManifestHash", default)]
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Human version, e.g. "13.40".
    pub version: String,
    pub release_string: String,
    /// Where the manifest itself is served from.
    pub manifest_url: String,
    pub source: ManifestSource,
    pub cloud_content: Option<CloudContent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestSource {
    /// Generated from the APK's embedded cloudcontent.json.
    ApkCloudContent,
    /// Generated from the version catalog with the known live manifest.
    Catalog,
    /// Uploaded manually.
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCatalog {
    pub entries: Vec<ManifestEntry>,
}

impl ManifestCatalog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Register a cloudcontent.json payload (extracted from an APK).
    pub fn add_cloud_content(&mut self, cc: CloudContent) -> &mut ManifestEntry {
        let version = parse_version(&cc.build_version).unwrap_or_else(|| cc.build_version.clone());
        let manifest_url = format!("{}/{}", LIVE_CDN_BASE, cc.manifest_path.trim_start_matches('/'));
        let entry = ManifestEntry {
            version,
            release_string: cc.build_version.clone(),
            manifest_url,
            source: ManifestSource::ApkCloudContent,
            cloud_content: Some(cc),
        };
        upsert(&mut self.entries, entry)
    }

    /// Register a version by catalog (no APK cloud content available).
    pub fn add_version(&mut self, version: &FortniteVersion, manifest_path: Option<String>) -> &mut ManifestEntry {
        let entry = match manifest_path {
            Some(path) => ManifestEntry {
                version: version.to_string(),
                release_string: version.release_string.clone(),
                manifest_url: format!("{}/{}", LIVE_CDN_BASE, path.trim_start_matches('/')),
                source: ManifestSource::Catalog,
                cloud_content: None,
            },
            None => ManifestEntry {
                version: version.to_string(),
                release_string: version.release_string.clone(),
                manifest_url: String::new(),
                source: ManifestSource::Catalog,
                cloud_content: None,
            },
        };
        upsert(&mut self.entries, entry)
    }

    /// Find the entry serving a given build version string.
    ///
    /// Matching is layered: exact `release_string`, then bare version, then
    /// a tolerant version+CL match — the game's label can differ in suffixes
    /// (e.g. `++Fortnite+Release-12.41-CL-12910313` without the trailing
    /// `-Android`), while the catalog entry carries the full Android label.
    pub fn find_release(&self, release: &str) -> Option<&ManifestEntry> {
        self.entries
            .iter()
            .find(|e| e.release_string == release)
            .or_else(|| self.entries.iter().find(|e| e.version == release))
            .or_else(|| {
                let (version, cl) = parse_version_cl(release)?;
                self.entries.iter().find(|e| {
                    match parse_version_cl(&e.release_string) {
                        Some((ev, ecl)) => ev == version && ecl == cl,
                        None => false,
                    }
                })
            })
    }

    /// Merge the launcher's `manifests.json` catalog (the array of
    /// [`ManifestListEntry`] objects shipped under `data/owen`) into this
    /// catalog, upserting by release string so manual/admin entries can
    /// still override. Returns the number of list entries merged.
    pub fn merge_manifest_list(&mut self, data: &[u8]) -> Result<usize, String> {
        let list: Vec<ManifestListEntry> =
            serde_json::from_slice(data).map_err(|e| e.to_string())?;
        let count = list.len();
        for item in list {
            let entry = item.into_entry();
            upsert(&mut self.entries, entry);
        }
        Ok(count)
    }
}

/// One entry of the launcher's `manifests.json` catalog file — the known
/// live CloudDir manifest per Fortnite Android build (see
/// `data/owen/manifests.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestListEntry {
    /// Human version, e.g. "12.41".
    pub version: String,
    /// Changelist, e.g. "12910313".
    #[serde(default)]
    pub cl: String,
    /// Full build label, e.g. `++Fortnite+Release-12.41-CL-12910313-Android`.
    #[serde(rename = "releaseString", default)]
    pub release_string: String,
    /// Manifest file stem on the CDN (`CloudDir/<hash>.manifest`).
    #[serde(rename = "manifestHash", default)]
    pub manifest_hash: String,
    /// Short label, e.g. "12.41-CL-12910313".
    #[serde(default)]
    pub label: String,
    /// Release date (informational).
    #[serde(default)]
    pub date: String,
    /// Full https URL of the live manifest on Epic's akamaized CDN.
    #[serde(default)]
    pub cdn: String,
}

impl ManifestListEntry {
    /// Convert to the runtime [`ManifestEntry`] shape: the `cdn` URL becomes
    /// the manifest URL (returned verbatim as the launcher-asset
    /// `distribution`), and the hash rides along as embedded cloud content
    /// so [`launcher_asset_response`] reports it.
    fn into_entry(self) -> ManifestEntry {
        let release_string = if self.release_string.is_empty() {
            format!("++Fortnite+Release-{}-CL-{}-Android", self.version, self.cl)
        } else {
            self.release_string
        };
        // `https://host/Builds/.../x.manifest` → `Builds/.../x.manifest`
        let manifest_path = self.cdn.splitn(4, '/').nth(3).unwrap_or("").to_string();
        let cloud_content = if self.manifest_hash.is_empty() && manifest_path.is_empty() {
            None
        } else {
            Some(CloudContent {
                app_name: "FortniteContentBuilds".into(),
                build_version: release_string.clone(),
                platform: "Android".into(),
                manifest_path,
                manifest_hash: self.manifest_hash,
            })
        };
        ManifestEntry {
            version: self.version,
            release_string,
            manifest_url: self.cdn,
            source: ManifestSource::Catalog,
            cloud_content,
        }
    }
}

fn upsert(entries: &mut Vec<ManifestEntry>, entry: ManifestEntry) -> &mut ManifestEntry {
    if let Some(idx) = entries
        .iter()
        .position(|e| e.release_string == entry.release_string)
    {
        entries[idx] = entry;
        &mut entries[idx]
    } else {
        entries.push(entry);
        entries.last_mut().expect("just pushed")
    }
}

fn parse_version(release: &str) -> Option<String> {
    let v = release
        .strip_prefix("++Fortnite+Release-")?
        .split("-CL-")
        .next()?;
    Some(v.to_string())
}

/// `++Fortnite+Release-12.41-CL-12910313-Android` → (`"12.41"`, `"12910313"`).
/// Returns `None` for strings that are not full version+CL build labels
/// (bare versions, manual manifest URLs, ...).
fn parse_version_cl(release: &str) -> Option<(String, String)> {
    let rest = release.strip_prefix("++Fortnite+Release-")?;
    let mut parts = rest.splitn(2, "-CL-");
    let version = parts.next()?;
    let cl = parts.next()?.split('-').next()?;
    if version.is_empty() || cl.is_empty() {
        return None;
    }
    Some((version.to_string(), cl.to_string()))
}

impl CloudContent {
    /// Parse from raw JSON bytes.
    pub fn from_json(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| e.to_string())
    }

    /// Read `assets/Cloud/cloudcontent.json` out of an extracted APK dir.
    pub fn from_apk_dir(dir: &Path) -> Option<Self> {
        let p: PathBuf = dir.join("assets/Cloud/cloudcontent.json");
        let data = std::fs::read(p).ok()?;
        Self::from_json(&data).ok()
    }
}

/// The launcher-API response shape for asset queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherAssetResponse {
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "labelName")]
    pub label_name: String,
    #[serde(rename = "buildVersion")]
    pub build_version: String,
    pub assets: Vec<LauncherAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherAsset {
    pub appName: String,
    pub labelName: String,
    pub buildVersion: String,
    /// The item kind — manifests are distributed as "MANIFEST" items.
    pub itemName: String,
    /// Format of the distribution info.
    pub itemFormat: String,
    /// Distribution info: for HTTP manifests this is the manifest URL.
    pub distribution: String,
    pub hash: String,
}

/// Build the `/launcher/api/public/assets/...` response for a manifest
/// entry.
pub fn launcher_asset_response(entry: &ManifestEntry, label: &str) -> LauncherAssetResponse {
    LauncherAssetResponse {
        app_name: "FortniteContentBuilds".into(),
        label_name: label.to_string(),
        build_version: entry.release_string.clone(),
        assets: vec![LauncherAsset {
            appName: "FortniteContentBuilds".into(),
            labelName: label.to_string(),
            buildVersion: entry.release_string.clone(),
            itemName: "CloudDir".into(),
            itemFormat: "HTTP".into(),
            distribution: entry.manifest_url.clone(),
            hash: entry
                .cloud_content
                .as_ref()
                .map(|c| c.manifest_hash.clone())
                .unwrap_or_default(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CDN_1241: &str = "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir/aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg.manifest";

    fn catalog_1241() -> ManifestCatalog {
        let mut catalog = ManifestCatalog::new();
        let count = catalog
            .merge_manifest_list(
                br#"[
                    {
                      "version": "12.41",
                      "cl": "12910313",
                      "releaseString": "++Fortnite+Release-12.41-CL-12910313-Android",
                      "manifestHash": "aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg",
                      "label": "12.41-CL-12910313",
                      "date": "2020-02-20",
                      "cdn": "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir/aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg.manifest"
                    },
                    {
                      "version": "12.41",
                      "cl": "12948829",
                      "releaseString": "++Fortnite+Release-12.41-CL-12948829-Android",
                      "manifestHash": "qEuvqZqHRP9IipPXDUK-c7g2-oa6Zg",
                      "label": "12.41-CL-12948829",
                      "date": "2020-02-20",
                      "cdn": "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir/qEuvqZqHRP9IipPXDUK-c7g2-oa6Zg.manifest"
                    }
                ]"#,
            )
            .expect("merge");
        assert_eq!(count, 2);
        catalog
    }

    #[test]
    fn manifest_list_entry_becomes_cdn_backed_entry() {
        let catalog = catalog_1241();
        assert_eq!(catalog.entries.len(), 2);
        let e = &catalog.entries[0];
        assert_eq!(e.version, "12.41");
        assert_eq!(e.release_string, "++Fortnite+Release-12.41-CL-12910313-Android");
        assert_eq!(e.manifest_url, CDN_1241);
        let cc = e.cloud_content.as_ref().expect("cloud content");
        assert_eq!(cc.manifest_hash, "aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg");
        assert_eq!(
            cc.manifest_path,
            "Builds/Fortnite/Content/CloudDir/aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg.manifest"
        );
    }

    #[test]
    fn find_release_matches_exact_version_and_cl_tolerance() {
        let catalog = catalog_1241();
        // exact build label (what the 12.41 Android client sends)
        let e = catalog
            .find_release("++Fortnite+Release-12.41-CL-12910313-Android")
            .expect("exact label");
        assert_eq!(e.manifest_url, CDN_1241);
        // the second 12.41 build resolves to its own manifest
        let e = catalog
            .find_release("++Fortnite+Release-12.41-CL-12948829-Android")
            .expect("second build");
        assert!(e.manifest_url.ends_with("qEuvqZqHRP9IipPXDUK-c7g2-oa6Zg.manifest"));
        // tolerant: same version+CL without the `-Android` suffix
        let e = catalog
            .find_release("++Fortnite+Release-12.41-CL-12910313")
            .expect("version+cl tolerance");
        assert_eq!(e.manifest_url, CDN_1241);
        // bare version still matches (first entry of that version)
        assert!(catalog.find_release("12.41").is_some());
        // unknown build → None
        assert!(catalog.find_release("++Fortnite+Release-13.40-CL-14009477-Android").is_none());
    }

    #[test]
    fn merge_manifest_list_upserts_by_release_string() {
        let mut catalog = catalog_1241();
        // admin-style override of the same build → replaces, keeps count
        let count = catalog
            .merge_manifest_list(
                br#"[{"version": "12.41", "cl": "12910313",
                     "releaseString": "++Fortnite+Release-12.41-CL-12910313-Android",
                     "manifestHash": "override", "label": "x", "date": "2020-02-20",
                     "cdn": "https://example.com/override.manifest"}]"#,
            )
            .expect("merge");
        assert_eq!(count, 1);
        assert_eq!(catalog.entries.len(), 2);
        let e = catalog
            .find_release("++Fortnite+Release-12.41-CL-12910313-Android")
            .unwrap();
        assert_eq!(e.manifest_url, "https://example.com/override.manifest");
    }

    #[test]
    fn launcher_asset_response_carries_the_cdn_distribution() {
        let catalog = catalog_1241();
        let e = catalog
            .find_release("++Fortnite+Release-12.41-CL-12910313-Android")
            .unwrap();
        let resp = launcher_asset_response(e, "Live");
        assert_eq!(resp.app_name, "FortniteContentBuilds");
        assert_eq!(resp.label_name, "Live");
        assert_eq!(resp.build_version, "++Fortnite+Release-12.41-CL-12910313-Android");
        assert_eq!(resp.assets.len(), 1);
        let a = &resp.assets[0];
        assert_eq!(a.itemName, "CloudDir");
        assert_eq!(a.itemFormat, "HTTP");
        assert_eq!(a.distribution, CDN_1241);
        assert_eq!(a.hash, "aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg");
    }

    #[test]
    fn parse_version_cl_shapes() {
        assert_eq!(
            parse_version_cl("++Fortnite+Release-12.41-CL-12910313-Android"),
            Some(("12.41".into(), "12910313".into()))
        );
        assert_eq!(
            parse_version_cl("++Fortnite+Release-12.41-CL-12910313"),
            Some(("12.41".into(), "12910313".into()))
        );
        assert_eq!(parse_version_cl("12.41"), None);
        assert_eq!(parse_version_cl("https://example.com/x.manifest"), None);
        assert_eq!(parse_version_cl("++Fortnite+Release-12.41"), None);
    }
}
