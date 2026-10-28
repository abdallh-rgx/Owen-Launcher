//! Redirect hook implementations (Android-only bodies).

#[cfg(target_os = "android")]
pub mod art;
#[cfg(target_os = "android")]
pub mod curl;
#[cfg(target_os = "android")]
pub mod hooks;
#[cfg(target_os = "android")]
pub mod kairos;
#[cfg(target_os = "android")]
pub mod webview;
