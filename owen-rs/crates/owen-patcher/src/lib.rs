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
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            owen_lib: PathBuf::from("libOwen.so"),
            target_lib: None,
            no_sign: false,
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

    // Sign (v2).
    if !opts.no_sign {
        let key = sign::SigningKey::generate_or_load("owen-key.pem")?;
        sign::sign_apk_v2(output, &key)?;
        report.signed_v2 = true;
        report.signature_verified = sign::verify_v2(output)?;
        if report.signature_verified {
            report.notes.push("APK Signature Scheme v2 verified".into());
        }
    }

    Ok(report)
}
