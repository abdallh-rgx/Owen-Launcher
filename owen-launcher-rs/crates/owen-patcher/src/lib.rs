//! # owen-patcher
//!
//! APK patching, the Owen 2.x way: **zero smali edits** — the literal port
//! of the Owen-Launcher Kotlin pipeline (`ApkPatcher.kt` + friends).
//!
//! ## Kotlin source map
//!
//! | Kotlin (`ApkPatcher.kt`)                  | Rust (here)                                   |
//! |-------------------------------------------|-----------------------------------------------|
//! | `ORIGINAL_PACKAGE`                        | [`ORIGINAL_PACKAGE`]                          |
//! | `OWEN_LIB_ENTRY`                          | [`OWEN_LIB_ENTRY`]                            |
//! | `SKIP_META`                               | [`ziprw::ZipArchive::remove_meta_inf`]        |
//! | `PatchOutcome.Success/Failure(stage, detail)` | [`PatchReport`] / [`PatchFailure`]        |
//! | `patchApk(context, input, backendUrl, loginUrl, isPartyHub, owenLib, targetPackage, isDev, majorVersion, onStatus)` | [`patch_apk_with_plan`] + [`patch_plan::PatchPlan`] |
//! | `rewriteManifest` + `ManifestRewrite`     | [`rewrite_manifest`] + [`ManifestRewrite`]    |
//! | `renameAndSign`                           | [`rename_and_sign`]                           |
//! | `swapLibAndSign`                          | [`swap_lib_and_sign`]                         |
//! | `apkHasOwenLib`                           | [`apk_has_owen_lib`]                          |
//! | `onStatus("…")` step names                | [`PatchReport::steps`] (verbatim strings)     |
//! | `ManifestRenamer.kt`                      | [`manifest_renamer`]                          |
//! | `ManifestFixer.kt`                        | [`manifest_fixer`]                            |
//! | `ApkSignerTool.kt` (v1+v2)                | [`sign_v1`] + [`sign`]                        |
//! | `PatchCache.kt`                           | [`patch_cache`]                               |
//! | `OwenManager.kt`                          | [`owen_manager`]                              |
//! | `NativeBuilder.kt`                        | [`native_builder`]                            |
//! | `SmaliPatcher.kt` + `DexTools.kt` (smali) | [`patch_plan`] (zero-smali plan + owen.json)  |
//! | `RawZip.kt` (repack/alignment)            | [`ziprw`]                                     |
//!
//! The pipeline order is identical to `ApkPatcher.patchApk`:
//! (dex extract→disassemble→inject→console→partyhub→assemble — all zero-smali
//! no-ops except the DT_NEEDED injection) → manifest fix+rename → raw zip
//! repack with STORED alignment → `lib/arm64-v8a/libOwen.so` embedding →
//! v1+v2 signing. Every step is recorded in [`PatchReport::steps`] under its
//! original Kotlin status string.
//!
//! The legacy smali path (baksmali/smali in-process, four-finger console,
//! PartyHub URL rewrites) is intentionally replaced by runtime mechanisms:
//! DT_NEEDED injection instead of `System.loadLibrary`, ART hooks driven by
//! `owen.json` instead of `GameActivity.smali` edits — see [`patch_plan`].
#![forbid(unsafe_code)]

pub mod manifest_fixer;
pub mod manifest_renamer;
pub mod native_builder;
pub mod needed;
pub mod owen_manager;
pub mod pack;
pub mod patch_cache;
pub mod patch_plan;
pub mod shim;
pub mod sign;
pub mod sign_v1;
pub mod ziprw;

use patch_plan::PatchPlan;
use std::path::{Path, PathBuf};

/// `ApkPatcher.ORIGINAL_PACKAGE`.
pub const ORIGINAL_PACKAGE: &str = "com.epicgames.fortnite";
/// `ApkPatcher.OWEN_LIB_ENTRY` (== `OwenManager.OWEN_LIB_ENTRY`).
pub const OWEN_LIB_ENTRY: &str = "lib/arm64-v8a/libOwen.so";
/// `ApkPatcher.kt`'s literal `2_000_000` byte floor for a repacked APK
/// ("Repacking produced no APK").
pub const MIN_UNSIGNED_APK_BYTES: u64 = 2_000_000;

/* ---- ApkPatcher.kt onStatus strings, kept verbatim for the report ---- */
pub const STEP_EXTRACT: &str = "Extracting classes.dex...";
pub const STEP_DISASSEMBLE: &str = "Disassembling dex...";
pub const STEP_INJECT: &str = "Injecting Owen loader...";
pub const STEP_CONSOLE: &str = "Patching console window (4-finger touch)...";
pub const STEP_PARTYHUB: &str = "Patching PartyHub endpoints...";
pub const STEP_ASSEMBLE: &str = "Rebuilding dex...";
pub const STEP_REPACK: &str = "Repacking APK...";
pub const STEP_SIGN: &str = "Signing APK...";

/// Description of one patch operation result — the Rust `PatchOutcome.Success`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchReport {
    pub input_apk: String,
    pub output_apk: String,
    pub target_lib: Option<String>,
    pub dt_needed_injected: bool,
    pub lib_added: Option<String>,
    pub signed_v1: bool,
    pub v1_verified: bool,
    pub signed_v2: bool,
    pub signature_verified: bool,
    pub manifest_renamed: bool,
    pub manifest_fixed: bool,
    pub final_package: Option<String>,
    /// Every pipeline step under its original `ApkPatcher.kt` status string.
    pub steps: Vec<String>,
    pub notes: Vec<String>,
}

/// `ApkPatcher.PatchOutcome.Failure(stage, detail)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFailure {
    pub stage: String,
    pub detail: String,
}

impl PatchFailure {
    fn new(stage: &str, detail: impl Into<String>) -> Self {
        PatchFailure {
            stage: stage.to_string(),
            detail: detail.into(),
        }
    }
}

/// Options for [`patch_apk`] (the legacy JNI entry point; the full-fidelity
/// port of `ApkPatcher.patchApk`'s parameter set lives in [`PatchPlan`]).
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
    /// Extra companion libs injected *alongside* libOwen.so into the same
    /// target library's DT_NEEDED list and embedded under
    /// `lib/arm64-v8a/<name>` — the seam the E2E pipeline uses for
    /// `libOwenDumper.so` (MobileDumper-7) and `libOwenOGS.so` (the in-game
    /// server). Empty for stock installs.
    pub extra_libs: Vec<(String, PathBuf)>,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            owen_lib: PathBuf::from("libOwen.so"),
            target_lib: None,
            no_sign: false,
            key_path: None,
            rename_package: None,
            extra_libs: Vec::new(),
        }
    }
}

/// High-level APK patch pipeline — the legacy entry point, kept for the JNI
/// adapter (`owen-patcher-jni`). Behaves exactly like
/// [`patch_apk_with_plan`] with a [`PatchPlan::from_options`] plan.
pub fn patch_apk(input: &Path, output: &Path, opts: &PatchOptions) -> Result<PatchReport, String> {
    let plan = PatchPlan::from_options(opts);
    patch_apk_with_plan(input, output, &plan)
        .map_err(|f| format!("patch failed at [{}]: {}", f.stage, f.detail))
}

/// The full-fidelity port of `ApkPatcher.patchApk`, executing the Kotlin
/// pipeline in the same order:
///
/// 1. `Extracting classes.dex...` / `Disassembling dex...` — zero-smali
///    no-ops (nothing is extracted or disassembled; the step names are kept
///    so the report stays 1:1).
/// 2. `Injecting Owen loader...` — [`needed::inject_needed`] writes
///    `libOwen.so` into the engine library's `DT_NEEDED` (the smali-free
///    replacement for the `onCreate` `System.loadLibrary` injection).
/// 3. `Patching console window (4-finger touch)...` /
///    `Patching PartyHub endpoints...` — flags funnelled into `owen.json`
///    ([`PatchPlan::owen_config_json`]) for the redirecter's runtime hooks.
/// 4. `Rebuilding dex...` — zero-smali no-op, `classes.dex` is copied
///    byte-identically.
/// 5. Manifest fix + rename ([`manifest_fixer`] + [`manifest_renamer`]).
/// 6. `Repacking APK...` — raw zip with STORED alignment via [`ziprw`],
///    `lib/arm64-v8a/libOwen.so` embedded, `META-INF/*` stripped.
/// 7. `Signing APK...` — v1 (JAR) + v2 with the same key.
pub fn patch_apk_with_plan(
    input: &Path,
    output: &Path,
    plan: &PatchPlan,
) -> Result<PatchReport, PatchFailure> {
    let mut report = PatchReport {
        input_apk: input.display().to_string(),
        output_apk: output.display().to_string(),
        target_lib: None,
        dt_needed_injected: false,
        lib_added: None,
        signed_v1: false,
        v1_verified: false,
        signed_v2: false,
        signature_verified: false,
        manifest_renamed: false,
        manifest_fixed: false,
        final_package: None,
        steps: Vec::new(),
        notes: Vec::new(),
    };

    // ---- dex steps (zero-smali no-ops; names kept for the report) --------
    report.steps.push(STEP_EXTRACT.into());
    report
        .notes
        .push("zero-smali: classes.dex left untouched (DexTools not needed)".into());
    report.steps.push(STEP_DISASSEMBLE.into());

    // ---- read + validate the Owen library --------------------------------
    let owen_bytes = std::fs::read(&plan.owen_lib).map_err(|e| {
        PatchFailure::new("extract", format!("cannot read {}: {e}", plan.owen_lib.display()))
    })?;
    if owen_bytes.len() < 64 || &owen_bytes[..4] != b"\x7fELF" {
        return Err(PatchFailure::new(
            "extract",
            format!("{} is not an ELF library", plan.owen_lib.display()),
        ));
    }

    let mut zip = ziprw::ZipArchive::open(input)
        .map_err(|e| PatchFailure::new("extract", format!("cannot open {}: {e}", input.display())))?;

    // ---- Injecting Owen loader (DT_NEEDED, the smali-free loader) --------
    report.steps.push(STEP_INJECT.into());
    let candidates = ["lib/arm64-v8a/libUnreal.so", "lib/arm64-v8a/libUE4.so"];
    let target: Option<String> = match &plan.target_lib {
        Some(t) => Some(format!("lib/arm64-v8a/{t}")),
        None => candidates
            .iter()
            .find(|c| zip.list().iter().any(|e| e.name.as_str() == **c))
            .map(|s| s.to_string()),
    };
    let Some(target) = target else {
        return Err(PatchFailure::new(
            "inject",
            "no libUnreal.so/libUE4.so found in APK",
        ));
    };
    report.target_lib = Some(target.clone());

    let lib_bytes = zip
        .read_entry(&target)
        .ok_or_else(|| PatchFailure::new("inject", format!("cannot read {target}")))?;
    let mut working = lib_bytes;
    let mut injected_any = false;
    let (patched_lib, injected) = needed::inject_needed(&working, "libOwen.so")
        .map_err(|e| PatchFailure::new("inject", e))?;
    working = patched_lib;
    injected_any |= injected;
    if injected {
        report.dt_needed_injected = true;
        report.notes.push(format!("DT_NEEDED injected into {target}"));
    } else {
        // Already present.
        report.notes.push("libOwen.so already in DT_NEEDED".into());
    }

    // ---- extra companion libs (libOwenDumper.so / libOwenOGS.so) ----------
    // Each one gets its own DT_NEEDED entry (chained: every call converts the
    // next spare DT_NULL) and its own zip entry under lib/arm64-v8a/.
    let mut extra_bytes: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, path) in &plan.extra_libs {
        if name.contains('/') || name.contains('\\') {
            return Err(PatchFailure::new(
                "inject",
                format!("extra lib name {name:?} must be a bare file name"),
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| {
            PatchFailure::new("inject", format!("cannot read extra lib {}: {e}", path.display()))
        })?;
        if bytes.len() < 64 || &bytes[..4] != b"\x7fELF" {
            return Err(PatchFailure::new(
                "inject",
                format!("{} is not an ELF library", path.display()),
            ));
        }
        let (patched, injected_extra) = needed::inject_needed(&working, name)
            .map_err(|e| PatchFailure::new("inject", format!("extra lib {name}: {e}")))?;
        working = patched;
        injected_any |= injected_extra;
        report
            .notes
            .push(format!("extra lib {name}: {}", if injected_extra { "DT_NEEDED injected" } else { "already present" }));
        extra_bytes.push((name.clone(), bytes));
    }
    if injected_any {
        zip.replace_entry(&target, &working)
            .map_err(|e| PatchFailure::new("inject", e))?;
    }

    // ---- console / PartyHub (runtime flags → owen.json) -------------------
    if plan.console_patch {
        report.steps.push(STEP_CONSOLE.into());
        report
            .notes
            .push("console patch → owen.json console.enabled=true (runtime ART hook)".into());
    } else if plan.is_dev {
        report
            .notes
            .push("Console patch skipped: dev build already ships a console".into());
    }
    if plan.partyhub {
        report.steps.push(STEP_PARTYHUB.into());
        report.notes.push(format!(
            "PartyHub link → owen.json webview.partyhub=auto (login={})",
            plan.login_url
        ));
    }

    // ---- Rebuilding dex (zero-smali no-op) -------------------------------
    report.steps.push(STEP_ASSEMBLE.into());

    // ---- rewriteManifest --------------------------------------------------
    let rewrite = rewrite_manifest(&zip, plan.target_package.as_deref())?;
    report.final_package = rewrite.final_package.clone();
    if rewrite.bytes.is_some() {
        report.manifest_renamed = rewrite.renamed;
        report.manifest_fixed = true;
    }

    // ---- Repacking APK ----------------------------------------------------
    report.steps.push(STEP_REPACK.into());
    zip.remove_meta_inf();
    if let Some(manifest_bytes) = &rewrite.bytes {
        zip.replace_entry("AndroidManifest.xml", manifest_bytes)
            .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    }
    zip.set_entry(OWEN_LIB_ENTRY, &owen_bytes);
    report.lib_added = Some(OWEN_LIB_ENTRY.into());
    for (name, bytes) in &extra_bytes {
        let entry = format!("lib/arm64-v8a/{name}");
        zip.set_entry(&entry, bytes);
        report.lib_added = Some(entry);
    }

    let unsigned = unsigned_path(output);
    zip.write_to(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    let unsigned_len = std::fs::metadata(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?
        .len();
    if unsigned_len < plan.min_output_bytes {
        std::fs::remove_file(&unsigned).ok();
        return Err(PatchFailure::new(
            "repack",
            format!("Repacking produced no APK ({unsigned_len} bytes)."),
        ));
    }

    // ---- Signing APK (v1 + v2, same key) ----------------------------------
    report.steps.push(STEP_SIGN.into());
    if plan.no_sign {
        std::fs::rename(&unsigned, output)
            .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    } else {
        let key_path = plan
            .key_path
            .as_deref()
            .unwrap_or_else(|| Path::new("owen-key.pem"))
            .to_str()
            .unwrap_or("owen-key.pem")
            .to_string();
        let key = sign::SigningKey::generate_or_load(&key_path)
            .map_err(|e| PatchFailure::new("sign", e))?;
        sign_v1::sign_apk_v1(&unsigned, output, &key).map_err(|_| {
            PatchFailure::new("sign", "apksigner failed — see the log for details.")
        })?;
        std::fs::remove_file(&unsigned).ok();
        report.signed_v1 = true;
        report.v1_verified = sign_v1::verify_v1(output).unwrap_or(false);
        sign::sign_apk_v2(output, &key)
            .map_err(|e| PatchFailure::new("sign", e))?;
        report.signed_v2 = true;
        report.signature_verified = sign::verify_v2(output).unwrap_or(false);
        if report.signature_verified {
            report.notes.push("APK Signature Scheme v2 verified".into());
        }
        if report.v1_verified {
            report.notes.push("JAR signature (v1) verified".into());
        }
    }

    // ---- runtime config for the redirecter (console/PartyHub replacement) -
    if plan.emit_owen_config && !plan.backend_url.is_empty() {
        match plan.write_owen_config(output) {
            Ok(path) => report
                .notes
                .push(format!("owen.json written to {}", path.display())),
            Err(e) => report.notes.push(format!("owen.json write failed: {e}")),
        }
    }

    Ok(report)
}

/// `ApkPatcher.renameAndSign` — rewrite the manifest only, repack, sign.
pub fn rename_and_sign(
    input: &Path,
    output: &Path,
    target_package: Option<&str>,
    key_path: Option<&Path>,
) -> Result<PatchReport, PatchFailure> {
    let mut report = PatchReport {
        input_apk: input.display().to_string(),
        output_apk: output.display().to_string(),
        target_lib: None,
        dt_needed_injected: false,
        lib_added: None,
        signed_v1: false,
        v1_verified: false,
        signed_v2: false,
        signature_verified: false,
        manifest_renamed: false,
        manifest_fixed: false,
        final_package: None,
        steps: Vec::new(),
        notes: Vec::new(),
    };

    let mut zip = ziprw::ZipArchive::open(input)
        .map_err(|e| PatchFailure::new("rename", format!("cannot open {}: {e}", input.display())))?;
    let rewrite = rewrite_manifest(&zip, target_package)?;
    let Some(manifest_bytes) = rewrite.bytes else {
        return Err(PatchFailure::new(
            "rename",
            "Nothing to rewrite — the manifest already matches the target package.",
        ));
    };
    report.final_package = rewrite.final_package.clone();
    report.manifest_renamed = rewrite.renamed;
    report.manifest_fixed = true;

    report.steps.push(STEP_REPACK.into());
    zip.remove_meta_inf();
    zip.replace_entry("AndroidManifest.xml", &manifest_bytes)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    let unsigned = unsigned_path(output);
    zip.write_to(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    let unsigned_len = std::fs::metadata(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?
        .len();
    if unsigned_len < MIN_UNSIGNED_APK_BYTES {
        std::fs::remove_file(&unsigned).ok();
        return Err(PatchFailure::new("repack", "Repacking produced no APK."));
    }

    report.steps.push(STEP_SIGN.into());
    let key_path = key_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "owen-key.pem".to_string());
    let key = sign::SigningKey::generate_or_load(&key_path)
        .map_err(|e| PatchFailure::new("sign", e))?;
    sign_v1::sign_apk_v1(&unsigned, output, &key)
        .map_err(|_| PatchFailure::new("sign", "apksigner failed — see the log for details."))?;
    std::fs::remove_file(&unsigned).ok();
    report.signed_v1 = true;
    report.v1_verified = sign_v1::verify_v1(output).unwrap_or(false);
    sign::sign_apk_v2(output, &key).map_err(|e| PatchFailure::new("sign", e))?;
    report.signed_v2 = true;
    report.signature_verified = sign::verify_v2(output).unwrap_or(false);
    Ok(report)
}

/// `ApkPatcher.swapLibAndSign` — fast re-patch: swap `libOwen.so`, repack,
/// sign (no DT_NEEDED work; the patched APK already has the dependency).
pub fn swap_lib_and_sign(
    input: &Path,
    output: &Path,
    owen_lib: &Path,
    key_path: Option<&Path>,
) -> Result<PatchReport, PatchFailure> {
    let mut report = PatchReport {
        input_apk: input.display().to_string(),
        output_apk: output.display().to_string(),
        target_lib: None,
        dt_needed_injected: false,
        lib_added: Some(OWEN_LIB_ENTRY.into()),
        signed_v1: false,
        v1_verified: false,
        signed_v2: false,
        signature_verified: false,
        manifest_renamed: false,
        manifest_fixed: false,
        final_package: None,
        steps: vec![STEP_REPACK.into()],
        notes: Vec::new(),
    };

    let owen_bytes = std::fs::read(owen_lib)
        .map_err(|e| PatchFailure::new("repack", format!("cannot read {}: {e}", owen_lib.display())))?;
    let mut zip = ziprw::ZipArchive::open(input)
        .map_err(|e| PatchFailure::new("repack", format!("cannot open {}: {e}", input.display())))?;
    zip.remove_meta_inf();
    zip.remove_entry(OWEN_LIB_ENTRY);
    zip.set_entry(OWEN_LIB_ENTRY, &owen_bytes);

    let unsigned = unsigned_path(output);
    zip.write_to(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    let unsigned_len = std::fs::metadata(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?
        .len();
    if unsigned_len < MIN_UNSIGNED_APK_BYTES {
        std::fs::remove_file(&unsigned).ok();
        return Err(PatchFailure::new("repack", "Repacking produced no APK."));
    }

    report.steps.push(STEP_SIGN.into());
    let key_path = key_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "owen-key.pem".to_string());
    let key = sign::SigningKey::generate_or_load(&key_path)
        .map_err(|e| PatchFailure::new("sign", e))?;
    sign_v1::sign_apk_v1(&unsigned, output, &key)
        .map_err(|_| PatchFailure::new("sign", "apksigner failed — see the log for details."))?;
    std::fs::remove_file(&unsigned).ok();
    report.signed_v1 = true;
    report.v1_verified = sign_v1::verify_v1(output).unwrap_or(false);
    sign::sign_apk_v2(output, &key).map_err(|e| PatchFailure::new("sign", e))?;
    report.signed_v2 = true;
    report.signature_verified = sign::verify_v2(output).unwrap_or(false);
    Ok(report)
}

/// `ApkPatcher.apkHasOwenLib`.
pub fn apk_has_owen_lib(apk: &Path) -> bool {
    ziprw::ZipArchive::open(apk)
        .map(|zip| zip.has(OWEN_LIB_ENTRY))
        .unwrap_or(false)
}

/// `ApkPatcher.ManifestRewrite`.
struct ManifestRewrite {
    bytes: Option<Vec<u8>>,
    renamed: bool,
    #[allow(dead_code)]
    ok: bool,
    final_package: Option<String>,
}

/// `ApkPatcher.rewriteManifest` — read the package, rename when a target is
/// requested, then apply the uses-sdk floors (ManifestFixer).
fn rewrite_manifest(
    zip: &ziprw::ZipArchive,
    target_package: Option<&str>,
) -> Result<ManifestRewrite, PatchFailure> {
    let Some(mut bytes) = zip.read_entry("AndroidManifest.xml") else {
        return Err(PatchFailure::new(
            "rename",
            "Could not rewrite the manifest (see the log).",
        ));
    };
    let actual_package = manifest_renamer::read_package_name(&bytes);
    let mut renamed = false;
    if let Some(target) = target_package {
        if Some(target) != actual_package.as_deref() {
            let from = actual_package
                .clone()
                .unwrap_or_else(|| ORIGINAL_PACKAGE.to_string());
            match manifest_renamer::rename(&bytes, &from, Some(ORIGINAL_PACKAGE), target) {
                None => {
                    return Err(PatchFailure::new(
                        "rename",
                        "Could not rewrite the manifest (see the log).",
                    ));
                }
                Some(result) => {
                    if result.renamed {
                        bytes = result.bytes;
                        renamed = true;
                    }
                }
            }
        }
    }
    let arsc_bytes = zip.read_entry("resources.arsc");
    let fix_result = manifest_fixer::fix(bytes, arsc_bytes.as_deref());
    if !renamed && !fix_result.changed {
        return Ok(ManifestRewrite {
            bytes: None,
            renamed: false,
            ok: true,
            final_package: actual_package,
        });
    }
    Ok(ManifestRewrite {
        bytes: Some(fix_result.bytes),
        renamed,
        ok: true,
        final_package: if renamed {
            target_package.map(str::to_string)
        } else {
            actual_package
        },
    })
}

/// The unsigned intermediate path next to `output`
/// (Kotlin: `workDir/owen_unsigned_<name>.apk`).
pub(crate) fn unsigned_path(output: &Path) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("owen_unsigned");
    output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.unsigned.apk"))
}
