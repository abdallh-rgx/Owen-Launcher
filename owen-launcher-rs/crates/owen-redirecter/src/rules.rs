//! Redirect rules for the in-process redirecter.
//!
//! Mirrors [`owen_core::urlrule`] but keeps a copy inside the library so
//! hook callbacks (which must never block on locks) read plain statics.
//! The configuration is resolved once at load time:
//!
//! 1. `/data/local/tmp/owen.conf` (single line: backend URL), when
//!    present — lets you re-point the game without re-patching.
//! 2. A URL baked at patch time (the patcher rewrites the placeholder
//!    bytes inside the .so).
//! 3. `http://127.0.0.1:3551` (on-device backend default).

use std::sync::OnceLock;

/// Placeholder the patcher rewrites. Keep in sync with owen-patcher.
pub const BACKEND_PLACEHOLDER: &str = "http://127.0.0.1:3551";
const CONF_PATH: &str = "/data/local/tmp/owen.conf";

pub fn backend_url() -> &'static str {
    static BACKEND: OnceLock<String> = OnceLock::new();
    BACKEND.get_or_init(|| {
        // 1. runtime override file
        if let Ok(text) = std::fs::read_to_string(CONF_PATH) {
            let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
            if line.starts_with("http://") || line.starts_with("https://") {
                return line.trim_end_matches('/').to_string();
            }
        }
        // 2. baked placeholder (the compiler keeps it contiguous; the
        //    patcher finds and rewrites it in the .so image)
        BACKEND_PLACEHOLDER.trim_end_matches('/').to_string()
    })
}

pub fn login_url() -> String {
    format!("{}/login", backend_url())
}

pub fn partyhub_graphql_url() -> String {
    format!("{}/partyhub/graphql", backend_url())
}

pub fn register_url() -> String {
    format!("{}/register", backend_url())
}

/// Host the backend serves itself on (used by `getHostName`).
pub fn host_name() -> String {
    let url = owen_core::urlrule::Url::parse(backend_url());
    url.host
}

/// Domains that get rewritten. Kept identical to the server defaults.
pub fn should_redirect(url: &str) -> bool {
    static RULES: OnceLock<owen_core::urlrule::RedirectRules> = OnceLock::new();
    let rules = RULES.get_or_init(|| {
        let mut r = owen_core::urlrule::RedirectRules::default();
        r.backend_url = backend_url().to_string();
        r
    });
    rules.should_redirect(url)
}

/// Rewrite a URL if it matches the rules; `None` means pass through.
pub fn rewrite(url: &str) -> Option<String> {
    static RULES: OnceLock<owen_core::urlrule::RedirectRules> = OnceLock::new();
    let rules = RULES.get_or_init(|| {
        let mut r = owen_core::urlrule::RedirectRules::default();
        r.backend_url = backend_url().to_string();
        r
    });
    rules.rewrite(url)
}
