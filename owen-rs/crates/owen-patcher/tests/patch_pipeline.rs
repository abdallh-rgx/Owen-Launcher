//! Full pipeline integration test: a synthetic Fortnite-shaped APK
//! (zip with lib/arm64-v8a/libUE4.so as a crafted ELF64 + a fake manifest)
//! is patched end-to-end — DT_NEEDED injection, libOwen.so embedding,
//! v2 signing + verification.

use owen_patcher::{needed, patch_apk, sign, ziprw, PatchOptions};
use std::io::Write;
use std::path::PathBuf;

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
    let mut d = |b: &mut Vec<u8>, i: usize, tag: i64, val: u64| {
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
