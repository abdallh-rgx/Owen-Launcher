//! The zero-smali patch plan — the Rust replacement for `SmaliPatcher.kt` +
//! the `DexTools` step of `ApkPatcher.kt`.
//!
//! ## Kotlin → zero-smali map
//!
//! | Kotlin (`SmaliPatcher.kt` / `DexTools.kt` / `ApkPatcher.kt`) | Rust executor |
//! |--------------------------------------------------------------|---------------|
//! | `DexTools.disassemble` (baksmali, in-process) + `SmaliPatcher.injectOwenLoader` (`const-string "Owen"` + `System.loadLibrary` into `GameActivity.onCreate`) | **No dex round-trip at all**: [`crate::needed::inject_needed`] writes `libOwen.so` into the engine library's `DT_NEEDED` list, so the Android linker loads Owen before the engine runs. The step is still reported as `Injecting Owen loader...` in [`crate::PatchReport::steps`]. |
//! | `SmaliPatcher.patchGameActivityConsole` (`dispatchTouchEvent` + `forceShowConsoleWindow_test` appended to `GameActivity.smali`) | [`PatchPlan::console_patch`] — the flag is funnelled into `owen.json` (`console.enabled`, see [`PatchPlan::owen_config_json`]) which `libowen.so` (owen-redirecter) reads at startup and applies at runtime through its ART hook layer. |
//! | `SmaliPatcher.patchPartyHub` (loginUrl in `AuthFragment.smali`, graphqlUrl in `KernelImpl.smali`, seasons 12–15) | [`PatchPlan::partyhub`] — funnelled into `owen.json` (`webview.partyhub = "auto"|"off"`, `server.base_url`, `webview.login_url`); the redirecter's existing hooks (`curl_hook`, `art_hooks`) apply it at runtime. |
//! | `DexTools.assemble` / `assembleFileValidates` (smali → dex) | No-op: nothing was disassembled, `classes.dex` is copied byte-identically. The step name is kept (`Rebuilding dex...`) so [`crate::PatchReport::steps`] stays 1:1 with the Kotlin pipeline. |
//! | `SmaliPatcher.gameActivityPaths(major)` / `preferredFirst` (version-specific smali paths) | Not needed — DT_NEEDED + runtime hooks are version-independent. `major_version` is still carried for reporting. |
//!
//! `owen.json` schema written by [`PatchPlan::owen_config_json`] mirrors
//! `owen_redirecter::config::OwenConfig`:
//!
//! ```json
//! {
//!   "version": 1,
//!   "server":   { "base_url": "...", "xmpp_host": "127.0.0.1", "xmpp_port": 5222,
//!                 "login_path": "/login", "graphql_path": "/partyhub/graphql" },
//!   "webview":  { "login_url": "...", "force_cleartext": true, "partyhub": "auto" },
//!   "console":  { "enabled": false }
//! }
//! ```
//!
//! The redirecter reads it from `/sdcard/Android/data/<pkg>/files/owen.json`
//! (see `owen-redirecter/src/config.rs`), so the launcher pushes the emitted
//! file there after installing the patched APK.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/{SmaliPatcher.kt,DexTools.kt,ApkPatcher.kt}

use crate::native_builder;
use std::path::{Path, PathBuf};

/// Options of one zero-smali patch run — the parameter set of
/// `ApkPatcher.patchApk` plus the two smali-feature flags.
#[derive(Debug, Clone)]
pub struct PatchPlan {
    /// `backendUrl` — where the game should connect (patched into
    /// `libOwen.so` by the caller via [`native_builder`], and written into
    /// `owen.json` here).
    pub backend_url: String,
    /// `loginUrl` — the PartyHub WebView login page.
    pub login_url: String,
    /// `owenLib` — prebuilt `libOwen.so` to embed.
    pub owen_lib: PathBuf,
    /// `targetPackage` — optional package rename (ManifestRenamer).
    pub target_package: Option<String>,
    /// `isDev` — dev builds already ship a console (Kotlin skips the console
    /// patch for them).
    pub is_dev: bool,
    /// `majorVersion` — Fortnite major version (smali path selection in the
    /// Kotlin pipeline; reporting only here).
    pub major_version: i32,
    /// Console patch flag (`SmaliPatcher.patchGameActivityConsole`).
    /// The Kotlin pipeline ran it when `!isDev`.
    pub console_patch: bool,
    /// PartyHub flag (`SmaliPatcher.patchPartyHub`, seasons 12–15).
    pub partyhub: bool,
    /// Override which native library receives the DT_NEEDED entry
    /// (`None` = auto-detect libUnreal.so → libUE4.so).
    pub target_lib: Option<String>,
    /// Skip signing (debug).
    pub no_sign: bool,
    /// Signing key path (generated on first use); `None` = `owen-key.pem`.
    pub key_path: Option<PathBuf>,
    /// `ApkPatcher.kt`'s 2,000,000-byte "repacking produced no APK" gate.
    /// Synthetic test APKs set this to 0 to disable the floor.
    pub min_output_bytes: u64,
    /// Write the derived `owen.json` next to the output APK (the runtime
    /// replacement for the console/PartyHub smali patches).
    pub emit_owen_config: bool,
}

impl PatchPlan {
    /// The plan matching the legacy `PatchOptions` JNI entry point.
    pub fn from_options(opts: &crate::PatchOptions) -> Self {
        PatchPlan {
            backend_url: String::new(),
            login_url: String::new(),
            owen_lib: opts.owen_lib.clone(),
            target_package: opts.rename_package.clone(),
            is_dev: false,
            major_version: 0,
            // The legacy entry never edited dex for the console; keep the
            // runtime flag off so behavior stays identical.
            console_patch: false,
            partyhub: false,
            target_lib: opts.target_lib.clone(),
            no_sign: opts.no_sign,
            key_path: opts.key_path.clone(),
            min_output_bytes: 0,
            emit_owen_config: false,
        }
    }

    /// The `owen.json` payload for this plan — the schema mirrors
    /// `owen_redirecter::config::OwenConfig` (see the module map above).
    pub fn owen_config_json(&self) -> String {
        let base_url = native_builder::sanitize_url(&self.backend_url);
        let login_url = if self.login_url.is_empty() {
            format!("{}/login", base_url.trim_end_matches('/'))
        } else {
            self.login_url.clone()
        };
        serde_json::json!({
            "version": 1,
            "server": {
                "base_url": base_url,
                "xmpp_host": "127.0.0.1",
                "xmpp_port": 5222,
                "login_path": "/login",
                "graphql_path": "/partyhub/graphql"
            },
            "webview": {
                "login_url": login_url,
                "force_cleartext": true,
                "partyhub": if self.partyhub { "auto" } else { "off" }
            },
            "console": { "enabled": self.console_patch }
        })
        .to_string()
    }

    /// Write `owen.json` next to `out_apk` and return its path.
    pub fn write_owen_config(&self, out_apk: &Path) -> std::io::Result<PathBuf> {
        let path = out_apk
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("owen.json");
        std::fs::write(&path, self.owen_config_json())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owen_config_json_flags() {
        let mut plan = PatchPlan::from_options(&crate::PatchOptions::default());
        plan.backend_url = "http://192.168.1.5:3551".into();
        let json = plan.owen_config_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["server"]["base_url"], "http://192.168.1.5:3551");
        assert_eq!(v["webview"]["partyhub"], "off");
        assert_eq!(v["console"]["enabled"], false);
        assert_eq!(v["webview"]["login_url"], "http://192.168.1.5:3551/login");

        plan.partyhub = true;
        plan.console_patch = true;
        let v: serde_json::Value = serde_json::from_str(&plan.owen_config_json()).unwrap();
        assert_eq!(v["webview"]["partyhub"], "auto");
        assert_eq!(v["console"]["enabled"], true);
    }

    #[test]
    fn owen_config_json_parses_as_redirecter_schema() {
        // The emitted JSON must deserialize into the redirecter's OwenConfig
        // shape (checked structurally here: same keys, same enum values).
        let mut plan = PatchPlan::from_options(&crate::PatchOptions::default());
        plan.backend_url = "http://10.0.0.2:3551".into();
        plan.login_url = "http://10.0.0.2:3551/login".into();
        plan.partyhub = true;
        plan.console_patch = true;
        let v: serde_json::Value = serde_json::from_str(&plan.owen_config_json()).unwrap();
        for key in ["version", "server", "webview", "console"] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert_eq!(v["server"]["login_path"], "/login");
        assert_eq!(v["server"]["graphql_path"], "/partyhub/graphql");
    }
}
