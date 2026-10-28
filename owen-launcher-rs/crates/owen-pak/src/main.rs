//! CLI wrapper — the real implementation lives in the library so the web
//! console (and future launcher integration) can call it directly.

fn main() -> std::process::ExitCode {
    owen_pak::main_entry()
}
