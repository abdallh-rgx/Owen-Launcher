#![forbid(unsafe_code)]
//! 1:1 port of `app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/lawin/LawinConfig.kt`.
//!
//! `config.ini` (INI) editing for the deployed server data — the same
//! section/key update algorithm, the same fallbacks, the same GameServer
//! reader, and the PartyHub writer.

use std::fs;
use std::path::Path;

use crate::core::catalogs::version_catalog::OgVersion;
use crate::core::diag;

const TAG: &str = "LawinConfig";

/// `LawinConfig.localConfigPath(context)`
pub fn local_config_path(data_root: &Path) -> std::path::PathBuf {
    data_root.join("lawin").join("Config").join("config.ini")
}

/// `LawinConfig.readLocal(context)`
pub fn read_local(data_root: &Path) -> Option<String> {
    let file = local_config_path(data_root);
    if file.is_file() {
        return match fs::read_to_string(&file) {
            Ok(s) => Some(s),
            Err(e) => {
                diag::e(TAG, &format!("readLocal failed: {e}"));
                None
            }
        };
    }
    None
}

/// `LawinConfig.writeLocal(context, content)`
pub fn write_local(data_root: &Path, content: &str) -> bool {
    let file = local_config_path(data_root);
    if file.is_dir() {
        let _ = fs::remove_dir_all(&file);
    }
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&file, content).is_ok()
}

/// `LawinConfig.applyDisplayName(context, name)`
pub fn apply_display_name(data_root: &Path, name: &str) -> bool {
    let Some(local) = read_local(data_root) else {
        return false;
    };
    write_local(data_root, &update_ini_value(&local, "Config", "displayName", name))
}

/// `LawinConfig.updateIniValue(content, section, key, newValue)` — 1:1.
///
/// The Kotlin regexes were:
///   section: `^\s*\[([^\]]+)\]\s*$`  (case-insensitive compare)
///   key:     `^\s*([A-Za-z0-9_]+)\s*=.*$`
/// On a hit the line is rewritten as `key=newValue` (canonical key casing).
pub fn update_ini_value(content: &str, section: &str, key: &str, new_value: &str) -> String {
    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
    let mut in_section = false;
    for i in 0..lines.len() {
        let line = &lines[i];
        if let Some(header) = parse_section(line) {
            in_section = header.eq_ignore_ascii_case(section);
        } else if in_section {
            if let Some(lhs) = parse_key_lhs(line) {
                if lhs.eq_ignore_ascii_case(key) {
                    lines[i] = format!("{key}={new_value}");
                    return lines.join("\n");
                }
            }
        }
    }
    let insert_at = find_section_end(&lines, section);
    if insert_at >= 0 {
        lines.insert(insert_at as usize, format!("{key}={new_value}"));
    } else {
        if !lines.is_empty() && !lines.last().map(|s| s.trim().is_empty()).unwrap_or(true) {
            lines.push(String::new());
        }
        lines.push(format!("[{section}]"));
        lines.push(format!("{key}={new_value}"));
    }
    lines.join("\n")
}

/// `^\s*\[([^\]]+)\]\s*$` — returns the section name when the line is a header.
fn parse_section(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
        let inner = &t[1..t.len() - 1];
        if !inner.contains(']') {
            return Some(inner);
        }
    }
    None
}

/// `^\s*([A-Za-z0-9_]+)\s*=.*$` — returns the key when the line is an assignment.
fn parse_key_lhs(line: &str) -> Option<String> {
    let t = line.trim_start();
    let eq = t.find('=')?;
    let lhs = t[..eq].trim_end();
    if !lhs.is_empty()
        && lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Some(lhs.to_string());
    }
    None
}

/// `LawinConfig.findSectionEnd(lines, section)` — index after the last line of
/// `section`, or -1 (Kotlin returns -1 / lines.size).
fn find_section_end(lines: &[String], section: &str) -> isize {
    let mut in_section = false;
    for (i, line) in lines.iter().enumerate() {
        if let Some(header) = parse_section(line) {
            if in_section {
                return i as isize;
            }
            in_section = header.eq_ignore_ascii_case(section);
        }
    }
    if in_section {
        lines.len() as isize
    } else {
        -1
    }
}

/// `LawinConfig.applyForVersion(context, version, gameServerIp, gameServerPort)` — 1:1.
pub fn apply_for_version(
    data_root: &Path,
    version: &OgVersion,
    game_server_ip: &str,
    game_server_port: u16,
) -> String {
    let Some(local) = read_local(data_root) else {
        return "config.ini missing (lawin data not deployed)".to_string();
    };
    let mut updated = update_ini_value(&local, "GameServer", "ip", game_server_ip);
    updated = update_ini_value(&updated, "GameServer", "port", &game_server_port.to_string());
    match &version.manifest_hash {
        Some(manifest_hash) if !manifest_hash.trim().is_empty() => {
            updated = update_ini_value(&updated, "Config", "manifest", manifest_hash);
            diag::i(
                TAG,
                &format!("manifest applied for {}: {}", version.release_key, manifest_hash),
            );
        }
        _ => {
            diag::i(
                TAG,
                &format!(
                    "No manifest hash for {}-CL-{} — keeping the current one.",
                    version.version, version.cl
                ),
            );
        }
    }
    write_local(data_root, &updated);
    diag::i(
        TAG,
        &format!("config.ini applied: GameServer={game_server_ip}:{game_server_port}"),
    );
    format!(
        "config.ini applied: GameServer={game_server_ip}:{game_server_port}, manifest={}",
        version.manifest_hash.clone().unwrap_or_else(|| "kept".to_string())
    )
}

/// `LawinConfig.readGameServer(context)` -> (ip, port); defaults `127.0.0.1:7777`.
pub fn read_game_server(data_root: &Path) -> (String, u16) {
    let Some(local) = read_local(data_root) else {
        return ("127.0.0.1".to_string(), 7777);
    };
    let mut ip = "127.0.0.1".to_string();
    let mut port: u16 = 7777;
    let mut in_game_server = false;
    for raw_line in local.split('\n') {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_game_server = line.eq_ignore_ascii_case("[GameServer]");
        } else if in_game_server {
            if let Some(rest) = line.strip_prefix("ip:") {
                // unreachable in practice; kept for symmetry with startsWith("ip=")
                let _ = rest;
            }
            if line.len() >= 3 && line[..3].eq_ignore_ascii_case("ip=") {
                ip = line[3..].trim().to_string();
            }
            if line.len() >= 5 && line[..5].eq_ignore_ascii_case("port=") {
                port = line[5..].trim().parse().unwrap_or(7777);
            }
        }
    }
    (ip, port)
}

/// `LawinConfig.applyPartyHubToLocal(context, port, header)` — 1:1
/// (the default header value is the on-wire token the shipped PartyHub assets
/// expect; it is protocol data, not branding).
pub fn apply_party_hub_to_local(data_root: &Path, port: u16, header: &str) -> bool {
    let Some(local) = read_local(data_root) else {
        return false;
    };
    let updated = update_ini_value(&local, "PartyHub", "port", &port.to_string());
    let updated = update_ini_value(&updated, "PartyHub", "header", header);
    write_local(data_root, &updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[Config]\nserver=owen\ndisplayName=old\n\n[GameServer]\nip=10.0.0.2\nport=8000\n\n[PartyHub]\nport=3551\n";

    #[test]
    fn update_existing_key_rewrites_line() {
        let out = update_ini_value(SAMPLE, "Config", "displayName", "OwenTester");
        assert!(out.contains("displayName=OwenTester"));
        assert!(!out.contains("displayName=old"));
        // other sections untouched
        assert!(out.contains("ip=10.0.0.2"));
    }

    #[test]
    fn insert_missing_key_inside_section() {
        let out = update_ini_value(SAMPLE, "GameServer", "newKey", "42");
        assert!(out.contains("newKey=42"));
        // inserted before the end of the GameServer section
        let gs = out.find("[GameServer]").unwrap();
        let ph = out.find("[PartyHub]").unwrap();
        let between = &out[gs..ph];
        assert!(between.contains("newKey=42"));
    }

    #[test]
    fn append_new_section_when_missing() {
        let out = update_ini_value(SAMPLE, "BrandNew", "k", "v");
        assert!(out.contains("[BrandNew]"));
        assert!(out.contains("k=v"));
    }

    #[test]
    fn read_game_server_defaults_and_values() {
        let tmp = std::env::temp_dir().join(format!("owen_cfg_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(tmp.join("lawin").join("Config"));
        assert_eq!(read_game_server(&tmp), ("127.0.0.1".to_string(), 7777));
        std::fs::write(tmp.join("lawin/Config/config.ini"), SAMPLE).unwrap();
        assert_eq!(read_game_server(&tmp), ("10.0.0.2".to_string(), 8000));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
