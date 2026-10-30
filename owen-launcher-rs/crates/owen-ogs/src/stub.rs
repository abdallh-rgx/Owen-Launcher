//! Stub build path (no `--cfg ogs_cpp`, i.e. OGS_SDK_DIR was not set when
//! the crate was built).  Log-only: the .init_array constructor is still
//! registered so dlopen behaviour matches the full build, but the worker
//! thread only logs and returns.

use std::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);

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

fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("owen-ogs-stub".into())
        .spawn(|| {
            #[cfg(target_os = "android")]
            {
                extern "C" {
                    fn __android_log_print(
                        prio: i32,
                        tag: *const u8,
                        fmt: *const u8,
                        ...
                    ) -> i32;
                }
                let tag = b"OwenOGS\0";
                let fmt = b"%s\0";
                let msg = b"owen-ogs: stub build (OGS_SDK_DIR was not set) - nothing to do\0";
                unsafe {
                    __android_log_print(3, tag.as_ptr(), fmt.as_ptr(), msg.as_ptr());
                }
            }
            #[cfg(not(target_os = "android"))]
            eprintln!("owen-ogs: stub build (OGS_SDK_DIR was not set) - nothing to do");
        })
        .ok();
}
