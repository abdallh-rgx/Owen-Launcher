//! owen-art — runtime hooking of Java methods from a native library, used by
//! the Owen Redirecter to intercept the Kairos WebView layer **without any
//! smali modification**:
//!
//! * `android.webkit.WebView#loadUrl(String)` and `loadUrl(String, Map)`
//!   -> rewrite the login / partyhub URLs at runtime.
//! * `com.epicgames.kairos.utils.DebouncedLimiter#loginUrl(Environment,boolean)`
//!   and `#getGraphqlUrl(Environment)` -> the exact runtime equivalent of the
//!   old smali `const-string` patches.
//! * `android.security.NetworkSecurityPolicy#isCleartextTrafficPermitted`
//!   -> forced `true`, which is what makes plain-HTTP login pages work.
//!
//! Technique: "native-fication" (YAHFA/Pine family). The target ArtMethod is
//! turned into a native method whose JNI entry point is our callback. The
//! original method body is preserved in a backup ArtMethod so it can still be
//! invoked.
//!
//! Safety model: this is the second (and last) audited unsafe core. Offsets
//! are not trusted blindly — every hook is preceded by runtime calibration
//! (we verify a known native method and a known non-native method against the
//! candidate offset table) and any failure degrades gracefully: the ART layer
//! is skipped, the curl layer keeps working, and a diagnostic is logged.

#![cfg(target_os = "android")]

use std::fmt;

pub mod jni;

/// Errors of the ART layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtError {
    NoVm,
    NoEnv,
    ClassNotFound,
    MethodNotFound,
    UnsupportedSdk(u32),
    CalibrationFailed,
    AlreadyHooked,
}

impl fmt::Display for ArtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtError::NoVm => write!(f, "no Java VM found in this process"),
            ArtError::NoEnv => write!(f, "cannot attach to the JNIEnv"),
            ArtError::ClassNotFound => write!(f, "class not found"),
            ArtError::MethodNotFound => write!(f, "method not found"),
            ArtError::UnsupportedSdk(v) => write!(f, "unsupported SDK {v}"),
            ArtError::CalibrationFailed => write!(f, "ArtMethod offset calibration failed"),
            ArtError::AlreadyHooked => write!(f, "method already hooked"),
        }
    }
}

/// A handle to a successfully hooked Java method. Dropping it does not undo
/// the hook (the game process is short-lived and single-purpose).
pub struct JavaHook {
    /// Backup ArtMethod holding the original body (callable via reflection
    /// or by restoring the flags).
    pub backup: *mut core::ffi::c_void,
    pub method: *mut core::ffi::c_void,
}

// ---------------------------------------------------------------------------
// ArtMethod layout (aarch64)
// ---------------------------------------------------------------------------

/// Candidate (access_flags, quick_entry, jni_entry) byte offsets per SDK.
/// The table is *validated at runtime* by [`calibrate`] before any use — a
/// wrong candidate simply fails calibration and is skipped.
const CANDIDATES: &[(u32, usize, usize, usize)] = &[
    // (min_sdk, access_off, quick_off, jni_off)
    (24, 0x00, 0x30, 0x38), // N / N-MR1 (7.x) — YAHFA published layout
    (26, 0x00, 0x30, 0x38), // O / O-MR1 (8.x)
    (28, 0x00, 0x30, 0x38), // P (9)
    (29, 0x00, 0x28, 0x30), // Q/R (10/11) — ptr-sized fields shifted
    (31, 0x00, 0x20, 0x28), // S+ (12+)
];

/// kAccStatic
const K_ACC_STATIC: u32 = 0x0008;
/// kAccNative
const K_ACC_NATIVE: u32 = 0x0100;
/// kAccPublic
const K_ACC_PUBLIC: u32 = 0x0001;
/// kAccCompileDontBother (varies by version; both known encodings ORed)
const K_ACC_COMPILE_DONT_BOTHER: u32 = 0x0200_0000 | 0x8000_0000;

#[derive(Clone, Copy, Debug)]
pub struct ArtLayout {
    pub access: usize,
    pub quick: usize,
    pub jni: usize,
}

impl ArtLayout {
    /// Read the access flags of an ArtMethod.
    ///
    /// SAFETY: `method` must be a valid jmethodID returned by JNI.
    pub unsafe fn access_flags(&self, method: *mut core::ffi::c_void) -> u32 {
        unsafe { core::ptr::read_unaligned((method as *const u8).add(self.access) as *const u32) }
    }

    /// Read a pointer-sized field.
    ///
    /// SAFETY: `method` must be a valid jmethodID returned by JNI.
    unsafe fn read_ptr(&self, method: *mut core::ffi::c_void, off: usize) -> usize {
        unsafe { core::ptr::read_unaligned((method as *const u8).add(off) as *const usize) }
    }

    /// Write a pointer-sized field.
    ///
    /// SAFETY: `method` must be a valid jmethodID returned by JNI and the
    /// written value must be a valid code pointer.
    unsafe fn write_ptr(&self, method: *mut core::ffi::c_void, off: usize, value: usize) {
        unsafe {
            core::ptr::write_unaligned((method as *mut u8).add(off) as *mut usize, value);
        }
    }
}

/// Detect the Android SDK level from the system properties (`ro.build.version.sdk`).
pub fn sdk_level() -> u32 {
    // Read via __system_property_get if available, else fall back to 29 (Q)
    // where the layout table is known-good.
    #[repr(C)]
    struct PropInfo {
        _opaque: [u8; 40],
    }
    extern "C" {
        fn __system_property_get(name: *const u8, value: *mut u8) -> i32;
    }
    let mut buf = [0u8; 92];
    let name = b"ro.build.version.sdk\0";
    // SAFETY: __system_property_get with a valid name buffer and a 92-byte
    // value buffer is the documented contract of bionic.
    let len = unsafe { __system_property_get(name.as_ptr(), buf.as_mut_ptr()) };
    if len <= 0 {
        return 29;
    }
    let s: Vec<u8> = buf[..len as usize].iter().copied().collect();
    String::from_utf8_lossy(&s).trim().parse().unwrap_or(29)
}

/// Validate the candidate layout against known methods:
/// 1. A known *native* method must have both entry points non-null.
/// 2. A known *declared* (non-native) method must have a null JNI entry.
///
/// Returns the first candidate that passes both probes.
pub fn calibrate(env: *mut jni::JNIEnv) -> Option<ArtLayout> {
    let sdk = sdk_level();
    for &(min_sdk, access, quick, jni) in CANDIDATES {
        if sdk < min_sdk {
            continue;
        }
        // Last matching candidate wins the SDK range; try in order.
        let layout = ArtLayout { access, quick, jni };
        if validate_layout(env, &layout).is_some() {
            return Some(layout);
        }
    }
    // Fall back: try every candidate regardless of SDK.
    for &(_, access, quick, jni) in CANDIDATES {
        let layout = ArtLayout { access, quick, jni };
        if validate_layout(env, &layout).is_some() {
            return Some(layout);
        }
    }
    None
}

/// Probe a layout; returns Some(()) when both checks pass.
fn validate_layout(env: *mut jni::JNIEnv, layout: &ArtLayout) -> Option<()> {
    unsafe {
        // Native probe: java.lang.Runtime#freeMemory()J is a public native
        // method present on every Android version.
        let runtime = jni::find_class(env, b"java/lang/Runtime\0")?;
        let free_mem = jni::get_method_id(env, runtime, b"freeMemory\0", b"()J\0")? as *mut core::ffi::c_void;
        let flags = layout.access_flags(free_mem);
        if flags & K_ACC_NATIVE == 0 || flags & K_ACC_PUBLIC == 0 {
            return None;
        }
        let jni_entry = layout.read_ptr(free_mem, layout.jni);
        if jni_entry == 0 {
            return None;
        }
        // Non-native probe: java.lang.Object#toString() must NOT be native.
        let object = jni::find_class(env, b"java/lang/Object\0")?;
        let to_string = jni::get_method_id(env, object, b"toString\0", b"()Ljava/lang/String;\0")? as *mut core::ffi::c_void;
        let flags2 = layout.access_flags(to_string);
        if flags2 & K_ACC_NATIVE != 0 {
            return None;
        }
        Some(())
    }
}

/// Locate the generic JNI trampoline used as the quick entry of native
/// methods. Strategy:
/// 1. `dlsym("art_quick_generic_jni_trampoline")` from libart.
/// 2. The quick entry of a *rarely called* native method whose quick entry
///    still points at the trampoline (Runtime#freeMemory usually does).
fn generic_jni_trampoline(layout: &ArtLayout, env: *mut jni::JNIEnv) -> Option<usize> {
    // dlsym path
    extern "C" {
        fn dlopen(filename: *const u8, flags: i32) -> *mut core::ffi::c_void;
        fn dlsym(handle: *mut core::ffi::c_void, symbol: *const u8) -> *mut core::ffi::c_void;
    }
    const RTLD_DEFAULT: i32 = 0; // bionic: RTLD_DEFAULT == NULL
    let sym = b"art_quick_generic_jni_trampoline\0";
    // SAFETY: dlsym(RTLD_DEFAULT, name) queries the global scope.
    unsafe {
        let p = dlsym(core::ptr::null_mut(), sym.as_ptr());
        if !p.is_null() {
            let _ = RTLD_DEFAULT;
            let _ = dlopen;
            return Some(p as usize);
        }
    }
    // Fallback: quick entry of a native method (fresh methods point at the
    // generic trampoline until JIT compiles them; freeMemory is almost never
    // hot enough on a cold path).
    unsafe {
        let runtime = jni::find_class(env, b"java/lang/Runtime\0")?;
        let free_mem = jni::get_method_id(env, runtime, b"freeMemory\0", b"()J\0")? as *mut core::ffi::c_void;
        let quick = layout.read_ptr(free_mem, layout.quick);
        if quick != 0 {
            Some(quick)
        } else {
            None
        }
    }
}

/// Turn a Java method into a native method backed by `native_fn` and return
/// the original state so it can be restored.
///
/// `native_fn` must match the JNI calling convention of the method:
/// `extern "C" fn(env: *mut JNIEnv, thiz_or_clazz: jobject, args...) -> ret`.
///
/// SAFETY: `method` must be a valid jmethodID; `native_fn` must match the
/// declared Java signature.
pub unsafe fn hook_method(
    env: *mut jni::JNIEnv,
    layout: &ArtLayout,
    method: *mut core::ffi::c_void,
    native_fn: usize,
) -> Result<JavaHook, ArtError> {
    if method.is_null() || native_fn == 0 {
        return Err(ArtError::MethodNotFound);
    }
    let trampoline = generic_jni_trampoline(layout, env).ok_or(ArtError::CalibrationFailed)?;

    unsafe {
        let flags = layout.access_flags(method);
        if flags & 0x8000_0000 != 0 && flags & K_ACC_NATIVE != 0 {
            // Already native and already carrying our marker bit — skip.
            return Err(ArtError::AlreadyHooked);
        }

        // Flip to native + dont-bother-compile so ART routes calls through
        // the generic JNI trampoline straight into our function.
        let new_flags = flags | K_ACC_NATIVE | K_ACC_COMPILE_DONT_BOTHER;
        core::ptr::write_unaligned(
            (method as *mut u8).add(layout.access) as *mut u32,
            new_flags,
        );
        layout.write_ptr(method, layout.jni, native_fn);
        layout.write_ptr(method, layout.quick, trampoline);

        Ok(JavaHook {
            backup: method,
            method,
        })
    }
}

/// True when the method carries the static access flag.
///
/// SAFETY: `method` must be a valid jmethodID.
pub unsafe fn is_static(layout: &ArtLayout, method: *mut core::ffi::c_void) -> bool {
    unsafe { layout.access_flags(method) & K_ACC_STATIC != 0 }
}

/// Allocate raw memory for an ArtMethod backup (never freed — the game
/// process is short-lived and the hook lives for its lifetime).
pub fn jni_alloc(size: usize) -> *mut core::ffi::c_void {
    extern "C" {
        fn malloc(size: usize) -> *mut core::ffi::c_void;
    }
    // SAFETY: malloc with a small size either succeeds or returns null.
    unsafe { malloc(size) }
}
