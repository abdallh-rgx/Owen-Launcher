//! Kairos-era redirect: `DebouncedLimiter` URL builders.
//!
//! Verified against real builds (12.50 decompiled):
//!
//! ```text
//! AuthFragment  -> DebouncedLimiter.loginUrl(Environment, boolean)
//!                  -> WebView.loadUrl(...)                 // login page
//! KernelImpl    -> DebouncedLimiter.getGraphqlUrl(Environment)
//!                  -> ApolloClient.Builder.serverUrl(...)  // PartyHub graphql
//! DebouncedLimiter.registerUrl(Environment, boolean)       // registration
//! DebouncedLimiter.getHostName(Environment)                // host override
//! ```
//!
//! All four are public static methods with stable signatures across the
//! whole Kairos era (10.31-15.50) — the dex obfuscation around them does
//! not matter because we hook the method, not its body. This is the
//! smali-free replacement for the old `const-string` edits.

use super::art;
use crate::rules;
use jni::sys::{jboolean, jclass, jobject, jstring};
use jni::JNIEnv;

const KAIROS_CLASS: &str = "com/epicgames/kairos/utils/DebouncedLimiter";
const ENV_TYPE: &str = "Lcom/epicgames/kairos/utils/Environment;";

unsafe extern "C" fn hook_login_url(
    env: *mut JNIEnv,
    _class: jclass,
    _environment: jobject,
    _b: jboolean,
) -> jstring {
    let mut env = JNIEnv::from_raw(env);
    let url = rules::login_url();
    art::new_jstring(&mut env, &url)
}

unsafe extern "C" fn hook_get_graphql_url(
    env: *mut JNIEnv,
    _class: jclass,
    _environment: jobject,
) -> jstring {
    let mut env = JNIEnv::from_raw(env);
    let url = rules::partyhub_graphql_url();
    art::new_jstring(&mut env, &url)
}

unsafe extern "C" fn hook_register_url(
    env: *mut JNIEnv,
    _class: jclass,
    _environment: jobject,
    _b: jboolean,
) -> jstring {
    let mut env = JNIEnv::from_raw(env);
    let url = rules::register_url();
    art::new_jstring(&mut env, &url)
}

unsafe extern "C" fn hook_get_host_name(
    env: *mut JNIEnv,
    _class: jclass,
    _environment: jobject,
) -> jstring {
    let mut env = JNIEnv::from_raw(env);
    let host = rules::host_name();
    art::new_jstring(&mut env, &host)
}

/// Install all Kairos hooks. Missing methods (non-Kairos builds) are
/// reported but do not fail the installation.
pub fn install(env: &mut JNIEnv) -> Result<Vec<String>, String> {
    let mut installed = Vec::new();

    let attempts: [(&str, &str, bool, usize); 4] = [
        (
            "loginUrl",
            &format!("({ENV_TYPE}Z)Ljava/lang/String;"),
            true,
            hook_login_url as usize,
        ),
        (
            "getGraphqlUrl",
            &format!("({ENV_TYPE})Ljava/lang/String;"),
            true,
            hook_get_graphql_url as usize,
        ),
        (
            "registerUrl",
            &format!("({ENV_TYPE}Z)Ljava/lang/String;"),
            true,
            hook_register_url as usize,
        ),
        (
            "getHostName",
            &format!("({ENV_TYPE})Ljava/lang/String;"),
            true,
            hook_get_host_name as usize,
        ),
    ];

    for (name, sig, is_static, fn_ptr) in attempts {
        match art::hook_method(env, KAIROS_CLASS, name, sig, is_static, fn_ptr) {
            Ok(_) => installed.push(format!("DebouncedLimiter.{name}")),
            Err(e) => {
                let _ = env.exception_clear();
                return Err(format!(
                    "Kairos hook {name} failed: {e} (build may not be Kairos-era)"
                ));
            }
        }
    }
    Ok(installed)
}
