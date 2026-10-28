#![forbid(unsafe_code)]
//! 1:1 port of `core/lawin/LawinBackendManager.kt`.
//!
//! BackendState machine + deploy/config/start/probe flow. The Kotlin original
//! started two Android foreground services (the monitor + the JNI server);
//! the Rust port starts the in-process Owen Server through [`crate::backend`]
//! and keeps the same state machine, port-wait and HTTP probe loop.
//! User-visible strings say Owen Server (branding rule).

use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::core::catalogs::version_catalog::OgVersion;
use crate::core::diag;
use crate::core::lawin::{config as lawin_config, deployer, settings as lawin_settings};

const TAG: &str = "LawinBackendManager";

/// Port of the `BackendState` sealed class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendState {
    Idle,
    Preparing(String),
    Running { http_port: u16, xmpp_port: u16 },
    Error(String),
}

static STATE: Mutex<BackendState> = Mutex::new(BackendState::Idle);
static CONFIGURED_FOR: Mutex<Option<String>> = Mutex::new(None);

/// `LawinBackendManager.state`
pub fn state() -> BackendState {
    STATE.lock().unwrap().clone()
}

/// `LawinBackendManager.isRunning()`
pub fn is_running() -> bool {
    matches!(*STATE.lock().unwrap(), BackendState::Running { .. }) && is_port_open_now(3551)
}

/// `LawinBackendManager.ensureRunning(context, version)` — the whole flow.
pub fn ensure_running(
    data_root: &Path,
    bundled: &Path,
    version: Option<&OgVersion>,
) -> BackendState {
    let settings = lawin_settings::load(data_root);
    let version_tag = version
        .map(|v| v.release_key.clone())
        .unwrap_or_else(|| settings.game_version_tag.clone());
    {
        let current = STATE.lock().unwrap().clone();
        let configured = CONFIGURED_FOR.lock().unwrap().clone();
        if let BackendState::Running { http_port, xmpp_port } = &current {
            if *http_port == settings.http_port
                && *xmpp_port == settings.xmpp_port
                && configured.as_deref() == Some(version_tag.as_str())
            {
                diag::i(TAG, &format!("Backend already running on {http_port}/{xmpp_port}"));
                return current;
            }
        }
    }
    let result = ensure_running_inner(data_root, bundled, version, &settings, &version_tag);
    *STATE.lock().unwrap() = result.clone();
    result
}

fn ensure_running_inner(
    data_root: &Path,
    bundled: &Path,
    version: Option<&OgVersion>,
    settings: &lawin_settings::LawinSettings,
    version_tag: &str,
) -> BackendState {
    // 1) data deployed?
    if !deployer::lawin_dir(data_root).join("Config").join("config.ini").is_file() {
        set_state(BackendState::Preparing("Deploying server data…".to_string()));
        diag::i(TAG, "Server data not deployed — deploying assets");
        if !deployer::deploy_local(data_root, bundled, false) {
            diag::e(TAG, "Server data deployment failed");
            return BackendState::Error("Server data deployment failed".to_string());
        }
    }
    // 2) per-version config?
    if let Some(version) = version {
        set_state(BackendState::Preparing("Applying server config…".to_string()));
        let result = lawin_config::apply_for_version(
            data_root,
            version,
            &settings.game_server.ip,
            settings.game_server.port,
        );
        diag::i(TAG, &result);
        *CONFIGURED_FOR.lock().unwrap() = Some(version_tag.to_string());
    }
    // 3) start services (in-process Owen Server; the port of the two
    //    Android foreground services).
    set_state(BackendState::Preparing("Starting Owen Server services…".to_string()));
    if let Err(message) = crate::backend::start_backend(data_root, settings.http_port, settings.xmpp_port) {
        diag::e(TAG, &format!("failed to start: {message}"));
        return BackendState::Error(message);
    }
    diag::i(TAG, "Started Owen Server services");
    // 4) wait for the HTTP port
    if !wait_for_port("127.0.0.1", settings.http_port, 15_000) {
        let msg = format!("Owen Server did not open port {}", settings.http_port);
        diag::e(TAG, &msg);
        return BackendState::Error(msg);
    }
    // 5) probe /login up to 3 times
    let login_url = format!("http://127.0.0.1:{}/login", settings.http_port);
    let mut http_ok = false;
    for attempt in 1..=3 {
        http_ok = http_get_ok(&login_url, 5_000);
        if http_ok {
            break;
        }
        diag::i(TAG, &format!("HTTP probe {attempt}/3 failed for {login_url}"));
    }
    if http_ok {
        diag::i(
            TAG,
            &format!(
                "Owen Server running on ports {}/{}",
                settings.http_port, settings.xmpp_port
            ),
        );
        BackendState::Running {
            http_port: settings.http_port,
            xmpp_port: settings.xmpp_port,
        }
    } else {
        let msg = format!(
            "The Owen Server HTTP endpoint did not respond (port {})",
            settings.http_port
        );
        diag::e(TAG, &msg);
        BackendState::Error(msg)
    }
}

/// `LawinBackendManager.stop(context)`
pub fn stop() {
    match crate::backend::stop_backend() {
        Ok(()) => {
            *STATE.lock().unwrap() = BackendState::Idle;
            *CONFIGURED_FOR.lock().unwrap() = None;
            diag::i(TAG, "Owen Server backend stopped");
        }
        Err(e) => {
            diag::e(TAG, &format!("stop failed: {e}"));
            *STATE.lock().unwrap() = BackendState::Error(e);
        }
    }
}

fn set_state(next: BackendState) {
    *STATE.lock().unwrap() = next;
}

/// `isPortOpenNow`
fn is_port_open_now(port: u16) -> bool {
    port_open("127.0.0.1", port, 500)
}

fn port_open(host: &str, port: u16, timeout_ms: u32) -> bool {
    let Ok(stream) = TcpStream::connect((host, port)) else {
        return false;
    };
    let ok = stream
        .set_read_timeout(Some(Duration::from_millis(timeout_ms as u64)))
        .is_ok()
        && stream
            .set_write_timeout(Some(Duration::from_millis(timeout_ms as u64)))
            .is_ok();
    // `Socket.connect` in the original only proves reachability; the TcpStream
    // connect above is that check.
    ok
}

/// `waitForPort` — poll every 500ms until the deadline.
fn wait_for_port(host: &str, port: u16, timeout_ms: u64) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if port_open(host, port, 500) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    port_open(host, port, 500)
}

/// `httpGetOk` — retry until deadline; 2s connect/read.
#[cfg(not(target_arch = "wasm32"))]
fn http_get_ok(url: &str, timeout_ms: u64) -> bool {
    use std::io::Read;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let probe = (|| -> Option<u16> {
            let parsed = parse_url_host_path(url)?;
            let mut stream = TcpStream::connect((parsed.0.as_str(), parsed.1)).ok()?;
            stream
                .set_read_timeout(Some(Duration::from_millis(2_000)))
                .ok()?;
            stream
                .set_write_timeout(Some(Duration::from_millis(2_000)))
                .ok()?;
            use std::io::Write;
            let req = format!(
                "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
                parsed.2, parsed.0
            );
            stream.write_all(req.as_bytes()).ok()?;
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).ok()?;
            let head = String::from_utf8_lossy(&buf);
            head.split_whitespace().nth(1)?.parse().ok()
        })();
        if let Some(code) = probe {
            if (200..300).contains(&code) {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(target_arch = "wasm32")]
fn http_get_ok(_url: &str, _timeout_ms: u64) -> bool {
    false
}

/// Minimal `http://host:port/path` splitter for the probe.
fn parse_url_host_path(url: &str) -> Option<(String, u16, String)> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(80)),
        None => (authority.to_string(), 80),
    };
    Some((host, port, path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_host_path_cases() {
        let (h, p, path) = parse_url_host_path("http://127.0.0.1:3551/login").unwrap();
        assert_eq!((h.as_str(), p, path.as_str()), ("127.0.0.1", 3551, "/login"));
        let (h, p, path) = parse_url_host_path("http://example.com/a/b?x=1").unwrap();
        assert_eq!((h.as_str(), p, path.as_str()), ("example.com", 80, "/a/b?x=1"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn http_get_ok_against_local_login_page() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 256];
                let _ = s.read(&mut buf);
                let body = "<html>ok</html>";
                let resp = format!("HTTP/1.0 200 OK\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(http_get_ok(&format!("http://127.0.0.1:{port}/login"), 4_000));
        assert!(!http_get_ok("http://127.0.0.1:1/nope", 700));
    }
}
