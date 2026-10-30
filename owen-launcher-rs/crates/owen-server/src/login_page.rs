//! Embedded login page assets.
//!
//! `LOGIN_PAGE_1241` is served at `/login` — the page the game's MCP identity
//! opens in its in-game webview (`LoginPortalUrl`). It self-completes the
//! 12.41 handshake: password grant → exchange code → redirect the webview to
//! `{RedirectUrl}/exchange?exchangeCode=…&redirectUrl=…`, which the identity
//! layer intercepts and redeems with grant_type=exchange_code.
//!
//! `LOGIN_PAGE` (the legacy Kairos/PartyHub page) is kept for the WebView
//! redirect layer; the redirect happens via ART hooks inside the game
//! process, no smali edits.

pub const LOGIN_PAGE: &str = include_str!("../assets/login.html");

/// The 12.41 self-completing login page (same file, distinct constant so the
/// legacy Kairos flow can keep its own copy later without a rewrite).
pub const LOGIN_PAGE_1241: &str = include_str!("../assets/login.html");
