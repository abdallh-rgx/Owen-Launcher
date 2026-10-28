//! Minimal JNIEnv bridge — just enough for the ART layer (class lookup,
//! method lookup, string creation and calls).
//!
//! Every function-table entry is addressed by its slot number in the
//! JNINativeInterface table (fixed by the JNI specification), which keeps the
//! bridge independent of struct-layout drift.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

// Opaque JNI types
#[repr(C)]
pub struct _jobject {
    _private: [u8; 0],
}
#[repr(C)]
pub struct _jclass {
    _private: [u8; 0],
}
#[repr(C)]
pub struct _jstring {
    _private: [u8; 0],
}
#[repr(C)]
pub struct _jmethodID {
    _private: [u8; 0],
}

pub type jobject = *mut _jobject;
pub type jclass = *mut _jclass;
pub type jstring = *mut _jstring;
pub type jmethodID = *mut _jmethodID;
pub type jvalue = u64;
pub type jint = i32;
pub type jboolean = u8;

pub const JNI_OK: i32 = 0;
pub const JNI_VERSION_1_6: i32 = 0x0001_0006;

#[repr(C)]
pub struct JNIEnv {
    pub functions: *mut core::ffi::c_void,
}

#[repr(C)]
pub struct JavaVM {
    pub functions: *mut core::ffi::c_void,
}

extern "C" {
    pub fn JNI_GetCreatedJavaVMs(
        vms: *mut *mut JavaVM,
        size: usize,
        count: *mut usize,
    ) -> jint;
}

// JNI function table slots (JNINativeInterface indices).
pub const SLOT_FIND_CLASS: usize = 6;
pub const SLOT_EXCEPTION_OCCURRED: usize = 15;
pub const SLOT_EXCEPTION_CLEAR: usize = 17;
pub const SLOT_GET_OBJECT_CLASS: usize = 31;
pub const SLOT_GET_METHOD_ID: usize = 33;
pub const SLOT_CALL_VOID_METHOD_V: usize = 62;
pub const SLOT_GET_STATIC_METHOD_ID: usize = 113;
pub const SLOT_CALL_STATIC_OBJECT_METHOD_V: usize = 115;
pub const SLOT_NEW_STRING_UTF: usize = 167;
pub const SLOT_GET_STRING_UTF_CHARS: usize = 169;
pub const SLOT_RELEASE_STRING_UTF_CHARS: usize = 170;

/// Fetch a function pointer from the JNIEnv table by slot.
///
/// SAFETY: `env` must be a valid JNIEnv and `slot` must index a real
/// function whose type matches `T`.
#[inline]
pub unsafe fn slot_fn<T>(env: *mut JNIEnv, slot: usize) -> Option<T>
where
    T: Copy,
{
    unsafe {
        let table = (*env).functions as *const *const core::ffi::c_void;
        if table.is_null() {
            return None;
        }
        let f = *table.add(slot);
        if f.is_null() {
            return None;
        }
        Some(core::mem::transmute_copy::<*const core::ffi::c_void, T>(&f))
    }
}

/// Obtain the JNIEnv of the current thread (attaching a native thread when
/// necessary) and run `f` with it.
pub fn with_env<F, R>(f: F) -> Option<R>
where
    F: FnOnce(*mut JNIEnv) -> Option<R>,
{
    type GetEnvFn =
        unsafe extern "C" fn(*mut JavaVM, *mut *mut core::ffi::c_void, jint) -> jint;
    type AttachFn =
        unsafe extern "C" fn(*mut JavaVM, *mut *mut JNIEnv, *mut core::ffi::c_void) -> jint;

    let mut vm: *mut JavaVM = core::ptr::null_mut();
    let mut count: usize = 0;
    // SAFETY: contract call with valid out-buffers.
    let rc = unsafe { JNI_GetCreatedJavaVMs(&mut vm, 1, &mut count) };
    if rc != JNI_OK || count == 0 || vm.is_null() {
        return None;
    }
    // SAFETY: the VM table pointer is valid for the process lifetime.
    let vm_table = unsafe { (*vm).functions as *const *const core::ffi::c_void };
    if vm_table.is_null() {
        return None;
    }
    // GetEnv is slot 6, AttachCurrentThread is slot 4 of the JavaVM table.
    // SAFETY: slot access on a valid table.
    let get_env = unsafe {
        let p = *vm_table.add(6);
        if p.is_null() {
            return None;
        }
        core::mem::transmute::<*const core::ffi::c_void, GetEnvFn>(p)
    };
    let attach = unsafe {
        let p = *vm_table.add(4);
        if p.is_null() {
            return None;
        }
        core::mem::transmute::<*const core::ffi::c_void, AttachFn>(p)
    };

    let mut env: *mut core::ffi::c_void = core::ptr::null_mut();
    // SAFETY: documented lookup.
    if unsafe { get_env(vm, &mut env, JNI_VERSION_1_6) } == JNI_OK && !env.is_null() {
        return f(env as *mut JNIEnv);
    }
    let mut attached: *mut JNIEnv = core::ptr::null_mut();
    // SAFETY: documented attach.
    if unsafe { attach(vm, &mut attached, core::ptr::null_mut()) } == JNI_OK && !attached.is_null()
    {
        return f(attached);
    }
    None
}

/// Find a class by slash-separated binary name (NUL-terminated literal).
pub fn find_class(env: *mut JNIEnv, name: &[u8]) -> Option<jclass> {
    // SAFETY: FindClass(env, const char*) with a NUL-terminated literal.
    unsafe {
        let f = slot_fn::<unsafe extern "C" fn(*mut JNIEnv, *const u8) -> jclass>(
            env,
            SLOT_FIND_CLASS,
        )?;
        let cls = f(env, name.as_ptr());
        if cls.is_null() {
            None
        } else {
            Some(cls)
        }
    }
}

/// Look up an instance method.
pub fn get_method_id(env: *mut JNIEnv, class: jclass, name: &[u8], sig: &[u8]) -> Option<jmethodID> {
    // SAFETY: GetMethodID(env, clazz, name, sig) with NUL-terminated inputs.
    unsafe {
        let f = slot_fn::<
            unsafe extern "C" fn(*mut JNIEnv, jclass, *const u8, *const u8) -> jmethodID,
        >(env, SLOT_GET_METHOD_ID)?;
        let id = f(env, class, name.as_ptr(), sig.as_ptr());
        if id.is_null() {
            clear_exception(env);
            None
        } else {
            Some(id)
        }
    }
}

/// Look up a static method.
pub fn get_static_method_id(
    env: *mut JNIEnv,
    class: jclass,
    name: &[u8],
    sig: &[u8],
) -> Option<jmethodID> {
    // SAFETY: GetStaticMethodID(env, clazz, name, sig).
    unsafe {
        let f = slot_fn::<
            unsafe extern "C" fn(*mut JNIEnv, jclass, *const u8, *const u8) -> jmethodID,
        >(env, SLOT_GET_STATIC_METHOD_ID)?;
        let id = f(env, class, name.as_ptr(), sig.as_ptr());
        if id.is_null() {
            clear_exception(env);
            None
        } else {
            Some(id)
        }
    }
}

/// Clear any pending JNI exception (lookups throw when missing).
pub fn clear_exception(env: *mut JNIEnv) {
    // SAFETY: both functions are documented and safe with no pending throw.
    unsafe {
        let occurred = slot_fn::<unsafe extern "C" fn(*mut JNIEnv) -> jobject>(
            env,
            SLOT_EXCEPTION_OCCURRED,
        );
        let clear = slot_fn::<unsafe extern "C" fn(*mut JNIEnv)>(env, SLOT_EXCEPTION_CLEAR);
        if let (Some(occurred), Some(clear)) = (occurred, clear) {
            if !occurred(env).is_null() {
                clear(env);
            }
        }
    }
}

/// java.lang.String from a NUL-terminated modified-UTF8 literal.
pub fn new_string_utf(env: *mut JNIEnv, s: &[u8]) -> Option<jstring> {
    // SAFETY: NewStringUTF(env, const char*).
    unsafe {
        let f = slot_fn::<unsafe extern "C" fn(*mut JNIEnv, *const u8) -> jstring>(
            env,
            SLOT_NEW_STRING_UTF,
        )?;
        let js = f(env, s.as_ptr());
        if js.is_null() {
            None
        } else {
            Some(js)
        }
    }
}

/// Borrow the modified-UTF8 contents of a jstring as a Rust String.
pub fn string_to_rust(env: *mut JNIEnv, s: jstring) -> Option<String> {
    // SAFETY: GetStringUTFChars/ReleaseStringUTFChars pairing per spec.
    unsafe {
        let get = slot_fn::<unsafe extern "C" fn(*mut JNIEnv, jstring, *mut u8) -> *const u8>(
            env,
            SLOT_GET_STRING_UTF_CHARS,
        )?;
        let release =
            slot_fn::<unsafe extern "C" fn(*mut JNIEnv, jstring, *const u8)>(
                env,
                SLOT_RELEASE_STRING_UTF_CHARS,
            )?;
        let p = get(env, s, core::ptr::null_mut());
        if p.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *p.add(len) != 0 {
            len += 1;
        }
        let bytes = core::slice::from_raw_parts(p, len);
        let out = String::from_utf8_lossy(bytes).into_owned();
        release(env, s, p);
        Some(out)
    }
}

/// Call `void thiz.method(args)` through a jmethodID (used to invoke the
/// backed-up original body of a hooked method).
///
/// SAFETY: `method` must belong to the class of `thiz` and match the vararg
/// types of the original Java signature.
pub unsafe fn call_void_method_v(
    env: *mut JNIEnv,
    thiz: jobject,
    method: jmethodID,
    args: *mut jvalue,
) {
    // SAFETY: CallVoidMethodV(env, obj, methodID, va_list-as-jvalue*).
    unsafe {
        if let Some(f) = slot_fn::<
            unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *mut jvalue),
        >(env, SLOT_CALL_VOID_METHOD_V)
        {
            f(env, thiz, method, args);
        }
    }
}
