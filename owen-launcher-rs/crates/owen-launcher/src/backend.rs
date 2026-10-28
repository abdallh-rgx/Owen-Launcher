#![forbid(unsafe_code)]
//! Owen Server control — the app layer that talks to / hosts the backend.
//!
//! * Native (Android/desktop): the Owen Server runs **in-process** (axum) on
//!   `127.0.0.1:<http_port>` — the port of the two original foreground
//!   services. Start/stop is synchronous through [`start_backend`] /
//!   [`stop_backend`].
//! * wasm (the preview build): the app cannot spawn processes; it drives the
//!   **hosted** Owen Server through the gateway (`?XTransformPort=3010` —
//!   the supervisor answers `/api/launcher/hosting/*` and proxies everything
//!   else to the real server). That is the on-demand hosting for the preview.
//!
//! All helper fetches marshal results back onto the egui loop via
//! `ctx.request_repaint()` (the port of the Kotlin coroutines + StateFlow).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::http::{fetch_async, HttpResponse};
use serde_json::Value;

/// Gateway port used by the wasm preview (matches mini-services/owen-server).
pub const GATEWAY_PORT: u16 = 3010;
/// The port the in-process/native server binds (the original HTTP port).
pub const NATIVE_HTTP_PORT: u16 = 3551;

static NATIVE_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn is_wasm() -> bool {
    cfg!(target_arch = "wasm32")
}

/// URL for an Owen Server API path on the current platform.
/// Native: `http://127.0.0.1:3551{path}`. wasm: same-origin relative with
/// the gateway port (`{path}[&]XTransformPort=3010`).
pub fn api_url(path: &str) -> String {
    if is_wasm() {
        if path.contains('?') {
            format!("{path}&XTransformPort={GATEWAY_PORT}")
        } else {
            format!("{path}?XTransferPort={GATEWAY_PORT}")
                .replace("XTransferPort", "XTransformPort")
        }
    } else {
        format!("http://127.0.0.1:{NATIVE_HTTP_PORT}{path}")
    }
}

/* ------------------------------ fetch helpers ------------------------------ */

/// GET JSON; the callback runs on a worker (native) or the JS loop (wasm) and
/// always triggers a repaint.
pub fn get_json(
    ctx: &egui::Context,
    path: &str,
    on_done: impl FnOnce(Result<Value, String>) + Send + 'static,
) {
    let url = api_url(path);
    fetch_async(
        url,
        Vec::new(),
        move |result: Result<HttpResponse, String>| {
            let parsed = result
                .map_err(|e| e)
                .and_then(|resp| {
                    if resp.status == 503 {
                        Err("__stopped__".to_string())
                    } else if !(200..300).contains(&resp.status) {
                        Err(format!("HTTP {}", resp.status))
                    } else {
                        serde_json::from_slice(&resp.body)
                            .map_err(|e| format!("bad JSON: {e}"))
                    }
                });
            ctx.request_repaint();
            on_done(parsed);
        },
    );
}

/// POST JSON with optional bearer token.
pub fn post_json(
    ctx: &egui::Context,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
    on_done: impl FnOnce(Result<Value, String>) + Send + 'static,
) {
    let url = api_url(path);
    let payload = body
        .map(|v| v.to_string().into_bytes())
        .unwrap_or_default();
    let mut headers: Vec<(String, String)> = Vec::new();
    headers.push(("Content-Type".into(), "application/json".into()));
    if let Some(token) = bearer {
        headers.push(("Authorization".into(), format!("Bearer {token}")));
    }
    fetch_async(
        url,
        headers,
        move |result: Result<HttpResponse, String>| {
            let parsed = result.map_err(|e| e).and_then(|resp| {
                if resp.status == 503 {
                    Err("__stopped__".to_string())
                } else if !(200..300).contains(&resp.status) {
                    // try to surface the server's errorCode/message
                    let detail = serde_json::from_slice::<Value>(&resp.body).ok().and_then(|v| {
                        let code = v.get("errorCode").and_then(|x| x.as_str()).unwrap_or("");
                        let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("");
                        if code.is_empty() && msg.is_empty() {
                            None
                        } else {
                            Some(if msg.is_empty() { code.to_string() } else { msg.to_string() })
                        }
                    });
                    Err(detail.unwrap_or_else(|| format!("HTTP {}", resp.status)))
                } else {
                    serde_json::from_slice(&resp.body).map_err(|e| format!("bad JSON: {e}"))
                }
            });
            ctx.request_repaint();
            on_done(parsed);
        },
    );
}

/* ------------------------------ hosting panel ------------------------------ */

/// Snapshot of the hosting panel (filled by the status poll).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostingStatus {
    pub running: bool,
    pub detail: String,
}

/// Poll the hosting state. Native reports the in-process server directly;
/// wasm asks the supervisor (`/api/launcher/hosting/status`).
pub fn hosting_status(
    ctx: &egui::Context,
    on_done: impl FnOnce(Result<HostingStatus, String>) + Send + 'static,
) {
    if !is_wasm() {
        let running = NATIVE_RUNNING.load(Ordering::SeqCst);
        ctx.request_repaint();
        on_done(Ok(HostingStatus {
            running,
            detail: if running {
                format!("in-process · 127.0.0.1:{NATIVE_HTTP_PORT}")
            } else {
                "stopped".into()
            },
        }));
        return;
    }
    get_json(
        ctx,
        "/api/launcher/hosting/status",
        move |result: Result<Value, String>| {
            let out = match result {
                Ok(v) => Ok(HostingStatus {
                    running: v.get("running").and_then(|x| x.as_bool()).unwrap_or(false),
                    detail: v
                        .get("detail")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                }),
                Err("__stopped__") => Ok(HostingStatus {
                    running: false,
                    detail: "stopped".into(),
                }),
                Err(e) => Err(e),
            };
            on_done(out);
        },
    );
}

/// Start hosting (on demand). Native: in-process spawn. wasm: supervisor.
pub fn hosting_start(
    ctx: &egui::Context,
    data_root: &Path,
    on_done: impl FnOnce(Result<HostingStatus, String>) + Send + 'static,
) {
    if !is_wasm() {
        let result = start_backend(data_root, NATIVE_HTTP_PORT, 8080)
            .map(|_| HostingStatus {
                running: true,
                detail: format!("in-process · 127.0.0.1:{NATIVE_HTTP_PORT}"),
            })
            .map_err(|e| e);
        ctx.request_repaint();
        on_done(result);
        return;
    }
    post_json(
        ctx,
        "/api/launcher/hosting/start",
        None,
        None,
        move |result: Result<Value, String>| {
            let out = result.map(|v| HostingStatus {
                running: v.get("running").and_then(|x| x.as_bool()).unwrap_or(true),
                detail: "gateway".into(),
            });
            on_done(out);
        },
    );
}

/// Stop hosting.
pub fn hosting_stop(
    ctx: &egui::Context,
    _data_root: &Path,
    on_done: impl FnOnce(Result<HostingStatus, String>) + Send + 'static,
) {
    if !is_wasm() {
        let result = stop_backend().map(|_| HostingStatus {
            running: false,
            detail: "stopped".into(),
        });
        ctx.request_repaint();
        on_done(result);
        return;
    }
    post_json(
        ctx,
        "/api/launcher/hosting/stop",
        None,
        None,
        move |result: Result<Value, String>| {
            let out = result.map(|_| HostingStatus {
                running: false,
                detail: "stopped".into(),
            });
            on_done(out);
        },
    );
}

/* ------------------------------ redirecter menu ---------------------------- */

/// GET `/api/launcher/manifests/redirect` — mode/target/versions.
pub fn redirecter_get(
    ctx: &egui::Context,
    on_done: impl FnOnce(Result<(String, Option<String>, Vec<String>), String>) + Send + 'static,
) {
    get_json(ctx, "/api/launcher/manifests/redirect", move |result| {
        let out = result.map(|v| {
            let mode = v
                .get("mode")
                .and_then(|x| x.as_str())
                .unwrap_or("auto")
                .to_string();
            let target = v
                .get("target")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            let versions = v
                .get("versions")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            (mode, target, versions)
        });
        on_done(out);
    });
}

/// PUT the redirecter mode (`auto` | `fixed:<version>` | `off`).
pub fn redirecter_set(
    ctx: &egui::Context,
    mode: &str,
    target: Option<&str>,
    on_done: impl FnOnce(Result<(), String>) + Send + 'static,
) {
    let mut body = serde_json::json!({ "mode": mode });
    if let Some(t) = target {
        body["target"] = Value::String(t.to_string());
    }
    post_json(
        ctx,
        "/api/launcher/manifests/redirect",
        Some(body),
        None,
        move |result| on_done(result.map(|_| ())),
    );
}

/// Run the selftest (`/api/launcher/redirecter/selftest`).
pub fn selftest(
    ctx: &egui::Context,
    on_done: impl FnOnce(Result<Vec<(String, bool, String)>, String>) + Send + 'static,
) {
    get_json(ctx, "/api/launcher/redirecter/selftest", move |result| {
        let out = result.map(|v| {
            v.get("checks")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|c| {
                            let id = c.get("id").and_then(|x| x.as_str())?.to_string();
                            let ok = c.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
                            let detail = c
                                .get("detail")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string();
                            Some((id, ok, detail))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
        on_done(out);
    });
}

/* --------------------------------- accounts -------------------------------- */

/// A stored app session (`owen_account.json` under the data root).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountSession {
    pub access_token: String,
    pub display_name: String,
    pub email: String,
}

pub fn account_path(data_root: &Path) -> PathBuf {
    data_root.join("owen_account.json")
}

pub fn load_account(data_root: &Path) -> Option<AccountSession> {
    let text = std::fs::read_to_string(account_path(data_root)).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    Some(AccountSession {
        access_token: v.get("access_token").and_then(|x| x.as_str())?.to_string(),
        display_name: v.get("displayName").and_then(|x| x.as_str())?.to_string(),
        email: v.get("email").and_then(|x| x.as_str())?.to_string(),
    })
}

pub fn save_account(data_root: &Path, session: &AccountSession) {
    let v = serde_json::json!({
        "access_token": session.access_token,
        "displayName": session.display_name,
        "email": session.email,
    });
    if let Some(parent) = account_path(data_root).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(account_path(data_root), v.to_string());
}

pub fn clear_account(data_root: &Path) {
    let _ = std::fs::remove_file(account_path(data_root));
}

/// POST /api/launcher/register — returns the session (token) like the server.
pub fn register(
    ctx: &egui::Context,
    email: &str,
    username: &str,
    display_name: &str,
    password: &str,
    on_done: impl FnOnce(Result<AccountSession, String>) + Send + 'static,
) {
    let body = serde_json::json!({
        "email": email,
        "username": username,
        "displayName": display_name,
        "password": password,
    });
    post_json(
        ctx,
        "/api/launcher/register",
        Some(body),
        None,
        move |result| {
            let out = result.and_then(|v| {
                let token = v
                    .get("access_token")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| "no token in response".to_string())?
                    .to_string();
                Ok(AccountSession {
                    access_token: token,
                    display_name: v
                        .get("displayName")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    email: v.get("email").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                })
            });
            on_done(out);
        },
    );
}

/// Login through the OAuth token endpoint (grant_type=password), then confirm
/// with /api/launcher/me.
pub fn login(
    ctx: &egui::Context,
    email: &str,
    password: &str,
    on_done: impl FnOnce(Result<AccountSession, String>) + Send + 'static,
) {
    let form = format!(
        "grant_type=password&username={}&password={}",
        urlencode(email),
        urlencode(password)
    );
    let url = api_url("/account/api/oauth/token");
    let headers = vec![
        ("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()),
    ];
    fetch_async(
        url,
        headers,
        move |result: Result<HttpResponse, String>| {
            let parsed = result.map_err(|e| e).and_then(|resp| {
                if !(200..300).contains(&resp.status) {
                    let detail = serde_json::from_slice::<Value>(&resp.body).ok().and_then(|v| {
                        v.get("message")
                            .or_else(|| v.get("errorCode"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string())
                    });
                    Err(detail.unwrap_or_else(|| format!("HTTP {}", resp.status)))
                } else {
                    serde_json::from_slice(&resp.body).map_err(|e| format!("bad JSON: {e}"))
                }
            });
            let token = parsed
                .as_ref()
                .ok()
                .and_then(|v| v.get("access_token").and_then(|x| x.as_str()))
                .map(|s| s.to_string());
            let Some(token) = token else {
                ctx.request_repaint();
                on_done(Err(parsed.err().unwrap_or_else(|| "no token".into())));
                return;
            };
            // confirm + enrich through /me
            let ctx2 = ctx.clone();
            get_json(
                &ctx2,
                "/api/launcher/me",
                move |me: Result<Value, String>| {
                    let out = me.map(|v| AccountSession {
                        access_token: token,
                        display_name: v
                            .get("displayName")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        email: v.get("email").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    });
                    ctx2.request_repaint();
                    on_done(out);
                },
            );
            // the outer closure already scheduled a repaint via get_json
        },
    );
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || "-_.!~*'()".contains(c) {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/* ------------------------------ native in-process --------------------------- */

#[cfg(not(target_arch = "wasm32"))]
mod native_server {
    use super::*;
    use std::sync::OnceLock;

    struct Running {
        shutdown: tokio::sync::oneshot::Sender<()>,
    }

    static HANDLE: Mutex<Option<Running>> = Mutex::new(None);
    static BUNDLED_HINT: OnceLock<PathBuf> = OnceLock::new();

    pub fn set_bundled_hint(p: PathBuf) {
        let _ = BUNDLED_HINT.set(p);
    }

    fn bundled_hint() -> PathBuf {
        BUNDLED_HINT
            .get()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("data"))
    }

    /// Ensure the server data tree exists under `data_root/owen` (copy from
    /// the bundled `owen` tree once — the deployer's 1:1 roots stay untouched).
    fn ensure_server_data(data_root: &Path) -> PathBuf {
        let target = data_root.join("owen");
        if !target.join("manifests.json").is_file() {
            let bundled = bundled_hint().join("owen");
            if bundled.is_dir() {
                copy_tree(&bundled, &target);
            }
        }
        target
    }

    fn copy_tree(src: &Path, dst: &Path) {
        if src.is_dir() {
            let _ = std::fs::create_dir_all(dst);
            if let Ok(entries) = std::fs::read_dir(src) {
                for e in entries.flatten() {
                    copy_tree(&e.path(), &dst.join(e.file_name()));
                }
            }
        } else if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = std::fs::copy(src, dst);
        }
    }

    pub fn start(data_root: &Path, http_port: u16, xmpp_port: u16) -> Result<(), String> {
        if NATIVE_RUNNING.load(Ordering::SeqCst) {
            return Ok(());
        }
        if crate::core::lawin::backend_manager::is_running() {
            return Ok(());
        }
        let server_data = ensure_server_data(data_root);
        let data_root = data_root.to_path_buf();
        let http_port2 = http_port;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        std::thread::Builder::new()
            .name("owen-server".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        crate::core::diag::e("backend", &format!("tokio runtime: {e}"));
                        return;
                    }
                };
                rt.block_on(async move {
                    let config = owen_redirecter::config::OwenConfig::load();
                    let state = std::sync::Arc::new(owen_server::state::AppState::load(
                        server_data.clone(),
                        config,
                    ));
                    let app = owen_server::build_router(state.clone());
                    crate::core::diag::i(
                        "backend",
                        &format!("Owen Server listening on 127.0.0.1:{http_port2} (xmpp {xmpp_port})"),
                    );
                    let _xmpp = crate::core::diag::i(
                        "backend",
                        &format!("xmpp listener: {}", start_xmpp(xmpp_port)),
                    );
                    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", http_port2)).await {
                        Ok(l) => l,
                        Err(e) => {
                            crate::core::diag::e("backend", &format!("bind {http_port2}: {e}"));
                            NATIVE_RUNNING.store(false, Ordering::SeqCst);
                            return;
                        }
                    };
                    NATIVE_RUNNING.store(true, Ordering::SeqCst);
                    let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
                        let _ = rx.await;
                    });
                    if let Err(e) = serve.await {
                        crate::core::diag::e("backend", &format!("server stopped: {e}"));
                    }
                    NATIVE_RUNNING.store(false, Ordering::SeqCst);
                    let _ = data_root;
                });
            })
            .map_err(|e| e.to_string())?;
        *HANDLE.lock().unwrap() = Some(Running { shutdown: tx });
        // wait briefly for the port so callers see a consistent state
        for _ in 0..40 {
            if NATIVE_RUNNING.load(Ordering::SeqCst) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        Err("Owen Server did not come up within 10s".to_string())
    }

    /// XMPP side listener (std TcpListener threads); returns a description.
    fn start_xmpp(port: u16) -> String {
        match owen_server::xmpp::spawn_listener(port) {
            Ok(()) => format!("spawned on {port}"),
            Err(e) => format!("disabled ({e})"),
        }
    }

    pub fn stop() -> Result<(), String> {
        let taken = HANDLE.lock().unwrap().take();
        NATIVE_RUNNING.store(false, Ordering::SeqCst);
        if let Some(r) = taken {
            let _ = r.shutdown.send(());
        }
        owen_server::xmpp::stop_listener();
        crate::core::diag::i("backend", "Owen Server stopped");
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_backend(data_root: &Path, http_port: u16, xmpp_port: u16) -> Result<(), String> {
    native_server::start(data_root, http_port, xmpp_port)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stop_backend() -> Result<(), String> {
    native_server::stop()
}

#[cfg(target_arch = "wasm32")]
pub fn start_backend(_data_root: &Path, _http_port: u16, _xmpp_port: u16) -> Result<(), String> {
    Err("This build drives the hosted Owen Server through the gateway".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn stop_backend() -> Result<(), String> {
    Err("This build drives the hosted Owen Server through the gateway".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_shapes() {
        let native = api_url("/api/launcher/me");
        assert!(native.starts_with("http://127.0.0.1:3551/api/launcher/me"));
        let native_q = api_url("/x?y=1");
        assert!(native_q.starts_with("http://127.0.0.1:3551/x?y=1"));
    }

    #[test]
    fn urlencode_query() {
        assert_eq!(urlencode("a b+c"), "a%20b%2Bc");
        assert_eq!(urlencode("o'malley@x.io"), "o%27malley%40x.io");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn account_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("owen_acc_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        assert!(load_account(&tmp).is_none());
        save_account(
            &tmp,
            &AccountSession {
                access_token: "tok".into(),
                display_name: "OwenTester".into(),
                email: "a@b.c".into(),
            },
        );
        assert_eq!(
            load_account(&tmp).map(|a| a.display_name),
            Some("OwenTester".to_string())
        );
        clear_account(&tmp);
        assert!(load_account(&tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
