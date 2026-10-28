//! WebView redirect — the universal safety net for every WebView load.
//!
//! ART-hooks `android.webkit.WebView#loadUrl(String)` (and the
//! `(String, Map)` overload). Any Epic URL that reaches a WebView — login
//! page, registration, help pages, embedded content — is rewritten to the
//! backend. Combined with the Kairos hooks this covers every WebView flow
//! from 10.31 through 15.50 **without touching a single line of smali**.

use super::art::{self, ArtMethodPatch};
use crate::rules;
use jni::objects::{JObject, JString};
use jni::sys::{jobject, jstring};
use jni::JNIEnv;
use std::sync::Mutex;

static PATCHES: Mutex<Vec<ArtMethodPatch>> = Mutex::new(Vec::new());
static LOAD_URL_FN: Mutex<usize> = Mutex::new(0);
static LOAD_URL_MAP_FN: Mutex<usize> = Mutex::new(0);

unsafe extern "C" fn hook_load_url(env: *mut JNIEnv, thiz: jobject, url: jstring) {
    let env = JNIEnv::from_raw(env);
    run_load_url(&env, thiz, url, false);
    let _ = env.exception_clear();
}

unsafe extern "C" fn hook_load_url_map(
    env: *mut JNIEnv,
    thiz: jobject,
    url: jstring,
    map: jobject,
) {
    let env = JNIEnv::from_raw(env);
    let _ = map;
    run_load_url(&env, thiz, url, true);
    let _ = env.exception_clear();
}

fn run_load_url(env: &JNIEnv, thiz: jobject, url: jstring, with_map: bool) {
    let Some(url_str) = (|| {
        let jstr = unsafe { JString::from_raw(url) };
        art::read_jstring(&mut env.unsafe_clone(), jstr)
    })() else {
        call_original(env, thiz, url, with_map);
        return;
    };

    let Some(new_url) = rules::rewrite(&url_str) else {
        call_original(env, thiz, url, with_map);
        return;
    };

    let jnew = match env.new_string(&new_url) {
        Ok(s) => s,
        Err(_) => {
            call_original(env, thiz, url, with_map);
            return;
        }
    };
    call_original_with(env, thiz, jnew.as_raw(), with_map);
}

/// Restore -> call original -> re-apply (WebView loads are UI-thread
/// serialized, so the dance is safe).
fn call_original(env: &JNIEnv, thiz: jobject, url: jstring, with_map: bool) {
    call_original_with(env, thiz, url, with_map);
}

fn call_original_with(env: &JNIEnv, thiz: jobject, url: jstring, with_map: bool) {
    let patches = PATCHES.lock().unwrap();
    let idx = if with_map { 1 } else { 0 };
    let Some(patch) = patches.get(idx) else {
        return;
    };
    let hook_fn = if with_map {
        *LOAD_URL_MAP_FN.lock().unwrap()
    } else {
        *LOAD_URL_FN.lock().unwrap()
    };

    // Unpatch, call, repatch.
    art::restore(patch);
    let result = (|| {
        let obj = unsafe { JObject::from_raw(thiz) };
        let jstr = unsafe { JString::from_raw(url) };
        let res = if with_map {
            env.call_method(&obj, "loadUrl", "(Ljava/lang/String;Ljava/util/Map;)V", &[jni::objects::JValue::Object(&jstr), jni::objects::JValue::Object(&JObject::null())])
        } else {
            env.call_method(&obj, "loadUrl", "(Ljava/lang/String;)V", &[jni::objects::JValue::Object(&jstr)])
        };
        let _ = env.exception_clear();
        res
    })();
    let _ = result;
    art::reapply(patch, hook_fn);
}

/// Install both `loadUrl` overloads.
pub fn install(env: &mut JNIEnv) -> Result<(), String> {
    let class = art::load_class(env, "android/webkit/WebView")?;

    let hook1 = hook_load_url as usize;
    let patch1 = art::hook_method(
        env,
        "android/webkit/WebView",
        "loadUrl",
        "(Ljava/lang/String;)V",
        false,
        hook1,
    )?;
    *LOAD_URL_FN.lock().unwrap() = hook1;

    let hook2 = hook_load_url_map as usize;
    let patch2 = match art::hook_method(
        env,
        "android/webkit/WebView",
        "loadUrl",
        "(Ljava/lang/String;Ljava/util/Map;)V",
        false,
        hook2,
    ) {
        Ok(p) => {
            *LOAD_URL_MAP_FN.lock().unwrap() = hook2;
            p
        }
        // The map overload appeared in API 21; ignore failures.
        Err(e) => {
            let _ = e;
            ArtMethodPatch {
                method: std::ptr::null_mut(),
                access_flags: 0,
                jni_entry: 0,
                quick_entry: 0,
            }
        }
    };

    let mut patches = PATCHES.lock().unwrap();
    patches.clear();
    patches.push(patch1);
    patches.push(patch2);
    let _ = class;
    Ok(())
}
