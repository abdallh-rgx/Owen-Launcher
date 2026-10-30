//! libowen.so — the universal Owen Redirecter, rewritten in safe Rust.
//!
//! Entry points:
//! * `JNI_OnLoad` — invoked by `System.loadLibrary("Owen")` (the standard
//!   injection used by owen-apk).
//! * `.init_array` constructor — fires on dlopen, before the JVM exists; the
//!   worker thread waits for the game libraries either way.
//!
//! Orchestration (identical guarantees to the original, minus the offset
//! tables):
//! 1. Load `owen.json` (HTTP or HTTPS backend URL, verbatim).
//! 2. Wait for libUE4.so / libUnreal.so (+ libEOSSDK.so on 22.00+).
//! 3. Locate `curl_easy_setopt` per library: dlsym first, owen-detect v2
//!    signature scan second — this is what makes every version from 10.31
//!    to 33.00+ work with zero per-version tables.
//! 4. Install the curl hook, the ART WebView/Kairos layer and the cleartext
//!    unlock, then report the detected game version to the backend.

mod art_hooks;
mod config;
mod curl_hook;
mod detect;
mod maps;
mod report;
mod rewrite;

/// Redirect rules shared with the backend tooling — mirrors
/// `owen_core::urlrule` but keeps a copy inside the library so hook
/// callbacks (which must never block on locks) read plain statics.
/// Preserved verbatim from the previous owen-rs redirecter so
/// `owen_redirecter::rules` users keep the same paths.
pub mod rules;

use config::owen_log;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);

/// The libraries we hook, in priority order.
///
/// `libUE4real.so` comes first: with the shim injection (owen-patcher
/// `shim_patch`), the engine library is renamed to `libUE4real.so` and the
/// 20KB loader shim takes the `libUE4.so` slot — the real engine must win
/// every module-name race (hook target, version detect, signature scan).
const TARGET_LIBS: &[&str] = &["libUE4real.so", "libUE4.so", "libUnreal.so", "libEOSSDK.so"];

#[no_mangle]
pub extern "C" fn JNI_OnLoad(_vm: *mut c_void, _reserved: *mut c_void) -> i32 {
    start();
    0x0001_0006 // JNI_VERSION_1_6
}

/// dlopen-time constructor. Runs before JNI_OnLoad on some loaders.
#[used]
#[link_section = ".init_array"]
static INIT: extern "C" fn() = owen_early_init;

extern "C" fn owen_early_init() {
    start();
}

fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("owen-main".into())
        .spawn(worker)
        .ok();
}

fn worker() {
    let cfg = config::OwenConfig::load();
    {
        if let Ok(mut guard) = curl_hook::CONFIG.write() {
            *guard = Some(cfg.clone());
        }
    }
    owen_log::info(&format!(
        "owen: starting (backend {}, manifest {}, partyhub {})",
        cfg.server.base_url,
        match cfg.manifest.mode {
            config::ManifestMode::Auto => "auto",
            config::ManifestMode::Off => "off",
        },
        match cfg.webview.partyhub {
            config::PartyHubMode::Auto => "auto",
            config::PartyHubMode::Off => "off",
        },
    ));

    // ---- Layer 2: ART hooks (best effort, graceful degradation) -----------
    #[cfg(target_os = "android")]
    if cfg.hooks.art_webview && cfg.webview.partyhub == config::PartyHubMode::Auto {
        // The JVM may not be up yet when the constructor fires; retry briefly.
        for _ in 0..20 {
            let ok = art_hooks::install_std();
            if ok {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    // ---- Layer 1: curl hooks (the universal path) -------------------------
    if !cfg.hooks.curl {
        owen_log::info("owen: curl hook disabled by config");
        return;
    }

    let mut hooked: Vec<String> = Vec::new();
    let mut version: Option<String> = None;

    // Wait up to 20s for the first engine library (libUE4real first: the
    // shim injection renames the engine; the shim itself carries no curl).
    let Some((first_name, first_mod)) =
        maps::wait_for_module(&["libUE4real.so", "libUE4.so", "libUnreal.so"], 20)
    else {
        owen_log::warn("owen: no engine library appeared within 20s");
        return;
    };

    if let Some(v) = detect::detect_version(&first_mod) {
        owen_log::info(&format!("owen: detected game version {v}"));
        version = Some(v);
    }

    if let Some((addr, how)) = detect::locate(
        &first_name,
        &first_mod,
        cfg.detect.strategy,
        cfg.detect.min_score,
    ) {
        match curl_hook::install_hook(addr) {
            Ok(()) => {
                owen_log::info(&format!(
                    "owen: {first_name} curl_easy_setopt @ 0x{addr:X} ({how}) hooked"
                ));
                hooked.push(first_name.clone());
            }
            Err(e) => owen_log::warn(&format!("owen: hook install failed on {first_name}: {e}")),
        }
    } else {
        owen_log::warn(&format!(
            "owen: curl_easy_setopt not found in {first_name} ({} candidates below threshold or none)",
            ""
        ));
    }

    // libEOSSDK.so ships with 22.00+; wait a little longer if absent.
    if hooked.is_empty() || first_name == "libUnreal.so" {
        if let Some(eos) = maps::wait_for_module(&["libEOSSDK.so"], 15) {
            let (eos_name, eos_mod) = eos;
            if let Some((addr, how)) =
                detect::locate(&eos_name, &eos_mod, cfg.detect.strategy, cfg.detect.min_score)
            {
                if curl_hook::install_hook(addr).is_ok() {
                    owen_log::info(&format!(
                        "owen: {eos_name} curl_easy_setopt @ 0x{addr:X} ({how}) hooked"
                    ));
                    hooked.push(eos_name);
                }
            }
        }
    }

    // ---- Version report ----------------------------------------------------
    if let Some(v) = version {
        report::send_version_report(&cfg, &v);
    }

    if hooked.is_empty() {
        owen_log::error("owen: no library could be hooked — redirecting inactive");
    } else {
        owen_log::info(&format!("owen: active on {}", hooked.join(", ")));
    }
}
