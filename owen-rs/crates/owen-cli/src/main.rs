//! Owen CLI — the host-side toolbelt.
//!
//! ```text
//! owen-cli scan  <lib.so>...     automatic curl_easy_setopt detection
//! owen-cli scan --json <lib.so>  machine-readable output
//! owen-cli pak   <file.pak>      parse a pak index and list entries
//! owen-cli pak   <file.pak> --cosmetics [--version 12.50]
//! owen-cli manifest <cloudcontent.json | apk-dir>  build manifest entries
//! ```

use owen_core::battlepass;
use owen_core::manifest::{CloudContent, ManifestCatalog};
use owen_core::pak;
use owen_core::version::{detect_from_bytes, FortniteVersion};
use owen_core::{scan_path, ScanReport};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("scan") => cmd_scan(&args[1..]),
        Some("pak") => cmd_pak(&args[1..]),
        Some("manifest") => cmd_manifest(&args[1..]),
        Some("serve") => {
            owen_server::run_server_from_args(&args[1..]);
            ExitCode::SUCCESS
        }
        Some("--version" | "-V" | "version") => {
            println!("owen-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            print_help();
            if args.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn print_help() {
    println!("owen-cli {} — Owen Launcher toolbelt (safe Rust)", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  owen-cli scan [--json] [--all] <lib.so>...");
    println!("      Automatic curl_easy_setopt detection in any stripped AArch64 ELF");
    println!("      (libUE4.so, libUnreal.so, libEOSSDK.so).");
    println!("  owen-cli pak [--cosmetics] [--json] [--version <ver>] <file.pak>");
    println!("      Parse an Unreal pak index; optionally extract Battle Pass cosmetics.");
    println!("  owen-cli manifest <cloudcontent.json | extracted-apk-dir>");
    println!("      Build auto-redirect manifest catalog entries.");
    println!("  owen-cli serve [--port N] [--backend-url URL]");
    println!("      Run the local Owen backend server.");
}

fn cmd_scan(args: &[String]) -> ExitCode {
    let mut json = false;
    let mut all = false;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            "--all" => all = true,
            _ => files.push(a.clone()),
        }
    }
    if files.is_empty() {
        eprintln!("scan: no input files");
        return ExitCode::FAILURE;
    }
    let mut ok = true;
    for f in &files {
        let report: ScanReport = match scan_path(&PathBuf::from(f)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[{f}] error: {e}");
                ok = false;
                continue;
            }
        };
        if json {
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            continue;
        }
        println!("=== {f} ===");
        if let Some(v) = &report.version {
            println!("  version           : {v}");
        }
        if let Some(sym) = report.exported_symbol_va {
            println!("  exported symbol   : curl_easy_setopt @ VA {sym:#010x}");
        }
        println!("  candidates        : {}", report.candidates.len());
        let limit = if all { report.candidates.len() } else { report.candidates.len().min(3) };
        for (i, c) in report.candidates.iter().take(limit).enumerate() {
            println!(
                "  candidate #{} (score {}/10): start={:#010x} cbz={:#010x} bl={:#010x} mov={:#010x} gap={:#x}",
                i + 1,
                c.score,
                c.function_start_va.or(c.cbz_va).unwrap_or(0),
                c.cbz_va.unwrap_or(0),
                c.bl_va.unwrap_or(0),
                c.mov_va.unwrap_or(0),
                c.gap_cbz_to_mov
            );
        }
        match report.best_va {
            Some(va) => println!("  BEST (virtual address): {va:#010x}"),
            None => {
                println!("  BEST: not found");
                ok = false;
            }
        }
        if let Some(off) = report.best_file_offset {
            println!("  BEST (file offset)    : {off:#010x}");
        }
        println!();
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_pak(args: &[String]) -> ExitCode {
    let mut cosmetics = false;
    let mut json = false;
    let mut version_arg: Option<String> = None;
    let mut key_arg: Option<String> = None;
    let mut files = Vec::new();
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--cosmetics" => cosmetics = true,
            "--json" => json = true,
            "--version" => {
                version_arg = iter.next().cloned();
            }
            "--key" | "-k" => {
                key_arg = iter.next().cloned();
            }
            _ => files.push(a.clone()),
        }
    }
    let Some(path) = files.first().cloned() else {
        eprintln!("pak: no input file");
        return ExitCode::FAILURE;
    };
    let key: Option<[u8; 32]> = key_arg.and_then(|k| parse_key(&k));
    let index = match pak::parse_path_with_key(&PathBuf::from(&path), key.as_ref()) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("[{path}] pak error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let version = version_arg
        .and_then(|v| parse_version(&v))
        .or_else(|| guess_version_from_name(&path));
    if cosmetics {
        let Some(v) = version else {
            eprintln!("pak: --cosmetics needs a version (--version 12.50 or a filename containing it)");
            return ExitCode::FAILURE;
        };
        let bp = battlepass::extract(&index, &v);
        if json {
            println!("{}", serde_json::to_string_pretty(&bp).unwrap_or_default());
        } else {
            println!("{} ({}) — {} cosmetic items out of {} files", bp.title, bp.version, bp.items.len(), bp.total_files);
            for item in &bp.items {
                println!("  [{}] {}", item.kind.label(), item.id);
            }
        }
        return ExitCode::SUCCESS;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&index).unwrap_or_default());
    } else {
        println!("mount point: {}", index.mount_point);
        println!("entries    : {}", index.entries.len());
        for e in index.entries.iter().take(40) {
            println!("  {} ({} bytes, {})", e.name, e.size, e.compression_method_name);
        }
        if index.entries.len() > 40 {
            println!("  ... and {} more", index.entries.len() - 40);
        }
    }
    ExitCode::SUCCESS
}

fn cmd_manifest(args: &[String]) -> ExitCode {
    let Some(path) = args.first().cloned() else {
        eprintln!("manifest: no input");
        return ExitCode::FAILURE;
    };
    let p = PathBuf::from(&path);
    let mut catalog = ManifestCatalog::new();
    let cc = if p.is_dir() {
        CloudContent::from_apk_dir(&p)
    } else {
        std::fs::read(&p).ok().and_then(|d| CloudContent::from_json(&d).ok())
    };
    match cc {
        Some(cc) => {
            catalog.add_cloud_content(cc);
            println!("{}", serde_json::to_string_pretty(&catalog).unwrap_or_default());
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("manifest: could not read cloudcontent.json from {path}");
            ExitCode::FAILURE
        }
    }
}

fn parse_key(s: &str) -> Option<[u8; 32]> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let hex = hex.trim_start_matches("0x");
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn parse_version(s: &str) -> Option<FortniteVersion> {
    let release = format!("++Fortnite+Release-{s}");
    detect_from_bytes(release.as_bytes())
}

fn guess_version_from_name(path: &str) -> Option<FortniteVersion> {
    // Look for something like 12.50 in the path.
    let mut best: Option<FortniteVersion> = None;
    let mut parts = path.split(|c: char| !c.is_ascii_digit() && c != '.').filter(|s| !s.is_empty());
    while let Some(p) = parts.next() {
        if let Some(v) = parse_version(p) {
            best = Some(v);
        }
    }
    best
}
