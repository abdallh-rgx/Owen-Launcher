//! curl_easy_setopt hook — the universal native redirect.
//!
//! `CURLOPT_URL == 10002`. Every HTTP request the engine makes through
//! libcurl (libUE4.so / libUnreal.so) and every EOS request
//! (libEOSSDK.so, which statically links its own libcurl) passes through
//! `curl_easy_setopt(handle, CURLOPT_URL, "https://...")`. One hook
//! covers all versions — the address is found automatically at load time
//! by the scanner (exported symbol on libUE4.so, pattern scan on the
//! stripped images).

use super::hooks::install_inline_hook;
use crate::rules;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const CURLOPT_URL: i32 = 10002;

type SetOptFn = unsafe extern "C" fn(*mut libc::c_void, i32, *mut libc::c_void) -> i32;

static ORIG: OnceLock<usize> = OnceLock::new();
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Thread-local buffer for the rewritten URL. curl copies the string
/// inside `curl_easy_setopt`, but the pointer only has to stay valid for
/// the duration of the call.
thread_local! {
    static REWRITE_BUF: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

unsafe extern "C" fn hook_setopt(
    handle: *mut libc::c_void,
    option: i32,
    args: *mut libc::c_void,
) -> i32 {
    let Some(&trampoline) = ORIG.get() else {
        return -1;
    };
    let orig: SetOptFn = std::mem::transmute(trampoline);

    if option == CURLOPT_URL && !args.is_null() {
        if let Ok(url) = CStr::from_ptr(args as *const libc::c_char).to_str() {
            if let Some(new_url) = rules::rewrite(url) {
                return REWRITE_BUF.with(|buf| {
                    let mut b = buf.borrow_mut();
                    b.clear();
                    b.push_str(&new_url);
                    orig(handle, option, b.as_ptr() as *mut libc::c_void)
                });
            }
        }
    }
    orig(handle, option, args)
}

/// Hook `curl_easy_setopt` at `addr` (runtime address = base + VA).
pub fn install(addr: u64) -> Result<(), String> {
    if addr == 0 {
        return Err("curl_easy_setopt address is null".into());
    }
    let hook = install_inline_hook(addr as usize, hook_setopt as usize)?;
    ORIG.set(hook.trampoline).map_err(|_| "curl hook already installed")?;
    INSTALLED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Whether the hook is installed (for status reporting).
pub fn is_installed() -> bool {
    INSTALLED.load(Ordering::SeqCst)
}
