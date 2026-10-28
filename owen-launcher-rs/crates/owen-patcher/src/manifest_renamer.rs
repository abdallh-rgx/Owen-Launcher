//! Binary-AXML package/label renaming — the literal Rust port of
//! `ManifestRenamer.kt` + `ManifestRenamerStrings.kt`.
//!
//! ## Kotlin source map
//!
//! | Kotlin (`ManifestRenamer.kt`)                        | Rust (here)                            |
//! |------------------------------------------------------|----------------------------------------|
//! | `Result(bytes, packageName, renamed)`                | [`RenameResult`]                       |
//! | `readPackageName(manifest)`                          | [`read_package_name`]                  |
//! | `readVersionName(manifest)`                          | [`read_version_name`]                  |
//! | `readPackageNameFromApk(apk)`                        | [`read_package_name_from_apk`]         |
//! | `readVersionNameFromApk(apk)`                        | [`read_version_name_from_apk`]         |
//! | `readManifestAttrFromApk` / `readManifestStringAttr` | [`read_manifest_attr_from_apk`] / [`read_manifest_string_attr`] |
//! | `rename(manifest, old, legacy, new)`                 | [`rename`]                             |
//! | `renameInternal`                                     | [`rename_internal`]                    |
//! | `indexOfNew`                                         | [`index_of_new`]                       |
//! | `replacePackagePrefix`                               | [`replace_package_prefix`]             |
//! | `renameAuthoritySegments`                            | [`rename_authority_segments`]          |
//! | `parsePool` / `Pool`                                 | [`owen_apk::axml::StringPool::parse_pool`] |
//! | `encodeString` / `var8` / `readVar8`                 | [`owen_apk::axml::encode_string`]      |
//! | `ManifestRenamerStrings.parse`                       | [`owen_apk::axml::strings_at`]         |
//! | `u16`/`u32`/`p16`/`p32`                              | [`u16le`]/[`u32le`]/[`p16`]/[`p32`]    |
//!
//! Kotlin exceptions (`catch (t) → null`) become `None` / `Err` returns; the
//! pool chunk parser itself lives in `owen-apk::axml` so the fixer reuses it.
//!
//! **Deviation from the letter of the Kotlin** (`renameInternal`, tail
//! patching): `ManifestRenamer.kt:261` computes
//! `newAbs = tailStartNew + (dataOffset - tailStart)` and indexes the copied
//! tail with it — an off-by-`tailStartNew` slip that either skips the write
//! or corrupts an unrelated u32. The port patches `dataOffset - tailStart`
//! (the attribute's offset inside the tail), which is the only reading under
//! which `Result.bytes` actually carries the renamed package — the thing
//! `ApkPatcher.rewriteManifest` depends on.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/{ManifestRenamer.kt,ManifestRenamerStrings.kt}

use owen_apk::axml::{
    encode_string, StringPool, RES_STRING_POOL_TYPE, RES_XML_START_ELEMENT_TYPE, RES_XML_TYPE,
};
use std::path::Path;

const NO_INDEX: u32 = 0xFFFF_FFFF;
const TYPE_STRING: u8 = 0x03;

/// `ManifestRenamer.Result`.
#[derive(Debug, Clone)]
pub struct RenameResult {
    pub bytes: Vec<u8>,
    pub package_name: String,
    pub renamed: bool,
}

fn u16le(b: &[u8], o: usize) -> Option<u16> {
    let s = b.get(o..o + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    let s = b.get(o..o + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn p16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn p32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// `ManifestRenamer.readPackageName(manifest: ByteArray): String?`
pub fn read_package_name(manifest: &[u8]) -> Option<String> {
    read_manifest_string_attr(manifest, "package")
}

/// `ManifestRenamer.readVersionName(manifest: ByteArray): String?`
pub fn read_version_name(manifest: &[u8]) -> Option<String> {
    read_manifest_string_attr(manifest, "versionName")
}

/// `ManifestRenamer.readPackageNameFromApk(apk: File): String?`
pub fn read_package_name_from_apk(apk: &Path) -> Option<String> {
    read_manifest_attr_from_apk(apk, "package")
}

/// `ManifestRenamer.readVersionNameFromApk(apk: File): String?`
pub fn read_version_name_from_apk(apk: &Path) -> Option<String> {
    read_manifest_attr_from_apk(apk, "versionName")
}

/// `ManifestRenamer.readManifestAttrFromApk` — reads `AndroidManifest.xml`
/// from the APK zip and extracts one string attribute (any error → `None`,
/// exactly like the Kotlin `catch (t: Throwable) { null }`).
fn read_manifest_attr_from_apk(apk: &Path, name: &str) -> Option<String> {
    let zip = crate::ziprw::ZipArchive::open(apk).ok()?;
    let bytes = zip.read_entry("AndroidManifest.xml")?;
    read_manifest_string_attr(&bytes, name)
}

/// `ManifestRenamer.readManifestStringAttr(manifest, name)` — walk the AXML
/// chunks after the string pool looking for the `manifest` start element and
/// a string-typed attribute with the requested name.
fn read_manifest_string_attr(manifest: &[u8], name: &str) -> Option<String> {
    if u16le(manifest, 0)? != RES_XML_TYPE {
        return None;
    }
    let file_size = u32le(manifest, 4)? as usize;
    let limit = file_size.min(manifest.len());
    let mut pool_start = None;
    let mut pool_size = 0usize;
    let mut offset = 8usize;
    while offset + 8 <= limit {
        let typ = u16le(manifest, offset)?;
        let chunk_size = u32le(manifest, offset + 4)? as usize;
        if chunk_size == 0 || offset + chunk_size > manifest.len() {
            break;
        }
        if typ == RES_STRING_POOL_TYPE {
            pool_start = Some(offset);
            pool_size = chunk_size;
            break;
        }
        offset += chunk_size;
    }
    let pool_start = pool_start?;
    let pool = StringPool::parse_pool(&manifest[pool_start..pool_start + pool_size])?;
    let mut pos = pool_start + pool_size;
    while pos + 8 <= limit {
        let typ = u16le(manifest, pos)?;
        let chunk_size = u32le(manifest, pos + 4)? as usize;
        if chunk_size == 0 || pos + chunk_size > manifest.len() {
            break;
        }
        if typ == RES_XML_START_ELEMENT_TYPE {
            let name_index = u32le(manifest, pos + 20)? as usize;
            let element_name = pool.strings.get(name_index).map(String::as_str).unwrap_or("");
            if element_name == "manifest" {
                let attribute_start = u16le(manifest, pos + 24)? as usize;
                let attribute_size = u16le(manifest, pos + 26)? as usize;
                let attribute_count = u16le(manifest, pos + 28)? as usize;
                let attr_base = pos + 16 + attribute_start;
                for a in 0..attribute_count {
                    let attr_at = attr_base + a * attribute_size;
                    if attr_at + 20 > manifest.len() {
                        break;
                    }
                    let attr_name_index = u32le(manifest, attr_at + 4)? as usize;
                    let attr_name = pool.strings.get(attr_name_index).map(String::as_str).unwrap_or("");
                    let data_type = manifest[attr_at + 15];
                    if attr_name == name && data_type == TYPE_STRING {
                        let value_index = u32le(manifest, attr_at + 16)? as usize;
                        return pool.strings.get(value_index).cloned();
                    }
                }
            }
        }
        pos += chunk_size;
    }
    None
}

/// `ManifestRenamer.rename(manifest, oldPackage, legacyPackage, newPackage)` —
/// returns `None` where the Kotlin caught a `Throwable` and logged
/// "AXML rename failed".
pub fn rename(
    manifest: &[u8],
    old_package: &str,
    legacy_package: Option<&str>,
    new_package: &str,
) -> Option<RenameResult> {
    rename_internal(manifest, old_package, legacy_package, new_package)
}

/// `ManifestRenamer.renameInternal`.
fn rename_internal(
    manifest: &[u8],
    old_package: &str,
    legacy_package: Option<&str>,
    new_package: &str,
) -> Option<RenameResult> {
    if u16le(manifest, 0)? != RES_XML_TYPE {
        return None;
    }
    let file_size = u32le(manifest, 4)? as usize;
    let limit = file_size.min(manifest.len());
    let mut pool_start = None;
    let mut pool_size = 0usize;
    let mut offset = 8usize;
    while offset + 8 <= limit {
        let typ = u16le(manifest, offset)?;
        let chunk_size = u32le(manifest, offset + 4)? as usize;
        if chunk_size == 0 || offset + chunk_size > manifest.len() {
            break;
        }
        if typ == RES_STRING_POOL_TYPE {
            pool_start = Some(offset);
            pool_size = chunk_size;
            break;
        }
        offset += chunk_size;
    }
    let pool_start = pool_start?;
    let pool_bytes = &manifest[pool_start..pool_start + pool_size];
    let pool = StringPool::parse_pool(pool_bytes)?;

    let mut added: Vec<String> = Vec::new();
    // (absolute offset in `manifest`, is_raw_value_patch, new pool index)
    let mut patches: Vec<(usize, bool, usize)> = Vec::new();

    let mut pos = pool_start + pool_size;
    while pos + 8 <= limit {
        let typ = u16le(manifest, pos)?;
        let chunk_size = u32le(manifest, pos + 4)? as usize;
        if chunk_size == 0 || pos + chunk_size > manifest.len() {
            break;
        }
        if typ == RES_XML_START_ELEMENT_TYPE {
            let name_index = u32le(manifest, pos + 20)? as usize;
            let element_name = pool.strings.get(name_index).cloned().unwrap_or_default();
            let attribute_start = u16le(manifest, pos + 24)? as usize;
            let attribute_size = u16le(manifest, pos + 26)? as usize;
            let attribute_count = u16le(manifest, pos + 28)? as usize;
            let attr_base = pos + 16 + attribute_start;
            for a in 0..attribute_count {
                let attr_at = attr_base + a * attribute_size;
                if attr_at + 20 > manifest.len() {
                    break;
                }
                let attr_name_index = u32le(manifest, attr_at + 4)? as usize;
                let attr_name = pool.strings.get(attr_name_index).cloned().unwrap_or_default();
                let raw_value_index = u32le(manifest, attr_at + 8)?;
                let data_type = manifest[attr_at + 15];
                let data_offset = attr_at + 16;
                let data = u32le(manifest, data_offset)?;
                if data_type != TYPE_STRING {
                    continue;
                }
                let value = pool.strings.get(data as usize).cloned().unwrap_or_default();
                let hit = (element_name == "manifest"
                    && attr_name == "package"
                    && (value == old_package
                        || legacy_package.map(|l| value == l).unwrap_or(false)))
                    || (element_name == "provider" && attr_name == "authorities");
                if !hit {
                    continue;
                }
                let renamed_value = if element_name == "manifest" {
                    new_package.to_string()
                } else {
                    match rename_authority_segments(&value, old_package, new_package) {
                        Some(v) => v,
                        None => match legacy_package
                            .and_then(|legacy| rename_authority_segments(&value, legacy, new_package))
                        {
                            Some(v) => v,
                            None => continue,
                        },
                    }
                };
                let new_index = index_of_new(&pool, &mut added, &renamed_value);
                patches.push((data_offset, false, new_index));
                if raw_value_index != NO_INDEX {
                    patches.push((attr_at + 8, true, new_index));
                }
            }
        }
        pos += chunk_size;
    }

    if patches.is_empty() {
        // Kotlin: pool.strings.firstOrNull { it == oldPackage } ?: "unknown"
        let current = pool
            .strings
            .iter()
            .find(|s| s.as_str() == old_package)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        return Some(RenameResult {
            bytes: manifest.to_vec(),
            package_name: current,
            renamed: false,
        });
    }

    // ---- rebuild the pool with the appended strings (Kotlin lines 202-246) --
    let strings_blob_start = pool.strings_start;
    let strings_blob_end = if pool.style_count > 0 { pool.styles_start } else { pool.size };
    let strings_blob = &pool_bytes[strings_blob_start..strings_blob_end];
    let styles_blob = &pool_bytes[strings_blob_end..pool.size];

    let mut appended: Vec<Vec<u8>> = Vec::new();
    let mut appended_offsets: Vec<usize> = Vec::new();
    let mut appended_len = 0usize;
    for s in &added {
        appended_offsets.push(appended_len);
        let encoded = encode_string(s, pool.is_utf8());
        appended_len += encoded.len();
        appended.push(encoded);
    }

    let new_count = pool.string_count + added.len();
    let new_strings_start = 28 + 4 * new_count;
    let styles_at = (new_strings_start + strings_blob.len() + appended_len + 3) & !3;
    let new_pool_size = styles_at + styles_blob.len();
    let mut new_pool = vec![0u8; new_pool_size];
    p16(&mut new_pool, 0, RES_STRING_POOL_TYPE);
    p16(&mut new_pool, 2, 28);
    p32(&mut new_pool, 4, new_pool_size as u32);
    p32(&mut new_pool, 8, new_count as u32);
    p32(&mut new_pool, 12, pool.style_count as u32);
    p32(&mut new_pool, 16, pool.flags);
    p32(&mut new_pool, 20, new_strings_start as u32);
    p32(&mut new_pool, 24, styles_at as u32);
    let mut write_at = 28usize;
    for i in 0..pool.string_count {
        p32(&mut new_pool, write_at, pool.offsets[i]);
        write_at += 4;
    }
    for i in 0..added.len() {
        p32(&mut new_pool, write_at, (strings_blob.len() + appended_offsets[i]) as u32);
        write_at += 4;
    }
    new_pool[write_at..write_at + strings_blob.len()].copy_from_slice(strings_blob);
    write_at += strings_blob.len();
    for encoded in &appended {
        new_pool[write_at..write_at + encoded.len()].copy_from_slice(encoded);
        write_at += encoded.len();
    }
    new_pool[styles_at..styles_at + styles_blob.len()].copy_from_slice(styles_blob);

    // ---- reassemble the file and patch the attribute references -----------
    let tail_start = pool_start + pool_size;
    let tail_end = file_size.min(manifest.len());
    let tail = &manifest[tail_start..tail_end];

    let mut out = Vec::with_capacity(8 + new_pool_size + tail.len());
    let mut header = [0u8; 8];
    p16(&mut header, 0, RES_XML_TYPE);
    p16(&mut header, 2, 8);
    p32(&mut header, 4, (8 + new_pool_size + tail.len()) as u32);
    out.extend_from_slice(&header);
    out.extend_from_slice(&new_pool);
    let mut patched_tail = tail.to_vec();
    for &(data_offset, _raw, new_index) in &patches {
        // Kotlin (ManifestRenamer.kt:261) computes
        // `newAbs = tailStartNew + (dataOffset - tailStart)` and then indexes
        // `patchedTail[newAbs]` — but `patchedTail` is a copy of the tail,
        // indexed from 0, so the `tailStartNew +` term is an off-by slip: it
        // either skips the write (tail shorter than the rebuilt pool) or
        // lands `tailStartNew` bytes past the attribute. The attribute's
        // position inside the tail is `data_offset - tail_start`; the port
        // patches there — the only reading under which `Result.bytes`
        // actually carries the renamed package (which `ApkPatcher.kt`
        // depends on when it reports `finalPackage`).
        let at = data_offset - tail_start;
        if at + 4 <= patched_tail.len() {
            p32(&mut patched_tail, at, new_index as u32);
        }
    }
    out.extend_from_slice(&patched_tail);
    Some(RenameResult {
        bytes: out,
        package_name: new_package.to_string(),
        renamed: true,
    })
}

/// `ManifestRenamer.indexOfNew` — index of `value` in (pool ∪ added),
/// appending it when new.
fn index_of_new(pool: &StringPool, added: &mut Vec<String>, value: &str) -> usize {
    if let Some(existing) = pool.strings.iter().position(|s| s == value) {
        return existing;
    }
    if let Some(in_added) = added.iter().position(|s| s == value) {
        return pool.string_count + in_added;
    }
    added.push(value.to_string());
    pool.string_count + added.len() - 1
}

/// `ManifestRenamer.replacePackagePrefix`.
fn replace_package_prefix(value: &str, old: &str, new: &str) -> Option<String> {
    if value == old {
        return Some(new.to_string());
    }
    let prefix = format!("{old}.");
    if !value.starts_with(&prefix) {
        return None;
    }
    Some(format!("{new}{}", &value[old.len()..]))
}

/// `ManifestRenamer.renameAuthoritySegments`.
fn rename_authority_segments(value: &str, old: &str, new: &str) -> Option<String> {
    if !value.contains(old) {
        return None;
    }
    let mut changed = false;
    let out = value
        .split(';')
        .map(|seg| match replace_package_prefix(seg.trim(), old, new) {
            Some(replaced) => {
                changed = true;
                replaced
            }
            None => seg.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";");
    if changed {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use owen_apk::axml::{strings_at, UTF8_FLAG};

    /// Build a small UTF-16 AXML manifest with a `manifest` element carrying
    /// `package` + `versionName` string attributes and a `provider` element
    /// with `authorities`.
    fn build_manifest(package: &str, authorities: &str) -> Vec<u8> {
        let strings = [
            "manifest".to_string(),
            "package".to_string(),
            package.to_string(),
            "versionName".to_string(),
            "1.0".to_string(),
            "provider".to_string(),
            "authorities".to_string(),
            authorities.to_string(),
        ];
        // Encode UTF-16 strings (u16 len + units + NUL).
        let mut blob: Vec<u8> = Vec::new();
        let mut offsets = Vec::new();
        for s in &strings {
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
        p16(&mut pool, 0, RES_STRING_POOL_TYPE);
        p16(&mut pool, 2, 28);
        p32(&mut pool, 4, pool_size as u32);
        p32(&mut pool, 8, strings.len() as u32);
        p32(&mut pool, 12, 0);
        p32(&mut pool, 16, 0); // UTF-16
        p32(&mut pool, 20, strings_start as u32);
        p32(&mut pool, 24, pool_size as u32);
        for (i, off) in offsets.iter().enumerate() {
            p32(&mut pool, 28 + i * 4, *off as u32);
        }
        pool[strings_start..].copy_from_slice(&blob);

        // start element: type, headerSize=16, size, line, comment, ns, name,
        // attributeStart=20, attributeSize=20, attributeCount, ids…
        let element = |name_idx: usize, attrs: &[(usize, u8, u32, u32)]| -> Vec<u8> {
            let size = 16 + 20 + attrs.len() * 20;
            let mut c = vec![0u8; size];
            p16(&mut c, 0, RES_XML_START_ELEMENT_TYPE);
            p16(&mut c, 2, 16);
            p32(&mut c, 4, size as u32);
            p32(&mut c, 16, 0xFFFF_FFFF); // ns
            p32(&mut c, 20, name_idx as u32);
            p16(&mut c, 24, 20); // attributeStart
            p16(&mut c, 26, 20); // attributeSize
            p16(&mut c, 28, attrs.len() as u16);
            for (a, &(name_i, dtype, data, raw)) in attrs.iter().enumerate() {
                let at = 16 + 20 + a * 20;
                p32(&mut c, at, 0xFFFF_FFFF); // ns
                p32(&mut c, at + 4, name_i as u32);
                p32(&mut c, at + 8, raw);
                p16(&mut c, at + 12, 8); // typedValue size
                c[at + 14] = 0; // res0
                c[at + 15] = dtype;
                p32(&mut c, at + 16, data);
            }
            c
        };

        let mut chunks: Vec<u8> = Vec::new();
        chunks.extend_from_slice(&element(0, &[(1, 0x03, 2, 2), (3, 0x03, 4, 4)])); // manifest
        chunks.extend_from_slice(&element(5, &[(6, 0x03, 7, 7)])); // provider authorities

        let total = 8 + pool.len() + chunks.len();
        let mut out = vec![0u8; 8 + pool.len() + chunks.len()];
        p16(&mut out, 0, RES_XML_TYPE);
        p16(&mut out, 2, 8);
        p32(&mut out, 4, total as u32);
        out[8..8 + pool.len()].copy_from_slice(&pool);
        out[8 + pool.len()..].copy_from_slice(&chunks);
        out
    }

    #[test]
    fn reads_package_and_version_name() {
        let m = build_manifest("com.epicgames.fortnite", "com.epicgames.fortnite.files");
        assert_eq!(read_package_name(&m).as_deref(), Some("com.epicgames.fortnite"));
        assert_eq!(read_version_name(&m).as_deref(), Some("1.0"));
    }

    #[test]
    fn renames_package_and_authorities() {
        let m = build_manifest("com.epicgames.fortnite", "com.epicgames.fortnite.files");
        let r = rename(
            &m,
            "com.epicgames.fortnite",
            Some("com.epicgames.fortnite"),
            "com.owen.test1",
        )
        .expect("rename");
        assert!(r.renamed);
        assert_eq!(r.package_name, "com.owen.test1");
        assert_eq!(read_package_name(&r.bytes).as_deref(), Some("com.owen.test1"));
        // provider authorities follow the package rename
        let attr = read_authorities(&r.bytes);
        assert_eq!(attr.as_deref(), Some("com.owen.test1.files"));
    }

    fn read_authorities(m: &[u8]) -> Option<String> {
        // reuse read_manifest_string_attr shape but for provider/authorities
        if u16le(m, 0)? != RES_XML_TYPE {
            return None;
        }
        let file_size = u32le(m, 4)? as usize;
        let limit = file_size.min(m.len());
        let mut offset = 8;
        let mut pool_start = None;
        let mut pool_size = 0;
        while offset + 8 <= limit {
            let typ = u16le(m, offset)?;
            let cs = u32le(m, offset + 4)? as usize;
            if cs == 0 || offset + cs > m.len() {
                break;
            }
            if typ == RES_STRING_POOL_TYPE {
                pool_start = Some(offset);
                pool_size = cs;
                break;
            }
            offset += cs;
        }
        let pool = StringPool::parse_pool(&m[pool_start?..pool_start? + pool_size])?;
        let mut pos = pool_start? + pool_size;
        while pos + 8 <= limit {
            let typ = u16le(m, pos)?;
            let cs = u32le(m, pos + 4)? as usize;
            if cs == 0 || pos + cs > m.len() {
                break;
            }
            if typ == RES_XML_START_ELEMENT_TYPE {
                let name_index = u32le(m, pos + 20)? as usize;
                let element = pool.strings.get(name_index).cloned().unwrap_or_default();
                if element == "provider" {
                    let attribute_start = u16le(m, pos + 24)? as usize;
                    let attribute_size = u16le(m, pos + 26)? as usize;
                    let attribute_count = u16le(m, pos + 28)? as usize;
                    let attr_base = pos + 16 + attribute_start;
                    for a in 0..attribute_count {
                        let attr_at = attr_base + a * attribute_size;
                        if attr_at + 20 > m.len() {
                            break;
                        }
                        let name_index = u32le(m, attr_at + 4)? as usize;
                        let name = pool.strings.get(name_index).cloned().unwrap_or_default();
                        if name == "authorities" && m[attr_at + 15] == TYPE_STRING {
                            let value_index = u32le(m, attr_at + 16)? as usize;
                            return pool.strings.get(value_index).cloned();
                        }
                    }
                }
            }
            pos += cs;
        }
        None
    }

    #[test]
    fn rename_no_hit_returns_unchanged() {
        let m = build_manifest("com.other.game", "com.other.game.files");
        let r = rename(&m, "com.epicgames.fortnite", Some("com.epicgames.fortnite"), "com.owen.x")
            .expect("rename parses");
        assert!(!r.renamed);
        assert_eq!(r.package_name, "unknown");
        assert_eq!(r.bytes, m);
    }

    #[test]
    fn authority_segment_helpers() {
        assert_eq!(
            rename_authority_segments("a.b.c;x.y", "a.b", "n.m").as_deref(),
            Some("n.m.c;x.y")
        );
        assert_eq!(replace_package_prefix("a.b", "a.b", "n.m").as_deref(), Some("n.m"));
        assert_eq!(replace_package_prefix("a.b.c", "a.b", "n.m").as_deref(), Some("n.m.c"));
        assert_eq!(replace_package_prefix("x.b", "a.b", "n.m"), None);
    }

    #[test]
    fn strings_at_parses_pool_in_place() {
        let m = build_manifest("com.epicgames.fortnite", "com.epicgames.fortnite.files");
        // the pool chunk is the second chunk (offset 8)
        let strings = strings_at(&m, 8).expect("strings_at");
        assert!(strings.iter().any(|s| s == "com.epicgames.fortnite"));
        assert_eq!(strings[0], "manifest");
    }

    #[test]
    fn encode_string_matches_pool_format() {
        // UTF-16: u16 len + units + NUL
        let e = encode_string("abc", false);
        assert_eq!(e, vec![3, 0, b'a', 0, b'b', 0, b'c', 0, 0, 0]);
        // UTF-8: charLen, byteLen, bytes, NUL
        let e8 = encode_string("abc", true);
        assert_eq!(e8, vec![3, 3, b'a', b'b', b'c', 0]);
    }

    #[test]
    fn utf8_flag_helper() {
        assert_eq!(UTF8_FLAG, 0x100);
    }
}
