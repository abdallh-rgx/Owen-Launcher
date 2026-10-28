//! libOwen.so preparation — the literal Rust port of `NativeBuilder.kt`:
//! binary-patch the packaged library's default backend URL, then embed it in
//! the APK.
//!
//! ## Kotlin source map
//!
//! | Kotlin (`NativeBuilder.kt`)                  | Rust (here)                        |
//! |----------------------------------------------|------------------------------------|
//! | `DEFAULT_URL = "http://127.0.0.1:3551"`      | [`DEFAULT_URL`]                    |
//! | `URL_CAPACITY = 4096`                        | [`URL_CAPACITY`]                   |
//! | `buildOwen(context, backendUrl)`             | [`build_owen`]                     |
//! | `patchUrlInLibrary(source, dest, newUrl)`    | [`patch_url_in_library`]           |
//! | `indexOfPattern(data, pattern, from)`        | [`index_of_pattern`]               |
//! | `sanitizeUrl(url)`                           | [`sanitize_url`]                   |
//! | `verifyPatchedLibrary(file, expectedUrl)`    | [`verify_patched_library`]         |
//! | `context.applicationInfo.nativeLibraryDir`   | [`locate`] (jniLibs / work files — the Rust library never shells out to cargo) |
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/NativeBuilder.kt

use std::path::{Path, PathBuf};

pub const DEFAULT_URL: &str = "http://127.0.0.1:3551";
pub const URL_CAPACITY: usize = 4096;

/// `NativeBuilder.buildOwen` — sanitize the URL, load the prebuilt
/// `libowen.so` and patch the URL into it. Returns the patched bytes (the
/// Kotlin wrote `libOwen_patched.so` into the cache dir; the caller here
/// feeds the bytes straight into the patch pipeline or writes them itself).
pub fn build_owen(source: &Path, backend_url: &str) -> Result<Vec<u8>, String> {
    let url = sanitize_url(backend_url);
    if url.is_empty() {
        return Err(format!("Invalid backend URL: {backend_url}"));
    }
    let data = std::fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
    patch_url_in_library(&data, &url)
}

/// `NativeBuilder.patchUrlInLibrary` — replace every `DEFAULT_URL\0` (or, as
/// a fallback, `DEFAULT_URL` without the NUL) occurrence with `newUrl\0`,
/// zero-padding up to the wider of the two patterns.
pub fn patch_url_in_library(source: &[u8], new_url: &str) -> Result<Vec<u8>, String> {
    let mut replacement = new_url.as_bytes().to_vec();
    replacement.push(0);
    if replacement.len() > URL_CAPACITY {
        return Err("URL too long (max 4095 bytes)".to_string());
    }
    let mut data = source.to_vec();
    let mut replaced = 0usize;
    let patterns: Vec<Vec<u8>> = vec![
        [DEFAULT_URL.as_bytes(), &[0u8][..]].concat(),
        DEFAULT_URL.as_bytes().to_vec(),
    ];
    for pattern in &patterns {
        let width = pattern.len().max(replacement.len());
        let mut at = index_of_pattern(&data, pattern, 0);
        while let Some(pos) = at {
            for i in 0..width {
                if let Some(slot) = data.get_mut(pos + i) {
                    *slot = if i < replacement.len() { replacement[i] } else { 0 };
                }
            }
            replaced += 1;
            at = index_of_pattern(&data, pattern, pos + width);
        }
        if replaced > 0 {
            break;
        }
    }
    if replaced == 0 {
        return Err("Default backend URL pattern not found in libOwen.so".to_string());
    }
    Ok(data)
}

/// `NativeBuilder.indexOfPattern` — naive first-match search from `from`.
fn index_of_pattern(data: &[u8], pattern: &[u8], from: usize) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }
    let start = from.min(data.len() - pattern.len());
    for i in start..=data.len() - pattern.len() {
        if data[i] == pattern[0] && data[i..i + pattern.len()] == pattern[..] {
            return Some(i);
        }
    }
    None
}

/// `NativeBuilder.sanitizeUrl` — trim, ensure a scheme, enforce the capacity.
pub fn sanitize_url(url: &str) -> String {
    let mut cleaned = url.trim().to_string();
    if cleaned.is_empty() {
        return String::new();
    }
    if !cleaned.starts_with("http://") && !cleaned.starts_with("https://") {
        cleaned = format!("http://{cleaned}");
    }
    if cleaned.len() + 1 <= URL_CAPACITY {
        cleaned
    } else {
        String::new()
    }
}

/// `NativeBuilder.verifyPatchedLibrary` — the expected URL must appear
/// verbatim in the patched image.
pub fn verify_patched_library(data: &[u8], expected_url: &str) -> bool {
    index_of_pattern(data, expected_url.as_bytes(), 0).is_some()
}

/// Locate the prebuilt `libowen.so` — the Rust replacement for reading
/// `context.applicationInfo.nativeLibraryDir`. The library NEVER invokes
/// cargo itself; it only looks at well-known locations:
///
/// 1. `$OWEN_LIB` (explicit override),
/// 2. `jniLibs/arm64-v8a/libowen.so` (project jniLibs layout, both names),
/// 3. `target/aarch64-linux-android/release/libowen.so` (work files from a
///    prior `cargo ndk` build),
/// 4. `libowen.so` in the current directory.
pub fn locate() -> Option<PathBuf> {
    if let Ok(env) = std::env::var("OWEN_LIB") {
        let p = PathBuf::from(env);
        if p.is_file() && p.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Some(p);
        }
    }
    let candidates = [
        "jniLibs/arm64-v8a/libowen.so",
        "jniLibs/arm64-v8a/libOwen.so",
        "target/aarch64-linux-android/release/libowen.so",
        "libowen.so",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file() && p.metadata().map(|m| m.len() > 0).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_lib() -> Vec<u8> {
        let mut data = vec![0u8; 8192];
        // The default URL buffer (URL + NUL + zero padding), like the
        // redirecter's DEFAULT_BACKEND_URL static.
        data[..DEFAULT_URL.len()].copy_from_slice(DEFAULT_URL.as_bytes());
        data[DEFAULT_URL.len()] = 0;
        data
    }

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_url("http://a.b"), "http://a.b");
        assert_eq!(sanitize_url("https://a.b"), "https://a.b");
        assert_eq!(sanitize_url("  a.b "), "http://a.b");
        assert_eq!(sanitize_url("   "), "");
        assert_eq!(sanitize_url(&"x".repeat(5000)), "");
    }

    #[test]
    fn patches_and_verifies_url() {
        let lib = fake_lib();
        let patched = patch_url_in_library(&lib, "http://192.168.1.10:3551").expect("patch");
        assert!(verify_patched_library(&patched, "http://192.168.1.10:3551"));
        assert!(!verify_patched_library(&patched, DEFAULT_URL));
        // everything after the replacement is zero-padded
        let replaced = b"http://192.168.1.10:3551\0";
        assert!(patched[..replaced.len()].to_vec() == replaced.to_vec());
        assert!(patched[replaced.len()..DEFAULT_URL.len() + 8].iter().all(|&b| b == 0));

        // build_owen path: sanitize + read + patch
        let dir = std::env::temp_dir().join(format!("owen-native-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("libowen.so");
        std::fs::write(&path, &lib).unwrap();
        let built = build_owen(&path, "192.168.1.10:3551").expect("build");
        assert!(verify_patched_library(&built, "http://192.168.1.10:3551"));

        // no pattern → error (Kotlin IllegalStateException)
        let clean = vec![1u8; 64];
        assert!(patch_url_in_library(&clean, "http://x").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pattern_search_bounds() {
        assert_eq!(index_of_pattern(b"abc", b"abc", 0), Some(0));
        assert_eq!(index_of_pattern(b"abcabc", b"abc", 1), Some(3));
        assert_eq!(index_of_pattern(b"abc", b"abd", 0), None);
        assert_eq!(index_of_pattern(b"ab", b"abc", 0), None);
        assert_eq!(index_of_pattern(b"abc", b"", 0), None);
    }
}
