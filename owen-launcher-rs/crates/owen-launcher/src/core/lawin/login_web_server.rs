#![forbid(unsafe_code)]
//! 1:1 port of `core/lawin/LoginWebServer.kt` — the tiny extra login web server
//! on port 48292 that serves the /login page and redirects into the game with
//! the player name. The page is OWEN-branded (branding rule: the original
//! LAWIN/LAWINSERVER headings become OWEN/OWEN SERVER).
//!
//! Native targets run the real socket server; on wasm the module compiles with
//! the same API but `start` reports failure (the preview talks to the hosted
//! Owen Server instead).

use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};
use std::sync::Mutex;

use crate::core::diag;

const TAG: &str = "LoginWebServer";
pub const PREFERRED_PORT: u16 = 48292;

static STATE: AtomicI32 = AtomicI32::new(-1); // port, -1 = stopped
static PLAYERNAME: Mutex<String> = Mutex::new(String::new());
static RUNNING: AtomicU8 = AtomicU8::new(0);

/// `LoginWebServer.currentPort()`
pub fn current_port() -> i32 {
    STATE.load(Ordering::SeqCst)
}

/// `LoginWebServer.setPlayername(name)`
pub fn set_playername(name: &str) {
    *PLAYERNAME.lock().unwrap() = name.trim().to_string();
}

/// `LoginWebServer.loginUrl()`
pub fn login_url() -> Option<String> {
    let port = STATE.load(Ordering::SeqCst);
    if port <= 0 || RUNNING.load(Ordering::SeqCst) == 0 {
        return None;
    }
    Some(format!("http://127.0.0.1:{port}/login"))
}

/// `LoginWebServer.start(name)` — 20 candidate ports, accept loop per conn.
pub fn start(name: &str) -> bool {
    set_playername(name);
    #[cfg(not(target_arch = "wasm32"))]
    {
        if RUNNING.load(Ordering::SeqCst) != 0 {
            return true;
        }
        for candidate in PREFERRED_PORT..PREFERRED_PORT + 20 {
            match start_on(candidate) {
                Ok(()) => {
                    diag::i(TAG, &format!("Login web server running on http://127.0.0.1:{candidate}/login"));
                    return true;
                }
                Err(e) => {
                    diag::i(TAG, &format!("port {candidate} busy: {e}"));
                }
            }
        }
        diag::e(TAG, "Could not start the login web server (ports busy)");
        false
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        false
    }
}

/// `LoginWebServer.stop()`
pub fn stop() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        stop_socket();
    }
    #[cfg(target_arch = "wasm32")]
    {}
}

/* --------------------------------- page --------------------------------- */

/// `LoginWebServer.page()` — the original layout, rebranded. The HTML is the
/// Kotlin template with LAWIN→OWEN, LAWINSERVER→OWEN SERVER.
fn page() -> String {
    let safe = PLAYERNAME
        .lock()
        .unwrap()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Owen Server - Login</title>
<style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    body {{
        background: #0b0b0f;
        color: #eaeaea;
        font-family: 'Segoe UI', 'Roboto', system-ui, sans-serif;
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
    }}
    .container {{ text-align: center; padding: 24px; width: 100%; max-width: 420px; }}
    .logo {{
        font-size: 64px;
        font-weight: 900;
        letter-spacing: 2px;
        background: linear-gradient(135deg, #ffb300, #ff8a00);
        -webkit-background-clip: text;
        background-clip: text;
        -webkit-text-fill-color: transparent;
        margin-bottom: 4px;
    }}
    .title {{
        font-size: 18px;
        letter-spacing: 8px;
        color: #9a9aa3;
        margin-bottom: 48px;
    }}
    input {{
        width: 100%;
        padding: 16px 20px;
        font-size: 18px;
        border-radius: 12px;
        border: 1px solid #2a2a35;
        background: #14141c;
        color: #fff;
        outline: none;
        text-align: center;
        margin-bottom: 18px;
        transition: border-color .2s;
    }}
    input:focus {{ border-color: #ffb300; }}
    .btn {{
        width: 100%;
        padding: 16px;
        font-size: 18px;
        font-weight: 700;
        border-radius: 12px;
        border: none;
        cursor: pointer;
        color: #241300;
        background: linear-gradient(135deg, #ffb300, #ff8a00);
        transition: transform .1s, filter .2s;
    }}
    .btn:active {{ transform: scale(.98); }}
    .note {{ margin-top: 32px; font-size: 12px; color: #5a5a63; }}
</style>
</head>
<body>
    <div class="container">
        <div class="logo">OWEN</div>
        <div class="title">OWEN SERVER</div>
        <input id="username" placeholder="Enter your name" maxlength="32" value="$safe" autocomplete="off">
        <button class="btn" onclick="redirect()">Launch Game</button>
        <div class="note">Enter the name you want to play with.<br>The game will open automatically.</div>
    </div>
<script>
    const input = document.getElementById('username');
    input.addEventListener('keyup', e => {{ if (e.key === 'Enter') redirect(); }});

    function redirect() {{
        const username = input.value.trim();
        if (!username) {{ input.style.borderColor = '#ffb300'; input.focus(); return; }}
        window.location.href = 'com.epicgames.fortnite://authorize/?code=' + encodeURIComponent(username);
    }}
</script>
</body>
</html>"#
    )
}

/* ------------------------------ native impl ------------------------------ */

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};

    struct StopSignal;
    static STOP: Mutex<Option<StopSignal>> = Mutex::new(None);

    pub(super) fn start_on(candidate: u16) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", candidate))?;
        STATE.store(candidate as i32, Ordering::SeqCst);
        RUNNING.store(1, Ordering::SeqCst);
        *STOP.lock().unwrap() = Some(StopSignal);
        std::thread::Builder::new()
            .name("login-web".into())
            .spawn(move || accept_loop(listener))?;
        Ok(())
    }

    pub(super) fn stop_socket() {
        STATE.store(-1, Ordering::SeqCst);
        RUNNING.store(0, Ordering::SeqCst);
        *STOP.lock().unwrap() = None;
        // listener closes when the accept thread errors out on next poll; the
        // port is released by the OS once the thread exits.
    }

    fn accept_loop(listener: TcpListener) {
        loop {
            if RUNNING.load(Ordering::SeqCst) == 0 {
                return;
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    std::thread::Builder::new()
                        .name("login-web-conn".into())
                        .spawn(move || handle(stream))
                        .ok();
                }
                Err(_) => return,
            }
        }
    }

    fn handle(mut client: TcpStream) {
        let _ = client.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        let Ok(reader) = client.try_clone() else { return };
        let mut reader = BufReader::new(reader);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return;
        }
        let parts: Vec<&str> = request_line.split(' ').collect();
        if parts.len() >= 2 {
            let method = parts[0];
            let path = parts[1].split('?').next().unwrap_or("");
            loop {
                let mut header = String::new();
                match reader.read_line(&mut header) {
                    Ok(0) => break,
                    Ok(_) if header.trim().is_empty() => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
            let body = page().into_bytes();
            let response = if method == "GET" && (path == "/login" || path == "/") {
                format!(
                    "HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
            } else {
                "HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
            };
            let _ = client.write_all(response.as_bytes());
            let _ = client.write_all(&body);
            let _ = client.flush();
        }
        let _ = read_drain(&mut client);
    }

    fn read_drain(stream: &mut TcpStream) {
        let mut buf = [0u8; 512];
        let _ = stream.read(&mut buf);
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native as impl_native;

#[cfg(not(target_arch = "wasm32"))]
fn start_on(candidate: u16) -> std::io::Result<()> {
    impl_native::start_on(candidate)
}

#[cfg(not(target_arch = "wasm32"))]
fn stop_socket() {
    impl_native::stop_socket()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_owen_branded_and_escapes() {
        set_playername("A<B>&\"' ");
        let html = page();
        assert!(html.contains("OWEN"));
        assert!(html.contains("OWEN SERVER"));
        assert!(!html.contains("LAWIN"));
        assert!(html.contains("&lt;B&gt;"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn server_serves_login_and_404() {
        use std::io::{Read, Write};
        set_playername("OwenTester");
        assert!(start("OwenTester"));
        let port = current_port() as u16;
        let mut sock = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let _ = sock.write_all(b"GET /login HTTP/1.0\r\nHost: x\r\n\r\n");
        let mut buf = String::new();
        sock.read_to_string(&mut buf).unwrap();
        assert!(buf.starts_with("HTTP/1.0 200 OK"));
        assert!(buf.contains("OWEN SERVER"));
        assert!(buf.contains("OwenTester"));

        let mut sock2 = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let _ = sock2.write_all(b"GET /nope HTTP/1.0\r\nHost: x\r\n\r\n");
        let mut buf2 = String::new();
        sock2.read_to_string(&mut buf2).unwrap();
        assert!(buf2.starts_with("HTTP/1.0 404"));
        stop();
        assert_eq!(current_port(), -1);
    }
}
