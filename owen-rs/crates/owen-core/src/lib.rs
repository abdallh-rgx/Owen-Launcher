//! # owen-core
//!
//! Safe-Rust core of the Owen stack. Everything in this crate is written with
//! zero `unsafe` code. It provides:
//!
//! * [`elf`] - minimal ELF64 little-endian parser (program headers, PT_LOAD
//!   segments, dynamic symbol table) used to map file offsets to virtual
//!   addresses.
//! * [`scanner`] - automatic detection of `curl_easy_setopt` inside **any**
//!   stripped AArch64 ELF (libUE4.so, libUnreal.so, libEOSSDK.so) using the
//!   four-layer strategy: `MOV W0, #0x2b` return, `CBZ X0` null check, `BL`
//!   into `Curl_vsetopt` and a backwards prologue walk.
//! * [`version`] - Fortnite version detection (ASCII and UTF-16LE release
//!   strings).
//! * [`urlrule`] - URL parsing plus the redirect rule engine. Both `http://`
//!   and `https://` schemes are first-class citizens.
//! * [`profiles`] - per-version redirect profiles (Kairos WebView era,
//!   native-curl era, FCurl offsets fallback).
//! * [`manifest`] - `cloudcontent.json` ingestion and local manifest
//!   generation for the auto manifest redirecter.
//! * [`pak`] - Unreal Engine pak index parser used to extract per-season
//!   Battle Pass data.
//! * [`battlepass`] - Battle Pass season extraction on top of [`pak`].
//! * [`config`] - shared configuration model.

pub mod battlepass;
pub mod config;
pub mod elf;
pub mod manifest;
pub mod pak;
pub mod profiles;
pub mod scanner;
pub mod urlrule;
pub mod version;

pub use scanner::{scan_image, scan_path, Candidate, ScanReport};
