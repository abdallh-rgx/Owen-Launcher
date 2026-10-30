//! `pack` — the pure-Rust APK packer CLI.
//!
//! ```text
//! cargo run --release -p owen-patcher --bin pack -- [--root DIR] [overrides]
//! ```
//!
//! Resolves every input from the workspace (built `.so`s, assets, data
//! trees, the ogfn signing pair), assembles `dist/OwenLauncher-v1.9.0-release.apk`,
//! signs it v1+v2 with the ORIGINAL ogfn certificate and self-verifies.
//!
//! The REAL-Android-build-tooling artifacts come in through the `--prebuilt-*`
//! flags (produced by `tools/build-apk.sh` phase 3):
//!
//! ```text
//! aapt2 link   → --prebuilt-manifest (binary AndroidManifest.xml)
//!              → --prebuilt-arsc     (resources.arsc)
//! javac + d8   → --prebuilt-dex      (classes.dex)
//! ```
//!
//! Without them the packer falls back to the handcrafted axmlwriter
//! manifest + empty arsc + `assets/dex/classes.dex` (SDK-less smoke builds).

use owen_patcher::pack::{self, PackConfig};
use std::path::PathBuf;

/// Read one `--prebuilt-*` file eagerly — bad paths fail at parse time with
/// the flag named (before any packing work starts).
fn read_prebuilt(flag: &str, path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{flag} {path}: {e}"))
}

const USAGE: &str = "\
usage: pack [options]

  --root DIR             workspace root (default: current directory)
  --out FILE             output APK (default: <root>/dist/OwenLauncher-v1.9.0-release.apk)
  --key PEM              signing key (default: <root>/assets/signing/ogfn-key.pem)
  --cert DER             signing cert (default: <root>/assets/signing/ogfn-cert.der)
  --app-so PATH          eframe app cdylib (default: auto-detect in <root>/target/
                         aarch64-linux-android/release: libowen_app.so, libowenapp.so,
                         libowen-app.so)
  --redirecter-so PATH   redirecter cdylib (default: libowen.so in the same dir)
  --dex PATH             classes.dex fallback (default: <root>/assets/dex/classes.dex;
                         ignored when --prebuilt-dex is given)
  --keystore PATH        ogfn-debug.keystore (default: <root>/assets/tools/)
  --seasons-dir DIR      season_N_folder.png dir (default: <repo>/public/owen/seasons)

  REAL Android build-tooling inputs (tools/build-apk.sh phase 3) — each
  REPLACES the handcrafted fallback and is validated before it is packed:
  --prebuilt-manifest FILE  binary AndroidManifest.xml from `aapt2 link`
  --prebuilt-arsc FILE      resources.arsc from `aapt2 link`
  --prebuilt-dex FILE       classes.dex from `javac` + `d8 --min-api 24`

  --allow-missing-dex    pack without classes.dex (stub smoke builds only)
  --list                 print the resolved configuration and exit
";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(".");
    let mut overrides: Vec<(&str, String)> = Vec::new();
    let mut allow_missing_dex = false;
    let mut list_only = false;
    let mut prebuilt_manifest: Option<Vec<u8>> = None;
    let mut prebuilt_arsc: Option<Vec<u8>> = None;
    let mut prebuilt_dex: Option<Vec<u8>> = None;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| -> String {
            args.next()
                .unwrap_or_else(|| panic!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--root" => root = PathBuf::from(value("--root")),
            "--out" => overrides.push(("out", value("--out"))),
            "--key" => overrides.push(("key", value("--key"))),
            "--cert" => overrides.push(("cert", value("--cert"))),
            "--app-so" => overrides.push(("app-so", value("--app-so"))),
            "--redirecter-so" => overrides.push(("redirecter-so", value("--redirecter-so"))),
            "--dex" => overrides.push(("dex", value("--dex"))),
            "--keystore" => overrides.push(("keystore", value("--keystore"))),
            "--seasons-dir" => overrides.push(("seasons-dir", value("--seasons-dir"))),
            "--prebuilt-manifest" => {
                let path = value("--prebuilt-manifest");
                prebuilt_manifest = Some(read_prebuilt("--prebuilt-manifest", &path));
            }
            "--prebuilt-arsc" => {
                let path = value("--prebuilt-arsc");
                prebuilt_arsc = Some(read_prebuilt("--prebuilt-arsc", &path));
            }
            "--prebuilt-dex" => {
                let path = value("--prebuilt-dex");
                prebuilt_dex = Some(read_prebuilt("--prebuilt-dex", &path));
            }
            "--allow-missing-dex" => allow_missing_dex = true,
            "--list" => list_only = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return;
            }
            other => panic!("unknown option {other}\n{USAGE}"),
        }
    }

    let root = root
        .canonicalize()
        .unwrap_or_else(|e| panic!("--root {}: {e}", root.display()));
    let mut cfg = PackConfig::resolve(&root).unwrap_or_else(|e| {
        eprintln!("pack: {e}");
        std::process::exit(2);
    });
    cfg.allow_missing_dex = allow_missing_dex;
    cfg.prebuilt_manifest = prebuilt_manifest;
    cfg.prebuilt_arsc = prebuilt_arsc;
    cfg.prebuilt_dex = prebuilt_dex;
    for (name, value) in &overrides {
        let path = PathBuf::from(value);
        match *name {
            "out" => cfg.out = path,
            "key" => cfg.key_pem = path,
            "cert" => cfg.cert_der = path,
            "app-so" => cfg.app_so = path,
            "redirecter-so" => cfg.redirecter_so = path,
            "dex" => cfg.dex = Some(path),
            "keystore" => cfg.keystore = path,
            "seasons-dir" => cfg.seasons_dir = Some(path),
            _ => unreachable!(),
        }
    }

    if list_only {
        println!("root             : {}", cfg.root.display());
        println!("out              : {}", cfg.out.display());
        println!("key / cert       : {} / {}", cfg.key_pem.display(), cfg.cert_der.display());
        println!("app .so          : {}  (lib_name: {})", cfg.app_so.display(), cfg.lib_name());
        println!("app so entry     : {}", cfg.app_so_entry());
        println!("redirecter .so   : {}", cfg.redirecter_so.display());
        let dex = cfg
            .prebuilt_dex
            .as_ref()
            .map(|b| format!("prebuilt ({} bytes)", b.len()))
            .or_else(|| {
                cfg.dex
                    .as_ref()
                    .map(|p| p.display().to_string())
            })
            .unwrap_or_else(|| "MISSING".into());
        println!("dex              : {dex}");
        println!(
            "prebuilt manifest: {}",
            cfg.prebuilt_manifest
                .as_ref()
                .map(|b| format!("{} bytes (binary AXML)", b.len()))
                .unwrap_or_else(|| "no — axmlwriter fallback".into())
        );
        println!(
            "prebuilt arsc    : {}",
            cfg.prebuilt_arsc
                .as_ref()
                .map(|b| format!("{} bytes", b.len()))
                .unwrap_or_else(|| "no — empty 12-byte stub".into())
        );
        println!("keystore         : {}", cfg.keystore.display());
        println!("seasons dir      : {}", cfg.seasons_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "none".into()));
        println!("allow-missing-dex: {}", cfg.allow_missing_dex);
        return;
    }

    println!("pack: assembling {}", cfg.out.display());
    println!("pack: lib_name = {:?} (from {})", cfg.lib_name(), cfg.app_so.display());
    if cfg.prebuilt_manifest.is_some() {
        println!("pack: manifest/arsc/dex = aapt2+javac+d8 prebuilts (REAL build tooling)");
    } else {
        println!("pack: WARNING — no prebuilt manifest; using the handcrafted axmlwriter");
        println!("pack:           fallback (NOT device-verified; pass --prebuilt-manifest)");
    }
    let report = match pack::pack(&cfg) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("pack: FAILED: {e}");
            std::process::exit(1);
        }
    };
    for note in &report.notes {
        println!("pack: {note}");
    }
    if !report.alignment.is_empty() {
        for problem in &report.alignment {
            eprintln!("pack: ALIGNMENT PROBLEM: {problem}");
        }
        std::process::exit(1);
    }
    println!(
        "pack: OK — {} entries, dex={}, verify_v1={}, verify_v2={}, alignment=clean",
        report.entries, report.dex_included, report.verify_v1, report.verify_v2
    );
    println!(
        "pack: {} ({} bytes)",
        report.apk.display(),
        std::fs::metadata(&report.apk).map(|m| m.len()).unwrap_or(0)
    );
}
