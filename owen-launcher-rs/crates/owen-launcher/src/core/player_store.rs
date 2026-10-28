#![forbid(unsafe_code)]
//! 1:1 port of `app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/PlayerStore.kt`.
//!
//! SharedPreferences("player_store") becomes a JSON file `player_store.json`
//! under the data root. Setting the name also updates the login web server
//! and `displayName=` in `Config/config.ini`, exactly like the original.

use std::fs;
use std::path::Path;

use crate::core::lawin::{config as lawin_config, login_web_server};

const PREFS: &str = "player_store.json";
const KEY_NAME: &str = "playername";

fn prefs_path(data_root: &Path) -> std::path::PathBuf {
    data_root.join(PREFS)
}

fn read_json(data_root: &Path) -> serde_json::Value {
    fs::read_to_string(prefs_path(data_root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// `PlayerStore.get(context)`
pub fn get(data_root: &Path) -> String {
    read_json(data_root)
        .get(KEY_NAME)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// `PlayerStore.set(context, name)`
pub fn set(data_root: &Path, name: &str) {
    let cleaned = name.trim().to_string();
    let mut prefs = read_json(data_root);
    prefs[KEY_NAME] = serde_json::Value::String(cleaned.clone());
    if let Some(parent) = prefs_path(data_root).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        prefs_path(data_root),
        serde_json::to_vec_pretty(&prefs).unwrap_or_default(),
    );
    login_web_server::set_playername(&cleaned);
    if !cleaned.is_empty() {
        lawin_config::apply_display_name(data_root, &cleaned);
    }
}

/// `PlayerStore.has(context)`
pub fn has(data_root: &Path) -> bool {
    !get(data_root).is_empty()
}

/// `PlayerStore.authorizeUrl(context)`
pub fn authorize_url(data_root: &Path) -> Option<String> {
    let name = get(data_root);
    if name.is_empty() {
        return None;
    }
    Some(format!(
        "com.epicgames.fortnite://authorize/?code={}",
        js_encode(&name)
    ))
}

/// `PlayerStore.jsEncode(value)` — RFC-unreserved percent encoding per byte.
fn js_encode(value: &str) -> String {
    let mut sb = String::new();
    for b in value.as_bytes() {
        let c = *b as char;
        if c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || "-_.!~*'()".contains(c)
        {
            sb.push(c);
        } else {
            sb.push('%');
            sb.push_str(&format!("{:02X}", b));
        }
    }
    sb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_encode_matches_kotlin() {
        // unreserved stay, space and Arabic escape
        assert_eq!(js_encode("abcXYZ019-_.!~*'()"), "abcXYZ019-_.!~*'()");
        assert_eq!(js_encode("player one"), "player%20one");
        assert_eq!(js_encode("عمر"), "%D8%B9%D9%85%D8%B1");
    }

    #[test]
    fn store_roundtrip_and_authorize() {
        let tmp = std::env::temp_dir().join(format!("owen_ps_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        assert!(get(&tmp).is_empty());
        set(&tmp, "  OwenTester ");
        assert_eq!(get(&tmp), "OwenTester");
        assert_eq!(
            authorize_url(&tmp).as_deref(),
            Some("com.epicgames.fortnite://authorize/?code=OwenTester")
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
