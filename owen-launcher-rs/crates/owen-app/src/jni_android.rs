//! jni_android.rs — the Android JNI bridge of `owen-app` (the audited
//! boundary).
//!
//! Everything Android-specific that the app layer needs goes through this
//! single module, exactly like the original Kotlin app had exactly one
//! Activity + a handful of `Context` helpers. Provenance, piece by piece:
//!
//! | Rust (here)                                | Kotlin original |
//! |--------------------------------------------|------------------|
//! | [`perm_status`]                            | `MainActivity.kt`: `canInstallApks(context)` + `isIgnoringBatteryOptimizations(context)` |
//! | [`files_dir`] / [`cache_dir`]              | `Context.getFilesDir()` / `Context.getCacheDir()` |
//! | [`bundled_dir`] + `materialize_bundled`    | `LawinDataDeployer.kt` (the APK `assets/` root → filesystem trees; the Kotlin read them straight from `assets/`, here they are unpacked once into `cacheDir/bundled`) |
//! | [`native_library_dir`]                     | `NativeBuilder.buildOwen` uses `ApplicationInfo.nativeLibraryDir` |
//! | [`open_url`]                               | `MainActivity` share/log intents (`Intent(ACTION_VIEW, uri)`) |
//! | [`request_install_permission`]             | `MainActivity.openInstallPermissionSettings` (+ `ACTION_SECURITY_SETTINGS` fallback) |
//! | [`request_battery_exemption`]              | `MainActivity.openBatterySettings` (+ `ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS` fallback) |
//! | [`launch_game`]                            | `BuildDetailsScreen.kt`: `launchGame` + `fallbackLaunch` |
//! | [`uninstall_package`]                      | `OwenManager.uninstall` |
//! | [`installed_package`]                      | `OwenManager.isInstalled` / `getPackageInfo` (+ versionName/versionCode for the season list) |
//! | [`JniInstaller`]                           | `PackageInstallHelper.kt` (`object PackageInstallHelper`) |
//! | `native_on_receive` + the dex receiver     | `InstallResultReceiver.kt` |
//!
//! Safety model (same spirit as `owen-art`): this is the audited unsafe core
//! of the app crate — `lib.rs` has `#![cfg_attr(not(target_os = "android"),
//! forbid(unsafe_code))]`, so on every other target unsafe stays forbidden,
//! and on Android it is allowed *only here*. The unsafe surface is kept
//! deliberately minimal:
//!
//! * `unsafe { JavaVM::from_raw(..) }` in [`JNI_OnLoad`] (capturing the VM
//!   once — afterwards only the safe wrappers of the `jni` crate are used),
//! * the two `extern "C"` entry points themselves ([`JNI_OnLoad`] and
//!   `native_on_receive`). `RegisterNatives` goes through the safe
//!   `JNIEnv::register_native_methods`.
//!
//! Every public function degrades gracefully: Java exceptions are cleared
//! and reported through `owen_launcher::core::diag`, never panicked on.

use jni::{
    objects::{GlobalRef, JClass, JObject, JString, JValue, JValueGen},
    sys::{jboolean, jint, jlong},
    JNIEnv, JavaVM,
};
use std::io::Write;
use owen_launcher::{
    core::diag,
    install::{
        helper::{
            InstallResult, InstallSession, Installer, RESULT_ACTION, WAIT_TIMEOUT_MS,
        },
        result_receiver::{
            InstallBroadcast, InstallResultBus, UserAction, STATUS_PENDING_USER_ACTION,
        },
    },
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

/* --------------------------- module constants ---------------------------- */

/// `Build.VERSION_CODES.S` (Android 12) — `FLAG_MUTABLE` is required from
/// SDK 31 on (and must NOT be used below it).
const SDK_S: jint = 31;

/// `Intent.FLAG_ACTIVITY_NEW_TASK`.
const FLAG_ACTIVITY_NEW_TASK: jint = 0x1000_0000;

/// `PendingIntent.FLAG_UPDATE_CURRENT`.
const PENDING_FLAG_UPDATE_CURRENT: jint = 0x10;

/// `PendingIntent.FLAG_MUTABLE`.
const PENDING_FLAG_MUTABLE: jint = 0x0200_0000;

/// `PackageInstaller.SessionParams.MODE_FULL_INSTALL`.
const MODE_FULL_INSTALL: jint = 1;

/// `Intent.ACTION_VIEW`.
const ACTION_VIEW: &str = "android.intent.action.VIEW";

/// `Intent.ACTION_DELETE`.
const ACTION_DELETE: &str = "android.intent.action.DELETE";

/// `Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES`.
const ACTION_MANAGE_UNKNOWN_APP_SOURCES: &str = "android.settings.MANAGE_UNKNOWN_APP_SOURCES";

/// `Settings.ACTION_SECURITY_SETTINGS` (the fallback of the above).
const ACTION_SECURITY_SETTINGS: &str = "android.settings.SECURITY_SETTINGS";

/// `Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`.
const ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS: &str =
    "android.settings.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS";

/// `Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS` (the fallback).
const ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS: &str =
    "android.settings.IGNORE_BATTERY_OPTIMIZATION_SETTINGS";

/// `PackageInstaller.EXTRA_SESSION_ID` — `"android.content.pm.extra.SESSION_ID"`
/// (same literal the Kotlin receiver reads).
const EXTRA_SESSION_ID: &str = "android.content.pm.extra.SESSION_ID";

/// `PackageInstaller.EXTRA_STATUS` — `"android.content.pm.extra.STATUS"`.
const EXTRA_STATUS: &str = "android.content.pm.extra.STATUS";

/// `PackageInstaller.EXTRA_STATUS_MESSAGE` — `"android.content.pm.extra.STATUS_MESSAGE"`.
const EXTRA_STATUS_MESSAGE: &str = "android.content.pm.extra.STATUS_MESSAGE";

/// `Intent.EXTRA_INTENT` — `"android.intent.extra.INTENT"` (the
/// pending-user-action dialog intent).
const INTENT_EXTRA_INTENT: &str = "android.intent.extra.INTENT";

/// The receiver dex class (`com.abdallhrgx.owenlauncher.core.patch.InstallResultReceiver`
/// in JNI form).
const RECEIVER_CLASS: &str = "com/abdallhrgx/owenlauncher/core/patch/InstallResultReceiver";

/// The smali `<clinit>` loads the app library under this name
/// (`System.loadLibrary("owen_app")` — cargo's default cdylib name of the
/// `owen-app` package is `libowen_app.so`). Kept as the documented contract
/// with the handcrafted dex (`tools/dex/make_dex.py` emits the same literal).
#[allow(dead_code)] // referenced by the dex emitter spec, not by code
const LIB_NAME: &str = "owen_app";

/// The APK asset trees materialized into `cacheDir/bundled` on first launch.
/// `lawin`/`owen`/`images`/`tools` mirror `LawinDataDeployer.DEPLOY_ROOTS`
/// (the trees the launcher actually uses); `signing` carries the original
/// ogfn key pair (`ui.rs` looks for `bundled/signing/ogfn-key.pem`).
const BUNDLED_TREES: &[&str] = &["lawin", "owen", "images", "tools", "signing"];

/// The APK asset files materialized at the bundled root.
const BUNDLED_FILES: &[&str] = &["manifest.txt"];

/// Marker file: written once the asset copy completed, checked to skip it.
const BUNDLED_DONE: &str = ".done";

/// The tag used for all diagnostics of this module.
const TAG: &str = "jni";

/* --------------------------------- state ---------------------------------- */

/// The JavaVM captured in `JNI_OnLoad` (`System.loadLibrary("owen_app")`).
static VM: OnceLock<JavaVM> = OnceLock::new();

/// The `Application` object (a `Context`) obtained through the standard
/// `ActivityThread.currentActivityThread().getApplication()` trick — the only
/// Context a NativeActivity app with no Java activity code can get. Kept as a
/// global reference for the lifetime of the process.
static CONTEXT: OnceLock<GlobalRef> = OnceLock::new();

/// `PackageInstallHelper.waiting: ConcurrentHashMap<Int, (InstallResult) -> Unit>`
/// — session id → oneshot sender, completed by `nativeOnReceive` / `on_result`.
/// (`HashMap::new` is not const — init once on first use, see [`waiting`].)
static WAITING: OnceLock<Mutex<HashMap<i32, mpsc::Sender<InstallResult>>>> = OnceLock::new();

/// The [`WAITING`] map (the `OnceLock` companion of the Kotlin companion's
/// `ConcurrentHashMap`).
fn waiting() -> &'static Mutex<HashMap<i32, mpsc::Sender<InstallResult>>> {
    WAITING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The `InstallResultReceiver` bus (the Rust twin of the Kotlin receiver's
/// companion routing). Fed by `nativeOnReceive`; `JniInstaller::on_result`
/// is the `PackageInstallHelper.onResult` it calls back into.
static BUS: OnceLock<Mutex<InstallResultBus>> = OnceLock::new();

fn bus() -> &'static Mutex<InstallResultBus> {
    BUS.get_or_init(|| Mutex::new(InstallResultBus::new()))
}

/* ------------------------------ JNI plumbing ------------------------------- */

/// The error type of the JNI closures: either a `jni`-crate failure or one of
/// this module's human-readable messages (the private helpers report
/// `String`). One type both convert into keeps `?` working on both kinds of
/// failure; [`jni_run`] flattens it back to `String` for its callers.
#[derive(Debug)]
enum JniErr {
    /// A failure reported by the `jni` crate itself.
    Jni(jni::errors::Error),
    /// One of this module's messages (the helpers' `String` errors).
    Msg(String),
}

impl From<jni::errors::Error> for JniErr {
    fn from(e: jni::errors::Error) -> Self {
        JniErr::Jni(e)
    }
}

impl From<String> for JniErr {
    fn from(m: String) -> Self {
        JniErr::Msg(m)
    }
}

impl From<&str> for JniErr {
    fn from(m: &str) -> Self {
        JniErr::Msg(m.to_owned())
    }
}

impl std::fmt::Display for JniErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JniErr::Jni(e) => write!(f, "{e:?}"),
            JniErr::Msg(m) => write!(f, "{m}"),
        }
    }
}

/// Run `f` with an attached-thread `JNIEnv` (`vm.attach_current_thread()` —
/// a no-op re-attach on threads that are already attached, e.g. the broadcast
/// or NativeActivity threads).
fn with_env<T>(f: impl FnOnce(&mut JNIEnv) -> Result<T, JniErr>) -> Result<T, JniErr> {
    let vm = VM
        .get()
        .ok_or_else(|| JniErr::Msg("JavaVM not captured (JNI_OnLoad has not run)".into()))?;
    let mut guard = vm.attach_current_thread()?;
    f(&mut guard)
}

/// String-error flavor of [`with_env`] for the public helpers.
fn jni_run<T>(f: impl FnOnce(&mut JNIEnv) -> Result<T, JniErr>) -> Result<T, String> {
    with_env(f).map_err(|e| e.to_string())
}

/// If a Java exception is pending, clear it and return `true` (the Java
/// `catch`/`runCatching` equivalent of the original code).
fn exception_pending(env: &mut JNIEnv<'_>) -> bool {
    match env.exception_check() {
        Ok(true) => {
            let _ = env.exception_clear();
            true
        }
        _ => false,
    }
}

/// The `Application` Context as a global reference (created on first use via
/// `ActivityThread.currentActivityThread().getApplication()`).
fn context_global() -> Result<GlobalRef, String> {
    if let Some(g) = CONTEXT.get() {
        return Ok(g.clone());
    }
    let g = jni_run(|env| {
        let thread_class = env.find_class("android/app/ActivityThread")?;
        let current = env.call_static_method(
            &thread_class,
            "currentActivityThread",
            "()Landroid/app/ActivityThread;",
            &[],
        )?;
        let thread = JObject::try_from(current)?;
        let app = env.call_method(&thread, "getApplication", "()Landroid/app/Application;", &[])?;
        let app_obj = JObject::try_from(app)?;
        if app_obj.is_null() {
            return Err(JniErr::Jni(jni::errors::Error::JniCall(
                jni::errors::JniError::InvalidArguments,
            )));
        }
        env.new_global_ref(&app_obj).map_err(JniErr::from)
    })?;
    let _ = CONTEXT.set(g.clone());
    Ok(g)
}

/// `Build.VERSION.SDK_INT`.
fn sdk_int(env: &mut JNIEnv<'_>) -> jni::errors::Result<jint> {
    let version = env.find_class("android/os/Build$VERSION")?;
    let v = env.get_static_field(&version, "SDK_INT", "I")?;
    Ok(jint::try_from(v)?)
}

/// Read a Java string held by `obj` (a `java.lang.String` local).
fn rust_string(env: &mut JNIEnv<'_>, obj: &JObject<'_>) -> Option<String> {
    if obj.is_null() {
        return None;
    }
    let js = unsafe { JString::from_raw(obj.as_raw()) };
    // SAFETY: `obj` is a live local reference of the right type (callers only
    // pass results of String-returning calls); the wrapper borrows the same
    // raw handle (jni 0.21 wrappers never delete local refs on drop).
    let s = env.get_string(&js).ok()?;
    Some(s.to_string_lossy().into_owned())
}

/// `Intent.getIntExtra(key, def)`.
fn int_extra(env: &mut JNIEnv<'_>, intent: &JObject<'_>, key: &str, def: jint) -> jint {
    let key = match env.new_string(key) {
        Ok(k) => k,
        Err(_) => return def,
    };
    match env.call_method(
        intent,
        "getIntExtra",
        "(Ljava/lang/String;I)I",
        &[(&key).into(), JValue::from(def)],
    ) {
        Ok(v) => jint::try_from(v).unwrap_or(def),
        Err(_) => {
            exception_pending(env);
            def
        }
    }
}

/// `Intent.getStringExtra(key)` — `None` when absent.
fn string_extra(env: &mut JNIEnv<'_>, intent: &JObject<'_>, key: &str) -> Option<String> {
    let key = env.new_string(key).ok()?;
    let v = match env.call_method(
        intent,
        "getStringExtra",
        "(Ljava/lang/String;)Ljava/lang/String;",
        &[(&key).into()],
    ) {
        Ok(v) => v,
        Err(_) => {
            exception_pending(env);
            return None;
        }
    };
    if exception_pending(env) {
        return None;
    }
    let o = JObject::try_from(v).ok()?;
    rust_string(env, &o)
}

/// `Intent.hasExtra(key)`.
fn has_extra(env: &mut JNIEnv<'_>, intent: &JObject<'_>, key: &str) -> bool {
    let key = match env.new_string(key) {
        Ok(k) => k,
        Err(_) => return false,
    };
    match env.call_method(intent, "hasExtra", "(Ljava/lang/String;)Z", &[(&key).into()]) {
        Ok(v) => matches!(v, JValueGen::Bool(b) if b != 0),
        Err(_) => {
            exception_pending(env);
            false
        }
    }
}

/// `Intent.getParcelableExtra(Intent.EXTRA_INTENT)` — typed (API 33+) first,
/// then the deprecated untyped form.
fn parcelable_extra<'local>(
    env: &mut JNIEnv<'local>,
    intent: &JObject<'_>,
    key: &str,
) -> Option<JObject<'local>> {
    let key = env.new_string(key).ok()?;
    // Typed: getParcelableExtra(String, Class<T>) — API 33+.
    let intent_class = env.find_class("android/content/Intent").ok()?;
    let typed = env.call_method(
        intent,
        "getParcelableExtra",
        "(Ljava/lang/String;Ljava/lang/Class;)Ljava/lang/Object;",
        &[(&key).into(), (&intent_class).into()],
    );
    if !exception_pending(env) {
        if let Ok(v) = typed {
            let o = JObject::try_from(v).ok()?;
            if !o.is_null() {
                return Some(o);
            }
        }
    }
    // Deprecated untyped: getParcelableExtra(String) — API 1..34+.
    let legacy = env.call_method(
        intent,
        "getParcelableExtra",
        "(Ljava/lang/String;)Landroid/os/Parcelable;",
        &[(&key).into()],
    );
    if exception_pending(env) {
        return None;
    }
    match legacy {
        Ok(v) => {
            let o = JObject::try_from(v).ok()?;
            if o.is_null() {
                None
            } else {
                Some(o)
            }
        }
        Err(_) => None,
    }
}

/// `Context.startActivity(intent)` — `Err(msg)` if it threw (cleared).
fn start_activity(env: &mut JNIEnv<'_>, intent: &JObject<'_>) -> Result<(), String> {
    let ctx = context_global()?;
    let ctx_obj = ctx.as_obj();
    env.call_method(ctx_obj, "startActivity", "(Landroid/content/Intent;)V", &[intent.into()])
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))?;
    if exception_pending(env) {
        return Err("startActivity failed".into());
    }
    Ok(())
}

/// `Context.getPackageManager()`.
fn package_manager<'local>(env: &mut JNIEnv<'local>) -> Result<JObject<'local>, String> {
    let ctx = context_global()?;
    let ctx_obj = ctx.as_obj();
    let v = env
        .call_method(ctx_obj, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])
        .map_err(|e| format!("{e:?}"))?;
    if exception_pending(env) {
        return Err("getPackageManager failed".into());
    }
    Ok(JObject::try_from(v).map_err(|e| format!("{e:?}"))?)
}

/// `Context.getPackageName()`.
fn package_name(env: &mut JNIEnv<'_>) -> Result<String, String> {
    let ctx = context_global()?;
    let v = env
        .call_method(ctx.as_obj(), "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("{e:?}"))?;
    if exception_pending(env) {
        return Err("getPackageName failed".into());
    }
    let o = JObject::try_from(v).map_err(|e| format!("{e:?}"))?;
    rust_string(env, &o).ok_or_else(|| "null package name".into())
}

/* ---------------------------- entry points (unsafe) ----------------------- */

/// `JNI_OnLoad` — called by `System.loadLibrary("owen_app")` (the receiver's
/// `<clinit>`, or NativeActivity's loader). Captures the JavaVM once and
/// registers the single native method of the install-result receiver.
///
/// This is one of the two unsafe spots of the module: `JavaVM::from_raw` on
/// the pointer Android hands us.
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: *mut std::ffi::c_void, _reserved: *mut std::ffi::c_void) -> jint {
    const JNI_VERSION_1_6: jint = 0x0001_0006;
    let raw = vm as *mut jni::sys::JavaVM;
    // SAFETY: Android guarantees `vm` is a valid JavaVM for the lifetime of
    // the library; we store it once and never call from_raw again.
    let java_vm = match unsafe { JavaVM::from_raw(raw) } {
        Ok(v) => v,
        Err(e) => {
            diag::e(TAG, &format!("JNI_OnLoad: cannot wrap JavaVM: {e:?}"));
            return JNI_VERSION_1_6;
        }
    };
    let _ = VM.set(java_vm);
    // RegisterNatives for the receiver class. This runs while the dex class
    // that called loadLibrary is initializing — its loader resolves the class.
    let registered = jni_run(|env| {
        let class = env.find_class(RECEIVER_CLASS)?;
        // SAFETY-free: register_native_methods is the checked wrapper.
        env.register_native_methods(
            &class,
            &[jni::NativeMethod {
                name: "nativeOnReceive".into(),
                sig: "(Landroid/content/Intent;)V".into(),
                fn_ptr: native_on_receive as *mut std::ffi::c_void,
            }],
        )
        .map_err(JniErr::from)
    });
    match registered {
        Ok(()) => diag::i(TAG, "JNI_OnLoad: JavaVM captured, natives registered"),
        Err(e) => diag::e(
            TAG,
            &format!("JNI_OnLoad: could not register the receiver natives: {e}"),
        ),
    }
    JNI_VERSION_1_6
}

/// The native side of `InstallResultReceiver.onReceive` — the smali class
/// forwards the broadcast Intent here. Second (and last) unsafe spot: the
/// `extern "C"` ABI of the registered function itself.
unsafe extern "C" fn native_on_receive(
    _env: JNIEnv,
    _class: JClass,
    intent: JObject,
) {
    native_on_receive_impl(intent);
}

/* ------------------------- the receiver routing (safe) -------------------- */

/// `InstallResultReceiver.onReceive(context, intent)` — 1:1 with the Kotlin,
/// routed through `owen_launcher::install::result_receiver::InstallResultBus`
/// (`on_receive` / `report_user_action_failed`).
fn native_on_receive_impl(intent: JObject) {
    if intent.is_null() {
        return;
    }
    // Read the extras into the plain-data InstallBroadcast (plus the
    // pending-dialog outcome that needs native handling).
    enum Pending {
        Absent,
        Missing,
        Opened(String),
        Failed(String),
    }
    let parsed: Result<(i32, Option<i32>, String, Pending), String> = jni_run(|env| {
        let session_id = int_extra(env, &intent, EXTRA_SESSION_ID, -1);
        if !has_extra(env, &intent, EXTRA_STATUS) {
            // "result arrived without status extras" — the bus logs + ignores.
            return Ok((session_id, None, String::new(), Pending::Absent));
        }
        // PackageInstaller.STATUS_FAILURE is the Kotlin's getIntExtra default.
        let status = int_extra(env, &intent, EXTRA_STATUS, 1);
        let message = string_extra(env, &intent, EXTRA_STATUS_MESSAGE).unwrap_or_default();
        if status == STATUS_PENDING_USER_ACTION {
            return match parcelable_extra(env, &intent, INTENT_EXTRA_INTENT) {
                None => Ok((session_id, Some(status), message, Pending::Missing)),
                Some(pending) => {
                    // UserAction record: the opaque intent URI (toUri(0)).
                    let uri = {
                        let v = env.call_method(&pending, "toUri", "(I)Ljava/lang/String;", &[JValue::from(0i32)]);
                        match v {
                            Ok(v) => match JObject::try_from(v) {
                                Ok(o) => rust_string(env, &o).unwrap_or_default(),
                                Err(_) => String::new(),
                            },
                            Err(_) => {
                                exception_pending(env);
                                String::new()
                            }
                        }
                    };
                    // pending.addFlags(FLAG_ACTIVITY_NEW_TASK); context.startActivity(pending)
                    let _ = env.call_method(
                        &pending,
                        "addFlags",
                        "(I)Landroid/content/Intent;",
                        &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
                    );
                    match start_activity(env, &pending) {
                        Ok(()) => Ok((session_id, Some(status), message, Pending::Opened(uri))),
                        Err(t) => Ok((session_id, Some(status), message, Pending::Failed(t))),
                    }
                }
            };
        }
        Ok((session_id, Some(status), message, Pending::Absent))
    });
    let (session_id, status, message, pending) = match parsed {
        Ok(v) => v,
        Err(e) => {
            diag::e(TAG, &format!("nativeOnReceive: {e}"));
            return;
        }
    };

    // Route through the bus exactly like the Kotlin receiver routed through
    // PackageInstallHelper. A throwaway JniInstaller stands in for
    // `PackageInstallHelper` (its on_result completes the waiting session).
    let mut installer = JniInstaller::new();
    let mut bus = bus().lock().unwrap_or_else(|p| p.into_inner());
    match pending {
        Pending::Absent => {
            bus.on_receive(
                &mut installer,
                &InstallBroadcast {
                    session_id,
                    status,
                    status_message: Some(message.clone()),
                    user_action: None,
                },
            );
        }
        Pending::Missing => {
            bus.on_receive(
                &mut installer,
                &InstallBroadcast {
                    session_id,
                    status: Some(STATUS_PENDING_USER_ACTION),
                    status_message: Some(message.clone()),
                    user_action: None,
                },
            );
        }
        Pending::Opened(uri) => {
            bus.on_receive(
                &mut installer,
                &InstallBroadcast {
                    session_id,
                    status: Some(STATUS_PENDING_USER_ACTION),
                    status_message: Some(message.clone()),
                    user_action: Some(UserAction { intent_uri: uri }),
                },
            );
            diag::i(TAG, &format!("session {session_id}: opened the system install confirmation"));
        }
        Pending::Failed(err) => {
            bus.report_user_action_failed(&mut installer, session_id, &err);
        }
    }
}

/* ------------------------- public item 1: perm_status --------------------- */

/// `MainActivity.canInstallApks` + `MainActivity.isIgnoringBatteryOptimizations`
/// — `(canRequestPackageInstalls, isIgnoreBatteryOptimizations(packageName))`.
/// `canRequestPackageInstalls` is API 26+ and *throws* below that: caught and
/// reported as granted, exactly like the Kotlin `if (SDK_INT >= O) … else true`.
pub fn perm_status() -> (bool, bool) {
    let install = jni_run(|env| {
        let pm = package_manager(env)?;
        let v = env.call_method(&pm, "canRequestPackageInstalls", "()Z", &[]);
        if exception_pending(env) {
            // API < 26 (NoSuchMethod) or any other failure: treat as granted.
            return Ok(true);
        }
        match v {
            Ok(v) => Ok(jboolean::try_from(v).map(|b| b != 0).unwrap_or(true)),
            Err(_) => Ok(true),
        }
    })
    .unwrap_or(true);

    let battery = jni_run(|env| {
        let ctx = context_global()?;
        let power_service = env.new_string("power")?;
        let power = env.call_method(
            ctx.as_obj(),
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[(&power_service).into()],
        )?;
        if exception_pending(env) {
            return Ok(false); // runCatching { … }.getOrDefault(false)
        }
        let power_obj = JObject::try_from(power)?;
        if power_obj.is_null() {
            return Ok(false);
        }
        let pkg = package_name(env)?;
        let pkg_js = env.new_string(&pkg)?;
        let v = env.call_method(
            &power_obj,
            "isIgnoringBatteryOptimizations",
            "(Ljava/lang/String;)Z",
            &[(&pkg_js).into()],
        )?;
        if exception_pending(env) {
            return Ok(false);
        }
        Ok(jboolean::try_from(v).map(|b| b != 0).unwrap_or(false))
    })
    .unwrap_or(false);

    (install, battery)
}

/* ------------------- public items 2-4: the app directories ---------------- */

/// `Context.getFilesDir()`.
pub fn files_dir() -> PathBuf {
    let p = jni_run(|env| {
        let ctx = context_global()?;
        let file = env.call_method(ctx.as_obj(), "getFilesDir", "()Ljava/io/File;", &[])?;
        if exception_pending(env) {
            return Err("getFilesDir failed".into());
        }
        let file = JObject::try_from(file)?;
        let v = env.call_method(&file, "getPath", "()Ljava/lang/String;", &[])?;
        let o = JObject::try_from(v)?;
        rust_string(env, &o).ok_or_else(|| "null files dir".into())
    });
    match p {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            diag::e(TAG, &format!("files_dir: {e}"));
            PathBuf::from(".")
        }
    }
}

/// `Context.getCacheDir()`.
pub fn cache_dir() -> PathBuf {
    let p = jni_run(|env| {
        let ctx = context_global()?;
        let file = env.call_method(ctx.as_obj(), "getCacheDir", "()Ljava/io/File;", &[])?;
        if exception_pending(env) {
            return Err("getCacheDir failed".into());
        }
        let file = JObject::try_from(file)?;
        let v = env.call_method(&file, "getPath", "()Ljava/lang/String;", &[])?;
        let o = JObject::try_from(v)?;
        rust_string(env, &o).ok_or_else(|| "null cache dir".into())
    });
    match p {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            diag::e(TAG, &format!("cache_dir: {e}"));
            PathBuf::from(".")
        }
    }
}

/// `cacheDir/bundled` — materialized from the APK assets on first call
/// (guarded by `bundled/.done`). The trees/files mirrored are
/// [`BUNDLED_TREES`] + [`BUNDLED_FILES`]; anything not present in the APK is
/// skipped with a diagnostic (the safe core's deployer then creates empty
/// folders, like the original "Asset folder not bundled" branch).
pub fn bundled_dir() -> PathBuf {
    let dir = cache_dir().join("bundled");
    if dir.join(BUNDLED_DONE).is_file() {
        return dir;
    }
    let _ = std::fs::create_dir_all(&dir);
    let summary = jni_run(|env| {
        let ctx = context_global()?;
        let assets = env.call_method(ctx.as_obj(), "getAssets", "()Landroid/content/res/AssetManager;", &[])?;
        if exception_pending(env) {
            return Err("getAssets failed".into());
        }
        let assets = JObject::try_from(assets)?;
        let mut counts: Vec<String> = Vec::new();
        for tree in BUNDLED_TREES {
            let target = dir.join(tree);
            let _ = std::fs::create_dir_all(&target);
            let n = copy_asset_tree(env, &assets, tree, &target);
            counts.push(format!("{tree}={n}"));
        }
        for file in BUNDLED_FILES {
            let n = copy_asset_file(env, &assets, file, &dir.join(file))
                .map(|_| 1usize)
                .unwrap_or(0);
            if n == 0 {
                diag::i(TAG, &format!("bundled asset not present: {file}"));
            }
            counts.push(format!("{file}={n}"));
        }
        Ok(counts.join(" "))
    });
    match summary {
        Ok(counts) => {
            diag::i(TAG, &format!("bundled assets materialized: {counts}"));
            if std::fs::write(dir.join(BUNDLED_DONE), b"ok\n").is_err() {
                diag::e(TAG, "cannot write the bundled/.done marker");
            }
        }
        Err(e) => diag::e(TAG, &format!("bundled asset materialization failed: {e}")),
    }
    dir
}

/// Recursively copy one APK asset tree; returns the number of files copied.
/// `AssetManager.list` returns the child names (empty array ⇒ the entry is a
/// file, or the tree is not bundled — same ambiguity the original had).
fn copy_asset_tree(env: &mut JNIEnv<'_>, assets: &JObject<'_>, asset_path: &str, fs_path: &Path) -> usize {
    let children: Vec<String> = {
        let key = match env.new_string(asset_path) {
            Ok(k) => k,
            Err(_) => return 0,
        };
        let v = match env.call_method(
            assets,
            "list",
            "(Ljava/lang/String;)[Ljava/lang/String;",
            &[(&key).into()],
        ) {
            Ok(v) => v,
            Err(_) => {
                exception_pending(env);
                diag::i(TAG, &format!("asset tree not bundled: {asset_path}"));
                return 0;
            }
        };
        if exception_pending(env) {
            return 0;
        }
        let arr = match JObject::try_from(v) {
            Ok(o) => o,
            Err(_) => return 0,
        };
        if arr.is_null() {
            return 0;
        }
        // java.lang.reflect.Array keeps everything as JObject (no typed
        // array wrappers needed — see the safety note at the top).
        let array_class = match env.find_class("java/lang/reflect/Array") {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let len = match env.call_static_method(
            &array_class,
            "getLength",
            "(Ljava/lang/Object;)I",
            &[(&arr).into()],
        ) {
            Ok(v) => jint::try_from(v).unwrap_or(0),
            Err(_) => {
                exception_pending(env);
                return 0;
            }
        };
        if exception_pending(env) || len < 0 {
            return 0;
        }
        let mut names = Vec::with_capacity(len as usize);
        for i in 0..len {
            let elem = match env.call_static_method(
                &array_class,
                "get",
                "(Ljava/lang/Object;I)Ljava/lang/Object;",
                &[(&arr).into(), JValue::from(i)],
            ) {
                Ok(v) => match JObject::try_from(v) {
                    Ok(o) => o,
                    Err(_) => continue,
                },
                Err(_) => {
                    exception_pending(env);
                    continue;
                }
            };
            if let Some(s) = rust_string(env, &elem) {
                names.push(s);
            }
        }
        names
    };
    if children.is_empty() {
        // A file (or a tree missing from the APK).
        return match copy_asset_file(env, assets, asset_path, fs_path) {
            Ok(_) => 1,
            Err(_) => 0,
        };
    }
    let mut total = 0;
    for child in children {
        let child_asset = format!("{asset_path}/{child}");
        total += copy_asset_tree(env, assets, &child_asset, &fs_path.join(&child));
    }
    total
}

/// `AssetManager.open(path)` + stream copy into `fs_path` (64 KiB chunks).
fn copy_asset_file(
    env: &mut JNIEnv<'_>,
    assets: &JObject<'_>,
    asset_path: &str,
    fs_path: &Path,
) -> Result<(), String> {
    let key = env
        .new_string(asset_path)
        .map_err(|e| format!("{e:?}"))?;
    let stream = env
        .call_method(
            assets,
            "open",
            "(Ljava/lang/String;)Ljava/io/InputStream;",
            &[(&key).into()],
        )
        .map_err(|e| format!("open({asset_path}): {e:?}"))?;
    if exception_pending(env) {
        return Err(format!("open({asset_path}) threw (not bundled?)"));
    }
    let stream = JObject::try_from(stream).map_err(|e| format!("{e:?}"))?;
    if stream.is_null() {
        return Err(format!("open({asset_path}) returned null"));
    }
    let mut out = std::fs::File::create(fs_path)
        .map_err(|e| format!("create {}: {e}", fs_path.display()))?;

    const CHUNK: usize = 65536;
    let buf = env
        .new_byte_array(CHUNK as jint)
        .map_err(|e| format!("new_byte_array: {e:?}"))?;
    let mut jtmp: Vec<jbyte_t> = vec![0; CHUNK]; // jbyte = i8
    loop {
        let n = match env.call_method(&stream, "read", "([B)I", &[(&buf).into()]) {
            Ok(v) => jint::try_from(v).unwrap_or(-1),
            Err(_) => {
                exception_pending(env);
                return Err(format!("read({asset_path}) threw"));
            }
        };
        if exception_pending(env) {
            return Err(format!("read({asset_path}) threw"));
        }
        if n <= 0 {
            break;
        }
        let n = n as usize;
        if n > CHUNK {
            return Err("read overflow".into());
        }
        env.get_byte_array_region(&buf, 0, &mut jtmp[..n])
            .map_err(|e| format!("get_byte_array_region: {e:?}"))?;
        let bytes: Vec<u8> = jtmp[..n].iter().map(|&b| b as u8).collect();
        out.write_all(&bytes)
            .map_err(|e| format!("write {}: {e}", fs_path.display()))?;
    }
    env.call_method(&stream, "close", "()V", &[])
        .map_err(|e| format!("close: {e:?}"))?;
    if exception_pending(env) {
        return Err("close threw".into());
    }
    Ok(())
}

/// `jni::sys::jbyte` spelled out (an `i8`); keeps `get_byte_array_region`
/// buffers honest without importing the alias everywhere.
type jbyte_t = i8;

/* ------------------- public item 5: native_library_dir --------------------- */

/// `ApplicationInfo.nativeLibraryDir` (via `PackageManager.getApplicationInfo`)
/// — where `libOwen.so` lives inside the APK.
pub fn native_library_dir() -> PathBuf {
    let p = jni_run(|env| {
        let pm = package_manager(env)?;
        let pkg = package_name(env)?;
        let pkg_js = env.new_string(&pkg)?;
        let ai = env.call_method(
            &pm,
            "getApplicationInfo",
            "(Ljava/lang/String;I)Landroid/content/ApplicationInfo;",
            &[(&pkg_js).into(), JValue::from(0i32)],
        )?;
        if exception_pending(env) {
            return Err("getApplicationInfo failed".into());
        }
        let ai = JObject::try_from(ai)?;
        let v = env.get_field(&ai, "nativeLibraryDir", "Ljava/lang/String;")?;
        let o = JObject::try_from(v)?;
        rust_string(env, &o).ok_or_else(|| "null nativeLibraryDir".into())
    });
    match p {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            diag::e(TAG, &format!("native_library_dir: {e}"));
            PathBuf::new()
        }
    }
}

/* --------------------- public items 6-8: the OS intents -------------------- */

/// `Intent(ACTION_VIEW, url) + FLAG_ACTIVITY_NEW_TASK` + `startActivity`.
pub fn open_url(url: &str) {
    let _ = jni_run::<()>(|env| {
        let uri_class = env.find_class("android/net/Uri")?;
        let url_js = env.new_string(url)?;
        let uri = env.call_static_method(
            &uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[(&url_js).into()],
        )?;
        if exception_pending(env) {
            return Err("Uri.parse failed".into());
        }
        let uri = JObject::try_from(uri)?;
        let action = env.new_string(ACTION_VIEW)?;
        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(
            &intent_class,
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[(&action).into(), (&uri).into()],
        )?;
        if exception_pending(env) {
            return Err("Intent constructor failed".into());
        }
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        start_activity(env, &intent).map_err(JniErr::from)
    });
}

/// `MainActivity.openInstallPermissionSettings`:
/// `Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES` + `package:<packageName>` URI,
/// falling back to `Settings.ACTION_SECURITY_SETTINGS`.
pub fn request_install_permission() {
    let _ = jni_run::<()>(|env| {
        let pkg = package_name(env)?;
        let uri = make_uri(env, &format!("package:{pkg}"))?;
        let action = env.new_string(ACTION_MANAGE_UNKNOWN_APP_SOURCES)?;
        let intent = new_action_intent(env, &action)?;
        env.call_method(&intent, "setData", "(Landroid/net/Uri;)Landroid/content/Intent;", &[(&uri).into()])?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        if start_activity(env, &intent).is_ok() {
            return Ok(());
        }
        // Fallback — plain settings screen.
        let action = env.new_string(ACTION_SECURITY_SETTINGS)?;
        let intent = new_action_intent(env, &action)?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        start_activity(env, &intent).map_err(JniErr::from)
    });
}

/// `MainActivity.openBatterySettings`:
/// `Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` + `package:<packageName>`
/// URI, falling back to `Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS`.
pub fn request_battery_exemption() {
    let _ = jni_run::<()>(|env| {
        let pkg = package_name(env)?;
        let uri = make_uri(env, &format!("package:{pkg}"))?;
        let action = env.new_string(ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS)?;
        let intent = new_action_intent(env, &action)?;
        env.call_method(&intent, "setData", "(Landroid/net/Uri;)Landroid/content/Intent;", &[(&uri).into()])?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        if start_activity(env, &intent).is_ok() {
            return Ok(());
        }
        let action = env.new_string(ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)?;
        let intent = new_action_intent(env, &action)?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        start_activity(env, &intent).map_err(JniErr::from)
    });
}

/// `Uri.parse(s)` → `android.net.Uri` object.
fn make_uri<'local>(env: &mut JNIEnv<'local>, s: &str) -> Result<JObject<'local>, String> {
    let uri_class = env.find_class("android/net/Uri").map_err(|e| format!("{e:?}"))?;
    let js = env.new_string(s).map_err(|e| format!("{e:?}"))?;
    let v = env
        .call_static_method(
            &uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[(&js).into()],
        )
        .map_err(|e| format!("{e:?}"))?;
    if exception_pending(env) {
        return Err("Uri.parse failed".into());
    }
    Ok(JObject::try_from(v).map_err(|e| format!("{e:?}"))?)
}

/// `Intent(action)` — the no-data constructor.
fn new_action_intent<'local>(
    env: &mut JNIEnv<'local>,
    action: &JString<'_>,
) -> Result<JObject<'local>, String> {
    let intent_class = env.find_class("android/content/Intent").map_err(|e| format!("{e:?}"))?;
    let intent = env
        .new_object(&intent_class, "(Ljava/lang/String;)V", &[action.into()])
        .map_err(|e| format!("{e:?}"))?;
    if exception_pending(env) {
        return Err("Intent(action) failed".into());
    }
    Ok(intent)
}

/* -------------------- public items 9-11: game interactions ----------------- */

/// `BuildDetailsScreen.launchGame` + `fallbackLaunch`:
/// deep-link `Intent(ACTION_VIEW, code).setPackage(pkg)` (NEW_TASK), falling
/// back to `getLaunchIntentForPackage(pkg)`. Returns whether something launched.
pub fn launch_game(pkg: &str, code: &str) -> bool {
    jni_run(|env| {
        // 1) the deep link
        let uri = make_uri(env, code)?;
        let action = env.new_string(ACTION_VIEW)?;
        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(
            &intent_class,
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[(&action).into(), (&uri).into()],
        )?;
        if exception_pending(env) {
            return Err("Intent constructor failed".into());
        }
        let pkg_js = env.new_string(pkg)?;
        env.call_method(
            &intent,
            "setPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[(&pkg_js).into()],
        )?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        if start_activity(env, &intent).is_ok() {
            return Ok(true);
        }
        // 2) fallbackLaunch: getLaunchIntentForPackage
        let pm = package_manager(env)?;
        let v = env.call_method(
            &pm,
            "getLaunchIntentForPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[(&pkg_js).into()],
        )?;
        if exception_pending(env) {
            return Ok(false);
        }
        let launch = JObject::try_from(v)?;
        if launch.is_null() {
            return Ok(false);
        }
        env.call_method(
            &launch,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        Ok(start_activity(env, &launch).is_ok())
    })
    .unwrap_or(false)
}

/// `OwenManager.uninstall`: `Intent(ACTION_DELETE, "package:$pkg")` (NEW_TASK).
pub fn uninstall_package(pkg: &str) {
    let _ = jni_run::<()>(|env| {
        let uri = make_uri(env, &format!("package:{pkg}"))?;
        let action = env.new_string(ACTION_DELETE)?;
        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(
            &intent_class,
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[(&action).into(), (&uri).into()],
        )?;
        if exception_pending(env) {
            return Err("Intent constructor failed".into());
        }
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::from(FLAG_ACTIVITY_NEW_TASK)],
        )?;
        let _ = start_activity(env, &intent); // runCatching { … }
        Ok(())
    });
}

/// `OwenManager.isInstalled` extended with the version: `getPackageInfo(pkg, 0)`
/// → `(versionName, versionCode)`; `None` on NameNotFoundException.
pub fn installed_package(pkg: &str) -> Option<(String, i64)> {
    jni_run(|env| {
        let pm = package_manager(env)?;
        let pkg_js = env.new_string(pkg)?;
        let info = env.call_method(
            &pm,
            "getPackageInfo",
            "(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;",
            &[(&pkg_js).into(), JValue::from(0i32)],
        );
        if exception_pending(env) {
            return Ok(None); // NameNotFoundException — not installed
        }
        let info = match info {
            Ok(v) => JObject::try_from(v)?,
            Err(_) => return Ok(None),
        };
        if info.is_null() {
            return Ok(None);
        }
        // versionName (may be null → "")
        let version_name = match env.get_field(&info, "versionName", "Ljava/lang/String;") {
            Ok(v) => match JObject::try_from(v) {
                Ok(o) => rust_string(env, &o).unwrap_or_default(),
                Err(_) => String::new(),
            },
            Err(_) => String::new(),
        };
        // getLongVersionCode() — API 28+; fall back to the int field.
        let code: i64 = match env.call_method(&info, "getLongVersionCode", "()J", &[]) {
            Ok(v) => {
                if exception_pending(env) {
                    return Ok(None);
                }
                jlong::try_from(v)?
            }
            Err(_) => {
                exception_pending(env); // NoSuchMethod on API < 28
                match env.get_field(&info, "versionCode", "I") {
                    Ok(v) => jint::try_from(v)? as i64,
                    Err(_) => 0,
                }
            }
        };
        Ok(Some((version_name, code)))
    })
    .ok()
    .flatten()
}

/* -------------------- public item 12: the PackageInstaller ------------------ */

/// The JNI `PackageInstallHelper` — a real Android `PackageInstaller` session
/// pipeline. The patch step (the app layer's `ApkPatcher` port) runs inside
/// [`JniInstaller::install`] *before* the session, exactly like the original
/// called `ApkPatcher.patchApk` before `PackageInstallHelper.install`.
pub struct JniInstaller {
    /// The open `PackageInstaller$Session` object (`installer.openSession(id)`),
    /// held as a global ref between `submit` and `commit`/`abandon`.
    session: Option<GlobalRef>,
}

impl JniInstaller {
    pub fn new() -> Self {
        JniInstaller { session: None }
    }

    /// The open session object, opening it if needed (`openSession`).
    /// Returns a fresh local reference valid in the caller's frame.
    fn session_obj<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        _id: jint,
    ) -> Result<JObject<'local>, String> {
        if let Some(g) = &self.session {
            // Safe: new_local_ref of the held global ref (owned by this frame).
            return env.new_local_ref(g.as_obj()).map_err(|e| format!("{e:?}"));
        }
        let pm = package_manager(env)?;
        let installer = env
            .call_method(&pm, "getPackageInstaller", "()Landroid/content/pm/PackageInstaller;", &[])
            .map_err(|e| format!("{e:?}"))?;
        if exception_pending(env) {
            return Err("getPackageInstaller failed".into());
        }
        let installer = JObject::try_from(installer).map_err(|e| format!("{e:?}"))?;
        let v = env
            .call_method(
                &installer,
                "openSession",
                "(I)Landroid/content/pm/PackageInstaller$Session;",
                &[JValue::from(_id)],
            )
            .map_err(|e| format!("{e:?}"))?;
        if exception_pending(env) {
            return Err("openSession failed".into());
        }
        let sess = JObject::try_from(v).map_err(|e| format!("{e:?}"))?;
        let g = env.new_global_ref(&sess).map_err(|e| format!("{e:?}"))?;
        self.session = Some(g);
        Ok(sess)
    }

    /// `installer.abandonSession(id)`-equivalent used by [`Installer::abandon`]
    /// (`session.abandon()` on the held object, else via the installer).
    fn drop_session(&mut self) {
        self.session = None;
    }
}

impl Default for JniInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl Installer for JniInstaller {
    /// `installer.createSession(SessionParams(MODE_FULL_INSTALL))` +
    /// `installer.openSession(sessionId)` (the session object is kept for
    /// `submit`/`commit`, like the Kotlin kept one `Session`).
    fn create_session(&mut self) -> Result<InstallSession, String> {
        jni_run(|env| {
            let pm = package_manager(env)?;
            let installer = env
                .call_method(&pm, "getPackageInstaller", "()Landroid/content/pm/PackageInstaller;", &[])
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("getPackageInstaller failed".into());
            }
            let installer = JObject::try_from(installer).map_err(|e| format!("{e:?}"))?;
            let params_class = env.find_class("android/content/pm/PackageInstaller$SessionParams")?;
            let params = env
                .new_object(&params_class, "(I)V", &[JValue::from(MODE_FULL_INSTALL)])
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("SessionParams constructor failed".into());
            }
            let v = env
                .call_method(
                    &installer,
                    "createSession",
                    "(Landroid/content/pm/PackageInstaller$SessionParams;)I",
                    &[(&params).into()],
                )
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("createSession failed".into());
            }
            let id = jint::try_from(v)?;
            // open + hold the session object
            let sess = JObject::try_from(
                env.call_method(
                    &installer,
                    "openSession",
                    "(I)Landroid/content/pm/PackageInstaller$Session;",
                    &[JValue::from(id)],
                )
                .map_err(|e| format!("{e:?}"))?,
            )?;
            if exception_pending(env) {
                return Err("openSession failed".into());
            }
            let g = env.new_global_ref(&sess).map_err(|e| format!("{e:?}"))?;
            self.session = Some(g);
            Ok(id)
        })
        .map(|id| InstallSession { id, apks: Vec::new() })
    }

    /// `session.openWrite("apk_N", 0, len)` + stream copy + `close`.
    fn submit(&mut self, session: &mut InstallSession, apk: &Path) -> Result<(), String> {
        let data = std::fs::read(apk).map_err(|e| format!("read {}: {e}", apk.display()))?;
        let name = format!("apk_{}", session.apks.len());
        jni_run(|env| {
            let sess = self.session_obj(env, session.id)?;
            let name_js = env.new_string(&name)?;
            let stream = env
                .call_method(
                    &sess,
                    "openWrite",
                    "(Ljava/lang/String;JJ)Ljava/io/OutputStream;",
                    &[
                        (&name_js).into(),
                        JValue::from(0i64),
                        JValue::from(data.len() as i64),
                    ],
                )
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("openWrite failed".into());
            }
            let stream = JObject::try_from(stream).map_err(|e| format!("{e:?}"))?;
            const CHUNK: usize = 65536;
            for offset in (0..data.len()).step_by(CHUNK) {
                let end = (offset + CHUNK).min(data.len());
                let buf = env
                    .byte_array_from_slice(&data[offset..end])
                    .map_err(|e| format!("{e:?}"))?;
                env.call_method(
                    &stream,
                    "write",
                    "([BII)V",
                    &[
                        (&buf).into(),
                        JValue::from(0i32),
                        JValue::from((end - offset) as jint),
                    ],
                )
                .map_err(|e| format!("{e:?}"))?;
                if exception_pending(env) {
                    return Err("write threw".into());
                }
            }
            env.call_method(&stream, "close", "()V", &[])
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("close threw".into());
            }
            Ok(())
        })?;
        session.apks.push(apk.to_path_buf());
        Ok(())
    }

    /// `session.commit(pendingIntent.intentSender)` — the status broadcast goes
    /// to the dex receiver, whose `nativeOnReceive` completes the waiter
    /// registered here. Blocks up to [`WAIT_TIMEOUT_MS`] (the original
    /// `installWithTimeout` cap).
    fn commit(
        &mut self,
        session: InstallSession,
        _on_status: &mut dyn FnMut(String),
    ) -> InstallResult {
        let session_id = session.id;
        let (tx, rx) = mpsc::channel();
        let committed = jni_run::<()>(|env| {
            let ctx = context_global()?;
            let pkg = package_name(env)?;
            // statusIntent = Intent(RESULT_ACTION).setPackage(pkg)
            //                 .setComponent(ComponentName(pkg, RECEIVER_CLASS))
            let action = env.new_string(RESULT_ACTION)?;
            let intent_class = env.find_class("android/content/Intent")?;
            let intent = env.new_object(&intent_class, "(Ljava/lang/String;)V", &[(&action).into()])?;
            if exception_pending(env) {
                return Err("Intent(RESULT_ACTION) failed".into());
            }
            let pkg_js = env.new_string(&pkg)?;
            env.call_method(
                &intent,
                "setPackage",
                "(Ljava/lang/String;)Landroid/content/Intent;",
                &[(&pkg_js).into()],
            )?;
            let cls_js = env.new_string(RECEIVER_CLASS.replace('/', "."))?;
            let cn_class = env.find_class("android/content/ComponentName")?;
            let cn = env.new_object(
                &cn_class,
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[(&pkg_js).into(), (&cls_js).into()],
            )?;
            if exception_pending(env) {
                return Err("ComponentName constructor failed".into());
            }
            env.call_method(
                &intent,
                "setComponent",
                "(Landroid/content/ComponentName;)Landroid/content/Intent;",
                &[(&cn).into()],
            )?;
            if exception_pending(env) {
                return Err("setComponent failed".into());
            }
            // PendingIntent.getBroadcast(ctx, sessionId, statusIntent,
            //     FLAG_UPDATE_CURRENT | (FLAG_MUTABLE if SDK >= 31))
            let mut flags = PENDING_FLAG_UPDATE_CURRENT;
            if sdk_int(env).unwrap_or(0) >= SDK_S {
                flags |= PENDING_FLAG_MUTABLE;
            }
            let pi_class = env.find_class("android/app/PendingIntent")?;
            let pi = env.call_static_method(
                &pi_class,
                "getBroadcast",
                "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
                &[
                    (ctx.as_obj()).into(),
                    JValue::from(session_id),
                    (&intent).into(),
                    JValue::from(flags),
                ],
            )?;
            if exception_pending(env) {
                return Err("PendingIntent.getBroadcast failed".into());
            }
            let pi = JObject::try_from(pi)?;
            if pi.is_null() {
                return Err("PendingIntent.getBroadcast returned null".into());
            }
            let sender = env.call_method(&pi, "getIntentSender", "()Landroid/content/IntentSender;", &[])?;
            if exception_pending(env) {
                return Err("getIntentSender failed".into());
            }
            let sender = JObject::try_from(sender)?;
            // waiting[sessionId] = callback — BEFORE commit, like the Kotlin.
            {
                let mut waiting = waiting()
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                waiting.insert(session_id, tx.clone());
            }
            let sess = self.session_obj(env, session_id)?;
            env.call_method(
                &sess,
                "commit",
                "(Landroid/content/IntentSender;)V",
                &[(&sender).into()],
            )
            .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("session.commit threw".into());
            }
            env.call_method(&sess, "close", "()V", &[])
                .map_err(|e| format!("{e:?}"))?;
            if exception_pending(env) {
                return Err("session.close threw".into());
            }
            Ok(())
        });
        self.drop_session();
        if let Err(e) = committed {
            waiting().lock().unwrap_or_else(|p| p.into_inner()).remove(&session_id);
            return InstallResult::new(false, -99, Some(e));
        }
        // The Kotlin: suspendCancellableCoroutine resumed by onResult.
        match rx.recv_timeout(Duration::from_millis(WAIT_TIMEOUT_MS)) {
            Ok(result) => result,
            Err(_) => {
                waiting().lock().unwrap_or_else(|p| p.into_inner()).remove(&session_id);
                diag::e(TAG, &format!("session {session_id}: timed out waiting for the result"));
                InstallResult::new(false, -99, Some("timed out waiting for the install result".into()))
            }
        }
    }

    /// `session.abandon()` (the cancellation path).
    fn abandon(&mut self, _session: InstallSession) {
        if let Some(g) = self.session.take() {
            let _ = jni_run::<()>(|env| {
                let _ = env.call_method(g.as_obj(), "abandon", "()V", &[])?;
                let _ = env.call_method(g.as_obj(), "close", "()V", &[]);
                Ok(())
            });
        }
    }

    /// `PackageInstallHelper.onResult(sessionId, status, message)` — completes
    /// the waiting session. Returns whether a waiter was found.
    fn on_result(&mut self, session_id: i32, status: i32, message: Option<String>) -> bool {
        let mut waiting = waiting().lock().unwrap_or_else(|p| p.into_inner());
        match waiting.remove(&session_id) {
            Some(tx) => {
                let _ = tx.send(InstallResult::new(status == 0, status, message));
                true
            }
            None => false,
        }
    }

    /// The app-layer pipeline: patch `apks[0]` (the Uptodown base APK / XAPK
    /// base), then run the PackageInstaller session over
    /// `[patched, apks[1..]]` (XAPK splits go in unpatched). Delegates the
    /// session lifecycle to the default `Installer::install` (same step order
    /// as `PackageInstallHelper.install`).
    fn install(&mut self, apks: &[PathBuf], on_status: &mut dyn FnMut(String)) -> InstallResult {
        let Some(ctx) = crate::platform::take_install_ctx() else {
            return InstallResult::new(false, -99, Some("no install context".into()));
        };
        let Some(base) = apks.first() else {
            return InstallResult::new(false, -99, Some("no apk".into()));
        };
        // ApkPatcher.patchApk (cache → full pipeline → cache store)
        let (patched, pkg) = match crate::platform::patch_step(&ctx, base, on_status) {
            Ok(v) => v,
            Err(e) => return InstallResult::new(false, -99, Some(e)),
        };
        on_status(format!("patched: {} → {}", patched.display(), pkg));
        // toInstall = [patchedBase] + the remaining XAPK splits
        let mut to_install = Vec::with_capacity(apks.len());
        to_install.push(patched);
        for extra in apks.iter().skip(1) {
            to_install.push(extra.clone());
        }
        let result = Installer::install(self, &to_install, on_status);
        if result.ok {
            // Like the desktop installer: the message carries the package.
            InstallResult::new(true, 0, Some(pkg))
        } else {
            result
        }
    }
}
