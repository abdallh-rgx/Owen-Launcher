//! owen-lawin — the LawinServer v2-compatible backend, ported from the C
//! server embedded in the original Owen-Launcher (`app/src/main/cpp`), now
//! in 100% safe Rust with zero dependencies.
//!
//! This is what finally makes the *device-side* server practical: the
//! original project shelled out to a C server because Node.js cannot run on
//! Android (V8 JIT SIGILL), and Rust needs no JIT at all — `libowen.so`
//! hosts this server in-process when `server.embedded = true`, and the
//! launcher binary can run it standalone.
//!
//! Feature coverage (LawinServer v2 / Reload-Backend compatible):
//! * HTTP/1.1 on the configured port (default 3551) with keep-alive.
//! * Wildcard router with the same patterns the game expects.
//! * Deployment directory serving: profiles, responses, CloudStorage,
//!   CloudDir manifests and the item shop straight from the deployed files
//!   (the "94 files" layout of the original launcher).
//! * OAuth token exchange, lightswitch, waitingroom, keychain, storefront,
//!   content pages, discovery, timeline, friends, party, matchmaking,
//!   socialban, feedback, datarouter and the `POST /` version handshake.
//! * MCP: QueryProfile / ClientQuestLogin / SetMCPProfile and the common
//!   cosmetic mutations, operating on the deployed `profiles/*.json`.
//! * XMPP accept loop (port 5222) with the minimal stream negotiation the
//!   game needs for presence and chat.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod mcp;
pub mod routes;
pub mod xmpp;

/// Runtime configuration of the embedded server.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Deployed data directory (`assets/lawin` layout).
    pub data_dir: PathBuf,
    pub http_port: u16,
    pub xmpp_port: u16,
    /// Optional override for the display name written into profiles.
    pub display_name: String,
}

impl Default for ServerOptions {
    fn default() -> Self {
        ServerOptions {
            data_dir: PathBuf::from("lawin"),
            http_port: 3551,
            xmpp_port: 5222,
            display_name: "LawinServer".into(),
        }
    }
}

/// A parsed HTTP request.
#[derive(Debug, Default)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(|s| s.as_str())
    }
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }
    pub fn user_agent(&self) -> &str {
        self.header("user-agent").unwrap_or("")
    }
    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// An HTTP response under construction.
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub reason: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Response {
        let reason = match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            _ => "OK",
        };
        Response {
            status,
            reason,
            headers: vec![
                ("Server".into(), "LawinServer/Owen-RS".into()),
                ("Access-Control-Allow-Origin".into(), "*".into()),
            ],
            body: Vec::new(),
        }
    }

    pub fn json(status: u16, body: impl Into<String>) -> Response {
        let mut r = Response::new(status);
        r.headers.push(("Content-Type".into(), "application/json".into()));
        r.body = body.into().into_bytes();
        r
    }

    pub fn text(status: u16, body: impl Into<String>) -> Response {
        let mut r = Response::new(status);
        r.headers.push(("Content-Type".into(), "text/plain; charset=utf-8".into()));
        r.body = body.into().into_bytes();
        r
    }

    pub fn bytes(content_type: &str, body: Vec<u8>) -> Response {
        let mut r = Response::new(200);
        r.headers.push(("Content-Type".into(), content_type.into()));
        r.body = body;
        r
    }

    pub fn not_found() -> Response {
        Response::json(404, "{\"errorCode\":\"errors.com.epicgames.common.not_found\",\"message\":\"not found\"}")
    }
}

/// Shared server context handed to every route.
pub struct Context {
    pub opts: ServerOptions,
    /// Cached JSON payloads keyed by their deployment-relative path.
    pub cache: std::sync::Mutex<HashMap<String, Arc<String>>>,
}

impl Context {
    pub fn new(opts: ServerOptions) -> Context {
        Context {
            opts,
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Read a deployed text asset (cached).
    pub fn asset(&self, rel: &str) -> Option<Arc<String>> {
        if let Some(hit) = self.cache.lock().ok().and_then(|c| c.get(rel).cloned()) {
            return Some(hit);
        }
        let path = self.opts.data_dir.join(rel);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return None;
        };
        let arc = Arc::new(text);
        if let Ok(mut c) = self.cache.lock() {
            c.insert(rel.to_string(), arc.clone());
        }
        Some(arc)
    }

    pub fn asset_bytes(&self, rel: &str) -> Option<Vec<u8>> {
        std::fs::read(self.opts.data_dir.join(rel)).ok()
    }
}

/// Run the HTTP + XMPP servers. Blocks the calling thread.
pub fn run(opts: ServerOptions) -> std::io::Result<()> {
    let ctx = Arc::new(Context::new(opts.clone()));

    // XMPP on its own thread.
    let xmpp_ctx = ctx.clone();
    let xmpp_port = opts.xmpp_port;
    std::thread::Builder::new()
        .name("owen-xmpp".into())
        .spawn(move || {
            if let Ok(listener) = TcpListener::bind(("0.0.0.0", xmpp_port)) {
                for stream in listener.incoming() {
                    if let Ok(s) = stream {
                        let ctx = xmpp_ctx.clone();
                        std::thread::spawn(move || xmpp::session(s, ctx));
                    }
                }
            }
        })
        .ok();

    // HTTP with a small accept pool.
    let listener = TcpListener::bind(("0.0.0.0", ctx.opts.http_port))?;
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = handle_connection(stream, &ctx);
        });
    }
    Ok(())
}

fn handle_connection(stream: TcpStream, ctx: &Context) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30))).ok();
    stream.set_nodelay(true).ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    loop {
        let Some(req) = read_request(&mut reader)? else {
            return Ok(());
        };
        let keep_alive = req
            .header("connection")
            .map(|c| !c.eq_ignore_ascii_case("close"))
            .unwrap_or(true);
        let head_only = req.method == "HEAD";

        let mut res = routes::dispatch(ctx, &req);
        if head_only {
            res.body.clear();
        }
        write_response(&mut writer, &res, keep_alive)?;
        if !keep_alive {
            return Ok(());
        }
    }
}

/// Read one request; `None` on clean EOF.
fn read_request(reader: &mut BufReader<TcpStream>) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut parts = line.trim_end().split(' ');
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    if method.is_empty() {
        return Ok(None);
    }

    let (path, query_str) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };
    let mut query = HashMap::new();
    for pair in query_str.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        query.insert(url_decode(k), url_decode(v));
    }

    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 {
            break;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; len.min(16 * 1024 * 1024)];
    if !body.is_empty() {
        reader.read_exact(&mut body)?;
    }

    Ok(Some(Request {
        method,
        path,
        query,
        headers,
        body,
    }))
}

fn write_response(w: &mut TcpStream, res: &Response, keep_alive: bool) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {} {}\r\n", res.status, res.reason);
    for (k, v) in &res.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n", res.body.len()));
    head.push_str(if keep_alive {
        "Connection: keep-alive\r\n"
    } else {
        "Connection: close\r\n"
    });
    head.push_str("\r\n");
    w.write_all(head.as_bytes())?;
    w.write_all(&res.body)?;
    w.flush()
}

pub fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() + 1 && i + 2 < bytes.len() + 1 => {
                let hex = &s[i + 1..(i + 3).min(s.len())];
                if hex.len() == 2 {
                    if let Ok(v) = u8::from_str_radix(hex, 16) {
                        out.push(v);
                        i += 3;
                        continue;
                    }
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encode a path component (used by manifest redirects).
pub fn url_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Serve a deployed file with a best-guess content type.
pub fn serve_file(ctx: &Context, rel: &str) -> Response {
    let Some(bytes) = ctx.asset_bytes(rel) else {
        return Response::not_found();
    };
    let ct = match Path::new(rel).extension().and_then(|e| e.to_str()) {
        Some("json") => "application/json",
        Some("ini") => "text/plain",
        Some("manifest") => "application/octet-stream",
        Some("chunk") => "application/octet-stream",
        Some("png") | Some("jpg") => "application/octet-stream",
        _ => "application/octet-stream",
    };
    Response::bytes(ct, bytes)
}
