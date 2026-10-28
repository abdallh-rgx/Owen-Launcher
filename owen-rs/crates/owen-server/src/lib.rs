//! Owen backend server — a safe-Rust, LawinServer v2-compatible HTTP
//! backend with Reload-Backend feature parity where it matters (Battle
//! Pass, MCP profiles, item shop, friends) plus the Owen-specific
//! endpoints: `/login`, `/partyhub/graphql`, the auto manifest redirecter
//! and the admin API that powers the web control panel.

pub mod login_page;
pub mod routes;
pub mod state;
pub mod xmpp;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// Server options (CLI flags or defaults).
#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub port: u16,
    pub data_dir: PathBuf,
    pub bind: String,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: 3551,
            data_dir: PathBuf::from("owen-data"),
            bind: "0.0.0.0".into(),
        }
    }
}

pub fn run_server_from_args(args: &[String]) {
    let mut opts = ServerOptions::default();
    let mut backend_url: Option<String> = None;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--port" | "-p" => {
                if let Some(p) = iter.next() {
                    opts.port = p.parse().unwrap_or(opts.port);
                }
            }
            "--bind" | "-b" => {
                if let Some(b) = iter.next() {
                    opts.bind = b.clone();
                }
            }
            "--data-dir" | "-d" => {
                if let Some(d) = iter.next() {
                    opts.data_dir = PathBuf::from(d);
                }
            }
            "--backend-url" => {
                if let Some(u) = iter.next() {
                    backend_url = Some(u.clone());
                }
            }
            _ => {}
        }
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(async move {
        let mut config = owen_core::config::OwenConfig::default();
        config.http_port = opts.port;
        config.login_port = opts.port;
        if let Some(u) = backend_url {
            let _ = config.set_backend_url(&u);
        } else {
            let _ = config.set_backend_url(&format!("http://127.0.0.1:{}", opts.port));
        }
        if let Err(e) = run_server(opts, config).await {
            eprintln!("server error: {e}");
        }
    });
}

/// Build the router (used by the CLI and by the launcher).
pub fn build_router(state: Arc<state::AppState>) -> axum::Router {
    routes::root_router(state)
}

/// Run the HTTP server (and the minimal XMPP listener when enabled).
pub async fn run_server(
    opts: ServerOptions,
    config: owen_core::config::OwenConfig,
) -> Result<(), String> {
    let state = Arc::new(state::AppState::load(opts.data_dir.clone(), config));

    let app = build_router(state.clone());

    let addr: SocketAddr = format!("{}:{}", opts.bind, state.config.read().unwrap().http_port)
        .parse()
        .map_err(|e| format!("bad bind address: {e}"))?;
    println!("owen-server listening on http://{addr}");
    println!("data dir: {}", opts.data_dir.display());

    // Minimal XMPP service (friends/party presence) on its own port.
    let xmpp_port = state.config.read().unwrap().xmpp_port;
    if xmpp_port > 0 {
        let xmpp_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = xmpp::run_xmpp(xmpp_state, xmpp_port).await {
                eprintln!("xmpp: {e}");
            }
        });
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}
