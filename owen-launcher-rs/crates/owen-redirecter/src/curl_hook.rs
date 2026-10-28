//! The curl_easy_setopt hook — Layer 1 of the no-smali stack.
//!
//! Every HTTP request the Unreal HTTP module or the EOS SDK makes flows
//! through `curl_easy_setopt(handle, CURLOPT_URL, const char*)`. We rewrite
//! the URL in flight according to the rewrite engine; the backend scheme
//! (http or https) is applied verbatim from the configuration.

use crate::config::OwenConfig;
use crate::rewrite;
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

pub const CURLOPT_URL: i32 = 10002;

type SetOptFn = unsafe extern "C" fn(*mut c_void, i32, *mut c_void) -> i32;

/// Relocated original, installed by `owen_hook::install`.
pub static ORIGINAL_SETOPT: AtomicUsize = AtomicUsize::new(0);

/// Live configuration snapshot (write-once at startup).
pub static CONFIG: RwLock<Option<OwenConfig>> = RwLock::new(None);

thread_local! {
    /// Rewritten URL storage. libcurl copies the string during setopt, so a
    /// per-thread buffer that lives until the next call is sufficient.
    static URL_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Install the hook on `target` and record the relocated original.
pub fn install_hook(target: usize) -> Result<(), owen_hook::HookError> {
    let installed = owen_hook::install(target, curl_easy_setopt_hook as usize)?;
    ORIGINAL_SETOPT.store(installed.original.as_addr(), Ordering::SeqCst);
    Ok(())
}

/// The hook body. Kept `extern "C"` so its address can be handed to
/// `owen_hook::install`.
///
/// Note on the variadic original: `curl_easy_setopt` is variadic, but its
/// first three register arguments (handle, option, value) are all we touch.
/// The integer/pointer varargs used by every CURLOption live in x2, which we
/// pass through untouched — the same proven arrangement as the original
/// Owen.c hook.
pub extern "C" fn curl_easy_setopt_hook(
    handle: *mut c_void,
    option: i32,
    value: *mut c_void,
) -> i32 {
    let original = ORIGINAL_SETOPT.load(Ordering::SeqCst);
    if original == 0 {
        return -1;
    }
    // SAFETY: the relocated original prologue is executable code produced by
    // owen-hook for exactly this signature family.
    let original: SetOptFn = unsafe { core::mem::transmute(original) };

    if option == CURLOPT_URL && !value.is_null() {
        // SAFETY: CURLOPT_URL passes a NUL-terminated C string by contract.
        let url = unsafe { cstr_to_string(value as *const u8) };
        if let Some(url) = url {
            let rewritten = {
                let guard = CONFIG.read();
                match guard {
                    Ok(cfg) => cfg.as_ref().and_then(|c| rewrite::rewrite(&url, c)),
                    Err(_) => None,
                }
            };
            if let Some(new_url) = rewritten {
                return URL_BUFFER.with(|buf| {
                    let mut b = buf.borrow_mut();
                    b.clear();
                    b.extend_from_slice(new_url.as_bytes());
                    b.push(0);
                    // SAFETY: passing our own NUL-terminated buffer is the
                    // documented CURLOPT_URL contract.
                    unsafe { original(handle, option, b.as_ptr() as *mut c_void) }
                });
            }
        }
    }

    // SAFETY: pass-through call into the relocated original.
    unsafe { original(handle, option, value) }
}

/// SAFETY: `p` must point to a NUL-terminated byte string.
unsafe fn cstr_to_string(p: *const u8) -> Option<String> {
    unsafe {
        if p.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *p.add(len) != 0 && len < rewrite::MAX_URL {
            len += 1;
        }
        Some(String::from_utf8_lossy(core::slice::from_raw_parts(p, len)).into_owned())
    }
}
