//! Embedded login page asset.
//!
//! Served at `/login` — the target the Kairos WebView gets redirected to by
//! the Owen redirecter (no smali edits, the redirect happens via ART hooks
//! inside the game process). The HTML lives in `assets/login.html` so the
//! page can be redesigned without touching Rust code.

pub const LOGIN_PAGE: &str = include_str!("../assets/login.html");
