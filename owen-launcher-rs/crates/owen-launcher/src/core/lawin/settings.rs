#![forbid(unsafe_code)]
//! 1:1 port of `core/lawin/{LawinSettings, LawinGameServer, BackendMode,
//! LawinSettingsStore}.kt`.
//!
//! SharedPreferences("lawin_settings") becomes `lawin_settings.json` under
//! the data root with the same keys (gs_ip, gs_port, http_port, …).
//! The BackendMode descriptions are rebranded to Owen Server per the
//! porting rules (no upstream names in user-visible text).

use std::fs;
use std::path::Path;

/// Port of `BackendMode.kt`. Descriptions now say Owen Server (branding rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Embedded,
    RemoteSelf,
    RemoteOther,
}

impl Default for BackendMode {
    fn default() -> Self {
        BackendMode::RemoteSelf
    }
}

impl BackendMode {
    pub fn label(&self) -> &'static str {
        match self {
            BackendMode::Embedded => "Local Embedded",
            BackendMode::RemoteSelf => "Remote (This Device)",
            BackendMode::RemoteOther => "Remote (Other Device)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BackendMode::Embedded => "Owen Server is injected into the game APK and runs inside the game process",
            BackendMode::RemoteSelf => "The game connects to an Owen Server running on 127.0.0.1",
            BackendMode::RemoteOther => "The game connects to an Owen Server running on another device over the network",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            BackendMode::Embedded => "EMBEDDED",
            BackendMode::RemoteSelf => "REMOTE_SELF",
            BackendMode::RemoteOther => "REMOTE_OTHER",
        }
    }

    fn from_name(name: &str) -> Option<BackendMode> {
        match name.to_ascii_uppercase().as_str() {
            "EMBEDDED" => Some(BackendMode::Embedded),
            "REMOTE_SELF" => Some(BackendMode::RemoteSelf),
            "REMOTE_OTHER" => Some(BackendMode::RemoteOther),
            _ => None,
        }
    }
}

/// Port of `LawinGameServer.kt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawinGameServer {
    pub ip: String,
    pub port: u16,
}

impl Default for LawinGameServer {
    fn default() -> Self {
        LawinGameServer {
            ip: "127.0.0.1".to_string(),
            port: 7777,
        }
    }
}

/// Port of `LawinSettings.kt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawinSettings {
    pub mode: BackendMode,
    pub game_server: LawinGameServer,
    pub http_port: u16,
    pub xmpp_port: u16,
    pub remote_backend_ip: String,
    pub remote_http_port: u16,
    pub remote_xmpp_port: u16,
    pub game_package: String,
    pub game_version_tag: String,
    pub party_hub_port: u16,
    pub party_hub_header: String,
}

impl Default for LawinSettings {
    fn default() -> Self {
        LawinSettings {
            mode: BackendMode::RemoteSelf,
            game_server: LawinGameServer::default(),
            http_port: 3551,
            xmpp_port: 8080,
            remote_backend_ip: String::new(),
            remote_http_port: 3551,
            remote_xmpp_port: 8080,
            game_package: String::new(),
            game_version_tag: String::new(),
            party_hub_port: 0,
            party_hub_header: String::new(),
        }
    }
}

fn settings_path(data_root: &Path) -> std::path::PathBuf {
    data_root.join("lawin_settings.json")
}

/// `LawinSettingsStore.save(context, settings)` — same keys.
pub fn save(data_root: &Path, settings: &LawinSettings) {
    let json = serde_json::json!({
        "mode": settings.mode.name(),
        "gs_ip": settings.game_server.ip,
        "gs_port": settings.game_server.port,
        "http_port": settings.http_port,
        "xmpp_port": settings.xmpp_port,
        "remote_ip": settings.remote_backend_ip,
        "remote_http_port": settings.remote_http_port,
        "remote_xmpp_port": settings.remote_xmpp_port,
        "game_package": settings.game_package,
        "game_version_tag": settings.game_version_tag,
        "ph_port": settings.party_hub_port,
        "ph_header": settings.party_hub_header,
    });
    if let Some(parent) = settings_path(data_root).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        settings_path(data_root),
        serde_json::to_vec_pretty(&json).unwrap_or_default(),
    );
}

/// `LawinSettingsStore.load(context)` — same defaults.
pub fn load(data_root: &Path) -> LawinSettings {
    let Ok(text) = fs::read_to_string(settings_path(data_root)) else {
        return LawinSettings::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return LawinSettings::default();
    };
    let get_str = |k: &str, d: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_else(|| d.to_string());
    let get_port = |k: &str, d: u16| v.get(k).and_then(|x| x.as_u64()).map(|n| n as u16).unwrap_or(d);
    LawinSettings {
        mode: v
            .get("mode")
            .and_then(|x| x.as_str())
            .and_then(BackendMode::from_name)
            .unwrap_or(BackendMode::RemoteSelf),
        game_server: LawinGameServer {
            ip: get_str("gs_ip", "127.0.0.1"),
            port: get_port("gs_port", 7777),
        },
        http_port: get_port("http_port", 3551),
        xmpp_port: get_port("xmpp_port", 8080),
        remote_backend_ip: get_str("remote_ip", ""),
        remote_http_port: get_port("remote_http_port", 3551),
        remote_xmpp_port: get_port("remote_xmpp_port", 8080),
        game_package: get_str("game_package", ""),
        game_version_tag: get_str("game_version_tag", ""),
        party_hub_port: get_port("ph_port", 0),
        party_hub_header: get_str("ph_header", ""),
    }
}

/// `LawinSettingsStore.detectGamePackages(context)` — on Android this lists
/// installed game packages via PackageManager. The pure-Rust port returns an
/// empty list until the Android JNI provider is wired (owen-art); the call
/// shape is kept so the app layer is already 1:1.
pub fn detect_game_packages(_data_root: &Path) -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_everything() {
        let tmp = std::env::temp_dir().join(format!("owen_set_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let mut s = LawinSettings::default();
        s.mode = BackendMode::RemoteOther;
        s.game_server = LawinGameServer { ip: "10.20.30.40".into(), port: 7778 };
        s.http_port = 3552;
        s.xmpp_port = 8081;
        s.remote_backend_ip = "192.168.1.9".into();
        s.remote_http_port = 3600;
        s.remote_xmpp_port = 8090;
        s.game_package = "com.epicgames.fortnite".into();
        s.game_version_tag = "++Fortnite+Release-12.50-CL-999".into();
        s.party_hub_port = 3551;
        s.party_hub_header = "owen".into();
        save(&tmp, &s);
        let back = load(&tmp);
        assert_eq!(back, s);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn defaults_match_kotlin() {
        let tmp = std::env::temp_dir().join(format!("owen_set_def_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let s = load(&tmp);
        assert_eq!(s.mode, BackendMode::RemoteSelf);
        assert_eq!(s.game_server.ip, "127.0.0.1");
        assert_eq!(s.game_server.port, 7777);
        assert_eq!(s.http_port, 3551);
        assert_eq!(s.xmpp_port, 8080);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mode_labels_and_descriptions_branded() {
        for m in [BackendMode::Embedded, BackendMode::RemoteSelf, BackendMode::RemoteOther] {
            assert!(!m.description().contains("Lawin"), "branding rule: {} -> {}", m.label(), m.description());
        }
    }
}
