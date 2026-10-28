//! # owen-patcher
//!
//! APK patching, the Owen 2.x way: **zero smali edits**.
//!
//! The legacy launcher edited `classes.dex` (smali) to call
//! `System.loadLibrary("Owen")` and to hardcode login/graphql URLs. Owen
//! 2.x replaces both steps with native-level operations:
//!
//! 1. [`needed`] — DT_NEEDED injection. `libOwen.so` is added to the
//!    dynamic dependency list of `libUE4.so` / `libUnreal.so`, so the
//!    Android linker loads it automatically before the game runs. No dex,
//!    no smali, no resources are touched.
//! 2. [`ziprw`] — raw ZIP entry manipulation (add files, replace files,
//!    keep everything else byte-identical).
//! 3. [`sign`] — APK Signature Scheme v2 (RSA-2048 / SHA-256) implemented
//!    in pure Rust.
//! 4. [`manifest`][self::patch_plan] — the high-level
//!    [`patch_apk`] pipeline.
//!
//! The optional legacy smali path (four-finger console) is intentionally
//! NOT implemented here — console opening stays a native concern of the
//! redirecter library itself.
#![forbid(unsafe_code)]


pub mod needed;
pub mod sign;
pub mod ziprw;

use std::path::{Path, PathBuf};

/// Description of one patch operation result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchReport {
    pub input_apk: String,
    pub output_apk: String,
    pub target_lib: Option<String>,
    pub dt_needed_injected: bool,
    pub lib_added: Option<String>,
    pub signed_v2: bool,
    pub signature_verified: bool,
    pub notes: Vec<String>,
}

/// Options for [`patch_apk`].
#[derive(Debug, Clone)]
pub struct PatchOptions {
    /// Path of the libOwen.so to embed (defaults to a placeholder build).
    pub owen_lib: PathBuf,
    /// Which native library to inject the dependency into. `None` = detect
    /// automatically (libUnreal.so first, then libUE4.so).
    pub target_lib: Option<String>,
    /// Skip signing (debug).
    pub no_sign: bool,
    /// Where the signing key lives (generated on first use). Defaults to
    /// `owen-key.pem` in the working directory; the Android launcher passes
    /// a path inside its private files dir so the key survives patches.
    pub key_path: Option<PathBuf>,
    /// Optional package rename (binary AXML string-pool edit). `None` keeps
    /// the original package.
    pub rename_package: Option<String>,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            owen_lib: PathBuf::from("libOwen.so"),
            target_lib: None,
            no_sign: false,
            key_path: None,
            rename_package: None,
        }
    }
}

/// High-level APK patch pipeline.
///
/// 1. Copy the APK.
/// 2. Detect `libUnreal.so` / `libUE4.so` (arm64-v8a) inside.
/// 3. Patch its DT_NEEDED to include `libOwen.so`.
/// 4. Add `lib/arm64-v8a/libOwen.so`.
/// 5. Sign with APK Signature Scheme v2.
pub fn patch_apk(input: &Path, output: &Path, opts: &PatchOptions) -> Result<PatchReport, String> {
    let mut report = PatchReport {
        input_apk: input.display().to_string(),
        output_apk: output.display().to_string(),
        target_lib: None,
        dt_needed_injected: false,
        lib_added: None,
        signed_v2: false,
        signature_verified: false,
        notes: Vec::new(),
    };

    let owen_bytes = std::fs::read(&opts.owen_lib)
        .map_err(|e| format!("cannot read {}: {e}", opts.owen_lib.display()))?;
    if owen_bytes.len() < 64 || &owen_bytes[..4] != b"\x7fELF" {
        return Err(format!("{} is not an ELF library", opts.owen_lib.display()));
    }

    let mut zip = ziprw::ZipArchive::open(input)?;

    // Detect the target library.
    let candidates = ["lib/arm64-v8a/libUnreal.so", "lib/arm64-v8a/libUE4.so"];
    let target: Option<String> = match &opts.target_lib {
        Some(t) => Some(format!("lib/arm64-v8a/{t}")),
        None => candidates
            .iter()
            .find(|c| zip.list().iter().any(|e| e.name.as_str() == **c))
            .map(|s| s.to_string()),
    };
    let Some(target) = target else {
        return Err("no libUnreal.so/libUE4.so found in APK".into());
    };
    report.target_lib = Some(target.clone());

    // Read + patch DT_NEEDED.
    let lib_bytes = zip
        .read_entry(&target)
        .ok_or_else(|| format!("cannot read {target}"))?;
    let (patched_lib, injected) = needed::inject_needed(&lib_bytes, "libOwen.so")?;
    if injected {
        report.dt_needed_injected = true;
        zip.replace_entry(&target, &patched_lib)?;
        report.notes.push(format!("DT_NEEDED injected into {target}"));
    } else {
        // Already present.
        report.notes.push("libOwen.so already in DT_NEEDED".into());
    }

    // Add libOwen.so.
    zip.add_entry("lib/arm64-v8a/libOwen.so", &owen_bytes);
    report.lib_added = Some("lib/arm64-v8a/libOwen.so".into());

    // Ensure the manifest allows cleartext http when present.
    if let Some(mf) = zip.read_entry("AndroidManifest.xml") {
        if mf.windows(2).any(|w| w == [0x98, 0x00]) {
            report.notes.push("binary manifest present; cleartext flag check skipped (min-level edit)".into());
        }
    }

    zip.write_to(output)?;

    // Optional package rename (binary AXML string-pool edit, no rebuild).
    if let Some(new_pkg) = opts.rename_package.clone() {
        let manifest = zip
            .read_entry("AndroidManifest.xml")
            .ok_or("no AndroidManifest.xml to rename")?;
        let renamed = axml_rename_package(&manifest, &new_pkg)?;
        let mut zip2 = ziprw::ZipArchive::open(Path::new(output))?;
        zip2.replace_entry("AndroidManifest.xml", &renamed)?;
        zip2.write_to(output)?;
        report.notes.push(format!("package renamed to {new_pkg}"));
    }

    // Sign (v2).
    if !opts.no_sign {
        let key = sign::SigningKey::generate_or_load(
            opts.key_path
                .as_deref()
                .unwrap_or_else(|| Path::new("owen-key.pem"))
                .to_str()
                .unwrap_or("owen-key.pem"),
        )?;
        sign::sign_apk_v2(output, &key)?;
        report.signed_v2 = true;
        report.signature_verified = sign::verify_v2(output)?;
        if report.signature_verified {
            report.notes.push("APK Signature Scheme v2 verified".into());
        }
    }

    Ok(report)
}

/// Minimal binary-AXML package rename: swap the manifest's package string
/// in the string pool for a same-or-shorter dotted name (in-place, length
/// prefix updated, no re-layout).
fn axml_rename_package(manifest: &[u8], new_package: &str) -> Result<Vec<u8>, String> {
    // String pool header: magic 0x0001/0x0002, then chunk size, string count,
    // style count, flags, strings start, styles start (all u32 LE).
    if manifest.len() < 60 {
        return Err("manifest too small".into());
    }
    let strings_start = u32::from_le_bytes([
        manifest[26], manifest[27], manifest[28], manifest[29],
    ]) as usize;
    let string_count = u32::from_le_bytes([
        manifest[8], manifest[9], manifest[10], manifest[11],
    ]) as usize;
    let mut offsets = Vec::with_capacity(string_count);
    for i in 0..string_count {
        let o = 20 + i * 4;
        if o + 4 > manifest.len() {
            return Err("string pool truncated".into());
        }
        offsets.push(u32::from_le_bytes([
            manifest[o], manifest[o + 1], manifest[o + 2], manifest[o + 3],
        ]) as usize);
    }
    let find_dot_package = |data: &[u8], at: usize| -> Option<String> {
        if at + 2 > data.len() {
            return None;
        }
        let len = u16::from_le_bytes([data[at], data[at + 1]]) as usize;
        if len == 0 || len > 260 || at + 2 + len * 2 > data.len() {
            return None;
        }
        let mut s = String::new();
        for i in 0..len {
            let c = u16::from_le_bytes([data[at + 2 + i * 2], data[at + 3 + i * 2]]);
            if c == 0 || c > 0x7f {
                return None;
            }
            s.push(c as u8 as char);
        }
        if s.split('.').count() >= 2 && !s.contains(' ') {
            Some(s)
        } else {
            None
        }
    };
    let mut data = manifest.to_vec();
    let mut renamed = false;
    for &off in &offsets {
        let at = strings_start + off;
        if let Some(pkg) = find_dot_package(&data, at) {
            if pkg == "com.epicgames.fortnite" {
                let new_bytes = new_package.as_bytes();
                if new_bytes.len() > 260 {
                    return Err("package name too long".into());
                }
                let len_bytes = (new_bytes.len() as u16).to_le_bytes();
                data[at] = len_bytes[0];
                data[at + 1] = len_bytes[1];
                for (i, b) in new_bytes.iter().enumerate() {
                    data[at + 2 + i * 2] = *b;
                    data[at + 3 + i * 2] = 0;
                }
                // Pad the remainder of the old string with NULs.
                let old_len = pkg.len();
                for i in new_bytes.len()..old_len {
                    data[at + 2 + i * 2] = 0;
                    data[at + 3 + i * 2] = 0;
                }
                renamed = true;
                break;
            }
        }
    }
    if !renamed {
        return Err("package string not found in the manifest string pool".into());
    }
    Ok(data)
}
