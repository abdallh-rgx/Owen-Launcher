#![forbid(unsafe_code)]
//! Owen Launcher core — the 1:1 safe-Rust port of the Owen-Launcher Android app.
//!
//! Layering (bottom → top):
//! * `http`   — shared HTTP abstraction (blocking reqwest natively, ehttp on wasm).
//! * `uptodown` — the Uptodown layer (api / resolver / installer).
//! * `install` — the PackageInstaller layer (helper + result bus).
//! * `core`   — the app core: diag log, player store, generated catalogs and the
//!   lawin server-side suite (settings / config.ini / deployer / login web
//!   server / backend manager). All user-facing strings say "Owen Server".
//! * `i18n`   — the 15 Fortnite languages.
//! * `backend`— the app layer that talks to / hosts the Owen Server
//!   (accounts, hosting panel, redirecter menu) and marshals results back
//!   onto the egui loop.

pub mod http;
pub mod uptodown;
pub mod install;
pub mod core;
pub mod i18n;
pub mod backend;
