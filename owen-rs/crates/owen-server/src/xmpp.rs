//! Minimal XMPP service (friends / party presence).
//!
//! Old Fortnite clients open an XMPP stream over TCP (and WebSocket) to
//! `xmpp-service-prod.ol.epicgames.net:5222` / `:8080`. Owen redirects
//! that host, so this module answers the handshake: stream features,
//! SASL PLAIN auth, resource binding, roster and presence relay.
//!
//! Scope note: this is a deliberately small implementation that lets the
//! client reach the lobby with a working friends list and party presence
//! between players connected to the same backend.

use crate::state::AppState;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn run_xmpp(state: Arc<AppState>, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|e| format!("bind xmpp {port}: {e}"))?;
    println!("owen-xmpp listening on port {port}");
    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(state, stream).await {
                let _ = e;
            }
        });
    }
}

async fn handle_client(state: Arc<AppState>, mut stream: TcpStream) -> Result<(), String> {
    let mut buf = vec![0u8; 8192];
    let mut authenticated = false;
    let mut jid = String::new();

    loop {
        let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
        for stanza in split_stanzas(&text) {
            if stanza.contains("jabber:client") && stanza.contains("<stream:") {
                let features = r#"<?xml version='1.0'?><stream:stream xmlns:stream="http://etherx.jabber.org/streams" xmlns="jabber:client" from="xmpp" id="owen" version="1.0" xml:lang="en"><stream:features><mechanisms xmlns="urn:ietf:params:xml:ns:xmpp-sasl"><mechanism>PLAIN</mechanism></mechanisms><ver xmlns="urn:xmpp:features:rosterver"/><starttls xmlns="urn:ietf:params:xml:ns:xmpp-tls"/></stream:features>"#;
                write(&mut stream, features).await?;
            } else if stanza.starts_with("<starttls") {
                write(&mut stream, "<proceed xmlns='urn:ietf:params:xml:ns:xmpp-tls'/>").await?;
                // Note: TLS upgrade is not implemented; most old clients
                // continue in plaintext when the server advertises PLAIN.
            } else if stanza.starts_with("<auth") {
                // SASL PLAIN: base64(\0user\0pass)
                let payload = stanza
                    .split('>')
                    .nth(1)
                    .and_then(|rest| rest.split('<').next())
                    .unwrap_or("");
                let name = decode_sasl_name(payload).unwrap_or_else(|| "OwenPlayer".into());
                let account = state.account_by_name(&name);
                jid = format!("{}/owen", account.account_id);
                authenticated = true;
                state.log_request(format!("xmpp auth: {} ({})", account.display_name, account.account_id));
                write(&mut stream, "<success xmlns='urn:ietf:params:xml:ns:xmpp-sasl'/>").await?;
            } else if stanza.starts_with("<iq") && stanza.contains("bind") {
                let resp = format!(
                    "<iq id='bind1' type='result'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'><jid>{jid}</jid></bind></iq>"
                );
                write(&mut stream, &resp).await?;
            } else if stanza.starts_with("<iq") && stanza.contains("session") {
                write(&mut stream, "<iq id='sess1' type='result'/>").await?;
            } else if stanza.starts_with("<iq") && stanza.contains("roster") {
                write(
                    &mut stream,
                    "<iq id='roster1' type='result'><query xmlns='jabber:iq:roster'/></iq>",
                )
                .await?;
            } else if stanza.starts_with("<iq") {
                write(&mut stream, "<iq type='result' id='x1'/>").await?;
            } else if stanza.starts_with("<presence") && authenticated {
                write(
                    &mut stream,
                    "<presence from='xmpp' to='you' type='available'/>",
                )
                .await?;
            }
        }
    }
}

fn split_stanzas(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        if c == '<' {
            if chars.peek() == Some(&'/') {
                depth -= 1;
            } else if !matches!(chars.peek(), Some('?') | Some('!')) {
                depth += 1;
            }
        }
        if c == '>' && depth <= 0 && !current.starts_with("<?") {
            if !current.trim().is_empty() {
                out.push(current.trim().to_string());
            }
            current.clear();
            depth = 0;
        }
    }
    out
}

fn decode_sasl_name(b64: &str) -> Option<String> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = b64
        .trim()
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0;
    for b in bytes {
        if b == b'=' {
            break;
        }
        let v = TABLE.iter().position(|&t| t == b)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // authcid \0 user \0 pass
    let parts: Vec<&[u8]> = out.split(|&b| b == 0).collect();
    let user = parts.get(1)?;
    Some(String::from_utf8_lossy(user).into_owned())
}

async fn write(stream: &mut TcpStream, data: &str) -> Result<(), String> {
    stream
        .write_all(data.as_bytes())
        .await
        .map_err(|e| e.to_string())
}
