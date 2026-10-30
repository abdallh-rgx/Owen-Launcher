//! CloudDir serving for the 12.41 content-build flow (InstallBundleSourceBPS).
//!
//! The game's `BuildInfoServiceV2` (launcher-public-service, an epicgames.net
//! host → curl-rewritten to this backend) answers with the real CloudDir
//! distribution URLs. Two shapes reach us afterwards (either directly or via
//! `cache.loc.epicgames.net`):
//!
//! ```text
//! /Builds/Fortnite/Content/CloudDir/<hash>.manifest
//! /Builds/Fortnite/Content/CloudDir/<hash>/BackgroundDownloads.ini
//! /Builds/Fortnite/Content/CloudDir/<hash>/Full.ini
//! /ChunkDbs/<hash>_FortniteBR.partNN.chunkdb          (S3Download entries)
//! ```
//!
//! Resolution per path: **local package first** (`<data_dir>/clouddir/1241/`,
//! the verified 12.41 fn1241 package — byte-identical to the CDN), then a
//! **302 to the live CDN** (`epicgames-download1.akamaized.net`), so the
//! ~3GB of chunkdbs stream straight from Akamai instead of transiting the
//! runner. The curl hook rewrites the epic hosts to us; we bounce the bulk
//! data back to the real CDN and keep every other request local.

use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use std::sync::Arc;

/// The verified 12.41 build label → manifest hash (MANIFEST-HASHES.md).
pub const HASH_1241: &str = "aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg";
/// The live CDN every CloudDir path falls back to.
pub const LIVE_CDN: &str = "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir";

/// The subdirectory (under the data dir) holding the local 12.41 package:
/// BackgroundDownloads.ini, Full.ini, `<HASH>.manifest`.
const LOCAL_DIR: &str = "clouddir/1241";

pub fn clouddir_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/Builds/Fortnite/Content/CloudDir/:rest", get(clouddir_rest))
        .route("/ChunkDbs/:rest", get(chunkdb_rest))
}

/// `/Builds/Fortnite/Content/CloudDir/<rest>` — local package, then CDN.
async fn clouddir_rest(
    State(state): State<Arc<AppState>>,
    Path(rest): Path<String>,
) -> Response {
    serve_local_or_cdn(&state, &rest, &format!("{LIVE_CDN}/{rest}"))
}

/// `/ChunkDbs/<rest>` — the S3Download shapes from BackgroundDownloads.ini
/// (relative `/ChunkDbs/...` paths resolve against the CloudDir base).
async fn chunkdb_rest(
    State(state): State<Arc<AppState>>,
    Path(rest): Path<String>,
) -> Response {
    serve_local_or_cdn(&state, &format!("ChunkDbs/{rest}"), &format!("{LIVE_CDN}/ChunkDbs/{rest}"))
}

fn serve_local_or_cdn(state: &Arc<AppState>, local_rel: &str, cdn_url: &str) -> Response {
    // 1. the local verified package
    if let Some(data) = state.data_file(&format!("{LOCAL_DIR}/{local_rel}")) {
        let ct = if local_rel.ends_with(".ini") {
            "text/plain; charset=utf-8"
        } else if local_rel.ends_with(".manifest") {
            "application/octet-stream"
        } else {
            "application/octet-stream"
        };
        state.log_request(format!("CloudDir LOCAL {local_rel} ({} bytes)", data.len()));
        return (
            StatusCode::OK,
            [("content-type", ct), ("cache-control", "no-store")],
            data,
        )
            .into_response();
    }
    // 2. bounce to the live CDN (curl follows the 302 transparently)
    state.log_request(format!("CloudDir CDN→ {cdn_url}"));
    Redirect::temporary(cdn_url).into_response()
}

/// Health/summary used by the E2E script: which files the local package has.
pub fn local_package_summary(state: &Arc<AppState>) -> Json<serde_json::Value> {
    let mut files = Vec::new();
    for name in [
        format!("{HASH_1241}.manifest"),
        format!("{HASH_1241}/BackgroundDownloads.ini"),
        format!("{HASH_1241}/Full.ini"),
    ] {
        let len = state
            .data_file(&format!("{LOCAL_DIR}/{name}"))
            .map(|d| d.len())
            .unwrap_or(0);
        files.push(json!({"file": name, "bytes": len}));
    }
    Json(json!({
        "hash": HASH_1241,
        "liveCdn": LIVE_CDN,
        "package": files,
    }))
}
