//! # owen-patcher-jni
//!
//! The JNI surface of [`owen_patcher`] — `libowenpatch.so`.
//!
//! The Android launcher (`NativePatcher.kt`) calls `patchApk` instead of the
//! legacy Kotlin smali pipeline (baksmali → dex edit → smali → repack), which
//! OOM-thrashed the on-device heap. The Rust pipeline is zero-smali
//! (DT_NEEDED injection), streams the zip entries and signs with APK
//! Signature Scheme v2 in-process.
//!
//! ## Safety model
//!
//! This crate is the thin, documented JNI adapter: the ONLY unsafe code is
//! the raw JNI boundary itself (string extraction + result strings). All
//! patching logic lives in `owen-patcher` which is `#![forbid(unsafe_code)]`.
//!
//! Java signature (com.abdallhrgx.owenlauncher.core.apk.NativePatcher):
//!
//! ```java
//! static { System.loadLibrary("owenpatch"); }
//! // returns a JSON PatchReport; throws RuntimeException on failure
//! static native String patchApk(String inputApk, String outputApk,
//!     String owenLib, String keyPath /* nullable */, String renamePackage /* nullable */);
//! ```

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use owen_patcher::{patch_apk, PatchOptions};
use std::path::PathBuf;

fn get_string(env: &mut JNIEnv, s: JString) -> Result<String, String> {
    // jni 0.21's `get_string` is a safe wrapper — no unsafe needed here at
    // all; this crate compiles with zero unsafe blocks.
    env.get_string(&s)
        .map(|j| String::from(std::borrow::Cow::from(&*j)))
        .map_err(|e| format!("jni string: {e}"))
}

fn inner(
    env: &mut JNIEnv,
    input: JString,
    output: JString,
    owen_lib: JString,
    key_path: JString,
    rename_package: JString,
) -> Result<String, String> {
    let input = get_string(env, input)?;
    let output = get_string(env, output)?;
    let owen_lib = get_string(env, owen_lib)?;
    let key_path = if key_path.as_raw().is_null() {
        None
    } else {
        Some(PathBuf::from(get_string(env, key_path)?))
    };
    let rename_package = if rename_package.as_raw().is_null() {
        None
    } else {
        Some(get_string(env, rename_package)?)
    };

    let opts = PatchOptions {
        owen_lib: PathBuf::from(&owen_lib),
        target_lib: None,
        no_sign: false,
        key_path,
        rename_package,
        extra_libs: Vec::new(),
    };
    let report = patch_apk(PathBuf::from(&input).as_path(), PathBuf::from(&output).as_path(), &opts)
        .map_err(|e| format!("patchApk failed: {e}"))?;
    serde_json::to_string(&report).map_err(|e| format!("serialize report: {e}"))
}

#[no_mangle]
pub extern "system" fn Java_com_abdallhrgx_owenlauncher_core_apk_NativePatcher_patchApk(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
    output: JString,
    owen_lib: JString,
    key_path: JString,
    rename_package: JString,
) -> jstring {
    match inner(&mut env, input, output, owen_lib, key_path, rename_package) {
        Ok(json) => match env.new_string(json) {
            Ok(s) => s.as_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException", e);
            std::ptr::null_mut()
        }
    }
}
