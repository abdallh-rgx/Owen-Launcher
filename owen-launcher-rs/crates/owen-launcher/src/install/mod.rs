#![forbid(unsafe_code)]
//! Install layer — 1:1 port of
//! `com.abdallhrgx.owenlauncher.core.patch` (the install plumbing of the
//! Kotlin original):
//!
//! | Kotlin (Owen-Launcher)                          | Rust (here)                     |
//! |--------------------------------------------------|---------------------------------|
//! | `PackageInstallHelper.kt` (object)               | [`helper::Installer`] trait + [`helper::InstallSession`] |
//! | `PackageInstallHelper.InstallResult`             | [`helper::InstallResult`]       |
//! | `InstallResultReceiver.kt` (BroadcastReceiver)   | [`result_receiver::InstallResultBus`] |
//!
//! No Android imports: the OS-facing implementation (JNI) arrives later;
//! [`helper::NoopInstaller`] covers desktop / preview builds.

pub mod helper;
pub mod result_receiver;

pub use helper::{InstallResult, InstallSession, Installer, NoopInstaller};
pub use result_receiver::{
    InstallBroadcast, InstallResultBus, InstallResultCallback, PackageInstallStatus, UserAction,
    STATUS_FAILURE, STATUS_PENDING_USER_ACTION, STATUS_SUCCESS,
};
