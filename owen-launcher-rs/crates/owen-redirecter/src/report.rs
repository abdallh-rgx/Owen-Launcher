//! Version detection + report to the backend (`POST /` with
//! `GameVersion = "..."`), mirroring the original Owen.c handshake the
//! LawinServer route `route_root_post` expects.

use crate::config::OwenConfig;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Extracted from the mapped library by `detect::detect_version`.
pub fn send_version_report(cfg: &OwenConfig, version: &str) {
    let parsed = crate::rewrite::parse_url(&cfg.server.base_url);
    let host = parsed.host.clone();
    if host.is_empty() {
        return;
    }
    let port = parsed
        .host_with_port
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .unwrap_or(if parsed.scheme == "https" { 443 } else { 80 });

    let body = format!("GameVersion = \"{version}\"");
    let request = format!(
        "POST / HTTP/1.1\r\nHost: {host}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let Ok(mut stream) = TcpStream::connect((host.as_str(), port)) else {
        crate::config::owen_log::warn("owen: version report could not connect");
        return;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    if stream.write_all(request.as_bytes()).is_ok() {
        let mut sink = [0u8; 256];
        let _ = stream.read(&mut sink);
    }
}
