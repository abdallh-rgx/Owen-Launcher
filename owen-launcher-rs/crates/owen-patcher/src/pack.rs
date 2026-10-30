//! The pure-Rust APK packer — assembles `dist/OwenLauncher-v1.9.0-release.apk`
//! from the workspace's own outputs, with zero Android SDK involvement at
//! pack time:
//!
//! ```text
//! AndroidManifest.xml   STORED  (aapt2 link output when prebuilt, else
//!                               axmlwriter; 4-aligned either way)
//! resources.arsc        STORED  (aapt2 link output when prebuilt, else the
//!                               empty 12-byte table; 4-aligned either way)
//! classes.dex           DEFLATE (javac+d8 output when prebuilt, else
//!                               assets/dex/classes.dex)
//! lib/arm64-v8a/*.so    STORED  (16 KiB page-aligned: libowen_app + libowen)
//! assets/lawin/**       DEFLATE (data/lawin)
//! assets/owen/**        DEFLATE (data/owen)
//! assets/images/**      DEFLATE (public/owen/seasons/season_N_folder.png)
//! assets/trailers/trailers.json  DEFLATE (season trailer URL registry — no videos)
//! assets/tools/*        DEFLATE (ogfn-debug.keystore)
//! assets/signing/*      DEFLATE (the ogfn key/cert the app re-uses)
//! ```
//!
//! The **prebuilt** inputs ([`PackConfig::prebuilt_manifest`] /
//! [`PackConfig::prebuilt_arsc`] / [`PackConfig::prebuilt_dex`], produced by
//! `aapt2` + `javac` + `d8` in `tools/build-apk.sh`) are the REAL Android
//! build-tooling artifacts and are strongly preferred — the handcrafted
//! axmlwriter manifest and the 12-byte stub arsc remain only as the fallback
//! for SDK-less smoke builds. Prebuilt inputs are roundtrip-validated before
//! they enter the zip (see [`validate_prebuilt_manifest`] /
//! [`validate_prebuilt_arsc`] / [`validate_prebuilt_dex`]).
//!
//! Then v1 (`CERT.RSA` with the ORIGINAL ogfn certificate) + v2 signing and a
//! full self-verification (verify_v1 + verify_v2 + zipalign checks).
//!
//! NOTE: this lives in `owen-patcher` (not `owen-apk`) because
//! `owen-patcher` already depends on `owen-apk::axml` — a reverse
//! `owen-apk → owen-patcher` edge would be a cargo cycle. `owen-patcher` is
//! the only crate that sees both the zip/sign stack and the new
//! `owen_apk::axmlwriter`.

use crate::sign::SigningKey;
use crate::{sign, sign_v1, ziprw};
use owen_apk::axmlwriter;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The final artifact name (matches the Kotlin app's package so it installs
/// as an update).
pub const OUT_NAME: &str = "OwenLauncher-v1.9.0-release.apk";

/// The dex foreground service the JNI layer starts when hosting toggles
/// (`app/src/com/abdallhrgx/owenlauncher/core/HostService.java`). Prebuilt
/// manifests are checked for it (soft — a warning note, not an error).
pub const HOST_SERVICE_CLASS: &str = "com.abdallhrgx.owenlauncher.core.HostService";

/// The redirecter's output file (`owen-redirecter` `[lib] name = "owen"`).
pub const REDIRECTER_SO: &str = "libowen.so";
/// The app crate's cdylib (package `owen-app` → `libowen_app.so`), with the
/// historical spellings accepted too.
pub const APP_SO_CANDIDATES: [&str; 3] =
    ["libowen_app.so", "libowenapp.so", "libowen-app.so"];

/// Where the packer looks for the built `.so`s.
pub fn android_release_dir(root: &Path) -> PathBuf {
    root.join("target/aarch64-linux-android/release")
}

/// The empty `resources.arsc` — a valid 12-byte `RES_TABLE_TYPE` chunk with
/// zero packages (the standard "no resources" APK trick).
pub fn empty_resources_arsc() -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&0x0002u16.to_le_bytes()); // RES_TABLE_TYPE
    out.extend_from_slice(&12u16.to_le_bytes()); // headerSize
    out.extend_from_slice(&12u32.to_le_bytes()); // size
    out.extend_from_slice(&0u32.to_le_bytes()); // packageCount
    out
}

/* ---------------------- prebuilt-input validation ----------------------- */

/// Little-endian u16 read with bounds checking (`None` when out of range).
fn u16_le(d: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(off)?, *d.get(off + 1)?]))
}

/// Little-endian u32 read with bounds checking (`None` when out of range).
fn u32_le(d: &[u8], off: usize) -> Option<u32> {
    let b = d.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// What [`validate_prebuilt_manifest`] found in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestCheck {
    /// Number of strings in the manifest's string pool.
    pub strings: usize,
    /// Whether `com.abdallhrgx.owenlauncher.core.HostService` (the foreground
    /// hosting service) is declared in the pool.
    pub host_service: bool,
}

/// Roundtrip-validate a prebuilt (aapt2) binary `AndroidManifest.xml` with
/// the EXISTING owen-apk reader — the same discipline the axmlwriter
/// self-test applies to its own output, now applied to aapt2's:
///
/// * the AXML file header must be `RES_XML_TYPE` with a size that matches the
///   buffer exactly,
/// * the chunk chain must cover the file with no gaps/overlaps and contain
///   at least one `START_ELEMENT` chunk,
/// * the string pool must parse with `owen_apk::axml::StringPool::parse_pool`
///   (UTF-8 or UTF-16 — both real aapt2 outputs),
/// * the pool must carry every string the launcher's install/boot contract
///   needs — package, `android.app.lib_name` **matching `lib_name`** (the
///   manifest ↔ packed `.so` contract), the NativeActivity, the receiver +
///   its action, MAIN/LAUNCHER and all 9 permissions.
///
/// The HostService declaration is reported (soft) rather than required so
/// minimal smoke manifests still pack — `pack()` turns it into a warning.
pub fn validate_prebuilt_manifest(bytes: &[u8], lib_name: &str) -> Result<ManifestCheck, String> {
    let bad = |msg: String| format!("prebuilt AndroidManifest.xml: {msg}");
    use owen_apk::axml::{
        StringPool, RES_STRING_POOL_TYPE, RES_XML_START_ELEMENT_TYPE, RES_XML_TYPE,
    };

    // --- file header ------------------------------------------------------
    if bytes.len() < 8 {
        return Err(bad("shorter than an AXML file header".into()));
    }
    if u16_le(bytes, 0) != Some(RES_XML_TYPE) {
        return Err(bad(
            "file header type is not RES_XML_TYPE — not binary AXML (aapt2 link output \
             expected, not the text manifest)"
                .into(),
        ));
    }
    if u16_le(bytes, 2) != Some(8) {
        return Err(bad("file header size is not 8".into()));
    }
    let total = u32_le(bytes, 4).unwrap_or(0) as usize;
    if total != bytes.len() {
        return Err(bad(format!(
            "header size {total} != buffer size {}",
            bytes.len()
        )));
    }

    // --- chunk walk -------------------------------------------------------
    let mut off = 8usize;
    let mut pool: Option<StringPool> = None;
    let mut elements = 0usize;
    while off < bytes.len() {
        let Some(ctype) = u16_le(bytes, off) else {
            return Err(bad(format!("truncated chunk header at {off}")));
        };
        let header_size = u16_le(bytes, off + 2).unwrap_or(0) as usize;
        let csize = u32_le(bytes, off + 4).unwrap_or(0) as usize;
        if header_size < 8 || csize < header_size || off + csize > bytes.len() {
            return Err(bad(format!(
                "chunk {ctype:#06x} at {off} has an inconsistent size (header {header_size}, \
                 size {csize}, buffer {})",
                bytes.len()
            )));
        }
        if ctype == RES_STRING_POOL_TYPE && pool.is_none() {
            let chunk = &bytes[off..off + csize];
            pool = Some(StringPool::parse_pool(chunk).ok_or_else(|| {
                bad("string pool does not parse with the owen-apk reader (roundtrip \
                     broken — not an aapt2 binary manifest?)"
                    .into())
            })?);
        }
        if ctype == RES_XML_START_ELEMENT_TYPE {
            elements += 1;
        }
        off += csize;
    }
    if off != bytes.len() {
        return Err(bad("chunks do not cover the file exactly".into()));
    }
    let pool = pool.ok_or_else(|| bad("no RES_STRING_POOL_TYPE chunk found".into()))?;
    if elements == 0 {
        return Err(bad("no START_ELEMENT chunks — an empty AXML file".into()));
    }

    // --- the install/boot contract in the pool ----------------------------
    let strings: Vec<&str> = pool.strings.iter().map(|s| s.as_str()).collect();
    let need = |s: &str| -> Result<(), String> {
        if strings.contains(&s) {
            Ok(())
        } else {
            Err(bad(format!(
                "string pool is missing {s:?} — the manifest does not match the launcher \
                 component set"
            )))
        }
    };
    need(axmlwriter::PACKAGE)?;
    // The manifest ↔ packed .so contract: android.app.lib_name's value must
    // equal PackConfig::lib_name() (libowen_app.so → "owen_app").
    need(lib_name)?;
    need(axmlwriter::META_LIB_NAME)?;
    need("android.app.NativeActivity")?;
    need(axmlwriter::INSTALL_RESULT_RECEIVER)?;
    need(axmlwriter::ACTION_INSTALL_RESULT)?;
    need("android.intent.action.MAIN")?;
    need("android.intent.category.LAUNCHER")?;
    for perm in axmlwriter::PERMISSIONS {
        need(perm)?;
    }
    let host_service = strings.contains(&HOST_SERVICE_CLASS);
    Ok(ManifestCheck { strings: strings.len(), host_service })
}

/// Validate a prebuilt (aapt2) `resources.arsc`: a `RES_TABLE_TYPE` chunk
/// whose header size matches the buffer and which carries at least one
/// package (the empty 12-byte stub has zero).
pub fn validate_prebuilt_arsc(bytes: &[u8]) -> Result<(), String> {
    let bad = |msg: &str| format!("prebuilt resources.arsc: {msg}");
    if bytes.len() < 12 {
        return Err(bad("shorter than a RES_TABLE_TYPE header"));
    }
    if u16_le(bytes, 0) != Some(0x0002) {
        return Err(bad("chunk type is not RES_TABLE_TYPE (0x0002)"));
    }
    let header_size = u16_le(bytes, 2).unwrap_or(0) as usize;
    let size = u32_le(bytes, 4).unwrap_or(0) as usize;
    let packages = u32_le(bytes, 8).unwrap_or(0);
    if header_size < 12 {
        return Err(bad(&format!("header size {header_size} < 12")));
    }
    if size != bytes.len() {
        return Err(bad(&format!(
            "table size {size} != buffer size {}",
            bytes.len()
        )));
    }
    if packages == 0 {
        return Err(bad(
            "packageCount is 0 — that is the empty stub, not an aapt2 table",
        ));
    }
    Ok(())
}

/// Validate a prebuilt (javac + d8) `classes.dex`: the `dex\n035` magic d8
/// emits for `--min-api 24`, plus a whole DEX header present.
pub fn validate_prebuilt_dex(bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"dex\n035") {
        return Err(
            "prebuilt classes.dex: magic is not \"dex\\n035\" (d8 --min-api 24 output expected)"
                .to_string(),
        );
    }
    if bytes.len() < 0x70 {
        return Err("prebuilt classes.dex: shorter than a DEX header (0x70)".to_string());
    }
    Ok(())
}


#[derive(Debug, Clone)]
pub struct PackConfig {
    /// Workspace root (`owen-launcher-rs/`).
    pub root: PathBuf,
    /// Final APK path.
    pub out: PathBuf,
    /// Signing key (PKCS#8 PEM; the ogfn pair).
    pub key_pem: PathBuf,
    /// Signing certificate (DER; the ogfn pair).
    pub cert_der: PathBuf,
    /// The eframe app cdylib (`libowen_app.so`).
    pub app_so: PathBuf,
    /// The redirecter cdylib (`libowen.so`).
    pub redirecter_so: PathBuf,
    /// `classes.dex` (T5-JNI agent output) — required unless
    /// `allow_missing_dex`.
    pub dex: Option<PathBuf>,
    /// Prebuilt **binary** `AndroidManifest.xml` (aapt2 link output — real
    /// build tooling). When present it REPLACES the axmlwriter manifest in
    /// the zip (same entry name, same STORED/4-aligned rules) and is
    /// roundtrip-validated against the owen-apk AXML reader first.
    pub prebuilt_manifest: Option<Vec<u8>>,
    /// Prebuilt `resources.arsc` (aapt2 link output). When present it
    /// REPLACES the empty 12-byte stub table (same entry name, STORED,
    /// 4-aligned).
    pub prebuilt_arsc: Option<Vec<u8>>,
    /// Prebuilt `classes.dex` (javac + d8 output). When present it REPLACES
    /// the `assets/dex/classes.dex` input (same entry name, DEFLATE).
    pub prebuilt_dex: Option<Vec<u8>>,
    /// `ogfn-debug.keystore` (extracted from git by `tools/build-apk.sh`).
    pub keystore: PathBuf,
    /// Season folder PNGs (defaults to `<repo>/public/owen/seasons`).
    pub seasons_dir: Option<PathBuf>,
    /// Skip a missing dex with a warning (stub/CI smoke builds only).
    pub allow_missing_dex: bool,
}

impl PackConfig {
    /// Resolve every default from the workspace root.
    pub fn resolve(root: &Path) -> Result<Self, String> {
        let release = android_release_dir(root);
        let app_so = APP_SO_CANDIDATES
            .iter()
            .map(|n| release.join(n))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                format!(
                    "no app .so in {} (tried {}; build with: cargo build --release \
                     -p owen-app --target aarch64-linux-android)",
                    release.display(),
                    APP_SO_CANDIDATES.join(", ")
                )
            })?;
        let redirecter_so = [REDIRECTER_SO, "libOwen.so"]
            .iter()
            .map(|n| release.join(n))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                format!(
                    "no redirecter .so in {} (expected {REDIRECTER_SO}; build with: \
                     cargo build --release -p owen-redirecter --target aarch64-linux-android)",
                    release.display()
                )
            })?;
        let dex_path = root.join("assets/dex/classes.dex");
        Ok(PackConfig {
            root: root.to_path_buf(),
            out: root.join("dist").join(OUT_NAME),
            key_pem: root.join("assets/signing/ogfn-key.pem"),
            cert_der: root.join("assets/signing/ogfn-cert.der"),
            app_so,
            redirecter_so,
            dex: dex_path.is_file().then_some(dex_path),
            prebuilt_manifest: None,
            prebuilt_arsc: None,
            prebuilt_dex: None,
            keystore: root.join("assets/tools/ogfn-debug.keystore"),
            seasons_dir: root
                .parent()
                .map(|repo| repo.join("public/owen/seasons")),
            allow_missing_dex: false,
        })
    }

    /// `android.app.lib_name` — derived from the app .so filename
    /// (`libowen_app.so` → `owen_app`) so the manifest and the packed entry
    /// can never disagree.
    pub fn lib_name(&self) -> String {
        let file = self
            .app_so
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("owen_app");
        file.strip_prefix("lib")
            .and_then(|s| s.strip_suffix(".so"))
            .unwrap_or(file)
            .to_string()
    }

    /// The zip entry name for the app cdylib (`lib/arm64-v8a/libowen_app.so`).
    pub fn app_so_entry(&self) -> String {
        let file = self
            .app_so
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("libowen_app.so");
        format!("lib/arm64-v8a/{file}")
    }
}

#[derive(Debug)]
pub struct PackReport {
    pub apk: PathBuf,
    pub entries: usize,
    pub lib_name: String,
    pub app_so_entry: String,
    pub dex_included: bool,
    pub notes: Vec<String>,
    pub verify_v1: bool,
    pub verify_v2: bool,
    pub alignment: Vec<String>,
}

/// Recursively add `dir`'s files under the `prefix/` zip namespace.
fn add_dir_recursive(zip: &mut ziprw::ZipArchive, dir: &Path, prefix: &str) -> Result<usize, String> {
    let mut count = 0usize;
    let mut stack = vec![(dir.to_path_buf(), prefix.to_string())];
    while let Some((cur, ns)) = stack.pop() {
        let entries = fs::read_dir(&cur).map_err(|e| format!("{}: {e}", cur.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                stack.push((path, format!("{ns}{name}/")));
            } else {
                let data = fs::read(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                zip.add_entry(&format!("{ns}{name}"), &data);
                count += 1;
            }
        }
    }
    Ok(count)
}

/// A zero-entry zip (just the EOCD) — the seed archive `ZipArchive::open`
/// can then append to.
fn write_empty_zip(path: &Path) -> Result<(), String> {
    let mut eocd = [0u8; 22];
    eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
    fs::write(path, eocd).map_err(|e| e.to_string())
}

/// Assemble + sign the APK. Every step self-verifies; the returned report
/// carries the evidence (`verify_v1`/`verify_v2` must both be true and
/// `alignment` empty for a green build).
pub fn pack(cfg: &PackConfig) -> Result<PackReport, String> {
    let mut notes: Vec<String> = Vec::new();
    let lib_name = cfg.lib_name();
    let app_so_entry = cfg.app_so_entry();

    // ---- inputs ---------------------------------------------------------
    let key = SigningKey::load_pair(
        cfg.key_pem.to_str().ok_or("key path not utf-8")?,
        cfg.cert_der.to_str().ok_or("cert path not utf-8")?,
    )?;

    let app_so = fs::read(&cfg.app_so)
        .map_err(|e| format!("app .so {}: {e}", cfg.app_so.display()))?;
    let redirecter_so = fs::read(&cfg.redirecter_so)
        .map_err(|e| format!("redirecter .so {}: {e}", cfg.redirecter_so.display()))?;
    let keystore = fs::read(&cfg.keystore).map_err(|e| {
        format!(
            "keystore {} missing: {e} (tools/build-apk.sh extracts it via \
             `git show owen/main:app/src/main/assets/tools/ogfn-debug.keystore`)",
            cfg.keystore.display()
        )
    })?;

    let dex = if let Some(bytes) = cfg.prebuilt_dex.clone() {
        // REAL build tooling: javac + d8 output (tools/build-apk.sh phase 3).
        validate_prebuilt_dex(&bytes)?;
        notes.push(format!(
            "dex: prebuilt (javac + d8) — {} bytes, magic ok",
            bytes.len()
        ));
        Some(bytes)
    } else {
        match &cfg.dex {
            Some(path) => Some(
                fs::read(path).map_err(|e| format!("dex {}: {e}", path.display()))?,
            ),
            None => {
                if cfg.allow_missing_dex {
                    notes.push(
                        "WARNING: classes.dex missing — packed without it (--allow-missing-dex); \
                         an APK with hasCode=true will NOT boot without the dex. Final assembly \
                         must include it."
                            .to_string(),
                    );
                    None
                } else {
                    return Err(
                        "classes.dex missing (expected at assets/dex/classes.dex, or pass \
                         --prebuilt-dex with the d8 output). The real path is javac + d8 in \
                         tools/build-apk.sh; --allow-missing-dex is only for stub smoke builds."
                            .to_string(),
                    );
                }
            }
        }
    };

    // ---- the zip ---------------------------------------------------------
    let tmp_dir = cfg.root.join("target/pack");
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let unsigned = tmp_dir.join("unsigned.apk");
    write_empty_zip(&unsigned)?;

    // AndroidManifest.xml — the aapt2 prebuilt when given, else the
    // handcrafted axmlwriter fallback. Same entry name + STORED/4-aligned
    // rules either way; the prebuilt is roundtrip-validated first.
    let manifest = match cfg.prebuilt_manifest.clone() {
        Some(bytes) => {
            let check = validate_prebuilt_manifest(&bytes, &lib_name)?;
            notes.push(format!(
                "manifest: prebuilt (aapt2) — {} bytes, {} pool strings, chunk walk ok",
                bytes.len(),
                check.strings
            ));
            if check.host_service {
                notes.push(format!(
                    "manifest: {HOST_SERVICE_CLASS} declared (foreground hosting)"
                ));
            } else {
                notes.push(
                    "WARNING: prebuilt manifest does not declare \
                     {HOST_SERVICE_CLASS} — hosting will NOT survive the app being \
                     backgrounded while the game runs"
                        .to_string(),
                );
            }
            bytes
        }
        None => {
            notes.push(
                "manifest: axmlwriter fallback (handcrafted — NOT produced by aapt2; \
                 device-verified builds must pass --prebuilt-manifest)"
                    .to_string(),
            );
            axmlwriter::build_manifest(&lib_name)
        }
    };

    // resources.arsc — the aapt2 prebuilt when given, else the empty stub.
    let arsc = match cfg.prebuilt_arsc.clone() {
        Some(bytes) => {
            validate_prebuilt_arsc(&bytes)?;
            notes.push(format!(
                "resources.arsc: prebuilt (aapt2) — {} bytes, table header ok",
                bytes.len()
            ));
            bytes
        }
        None => {
            notes.push(
                "resources.arsc: empty 12-byte stub (no prebuilt passed)".to_string(),
            );
            empty_resources_arsc()
        }
    };

    let mut zip = ziprw::ZipArchive::open(&unsigned)?;
    zip.add_stored("AndroidManifest.xml", &manifest);
    zip.add_stored("resources.arsc", &arsc);
    if let Some(dex) = &dex {
        zip.add_entry("classes.dex", dex);
    }
    // Native libs: STORED + 16 KiB page aligned (ziprw alignment_for).
    zip.add_entry(&app_so_entry, &app_so);
    zip.add_entry(&format!("lib/arm64-v8a/{REDIRECTER_SO}"), &redirecter_so);

    // assets/
    let n_lawin = add_dir_recursive(&mut zip, &cfg.root.join("data/lawin"), "assets/lawin/")
        .map_err(|e| format!("data/lawin: {e}"))?;
    let n_owen = add_dir_recursive(&mut zip, &cfg.root.join("data/owen"), "assets/owen/")
        .map_err(|e| format!("data/owen: {e}"))?;
    notes.push(format!("assets: lawin={n_lawin} files, owen={n_owen} files"));
    let mut n_seasons = 0usize;
    if let Some(seasons) = cfg.seasons_dir.as_ref().filter(|d| d.is_dir()) {
        let mut files: Vec<PathBuf> = fs::read_dir(seasons)
            .map_err(|e| format!("{}: {e}", seasons.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("season_") && n.ends_with("_folder.png"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        for path in files {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let data = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            zip.add_entry(&format!("assets/images/{name}"), &data);
            n_seasons += 1;
        }
        notes.push(format!("assets: images={n_seasons} season folder pngs"));
    } else {
        notes.push("NOTE: seasons dir not found — assets/images/ skipped".to_string());
    }
    zip.add_entry("assets/tools/ogfn-debug.keystore", &keystore);
    let key_pem = fs::read(&cfg.key_pem).map_err(|e| format!("{}: {e}", cfg.key_pem.display()))?;
    let cert = fs::read(&cfg.cert_der).map_err(|e| format!("{}: {e}", cfg.cert_der.display()))?;
    zip.add_entry("assets/signing/ogfn-key.pem", &key_pem);
    zip.add_entry("assets/signing/ogfn-cert.der", &cert);

    // assets/trailers/trailers.json — the season trailer registry (URL map).
    // JSON ONLY: the trailer videos themselves are never packed into the
    // APK; they stream from YouTube or the "owen-trailers" GitHub release
    // (see .github/workflows/fetch-trailers.yml).
    let trailers = cfg.root.join("assets/trailers/trailers.json");
    if trailers.is_file() {
        let data = fs::read(&trailers).map_err(|e| format!("{}: {e}", trailers.display()))?;
        zip.add_entry("assets/trailers/trailers.json", &data);
        notes.push("assets: trailers/trailers.json (URL registry only — no videos)".into());
    } else {
        notes.push(
            "NOTE: assets/trailers/trailers.json missing — trailer registry skipped".into(),
        );
    }

    // assets/manifest.txt — only if the data dir ships one.
    for candidate in ["data/lawin/manifest.txt", "data/owen/manifest.txt"] {
        let path = cfg.root.join(candidate);
        if path.is_file() {
            let data = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            zip.add_entry("assets/manifest.txt", &data);
            notes.push(format!("assets/manifest.txt ← {candidate}"));
            break;
        }
    }
    if !notes.iter().any(|n| n.contains("assets/manifest.txt")) {
        notes.push(
            "NOTE: no manifest.txt in data/lawin or data/owen — assets/manifest.txt skipped \
             (the version catalog ships as assets/owen/manifests.json)"
                .to_string(),
        );
    }

    let unsigned_final = tmp_dir.join("unsigned-final.apk");
    zip.write_to(&unsigned_final)?;

    // ---- v1 + v2 signing --------------------------------------------------
    let signed_v1 = tmp_dir.join("signed-v1.apk");
    sign_v1::sign_apk_v1(&unsigned_final, &signed_v1, &key)?;
    sign::sign_apk_v2(&signed_v1, &key)?;

    let out_dir = cfg
        .out
        .parent()
        .ok_or_else(|| "output path has no parent".to_string())?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    fs::copy(&signed_v1, &cfg.out).map_err(|e| format!("copy to {}: {e}", cfg.out.display()))?;

    // ---- self-verification ------------------------------------------------
    let verify_v1 = sign_v1::verify_v1(&cfg.out).unwrap_or(false);
    let verify_v2 = sign::verify_v2(&cfg.out).unwrap_or(false);
    if !verify_v1 {
        return Err("self-check failed: v1 verification".to_string());
    }
    if !verify_v2 {
        return Err("self-check failed: v2 verification".to_string());
    }
    let alignment = check_alignment(&cfg.out)?;
    let entries = ziprw::ZipArchive::open(&cfg.out)
        .map_err(|e| format!("reopen {}: {e}", cfg.out.display()))?
        .list()
        .len();

    fs::remove_file(&unsigned).ok();
    fs::remove_file(&unsigned_final).ok();
    fs::remove_file(&signed_v1).ok();

    Ok(PackReport {
        apk: cfg.out.clone(),
        entries,
        lib_name,
        app_so_entry,
        dex_included: dex.is_some(),
        notes,
        verify_v1,
        verify_v2,
        alignment,
    })
}

/// zipalign self-check: every `lib/<abi>/*.so` data offset % 16384 == 0 and
/// `AndroidManifest.xml` / `resources.arsc` are STORED with data offsets
/// % 4 == 0. Returns the list of violations (empty = green).
pub fn check_alignment(apk: &Path) -> Result<Vec<String>, String> {
    let mut f = fs::File::open(apk).map_err(|e| e.to_string())?;
    let zip = ziprw::ZipArchive::open(apk)?;
    let mut problems = Vec::new();
    for entry in zip.list() {
        // Local header at entry.local_header_offset: 30 bytes + name + extra.
        use std::io::Seek;
        f.seek(std::io::SeekFrom::Start(entry.local_header_offset))
            .map_err(|e| e.to_string())?;
        let mut lh = [0u8; 30];
        f.read_exact(&mut lh).map_err(|e| e.to_string())?;
        let name_len = u16::from_le_bytes([lh[26], lh[27]]) as u64;
        let extra_len = u16::from_le_bytes([lh[28], lh[29]]) as u64;
        let data_off = entry.local_header_offset + 30 + name_len + extra_len;
        if entry.name.starts_with("lib/") && entry.name.ends_with(".so") {
            if entry.method != 0 {
                problems.push(format!("{}: method {} (must be STORED)", entry.name, entry.method));
            }
            if data_off % 16384 != 0 {
                problems.push(format!(
                    "{}: data offset {data_off} not 16384-aligned",
                    entry.name
                ));
            }
        }
        if entry.name == "AndroidManifest.xml" || entry.name == "resources.arsc" {
            if entry.method != 0 {
                problems.push(format!(
                    "{}: method {} (must be STORED)",
                    entry.name, entry.method
                ));
            }
            if data_off % 4 != 0 {
                problems.push(format!("{}: data offset {data_off} not 4-aligned", entry.name));
            }
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_validator_accepts_valid_axml() {
        // The axmlwriter fallback output must pass the same validator the
        // aapt2 prebuilts go through (it lacks HostService → soft flag only).
        let bytes = axmlwriter::build_manifest("owen_app");
        let check = validate_prebuilt_manifest(&bytes, "owen_app")
            .expect("axmlwriter output must roundtrip-validate");
        assert!(check.strings >= 16, "a plausible pool size");
        assert!(!check.host_service, "the fallback manifest has no HostService");
    }

    #[test]
    fn manifest_validator_rejects_garbage() {
        for garbage in [
            &b""[..],
            b"not axml at all, just text",
            b"\x03\x00\x08\x00\xff\xff\xff\xff", // header size != buffer
        ] {
            assert!(
                validate_prebuilt_manifest(garbage, "owen_app").is_err(),
                "garbage {garbage:?} must not validate"
            );
        }
    }

    #[test]
    fn manifest_validator_enforces_lib_name_contract() {
        // A manifest built for `owen_app` must NOT validate against a packed
        // .so whose lib_name is different (the manifest ↔ .so contract).
        let bytes = axmlwriter::build_manifest("owen_app");
        let err = validate_prebuilt_manifest(&bytes, "owenapp").unwrap_err();
        assert!(err.contains("owenapp"), "error must name the missing string: {err}");
    }

    #[test]
    fn arsc_validator() {
        // the empty stub is NOT a valid prebuilt (0 packages) …
        assert!(validate_prebuilt_arsc(&empty_resources_arsc()).is_err());
        // … but a one-package table is: header + a minimal package body.
        let mut table: Vec<u8> = Vec::new();
        table.extend_from_slice(&0x0002u16.to_le_bytes()); // RES_TABLE_TYPE
        table.extend_from_slice(&12u16.to_le_bytes()); // headerSize
        table.extend_from_slice(&(12u32 + 8).to_le_bytes()); // size (== len)
        table.extend_from_slice(&1u32.to_le_bytes()); // packageCount
        table.extend_from_slice(&[0u8; 8]); // minimal package body
        assert_eq!(u32_le(&table, 4), Some(table.len() as u32));
        validate_prebuilt_arsc(&table).expect("one-package table must validate");
        // wrong type byte → reject
        let mut bad_type = table.clone();
        bad_type[0..2].copy_from_slice(&0x0001u16.to_le_bytes());
        assert!(validate_prebuilt_arsc(&bad_type).is_err());
        // size mismatch → reject
        let mut bad_size = table.clone();
        bad_size.truncate(bad_size.len() - 1);
        assert!(validate_prebuilt_arsc(&bad_size).is_err());
    }

    #[test]
    fn dex_validator() {
        let mut dex = vec![0u8; 0x70];
        dex[..7].copy_from_slice(b"dex\n035");
        validate_prebuilt_dex(&dex).expect("dex\\n035 magic must validate");
        dex[3] = b'x';
        assert!(validate_prebuilt_dex(&dex).is_err(), "wrong magic must fail");
        assert!(
            validate_prebuilt_dex(b"dex\n035 short").is_err(),
            "shorter than a header must fail"
        );
        // the CURRENT handcrafted dex (assets/dex/classes.dex) is 035 too —
        // if present it must satisfy the same check.
        if let Ok(bytes) = fs::read("assets/dex/classes.dex") {
            validate_prebuilt_dex(&bytes)
                .expect("the handcrafted dex is also 035 and must validate");
        }
    }
}

