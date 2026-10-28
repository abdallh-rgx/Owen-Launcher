//! owen-lawin CLI — run the LawinServer-compatible backend standalone.
//!
//! Usage:
//!   owen-lawin --data-dir ./lawin --http-port 3551 --xmpp-port 5222

use std::path::PathBuf;

fn main() {
    let mut opts = owen_lawin::ServerOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => {
                if let Some(v) = args.next() {
                    opts.data_dir = PathBuf::from(v);
                }
            }
            "--http-port" => {
                if let Some(v) = args.next() {
                    opts.http_port = v.parse().unwrap_or(opts.http_port);
                }
            }
            "--xmpp-port" => {
                if let Some(v) = args.next() {
                    opts.xmpp_port = v.parse().unwrap_or(opts.xmpp_port);
                }
            }
            "--display-name" => {
                if let Some(v) = args.next() {
                    opts.display_name = v;
                }
            }
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(1);
            }
        }
    }
    println!(
        "[owen-lawin] HTTP 0.0.0.0:{}  XMPP 0.0.0.0:{}  data: {}",
        opts.http_port,
        opts.xmpp_port,
        opts.data_dir.display()
    );
    if let Err(e) = owen_lawin::run(opts) {
        eprintln!("[owen-lawin] fatal: {e}");
        std::process::exit(1);
    }
}
