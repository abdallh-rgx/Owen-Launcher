//! XMPP — the minimal stream the Fortnite client needs to reach the lobby:
//! stream negotiation, SASL PLAIN auth, bind + session and presence.

use crate::Context;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub fn session(stream: TcpStream, _ctx: std::sync::Arc<Context>) {
    let Ok(clone) = stream.try_clone() else { return };
    let mut reader = BufReader::new(stream);
    let mut writer = clone;
    let mut authenticated = false;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("<stream:stream") || trimmed.starts_with("<?xml") {
            let _ = write(&mut writer,
                "<?xml version='1.0'?><stream:stream xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' id='owen' version='1.0' xml:lang='en'>\
                 <stream:features><mechanisms xmlns='urn:ietf:params:xml:ns:xmpp-sasl'><mechanism>PLAIN</mechanism></mechanisms></stream:features>");
            continue;
        }
        if trimmed.starts_with("<auth") {
            authenticated = true;
            let _ = write(&mut writer, "<success xmlns='urn:ietf:params:xml:ns:xmpp-sasl'/>");
            continue;
        }
        if trimmed.starts_with("<iq") {
            if trimmed.contains("bind") {
                let _ = write(&mut writer,
                    "<iq id='bind_1' type='result'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'><jid>owen@localhost/owen</jid></bind></iq>");
            } else if trimmed.contains("session") {
                let _ = write(&mut writer, "<iq id='sess_1' type='result'/>");
            } else {
                let _ = write(&mut writer, "<iq type='result' id='x'/>");
            }
            continue;
        }
        if trimmed.starts_with("<presence") && authenticated {
            let _ = write(&mut writer,
                "<presence from='owen@localhost' to='owen@localhost'><status>online</status></presence>");
            continue;
        }
        if trimmed.starts_with("</stream:stream") {
            let _ = write(&mut writer, "</stream:stream>");
            return;
        }
    }
}

fn write(w: &mut TcpStream, s: &str) -> std::io::Result<()> {
    w.write_all(s.as_bytes())?;
    w.flush()
}
