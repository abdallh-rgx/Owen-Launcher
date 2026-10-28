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
    pub fn find_release(&self, release: &str) -> Option<&ManifestEntry> {
        self.entries
            .iter()
            .find(|e| e.release_string == release)
            .or_else(|| self.entries.iter().find(|e| e.version == release))
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
