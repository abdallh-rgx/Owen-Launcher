//! Runtime detection: version, redirect profile and curl_easy_setopt
//! address — entirely from the mapped image, using safe reads of
//! `/proc/self/mem` through the module file when possible.

use crate::procmaps::ModuleInfo;
use owen_core::elf::ElfImage;
use owen_core::profiles::{self, RedirectMethod, VersionProfile};
use owen_core::version::FortniteVersion;

/// Everything detection produced for one process.
#[derive(Debug, Clone)]
pub struct Detection {
    pub module_path: String,
    pub version: FortniteVersion,
    pub profile: VersionProfile,
    /// Resolved `curl_easy_setopt` runtime address (base + VA) if found.
    pub curl_setopt: Option<u64>,
    /// How curl_setopt was found.
    pub curl_source: CurlSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurlSource {
    ExportedSymbol,
    AutoScan,
    None,
}

/// ELF header page size — enough to parse program headers.
const ELF_HDR: usize = 4096;

/// Parse the ELF header of a mapped module (safe: reads via /proc/self/mem
/// is unnecessary — the first page is always readable, so we can read it
/// through a small helper that uses `std::fs` on `/proc/self/map_files`...
/// which needs permissions. Instead, we read the module *file* from disk
/// (the maps path) — always accessible to the process itself.
pub fn detect(module: &ModuleInfo) -> Option<Detection> {
    let data = std::fs::read(&module.path).ok()?;
    detect_from_bytes(&data, module.base, &module.path)
}

/// Detection over raw library bytes (also used by the host CLI).
pub fn detect_from_bytes(data: &[u8], base: u64, module_path: &str) -> Option<Detection> {
    let elf = ElfImage::parse(data).ok()?;

    // Version.
    let version = owen_core::version::detect_from_bytes(data)?;
    let profile = profiles::profile_for(version.clone());

    // curl_easy_setopt.
    let mut curl_setopt = None;
    let mut curl_source = CurlSource::None;
    if let Some(sym) = elf.find_symbol("curl_easy_setopt").filter(|s| s.value != 0) {
        curl_setopt = Some(base + sym.value);
        curl_source = CurlSource::ExportedSymbol;
    } else {
        let report = owen_core::scanner::scan_image_data(data);
        if let Some(va) = report.best_va {
            curl_setopt = Some(base + va);
            curl_source = CurlSource::AutoScan;
        }
    }

    Some(Detection {
        module_path: module_path.to_string(),
        version,
        profile,
        curl_setopt,
        curl_source,
    })
}

impl Detection {
    /// Whether ART WebView hooks apply for this build.
    pub fn wants_art_hooks(&self) -> bool {
        self.profile.method == RedirectMethod::KairosWeb || self.profile.kairos_webview
    }
}
