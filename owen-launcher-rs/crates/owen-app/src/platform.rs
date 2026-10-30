//! Platform facade — ONE safe API, three hosts:
//!
//! * **Android** (`libowenapp.so`): the JNI bridge in [`crate::jni_android`].
//! * **Desktop**: plain std/fs + a real owen-patcher pipeline (the patched
//!   APK is produced and signed; "install" completes when it sits on disk).
//! * **wasm** (the preview build): everything heavy is disabled; the app
//!   drives the hosted Owen Server through the gateway.

use owen_launcher::install::helper::{InstallResult, InstallSession, Installer};
use std::path::{Path, PathBuf};

pub enum Host {
    Android,
    Desktop,
    Wasm,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Host::Android => "android",
            Host::Desktop => "desktop",
            Host::Wasm => "wasm",
        })
    }
}

pub fn host() -> Host {
    #[cfg(target_os = "android")]
    {
        Host::Android
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        Host::Desktop
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        Host::Wasm
    }
}

impl Host {
    pub fn is_wasm(&self) -> bool {
        matches!(self, Host::Wasm)
    }

    pub fn is_android(&self) -> bool {
        matches!(self, Host::Android)
    }

    /// The original first-time-setup gate: install permission + battery
    /// exemption + player name, re-checked on every launch.
    pub fn needs_setup(&self, data_root: &Path) -> bool {
        if self.is_wasm() {
            return false; // preview: no android permissions to grant
        }
        if !owen_launcher::core::player_store::has(data_root) {
            return true;
        }
        #[cfg(target_os = "android")]
        {
            let (perm, batt) = crate::jni_android::perm_status();
            !perm || !batt
        }
        #[cfg(not(target_os = "android"))]
        {
            false
        }
    }
}

/* ------------------------------- directories ------------------------------- */

pub fn data_root() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::files_dir()
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        if let Some(dir) = std::env::var_os("OWEN_DATA_DIR") {
            return PathBuf::from(dir);
        }
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("owen-data")
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        PathBuf::from("/owen-data")
    }
}

/// The bundled trees (lawin/owen/images/tools). Android: the APK assets are
/// materialized once into `cacheDir/bundled` by [`crate::jni_android`].
pub fn bundled_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::bundled_dir()
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        // walk up from cwd (and from the exe) to find the repo `data` dir
        let markers = ["data/owen", "data/lawin"];
        let from = [
            std::env::current_dir().ok(),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf)),
        ];
        let mut root: Option<PathBuf> = None;
        'outer: for base in from.into_iter().flatten() {
            let mut cur: Option<PathBuf> = Some(base.clone());
            while let Some(dir) = cur {
                for m in markers.iter() {
                    if dir.join(m).is_dir() {
                        root = Some(dir.join("data"));
                        break 'outer;
                    }
                }
                cur = dir.parent().map(Path::to_path_buf);
            }
        }
        root.unwrap_or_else(|| PathBuf::from("data"))
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        PathBuf::new()
    }
}

pub fn cache_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::cache_dir()
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        std::env::temp_dir()
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        PathBuf::from("/tmp")
    }
}

/// The backend base URL as a string (native). Empty on wasm.
pub fn base_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        String::new()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        owen_launcher::uptodown::installer::DEFAULT_BACKEND_URL.to_string()
    }
}

/// Path of the redirecter .so that gets embedded into patched games.
/// Android: `nativeLibraryDir/libOwen.so`. Desktop: the aarch64-android
/// build output when present.
pub fn owen_lib_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::native_library_dir().join("libOwen.so")
    }
    #[cfg(not(target_os = "android"))]
    {
        let marker = PathBuf::from("target/aarch64-linux-android/release/libOwen.so");
        let mut cur = std::env::current_dir().ok();
        let mut found: Option<PathBuf> = None;
        while let Some(dir) = cur {
            if dir.join(&marker).is_file() {
                found = Some(dir.join(&marker));
                break;
            }
            cur = dir.parent().map(Path::to_path_buf);
        }
        found.unwrap_or(marker)
    }
}

/* ----------------------------- OS interactions ----------------------------- */

pub fn open_url(url: &str) {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::open_url(url);
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        let _ = url;
    }
}

pub fn request_install_permission() {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::request_install_permission();
    }
}

pub fn request_battery_exemption() {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::request_battery_exemption();
    }
}

pub fn launch_game(pkg: &str, code: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::launch_game(pkg, code)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (pkg, code);
        false
    }
}

pub fn uninstall_package(pkg: &str) {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::uninstall_package(pkg);
    }
}

/// Installed check for a renamed-package scheme name.
pub fn installed_package(pkg: &str) -> Option<(String, i64)> {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::installed_package(pkg)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = pkg;
        None
    }
}

/* ------------------------------ install layer ------------------------------ */

/// Context the patch step needs (handed to the next `install()`).
pub struct InstallCtx {
    /// Patcher-side version record.
    pub version: owen_patcher::owen_manager::OgVersion,
    pub backend_url: String,
    pub login_url: String,
    pub cache_dir: PathBuf,
    pub work_dir: PathBuf,
    pub owen_lib: PathBuf,
    pub key_path: PathBuf,
}

static INSTALL_CTX: std::sync::Mutex<Option<InstallCtx>> = std::sync::Mutex::new(None);

/// Hand the patch context to the next `install()` of any host installer.
pub fn set_install_ctx(ctx: InstallCtx) {
    *INSTALL_CTX.lock().unwrap() = Some(ctx);
}

/// Installers pick their context up here (single install thread at a time).
pub fn take_install_ctx() -> Option<InstallCtx> {
    INSTALL_CTX.lock().unwrap().take()
}

/// The shared patch+sign+cache core (host-agnostic; `owen-patcher` runs
/// everywhere). Returns the patched APK path ready for the OS install step.
pub fn patch_step(
    ctx: &InstallCtx,
    input: &Path,
    on_status: &mut dyn FnMut(String),
) -> Result<(PathBuf, String), String> {
    // 0. extra companion libs (E2E seam): a `owen-libs.json` next to the
    // app's data root names bare file names; the bytes are read from
    // `<files>/ogs/<name>`. Stock installs have no such file → empty list.
    let extra_libs = resolve_extra_libs();

    // 1. cache hit? (the key folds in the extra-lib fingerprint so a
    // changed dumper/OGS lib invalidates the cache)
    let mut cache = owen_patcher::patch_cache::PatchCache::open(&ctx.cache_dir);
    let key = owen_patcher::patch_cache::key_for_with(
        &ctx.version,
        &ctx.backend_url,
        if owen_patcher::owen_manager::is_party_hub_version(&ctx.version) {
            ctx.login_url.as_str()
        } else {
            ""
        },
        &extra_libs_fingerprint(&extra_libs),
    );
    if let Some(hit) = cache.lookup(&key) {
        on_status("Patched build restored from cache".into());
        let pkg = owen_patcher::owen_manager::package_name_for(&ctx.version);
        return Ok((hit, pkg));
    }

    // 2. compute the target package the way OwenManager does
    let target_pkg =
        owen_patcher::owen_manager::compute_renamed_package(&ctx.version, None);

    // 3. full patch pipeline
    let out = ctx
        .work_dir
        .join(format!("Fortnite-{}-{}-owen.apk", ctx.version.version, ctx.version.cl));
    let opts = owen_patcher::PatchOptions {
        owen_lib: ctx.owen_lib.clone(),
        target_lib: None, // auto-detect (libUnreal.so first, then libUE4.so)
        no_sign: false,
        key_path: Some(ctx.key_path.clone()),
        rename_package: target_pkg.clone(),
        extra_libs: extra_libs.clone(),
    };
    let plan = owen_patcher::patch_plan::PatchPlan::from_options(&opts);
    match owen_patcher::patch_apk_with_plan(input, &out, &plan) {
        Ok(report) => {
            for step in report.steps.iter() {
                on_status(step.clone());
            }
            let final_pkg = report
                .final_package
                .clone()
                .or(target_pkg)
                .unwrap_or_else(|| owen_patcher::owen_manager::package_name_for(&ctx.version));
            let _ = cache.store(&key, &out);
            Ok((out, final_pkg))
        }
        Err(f) => Err(format!("{f:?}")),
    }
}

/* --------------------------- the host installers --------------------------- */

/// Read `<data root>/owen-libs.json` (a plain JSON array of bare file
/// names, e.g. `["libOwenDumper.so"]`) and pair each with
/// `<data root>/ogs/<name>`. Any missing file drops out with a logged
/// warning instead of failing the patch.
fn resolve_extra_libs() -> Vec<(String, std::path::PathBuf)> {
    let root = data_root();
    let list = root.join("owen-libs.json");
    let Ok(text) = std::fs::read_to_string(&list) else {
        return Vec::new();
    };
    let Ok(names) = serde_json::from_str::<Vec<String>>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for name in names {
        if name.contains('/') || name.contains('\\') {
            continue;
        }
        let path = root.join("ogs").join(&name);
        if path.is_file() {
            out.push((name, path));
        }
    }
    out
}

/// Conservative cache fingerprint for the extra libs: name + length +
/// first 8 bytes. Enough to invalidate on any real change without a hash
/// dependency (the CI device starts with an empty cache anyway).
fn extra_libs_fingerprint(libs: &[(String, std::path::PathBuf)]) -> String {
    if libs.is_empty() {
        return String::new();
    }
    libs.iter()
        .map(|(name, path)| {
            let head = std::fs::read(path)
                .ok()
                .map(|b| {
                    b.iter()
                        .take(8)
                        .map(|x| format!("{x:02x}"))
                        .collect::<String>()
                })
                .unwrap_or_default();
            let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            format!("{name}:{len}:{head}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Android: real PackageInstaller sessions through the JNI bridge; the
/// patch core runs before the session.
#[cfg(target_os = "android")]
pub fn make_installer() -> Box<dyn Installer> {
    Box::new(crate::jni_android::JniInstaller::new())
}

/// Desktop: the full patch+sign pipeline; "install" completes when the
/// patched APK sits on disk (no OS installer on a desktop).
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
pub fn make_installer() -> Box<dyn Installer> {
    Box::new(DesktopInstaller)
}

/// wasm preview: the Noop installer (the UI shows the preview note).
#[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
pub fn make_installer() -> Box<dyn Installer> {
    Box::new(owen_launcher::install::helper::NoopInstaller)
}

#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
pub struct DesktopInstaller;

#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
impl Installer for DesktopInstaller {
    fn create_session(&mut self) -> Result<InstallSession, String> {
        Ok(InstallSession { id: 1, apks: Vec::new() })
    }
    fn submit(&mut self, _session: &mut InstallSession, _apk: &Path) -> Result<(), String> {
        Ok(())
    }
    fn commit(
        &mut self,
        _session: InstallSession,
        _on_status: &mut dyn FnMut(String),
    ) -> InstallResult {
        InstallResult::new(true, 0, Some("patched apk ready".into()))
    }
    fn abandon(&mut self, _session: InstallSession) {}
    fn on_result(&mut self, _session_id: i32, _status: i32, _message: Option<String>) -> bool {
        false
    }
    fn install(&mut self, apks: &[PathBuf], on_status: &mut dyn FnMut(String)) -> InstallResult {
        let Some(ctx) = take_install_ctx() else {
            return InstallResult::new(false, -99, Some("no install context".into()));
        };
        let Some(base) = apks.first() else {
            return InstallResult::new(false, -99, Some("no apk".into()));
        };
        match patch_step(&ctx, base, on_status) {
            Ok((path, pkg)) => {
                on_status(format!("patched: {} → {}", path.display(), pkg));
                InstallResult::new(true, 0, Some(pkg))
            }
            Err(e) => InstallResult::new(false, -99, Some(e)),
        }
    }
}
