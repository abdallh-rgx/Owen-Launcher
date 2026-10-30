//! URL parsing and the redirect rule engine.
//!
//! Key design point (requested feature): **both `http://` and `https://` are
//! accepted everywhere.** The engine never forces one scheme. When the user
//! provides a bare host, a smart default picks `http://` for loopback and
//! LAN addresses and `https://` for public names — and the user can always
//! type the scheme explicitly to override.

use serde::{Deserialize, Serialize};

pub const URL_HOST_MAX: usize = 512;
pub const URL_PATH_MAX: usize = 2048;

/// Default Epic domains that get redirected to the local backend.
pub const DEFAULT_REDIRECT_DOMAINS: &[&str] = &[
    "game-social.epicgames.com",
    "ol.epicgames.com",
    "ol.epicgames.net",
    "on.epicgames.com",
    "ak.epicgames.com",
    "epicgames.dev",
    "www.epicgames.com",
    "account.epicgames.com",
    "download.epicgames.com",
    "download.akamaized.net",
    "fortnite-public-service-prod11.ol.epicgames.net",
    "xmpp-service-prod.ol.epicgames.net",
];

/// A parsed URL split into scheme, host and path+query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub path_and_query: String,
}

impl Url {
    /// Parse a URL. Accepts `scheme://host/path`, `host/path` (scheme-less,
    /// treated as unknown scheme).
    pub fn parse(url: &str) -> Url {
        let mut out = Url {
            scheme: String::new(),
            host: String::new(),
            path_and_query: String::new(),
        };
        let url = url.trim();
        if url.is_empty() {
            return out;
        }
        if let Some(idx) = url.find("://") {
            out.scheme = url[..idx].to_ascii_lowercase();
            let rest = &url[idx + 3..];
            if let Some(slash) = rest.find('/') {
                out.host = rest[..slash].to_string();
                out.path_and_query = rest[slash..].to_string();
            } else {
                out.host = rest.to_string();
            }
        } else if let Some(slash) = url.find('/') {
            out.host = url[..slash].to_string();
            out.path_and_query = url[slash..].to_string();
        } else {
            out.host = url.to_string();
        }
        if out.path_and_query.len() > URL_PATH_MAX {
            out.path_and_query.truncate(URL_PATH_MAX);
        }
        out
    }

    /// Host with any port stripped.
    pub fn host_only(&self) -> &str {
        match self.host.rfind(':') {
            Some(idx) if !self.host[idx + 1..].contains(']') => &self.host[..idx],
            _ => &self.host,
        }
    }

    pub fn is_http(&self) -> bool {
        self.scheme == "http"
    }

    pub fn is_https(&self) -> bool {
        self.scheme == "https"
    }

    /// Rebuild the full URL string.
    pub fn to_string_full(&self) -> String {
        if self.scheme.is_empty() {
            format!("{}{}", self.host, self.path_and_query)
        } else {
            format!("{}://{}{}", self.scheme, self.host, self.path_and_query)
        }
    }
}

/// The redirect rule set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectRules {
    /// Domain suffixes that should be redirected to the backend.
    pub domains: Vec<String>,
    /// The backend base URL, e.g. `http://127.0.0.1:3551`.
    /// Scheme is preserved exactly as configured (http or https).
    pub backend_url: String,
    /// When true, hosts that are already the backend are never rewritten.
    pub skip_backend_host: bool,
}

impl Default for RedirectRules {
    fn default() -> Self {
        Self {
            domains: DEFAULT_REDIRECT_DOMAINS.iter().map(|s| s.to_string()).collect(),
            backend_url: "http://127.0.0.1:3551".to_string(),
            skip_backend_host: true,
        }
    }
}

impl RedirectRules {
    /// Does this host (with optional port) match any redirect domain?
    pub fn should_redirect(&self, host: &str) -> bool {
        let mut stripped = host.trim().to_ascii_lowercase();
        for prefix in ["http://", "https://"] {
            if let Some(rest) = stripped.strip_prefix(prefix) {
                stripped = rest.to_string();
            }
        }
        if let Some(idx) = stripped.find(':') {
            if !stripped[idx + 1..].contains(']') {
                stripped = stripped[..idx].to_string();
            }
        }
        for d in &self.domains {
            if stripped == *d || stripped.ends_with(&format!(".{d}")) {
                return true;
            }
        }
        false
    }

    /// Rewrite a URL to point at the backend, keeping its path and query and
    /// **keeping the backend's scheme** (http or https, as configured).
    pub fn rewrite(&self, url: &str) -> Option<String> {
        let parsed = Url::parse(url);
        if parsed.host.is_empty() {
            return None;
        }
        if !self.should_redirect(&parsed.host) {
            return None;
        }
        if self.skip_backend_host {
            let backend = Url::parse(&self.backend_url);
            if parsed.host_only() == backend.host_only() && parsed.host.contains(':') == backend.host.contains(':')
            {
                return None;
            }
        }
        let backend = Url::parse(&self.backend_url);
        let mut path = parsed.path_and_query.clone();
        if path.is_empty() {
            path = "/".to_string();
        }
        Some(format!(
            "{}://{}{}",
            if backend.scheme.is_empty() { "http" } else { &backend.scheme },
            backend.host,
            path
        ))
    }
}

/// Normalize a user-entered backend URL.
///
/// * Explicit `http://` or `https://` is kept verbatim (the requested
///   feature: local http pages must work).
/// * Bare `host` / `host:port`: loopback, private LAN and IP literals get
///   `http://`; public names get `https://`.
/// * Trailing slash is removed.
pub fn normalize_backend_url(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("empty URL".into());
    }
    let lowered = s.to_ascii_lowercase();
    if lowered.starts_with("http://") || lowered.starts_with("https://") {
        return Ok(s.trim_end_matches('/').to_string());
    }
    let host_part = s.split('/').next().unwrap_or(s);
    let host = host_part.split(':').next().unwrap_or(host_part);
    let scheme = if is_local_host(host) { "http" } else { "https" };
    Ok(format!("{scheme}://{}", s.trim_end_matches('/')))
}

/// Loopback / LAN / link-local detection for smart scheme defaults.
pub fn is_local_host(host: &str) -> bool {
    let h = host.trim().trim_start_matches('[').trim_end_matches(']');
    if h.eq_ignore_ascii_case("localhost") {
        return true;
    }
    for prefix in ["127.", "10.", "192.168.", "169.254."] {
        if h.starts_with(prefix) {
            return true;
        }
    }
    if let Some(rest) = h.strip_prefix("172.") {
        if let Some(second) = rest.split('.').next() {
            if let Ok(n) = second.parse::<u8>() {
                if (16..=31).contains(&n) {
                    return true;
                }
            }
        }
    }
    h.parse::<std::net::Ipv6Addr>()
        .map(|ip| ip.is_loopback() || ip.is_unspecified())
        .unwrap_or(false)
}

/// Percent-encode a string the way query components require.
pub fn url_encode(src: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(src.len());
    for &b in src.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
    out
}

/// Strip the scheme the game's identity layer blindly prepends.
///
/// `OnlineSubsystemMcp.OnlineIdentityMcp` builds the login/portal URL by
/// unconditionally prefixing `https://` to whatever `RedirectUrl` contains,
/// so `RedirectUrl="http://127.0.0.1:8080"` surfaces as
/// `https://http://127.0.0.1:8080/login` — and an `https://` target becomes
/// `https://https://ex.com/...`. The redirecter honors **the protocol that
/// was written from the start**: every auto-prepended outer scheme is peeled
/// off until the first real one is exposed. Single source of truth — used by
/// libowen.so (curl hook + WebView) and the Owen Server selftest.
pub fn normalize_scheme(url: &str) -> &str {
    let mut u = url;
    while let Some(rest) = u.strip_prefix("https://") {
        if rest.starts_with("http://") || rest.starts_with("https://") {
            u = rest;
        } else {
            break;
        }
    }
    u
}

#[cfg(test)]
mod normalize_tests {
    use super::normalize_scheme;

    #[test]
    fn doubled_http_scheme_from_the_identity_layer() {
        // RedirectUrl="http://127.0.0.1:8080" + the game's blind prefix
        assert_eq!(
            normalize_scheme("https://http://127.0.0.1:8080/login"),
            "http://127.0.0.1:8080/login"
        );
    }

    #[test]
    fn doubled_https_scheme_from_the_identity_layer() {
        assert_eq!(
            normalize_scheme("https://https://ex.com/login"),
            "https://ex.com/login"
        );
    }

    #[test]
    fn idempotent_and_peels_repeatedly() {
        assert_eq!(normalize_scheme("https://https://https://ex.com/a"), "https://ex.com/a");
        assert_eq!(normalize_scheme("http://plain.example/x"), "http://plain.example/x");
        assert_eq!(normalize_scheme("https://normal.example/x"), "https://normal.example/x");
        // a path that merely contains "http://" is NOT touched
        assert_eq!(
            normalize_scheme("https://ex.com/http://not.a.prefix/"),
            "https://ex.com/http://not.a.prefix/"
        );
    }
}
