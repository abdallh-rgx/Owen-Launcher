//! `pack` — the pure-Rust APK packer CLI.
//!
//! ```text
//! cargo run --release -p owen-patcher --bin pack -- [--root DIR] [overrides]
//! ```
//!
//! Resolves every input from the workspace (built `.so`s, assets, data
//! trees, the ogfn signing pair), assembles `dist/OwenLauncher-v1.8.0-release.apk`,
//! signs it v1+v2 with the ORIGINAL ogfn certificate and self-verifies.

use owen_patcher::pack::{self, PackConfig};
use std::path::PathBuf;

const USAGE: &str = "\
usage: pack [options]

  --root DIR             workspace root (default: current directory)
  --out FILE             output APK (default: <root>/dist/OwenLauncher-v1.8.0-release.apk)
  --key PEM              signing key (default: <root>/assets/signing/ogfn-key.pem)
  --cert DER             signing cert (default: <root>/assets/signing/ogfn-cert.der)
  --app-so PATH          eframe app cdylib (default: auto-detect in <root>/target/
                         aarch64-linux-android/release: libowen_app.so, libowenapp.so,
                         libowen-app.so)
  --redirecter-so PATH   redirecter cdylib (default: libowen.so in the same dir)
  --dex PATH             classes.dex (default: <root>/assets/dex/classes.dex)
  --keystore PATH        ogfn-debug.keystore (default: <root>/assets/tools/)
  --seasons-dir DIR      season_N_folder.png dir (default: <repo>/public/owen/seasons)
  --allow-missing-dex    pack without classes.dex (stub smoke builds only)
  --list                 print the resolved configuration and exit
";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(".");
    let mut overrides: Vec<(&str, String)> = Vec::new();
    let mut allow_missing_dex = false;
    let mut list_only = false;
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
        println!("dex              : {}", cfg.dex.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "MISSING".into()));
        println!("keystore         : {}", cfg.keystore.display());
        println!("seasons dir      : {}", cfg.seasons_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "none".into()));
        println!("allow-missing-dex: {}", cfg.allow_missing_dex);
        return;
    }

    println!("pack: assembling {}", cfg.out.display());
    println!("pack: lib_name = {:?} (from {})", cfg.lib_name(), cfg.app_so.display());
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
