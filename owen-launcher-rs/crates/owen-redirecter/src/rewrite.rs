//! URL rewriting — the heart of the redirecter. Pure safe code, fully
//! unit-testable on the host.
//!
//! Contract (the headline feature): the backend scheme is used **verbatim**.
//! `http://` and `https://` are both first-class; nothing is auto-prepended,
//! downgraded or upgraded.

use crate::config::OwenConfig;

pub const MAX_URL: usize = 8192;

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
pub fn rewrite(url: &str, cfg: &OwenConfig) -> Option<String> {
    let parsed = parse_url(url);
    if parsed.host.is_empty() {
        return None;
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
pub fn webview_rewrite(url: &str, cfg: &OwenConfig) -> Option<String> {
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
    rewrite(url, cfg)
}
