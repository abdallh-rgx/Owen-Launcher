//! `shim` — CLI for the shim injection pipeline.
//!
//! ```text
//! cargo run --release -p owen-patcher --bin shim -- \
//!     --in  dist/game/fortnite-12.41.apk \
//!     --out dist/game/fortnite-12.41-owen.apk \
//!     --shim target/shim/libUE4shim.so \
//!     --mod libOwen.so=target/aarch64-linux-android/release/libowen.so \
//!     --mod libOwenOGS.so=ci-out/libOwenOGS.so \
//!     [--key dist/keys/owen-key.pem] [--min-bytes 2000000]
//! ```
//!
//! Prints the PatchReport as JSON on success. Exit code 2 = PatchFailure.

use owen_patcher::shim::{shim_patch_apk, ShimOptions};
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!(
        "usage: shim --in APK --out APK --shim SO [--mod NAME=PATH ...] [--key PEM] [--min-bytes N]"
    );
    std::process::exit(64)
}

fn main() -> ExitCode {
    let mut in_apk: Option<PathBuf> = None;
    let mut out_apk: Option<PathBuf> = None;
    let mut shim: Option<PathBuf> = None;
    let mut mods: Vec<(String, PathBuf)> = Vec::new();
    let mut key: Option<PathBuf> = None;
    let mut min_bytes: u64 = 2_000_000;

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--in" => in_apk = Some(it.next().map(PathBuf::from).unwrap_or_else(|| usage())),
            "--out" => out_apk = Some(it.next().map(PathBuf::from).unwrap_or_else(|| usage())),
            "--shim" => shim = Some(it.next().map(PathBuf::from).unwrap_or_else(|| usage())),
            "--mod" => {
                let Some(spec) = it.next() else { usage() };
                let Some((name, path)) = spec.split_once('=') else { usage() };
                mods.push((name.to_string(), PathBuf::from(path)));
            }
            "--key" => key = Some(it.next().map(PathBuf::from).unwrap_or_else(|| usage())),
            "--min-bytes" => {
                min_bytes = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| usage())
            }
            _ => usage(),
        }
    }

    let (Some(in_apk), Some(out_apk), Some(shim)) = (in_apk, out_apk, shim) else { usage() };

    let opts = ShimOptions {
        shim_lib: shim,
        mods,
        key_path: key,
        min_output_bytes: min_bytes,
    };
    match shim_patch_apk(&in_apk, &out_apk, &opts) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::SUCCESS
        }
        Err(f) => {
            eprintln!("shim patch failed at `{}`: {}", f.stage, f.detail);
            ExitCode::from(2)
        }
    }
}
