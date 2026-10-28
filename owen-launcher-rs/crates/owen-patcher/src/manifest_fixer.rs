//! Binary-AXML `uses-sdk` floor fixer — the literal Rust port of
//! `ManifestFixer.kt`.
//!
//! Raises `minSdkVersion` to 24 and `targetSdkVersion` to 29 (in place, no
//! rebuild), and reports whether the app icon has an `anydpi-v26` entry in
//! `resources.arsc` (legacy icon warning). Everything is binary AXML — no
//! textual manifest is ever produced.
//!
//! ## Kotlin source map
//!
//! | Kotlin (`ManifestFixer.kt`)                            | Rust (here)                       |
//! |--------------------------------------------------------|-----------------------------------|
//! | `Result(bytes, minSdk, targetSdk, changed, iconRedirected)` | [`FixResult`]                |
//! | `fix(manifest, arsc)`                                  | [`fix`]                           |
//! | `fixInternal`                                          | [`fix_internal`]                  |
//! | `ArscInspector.hasRegisteredAnydpiV26`                 | [`ArscInspector::has_registered_anydpi_v26`] |
//! | `ArscInspector.scanPackageTypes`                       | [`ArscInspector::scan_package_types`] |
//! | `ArscInspector.typeHasAnydpiV26Entry`                  | [`ArscInspector::type_has_anydpi_v26_entry`] |
//! | `ArscInspector.globalPool` (static field)              | `ArscInspector.global_pool` (struct field) |
//! | `ManifestRenamerStrings.parse(manifest, pos, size)`    | [`owen_apk::axml::strings_at`]    |
//! | `u16`/`u32`/`p32`                                      | [`u16le`]/[`u32le`]/[`p32`]       |
//!
//! Kotlin `catch (t) → Result(manifest, -1, -1, false, false)` (keep the
//! original manifest) becomes `fix_internal` returning `None`.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/ManifestFixer.kt

use owen_apk::axml::{strings_at, RES_STRING_POOL_TYPE, RES_XML_START_ELEMENT_TYPE, RES_XML_TYPE};

const MIN_SDK_FLOOR: u32 = 24;
const TARGET_SDK_FLOOR: u32 = 29;
const TYPE_INT_DEC: u8 = 0x10;
const TYPE_REFERENCE: u8 = 0x01;

/// `ManifestFixer.Result`.
#[derive(Debug, Clone)]
pub struct FixResult {
    pub bytes: Vec<u8>,
    pub min_sdk: i32,
    pub target_sdk: i32,
    pub changed: bool,
    pub icon_redirected: bool,
}

impl FixResult {
    /// The Kotlin failure fallback: keep the original manifest untouched.
    fn original(manifest: Vec<u8>) -> Self {
        FixResult {
            bytes: manifest,
            min_sdk: -1,
            target_sdk: -1,
            changed: false,
            icon_redirected: false,
        }
    }
}

/// `ManifestFixer.fix(manifest: ByteArray, arsc: ByteArray?): Result`
///
/// Takes the manifest by value and returns it (possibly patched) inside
/// the result — the Kotlin original mutated the same array in place.
pub fn fix(manifest: Vec<u8>, arsc: Option<&[u8]>) -> FixResult {
    match fix_internal(manifest.clone(), arsc) {
        Some(result) => result,
        None => FixResult::original(manifest),
    }
}

/// `ManifestFixer.fixInternal`.
fn fix_internal(mut manifest: Vec<u8>, arsc: Option<&[u8]>) -> Option<FixResult> {
    if u16le(&manifest, 0)? != RES_XML_TYPE {
        return Some(FixResult::original(manifest));
    }
    let file_size = u32le(&manifest, 4)? as usize;
    let mut min_sdk: i32 = -1;
    let mut target_sdk: i32 = -1;
    let mut changed = false;
    let icon_redirected = false;
    let mut strings: Vec<String> = Vec::new();
    let mut pos = 8usize;
    let mut scanned = 0usize;
    while pos + 8 <= file_size.min(manifest.len()) && scanned < 65_536 {
        scanned += 1;
        let typ = u16le(&manifest, pos)?;
        let chunk_size = u32le(&manifest, pos + 4)? as usize;
        if chunk_size == 0 || pos + chunk_size > manifest.len() {
            break;
        }
        if typ == RES_STRING_POOL_TYPE {
            strings.extend(strings_at(&manifest, pos)?);
        } else if typ == RES_XML_START_ELEMENT_TYPE {
            let name_index = u32le(&manifest, pos + 20)? as usize;
            let element_name = strings.get(name_index).cloned().unwrap_or_default();
            let attribute_start = u16le(&manifest, pos + 24)? as usize;
            let attribute_size = u16le(&manifest, pos + 26)? as usize;
            let attribute_count = u16le(&manifest, pos + 28)? as usize;
            let attr_base = pos + 16 + attribute_start;
            for a in 0..attribute_count {
                let attr_at = attr_base + a * attribute_size;
                if attr_at + 20 > manifest.len() {
                    break;
                }
                // Kotlin: val attrName = strings.getOrNull(...) ?: continue
                let Some(attr_name) = strings.get(u32le(&manifest, attr_at + 4)? as usize) else {
                    continue;
                };
                let attr_name = attr_name.clone();
                let data_type = manifest[attr_at + 15];
                let data_at = attr_at + 16;
                if element_name == "uses-sdk" && attr_name == "minSdkVersion" && data_type == TYPE_INT_DEC {
                    let current = u32le(&manifest, data_at)?;
                    min_sdk = current as i32;
                    if current < MIN_SDK_FLOOR {
                        p32(&mut manifest, data_at, MIN_SDK_FLOOR);
                        min_sdk = MIN_SDK_FLOOR as i32;
                        changed = true;
                    }
                } else if element_name == "uses-sdk"
                    && attr_name == "targetSdkVersion"
                    && data_type == TYPE_INT_DEC
                {
                    let current = u32le(&manifest, data_at)?;
                    target_sdk = current as i32;
                    if current < TARGET_SDK_FLOOR {
                        p32(&mut manifest, data_at, TARGET_SDK_FLOOR);
                        target_sdk = TARGET_SDK_FLOOR as i32;
                        changed = true;
                    }
                } else if element_name == "application"
                    && attr_name == "icon"
                    && data_type == TYPE_REFERENCE
                {
                    let res_id = u32le(&manifest, data_at)?;
                    if let Some(arsc_bytes) = arsc {
                        if !ArscInspector::has_registered_anydpi_v26(arsc_bytes, res_id) {
                            // Kotlin logs: "App icon has no anydpi-v26 entry
                            // (@0x…) — legacy icon used on O+".
                        }
                    }
                }
            }
        }
        pos += chunk_size;
    }
    Some(FixResult {
        bytes: manifest,
        min_sdk,
        target_sdk,
        changed,
        icon_redirected,
    })
}

/// `ManifestFixer.ArscInspector` — reads `resources.arsc` looking for a
/// registered `anydpi-v26` (density 0xFFFE, sdk ≥ 26) icon entry.
struct ArscInspector {
    /// Kotlin kept this as a static field; here it is per-scan state.
    global_pool: Vec<String>,
}

const NO_ENTRY: u32 = 0xFFFF_FFFF;
const RES_TABLE_TYPE: u16 = 2;
const RES_PACKAGE_TYPE: u16 = 0x0200;
const RES_TABLE_TYPE_TYPE: u16 = 0x0201;

impl ArscInspector {
    /// `ArscInspector.hasRegisteredAnydpiV26(arsc, resId)`.
    fn has_registered_anydpi_v26(arsc: &[u8], res_id: u32) -> bool {
        let package_id = (res_id >> 24) & 0xFF;
        let type_id = (res_id >> 16) & 0xFF;
        let entry_id = res_id & 0xFFFF;
        // Kotlin: if ((resId.toLong() and -0x1000000L) != 0x7F000000L && packageId == 0) return false
        if (res_id & 0xFF00_0000) != 0x7F00_0000 && package_id == 0 {
            return false;
        }
        let mut inspector = ArscInspector {
            global_pool: Vec::new(),
        };
        inspector.has_registered_anydpi_v26_inner(arsc, package_id as u8, type_id as u8, entry_id)
    }

    fn has_registered_anydpi_v26_inner(
        &mut self,
        arsc: &[u8],
        package_id: u8,
        type_id: u8,
        entry_id: u32,
    ) -> bool {
        let check = (|| -> Option<bool> {
            if u16le(arsc, 0)? != RES_TABLE_TYPE {
                return Some(false);
            }
            let header_size = u16le(arsc, 2)? as usize;
            self.global_pool = if u16le(arsc, header_size)? == RES_STRING_POOL_TYPE {
                strings_at(arsc, header_size)?
            } else {
                Vec::new()
            };
            let table_size = u32le(arsc, 4)? as usize;
            let mut at = header_size + Self::global_pool_size(arsc, header_size);
            while at + 12 <= table_size.min(arsc.len()) {
                let chunk_type = u16le(arsc, at)?;
                let chunk_size = u32le(arsc, at + 4)? as usize;
                if chunk_size == 0 || at + chunk_size > arsc.len() {
                    break;
                }
                if chunk_type == RES_PACKAGE_TYPE && *arsc.get(at + 8)? == package_id {
                    return Some(self.scan_package_types(arsc, at, chunk_size, type_id, entry_id));
                }
                at += chunk_size;
            }
            Some(false)
        })();
        check.unwrap_or(false)
    }

    /// `ArscInspector.globalPoolSize`.
    fn global_pool_size(arsc: &[u8], off: usize) -> usize {
        if u16le(arsc, off) == Some(RES_STRING_POOL_TYPE) {
            u32le(arsc, off + 4).unwrap_or(0) as usize
        } else {
            0
        }
    }

    /// `ArscInspector.scanPackageTypes`.
    fn scan_package_types(
        &mut self,
        arsc: &[u8],
        pkg_start: usize,
        pkg_size: usize,
        type_id: u8,
        entry_id: u32,
    ) -> bool {
        let pkg_end = pkg_start + pkg_size;
        let mut at = pkg_start + u16le(arsc, pkg_start + 2).unwrap_or(0) as usize;
        while at + 8 <= pkg_end {
            let chunk_type = u16le(arsc, at).unwrap_or(0);
            let chunk_size = u32le(arsc, at + 4).unwrap_or(0) as usize;
            if chunk_size < 8 || at + chunk_size > pkg_end {
                return false;
            }
            if chunk_type == RES_TABLE_TYPE_TYPE && *arsc.get(at + 8).unwrap_or(&0) == type_id {
                if self.type_has_anydpi_v26_entry(arsc, at, chunk_size, entry_id) {
                    return true;
                }
            }
            at += chunk_size;
        }
        false
    }

    /// `ArscInspector.typeHasAnydpiV26Entry`.
    fn type_has_anydpi_v26_entry(
        &self,
        arsc: &[u8],
        chunk_start: usize,
        chunk_size: usize,
        entry_id: u32,
    ) -> bool {
        let check = (|| -> Option<bool> {
            let header_size = u16le(arsc, chunk_start + 2)? as usize;
            let entry_count = u32le(arsc, chunk_start + 12)?;
            let entries_start = u32le(arsc, chunk_start + 16)?;
            if entry_id >= entry_count || u32le(arsc, chunk_start + 20)? < 28 {
                return Some(false);
            }
            let density = u16le(arsc, chunk_start + 34)?;
            let sdk_version = u16le(arsc, chunk_start + 44)?;
            if density != 0xFFFE || sdk_version < 26 {
                return Some(false);
            }
            let entry_offset_at = chunk_start + header_size + entry_id as usize * 4;
            if entry_offset_at + 4 > chunk_start + chunk_size {
                return Some(false);
            }
            let entry_offset = u32le(arsc, entry_offset_at)?;
            if entry_offset == NO_ENTRY {
                return Some(false);
            }
            let entry_at = chunk_start + entries_start as usize + entry_offset as usize;
            if entry_at + 8 > arsc.len()
                || u16le(arsc, entry_at + 2)? & 1 != 0
                || entry_at + 16 > arsc.len()
            {
                return Some(false);
            }
            let data_type = *arsc.get(entry_at + 11)?;
            let data = u32le(arsc, entry_at + 12)?;
            if data_type == 3 {
                if let Some(name) = self.global_pool.get(data as usize) {
                    let lower = name.to_ascii_lowercase();
                    return Some(
                        lower.ends_with(".xml") || lower.ends_with(".webp") || lower.ends_with(".png"),
                    );
                }
            }
            Some(false)
        })();
        check.unwrap_or(false)
    }
}

fn u16le(b: &[u8], o: usize) -> Option<u16> {
    let s = b.get(o..o + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    let s = b.get(o..o + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn p32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal UTF-16 AXML with a `uses-sdk` element carrying integer
    /// `minSdkVersion` / `targetSdkVersion` attributes.
    fn build_manifest(min_sdk: u32, target_sdk: u32) -> Vec<u8> {
        let strings = ["uses-sdk".to_string(), "minSdkVersion".to_string(), "targetSdkVersion".to_string()];
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
        p32(&mut pool, 0, RES_STRING_POOL_TYPE as u32);
        p16(&mut pool, 2, 28);
        p32(&mut pool, 4, pool_size as u32);
        p32(&mut pool, 8, strings.len() as u32);
        p32(&mut pool, 12, 0);
        p32(&mut pool, 16, 0);
        p32(&mut pool, 20, strings_start as u32);
        p32(&mut pool, 24, pool_size as u32);
        for (i, off) in offsets.iter().enumerate() {
            p32(&mut pool, 28 + i * 4, *off as u32);
        }
        pool[strings_start..].copy_from_slice(&blob);

        let attrs: [(usize, u8, u32); 2] = [(1, TYPE_INT_DEC, min_sdk), (2, TYPE_INT_DEC, target_sdk)];
        let size = 16 + 20 + attrs.len() * 20;
        let mut c = vec![0u8; size];
        p16(&mut c, 0, RES_XML_START_ELEMENT_TYPE);
        p16(&mut c, 2, 16);
        p32(&mut c, 4, size as u32);
        p32(&mut c, 16, 0xFFFF_FFFF);
        p32(&mut c, 20, 0); // "uses-sdk"
        p16(&mut c, 24, 20);
        p16(&mut c, 26, 20);
        p16(&mut c, 28, attrs.len() as u16);
        for (a, &(name_i, dtype, data)) in attrs.iter().enumerate() {
            let at = 16 + 20 + a * 20;
            p32(&mut c, at, 0xFFFF_FFFF);
            p32(&mut c, at + 4, name_i as u32);
            p32(&mut c, at + 8, 0xFFFF_FFFF); // rawValue = NO_INDEX for ints
            p16(&mut c, at + 12, 8);
            c[at + 15] = dtype;
            p32(&mut c, at + 16, data);
        }

        let total = 8 + pool.len() + c.len();
        let mut out = vec![0u8; total];
        p16(&mut out, 0, RES_XML_TYPE);
        p16(&mut out, 2, 8);
        p32(&mut out, 4, total as u32);
        out[8..8 + pool.len()].copy_from_slice(&pool);
        out[8 + pool.len()..].copy_from_slice(&c);
        out
    }

    fn p16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn raises_sdk_floors() {
        let m = build_manifest(23, 28);
        let r = fix(m, None);
        assert!(r.changed);
        assert_eq!(r.min_sdk, 24);
        assert_eq!(r.target_sdk, 29);
        // Verify the actual bytes were raised.
        let fixed = r.bytes;
        let result2 = fix(fixed, None);
        assert!(!result2.changed, "already at the floors");
        assert_eq!(result2.min_sdk, 24);
        assert_eq!(result2.target_sdk, 29);
    }

    #[test]
    fn keeps_high_values() {
        let r = fix(build_manifest(28, 33), None);
        assert!(!r.changed);
        assert_eq!(r.min_sdk, 28);
        assert_eq!(r.target_sdk, 33);
    }

    #[test]
    fn garbage_manifest_is_kept_verbatim() {
        let garbage = b"not-an-axml".to_vec();
        let r = fix(garbage.clone(), None);
        assert!(!r.changed);
        assert_eq!(r.min_sdk, -1);
        assert_eq!(r.target_sdk, -1);
        assert_eq!(r.bytes, garbage);
    }

    #[test]
    fn fake_arsc_reports_no_anydpi() {
        // A non-arsc blob: header magic wrong → false (legacy icon).
        assert!(!ArscInspector::has_registered_anydpi_v26(b"garbage", 0x7F00_0001));
    }
}
