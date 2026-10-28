//! Layer 2 — the ART hooks that replace every smali patch:
//!
//! * `WebView.loadUrl(String)` and `loadUrl(String, Map)` rewritten through
//!   the same engine (login page, partyhub pages).
//! * `DebouncedLimiter.loginUrl(Environment, boolean)` and
//!   `DebouncedLimiter.getGraphqlUrl(Environment)` return our backend URLs —
//!   the runtime equivalent of the old `const-string` smali edits for
//!   versions 10.31 through 15.50.
//! * `NetworkSecurityPolicy.isCleartextTrafficPermitted(String)` returns
//!   `true`, which is what makes plain-HTTP login pages loadable.
//!
//! Everything degrades gracefully: on a non-Kairos build the Kairos classes
//! simply do not exist, the lookups fail and the layer continues with the
//! WebView hooks only.

#![cfg(target_os = "android")]

use crate::curl_hook::CONFIG;
use crate::rewrite;
use owen_art::jni;
use std::ffi::c_void;

pub struct ArtLayer {
    pub layout: owen_art::ArtLayout,
}

/// Global layer state (written once during setup; read from the UI thread
/// where all hooked methods live).
static mut LAYER: Option<ArtLayer> = None;

/// Install all Layer-2 hooks. Returns false when the JVM is not reachable
/// yet (caller retries).
pub fn install_std() -> bool {
    let cfg = {
        match CONFIG.read() {
            Ok(g) => g.clone(),
            Err(_) => return false,
        }
    };
    let Some(cfg) = cfg else { return false };

    jni::with_env(|env| {
        let layout = owen_art::calibrate(env)?;
        install_webview(env, &layout);
        install_kairos(env, &layout);
        if cfg.webview.force_cleartext {
            install_cleartext_unlock(env, &layout);
        }
        // SAFETY: single-threaded setup on the worker thread before any
        // hook can fire (hooks are installed last within each sub-step and
        // only read `LAYER` after it is assigned here).
        unsafe {
            LAYER = Some(ArtLayer { layout });
        }
        Some(())
    })
    .is_some()
}

fn install_webview(env: *mut jni::JNIEnv, layout: &owen_art::ArtLayout) {
    let Some(webview) = jni::find_class(env, b"android/webkit/WebView\0") else {
        crate::config::owen_log::warn("owen: android/webkit/WebView not found");
        return;
    };
    let variants: &[(&[u8], &[u8], usize, &str)] = &[
        (
            b"loadUrl\0",
            b"(Ljava/lang/String;)V\0",
            webview_loadurl as usize,
            "WebView.loadUrl(String)",
        ),
        (
            b"loadUrl\0",
            b"(Ljava/lang/String;Ljava/util/Map;)V\0",
            webview_loadurl2 as usize,
            "WebView.loadUrl(String, Map)",
        ),
    ];
    for (name, sig, handler, label) in variants {
        let Some(m) = jni::get_method_id(env, webview, name, sig) else {
            continue;
        };
        // SAFETY: verified jmethodID from JNI and a matching extern "C" handler.
        match unsafe { owen_art::hook_method(env, layout, m as *mut c_void, *handler) } {
            Ok(_) => {
                // SAFETY: valid jmethodID from JNI in a single-threaded setup.
                let backup = unsafe { backup_of(layout, m as *mut c_void) };
                // SAFETY: single-threaded setup writes.
                unsafe {
                    store_backup(*label, backup);
                }
                crate::config::owen_log::info(&format!("owen: {label} hooked"));
            }
            Err(e) => {
                crate::config::owen_log::warn(&format!("owen: {label} hook failed: {e}"))
            }
        }
    }
}

fn install_kairos(env: *mut jni::JNIEnv, layout: &owen_art::ArtLayout) {
    let Some(limiter) =
        jni::find_class(env, b"com/epicgames/kairos/utils/DebouncedLimiter\0")
    else {
        crate::config::owen_log::info(
            "owen: Kairos DebouncedLimiter absent (not a 10.31-15.50 build) — WebView hooks remain active",
        );
        return;
    };
    let targets: &[(&[u8], &[u8], usize, &str)] = &[
        (
            b"loginUrl\0",
            b"(Lcom/epicgames/kairos/utils/Environment;Z)Ljava/lang/String;\0",
            kairos_loginurl as usize,
            "DebouncedLimiter.loginUrl",
        ),
        (
            b"getGraphqlUrl\0",
            b"(Lcom/epicgames/kairos/utils/Environment;)Ljava/lang/String;\0",
            kairos_graphql as usize,
            "DebouncedLimiter.getGraphqlUrl",
        ),
    ];
    for (name, sig, handler, label) in targets {
        let Some(m) = jni::get_static_method_id(env, limiter, name, sig) else {
            crate::config::owen_log::info(&format!("owen: Kairos method {label} not found"));
            continue;
        };
        // SAFETY: verified static jmethodID from JNI.
        if unsafe { owen_art::hook_method(env, layout, m as *mut c_void, *handler) }.is_ok() {
            crate::config::owen_log::info(&format!(
                "owen: {label} hooked — the smali-free PartyHub path"
            ));
        }
    }
}

fn install_cleartext_unlock(env: *mut jni::JNIEnv, layout: &owen_art::ArtLayout) {
    let Some(nsp) = jni::find_class(env, b"android/security/NetworkSecurityPolicy\0") else {
        return;
    };
    let Some(m) = jni::get_method_id(
        env,
        nsp,
        b"isCleartextTrafficPermitted\0",
        b"(Ljava/lang/String;)Z\0",
    ) else {
        return;
    };
    // SAFETY: verified jmethodID from JNI.
    if unsafe { owen_art::hook_method(env, layout, m as *mut c_void, cleartext_ok as usize) }
        .is_ok()
    {
        crate::config::owen_log::info(
            "owen: NetworkSecurityPolicy cleartext unlocked — http login pages work",
        );
    }
}

// --- backup storage ---------------------------------------------------------

static mut WEBVIEW_LOAD_URL_BACKUP: *mut c_void = core::ptr::null_mut();
static mut WEBVIEW_LOAD_URL2_BACKUP: *mut c_void = core::ptr::null_mut();

/// SAFETY: single-threaded setup only.
unsafe fn store_backup(label: &str, backup: *mut c_void) {
    unsafe {
        if label.contains("Map") {
            WEBVIEW_LOAD_URL2_BACKUP = backup;
        } else {
            WEBVIEW_LOAD_URL_BACKUP = backup;
        }
    }
}

/// Duplicate an ArtMethod into a fresh allocation so the original body stays
/// callable. The duplicate keeps the *pre-hook* entry points, i.e. the real
/// compiled Java code.
///
/// SAFETY: `method` must be a valid jmethodID.
unsafe fn backup_of(layout: &owen_art::ArtLayout, method: *mut c_void) -> *mut c_void {
    // ArtMethod is at most 0x40 bytes on every supported version; copy a
    // little extra padding which is never interpreted.
    const SIZE: usize = 0x48;
    let backup = owen_art::jni_alloc(SIZE);
    if backup.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::copy_nonoverlapping(method as *const u8, backup as *mut u8, SIZE);
        let flags =
            core::ptr::read_unaligned((backup as *const u8).add(layout.access) as *const u32);
        core::ptr::write_unaligned(
            (backup as *mut u8).add(layout.access) as *mut u32,
            flags & !0x8000_0000,
        );
    }
    backup
}

// ---------------------------------------------------------------------------
// Hook bodies (JNI calling convention)
// ---------------------------------------------------------------------------

fn current_rewrite(url: &str) -> Option<String> {
    let guard = CONFIG.read().ok()?;
    let cfg = guard.as_ref()?;
    rewrite::webview_rewrite(url, cfg)
}

/// `WebView.loadUrl(String)` -> rewrite and call the backup.
pub unsafe extern "C" fn webview_loadurl(
    env: *mut jni::JNIEnv,
    thiz: jni::jobject,
    url: jni::jstring,
) {
    unsafe {
        if let Some(url_str) = jni::string_to_rust(env, url) {
            let target = match current_rewrite(&url_str) {
                Some(u) => jni::new_string_utf(env, terminated(u.as_bytes()).as_slice()).unwrap_or(url),
                None => url,
            };
            let backup = WEBVIEW_LOAD_URL_BACKUP as jni::jmethodID;
            if !backup.is_null() {
                let mut args = [target as jni::jvalue];
                jni::call_void_method_v(env, thiz, backup, args.as_mut_ptr());
                return;
            }
        }
    }
}

/// `WebView.loadUrl(String, Map)` -> rewrite the string, keep the map.
pub unsafe extern "C" fn webview_loadurl2(
    env: *mut jni::JNIEnv,
    thiz: jni::jobject,
    url: jni::jstring,
    map: jni::jobject,
) {
    unsafe {
        if let Some(url_str) = jni::string_to_rust(env, url) {
            let target = match current_rewrite(&url_str) {
                Some(u) => jni::new_string_utf(env, terminated(u.as_bytes()).as_slice()).unwrap_or(url),
                None => url,
            };
            let backup = WEBVIEW_LOAD_URL2_BACKUP as jni::jmethodID;
            if !backup.is_null() {
                let mut args = [target as jni::jvalue, map as jni::jvalue];
                jni::call_void_method_v(env, thiz, backup, args.as_mut_ptr());
                return;
            }
        }
    }
}

/// `DebouncedLimiter.loginUrl(Environment, boolean)` -> our login URL.
pub unsafe extern "C" fn kairos_loginurl(
    env: *mut jni::JNIEnv,
    _clazz: jni::jobject,
    _environment: jni::jobject,
    _force: jni::jboolean,
) -> jni::jstring {
    unsafe {
        let url = login_url();
        jni::new_string_utf(env, terminated(url.as_bytes()).as_slice()).unwrap_or(core::ptr::null_mut())
    }
}

/// `DebouncedLimiter.getGraphqlUrl(Environment)` -> our graphql URL.
pub unsafe extern "C" fn kairos_graphql(
    env: *mut jni::JNIEnv,
    _clazz: jni::jobject,
    _environment: jni::jobject,
) -> jni::jstring {
    unsafe {
        let url = graphql_url();
        jni::new_string_utf(env, terminated(url.as_bytes()).as_slice()).unwrap_or(core::ptr::null_mut())
    }
}

/// `NetworkSecurityPolicy.isCleartextTrafficPermitted(String)` -> `true`.
pub unsafe extern "C" fn cleartext_ok(
    _env: *mut jni::JNIEnv,
    _thiz: jni::jobject,
    _host: jni::jstring,
) -> jni::jboolean {
    1
}

fn login_url() -> String {
    if let Ok(guard) = CONFIG.read() {
        if let Some(cfg) = guard.as_ref() {
            return cfg.login_url();
        }
    }
    "http://127.0.0.1:3551/login".to_string()
}

fn graphql_url() -> String {
    if let Ok(guard) = CONFIG.read() {
        if let Some(cfg) = guard.as_ref() {
            return cfg.graphql_url();
        }
    }
    "http://127.0.0.1:3551/partyhub/graphql".to_string()
}

fn terminated(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}
