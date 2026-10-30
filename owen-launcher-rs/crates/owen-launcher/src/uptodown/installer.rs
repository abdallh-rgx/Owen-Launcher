#![forbid(unsafe_code)]
//! 1:1 port of `UptodownInstaller.kt` (object `UptodownInstaller`) with the
//! launcher-crate split requested for the app layer:
//!
//! * **download with progress** — [`download_with_progress`] emits
//!   [`DownloadEvent`]s (`Started(total)` / `Progress(bytes)` / `Done(path)` /
//!   `Failed(msg)`) and keeps the original `.part`-file dance, 512 KiB buffer
//!   and progress math;
//! * **XAPK unpacking** — via `owen_patcher::ziprw` (same entry rules:
//!   `base.apk` + `split_config*.apk`, `.obb` skipped);
//! * **temp files** — under the caller-passed work dir (the original used
//!   `context.cacheDir` / `getExternalFilesDir("download")`);
//! * **the install step** — delegated to the [`Installer`][crate::install::Installer]
//!   trait (no Android imports; the JNI port arrives later, desktop builds
//!   use `NoopInstaller`).
//!
//! The APK-patching step of the original (`ApkPatcher` / `PatchCache` /
//! `NativeBuilder` / `OwenManager` / `ManifestRenamer` / `LoginWebServer`)
//! belongs to the app layer: this module keeps the same
//! [`InstallState::Patching`] state and the same step order around the seam,
//! and the caller can wire `owen_patcher::patch_apk` either around
//! [`install_apk`] / [`install_xapk`] or inside its own `Installer`
//! implementation (the staged APK paths travel in
//! [`InstallSession::apks`][crate::install::InstallSession]).
//!
//! | Kotlin (`UptodownInstaller`)                    | Rust (here)                        |
//! |-------------------------------------------------|------------------------------------|
//! | `InstallState` (sealed class)                   | [`InstallState`]                   |
//! | `installSuspend(context, version, onState)`     | [`UptodownInstaller::install`]     |
//! | `installApk(context, file, …)`                  | [`install_apk`]                    |
//! | `installXapk(context, xapk, version, …)`        | [`install_xapk`]                   |
//! | `UptodownResolver.downloadWithProgress`         | [`download_with_progress`]         |
//! | `statusToState` / `isPartyHubVersion` /         | [`status_to_state`] /
//! | `readManifestPackage` / `isZipBundle`           | [`is_party_hub_version`] / …       |

use super::api::UptodownApi;
use super::resolver::UptodownResolver;
use crate::http::HttpBackend;
use crate::install::helper::Installer;
use std::path::{Path, PathBuf};

/// `private const val TAG`.
const TAG: &str = "UptodownInstaller";

/// `private const val DEFAULT_BACKEND_URL` — the Owen backend URL used by the
/// (app-layer) patch step; kept public so the app layer wires it into
/// `owen_patcher` the same way.
pub const DEFAULT_BACKEND_URL: &str = "http://127.0.0.1:3551";

/// `private const val FALLBACK_PACKAGE`.
pub const FALLBACK_PACKAGE: &str = "com.epicgames.fortnite";

/// `UptodownResolver.CONNECT_TIMEOUT_MS` — moved here together with
/// [`download_with_progress`].
const CONNECT_TIMEOUT_MS: u64 = 30_000;

/// `UptodownResolver.READ_TIMEOUT_MS`.
const READ_TIMEOUT_MS: u64 = 60_000;

/// `UptodownResolver.BUFFER_SIZE = 512 * 1024`.
const BUFFER_SIZE: usize = 512 * 1024;

/// `UptodownInstaller.InstallState` (sealed class) — same variants, same
/// field names.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallState {
    Idle,
    /// `Resolving(val status: String)`.
    Resolving { status: String },
    /// `Downloading(val progress: Float, val bytesDone: Long, val bytesTotal: Long)`.
    Downloading {
        progress: f32,
        bytes_done: u64,
        bytes_total: u64,
    },
    /// `Patching(val status: String)` — driven by the app-layer patch step.
    Patching { status: String },
    /// `Installing(val status: String)`.
    Installing { status: String },
    /// `Completed(val packageName: String)`.
    Completed { package_name: String },
    /// `Error(val message: String)`.
    Error { message: String },
}

/// Download progress events of [`download_with_progress`] /
/// [`UptodownInstaller::install`].
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadEvent {
    /// The transfer started; `total` is the Content-Length when known
    /// (0 otherwise).
    Started { total: u64 },
    /// Cumulative bytes written so far.
    Progress { bytes: u64 },
    /// The download finished and was renamed to its final path.
    Done { path: PathBuf },
    /// The download failed (`message` carries the technical reason).
    Failed { message: String },
}

/// The installer-facing subset of `VersionCatalog.OgVersion` — the fields the
/// download / XAPK / install flow needs. The app layer's catalog maps onto
/// it (the patch-only fields `season` / `releaseKey` / `manifestHash` /
/// `isDev` stay in the catalog).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OgVersion {
    /// e.g. `"13.40"` (Kotlin: `version`).
    pub version: String,
    /// Changelist (Kotlin: `cl`).
    pub cl: i64,
    /// `uptodownId` — `None` when the entry has no Uptodown id.
    pub uptodown_id: Option<i64>,
    /// `directApkUrl` — empty when the Uptodown resolver must be used.
    pub direct_apk_url: String,
    /// `fixedPackage` — when set it is passed on as the install target
    /// package (what `OwenManager.computeRenamedPackage` pinned in the
    /// original).
    pub fixed_package: String,
}

/// 1:1 port of `object UptodownInstaller` — the `MutableStateFlow` becomes
/// the `state` field (last emitted state), plus the `UptodownApi` instance
/// the singleton used for `findByCl`.
pub struct UptodownInstaller {
    state: InstallState,
    api: UptodownApi,
}

impl Default for UptodownInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl UptodownInstaller {
    pub fn new() -> Self {
        Self {
            state: InstallState::Idle,
            api: UptodownApi::new(),
        }
    }

    /// `val state: StateFlow<InstallState>`.
    pub fn state(&self) -> &InstallState {
        &self.state
    }

    /// `fun reset()`.
    pub fn reset(&mut self) {
        self.state = InstallState::Idle;
    }

    /// `fun emit(st: InstallState)` — `_state.value = st; onState(st)`.
    fn set_state(&mut self, on_state: &mut impl FnMut(InstallState), state: InstallState) {
        self.state = state.clone();
        on_state(state);
    }

    /// Native download backend configured with `CONNECT_TIMEOUT_MS` /
    /// `READ_TIMEOUT_MS` (the original `downloadWithProgress` connection
    /// settings).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn default_backend() -> crate::http::ReqwestBackend {
        crate::http::ReqwestBackend::with_timeouts_ms(CONNECT_TIMEOUT_MS, READ_TIMEOUT_MS)
    }

    /// `suspend fun installSuspend(context, version, onState): Result<String>`
    /// — same step order and the same English messages. `context` becomes the
    /// caller-passed `work_dir` (+ [`HttpBackend`] + [`Installer`]).
    pub fn install(
        &mut self,
        http: &dyn HttpBackend,
        installer: &mut dyn Installer,
        work_dir: &Path,
        version: &OgVersion,
        mut on_download: impl FnMut(DownloadEvent),
        mut on_state: impl FnMut(InstallState),
    ) -> Result<String, String> {
        // val dir = context.getExternalFilesDir("download") ?: context.filesDir
        // val dest = File(dir, "Fortnite-${version.version}-${version.cl}.apk")
        let dest = work_dir.join(format!(
            "Fortnite-{}-{}.apk",
            version.version, version.cl
        ));

        // val url = … directApkUrl ?: resolveDirectUrl(uptodownId)
        let url = if !version.direct_apk_url.is_empty() {
            version.direct_apk_url.clone()
        } else {
            let version_id = version.uptodown_id.unwrap_or(0);
            if version_id <= 0 {
                let msg = format!(
                    "Version {} has no Uptodown id and no direct APK URL",
                    version.version
                );
                self.set_state(&mut on_state, InstallState::Error { message: msg.clone() });
                return Err(msg);
            }
            self.set_state(
                &mut on_state,
                InstallState::Resolving {
                    status: "Resolving download link…".to_string(),
                },
            );
            match UptodownResolver::resolve_direct_url(http, version_id) {
                Some(resolved) => resolved,
                None => {
                    let msg =
                        "Could not resolve the download link (Uptodown page changed?)".to_string();
                    self.set_state(&mut on_state, InstallState::Error { message: msg.clone() });
                    return Err(msg);
                }
            }
        };

        // emit(InstallState.Downloading(0f, 0L, 0L))
        self.set_state(
            &mut on_state,
            InstallState::Downloading {
                progress: 0.0,
                bytes_done: 0,
                bytes_total: 0,
            },
        );
        // val referer = version.uptodownId?.let { UptodownApi.downloadPageUrl(it) } ?: UptodownApi.HOST
        let referer = version
            .uptodown_id
            .map(UptodownApi::download_page_url)
            .unwrap_or_else(|| UptodownApi::HOST.to_string());

        // val downloaded = UptodownResolver.downloadWithProgress(url, dest, referer) { p, d, t -> … }
        // The (progress, done, total) triple of the original is reconstructed
        // from the DownloadEvents (same math: reportedTotal / progress).
        let downloaded = {
            let this = &mut *self;
            let mut dl_total: u64 = 0;
            let mut sink = |event: DownloadEvent| {
                match event {
                    DownloadEvent::Started { total } => dl_total = total,
                    DownloadEvent::Progress { bytes } => {
                        let reported_total = if dl_total > 0 { dl_total } else { bytes };
                        let progress = if dl_total > 0 {
                            bytes as f32 / dl_total as f32
                        } else {
                            0.0
                        };
                        this.set_state(
                            &mut on_state,
                            InstallState::Downloading {
                                progress,
                                bytes_done: bytes,
                                bytes_total: reported_total,
                            },
                        );
                    }
                    _ => {}
                }
                on_download(event);
            };
            download_with_progress(http, &url, &dest, &referer, &mut sink)
        };
        if !downloaded
            || !dest.is_file()
            || dest.metadata().map(|m| m.len()).unwrap_or(0) == 0
        {
            let msg = format!("Download failed for {}", version.version);
            self.set_state(&mut on_state, InstallState::Error { message: msg.clone() });
            return Err(msg);
        }

        // if (!isZipBundle(dest)) …
        if !is_zip_bundle(&dest) {
            let msg = format!(
                "Downloaded file for {} is not a valid package",
                version.version
            );
            self.set_state(&mut on_state, InstallState::Error { message: msg.clone() });
            return Err(msg);
        }

        // val uptodownEntry = runCatching { UptodownApi.findByCl(version.cl) }.getOrNull()
        let uptodown_entry = self.api.find_by_cl(http, version.cl);
        // val isXapk = url.endsWith(".xapk", ignoreCase = true) || uptodownEntry?.isXapk() == true
        let is_xapk = url.to_ascii_lowercase().ends_with(".xapk")
            || uptodown_entry.as_ref().map(|entry| entry.is_xapk()) == Some(true);

        // var installFailure: String? = null — collected through on_error.
        let mut install_failure: Option<String> = None;
        // The app layer's renamed package (OwenManager.computeRenamedPackage)
        // is forwarded via OgVersion::fixed_package.
        let target_package: Option<String> = if version.fixed_package.is_empty() {
            None
        } else {
            Some(version.fixed_package.clone())
        };

        let installed = if is_xapk {
            install_xapk(
                installer,
                &dest,
                work_dir,
                version,
                &mut |status| {
                    self.set_state(&mut on_state, status_to_state(&status));
                },
                &mut |message| install_failure = Some(message),
            )
        } else {
            install_apk(
                installer,
                &dest,
                target_package.as_deref(),
                &mut |status| {
                    self.set_state(&mut on_state, status_to_state(&status));
                },
                &mut |message| install_failure = Some(message),
            )
        };

        match installed {
            None => {
                let msg = install_failure.unwrap_or_else(|| {
                    format!("Installation failed for {}", version.version)
                });
                self.set_state(&mut on_state, InstallState::Error { message: msg.clone() });
                Err(msg)
            }
            Some(package) => {
                self.set_state(
                    &mut on_state,
                    InstallState::Completed {
                        package_name: package.clone(),
                    },
                );
                Ok(package)
            }
        }
    }
}

/// `UptodownResolver.downloadWithProgress(url, dest, referer, onProgress)` —
/// same `.part`-file dance (create after the status check, rename at the
/// end, delete on failure), same 512 KiB chunks, same progress math; the
/// `(progress, done, total)` callback of the original is reported through
/// [`DownloadEvent`]s (`Progress.bytes` is cumulative `done`).
pub fn download_with_progress(
    http: &dyn HttpBackend,
    url: &str,
    dest: &Path,
    referer: &str,
    on_event: &mut dyn FnMut(DownloadEvent),
) -> bool {
    // val parent = dest.parentFile ?: return false; parent.mkdirs()
    let Some(parent) = dest.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let Some(file_name) = dest.file_name() else {
        return false;
    };
    let file_name = file_name.to_string_lossy();
    // val part = File(parent, dest.name + ".part")
    let part = parent.join(format!("{file_name}.part"));

    let headers = vec![
        ("User-Agent".to_string(), UptodownResolver::UA.to_string()),
        ("Referer".to_string(), referer.to_string()),
        ("Accept".to_string(), "*/*".to_string()),
    ];

    let mut out: Option<std::fs::File> = None;
    let mut done: u64 = 0;
    let mut total: i64 = -1; // conn.contentLengthLong (-1 = unknown)
    let mut started = false;
    let mut fail_reason: Option<String> = None;

    let streamed = http.get_streaming(
        url,
        &headers,
        BUFFER_SIZE,
        &mut |status, chunk, content_len| {
            if !started {
                started = true;
                total = content_len.map(|t| t as i64).unwrap_or(-1);
                on_event(DownloadEvent::Started {
                    total: content_len.unwrap_or(0),
                });
                // if (code < 200 || code >= 400) return false
                if status < 200 || status >= 400 {
                    fail_reason = Some(format!("HTTP status {status}"));
                    return false;
                }
            }
            // FileOutputStream(part) — created only after the status check,
            // like the original.
            if out.is_none() {
                match std::fs::File::create(&part) {
                    Ok(file) => out = Some(file),
                    Err(_) => {
                        fail_reason = Some(format!("cannot create {}", part.display()));
                        return false;
                    }
                }
            }
            let file = out.as_mut().expect("part file just created");
            if std::io::Write::write_all(file, chunk).is_err() {
                fail_reason = Some(format!("cannot write {}", part.display()));
                return false;
            }
            done += chunk.len() as u64;
            // onProgress(progress, done, reportedTotal) — the caller derives
            // the triple from Started.total + Progress.bytes.
            on_event(DownloadEvent::Progress { bytes: done });
            true
        },
    );

    let ok = match streamed {
        Ok(_) => fail_reason.is_none(),
        Err(err) => {
            fail_reason.get_or_insert(err);
            false
        }
    };
    if !ok {
        // catch (t) { part.delete(); false }
        let _ = std::fs::remove_file(&part);
        on_event(DownloadEvent::Failed {
            message: fail_reason.unwrap_or_else(|| "download failed".to_string()),
        });
        return false;
    }
    if let Some(mut file) = out.take() {
        let _ = std::io::Write::flush(&mut file);
    }

    // if (part.length() == 0L || (total > 0 && done < total)) { part.delete(); false }
    let part_len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if part_len == 0 || (total > 0 && (done as i64) < total) {
        let _ = std::fs::remove_file(&part);
        on_event(DownloadEvent::Failed {
            message: "download incomplete".to_string(),
        });
        return false;
    }

    // if (dest.exists()) dest.delete(); val renamed = part.renameTo(dest)
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    match std::fs::rename(&part, dest) {
        Ok(()) => {
            on_event(DownloadEvent::Done {
                path: dest.to_path_buf(),
            });
            true
        }
        Err(err) => {
            // if (!renamed) part.delete()
            let _ = std::fs::remove_file(&part);
            on_event(DownloadEvent::Failed {
                message: err.to_string(),
            });
            false
        }
    }
}

/// `installApk(context, file, …)` — same flow minus the app-layer patch step:
/// install via [`Installer`], then resolve the resulting package
/// (`targetPackage ?: FALLBACK_PACKAGE`; reading the package back from the
/// patched APK's manifest is an app-layer concern).
pub fn install_apk(
    installer: &mut dyn Installer,
    file: &Path,
    target_package: Option<&str>,
    on_status: &mut dyn FnMut(String),
    on_error: &mut dyn FnMut(String),
) -> Option<String> {
    // DiagLog.section("Install: Fortnite ${file.name}")
    eprintln!("[{TAG}] === Install: Fortnite {} ===", file.display());
    // PackageInstallHelper.install(context, listOf(patchedApk)) {
    //     onStatus("Installing: " + it)
    // }
    let result = installer.install(
        &[file.to_path_buf()],
        &mut |status| on_status(format!("Installing: {status}")),
    );
    if !result.ok {
        // DiagLog.e(TAG, "Install failed: " + result.describe())
        eprintln!("[{TAG}] Install failed: {}", result.describe());
        on_error(result.describe());
        return None;
    }
    // val actual = readPackageNameFromApk(patchedApk) ?: targetPackage ?: FALLBACK_PACKAGE
    // (reading the package from the patched APK is app-layer / owen-apk)
    let actual = target_package
        .map(|p| p.to_string())
        .unwrap_or_else(|| FALLBACK_PACKAGE.to_string());
    Some(actual)
}

/// `installXapk(context, xapk, version, …)` — same flow minus the app-layer
/// patch step: extract `base.apk` + `split_config*.apk` (skip `.obb` and
/// directories) into a temp dir under `work_dir`, install them all via
/// [`Installer`], resolve the package from the XAPK `manifest.json`.
pub fn install_xapk(
    installer: &mut dyn Installer,
    xapk: &Path,
    work_dir: &Path,
    version: &OgVersion,
    on_status: &mut dyn FnMut(String),
    on_error: &mut dyn FnMut(String),
) -> Option<String> {
    // DiagLog.section("Install XAPK: Fortnite ${version.version}")
    eprintln!("[{TAG}] === Install XAPK: Fortnite {} ===", version.version);
    // val tempDir = File(context.cacheDir, "xapk_${version.cl}_${System.currentTimeMillis()}")
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let temp_dir = work_dir.join(format!("xapk_{}_{}", version.cl, millis));

    // try { tempDir.mkdirs(); ZipFile(xapk).use { … } } finally { tempDir.deleteRecursively() }
    let outcome = (|| -> Option<String> {
        std::fs::create_dir_all(&temp_dir).ok()?;
        let zip = match owen_patcher::ziprw::ZipArchive::open(xapk) {
            Ok(zip) => zip,
            Err(err) => {
                // DiagLog.e(TAG, "installXapk failed", t)
                eprintln!("[{TAG}] installXapk failed: {err}");
                return None;
            }
        };
        // val detectedPkg = readManifestPackage(zip)
        let detected_pkg = read_manifest_package(&zip);
        // onStatus("Extracting APKs from XAPK…")
        on_status("Extracting APKs from XAPK…".to_string());
        let mut apk_files: Vec<PathBuf> = Vec::new();
        for entry in zip.list() {
            let name = entry.name.as_str();
            if name.ends_with('/') {
                // entry.isDirectory
                continue;
            }
            let lower = name.to_lowercase();
            if lower.ends_with(".obb") {
                // DiagLog.i(TAG, "Skipping OBB entry in XAPK: $name")
                eprintln!("[{TAG}] Skipping OBB entry in XAPK: {name}");
                continue;
            }
            if name == "base.apk" || (lower.starts_with("split_config") && lower.ends_with(".apk"))
            {
                let out_file = temp_dir.join(name.rsplit('/').next().unwrap_or(name));
                match zip.read_entry(name) {
                    Some(bytes) => {
                        if let Err(err) = std::fs::write(&out_file, &bytes) {
                            eprintln!("[{TAG}] installXapk failed: {err}");
                            return None;
                        }
                        apk_files.push(out_file);
                    }
                    None => {
                        eprintln!("[{TAG}] installXapk failed: cannot read XAPK entry {name}");
                        return None;
                    }
                }
            }
        }
        // val baseApk = apkFiles.firstOrNull { it.name == "base.apk" } ?: apkFiles.firstOrNull()
        let base_apk = apk_files
            .iter()
            .find(|path| {
                path.file_name()
                    .map(|name| name == std::ffi::OsStr::new("base.apk"))
                    .unwrap_or(false)
            })
            .cloned()
            .or_else(|| apk_files.first().cloned());
        let Some(base_apk) = base_apk else {
            // DiagLog.e(TAG, "XAPK does not contain a base APK")
            eprintln!("[{TAG}] XAPK does not contain a base APK");
            on_error("XAPK does not contain a base APK".to_string());
            return None;
        };
        // val resolvedPkg = detectedPkg ?: readPackageName(baseApk) (app-layer)
        let resolved_pkg = detected_pkg;

        // val toInstall = mutableListOf(patchedBase); … + apkFiles
        let mut to_install: Vec<PathBuf> = Vec::with_capacity(apk_files.len());
        to_install.push(base_apk.clone());
        for apk_file in &apk_files {
            if apk_file != &base_apk {
                to_install.push(apk_file.clone());
            }
        }
        // PackageInstallHelper.install(context, toInstall) { onStatus("Installing: " + it) }
        let install_result = installer.install(
            &to_install,
            &mut |status| on_status(format!("Installing: {status}")),
        );
        if !install_result.ok {
            // DiagLog.e(TAG, "XAPK install failed: " + installResult.describe())
            eprintln!("[{TAG}] XAPK install failed: {}", install_result.describe());
            on_error(install_result.describe());
            return None;
        }
        // val pkg = readPackageName(patchedBase) ?: resolvedPkg ?: FALLBACK_PACKAGE
        Some(resolved_pkg.unwrap_or_else(|| FALLBACK_PACKAGE.to_string()))
    })();
    // finally { tempDir.deleteRecursively() }
    let _ = std::fs::remove_dir_all(&temp_dir);
    outcome
}

/// `private fun statusToState(status: String): InstallState`.
fn status_to_state(status: &str) -> InstallState {
    if status.starts_with("Installing") {
        InstallState::Installing {
            status: status.to_string(),
        }
    } else {
        InstallState::Patching {
            status: status.to_string(),
        }
    }
}

/// `private fun isPartyHubVersion(version: String): Boolean` — major in 12..=15.
pub fn is_party_hub_version(version: &str) -> bool {
    let major = version
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .parse::<i32>()
        .ok();
    matches!(major, Some(major) if (12..=15).contains(&major))
}

/// `private fun readManifestPackage(zip: ZipFile): String?` — the XAPK
/// `manifest.json` `package_name` (falling back to `package`).
fn read_manifest_package(zip: &owen_patcher::ziprw::ZipArchive) -> Option<String> {
    let bytes = zip.read_entry("manifest.json")?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let manifest: serde_json::Value = serde_json::from_str(&text).ok()?;
    let obj = manifest.as_object()?;
    let pkg = match obj.get("package_name") {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let pkg = if pkg.is_empty() {
        match obj.get("package") {
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    } else {
        pkg
    };
    // pkg.takeIf { it.isNotEmpty() }
    if pkg.is_empty() {
        None
    } else {
        Some(pkg)
    }
}

/// `private fun isZipBundle(file: File): Boolean` — `PK\x03\x04` magic.
fn is_zip_bundle(file: &Path) -> bool {
    use std::io::Read;
    let Ok(mut input) = std::fs::File::open(file) else {
        return false;
    };
    let mut magic = [0u8; 4];
    if input.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == [0x50, 0x4B, 0x03, 0x04]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /* --------------------- tiny stored-zip builder --------------------- */

    fn zip_build_stored(path: &std::path::Path, entries: &[(&str, Vec<u8>)]) {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let mut offset: u32 = 0;
        for (name, data) in entries {
            let crc = owen_patcher::ziprw::crc32(data);
            let nb = name.as_bytes();
            let mut lh = [0u8; 30];
            lh[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
            lh[4..6].copy_from_slice(&20u16.to_le_bytes());
            lh[8..10].copy_from_slice(&0u16.to_le_bytes()); // stored
            lh[14..18].copy_from_slice(&crc.to_le_bytes());
            lh[18..22].copy_from_slice(&(data.len() as u32).to_le_bytes());
            lh[22..26].copy_from_slice(&(data.len() as u32).to_le_bytes());
            lh[26..28].copy_from_slice(&(nb.len() as u16).to_le_bytes());
            out.extend_from_slice(&lh);
            out.extend_from_slice(nb);
            out.extend_from_slice(data);

            let mut ch = [0u8; 46];
            ch[0..4].copy_from_slice(&0x0201_4b50u32.to_le_bytes());
            ch[6..8].copy_from_slice(&20u16.to_le_bytes());
            ch[10..12].copy_from_slice(&0u16.to_le_bytes());
            ch[16..20].copy_from_slice(&crc.to_le_bytes());
            ch[20..24].copy_from_slice(&(data.len() as u32).to_le_bytes());
            ch[24..28].copy_from_slice(&(data.len() as u32).to_le_bytes());
            ch[28..30].copy_from_slice(&(nb.len() as u16).to_le_bytes());
            ch[42..46].copy_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(&ch);
            central.extend_from_slice(nb);
            offset += 30 + nb.len() as u32 + data.len() as u32;
        }
        let cd_offset = offset;
        out.extend_from_slice(&central);
        let mut eocd = [0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        eocd[8..10].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        eocd[10..12].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        eocd[12..16].copy_from_slice(&(central.len() as u32).to_le_bytes());
        eocd[16..20].copy_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&eocd);
        std::fs::write(path, out).unwrap();
    }

    /* --------------------------- mock backend --------------------------- */

    struct MockBackend {
        payload: Vec<u8>,
        status: u16,
        total: Option<u64>,
        short: bool,
    }

    impl HttpBackend for MockBackend {
        fn get(
            &self,
            url: &str,
            _headers: &[(String, String)],
        ) -> Result<crate::http::HttpResponse, String> {
            if url.contains("/versions/1") {
                // Empty versions list → findByCl finds nothing, pagination stops.
                Ok(crate::http::HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: br#"{"success":1,"data":[]}"#.to_vec(),
                    url: url.to_string(),
                })
            } else if url.contains("/android/download/") {
                // The download page without any buttons.
                Ok(crate::http::HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: b"<html>no buttons</html>".to_vec(),
                    url: url.to_string(),
                })
            } else {
                Err("no route".to_string())
            }
        }

        fn post(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _body: &[u8],
        ) -> Result<crate::http::HttpResponse, String> {
            unimplemented!()
        }

        fn get_streaming(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            chunk_size: usize,
            on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
        ) -> Result<crate::http::HttpResponse, String> {
            let data: &[u8] = if self.short {
                &self.payload[..self.payload.len() / 2]
            } else {
                &self.payload
            };
            for chunk in data.chunks(chunk_size.max(1)) {
                if !on_chunk(self.status, chunk, self.total) {
                    return Err("download aborted by caller".to_string());
                }
            }
            Ok(crate::http::HttpResponse {
                status: self.status,
                headers: vec![],
                body: Vec::new(),
                url: "https://mock/dwn".to_string(),
            })
        }
    }

    /// Installer that records the staged APK groups and simulates success.
    struct RecordingInstaller {
        groups: RefCell<Vec<Vec<PathBuf>>>,
    }

    impl Installer for RecordingInstaller {
        fn create_session(&mut self) -> Result<crate::install::InstallSession, String> {
            Ok(crate::install::InstallSession {
                id: 1,
                apks: Vec::new(),
            })
        }
        fn submit(
            &mut self,
            session: &mut crate::install::InstallSession,
            apk: &Path,
        ) -> Result<(), String> {
            session.apks.push(apk.to_path_buf());
            Ok(())
        }
        fn commit(
            &mut self,
            session: crate::install::InstallSession,
            _on_status: &mut dyn FnMut(String),
        ) -> crate::install::InstallResult {
            self.groups.borrow_mut().push(session.apks);
            crate::install::InstallResult::success()
        }
        fn abandon(&mut self, _session: crate::install::InstallSession) {}
        fn on_result(&mut self, _s: i32, _st: i32, _m: Option<String>) -> bool {
            false
        }
    }

    fn test_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("owen-inst-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /* ------------------------------ tests ------------------------------ */

    #[test]
    fn is_party_hub_version_range() {
        assert!(is_party_hub_version("13.40"));
        assert!(is_party_hub_version("12.61"));
        assert!(is_party_hub_version("15.00"));
        assert!(is_party_hub_version("14"));
        assert!(!is_party_hub_version("16.00"));
        assert!(!is_party_hub_version("11.99"));
        assert!(!is_party_hub_version("banana"));
        // " 13.40".substringBefore('.').trim() == "13" → party hub (Kotlin
        // original behavior).
        assert!(is_party_hub_version(" 13.40"));
    }

    #[test]
    fn status_to_state_mapping() {
        assert_eq!(
            status_to_state("Installing: Committing install session..."),
            InstallState::Installing {
                status: "Installing: Committing install session...".into()
            }
        );
        assert_eq!(
            status_to_state("Patching APK (inject Owen + console)…"),
            InstallState::Patching {
                status: "Patching APK (inject Owen + console)…".into()
            }
        );
        assert_eq!(
            status_to_state("Extracting APKs from XAPK…"),
            InstallState::Patching {
                status: "Extracting APKs from XAPK…".into()
            }
        );
    }

    #[test]
    fn is_zip_bundle_magic() {
        let dir = test_dir("magic");
        let good = dir.join("good.zip");
        std::fs::write(&good, b"PK\x03\x04rest").unwrap();
        assert!(is_zip_bundle(&good));
        let empty_eocd = dir.join("eocd.zip");
        std::fs::write(&empty_eocd, b"PK\x05\x06").unwrap();
        assert!(!is_zip_bundle(&empty_eocd));
        assert!(!is_zip_bundle(&dir.join("missing.zip")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_manifest_package_from_xapk() {
        let dir = test_dir("manifest");
        let xapk = dir.join("game.xapk");
        zip_build_stored(
            &xapk,
            &[
                (
                    "manifest.json",
                    br#"{"package_name": "com.epicgames.fortnite2", "name": "Fortnite"}"#
                        .to_vec(),
                ),
                ("base.apk", b"PK\x03\x04base".to_vec()),
            ],
        );
        let zip = owen_patcher::ziprw::ZipArchive::open(&xapk).unwrap();
        assert_eq!(
            read_manifest_package(&zip).as_deref(),
            Some("com.epicgames.fortnite2")
        );
        // Fallback key: package_name missing → "package".
        let xapk2 = dir.join("game2.xapk");
        zip_build_stored(
            &xapk2,
            &[("manifest.json", br#"{"package": "com.other"}"#.to_vec())],
        );
        let zip = owen_patcher::ziprw::ZipArchive::open(&xapk2).unwrap();
        assert_eq!(read_manifest_package(&zip).as_deref(), Some("com.other"));
        // No manifest at all.
        let xapk3 = dir.join("game3.xapk");
        zip_build_stored(&xapk3, &[("base.apk", b"x".to_vec())]);
        let zip = owen_patcher::ziprw::ZipArchive::open(&xapk3).unwrap();
        assert_eq!(read_manifest_package(&zip), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn download_with_progress_success_flow() {
        let dir = test_dir("dl-ok");
        let dest = dir.join("nested").join("Fortnite-13.40-1.apk");
        let mut payload = Vec::new();
        for i in 0..250u32 {
            payload.extend_from_slice(&i.to_le_bytes());
        } // 1000 bytes
        let backend = MockBackend {
            payload: payload.clone(),
            status: 200,
            total: Some(1000),
            short: false,
        };
        let mut events: Vec<DownloadEvent> = Vec::new();
        // Chunk size is fixed at BUFFER_SIZE inside; the mock delivers one
        // chunk — assert the event sequence and the file dance.
        assert!(download_with_progress(
            &backend,
            "https://dw.uptodown.com/dwn/x",
            &dest,
            "https://fortnite.en.uptodown.com/android/download/1",
            &mut |ev| events.push(ev),
        ));
        assert_eq!(
            events,
            vec![
                DownloadEvent::Started { total: 1000 },
                DownloadEvent::Progress { bytes: 1000 },
                DownloadEvent::Done {
                    path: dest.clone()
                },
            ]
        );
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        assert!(!dir.join("nested").join("Fortnite-13.40-1.apk.part").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn download_with_progress_bad_status_and_short_read() {
        let dir = test_dir("dl-bad");
        let dest = dir.join("a.apk");

        // HTTP 404 → Failed, no dest, no .part.
        let backend = MockBackend {
            payload: b"error page".to_vec(),
            status: 404,
            total: None,
            short: false,
        };
        let mut events = Vec::new();
        assert!(!download_with_progress(&backend, "u", &dest, "r", &mut |ev| {
            events.push(ev)
        }));
        assert_eq!(
            events,
            vec![
                DownloadEvent::Started { total: 0 },
                DownloadEvent::Failed {
                    message: "HTTP status 404".into()
                },
            ]
        );
        assert!(!dest.exists());
        assert!(!dir.join("a.apk.part").exists());

        // Short read: content-length 1000, only 500 served → incomplete.
        let backend = MockBackend {
            payload: vec![7u8; 1000],
            status: 200,
            total: Some(1000),
            short: true,
        };
        let mut events = Vec::new();
        assert!(!download_with_progress(&backend, "u", &dest, "r", &mut |ev| {
            events.push(ev)
        }));
        assert!(matches!(events.last(), Some(DownloadEvent::Failed { .. })));
        assert!(!dest.exists());
        assert!(!dir.join("a.apk.part").exists(), "part cleaned up");

        // Unknown total + empty payload → incomplete.
        let backend = MockBackend {
            payload: Vec::new(),
            status: 200,
            total: None,
            short: false,
        };
        assert!(!download_with_progress(&backend, "u", &dest, "r", &mut |_| {}));
        assert!(!dest.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn install_apk_flow_with_noop() {
        let dir = test_dir("apk");
        let apk = dir.join("patched.apk");
        std::fs::write(&apk, b"PK\x03\x04apk").unwrap();

        let mut noop = crate::install::NoopInstaller::new();
        let mut statuses = Vec::new();
        let mut errors = Vec::new();
        let package = install_apk(&mut noop, &apk, None, &mut |s| statuses.push(s), &mut |e| {
            errors.push(e)
        });
        assert_eq!(package.as_deref(), Some(FALLBACK_PACKAGE));
        assert!(errors.is_empty());
        assert_eq!(
            statuses,
            vec!["Installing: Committing install session...".to_string()]
        );

        // A fixed target package is returned as-is.
        let package = install_apk(&mut noop, &apk, Some("com.fixed.pkg"), &mut |_| {}, &mut |_| {});
        assert_eq!(package.as_deref(), Some("com.fixed.pkg"));
        std::fs::remove_dir_all(&dir).ok();
    }

    fn version(version: &str, cl: i64, uptodown_id: Option<i64>, url: &str) -> OgVersion {
        OgVersion {
            version: version.into(),
            cl,
            uptodown_id,
            direct_apk_url: url.into(),
            fixed_package: String::new(),
        }
    }

    #[test]
    fn install_end_to_end_apk() {
        let dir = test_dir("e2e-apk");
        let mut payload = Vec::new();
        zip_build_stored_bytes(&mut payload, &[("base.apk", b"PK\x03\x04inner".to_vec())]);
        let backend = MockBackend {
            payload,
            status: 200,
            total: None,
            short: false,
        };
        let mut installer = RecordingInstaller {
            groups: RefCell::new(Vec::new()),
        };
        let mut launcher = UptodownInstaller::new();
        let mut states: Vec<InstallState> = Vec::new();
        let mut downloads: Vec<DownloadEvent> = Vec::new();

        let result = launcher.install(
            &backend,
            &mut installer,
            &dir,
            &version("13.40", 123, None, "https://mock/file.apk"),
            |ev| downloads.push(ev),
            |st| states.push(st.clone()),
        );

        assert_eq!(result.as_deref().ok(), Some("com.epicgames.fortnite"));
        // dest file name + content
        let dest = dir.join("Fortnite-13.40-123.apk");
        assert!(dest.is_file());
        // exactly one APK staged
        assert_eq!(installer.groups.borrow().len(), 1);
        assert_eq!(installer.groups.borrow()[0], vec![dest.clone()]);
        // state machine: Downloading(0,0,0) → Downloading(…) → Completed
        assert_eq!(states.first(), Some(&InstallState::Downloading {
            progress: 0.0,
            bytes_done: 0,
            bytes_total: 0,
        }));
        assert_eq!(
            states.last(),
            Some(&InstallState::Completed {
                package_name: "com.epicgames.fortnite".into()
            })
        );
        assert!(matches!(
            downloads.last(),
            Some(DownloadEvent::Done { .. })
        ));
        assert_eq!(launcher.state(), states.last().unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    fn zip_build_stored_bytes(out: &mut Vec<u8>, entries: &[(&str, Vec<u8>)]) {
        let tmp = std::env::temp_dir().join(format!(
            "owen-inst-bytes-{}-{}",
            std::process::id(),
            entries.len()
        ));
        zip_build_stored(&tmp, entries);
        out.extend_from_slice(&std::fs::read(&tmp).unwrap());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn install_end_to_end_xapk() {
        let dir = test_dir("e2e-xapk");
        let mut payload = Vec::new();
        zip_build_stored_bytes(
            &mut payload,
            &[
                (
                    "manifest.json",
                    br#"{"package_name": "com.epicgames.fortnite2"}"#.to_vec(),
                ),
                ("base.apk", b"PK\x03\x04base".to_vec()),
                (
                    "split_config.arm64_v8a.apk",
                    b"PK\x03\x04split".to_vec(),
                ),
                ("split_config.en.apk", b"PK\x03\x04split2".to_vec()),
                ("Android/obb/com.epicgames.fortnite.obb", b"obb".to_vec()),
                ("icons/icon.png", b"png".to_vec()),
            ],
        );
        let backend = MockBackend {
            payload,
            status: 200,
            total: None,
            short: false,
        };
        let mut installer = RecordingInstaller {
            groups: RefCell::new(Vec::new()),
        };
        let mut launcher = UptodownInstaller::new();
        let result = launcher.install(
            &backend,
            &mut installer,
            &dir,
            &version("14.60", 15534296, None, "https://mock/file.xapk"),
            |_| {},
            |_| {},
        );

        // Package from manifest.json.
        assert_eq!(result.as_deref().ok(), Some("com.epicgames.fortnite2"));
        // base + both splits staged (obb + icons skipped); base first.
        assert_eq!(installer.groups.borrow().len(), 1);
        let group = &installer.groups.borrow()[0];
        assert_eq!(group.len(), 3);
        assert_eq!(group[0].file_name().unwrap(), "base.apk");
        // The temp xapk_ dir is cleaned up.
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            leftovers,
            vec!["Fortnite-14.60-15534296.apk".to_string()],
            "only the downloaded file remains: {leftovers:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn install_error_paths() {
        let dir = test_dir("errors");

        // No id and no direct URL.
        let backend = MockBackend {
            payload: Vec::new(),
            status: 200,
            total: None,
            short: false,
        };
        let mut launcher = UptodownInstaller::new();
        let mut states = Vec::new();
        let err = launcher
            .install(
                &backend,
                &mut crate::install::NoopInstaller::new(),
                &dir,
                &version("13.40", 1, None, ""),
                |_| {},
                |st| states.push(st.clone()),
            )
            .unwrap_err();
        assert_eq!(
            err,
            "Version 13.40 has no Uptodown id and no direct APK URL"
        );
        assert_eq!(
            states.last(),
            Some(&InstallState::Error {
                message: err.clone()
            })
        );

        // Resolver finds nothing (the mock download page has no buttons).
        let mut launcher = UptodownInstaller::new();
        let err = launcher
            .install(
                &backend,
                &mut crate::install::NoopInstaller::new(),
                &dir,
                &version("13.40", 1, Some(99), ""),
                |_| {},
                |_| {},
            )
            .unwrap_err();
        assert_eq!(
            err,
            "Could not resolve the download link (Uptodown page changed?)"
        );

        // Not a zip package.
        let backend = MockBackend {
            payload: b"this is not a zip".to_vec(),
            status: 200,
            total: None,
            short: false,
        };
        let mut launcher = UptodownInstaller::new();
        let err = launcher
            .install(
                &backend,
                &mut crate::install::NoopInstaller::new(),
                &dir,
                &version("13.40", 1, None, "https://mock/file.apk"),
                |_| {},
                |_| {},
            )
            .unwrap_err();
        assert_eq!(err, "Downloaded file for 13.40 is not a valid package");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn install_failure_message_from_on_error() {
        // A failing installer surfaces its describe() through on_error.
        let dir = test_dir("failmsg");
        struct FailingInstaller;
        impl Installer for FailingInstaller {
            fn create_session(&mut self) -> Result<crate::install::InstallSession, String> {
                Ok(crate::install::InstallSession {
                    id: 1,
                    apks: Vec::new(),
                })
            }
            fn submit(
                &mut self,
                _s: &mut crate::install::InstallSession,
                _a: &Path,
            ) -> Result<(), String> {
                Ok(())
            }
            fn commit(
                &mut self,
                _s: crate::install::InstallSession,
                _on_status: &mut dyn FnMut(String),
            ) -> crate::install::InstallResult {
                crate::install::InstallResult::failure(
                    -2,
                    "INSTALL_FAILED_UPDATE_INCOMPATIBLE",
                )
            }
            fn abandon(&mut self, _s: crate::install::InstallSession) {}
            fn on_result(&mut self, _s: i32, _st: i32, _m: Option<String>) -> bool {
                false
            }
        }

        let mut payload = Vec::new();
        zip_build_stored_bytes(&mut payload, &[("base.apk", b"PK\x03\x04x".to_vec())]);
        let backend = MockBackend {
            payload,
            status: 200,
            total: None,
            short: false,
        };
        let mut launcher = UptodownInstaller::new();
        let mut states = Vec::new();
        let err = launcher
            .install(
                &backend,
                &mut FailingInstaller,
                &dir,
                &version("13.40", 1, None, "https://mock/file.apk"),
                |_| {},
                |st| states.push(st.clone()),
            )
            .unwrap_err();
        assert_eq!(
            err,
            "The installed game has a different signature. Uninstall it first, then install again from Owen."
        );
        assert_eq!(
            states.last(),
            Some(&InstallState::Error { message: err })
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_constants_match_kotlin() {
        assert_eq!(DEFAULT_BACKEND_URL, "http://127.0.0.1:3551");
        assert_eq!(FALLBACK_PACKAGE, "com.epicgames.fortnite");
        assert_eq!(BUFFER_SIZE, 512 * 1024);
        assert_eq!(CONNECT_TIMEOUT_MS, 30_000);
        assert_eq!(READ_TIMEOUT_MS, 60_000);
    }
}
