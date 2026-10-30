//! Full pipeline integration test: a synthetic Fortnite-shaped APK
//! (zip with lib/arm64-v8a/libUE4.so as a crafted ELF64 + a fake manifest)
//! is patched end-to-end — DT_NEEDED injection, libOwen.so embedding,
//! v1+v2 signing + verification.
//!
//! `adaptive_icon_alignment_preserved_after_patch` is the missing icon
//! alignment regression test: a hybrid APK (STORED libs/arsc/icons +
//! DEFLATED dex/manifests) goes through `patch_apk` and every STORED entry
//! must keep its zipalign semantics — `resources.arsc` 4-byte aligned with a
//! byte-identical payload, `lib/*.so` page aligned, the icon PNG untouched.

use owen_patcher::{manifest_fixer, manifest_renamer, needed, patch_apk, sign, ziprw, PatchOptions};
use std::io::Write;

/* ------------------------- tiny zip builder ------------------------- */

fn zip_build(path: &std::path::Path, entries: &[(&str, Vec<u8>)]) {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let mut offset: u32 = 0;
    for (name, data) in entries {
        let crc = ziprw::crc32(data);
        let name_b = name.as_bytes();
        // deflate
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
    out.extend_from_slice(&central);
    let mut eocd = [0u8; 22];
    eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
    eocd[8..10].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd[10..12].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd[12..16].copy_from_slice(&cd_size.to_le_bytes());
    eocd[16..20].copy_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&eocd);
    std::fs::write(path, out).unwrap();
}

/* --------------------- synthetic ELF64 .so builder --------------------- */

fn build_fake_engine_so() -> Vec<u8> {
    // Layout (all offsets = VAs, base 0):
    //   0x00: ELF header (64 bytes)
    //   0x40: program headers: PT_LOAD(whole file), PT_DYNAMIC
    //   0x90: .dynstr: "libc.so\0" + 64 NUL bytes (injection space)
    //   0xE0: dynamic table: DT_STRTAB, DT_STRSZ, DT_NEEDED(libc.so), DT_NULL ×3
    let strtab_off: u64 = 0x90;
    let dyn_off: u64 = 0xE0;
    let file_size: u64 = dyn_off + 6 * 16 + 64;
    let mut b = vec![0u8; file_size as usize];

    // ELF64 header
    b[0..4].copy_from_slice(b"\x7fELF");
    b[4] = 2; // ELFCLASS64
    b[5] = 1; // little-endian
    b[6] = 1; // version
    b[7] = 0; // osabi
    b[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN
    b[18..20].copy_from_slice(&0xB7u16.to_le_bytes()); // EM_AARCH64
    b[20..24].copy_from_slice(&1u32.to_le_bytes()); // version
    b[24..32].copy_from_slice(&0u64.to_le_bytes()); // entry
    b[32..40].copy_from_slice(&0x40u64.to_le_bytes()); // phoff
    b[40..48].copy_from_slice(&0u64.to_le_bytes()); // shoff (none)
    b[52..54].copy_from_slice(&64u16.to_le_bytes()); // ehsize
    b[54..56].copy_from_slice(&56u16.to_le_bytes()); // phentsize
    b[56..58].copy_from_slice(&2u16.to_le_bytes()); // phnum

    // Program headers
    let ph = |b: &mut Vec<u8>, at: usize, p_type: u32, flags: u32, off: u64, va: u64, filesz: u64| {
        b[at..at + 4].copy_from_slice(&p_type.to_le_bytes());
        b[at + 4..at + 8].copy_from_slice(&flags.to_le_bytes());
        b[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
        b[at + 16..at + 24].copy_from_slice(&va.to_le_bytes());
        b[at + 32..at + 40].copy_from_slice(&filesz.to_le_bytes());
        b[at + 40..at + 48].copy_from_slice(&filesz.to_le_bytes());
    };
    ph(&mut b, 0x40, 1 /*PT_LOAD*/, 5, 0, 0, file_size);
    ph(&mut b, 0x40 + 56, 2 /*PT_DYNAMIC*/, 6, dyn_off, dyn_off, 6 * 16);

    // .dynstr: "libc.so" at strtab+0
    let libc = b"libc.so\0";
    b[strtab_off as usize..strtab_off as usize + libc.len()].copy_from_slice(libc);

    // Dynamic entries
    let d = |b: &mut Vec<u8>, i: usize, tag: i64, val: u64| {
        let at = dyn_off as usize + i * 16;
        b[at..at + 8].copy_from_slice(&tag.to_le_bytes());
        b[at + 8..at + 16].copy_from_slice(&val.to_le_bytes());
    };
    d(&mut b, 0, 5 /*DT_STRTAB*/, strtab_off);
    d(&mut b, 1, 10 /*DT_STRSZ*/, 8 + 64);
    d(&mut b, 2, 1 /*DT_NEEDED*/, 0); // "libc.so"
    d(&mut b, 3, 0 /*DT_NULL*/, 0);
    d(&mut b, 4, 0 /*DT_NULL*/, 0);
    d(&mut b, 5, 0 /*DT_NULL*/, 0);
    b
}

fn build_fake_owen_so() -> Vec<u8> {
    // Minimal ELF64 header is enough for patch_apk's magic check.
    let mut b = vec![0u8; 256];
    b[0..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[18..20].copy_from_slice(&0xB7u16.to_le_bytes());
    b
}

/* ------------------------------ the tests ------------------------------ */

#[test]
fn full_patch_pipeline_synthetic_apk() {
    let dir = std::env::temp_dir().join(format!("owen-patch-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let engine = build_fake_engine_so();
    let owen = build_fake_owen_so();
    let owen_path = dir.join("libOwen.so");
    std::fs::write(&owen_path, &owen).unwrap();

    let input = dir.join("game.apk");
    zip_build(
        &input,
        &[
            ("AndroidManifest.xml", b"manifest-bytes".to_vec()),
            ("classes.dex", vec![1u8; 64]),
            ("lib/arm64-v8a/libUE4.so", engine.clone()),
            ("assets/foo.txt", b"hello".to_vec()),
        ],
    );

    let output = dir.join("game-owen.apk");
    let key = dir.join("test-key.pem");
    let opts = PatchOptions {
        owen_lib: owen_path.clone(),
        target_lib: None,
        no_sign: false,
        key_path: Some(key.clone()),
        rename_package: None,
    };
    let report = patch_apk(&input, &output, &opts).expect("patch_apk");
    assert!(report.dt_needed_injected, "DT_NEEDED must be injected");
    assert_eq!(report.lib_added.as_deref(), Some("lib/arm64-v8a/libOwen.so"));
    assert!(report.signed_v2, "must be signed v2");
    assert!(report.signature_verified, "v2 signature must verify");
    assert!(key.is_file(), "the signing key must persist");

    // The patched APK contains both libs and the injection is visible.
    let zip = ziprw::ZipArchive::open(&output).expect("open patched");
    assert!(zip.has("lib/arm64-v8a/libOwen.so"));
    assert!(zip.has("lib/arm64-v8a/libUE4.so"));
    assert!(zip.has("classes.dex"));
    assert!(zip.has("assets/foo.txt"));
    let engine_out = zip.read_entry("lib/arm64-v8a/libUE4.so").unwrap();
    let (image, injected) = needed::inject_needed(&engine_out, "libOwen.so").unwrap();
    assert!(!injected, "the dependency must already be present");
    assert_eq!(image.len(), engine_out.len());

    // Re-patching the PATCHED output is idempotent and re-verifies.
    let output2 = dir.join("game-owen-2.apk");
    let report2 = patch_apk(&output, &output2, &opts).expect("re-patch");
    assert!(!report2.dt_needed_injected);
    assert!(report2.signature_verified);

    // verify_v2 also rejects tampering.
    let mut tampered = std::fs::read(&output).unwrap();
    let len = tampered.len();
    tampered[len - 30] ^= 0xFF; // flip a byte inside the EOCD region
    let tampered_path = dir.join("tampered.apk");
    std::fs::write(&tampered_path, &tampered).unwrap();
    assert!(
        sign::verify_v2(&tampered_path).is_err() || !sign::verify_v2(&tampered_path).unwrap_or(false),
        "tampered APK must fail v2 verification"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dt_needed_injection_on_real_shape() {
    // Direct injection on the synthetic engine lib.
    let engine = build_fake_engine_so();
    let (patched, injected) = needed::inject_needed(&engine, "libOwen.so").unwrap();
    assert!(injected);
    assert_eq!(patched.len(), engine.len(), "in-place byte surgery keeps the size");
    // The injected name is visible through the parser.
    let image = owen_core::elf::ElfImage::parse(&patched).unwrap();
    let names = image.needed_libraries_from(&patched);
    assert!(names.iter().any(|n| n == "libOwen.so"), "found {:?}", names);
    assert!(names.iter().any(|n| n == "libc.so"), "original dependency kept");
}

/* ---------------- hybrid (STORED + DEFLATED) zip builder ---------------- */

/// Build a zip whose entries keep their own compression method: `true` →
/// STORED (zipalign-padded: `lib/*.so` to 4096, others to 4), `false` →
/// DEFLATED. This mimics a real installed APK layout.
fn zip_build_mixed(path: &std::path::Path, entries: &[(&str, Vec<u8>, bool)]) {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let mut offset: u32 = 0;
    for (name, data, stored) in entries {
        let crc = ziprw::crc32(data);
        let name_b = name.as_bytes();
        let (raw, method): (Vec<u8>, u16) = if *stored {
            (data.clone(), 0)
        } else {
            let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
            enc.write_all(data).unwrap();
            (enc.finish().unwrap(), 8)
        };
        let align: u32 = if *stored {
            if name.starts_with("lib/") && name.ends_with(".so") {
                4096
            } else {
                4
            }
        } else {
            1
        };
        let mut extra_pad: u32 = 0;
        if align > 1 {
            let hdr = 30u32 + name_b.len() as u32;
            let rem = (offset + hdr) % align;
            if rem != 0 {
                extra_pad = align - rem;
            }
        }
        let mut lh = vec![0u8; 30];
        lh[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
        lh[4..6].copy_from_slice(&20u16.to_le_bytes());
        lh[8..10].copy_from_slice(&method.to_le_bytes());
        lh[14..18].copy_from_slice(&crc.to_le_bytes());
        lh[18..22].copy_from_slice(&(raw.len() as u32).to_le_bytes());
        lh[22..26].copy_from_slice(&(data.len() as u32).to_le_bytes());
        lh[26..28].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
        lh[28..30].copy_from_slice(&(extra_pad as u16).to_le_bytes());
        out.extend_from_slice(&lh);
        out.extend_from_slice(name_b);
        out.extend_from_slice(&vec![0u8; extra_pad as usize]);
        out.extend_from_slice(&raw);

        let mut ch = vec![0u8; 46];
        ch[0..4].copy_from_slice(&0x0201_4b50u32.to_le_bytes());
        ch[6..8].copy_from_slice(&20u16.to_le_bytes());
        ch[10..12].copy_from_slice(&method.to_le_bytes());
        ch[16..20].copy_from_slice(&crc.to_le_bytes());
        ch[20..24].copy_from_slice(&(raw.len() as u32).to_le_bytes());
        ch[24..28].copy_from_slice(&(data.len() as u32).to_le_bytes());
        ch[28..30].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
        ch[42..46].copy_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(&ch);
        central.extend_from_slice(name_b);
        offset += (30 + name_b.len() + extra_pad as usize + raw.len()) as u32;
    }
    let cd_offset = offset;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    let mut eocd = [0u8; 22];
    eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
    eocd[8..10].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd[10..12].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd[12..16].copy_from_slice(&cd_size.to_le_bytes());
    eocd[16..20].copy_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&eocd);
    std::fs::write(path, out).unwrap();
}

/// Read one entry's (method, DATA offset, compressed size, uncompressed
/// size) straight from the central + local headers — no decompression, so
/// alignment of the on-disk payload can be asserted.
fn entry_data_offset_direct(
    path: &std::path::Path,
    name: &str,
) -> Option<(u16, u64, u32, u32)> {
    let data = std::fs::read(path).ok()?;
    let mut eocd = None;
    if data.len() >= 22 {
        for i in (0..=data.len() - 22).rev() {
            if data[i..i + 4] == 0x0605_4b50u32.to_le_bytes() {
                eocd = Some(i);
                break;
            }
        }
    }
    let eocd = eocd?;
    let cd_offset =
        u32::from_le_bytes([data[eocd + 16], data[eocd + 17], data[eocd + 18], data[eocd + 19]])
            as usize;
    let count = u16::from_le_bytes([data[eocd + 10], data[eocd + 11]]) as usize;
    let mut off = cd_offset;
    for _ in 0..count {
        if off + 46 > data.len() || data[off..off + 4] != 0x0201_4b50u32.to_le_bytes() {
            break;
        }
        let method = u16::from_le_bytes([data[off + 10], data[off + 11]]);
        let comp = u32::from_le_bytes([data[off + 20], data[off + 21], data[off + 22], data[off + 23]]);
        let uncomp = u32::from_le_bytes([data[off + 24], data[off + 25], data[off + 26], data[off + 27]]);
        let name_len = u16::from_le_bytes([data[off + 28], data[off + 29]]) as usize;
        let extra_len = u16::from_le_bytes([data[off + 30], data[off + 31]]) as usize;
        let comment_len = u16::from_le_bytes([data[off + 32], data[off + 33]]) as usize;
        let lho = u32::from_le_bytes([data[off + 42], data[off + 43], data[off + 44], data[off + 45]])
            as usize;
        if off + 46 + name_len > data.len() {
            break;
        }
        if &data[off + 46..off + 46 + name_len] == name.as_bytes() {
            let nl = u16::from_le_bytes([data[lho + 26], data[lho + 27]]) as usize;
            let el = u16::from_le_bytes([data[lho + 28], data[lho + 29]]) as usize;
            let data_off = lho + 30 + nl + el;
            return Some((method, data_off as u64, comp, uncomp));
        }
        off += 46 + name_len + extra_len + comment_len;
    }
    None
}

/* ------------- synthetic binary AXML manifest (UTF-16 pool) -------------- */

fn axml_start_element(name_idx: usize, attrs: &[(usize, u8, u32, u32)]) -> Vec<u8> {
    let size = 16 + 20 + attrs.len() * 20;
    let mut c = vec![0u8; size];
    c[0..2].copy_from_slice(&0x0102u16.to_le_bytes()); // RES_XML_START_ELEMENT
    c[2..4].copy_from_slice(&16u16.to_le_bytes());
    c[4..8].copy_from_slice(&(size as u32).to_le_bytes());
    c[16..20].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // ns
    c[20..24].copy_from_slice(&(name_idx as u32).to_le_bytes());
    c[24..26].copy_from_slice(&20u16.to_le_bytes()); // attributeStart
    c[26..28].copy_from_slice(&20u16.to_le_bytes()); // attributeSize
    c[28..30].copy_from_slice(&(attrs.len() as u16).to_le_bytes());
    for (a, &(ni, dtype, value, raw)) in attrs.iter().enumerate() {
        let at = 16 + 20 + a * 20;
        c[at..at + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // ns
        c[at + 4..at + 8].copy_from_slice(&(ni as u32).to_le_bytes());
        c[at + 8..at + 12].copy_from_slice(&raw.to_le_bytes()); // rawValue
        c[at + 12..at + 14].copy_from_slice(&8u16.to_le_bytes()); // typedValue size
        c[at + 15] = dtype;
        c[at + 16..at + 20].copy_from_slice(&value.to_le_bytes());
    }
    c
}

/// manifest element (package + versionName) + uses-sdk element
/// (minSdkVersion=23, targetSdkVersion=28, TYPE_INT_DEC).
fn build_hybrid_manifest_axml() -> Vec<u8> {
    let strings = [
        "manifest",
        "package",
        "com.epicgames.fortnite",
        "versionName",
        "1.0",
        "uses-sdk",
        "minSdkVersion",
        "targetSdkVersion",
    ];
    let mut blob: Vec<u8> = Vec::new();
    let mut offsets = Vec::new();
    for s in strings {
        offsets.push(blob.len());
        let units: Vec<u16> = s.encode_utf16().collect();
        blob.extend_from_slice(&(units.len() as u16).to_le_bytes());
        for u in units {
            blob.extend_from_slice(&u.to_le_bytes());
        }
        blob.extend_from_slice(&0u16.to_le_bytes());
    }
    let strings_start = 28 + 4 * strings.len();
    let pool_size = strings_start + blob.len();
    let mut pool = vec![0u8; pool_size];
    pool[0..2].copy_from_slice(&0x0001u16.to_le_bytes()); // RES_STRING_POOL
    pool[2..4].copy_from_slice(&28u16.to_le_bytes());
    pool[4..8].copy_from_slice(&(pool_size as u32).to_le_bytes());
    pool[8..12].copy_from_slice(&(strings.len() as u32).to_le_bytes());
    pool[12..16].copy_from_slice(&0u32.to_le_bytes()); // styleCount
    pool[16..20].copy_from_slice(&0u32.to_le_bytes()); // flags: UTF-16
    pool[20..24].copy_from_slice(&(strings_start as u32).to_le_bytes());
    pool[24..28].copy_from_slice(&(pool_size as u32).to_le_bytes());
    for (i, off) in offsets.iter().enumerate() {
        pool[28 + i * 4..32 + i * 4].copy_from_slice(&(*off as u32).to_le_bytes());
    }
    pool[strings_start..].copy_from_slice(&blob);

    let mut chunks: Vec<u8> = Vec::new();
    // manifest: package (string, raw=index 2), versionName (string, raw=4)
    chunks.extend_from_slice(&axml_start_element(
        0,
        &[(1, 0x03, 2, 2), (3, 0x03, 4, 4)],
    ));
    // uses-sdk: minSdkVersion=23, targetSdkVersion=28 (TYPE_INT_DEC, no raw)
    chunks.extend_from_slice(&axml_start_element(
        5,
        &[(6, 0x10, 23, 0xFFFF_FFFF), (7, 0x10, 28, 0xFFFF_FFFF)],
    ));

    let total = 8 + pool.len() + chunks.len();
    let mut out = vec![0u8; total];
    out[0..2].copy_from_slice(&0x0003u16.to_le_bytes()); // RES_XML_TYPE
    out[2..4].copy_from_slice(&8u16.to_le_bytes());
    out[4..8].copy_from_slice(&(total as u32).to_le_bytes());
    out[8..8 + pool.len()].copy_from_slice(&pool);
    out[8 + pool.len()..].copy_from_slice(&chunks);
    out
}

/* ------------------- the missing regression test ------------------------ */

#[test]
fn adaptive_icon_alignment_preserved_after_patch() {
    let dir = std::env::temp_dir().join(format!("owen-align-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let engine = build_fake_engine_so();
    let owen = build_fake_owen_so();
    let owen_path = dir.join("libOwen.so");
    std::fs::write(&owen_path, &owen).unwrap();

    // Deterministic payloads we can byte-compare after the patch.
    let arsc: Vec<u8> = (0..96u32).map(|i| (i * 7 + 3) as u8).collect();
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend((0..120u32).map(|i| (i * 31 + 7) as u8));
    let manifest = build_hybrid_manifest_axml();
    let dex = vec![2u8; 96];
    let extra_manifest = b"hybrid-extra-manifest".to_vec();

    // Sanity: the synthetic manifest parses before the patch.
    assert_eq!(
        manifest_renamer::read_package_name(&manifest).as_deref(),
        Some("com.epicgames.fortnite")
    );

    let input = dir.join("hybrid.apk");
    zip_build_mixed(
        &input,
        &[
            ("AndroidManifest.xml", manifest, false),          // DEFLATED
            ("classes.dex", dex.clone(), false),               // DEFLATED
            ("assets/extra_manifest.xml", extra_manifest.clone(), false), // DEFLATED
            ("resources.arsc", arsc.clone(), true),            // STORED
            ("res/mipmap/ic_launcher.png", png.clone(), true), // STORED
            ("lib/arm64-v8a/libUE4.so", engine, true),         // STORED
        ],
    );

    let output = dir.join("hybrid-owen.apk");
    let key = dir.join("hybrid-key.pem");
    let opts = PatchOptions {
        owen_lib: owen_path,
        target_lib: None,
        no_sign: false,
        key_path: Some(key),
        rename_package: Some("com.owen.aligned1".to_string()),
    };
    let report = patch_apk(&input, &output, &opts).expect("patch_apk");

    // Every ApkPatcher.kt step name is recorded.
    for step in [
        "Extracting classes.dex...",
        "Disassembling dex...",
        "Injecting Owen loader...",
        "Rebuilding dex...",
        "Repacking APK...",
        "Signing APK...",
    ] {
        assert!(report.steps.iter().any(|s| s == step), "missing step {step}");
    }
    assert!(report.dt_needed_injected);
    assert!(report.manifest_renamed);
    assert!(report.manifest_fixed);
    assert_eq!(report.final_package.as_deref(), Some("com.owen.aligned1"));
    assert!(report.signed_v1, "v1 JAR signature present");
    assert!(report.v1_verified, "v1 verifies");
    assert!(report.signed_v2, "v2 signature present");
    assert!(report.signature_verified, "v2 verifies");

    let raw = std::fs::read(&output).unwrap();

    // ---- resources.arsc: still STORED, 4-byte aligned, byte-identical ----
    let (method, arsc_off, arsc_comp, _) =
        entry_data_offset_direct(&output, "resources.arsc").expect("resources.arsc entry");
    assert_eq!(method, 0, "resources.arsc must remain STORED");
    assert_eq!(arsc_off % 4, 0, "resources.arsc data offset must be 4-byte aligned");
    assert_eq!(arsc_comp as usize, arsc.len());
    assert_eq!(
        &raw[arsc_off as usize..arsc_off as usize + arsc.len()],
        &arsc[..],
        "resources.arsc payload must be byte-identical"
    );

    // ---- icon PNG: still STORED, byte-identical --------------------------
    let (png_method, png_off, _, _) =
        entry_data_offset_direct(&output, "res/mipmap/ic_launcher.png").expect("icon entry");
    assert_eq!(png_method, 0, "the icon must remain STORED");
    assert_eq!(
        &raw[png_off as usize..png_off as usize + png.len()],
        &png[..],
        "icon PNG payload must be byte-identical"
    );

    // ---- libUE4.so: page aligned, DT_NEEDED patched in place -------------
    let (so_method, so_off, _, _) =
        entry_data_offset_direct(&output, "lib/arm64-v8a/libUE4.so").expect("engine entry");
    assert_eq!(so_method, 0, "libUE4.so must remain STORED");
    assert_eq!(so_off % 4096, 0, "libUE4.so data offset must be 4096-aligned");

    // ---- libOwen.so: added STORED + page aligned -------------------------
    let (owen_method, owen_off, _, _) =
        entry_data_offset_direct(&output, "lib/arm64-v8a/libOwen.so").expect("owen entry");
    assert_eq!(owen_method, 0, "libOwen.so must be STORED");
    assert_eq!(owen_off % 4096, 0, "libOwen.so data offset must be 4096-aligned");
    assert_eq!(
        &raw[owen_off as usize..owen_off as usize + owen.len()],
        &owen[..],
        "libOwen.so payload must be byte-identical"
    );

    // ---- DEFLATED entries survive with identical content -----------------
    let zip = ziprw::ZipArchive::open(&output).expect("open patched");
    assert_eq!(zip.read_entry("classes.dex").unwrap(), dex);
    assert_eq!(zip.read_entry("assets/extra_manifest.xml").unwrap(), extra_manifest);

    // ---- the manifest was renamed + the sdk floors applied ---------------
    let out_manifest = zip.read_entry("AndroidManifest.xml").unwrap();
    assert_eq!(
        manifest_renamer::read_package_name(&out_manifest).as_deref(),
        Some("com.owen.aligned1")
    );
    assert_eq!(
        manifest_renamer::read_version_name(&out_manifest).as_deref(),
        Some("1.0")
    );
    let fix = manifest_fixer::fix(out_manifest, None);
    assert!(!fix.changed, "floors already applied by the pipeline");
    assert_eq!(fix.min_sdk, 24);
    assert_eq!(fix.target_sdk, 29);

    // ---- the shipped engine lib carries the DT_NEEDED entry --------------
    let engine_out = zip.read_entry("lib/arm64-v8a/libUE4.so").unwrap();
    let elf = owen_core::elf::ElfImage::parse(&engine_out).unwrap();
    let needed = elf.needed_libraries_from(&engine_out);
    assert!(needed.iter().any(|n| n == "libOwen.so"), "found {needed:?}");

    // ---- v1 signature files present --------------------------------------
    assert!(zip.has("META-INF/MANIFEST.MF"));
    assert!(zip.has("META-INF/CERT.SF"));
    assert!(zip.has("META-INF/CERT.RSA"));

    std::fs::remove_dir_all(&dir).ok();
}

/* ---------------- DT_NEEDED gap hardening (real-device crash fix) ------- */

/// Engine variant WITH section headers, a tight strtab (no inner padding)
/// and a 32-byte alignment gap immediately after it. `poison_gap` decides
/// whether the gap contains live-looking non-zero bytes.
fn build_engine_with_gap(poison_gap: bool) -> Vec<u8> {
    // Layout (every region disjoint — the program headers end at 0xB0):
    //   0x00  ELF header          (64)
    //   0x40  phdrs ×2            (112 → ends 0xB0)
    //   0xB0  .dynstr "libc.so\0" (8 bytes, DT_STRSZ == 8 — no inner room)
    //   0xB8  32-byte gap         (zero or poisoned)
    //   0xD8  .dummy section      (16 bytes)
    //   0xE8  dynamic table       (6 * 16)
    //   0x148 section headers     (null + .dynstr + .dummy)
    let strtab_off: u64 = 0xB0;
    let gap_off: u64 = 0xB8;
    let dummy_off: u64 = 0xD8;
    let dyn_off: u64 = 0xE8;
    let shoff: u64 = dyn_off + 6 * 16;
    let file_size: u64 = shoff + 3 * 64;
    let mut b = vec![0u8; file_size as usize];

    b[0..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[6] = 1;
    b[16..18].copy_from_slice(&3u16.to_le_bytes());
    b[18..20].copy_from_slice(&0xB7u16.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[32..40].copy_from_slice(&0x40u64.to_le_bytes());
    b[40..48].copy_from_slice(&shoff.to_le_bytes()); // section headers present
    b[52..54].copy_from_slice(&64u16.to_le_bytes());
    b[54..56].copy_from_slice(&56u16.to_le_bytes());
    b[56..58].copy_from_slice(&2u16.to_le_bytes()); // phnum
    b[58..60].copy_from_slice(&64u16.to_le_bytes()); // shentsize
    b[60..62].copy_from_slice(&3u16.to_le_bytes()); // shnum (null + 2)

    let ph = |b: &mut Vec<u8>, at: usize, p_type: u32, flags: u32, off: u64, va: u64, filesz: u64| {
        b[at..at + 4].copy_from_slice(&p_type.to_le_bytes());
        b[at + 4..at + 8].copy_from_slice(&flags.to_le_bytes());
        b[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
        b[at + 16..at + 24].copy_from_slice(&va.to_le_bytes());
        b[at + 32..at + 40].copy_from_slice(&filesz.to_le_bytes());
        b[at + 40..at + 48].copy_from_slice(&filesz.to_le_bytes());
    };
    ph(&mut b, 0x40, 1, 5, 0, 0, file_size);
    ph(&mut b, 0x40 + 56, 2, 6, dyn_off, dyn_off, 6 * 16);

    let libc = b"libc.so\0";
    b[strtab_off as usize..strtab_off as usize + 8].copy_from_slice(libc);
    if poison_gap {
        b[gap_off as usize..gap_off as usize + 32].fill(0xAA); // live data!
    }

    let d = |b: &mut Vec<u8>, i: usize, tag: i64, val: u64| {
        let at = dyn_off as usize + i * 16;
        b[at..at + 8].copy_from_slice(&tag.to_le_bytes());
        b[at + 8..at + 16].copy_from_slice(&val.to_le_bytes());
    };
    d(&mut b, 0, 5 /*DT_STRTAB*/, strtab_off);
    d(&mut b, 1, 10 /*DT_STRSZ*/, 8); // tight — forces the gap path
    d(&mut b, 2, 1 /*DT_NEEDED*/, 0);
    d(&mut b, 3, 0, 0);
    d(&mut b, 4, 0, 0);
    d(&mut b, 5, 0, 0);

    // Section headers: [0] null, [1] .dynstr (SHT_STRTAB), [2] .dummy.
    let sh = |b: &mut Vec<u8>, i: usize, sh_type: u32, off: u64, size: u64| {
        let at = shoff as usize + i * 64;
        b[at + 4..at + 8].copy_from_slice(&sh_type.to_le_bytes());
        b[at + 24..at + 32].copy_from_slice(&off.to_le_bytes());
        b[at + 32..at + 40].copy_from_slice(&size.to_le_bytes());
    };
    sh(&mut b, 0, 0, 0, 0);
    sh(&mut b, 1, 3 /*SHT_STRTAB*/, strtab_off, 8);
    sh(&mut b, 2, 1 /*SHT_PROGBITS*/, dummy_off, 16);
    b
}

#[test]
fn dt_needed_refuses_poisoned_gap_and_uses_clean_gap() {
    // A gap full of non-zero bytes must NEVER host the dependency name —
    // the previous behavior extended DT_STRSZ over it and the dynamic
    // linker read garbage strings (game crashes during loading).
    let poisoned = build_engine_with_gap(true);
    let err = needed::inject_needed(&poisoned, "libOwen.so")
        .err()
        .expect("injection into a poisoned gap must be refused");
    assert!(
        err.contains("no safe padding"),
        "unexpected error: {err}"
    );

    // The same layout with a clean zero gap injects fine.
    let clean = build_engine_with_gap(false);
    let (patched, injected) = needed::inject_needed(&clean, "libOwen.so")
        .expect("clean gap must host the name");
    assert!(injected);
    let image = owen_core::elf::ElfImage::parse(&patched).unwrap();
    let names = image.needed_libraries_from(&patched);
    assert!(names.iter().any(|n| n == "libOwen.so"), "found {:?}", names);
    assert!(names.iter().any(|n| n == "libc.so"));
}
