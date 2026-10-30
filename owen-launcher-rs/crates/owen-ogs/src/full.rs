//! Full build path (C++ mod compiled in via `--cfg ogs_cpp`).
//!
//! Boot flow (mirrors crates/owen-redirecter/src/lib.rs):
//! `.init_array` → idempotent `ogs_early_init` → worker thread →
//! config assembly (serde_json) → `ogs_main(config_json)` (C++).

use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

// Shared logging + ogsen.json discovery (candidate paths mirror
// crates/owen-redirecter/src/config.rs) live in `crate::common`.
pub use crate::common::{ogs_config, ogs_log};

static STARTED: AtomicBool = AtomicBool::new(false);

/// dlopen-time constructor — fires before JNI_OnLoad/UE init.
#[used]
#[link_section = ".init_array"]
static INIT: extern "C" fn() = ogs_early_init;

extern "C" fn ogs_early_init() {
    start();
}

#[no_mangle]
pub extern "C" fn JNI_OnLoad(_vm: *mut std::ffi::c_void, _reserved: *mut std::ffi::c_void) -> i32 {
    start();
    0x0001_0006 // JNI_VERSION_1_6
}

/// The C++ entry point (cpp/ogs_all.cpp).  Receives the merged config JSON.
extern "C" {
    fn ogs_main(config_json: *const c_char);
}

fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("owen-ogs-main".into())
        .spawn(worker)
        .ok();
}

fn worker() {
    let (json, _path) = ogs_config::effective_config_json();
    ogs_log::info("owen-ogs: handing config to ogs_main");
    let cstr = std::ffi::CString::new(json.replace('\0', " "))
        .unwrap_or_else(|_| std::ffi::CString::new("{}").unwrap());
    // SAFETY: ogs_main takes a NUL-terminated read-only string and copies
    // everything it needs before returning.
    unsafe { ogs_main(cstr.as_ptr()) };
    ogs_log::info("owen-ogs: ogs_main returned (boot thread exiting)");
}
