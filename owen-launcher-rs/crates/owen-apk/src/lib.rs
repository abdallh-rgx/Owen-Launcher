//! owen-apk — APK surgery for the Owen pipeline, in safe Rust:
//!
//! * `zip` — minimal STORED-only zip reader/writer (no dependencies).
//! * `axml` — binary AndroidManifest editor: rename the package / raise
//!   minSdk/targetSdk by editing the string pool in place (the Rust port of
//!   the original `ManifestRenamer.kt`).
//! * `needed` — **zero-smali loader injection**: instead of patching dex,
//!   `libowen.so` is added to `lib/arm64-v8a/` and written into the dynamic
//!   section of the engine library as a `DT_NEEDED` dependency. The Android
//!   linker then loads Owen before the game engine itself runs — no smali,
//!   no dex, no bytecode edits.
//!
//! Signing intentionally reuses the project's existing keystore + signer
//! step (the original pipeline is proven end to end); the Rust signing
//! module lands separately.

#![forbid(unsafe_code)]

pub mod axml;
pub mod needed;
pub mod zip;
