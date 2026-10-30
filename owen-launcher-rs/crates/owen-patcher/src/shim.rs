//! The shim injection pipeline — the flash-crash-proof alternative to
//! [`crate::needed`] DT_NEEDED surgery.
//!
//! Instead of editing the 268MB engine ELF's dynamic tables (the class of
//! bug that made the game die at load: strtab/DT_NULL corner cases across
//! Android linker versions), the shim mode does **zero ELF surgery**:
//!
//! ```text
//! lib/arm64-v8a/libUE4.so      → lib/arm64-v8a/libUE4real.so   (rename, same bytes)
//! lib/arm64-v8a/libUE4.so      ← the ~20KB loader shim (tools/shim/libue4_shim.c)
//! lib/arm64-v8a/libOwen.so     ← companion mod #1 (owen-redirecter)
//! lib/arm64-v8a/libOwenOGS.so  ← companion mod #2 (owen-ogs)     ─ optional
//! lib/arm64-v8a/libOwenDumper.so ← MobileDumper-7 (CI dumps)     ─ optional
//! ```
//!
//! `System.loadLibrary("UE4")` loads the shim; the shim's `JNI_OnLoad`
//! dlopens the renamed engine, calls its own `JNI_OnLoad`, then dlopens
//! every companion lib that is present (see `tools/shim/libue4_shim.c`).
//!
//! The engine entry keeps the ORIGINAL compression of the source APK
//! (deflated for the 12.41 APK → the patched APK stays ~150MB, not 500MB);
//! the shim and the companion libs are stored + page-aligned (safe for
//! both `extractNativeLibs` modes).
//!
//! The AndroidManifest is untouched: same package, same activities, same
//! SDK levels — the game has no idea anything changed.

use crate::{
    sign, sign_v1, ziprw, PatchFailure, PatchReport, STEP_REPACK, STEP_SIGN,
};
use std::path::Path;

/// The engine entry in the stock APK.
pub const ENGINE_ENTRY: &str = "lib/arm64-v8a/libUE4.so";
/// The renamed engine in the shim-patched APK.
pub const ENGINE_REAL_ENTRY: &str = "lib/arm64-v8a/libUE4real.so";

/// Options for [`shim_patch_apk`].
#[derive(Debug, Clone)]
pub struct ShimOptions {
    /// Prebuilt loader shim that takes the `libUE4.so` slot
    /// (`tools/shim/libue4_shim.c` compiled for aarch64).
    pub shim_lib: std::path::PathBuf,
    /// Companion libs injected alongside the shim — `(soname, path)` pairs,
    /// e.g. `("libOwen.so", …)`, `("libOwenOGS.so", …)`. The shim dlopens
    /// whichever are installed; missing ones are skipped.
    pub mods: Vec<(String, std::path::PathBuf)>,
    /// Signing key path; `None` = `owen-key.pem`.
    pub key_path: Option<std::path::PathBuf>,
    /// The "repacking produced no APK" floor (same as the legacy pipeline).
    pub min_output_bytes: u64,
}

impl Default for ShimOptions {
    fn default() -> Self {
        ShimOptions {
            shim_lib: std::path::PathBuf::from("libUE4shim.so"),
            mods: Vec::new(),
            key_path: None,
            min_output_bytes: crate::MIN_UNSIGNED_APK_BYTES,
        }
    }
}

/// Shim-patch `input` into `output`.
///
/// Steps: read + drop the engine entry → re-add it deflated as
/// `libUE4real.so` → add the stored shim as `libUE4.so` → add every
/// companion mod → repack (aligned) → sign v1+v2 → verify.
pub fn shim_patch_apk(
    input: &Path,
    output: &Path,
    opts: &ShimOptions,
) -> Result<PatchReport, PatchFailure> {
    let mut report = PatchReport {
        input_apk: input.display().to_string(),
        output_apk: output.display().to_string(),
        target_lib: Some("libUE4.so (shim)".into()),
        dt_needed_injected: false,
        lib_added: Some(ENGINE_REAL_ENTRY.into()),
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
    report
        .notes
        .push("shim mode: engine renamed to libUE4real.so; loader shim in the libUE4.so slot".into());

    let mut zip = ziprw::ZipArchive::open(input).map_err(|e| {
        PatchFailure::new("open", format!("cannot open {}: {e}", input.display()))
    })?;

    // ---- the engine bytes --------------------------------------------------
    let engine = zip
        .read_entry(ENGINE_ENTRY)
        .ok_or_else(|| PatchFailure::new("shim", "no lib/arm64-v8a/libUE4.so in the source APK"))?;
    zip.remove_meta_inf();
    if !zip.remove_entry(ENGINE_ENTRY) {
        return Err(PatchFailure::new("shim", "cannot drop the engine entry"));
    }
    // Keep the engine DEFLATED like the stock entry (the installer extracts
    // it; a stored 268MB entry would balloon the APK to half a gig).
    zip.add_deflated(ENGINE_REAL_ENTRY, &engine);
    report
        .notes
        .push(format!("engine: {} bytes as {}", ENGINE_REAL_ENTRY, ENGINE_REAL_ENTRY));

    // ---- the shim takes the libUE4.so slot ---------------------------------
    let shim = std::fs::read(&opts.shim_lib).map_err(|e| {
        PatchFailure::new(
            "shim",
            format!("cannot read shim {}: {e}", opts.shim_lib.display()),
        )
    })?;
    if shim.len() < 512 {
        return Err(PatchFailure::new(
            "shim",
            format!("{} does not look like an ELF ({})", opts.shim_lib.display(), shim.len()),
        ));
    }
    if shim.get(0..4) != Some(b"\x7fELF") {
        return Err(PatchFailure::new(
            "shim",
            format!("{} is not an ELF", opts.shim_lib.display()),
        ));
    }
    // STORED + page-aligned (add_entry does exactly that for lib/*.so).
    zip.add_entry(ENGINE_ENTRY, &shim);

    // ---- companion mods -----------------------------------------------------
    for (name, path) in &opts.mods {
        let bytes = std::fs::read(path).map_err(|e| {
            PatchFailure::new("shim", format!("cannot read {name} at {}: {e}", path.display()))
        })?;
        let entry = format!("lib/arm64-v8a/{name}");
        zip.remove_entry(&entry);
        zip.add_entry(&entry, &bytes);
        report.notes.push(format!("mod: {entry} ({} bytes)", bytes.len()));
    }

    // ---- repack -------------------------------------------------------------
    report.steps.push(STEP_REPACK.into());
    let unsigned = crate::unsigned_path(output);
    zip.write_to(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?;
    let unsigned_len = std::fs::metadata(&unsigned)
        .map_err(|e| PatchFailure::new("repack", format!("Repacking failed: {e}")))?
        .len();
    if unsigned_len < opts.min_output_bytes {
        std::fs::remove_file(&unsigned).ok();
        return Err(PatchFailure::new("repack", "Repacking produced no APK."));
    }

    // ---- sign v1 + v2 -------------------------------------------------------
    report.steps.push(STEP_SIGN.into());
    let key_path = opts
        .key_path
        .as_ref()
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

    // ---- sanity: the patched APK must contain the whole chain ----------------
    let check = ziprw::ZipArchive::open(output)
        .map_err(|e| PatchFailure::new("verify", format!("cannot reopen {}: {e}", output.display())))?;
    let mut must_have: Vec<String> = vec![ENGINE_ENTRY.into(), ENGINE_REAL_ENTRY.into()];
    for (n, _) in &opts.mods {
        must_have.push(format!("lib/arm64-v8a/{n}"));
    }
    for e in &must_have {
        if !check.has(e) {
            return Err(PatchFailure::new("verify", format!("missing entry in the patched APK: {e}")));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Synthetic mini-APK: manifest + a fake 4KB "engine" (raw zip bytes, all deflated).
    fn synth_apk(dir: &Path, name: &str) -> PathBuf {
        let apk = dir.join(name);
        let engine: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let manifest: Vec<u8> = vec![0x03, 0x00, 0x08, 0x00, 0x1c, 0x00, 0x00, 0x00];
        let entries: Vec<(&str, Vec<u8>)> = vec![
            ("AndroidManifest.xml", manifest),
            (ENGINE_ENTRY, engine),
        ];
        zip_build_raw(&apk, &entries);
        apk
    }

    /// Minimal all-deflated zip writer (same layout the integration tests use).
    fn zip_build_raw(path: &std::path::Path, entries: &[(&str, Vec<u8>)]) {
        use std::io::Write;
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let mut offset: u32 = 0;
        for (name, data) in entries {
            let crc = ziprw::crc32(data);
            let name_b = name.as_bytes();
            let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
            enc.write_all(data).unwrap();
            let raw = enc.finish().unwrap();
            let mut lh = vec![0u8; 30];
            lh[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
            lh[4..6].copy_from_slice(&20u16.to_le_bytes());
            lh[8..10].copy_from_slice(&8u16.to_le_bytes());
            lh[14..18].copy_from_slice(&crc.to_le_bytes());
            lh[18..22].copy_from_slice(&(raw.len() as u32).to_le_bytes());
            lh[22..26].copy_from_slice(&(data.len() as u32).to_le_bytes());
            lh[26..28].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
            out.extend_from_slice(&lh);
            out.extend_from_slice(name_b);
            out.extend_from_slice(&raw);
            let mut ch = vec![0u8; 46];
            ch[0..4].copy_from_slice(&0x0201_4b50u32.to_le_bytes());
            ch[6..8].copy_from_slice(&20u16.to_le_bytes());
            ch[10..12].copy_from_slice(&8u16.to_le_bytes());
            ch[16..20].copy_from_slice(&crc.to_le_bytes());
            ch[20..24].copy_from_slice(&(raw.len() as u32).to_le_bytes());
            ch[24..28].copy_from_slice(&(data.len() as u32).to_le_bytes());
            ch[28..30].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
            ch[42..46].copy_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(&ch);
            central.extend_from_slice(name_b);
            offset += (30 + name_b.len() + raw.len()) as u32;
        }
        let cd_offset = offset;
        let cd_size = central.len() as u32;
        let n = entries.len() as u16;
        out.extend_from_slice(&central);
        let mut eocd = [0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        eocd[8..10].copy_from_slice(&n.to_le_bytes());
        eocd[10..12].copy_from_slice(&n.to_le_bytes());
        eocd[12..16].copy_from_slice(&cd_size.to_le_bytes());
        eocd[16..20].copy_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&eocd);
        std::fs::write(path, out).unwrap();
    }

    /// Synthetic shim ELF (header + padding; the pipeline only checks magic).
    fn synth_shim(dir: &Path) -> PathBuf {
        let mut shim = vec![0x7f, b'E', b'L', b'F'];
        shim.extend(std::iter::repeat(0x90u8).take(2048));
        let p = dir.join("libUE4shim.so");
        std::fs::write(&p, shim).unwrap();
        p
    }

    #[test]
    fn shim_patch_roundtrip() {
        let dir = std::env::temp_dir().join(format!("owen-shim-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = synth_apk(&dir, "in.apk");
        let out = dir.join("out.apk");
        let opts = ShimOptions {
            shim_lib: synth_shim(&dir),
            mods: vec![("libOwen.so".into(), {
                let p = dir.join("libOwen.so");
                let mut b = vec![0x7f, b'E', b'L', b'F'];
                b.extend(std::iter::repeat(1u8).take(4096));
                std::fs::write(&p, b).unwrap();
                p
            })],
            key_path: Some(dir.join("k.pem")),
            min_output_bytes: 0,
        };
        let report = shim_patch_apk(&input, &out, &opts).expect("shim patch");
        assert!(report.signed_v1 && report.signed_v2);
        assert!(report.signature_verified, "v2 signature must verify");
        assert!(report.v1_verified, "v1 signature must verify");
        // chain present + engine bytes intact
        let zip = ziprw::ZipArchive::open(&out).unwrap();
        let engine = zip.read_entry(ENGINE_REAL_ENTRY).expect("engine entry");
        assert_eq!(engine.len(), 4096);
        assert_eq!(engine[7], 7 % 251);
        assert!(zip.has(ENGINE_ENTRY), "shim in the libUE4.so slot");
        assert!(zip.has("lib/arm64-v8a/libOwen.so"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
