//! # libOwen — the Owen redirecter (safe-Rust core, contained unsafe for
//! the hook trampolines).
//!
//! ## What it does, without touching a single line of smali
//!
//! 1. **Waits for the engine library** (`libUE4.so` / `libUnreal.so`) to
//!    appear in `/proc/self/maps`.
//! 2. **Detects the game version** from the release string embedded in
//!    the binary (ASCII or UTF-16LE).
//! 3. **Locates `curl_easy_setopt`** — by exported symbol on `libUE4.so`,
//!    otherwise by the automatic four-layer scanner (the Rust port of the
//!    reference Python tool), which works on stripped images
//!    (`libUnreal.so`, `libEOSSDK.so`).
//! 4. **Hooks `curl_easy_setopt`** and rewrites every `CURLOPT_URL`
//!    matching the Epic domain list to the local backend. Both `http://`
//!    and `https://` backends are supported — the scheme is preserved.
//! 5. **Kairos era (10.31-15.50)**: ART-hooks the WebView layer —
//!    `DebouncedLimiter.loginUrl`, `getGraphqlUrl`, `registerUrl`,
//!    `getHostName` and `WebView.loadUrl` — redirecting the login page,
//!    the registration page and **PartyHub graphql** to the backend.
//! 6. Reports the detected version back to the backend
//!    (`/api/owen/hello`), so the panel can show what is running.
//!
//! ## Loading
//!
//! The library is pulled into the game process by DT_NEEDED injection
//! (see `owen-patcher`): `libUE4.so`/`libUnreal.so` gets a dependency on
//! `libOwen.so`, and the Android linker loads it before any game code
//! runs. No `classes.dex` modification is involved anywhere.

#[cfg(target_os = "android")]
mod android;

pub mod detect;
pub mod procmaps;
pub mod redirect;
pub mod rules;

#[cfg(target_os = "android")]
pub use android::entry;

/// Build information reported to the backend.
pub const OWEN_VERSION: &str = env!("CARGO_PKG_VERSION");
