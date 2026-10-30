//! build.rs for owen-ogs — compiles the C++ port (cpp/) together with the
//! MobileDumper-7 SDK dump of the target game into the cdylib.
//!
//! Gated on the `OGS_SDK_DIR` environment variable, which must point at the
//! DUMP ROOT — the directory that either contains `CppSDK/` (MobileDumper-7
//! layout) or directly contains `SDK.hpp` (already-unpacked layout):
//!
//! ```text
//! OGS_SDK_DIR=/path/to/dump              # contains CppSDK/SDK.hpp
//! OGS_SDK_DIR=/path/to/dump/CppSDK       # contains SDK.hpp directly
//! ```
//!
//! When OGS_SDK_DIR is NOT set the crate builds as a log-only stub
//! (`cargo check --workspace` stays green without the dump or the NDK);
//! src/lib.rs then compiles the `stub` module instead of the `full` module.
//!
//! What happens when it IS set:
//!   1. resolve include root (root) + its parent (parent) so both
//!      `#include "SDK.hpp"` and `#include "SDK/SDK.hpp"` resolve;
//!   2. read offsets/ogs-1241-android.json and generate
//!      $OUT_DIR/OgsOffsetsGen.h (one `OGSOFF_<key>_RVA/_FOUND` pair per
//!      scanner entry — see cpp/OgsCompat.h for the full key contract);
//!   3. compile every `$ROOT/SDK/*_functions.cpp` (~1300 files) with the NDK
//!      clang++ (-O1 -std=c++20 -fexceptions), in parallel over
//!      available_parallelism(), plus the mod TUs (cpp/ogs_all.cpp -O2,
//!      cpp/ogshook.cpp, cpp/ogscompat.cpp -O2);
//!   4. archive everything into libogs_mod.a and link it into the cdylib
//!      (owen_hook is linked from the Rust side via the owen-hook dep);
//!   5. emit `--cfg ogs_cpp` so src/lib.rs activates the real boot path.
//!
//! The NDK toolchain is located exactly like tools/build-apk.sh does:
//! `$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn main() {
    let sdk_dir = match env::var("OGS_SDK_DIR") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            println!(
                "cargo:warning=owen-ogs: OGS_SDK_DIR not set - building the log-only stub \
                 (set it to the dump root containing CppSDK/ or SDK.hpp to build the mod)"
            );
            return;
        }
    };

    // --- (a) resolve include root + parent --------------------------------
    let root = if sdk_dir.join("CppSDK").join("SDK.hpp").is_file() {
        sdk_dir.join("CppSDK")
    } else if sdk_dir.join("SDK.hpp").is_file() {
        sdk_dir.clone()
    } else {
        panic!(
            "owen-ogs: OGS_SDK_DIR={} contains neither CppSDK/SDK.hpp nor SDK.hpp",
            sdk_dir.display()
        );
    };
    let parent = root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.clone());

    // --- (b) generate OgsOffsetsGen.h from the scanner JSON ---------------
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let offsets_json = manifest
        .join("..")
        .join("..")
        .join("offsets")
        .join("ogs-1241-android.json");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let gen_header = out_dir.join("OgsOffsetsGen.h");
    write_offsets_header(&offsets_json, &gen_header);

    // --- toolchain ---------------------------------------------------------
    let ndk = env::var("ANDROID_NDK_HOME").unwrap_or_else(|_| {
        panic!("owen-ogs: ANDROID_NDK_HOME must be set (same requirement as tools/build-apk.sh)")
    });
    let bin = Path::new(&ndk)
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join("linux-x86_64")
        .join("bin");
    let clangxx = bin.join("aarch64-linux-android24-clang++");
    let ar = bin.join("llvm-ar");
    if !clangxx.is_file() {
        panic!("owen-ogs: NDK clang++ not found at {}", clangxx.display());
    }

    let cpp = manifest.join("cpp");

    // --- (c) compile the mod TUs -------------------------------------------
    let mod_units: [(&str, &str); 3] = [
        ("ogs_all.cpp", "-O2"),   // the ported mod: hot paths
        ("ogshook.cpp", "-O2"),   // hook facade: detour trampolines
        ("ogscompat.cpp", "-O2"), // compat layer
    ];
    let mut all_objs: Vec<PathBuf> = Vec::new();
    for (src, opt) in mod_units {
        let obj = out_dir.join(src.replace(".cpp", ".og.o"));
        let mut cmd = Command::new(&clangxx);
        cmd.arg(opt)
            .arg(format!("-I{}", parent.display()))
            .arg(format!("-I{}", root.display()))
            .arg(format!("-I{}", out_dir.display()))
            .arg("-std=c++20")
            .arg("-fexceptions")
            .arg("-fno-rtti") // UE SDK + mod are RTTI-free; keeps size down
            .arg("-fvisibility=hidden")
            .arg("-DANDROID")
            .arg("-c")
            .arg(cpp.join(src))
            .arg(format!("-o{}", obj.display()));
        run(&mut cmd, &format!("compile {src}"));
        all_objs.push(obj);
    }

    // --- compile the SDK *_functions.cpp in parallel ------------------------
    let mut sdk_sources: Vec<PathBuf> = match root.join("SDK").read_dir() {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().ends_with("_functions.cpp"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => panic!("owen-ogs: cannot read {} — {e}", root.join("SDK").display()),
    };
    sdk_sources.sort();
    println!(
        "cargo:warning=owen-ogs: compiling {} SDK translation units",
        sdk_sources.len()
    );

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(sdk_sources.len().max(1));
    let next = AtomicUsize::new(0);
    let any_failed = AtomicBool::new(false);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= sdk_sources.len() {
                    break;
                }
                let src = &sdk_sources[i];
                let obj = out_dir.join(format!("sdk.{i}.o"));
                let ok = Command::new(&clangxx)
                    .arg("-O1")
                    .arg(format!("-I{}", parent.display()))
                    .arg(format!("-I{}", root.display()))
                    .arg(format!("-I{}", out_dir.display()))
                    .arg("-std=c++20")
                    .arg("-fexceptions")
                    .arg("-fno-rtti")
                    .arg("-w") // SDK layout asserts are verified by the dumper
                    .arg("-c")
                    .arg(src)
                    .arg(format!("-o{}", obj.display()))
                    .status()
                    .map(|st| st.success())
                    .unwrap_or(false);
                if !ok {
                    eprintln!(
                        "owen-ogs: FAILED to compile SDK unit {}",
                        src.display()
                    );
                    any_failed.store(true, Ordering::SeqCst);
                    break;
                }
            });
        }
    });
    if any_failed.load(Ordering::SeqCst) {
        panic!("owen-ogs: SDK compilation failed (see stderr above)");
    }
    for i in 0..sdk_sources.len() {
        all_objs.push(out_dir.join(format!("sdk.{i}.o")));
    }

    // --- (d) archive + link directives -------------------------------------
    let archive = out_dir.join("libogs_mod.a");
    let _ = fs::remove_file(&archive);
    let mut cmd = Command::new(&ar);
    cmd.arg("rcs").arg(&archive).args(&all_objs);
    run(&mut cmd, "archive libogs_mod.a");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=ogs_mod");
    println!("cargo:rustc-link-lib=c++_static");
    println!("cargo:rustc-link-arg=-landroid");
    println!("cargo:rustc-link-arg=-llog");
    println!("cargo:rustc-cfg=ogs_cpp");
    println!("cargo:rerun-if-changed={}", cpp.display());
    println!("cargo:rerun-if-changed={}", offsets_json.display());
    println!("cargo:rerun-if-env-changed=OGS_SDK_DIR");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
}

fn run(cmd: &mut Command, what: &str) {
    match cmd.status() {
        Ok(st) if st.success() => {}
        Ok(st) => panic!("owen-ogs: {what} failed with {st}"),
        Err(e) => panic!("owen-ogs: {what} could not be started: {e}"),
    }
}

/// Generate $OUT_DIR/OgsOffsetsGen.h from the scanner JSON.  Every entry
/// under "entries" emits exactly two macros; keys the scanner does not know
/// about fall back to the `#ifndef` defaults documented in OgsCompat.h.
fn write_offsets_header(json_path: &Path, out: &Path) {
    let text = fs::read_to_string(json_path)
        .unwrap_or_else(|e| panic!("owen-ogs: cannot read {} — {e}", json_path.display()));
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("owen-ogs: {} is not valid JSON — {e}", json_path.display()));
    let mut body = String::new();
    body.push_str("// GENERATED by owen-ogs build.rs — do not edit.\n");
    body.push_str("// Source: offsets/ogs-1241-android.json (tools/ogs/find_globals.py)\n");
    body.push_str("// Keys are the JSON keys verbatim; OgsCompat.h fills the mod-only keys.\n");
    body.push_str("#pragma once\n");
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("owen-ogs: {} has no \"entries\" object", json_path.display()));
    for (key, val) in entries {
        let found = val.get("found").and_then(|f| f.as_bool()).unwrap_or(false);
        let rva = val
            .get("rva")
            .and_then(|r| r.as_str())
            .and_then(|r| u64::from_str_radix(r.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        // sanitize the key the same way OgsCompat.h names fields
        let safe: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
            .collect();
        body.push_str(&format!(
            "#define OGSOFF_{safe}_FOUND {}\n",
            if found { 1 } else { 0 }
        ));
        body.push_str(&format!("#define OGSOFF_{safe}_RVA 0x{rva:x}\n"));
        // surface found offsets to the Rust side (cfg!(ogs_off_found_<key>))
        // so the built-in defaults can be "on iff the offset exists".
        if found {
            println!("cargo:rustc-cfg=ogs_off_found_{safe}");
        }
    }
    fs::write(out, body).unwrap();
}
