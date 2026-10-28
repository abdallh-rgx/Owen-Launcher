//! Thin binary wrapper. The real implementation lives in the library so
//! the CLI and the launcher can embed the server.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    owen_server::run_server_from_args(&args);
}
