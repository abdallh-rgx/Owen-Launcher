#![forbid(unsafe_code)]
//! Uptodown layer — 1:1 port of
//! `com.abdallhrgx.owenlauncher.core.uptodown` from the Kotlin original:
//!
//! | Kotlin (Owen-Launcher)                        | Rust (here)                      |
//! |-----------------------------------------------|----------------------------------|
//! | `UptodownApi.kt` (object)                     | [`api::UptodownApi`]             |
//! | `UptodownResolver.kt` (object)                | [`resolver::UptodownResolver`]   |
//! | `UptodownVersion.kt` (data class)             | [`version::UptodownVersion`]     |
//! | `VersionRules.kt` (object)                    | [`rules::VersionRules`]          |
//! | `UptodownInstaller.kt` (object)               | [`installer::UptodownInstaller`] |
//!
//! The hidden `WebView` of `UptodownResolver` is replaced by the
//! [`crate::http::HttpBackend`] with the same literal matching rules
//! (`detail-download-button[-xapk]`, `data-url`, `dw.uptodown.com`).

pub mod api;
pub mod installer;
pub mod resolver;
pub mod rules;
pub mod version;

pub use api::UptodownApi;
pub use installer::{DownloadEvent, InstallState, OgVersion, UptodownInstaller};
pub use resolver::UptodownResolver;
pub use rules::VersionRules;
pub use version::UptodownVersion;
