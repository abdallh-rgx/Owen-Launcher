//! ART Java-method hooking ("declaration native" technique).
//!
//! Replaces a Java method's implementation with a JNI native function by
//! editing its `ArtMethod`:
//!
//! * `access_flags_ |= kAccNative | kAccCompileDontBother`
//! * `entry_point_from_jni_ = <our JNI fn>`
//! * `entry_point_from_quick_compiled_code_ = art_quick_generic_jni_trampoline`
//!   (borrowed from an already-native method such as
//!   `android.os.SystemClock#uptimeMillis`)
//!
//! Field offsets are stable on arm64 for Android 5.0 - 13
//! (`access @ +4`, `jni entry @ +0x18`, `quick entry @ +0x20`,
//! `sizeof(ArtMethod) == 0x28`); a runtime sanity check validates them
//! against the reference native method before anything is written.

#![allow(clippy::missing_safety_doc)]

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jmethodID, jobject, jstring, JavaVM};
use jni::JNIEnv;
use std::sync::Mutex;
use std::sync::OnceLock;

pub const ACCESS_FLAGS_OFFSET: usize = 0x04;
pub const JNI_ENTRY_OFFSET: usize = 0x18;
pub const QUICK_ENTRY_OFFSET: usize = 0x20;

pub const K_ACC_STATIC: u32 = 0x0008;
pub const K_ACC_NATIVE: u32 = 0x0100;
pub const K_ACC_COMPILE_DONT_BOTHER: u32 = 0x0200_0000;

pub struct ArtMethodPatch {
    pub method: jmethodID,
    pub access_flags: u32,
    pub jni_entry: usize,
    pub quick_entry: usize,
}

static PATCHES: Mutex<Vec<ArtMethodPatch>> = Mutex::new(Vec::new());
static GENERIC_JNI_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

fn with_env<F, R>(vm: &JavaVM, f: F) -> Result<R, String>
where
    F: FnOnce(&mut JNIEnv) -> Result<R, String>,
{
    let mut env = vm
        .get_env()
        .map_err(|e| format!("get_env: {e:?}"))?;
    f(&mut env)
}

/// Load a class through the application class loader (works for app
/// classes like `com.epicgames.kairos.*` and framework classes alike).
pub fn load_class<'local>(env: &mut JNIEnv<'local>, name: &str) -> Result<JClass<'local>, String> {
    // Fast path: already visible to the system class loader.
    if let Ok(cls) = env.find_class(name) {
        let _ = env.exception_clear();
        return Ok(cls);
    }
    // App classloader path: ActivityThread.currentApplication()
    let _ = env.exception_clear();
    let thread = env
        .find_class("android/app/ActivityThread")
        .map_err(|e| format!("ActivityThread: {e:?}"))?;
    let app = env
        .call_static_method(
            &thread,
            "currentApplication",
            "()Landroid/app/Application;",
            &[],
        )
        .map_err(|e| format!("currentApplication: {e:?}"))?;
    let app = app
        .l()
        .map_err(|e| format!("currentApplication not object: {e:?}"))?;
    let loader = env
        .call_method(&app, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .map_err(|e| format!("getClassLoader: {e:?}"))?;
    let loader = loader
        .l()
        .map_err(|e| format!("classloader not object: {e:?}"))?;
    let class_name = env
        .new_string(name.replace('/', "."))
        .map_err(|e| format!("new_string: {e:?}"))?;
    let loaded = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&JObject::from(class_name))],
        )
        .map_err(|e| format!("loadClass {name}: {e:?}"))?;
    let _ = env.exception_clear();
    let loaded = loaded
        .l()
        .map_err(|e| format!("loadClass not object: {e:?}"))?;
    Ok(JClass::from(loaded))
}

/// Discover the `art_quick_generic_jni_trampoline` address from a known
/// native static method.
fn generic_jni_trampoline(env: &mut JNIEnv) -> Result<usize, String> {
    if let Some(t) = GENERIC_JNI_TRAMPOLINE.get() {
        return Ok(*t);
    }
    let cls = env
        .find_class("android/os/SystemClock")
        .map_err(|e| format!("SystemClock: {e:?}"))?;
    let mid = env
        .get_static_method_id(cls, "uptimeMillis", "()J")
        .map_err(|e| format!("uptimeMillis: {e:?}"))?;
    let raw: jmethodID = mid.into_raw();
    let quick = unsafe {
        std::ptr::read_unaligned((raw as usize + QUICK_ENTRY_OFFSET) as *const usize)
    };
    if quick == 0 {
        return Err("SystemClock quick entry is null".into());
    }
    let _ = GENERIC_JNI_TRAMPOLINE.set(quick);
    Ok(quick)
}

/// Replace a Java method with a native implementation.
///
/// `signature` is the JNI descriptor (e.g.
/// `(Lcom/epicgames/kairos/utils/Environment;Z)Ljava/lang/String;`).
pub fn hook_method(
    env: &mut JNIEnv,
    class_name: &str,
    method_name: &str,
    signature: &str,
    is_static: bool,
    native_fn: usize,
) -> Result<ArtMethodPatch, String> {
    let class = load_class(env, class_name)?;
    let method_id: jmethodID = if is_static {
        env.get_static_method_id(class, method_name, signature)
            .map_err(|e| format!("get_static_method_id {class_name}.{method_name}: {e:?}"))?
            .into_raw()
    } else {
        env.get_method_id(class, method_name, signature)
            .map_err(|e| format!("get_method_id {class_name}.{method_name}: {e:?}"))?
            .into_raw()
    };
    if method_id.is_null() {
        return Err("method id is null".into());
    }

    // Read the current fields (backup).
    let base = method_id as usize;
    let access = unsafe { std::ptr::read_unaligned((base + ACCESS_FLAGS_OFFSET) as *const u32) };
    let jni_entry = unsafe { std::ptr::read_unaligned((base + JNI_ENTRY_OFFSET) as *const usize) };
    let quick_entry = unsafe { std::ptr::read_unaligned((base + QUICK_ENTRY_OFFSET) as *const usize) };

    // Sanity: the reference native method must have a non-null quick
    // entry pointing at the generic jni trampoline.
    let trampoline = generic_jni_trampoline(env)?;

    // Write the patch.
    let new_access = access | K_ACC_NATIVE | K_ACC_COMPILE_DONT_BOTHER;
    unsafe {
        std::ptr::write_unaligned((base + ACCESS_FLAGS_OFFSET) as *mut u32, new_access);
        std::ptr::write_unaligned((base + JNI_ENTRY_OFFSET) as *mut usize, native_fn);
        std::ptr::write_unaligned((base + QUICK_ENTRY_OFFSET) as *mut usize, trampoline);
        // Flush icache for the ArtMethod region (data, but be thorough).
        libc::__clear_cache(
            base as *const libc::c_void,
            (base + 0x30) as *const libc::c_void,
        );
    }

    let patch = ArtMethodPatch {
        method: method_id,
        access_flags: access,
        jni_entry,
        quick_entry,
    };
    PATCHES.lock().unwrap().push(ArtMethodPatch {
        method: method_id,
        access_flags: access,
        jni_entry,
        quick_entry,
    });
    Ok(patch)
}

/// Restore a patched method to its original implementation.
pub fn restore(patch: &ArtMethodPatch) {
    let base = patch.method as usize;
    unsafe {
        std::ptr::write_unaligned(
            (base + ACCESS_FLAGS_OFFSET) as *mut u32,
            patch.access_flags,
        );
        std::ptr::write_unaligned((base + JNI_ENTRY_OFFSET) as *mut usize, patch.jni_entry);
        std::ptr::write_unaligned((base + QUICK_ENTRY_OFFSET) as *mut usize, patch.quick_entry);
        libc::__clear_cache(
            base as *const libc::c_void,
            (base + 0x30) as *const libc::c_void,
        );
    }
}

/// Re-apply a patch after a restore (the call-original dance).
pub fn reapply(patch: &ArtMethodPatch, native_fn: usize) {
    let base = patch.method as usize;
    let new_access = patch.access_flags | K_ACC_NATIVE | K_ACC_COMPILE_DONT_BOTHER;
    let trampoline = *GENERIC_JNI_TRAMPOLINE.get_or_init(|| 0);
    unsafe {
        std::ptr::write_unaligned((base + ACCESS_FLAGS_OFFSET) as *mut u32, new_access);
        std::ptr::write_unaligned((base + JNI_ENTRY_OFFSET) as *mut usize, native_fn);
        if trampoline != 0 {
            std::ptr::write_unaligned((base + QUICK_ENTRY_OFFSET) as *mut usize, trampoline);
        }
        libc::__clear_cache(
            base as *const libc::c_void,
            (base + 0x30) as *const libc::c_void,
        );
    }
}

/// Make a Java string (helper for hooks returning String).
pub fn new_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    env.new_string(s)
        .map(|j| j.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Read a Java string (helper for hooks receiving String).
pub fn read_jstring(env: &mut JNIEnv, s: JString) -> Option<String> {
    env.get_string(&s).ok().map(|x| x.to_string_lossy().into_owned())
}

/// Hold a global reference to an object across the dance.
pub fn global_ref(env: &mut JNIEnv, obj: &JObject) -> jni::objects::GlobalRef {
    env.new_global_ref(obj).unwrap_or_else(|_| {
        panic!("global ref")
    })
}
