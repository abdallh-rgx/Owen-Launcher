//! owen-detect CLI — same flags and output shape as the reference Python tool.
//!
//! Usage:
//!   owen-detect <file.so> [--json] [--all] [-n N] [-q]

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut path: Option<&str> = None;
    let mut json = false;
    let mut all = false;
    let mut top: Option<usize> = None;
    let mut quiet = false;
    let mut legacy = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--all" => all = true,
            "-q" | "--quiet" => quiet = true,
            "--legacy" | "--v1" => legacy = true,
            "-n" | "--top" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    top = v.parse().ok();
                }
            }
            other => path = Some(other),
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!("usage: owen-detect <file.so> [--json] [--all] [-n N] [-q] [--legacy]");
        return ExitCode::from(1);
    };

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] cannot read {path}: {e}");
            return ExitCode::from(1);
        }
    };

    let report = if legacy {
        owen_detect::scan_elf_image_legacy(&data)
    } else {
        owen_detect::scan_elf_image(&data)
    };
    let Some(report) = report else {
        eprintln!("[-] not a valid ELF64-LE image");
        return ExitCode::from(1);
    };

    let candidates = report.candidates;
    if candidates.is_empty() {
        eprintln!("[-] no candidate found");
        return ExitCode::from(2);
    }

    let limit = if all {
        candidates.len()
    } else {
        top.unwrap_or(1).min(candidates.len())
    };

    if quiet {
        let best = &candidates[0];
        let target = best.hook_off();
        match owen_detect::off_to_va(&report.segments, target) {
            Some(va) => println!("0x{va:08X}"),
            None => println!("0x{target:08X}"),
        }
        return ExitCode::SUCCESS;
    }

    if json {
        println!("[");
        for (idx, c) in candidates.iter().take(limit).enumerate() {
            let comma = if idx + 1 < limit { "," } else { "" };
            println!(
                "  {{\"score\": {}, \"function_start\": {}, \"cbz_x0\": {}, \"bl\": {{\"off\": {}, \"target\": {}}}, \"mov_w0_2b\": {}, \"gap_cbz_to_mov\": {}}}{}",
                c.score,
                json_off(&report.segments, c.func_start),
                json_off(&report.segments, Some(c.cbz_off)),
                c.bl_off,
                c.bl_target
                    .and_then(|t| owen_detect::off_to_va(&report.segments, t))
                    .map(|v| format!("\"0x{v:08X}\""))
                    .unwrap_or_else(|| "null".into()),
                c.mov_off,
                c.gap,
                comma
            );
        }
        println!("]");
        return ExitCode::SUCCESS;
    }

    println!("[+] candidates: {}\n", candidates.len());
    for (n, c) in candidates.iter().take(limit).enumerate() {
        println!("--- candidate #{} (score {}/10) ---", n + 1, c.score);
        match c.func_start {
            Some(fs) => println!(
                "[*] function start : 0x{fs:08X} (file)   {}",
                fmt_va(&report.segments, fs)
            ),
            None => println!("    [!] function start not located - CBZ used as estimate"),
        }
        println!(
            "    CBZ  X0        : 0x{:08X} (file)   {}",
            c.cbz_off,
            fmt_va(&report.segments, c.cbz_off)
        );
        println!(
            "    BL             : 0x{:08X} (file)   {}",
            c.bl_off,
            fmt_va(&report.segments, c.bl_off)
        );
        if let Some(t) = c.bl_target {
            println!(
                "      -> Curl_vsetopt : 0x{t:08X} (file)   {}",
                fmt_va(&report.segments, t)
            );
        }
        println!(
            "    MOV  W0,#0x2b  : 0x{:08X} (file)   {}",
            c.mov_off,
            fmt_va(&report.segments, c.mov_off)
        );
        println!("    gap CBZ->MOV   : 0x{:X}", c.gap);
        println!();
    }

    let best = &candidates[0];
    println!("{}", "=".repeat(62));
    println!("[+] best match:");
    if let Some(fs) = best.func_start {
        println!("    curl_easy_setopt:");
        println!("        File Offset : 0x{fs:08X}");
        if let Some(va) = owen_detect::off_to_va(&report.segments, fs) {
            println!("        Virtual Addr: 0x{va:08X}");
        }
    } else {
        println!("    [!] function start not located - start from the CBZ");
    }
    println!("{}", "=".repeat(62));
    ExitCode::SUCCESS
}

fn fmt_va(segs: &[owen_detect::Segment], off: usize) -> String {
    match owen_detect::off_to_va(segs, off) {
        Some(va) => format!("0x{va:08X} (VA)"),
        None => "(no VA mapping)".into(),
    }
}

fn json_off(segs: &[owen_detect::Segment], off: Option<usize>) -> String {
    match off {
        None => "null".into(),
        Some(o) => owen_detect::off_to_va(segs, o)
            .map(|v| format!("\"0x{v:08X}\""))
            .unwrap_or_else(|| "null".into()),
    }
}
