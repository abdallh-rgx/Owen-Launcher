//! URL rewriting — the heart of the redirecter. Pure safe code, fully
//! unit-testable on the host.
//!
//! Contract (the headline feature): the backend scheme is used **verbatim**.
//! `http://` and `https://` are both first-class; nothing is auto-prepended,
//! downgraded or upgraded.

use crate::config::OwenConfig;

pub const MAX_URL: usize = 8192;

/// Strip the scheme the game's identity layer blindly prepends.
///
/// `OnlineSubsystemMcp.OnlineIdentityMcp` builds the login/portal URL by
/// unconditionally prefixing `https://` to whatever `RedirectUrl` contains,
/// so `RedirectUrl="http://127.0.0.1:3551"` surfaces in curl as
/// `https://http://127.0.0.1:3551/login` — and an `https://` target becomes
/// `https://https://ex.com/...`. The redirecter honors **the protocol that
/// was written from the start**: every auto-prepended outer scheme is peeled
/// off until the first real one is exposed.
pub fn normalize_scheme(url: &str) -> &str {
    let mut u = url;
    // The identity layer prepends `https://` to the ORIGINAL value, so a
    // mangled URL is `https://` + (something that itself starts with a
    // scheme). Peel the spurious outer prefix — keep the scheme that was
    // written from the start — but never touch a genuine `https://host` URL.
    while let Some(rest) = u.strip_prefix("https://") {
        if rest.starts_with("http://") || rest.starts_with("https://") {
            u = rest;
        } else {
            break;
        }
    }
    u
}

/// Effective TCP port of a parsed URL (explicit port or the scheme default).
fn effective_port(p: &ParsedUrl) -> Option<u16> {
    if let Some((h, port)) = p.host_with_port.rsplit_once(':') {
        if !h.contains('[') && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return port.parse().ok();
        }
    }
    match p.scheme.as_str() {
        "https" => Some(443),
        "http" => Some(80),
        _ => None,
    }
}

/// The port a URL explicitly names, if any (before scheme defaults apply).
fn explicit_port(p: &ParsedUrl) -> Option<u16> {
    if let Some((h, port)) = p.host_with_port.rsplit_once(':') {
        if !h.contains('[') && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return port.parse().ok();
        }
    }
    None
}

/// True when both sides address the same endpoint: the same explicit port,
/// or both implicit (scheme defaults) — an implicit `http://host` and an
/// implicit `https://host` name the same host, so the backend scheme wins.
fn ports_compatible(a: &ParsedUrl, b: &ParsedUrl) -> bool {
    match (explicit_port(a), explicit_port(b)) {
        (Some(pa), Some(pb)) => pa == pb,
        (None, None) => true,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    /// Host including an explicit port, when present.
    pub host_with_port: String,
    /// Host without the port (the part rules match against).
    pub host: String,
    pub path_and_query: String,
}

pub fn parse_url(url: &str) -> ParsedUrl {
    let mut out = ParsedUrl {
        scheme: String::new(),
        host_with_port: String::new(),
        host: String::new(),
        path_and_query: String::new(),
    };
    let rest = if let Some(idx) = url.find("://") {
        out.scheme = url[..idx].to_string();
        &url[idx + 3..]
    } else {
        url
    };
    let (authority, path) = match rest.find('/') {
        Some(p) => (&rest[..p], &rest[p..]),
        None => (rest, ""),
    };
    out.host_with_port = authority.to_string();
    out.host = match authority.rsplit_once(':') {
        // Do not strip the port when it is part of an IPv6 literal.
        Some((h, p)) if !h.contains('[') && p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
            h.to_string()
        }
        _ => authority.to_string(),
    };
    out.path_and_query = path.to_string();
    out
}

/// Manifest/asset traffic patterns that the `auto` manifest mode always
/// rewrites (even for hosts outside the configured domain list).
const MANIFEST_PATH_PREFIXES: &[&str] = &[
    "/launcher/api/public/assets",
    "/launcher/api/public/distributionpoints",
    "/Builds/Fortnite/Content/CloudDir",
];

const MANIFEST_HOST_SUFFIXES: &[&str] = &[".epicgames.com", ".epicgames.net", "epicgames.dev"];

/// Decide whether `url` should be redirected, and produce the new URL.
pub fn rewrite(raw: &str, cfg: &OwenConfig) -> Option<String> {
    let url = normalize_scheme(raw);
    let parsed = parse_url(url);
    if parsed.host.is_empty() {
        return None;
    }

    // A URL that already points at the backend host must keep the backend's
    // scheme — the identity/WebView layers may upgrade `http` to `https`
    // behind our back (the in-game login portal does exactly that). The
    // protocol the operator configured wins, always.
    let backend = parse_url(&cfg.server.base_url);
    if !backend.host.is_empty()
        && parsed.host == backend.host
        && ports_compatible(&parsed, &backend)
        && parsed.scheme != backend.scheme
        && (parsed.scheme == "http" || parsed.scheme == "https")
        && (backend.scheme == "http" || backend.scheme == "https")
    {
        let mut new_url = String::with_capacity(MAX_URL);
        new_url.push_str(&backend.scheme);
        new_url.push_str("://");
        new_url.push_str(&parsed.host_with_port);
        if parsed.path_and_query.is_empty() {
            new_url.push('/');
        } else {
            new_url.push_str(&parsed.path_and_query);
        }
        return Some(truncate(&new_url));
    }

    let mut matched = cfg
        .redirect
        .domains
        .iter()
        .any(|d| suffix_match(&parsed.host, d));

    // Auto manifest redirect: broaden the match for known CDN/launcher paths.
    if !matched && cfg.manifest.mode == crate::config::ManifestMode::Auto {
        let path_hit = MANIFEST_PATH_PREFIXES
            .iter()
            .any(|p| parsed.path_and_query.starts_with(p));
        let host_hit = parsed.path_and_query.contains(".manifest")
            || parsed.path_and_query.ends_with(".chunk")
            || parsed.path_and_query.ends_with(".ini");
        let host_ok = MANIFEST_HOST_SUFFIXES
            .iter()
            .any(|s| parsed.host.ends_with(s));
        if host_ok && (path_hit || host_hit) {
            matched = true;
        }
    }

    if matched {
        let mut new_url = String::with_capacity(MAX_URL);
        new_url.push_str(&cfg.server.base_url);
        if !parsed.path_and_query.is_empty() {
            new_url.push_str(&parsed.path_and_query);
        } else {
            new_url.push('/');
        }
        return Some(truncate(&new_url));
    }

    // Extra rules: host suffix or path prefix -> path replacement.
    for rule in &cfg.redirect.extra_rules {
        let m = &rule.r#match;
        let hit = if m.starts_with('/') {
            parsed.path_and_query.starts_with(m.as_str())
        } else {
            suffix_match(&parsed.host, m)
        };
        if hit {
            let query = parsed
                .path_and_query
                .split_once('?')
                .map(|(_, q)| q)
                .unwrap_or("");
            let mut new_url = String::with_capacity(MAX_URL);
            new_url.push_str(&cfg.server.base_url);
            new_url.push_str(&rule.replace_path);
            if !query.is_empty() {
                new_url.push('?');
                new_url.push_str(query);
            }
            return Some(truncate(&new_url));
        }
    }

    // No rule fired. If normalization still peeled a spurious `https://`
    // prefix, hand the healed URL back so the hook replaces the mangled
    // original (e.g. `https://http://127.0.0.1:3551/login` from the game's
    // identity layer — already aimed at us, just double-prefixed).
    if url != raw {
        return Some(truncate(url));
    }

    None
}

fn suffix_match(host: &str, domain: &str) -> bool {
    let d = domain.trim_start_matches('.');
    host == d || host.ends_with(&format!(".{d}"))
}

fn truncate(s: &str) -> String {
    if s.len() <= MAX_URL {
        s.to_string()
    } else {
        s[..MAX_URL].to_string()
    }
}

/// Rewrite only the login page URL for the WebView layer.
pub fn webview_rewrite(raw: &str, cfg: &OwenConfig) -> Option<String> {
    let url = normalize_scheme(raw);
    let parsed = parse_url(url);
    let is_login = url.contains("/login")
        || url.contains("/id/login")
        || url.contains("authorize")
        || url.contains("logout");
    let host_ok = parsed.host.ends_with("epicgames.com")
        || parsed.host.ends_with("epicgames.net")
        || parsed.host.ends_with("epicgames.dev");
    if is_login && host_ok {
        return Some(cfg.login_url());
    }
    // PartyHub graphql calls from the WebView.
    if url.contains("/graphql") && host_ok {
        return Some(cfg.graphql_url());
    }
    // Pass the ORIGINAL through: rewrite() re-normalizes and its final
    // peel-fallback returns the healed URL when the game mangled it.
    rewrite(raw, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OwenConfig;

    fn cfg_http() -> OwenConfig {
        let mut c = OwenConfig::default();
        c.server.base_url = "http://127.0.0.1:3551".into();
        c
    }

    fn cfg_https() -> OwenConfig {
        let mut c = OwenConfig::default();
        c.server.base_url = "https://owen.example".into();
        c
    }

    // ---- normalize_scheme: the identity layer's blind https:// prefix -------

    #[test]
    fn normalize_strips_doubled_http() {
        // RedirectUrl="http://127.0.0.1:3551" + the game's prefix
        assert_eq!(
            normalize_scheme("https://http://127.0.0.1:3551/login"),
            "http://127.0.0.1:3551/login"
        );
    }

    #[test]
    fn normalize_strips_doubled_https() {
        // RedirectUrl="https://ex.com" + the game's prefix
        assert_eq!(
            normalize_scheme("https://https://ex.com/login"),
            "https://ex.com/login"
        );
    }

    #[test]
    fn normalize_is_idempotent_and_peels_repeatedly() {
        assert_eq!(normalize_scheme("https://https://https://ex.com/a"), "https://ex.com/a");
        assert_eq!(normalize_scheme("http://plain.example/x"), "http://plain.example/x");
        assert_eq!(normalize_scheme("https://normal.example/x"), "https://normal.example/x");
        assert_eq!(normalize_scheme("https://ex.com/http://not.a.prefix/"), "https://ex.com/http://not.a.prefix/");
    }

    // ---- rewrite: the doubled-scheme login URL lands on our backend -----------

    #[test]
    fn doubled_scheme_login_url_is_honored_verbatim() {
        let cfg = cfg_http();
        // What curl actually sees with RedirectUrl="http://127.0.0.1:3551":
        let out = rewrite("https://http://127.0.0.1:3551/login?client_id=x", &cfg);
        assert_eq!(
            out.as_deref(),
            Some("http://127.0.0.1:3551/login?client_id=x")
        );
    }

    #[test]
    fn scheme_upgrade_to_backend_host_is_reverted() {
        let cfg = cfg_http();
        // Some builds silently upgrade the scheme but keep the host:port.
        let out = rewrite("https://127.0.0.1:3551/fortnite/api/calendar/v1/timeline", &cfg);
        assert_eq!(
            out.as_deref(),
            Some("http://127.0.0.1:3551/fortnite/api/calendar/v1/timeline")
        );
    }

    #[test]
    fn backend_https_scheme_is_never_downgraded() {
        let cfg = cfg_https();
        // An https backend stays https even when the URL arrives as http.
        let out = rewrite("http://owen.example/fortnite/api/calendar/v1/timeline", &cfg);
        assert_eq!(
            out.as_deref(),
            Some("https://owen.example/fortnite/api/calendar/v1/timeline")
        );
    }

    #[test]
    fn same_scheme_backend_url_passes_through_untouched() {
        let cfg = cfg_http();
        assert_eq!(rewrite("http://127.0.0.1:3551/login", &cfg), None);
    }

    #[test]
    fn epic_login_url_still_redirects_to_backend_login() {
        let cfg = cfg_http();
        let out = webview_rewrite("https://www.epicgames.com/id/login?redirectUrl=x", &cfg);
        assert_eq!(out.as_deref(), Some("http://127.0.0.1:3551/login"));
    }

    #[test]
    fn epic_api_url_still_redirects_with_verbatim_scheme() {
        let cfg = cfg_http();
        let out = rewrite(
            "https://ol.epicgames.com/fortnite/api/calendar/v1/timeline",
            &cfg,
        );
        assert_eq!(
            out.as_deref(),
            Some("http://127.0.0.1:3551/fortnite/api/calendar/v1/timeline")
        );
    }

    #[test]
    fn different_port_on_backend_host_is_left_alone() {
        let cfg = cfg_http();
        // 127.0.0.1:9999 is NOT our backend — must not be rewritten.
        assert_eq!(rewrite("https://127.0.0.1:9999/login", &cfg), None);
    }
}
