//! Shared helpers for the owen-ogs boot path, used by `full.rs` (and kept
//! linkable-but-unused by the stub build so `cargo check` stays green
//! without the C++ side).
//!
//! * `ogs_log`    — logcat/stderr logging (same policy as
//!   `crates/owen-redirecter/src/config.rs::owen_log`).
//! * `ogs_config` — `ogsen.json` discovery + merge, mirroring the candidate
//!   paths of `crates/owen-redirecter/src/config.rs`:
//!   `$OGSEN_CONFIG`, `/sdcard/Android/data/<pkg>/files/ogsen.json`
//!   (pkg from `/proc/self/cmdline`), `/data/local/tmp/ogsen.json`;
//!   built-in defaults when none is found.  The merged config is written
//!   back as `ogsen_config_effective.json` next to the game files dir
//!   (best effort) and handed to the C++ `ogs_main(const char*)`.

pub mod ogs_log {
    #[cfg(target_os = "android")]
    extern "C" {
        fn __android_log_print(prio: i32, tag: *const u8, fmt: *const u8, ...) -> i32;
    }

    #[cfg(target_os = "android")]
    fn log(prio: i32, msg: &str) {
        let tag = b"OwenOGS\0";
        let fmt = b"%s\0";
        let c = std::ffi::CString::new(msg.replace('\0', " ")).unwrap_or_default();
        unsafe {
            __android_log_print(prio, tag.as_ptr(), fmt.as_ptr(), c.as_ptr());
        }
    }

    #[cfg(not(target_os = "android"))]
    fn log(prio: i32, msg: &str) {
        let tag = if prio <= 3 {
            "E"
        } else if prio == 4 {
            "W"
        } else {
            "I"
        };
        eprintln!("[{tag}/OwenOGS] {msg}");
    }

    pub fn info(msg: &str) {
        log(4, msg);
    }
    pub fn warn(msg: &str) {
        log(5, msg);
    }
    pub fn error(msg: &str) {
        log(6, msg);
    }
}

pub mod ogs_config {
    use serde_json::{json, Map, Value};
    use std::path::PathBuf;

    /// Baked defaults.  The three offset-gated features default to "on iff
    /// the scanner found the backing offset" — build.rs emits
    /// `--cfg ogs_off_found_<key>` for every found entry in
    /// offsets/ogs-1241-android.json:
    ///
    /// * `netmode_hooks`  ← uworld_getnetmode (FOUND in 12.41-android)
    /// * `gis_flip`       ← gis_client        (FOUND; note gis_server was
    ///   NOT found — the flip then only writes GIsClient=false)
    /// * `listen_server`  ← create_netdriver  (NOT found → off)
    pub fn default_config() -> Value {
        json!({
            "version": 1,
            "game": "12.41",
            "features": {
                "netmode_hooks": cfg!(ogs_off_found_uworld_getnetmode),
                "gis_flip": cfg!(ogs_off_found_gis_client),
                "listen_server": cfg!(ogs_off_found_create_netdriver),
                "bots": true,
                "bosses": true,
                "booths": true,
                "vehicles": true,
                "late_game": false,
                "exec_hooks": true,
                "bots_tick_thread": true,
                "legacy_ai_mang": true,
                "max_bots": 100,
                "min_players_early_start": 100,
                // Seconds to hold at the lobby BEFORE `open Apollo_Terrain`
                // fires — the E2E driver takes its lobby screenshot inside
                // this window (see ogs_all.cpp ogs_main).
                "world_open_delay": 90
            },
            "vt_indices": {}
        })
    }

    fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(env) = std::env::var("OGSEN_CONFIG") {
            paths.push(PathBuf::from(env));
        }
        // Per-package external dir: /sdcard/Android/data/<pkg>/files/ogsen.json
        if let Ok(cmdline) = std::fs::read_to_string("/proc/self/cmdline") {
            let pkg = cmdline.split('\0').next().unwrap_or("").trim();
            if !pkg.is_empty() {
                paths.push(PathBuf::from(format!(
                    "/sdcard/Android/data/{pkg}/files/ogsen.json"
                )));
            }
        }
        paths.push(PathBuf::from("/data/local/tmp/ogsen.json"));
        paths
    }

    /// Package files dir (used for the effective-config/log sidecars), the
    /// same way owen-redirecter derives its owen.json path.
    fn package_files_dir() -> Option<PathBuf> {
        std::fs::read_to_string("/proc/self/cmdline").ok().and_then(|cmdline| {
            let pkg = cmdline.split('\0').next().unwrap_or("").trim();
            if pkg.is_empty() {
                None
            } else {
                Some(PathBuf::from(format!("/sdcard/Android/data/{pkg}/files")))
            }
        })
    }

    fn merge_file(cfg: &mut Value, text: &str, path: &str) -> Result<(), String> {
        let parsed: Value =
            serde_json::from_str(text).map_err(|e| format!("bad JSON: {e}"))?;
        let obj = match parsed {
            Value::Object(o) => o,
            _ => return Err("top level is not an object".into()),
        };
        for (k, v) in obj {
            match (cfg.get_mut(&k), &v) {
                (Some(Value::Object(dst)), Value::Object(src)) => {
                    // shallow-merge "features" / "vt_indices" so a partial
                    // user config only overrides what it mentions
                    for (fk, fv) in src {
                        dst.insert(fk.clone(), fv.clone());
                    }
                }
                _ => {
                    cfg.as_object_mut()
                        .unwrap()
                        .insert(k, v);
                }
            }
        }
        let _ = path;
        Ok(())
    }

    /// Load, merge and serialize the effective config; also best-effort
    /// write it next to the log as `ogsen_config_effective.json`.
    /// Returns the JSON string handed to ogs_main.
    pub fn effective_config_json() -> (String, Option<String>) {
        let mut cfg = default_config();
        let mut loaded_from: Option<String> = None;
        for path in candidate_paths() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                match merge_file(&mut cfg, &text, &path.to_string_lossy()) {
                    Ok(()) => {
                        loaded_from = Some(path.to_string_lossy().into_owned());
                        break;
                    }
                    Err(e) => super::ogs_log::warn(&format!(
                        "owen-ogs: bad config at {}: {e}",
                        path.display()
                    )),
                }
            }
        }
        let rendered = serde_json::to_string_pretty(&cfg).unwrap_or_else(|_| "{}".into());
        if let Some(dir) = package_files_dir() {
            let out = dir.join("ogsen_config_effective.json");
            if std::fs::write(&out, &rendered).is_ok() {
                super::ogs_log::info(&format!(
                    "owen-ogs: effective config written to {}",
                    out.display()
                ));
            }
        }
        match &loaded_from {
            Some(p) => super::ogs_log::info(&format!("owen-ogs: config loaded from {p}")),
            None => super::ogs_log::info("owen-ogs: no ogsen.json found — using built-in defaults"),
        }
        (rendered, loaded_from)
    }
}
