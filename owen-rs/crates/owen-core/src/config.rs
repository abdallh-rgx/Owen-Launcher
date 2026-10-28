//! Shared configuration model for the Owen stack.

use crate::urlrule::{normalize_backend_url, RedirectRules};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwenConfig {
    /// Backend base URL. Both http and https are accepted verbatim.
    pub backend_url: String,
    /// HTTP port the local backend listens on.
    pub http_port: u16,
    /// Optional TLS port (0 = disabled).
    pub tls_port: u16,
    /// Login-page port used by the WebView redirect target.
    pub login_port: u16,
    /// XMPP port (0 = disabled).
    pub xmpp_port: u16,
    /// Redirect rules (domains, scheme handling).
    pub redirect: RedirectRules,
    /// PartyHub graphql redirection enabled (Kairos era).
    pub partyhub_redirect: bool,
    /// Redirect WebView login pages via ART hooks (no smali).
    pub webview_redirect: bool,
    /// Serve auto-generated manifests to the game.
    pub manifest_redirect: bool,
    /// Default display name for the local player.
    pub display_name: String,
}

impl Default for OwenConfig {
    fn default() -> Self {
        Self {
            backend_url: "http://127.0.0.1:3551".into(),
            http_port: 3551,
            tls_port: 0,
            login_port: 3551,
            xmpp_port: 8080,
            redirect: RedirectRules::default(),
            partyhub_redirect: true,
            webview_redirect: true,
            manifest_redirect: true,
            display_name: "OwenPlayer".into(),
        }
    }
}

impl OwenConfig {
    /// Normalize a user-supplied backend URL (accepts http://, https:// and
    /// bare hosts) and stores it.
    pub fn set_backend_url(&mut self, input: &str) -> Result<(), String> {
        let normalized = normalize_backend_url(input)?;
        self.backend_url = normalized.clone();
        self.redirect.backend_url = normalized;
        // Keep the login port aligned with the backend when it carries one.
        if let Some(port) = port_of(&self.backend_url) {
            self.login_port = port;
        }
        Ok(())
    }

    pub fn login_url(&self) -> String {
        format!("{}/login", self.backend_url.trim_end_matches('/'))
    }

    pub fn partyhub_graphql_url(&self) -> String {
        format!("{}/partyhub/graphql", self.backend_url.trim_end_matches('/'))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| e.to_string())
    }
}

fn port_of(url: &str) -> Option<u16> {
    let rest = url.split("://").nth(1)?;
    let host = rest.split('/').next()?;
    let port = host.rsplit(':').next()?;
    if port.chars().all(|c| c.is_ascii_digit()) && port.len() <= 5 && host.contains(':') {
        port.parse().ok()
    } else {
        None
    }
}
