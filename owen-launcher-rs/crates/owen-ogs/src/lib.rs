//! libOwenOGS.so — OGS 12.41 for Android (Task 4-ogs).
//!
//! Rust side of the port: config assembly + boot orchestration.  All game
//! logic lives in the C++ tree (`cpp/`), compiled against the MobileDumper-7
//! dump of the SAME game by build.rs (only when `OGS_SDK_DIR` is set) and
//! linked into this cdylib together with the audited `owen-hook` engine.
//!
//! Entry point mirrors `crates/owen-redirecter/src/lib.rs`: an
//! `.init_array` constructor (dlopen time, before the JVM exists) starts an
//! idempotent worker thread that
//!   1. loads / merges `ogsen.json` (serde_json) from the standard candidate
//!      locations,
//!   2. writes the effective config back next to the log (best effort),
//!   3. hands the JSON string to the C++ entry point `ogs_main(const char*)`
//!      through the plain C ABI.
//!
//! When the C++ part was not compiled (no `OGS_SDK_DIR` at build time) the
//! `stub` module logs and returns instead — `cargo check --workspace` stays
//! green on hosts without the dump or the NDK.

#[cfg(ogs_cpp)]
mod full;
#[cfg(ogs_cpp)]
pub use full::*;

#[cfg(not(ogs_cpp))]
mod stub;
#[cfg(not(ogs_cpp))]
pub use stub::*;

#[allow(dead_code)]
mod common;
