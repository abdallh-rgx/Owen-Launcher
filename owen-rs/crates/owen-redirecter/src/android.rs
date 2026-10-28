//! Android entry point.
//!
//! libOwen is loaded by the dynamic linker via DT_NEEDED injection, so
//! there is no `System.loadLibrary("Owen")` call — which also means no
//! `JNI_OnLoad`. A `.init_array` constructor starts the worker thread.

use crate::detect::CurlSource;
use crate::{detect, procmaps, redirect};
use jni::sys::JavaVM;
use std::ffi::CString;
use std::time::Duration;

/// Report the detected version to the backend (best effort, non-blocking).
fn report(version: &str, curl: CurlSource) {
    let url = format!(
        "{}/api/owen/hello?version={}&curl={}&lib={}",
        crate::rules::backend_url(),
        urlencode(version),
        match curl {
            CurlSource::ExportedSymbol => "exported",
            CurlSource::AutoScan => "autoscanned",
            CurlSource::None => "none",
        },
        urlencode(env!("CARGO_PKG_VERSION"))
    );
    std::thread::spawn(move || {
        // Minimal blocking HTTP GET via libc socket (no TLS; backends on
        // 127.0.0.1 are plain http). Failures are ignored.
        let _ = http_get(&url);
    });
}

fn http_get(url: &str) -> Option<String> {
    // Parse http://host:port/path
    let rest = url.strip_prefix("http://")?;
    let (hostport, path) = rest.split_once('/')?;
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().ok()?),
        None => (hostport, 80),
    };
    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if fd < 0 {
            return None;
        }
        let host_c = CString::new(host).ok()?;
        let he = libc::gethostbyname(host_c.as_ptr());
        if he.is_null() {
            libc::close(fd);
            return None;
        }
        // h_addr_list[0] -> 4 bytes of IPv4
        let addr_list = (*he).h_addr_list;
        if addr_list.is_null() || (*addr_list).is_null() {
            libc::close(fd);
            return None;
        }
        let mut addr_bytes = [0u8; 4];
        std::ptr::copy_nonoverlapping(*addr_list as *const u8, addr_bytes.as_mut_ptr(), 4);
        let addr_in: libc::sockaddr_in = std::mem::zeroed();
        let mut addr_in = addr_in;
        addr_in.sin_family = libc::AF_INET as libc::sa_family_t;
        addr_in.sin_port = port.to_be();
        addr_in.sin_addr = std::mem::transmute::<[u8; 4], libc::in_addr_t>(addr_bytes);
        let connect_result = libc::connect(
            fd,
            &addr_in as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        );
        if connect_result != 0 {
            libc::close(fd);
            return None;
        }
        let req = format!("GET /{path} HTTP/1.0\r\nHost: {hostport}\r\nConnection: close\r\n\r\n");
        let req_c = CString::new(req).ok()?;
        libc::send(fd, req_c.as_ptr(), req_c.to_bytes().len(), 0);
        let mut response = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);
            if n <= 0 {
                break;
            }
            response.extend_from_slice(&buf[..n as usize]);
        }
        libc::close(fd);
        Some(String::from_utf8_lossy(&response).into_owned())
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The worker: wait for the engine, detect, hook, report.
fn worker() {
    // 1. Wait for the engine library (up to 60s).
    let module = loop {
        if let Some(m) = procmaps::find_module("libUnreal.so") {
            break m;
        }
        if let Some(m) = procmaps::find_module("libUE4.so") {
            break m;
        }
        std::thread::sleep(Duration::from_millis(200));
    };

    // 2. Let the engine finish initializing its own constructors.
    std::thread::sleep(Duration::from_millis(1200));

    // 3. Detect version + curl address from the on-disk image.
    let detection = detect::detect(&module);

    // 4. Native curl hook.
    if let Some(d) = &detection {
        if let Some(curl_addr) = d.curl_setopt {
            let _ = redirect::curl::install(curl_addr);
        }
    }

    // 5. libEOSSDK.so has its own statically linked curl.
    if let Some(eos) = procmaps::find_module("libEOSSDK.so") {
        if let Some(d) = detect::detect(&eos) {
            if let Some(curl_addr) = d.curl_setopt {
                let _ = redirect::curl::install(curl_addr);
            }
        }
    }

    // 6. ART hooks (Kairos era + WebView safety net).
    if detection.as_ref().map(|d| d.wants_art_hooks()).unwrap_or(true) {
        if let Some(vm) = get_java_vm() {
            install_art_hooks(&vm);
        }
    }

    // 7. Report to the backend.
    if let Some(d) = &detection {
        report(&d.version.release_string, d.curl_source);
    }
}

fn install_art_hooks(vm: &JavaVM) {
    let mut env = match vm.get_env() {
        Ok(e) => e,
        Err(_) => return,
    };
    // WebView safety net first (all versions).
    let _ = redirect::webview::install(&mut env);
    // Kairos URL builders (10.31-15.50).
    let _ = redirect::kairos::install(&mut env);
}

fn get_java_vm() -> Option<JavaVM> {
    // JNI_GetCreatedJavaVMs from libart.so.
    unsafe {
        let name = CString::new("JNI_GetCreatedJavaVMs").ok()?;
        let handle = libc::dlopen(
            CString::new("libart.so").ok()?.as_ptr(),
            libc::RTLD_NOW | libc::RTLD_NOLOAD,
        );
        if handle.is_null() {
            return None;
        }
        let sym = libc::dlsym(handle, name.as_ptr());
        if sym.is_null() {
            return None;
        }
        type GetVMs = unsafe extern "C" fn(*mut *mut JavaVM, isize, *mut isize) -> i32;
        let get_vms: GetVMs = std::mem::transmute(sym);
        let mut vm_ptr: *mut JavaVM = std::ptr::null_mut();
        let mut count: isize = 0;
        if get_vms(&mut vm_ptr, 1, &mut count) != 0 || count == 0 {
            return None;
        }
        Some(vm_ptr.read())
    }
}

/// `.init_array` constructor: start the worker thread. The attribute
/// places this function pointer into the library's init array so the
/// dynamic linker invokes it when the library is mapped — the DT_NEEDED
/// loading path.
#[used]
#[link_section = ".init_array"]
static INIT_ARRAY: extern "C" fn() = owen_init;

extern "C" fn owen_init() {
    std::thread::spawn(worker);
}
