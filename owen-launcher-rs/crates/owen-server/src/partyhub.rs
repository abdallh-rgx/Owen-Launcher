//! Native PartyHub (Kairos companion app, 12.61-15.50 era) support:
//! static SPA serving from a data directory plus the Apollo-shaped
//! graphql responder that answers every query the app sends — natively,
//! with no external service.

use crate::state::{AppState, Account};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

/// Bridge script injected into the PartyHub webview (replaced by the
/// main agent; see assets/partyhub-bridge.js).
pub const OWEN_BRIDGE_JS: &str = include_str!("../assets/partyhub-bridge.js");

pub fn partyhub_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/partyhub", get(partyhub_index))
        .route("/partyhub/", get(partyhub_index))
        .route("/partyhub/owen-bridge.js", get(partyhub_bridge))
        .route("/partyhub/graphql", post(partyhub_graphql))
        // NOTE: axum 0.7 / matchit 0.7 does not allow a catch-all in the
        // middle of a pattern (`/partyhub/*rest/owen-graphql`), so the
        // POST wildcard handler itself dispatches on the `owen-graphql`
        // suffix.
        .route("/partyhub/*rest", get(partyhub_static).post(partyhub_static_post))
}

// ---------------------------------------------------------------------------
// static serving
// ---------------------------------------------------------------------------

async fn partyhub_index() -> Response {
    Redirect::temporary("/partyhub/offline/en-US/").into_response()
}

async fn partyhub_bridge() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/javascript"),
            ("cache-control", "no-store"),
        ],
        OWEN_BRIDGE_JS,
    )
        .into_response()
}

async fn partyhub_static(State(state): State<Arc<AppState>>, Path(rest): Path<String>) -> Response {
    serve_partyhub_path(&state, &rest)
}

/// POST /partyhub/<anything>/owen-graphql — the suffix-routed graphql
/// endpoint (Kairos apps request it under their locale path).
async fn partyhub_static_post(
    State(state): State<Arc<AppState>>,
    Path(rest): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // The Kairos bundle resolves "./graphql" against its nested locale path
    // (e.g. /partyhub/offline/en-US/graphql) — accept both suffixes so the
    // app also works if the bridge has not intercepted the request.
    let is_graphql = rest == "owen-graphql" || rest == "graphql"
        || rest.ends_with("/owen-graphql") || rest.ends_with("/graphql");
    if !is_graphql {
        return jresp(
            StatusCode::NOT_FOUND,
            json!({"errorCode": "errors.com.owen.no_route"}),
        );
    }
    state.log_request(format!("POST /partyhub/{rest}"));
    ok_json(graphql_answer(&state, &headers, &body))
}

/// Serve a file from the PartyHub dir with path-traversal protection,
/// correct MIME and SPA fallback to `offline/entry/index.html`.
///
/// Two overrides keep a stale third-party copy from breaking the hub:
/// * any path ending in `owen-bridge.js` is answered with the **embedded
///   v4 bridge** (the on-disk copies in old shippedweb trees are the v3
///   relay edition, which hangs the app on "CONNECTING…" when its relay is
///   not running);
/// * `index.html` gets the bridge `<script>` tag injected when the shipped
///   copy predates the Owen patch.
pub fn serve_partyhub_path(state: &AppState, rest: &str) -> Response {
    if rest.contains("..") || rest.starts_with('/') || rest.contains('\\') {
        return not_found();
    }
    // The embedded v4 bridge always wins — the CONNECTING-stuck bug was a
    // stale v3 bridge on disk pointing at a dead relay.
    let base = rest.rsplit('/').next().unwrap_or("");
    if base == "owen-bridge.js" {
        return (
            StatusCode::OK,
            [
                ("content-type", "application/javascript"),
                ("cache-control", "no-store"),
            ],
            OWEN_BRIDGE_JS,
        )
            .into_response();
    }
    let Some(dir) = state.partyhub_dir() else {
        // No third-party tree deployed: serve the embedded fallback page so
        // /partyhub never 404s (the login + install flow still works).
        return embedded_fallback_page(rest);
    };
    let candidate = dir.join(rest);
    if candidate.is_file() {
        if let Ok(data) = std::fs::read(&candidate) {
            if base == "index.html" {
                if let Some(page) = inject_bridge(&data) {
                    return (
                        StatusCode::OK,
                        [("content-type", "text/html"), ("cache-control", "no-store")],
                        page,
                    )
                        .into_response();
                }
            }
            return (
                StatusCode::OK,
                [
                    ("content-type", partyhub_mime(rest)),
                    ("cache-control", "no-store"),
                ],
                data,
            )
                .into_response();
        }
    }
    // Relative assets from nested SPA paths: the Kairos index references
    // "./main.kairos.*.js" etc., which resolve under /offline/<locale>/ —
    // the files actually live in offline/entry/, so retry by basename.
    let last = rest.rsplit('/').next().unwrap_or("");
    if !last.is_empty() && last.contains('.') {
        let alt = dir.join("offline/entry").join(last);
        if alt.is_file() {
            if let Ok(data) = std::fs::read(&alt) {
                return (
                    StatusCode::OK,
                    [
                        ("content-type", partyhub_mime(last)),
                        ("cache-control", "no-store"),
                    ],
                    data,
                )
                    .into_response();
            }
        }
    }
    // SPA fallback: extension-less last segment (or trailing slash) →
    // the offline entry page.
    if rest.ends_with('/') || !last.contains('.') {
        let index = dir.join("offline/entry/index.html");
        if let Ok(data) = std::fs::read(&index) {
            return (
                StatusCode::OK,
                [
                    ("content-type", "text/html"),
                    ("cache-control", "no-store"),
                ],
                data,
            )
                .into_response();
        }
    }
    not_found()
}

/// Inject `<script src="./owen-bridge.js"></script>` before the first
/// script/polyfill tag when the shipped index.html has none yet. Returns
/// None when the page already carries the bridge (or is not HTML).
fn inject_bridge(html: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(html).ok()?;
    if text.contains("owen-bridge.js") {
        return None;
    }
    let tag = "<script src=\"./owen-bridge.js\"></script>\n";
    let marker = text
        .find("<script")
        .or_else(|| text.find("</head>"))
        .or_else(|| text.find("<body"))?;
    let mut out = Vec::with_capacity(html.len() + tag.len());
    out.extend_from_slice(&html[..marker]);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(&html[marker..]);
    Some(out)
}

/// The fully-embedded PartyHub fallback page (no third-party Kairos tree
/// required): the v4 bridge + the glass login + an immediate transition
/// off the CONNECTING screen, so the hub always renders.
fn embedded_fallback_page(rest: &str) -> Response {
    // Asset-looking paths (a file extension) stay 404 — only SPA routes
    // and the entry get the embedded page.
    let last = rest.rsplit('/').next().unwrap_or("");
    if !rest.ends_with('/') && last.contains('.') {
        return not_found();
    }
    let page = format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>Owen PartyHub</title>\n<style>html,body{{margin:0;height:100%;background:#05060b;color:#f4f6fb;font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif}}#app{{display:flex;align-items:center;justify-content:center;height:100%}}</style>\n<script src=\"./owen-bridge.js\"></script>\n</head><body><div id=\"root\"></div>\n<script>/* The Kairos bundle normally mounts here; without it the v4 bridge\n   still answers the native handshake and drives the login flow. */\ndocument.getElementById('root').innerHTML = '<div style=\"text-align:center\"><h2>Owen PartyHub</h2><p>offline · bridge v4</p></div>';</script>\n</body></html>\n"
    );
    (
        StatusCode::OK,
        [("content-type", "text/html"), ("cache-control", "no-store")],
        page,
    )
        .into_response()
}

fn not_found() -> Response {
    jresp(
        StatusCode::NOT_FOUND,
        json!({"errorCode": "errors.com.owen.partyhub.not_found"}),
    )
}

fn jresp(code: StatusCode, v: Value) -> Response {
    (code, Json(v)).into_response()
}

fn ok_json(v: Value) -> Response {
    jresp(StatusCode::OK, v)
}

fn partyhub_mime(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html",
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "json" | "map" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// graphql
// ---------------------------------------------------------------------------

async fn partyhub_graphql(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.log_request("POST /partyhub/graphql".into());
    ok_json(graphql_answer(&state, &headers, &body))
}

/// One graphql request (parsed from JSON body or raw text).
#[derive(Debug, Clone)]
pub struct GqlRequest {
    pub query: String,
    pub variables: Value,
}

/// Parse a graphql POST body: JSON `{query, variables}`, a JSON array of
/// those, or raw graphql text. Returns one entry per operation.
pub fn parse_gql_body(body: &[u8]) -> Vec<GqlRequest> {
    let text = String::from_utf8_lossy(body);
    let trimmed = text.trim();
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(trimmed) {
            return items.iter().filter_map(gql_from_value).collect();
        }
    } else if trimmed.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            // A JSON object body must carry a "query" field; otherwise it
            // is not a graphql request at all.
            return gql_from_value(&v).into_iter().collect();
        }
    }
    if trimmed.is_empty() {
        Vec::new()
    } else {
        vec![GqlRequest {
            query: trimmed.to_string(),
            variables: json!({}),
        }]
    }
}

fn gql_from_value(v: &Value) -> Option<GqlRequest> {
    let query = v.get("query").and_then(|q| q.as_str())?;
    Some(GqlRequest {
        query: query.to_string(),
        variables: v.get("variables").cloned().unwrap_or(json!({})),
    })
}

/// Collapse a graphql query for fingerprinting: all whitespace removed so
/// spacing variants compare equal.
fn normalize_query(q: &str) -> String {
    q.split_whitespace().collect::<Vec<_>>().join("")
}

/// Identify the account a PartyHub request belongs to. NEVER fails: the
/// client must render, so we fall back to the legacy quick account.
pub fn identify_account(state: &AppState, headers: &HeaderMap) -> Account {
    if let Some(acc) = state.auth_account(headers) {
        return acc;
    }
    if let Some(user) = headers
        .get("x-owen-user")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if let Some(acc) = state.account_by_login(user) {
            return acc;
        }
        return state.account_by_name(user);
    }
    state.account_by_name("OwenTester")
}

/// Answer a PartyHub graphql POST body (single object or batch array).
pub fn graphql_answer(state: &AppState, headers: &HeaderMap, body: &Bytes) -> Value {
    let reqs = parse_gql_body(body);
    if reqs.is_empty() {
        return json!({"data": {}});
    }
    let account = identify_account(state, headers);
    if reqs.len() == 1 {
        return respond_graphql(state, &account, &reqs[0]);
    }
    Value::Array(
        reqs.iter()
            .map(|r| respond_graphql(state, &account, r))
            .collect(),
    )
}

/// Fingerprint one operation and build the Apollo-shaped `{"data": ...}`
/// response.
pub fn respond_graphql(state: &AppState, account: &Account, req: &GqlRequest) -> Value {
    // Re-fetch so batch sequences (updateSetting → mySetting) and reads
    // after mutations see current state.
    let account = state
        .account_by_id(&account.account_id)
        .unwrap_or_else(|| account.clone());
    let q = normalize_query(&req.query);
    state.log_request(format!(
        "GQL {}",
        &q.chars().take(150).collect::<String>()
    ));
    let vars = &req.variables;
    let id = account.account_id.as_str();
    let display = account.display_name.as_str();
    let email = if account.email.is_empty() {
        "player@owen.local"
    } else {
        account.email.as_str()
    };

    let data: Value = if q.contains("fortniteStatus") && q.contains("kairosStatus") {
        json!({
            "ContentControl": {"namespace": {"result": {"canUseVoiceChat": true}}},
            "LightSwitch": {
                "fortniteStatus": {"allowedActions": ["PLAY"], "status": "UP", "banned": false},
                "kairosStatus": {"status": "UP"}
            },
            "SocialBan": {"socialBan": null}
        })
    } else if q.contains("canUseVoiceChat") {
        json!({"ContentControl": {"namespace": {"result": {"canUseVoiceChat": true}}}})
    } else if q.contains("myAccount") && q.contains("externalAuths") {
        json!({
            "Account": {"myAccount": {
                "id": id, "displayName": display, "email": email,
                "country": "US", "externalAuths": []
            }},
            "Fortnite": {"myProfile": {"id": id}}
        })
    } else if q == "query{Account{myAccount{id}}}" {
        json!({"Account": {"myAccount": {"id": id}}})
    } else if q.contains("hasAccountAccepted") {
        json!({"Eula": {"hasAccountAccepted": {"accepted": true}}})
    } else if q.contains("getLatestAndConvert") {
        json!({"Eula": {"getLatestAndConvert": {
            "accepted": true, "body": "", "key": "fortnite",
            "locale": "en-US", "title": "", "version": 1
        }}})
    } else if q.contains("myAvailableSetting") {
        json!({"UserSettings": {"myAvailableSetting": null}})
    } else if q.contains("mySetting") {
        let value = vars
            .get("key")
            .and_then(|k| k.as_str())
            .and_then(|key| account.settings.get(key))
            .cloned()
            .unwrap_or(Value::Null);
        json!({"UserSettings": {"mySetting": value}})
    } else if q.contains("setting(key") && q.contains("accountIds") {
        json!({"UserSettings": {"setting": []}})
    } else if q.contains("notificationSettings") {
        json!({
            "Friends": {"notificationSettings": {"offline": null, "success": true, "message": null}},
            "PartySettings": {"notificationSettings": {"offline": null, "success": true, "message": null}}
        })
    } else if q.contains("allowedActions") {
        json!({"LightSwitch": {"serviceStatus": {"allowedActions": ["PLAY"], "status": "UP", "banned": false}}})
    } else if q.contains("getMySocialBan") {
        json!({"SocialBan": {"summary": {"warnings": [], "bans": []}}})
    } else if q.contains("summary(displayNames") {
        let friends: Vec<Value> = account
            .friends
            .iter()
            .filter_map(|fid| state.account_by_id(fid))
            .map(|f| json!({"accountId": f.account_id, "displayName": f.display_name}))
            .collect();
        json!({"Friends": {"summary": {
            "friends": friends, "incoming": [], "outgoing": [],
            "blocklist": [], "suggestions": []
        }}})
    } else if q.contains("blockList") {
        json!({"Friends": {"blockList": []}})
    } else if q.contains("friend(friendId") {
        json!({"Friends": {"friend": null}})
    } else if q.contains("accounts(accountIds") {
        json!({"Account": {"accounts": []}})
    } else if q.contains("friendshipStatus") {
        json!({"Account": {"account": null}})
    } else if q.contains("unlinkedDieselPlatforms") {
        json!({"Account": {"myAccount": {"unlinkedDieselPlatforms": []}}})
    } else if q.contains("myProfile") && q.contains("character") {
        json!({"Fortnite": {"myProfile": {
            "id": id,
            "character": {
                "templateId": "CID_029_Athena_Commando_F_Halloween",
                "attributes": {
                    "favorite": false, "item_seen": true,
                    "level": account.level, "variants": []
                }
            },
            "backpack": {"templateId": ""}
        }}})
    } else if q.contains("updateSetting") {
        if let Some(key) = vars.get("key").and_then(|k| k.as_str()) {
            let value = vars.get("value").cloned().unwrap_or(Value::Null);
            state.set_setting(&account.account_id, key, value);
        }
        json!({"UserSettings": {"updateSetting": {"success": true}}})
    } else if q.contains("acceptEula") {
        json!({"Eula": {"acceptEula": {"accepted": true}}})
    } else if q.contains("setNotificationSettings") {
        json!({
            "Friends": {"notificationSettings": {"offline": null, "success": true, "message": null}},
            "PartySettings": {"notificationSettings": {"offline": null, "success": true, "message": null}}
        })
    } else if q.contains("reportPlayer") {
        json!({"Fortnite": {"reportPlayer": {
            "success": true, "status": "OK", "errorCode": "", "errorMessage": ""
        }}})
    } else if q.starts_with("mutation") {
        json!({"success": true})
    } else {
        json!({})
    };

    json!({"data": data})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> (AppState, std::path::PathBuf) {
        crate::state::testing::test_state()
    }

    fn headers_with_user(user: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-owen-user",
            axum::http::HeaderValue::from_str(user).expect("valid header"),
        );
        headers
    }

    fn gql(query: &str) -> GqlRequest {
        GqlRequest {
            query: query.to_string(),
            variables: json!({}),
        }
    }

    #[test]
    fn normalize_query_strips_all_whitespace() {
        assert_eq!(
            normalize_query("query { Account { myAccount { id } } }"),
            "query{Account{myAccount{id}}}"
        );
    }

    #[test]
    fn parse_body_json_object_array_and_raw() {
        let one = parse_gql_body(br#"{"query":"query{A{b}}","variables":{"k":1}}"#.as_slice());
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].query, "query{A{b}}");
        assert_eq!(one[0].variables["k"], 1);

        let batch = parse_gql_body(
            br#"[{"query":"query{A{b}}"},{"query":"query{C{d}}"}]"#.as_slice(),
        );
        assert_eq!(batch.len(), 2);

        let raw = parse_gql_body(b"query { Account { myAccount { id } } }".as_slice());
        assert_eq!(raw.len(), 1);
        assert!(raw[0].query.contains("myAccount"));

        assert!(parse_gql_body(b"".as_slice()).is_empty());
        assert!(parse_gql_body(b"{}".as_slice()).is_empty());
    }

    #[test]
    fn status_query_answers_combined_services() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("StatusUser");
        let resp = respond_graphql(
            &state,
            &acc,
            &gql(
                "query { ContentControl { namespace { result { canUseVoiceChat } } } \
                 LightSwitch { fortniteStatus { allowedActions status } kairosStatus { status } } \
                 SocialBan { socialBan { status } } }",
            ),
        );
        assert_eq!(resp["data"]["LightSwitch"]["fortniteStatus"]["status"], "UP");
        assert_eq!(resp["data"]["LightSwitch"]["fortniteStatus"]["allowedActions"][0], "PLAY");
        assert_eq!(resp["data"]["LightSwitch"]["kairosStatus"]["status"], "UP");
        assert_eq!(resp["data"]["ContentControl"]["namespace"]["result"]["canUseVoiceChat"], true);
        assert!(resp["data"]["SocialBan"]["socialBan"].is_null());
    }

    #[test]
    fn my_account_query_uses_fake_account() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("KairosUser");
        let resp = respond_graphql(
            &state,
            &acc,
            &gql("query { Account { myAccount { id displayName email country externalAuths } } }"),
        );
        let my = &resp["data"]["Account"]["myAccount"];
        assert_eq!(my["id"], acc.account_id.as_str());
        assert_eq!(my["displayName"], "KairosUser");
        assert_eq!(my["email"], "player@owen.local");
        assert_eq!(my["country"], "US");
        assert_eq!(my["externalAuths"].as_array().unwrap().len(), 0);
        assert_eq!(resp["data"]["Fortnite"]["myProfile"]["id"], acc.account_id.as_str());
    }

    #[test]
    fn exact_id_query_matches_after_normalization() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("IdOnly");
        for spacing in [
            "query{Account{myAccount{id}}}",
            "query { Account { myAccount { id } } }",
            "query\n { Account {\n myAccount { id }\n } }",
        ] {
            let resp = respond_graphql(&state, &acc, &gql(spacing));
            assert_eq!(resp["data"]["Account"]["myAccount"]["id"], acc.account_id.as_str());
        }
    }

    #[test]
    fn update_setting_persists_and_my_setting_reads_it() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("SettingsUser");
        let req = GqlRequest {
            query: "mutation { UserSettings { updateSetting { success } } }".into(),
            variables: json!({"key": "favoriteMode", "value": "solo"}),
        };
        let resp = respond_graphql(&state, &acc, &req);
        assert_eq!(resp["data"]["UserSettings"]["updateSetting"]["success"], true);

        // persisted on the stored account
        let stored = state.account_by_id(&acc.account_id).expect("stored");
        assert_eq!(
            stored.settings.get("favoriteMode"),
            Some(&json!("solo"))
        );

        // mySetting now returns it
        let read = GqlRequest {
            query: "query { UserSettings { mySetting } }".into(),
            variables: json!({"key": "favoriteMode"}),
        };
        let resp = respond_graphql(&state, &acc, &read);
        assert_eq!(resp["data"]["UserSettings"]["mySetting"], "solo");

        // unknown key → null
        let missing = GqlRequest {
            query: "query { UserSettings { mySetting } }".into(),
            variables: json!({"key": "nope"}),
        };
        let resp = respond_graphql(&state, &acc, &missing);
        assert!(resp["data"]["UserSettings"]["mySetting"].is_null());
    }

    #[test]
    fn friends_summary_maps_friend_accounts() {
        let (state, _dir) = test_state();
        let mut acc = state.account_by_name("SocialUser");
        let friend = state.account_by_name("BestFriend");
        state
            .update_account(&acc.account_id, |a| {
                a.friends.push(friend.account_id.clone());
            })
            .expect("add friend");
        acc = state.account_by_id(&acc.account_id).expect("fresh");

        let resp = respond_graphql(
            &state,
            &acc,
            &gql("query { Friends { summary(displayNames: [\"id1\"]) { friends { accountId displayName } incoming outgoing blocklist suggestions } } }"),
        );
        let summary = &resp["data"]["Friends"]["summary"];
        let friends = summary["friends"].as_array().expect("friends array");
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0]["accountId"], friend.account_id.as_str());
        assert_eq!(friends[0]["displayName"], "BestFriend");
        assert_eq!(summary["incoming"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn character_profile_and_misc_queries() {
        let (state, _dir) = test_state();
        let mut acc = state.account_by_name("CharUser");
        state.update_account(&acc.account_id, |a| a.level = 42).expect("level");
        acc = state.account_by_id(&acc.account_id).expect("fresh");

        let resp = respond_graphql(
            &state,
            &acc,
            &gql("query { Fortnite { myProfile { id character { templateId attributes { level } } backpack { templateId } } } }"),
        );
        let profile = &resp["data"]["Fortnite"]["myProfile"];
        assert_eq!(profile["id"], acc.account_id.as_str());
        assert_eq!(profile["character"]["templateId"], "CID_029_Athena_Commando_F_Halloween");
        assert_eq!(profile["character"]["attributes"]["level"], 42);
        assert_eq!(profile["backpack"]["templateId"], "");

        let resp = respond_graphql(&state, &acc, &gql("query { Eula { hasAccountAccepted { accepted } } }"));
        assert_eq!(resp["data"]["Eula"]["hasAccountAccepted"]["accepted"], true);

        let resp = respond_graphql(&state, &acc, &gql("query { Friends { blockList } }"));
        assert_eq!(resp["data"]["Friends"]["blockList"].as_array().unwrap().len(), 0);

        let resp = respond_graphql(&state, &acc, &gql("mutation { reportPlayer { success } }"));
        assert_eq!(resp["data"]["Fortnite"]["reportPlayer"]["status"], "OK");
    }

    #[test]
    fn generic_mutation_and_unknown_query() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("MiscUser");
        let resp = respond_graphql(&state, &acc, &gql("mutation { Something { else } }"));
        assert_eq!(resp["data"]["success"], true);
        let resp = respond_graphql(&state, &acc, &gql("query { Nothing { known } }"));
        assert_eq!(resp["data"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn batch_body_answers_array() {
        let (state, _dir) = test_state();
        let headers = headers_with_user("BatchUser");
        let body = Bytes::from_static(
            br#"[{"query":"query{Account{myAccount{id}}}"},
                       {"query":"query{Friends{blockList}}"}]"#,
        );
        let resp = graphql_answer(&state, &headers, &body);
        let arr = resp.as_array().expect("array answer");
        assert_eq!(arr.len(), 2);
        assert!(arr[0]["data"]["Account"]["myAccount"]["id"].is_string());
        assert_eq!(arr[1]["data"]["Friends"]["blockList"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn identify_account_never_fails() {
        let (state, _dir) = test_state();
        // no headers at all → OwenTester quick account
        let acc = identify_account(&state, &HeaderMap::new());
        assert_eq!(acc.display_name, "OwenTester");
        assert_eq!(acc.account_id.len(), 32);
        // X-Owen-User finds/creates the named account
        let acc = identify_account(&state, &headers_with_user("HeaderUser"));
        assert_eq!(acc.display_name, "HeaderUser");
        // a session bearer wins
        let session = state.create_session(&acc.account_id);
        let mut headers = headers_with_user("SomeoneElse");
        headers.insert(
            "authorization",
            axum::http::HeaderValue::from_str(&format!("Bearer {}", session.access_token))
                .expect("valid header"),
        );
        let acc = identify_account(&state, &headers);
        assert_eq!(acc.display_name, "HeaderUser");
    }

    #[test]
    fn static_serving_traversal_and_fallback() {
        let (_state, dir) = test_state();
        let ph = dir.join("partyhub");
        std::fs::create_dir_all(ph.join("offline/entry")).expect("mkdir");
        std::fs::write(ph.join("offline/entry/index.html"), b"<html>offline</html>").expect("write");
        std::fs::create_dir_all(ph.join("assets")).expect("mkdir");
        std::fs::write(ph.join("assets/app.js"), b"console.log(1)").expect("write");

        let state = AppState::load_with(
            dir.clone(),
            owen_core::config::OwenConfig::default(),
            Some(ph.clone()),
            Vec::new(),
        );

        // real file, correct mime
        let resp = serve_partyhub_path(&state, "assets/app.js");
        assert_eq!(resp.status(), StatusCode::OK);
        // SPA fallback for extension-less route
        let resp = serve_partyhub_path(&state, "offline/en-US");
        assert_eq!(resp.status(), StatusCode::OK);
        // trailing slash also falls back
        let resp = serve_partyhub_path(&state, "offline/en-US/");
        assert_eq!(resp.status(), StatusCode::OK);
        // missing file with extension → 404
        let resp = serve_partyhub_path(&state, "assets/missing.png");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        // traversal rejected
        for bad in ["../accounts.json", "assets/../../accounts.json", "/etc/passwd"] {
            let resp = serve_partyhub_path(&state, bad);
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{bad}");
        }
        // no partyhub dir → the EMBEDDED fallback page (the hub never 404s;
        // stale-tree CONNECTING hangs are gone)
        let (state3, _d3) = test_state();
        let resp = serve_partyhub_path(&state3, "anything");
        assert_eq!(resp.status(), StatusCode::OK);
        // …but a missing asset-looking path still 404s
        let resp = serve_partyhub_path(&state3, "assets/missing.png");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        // the bridge is ALWAYS the embedded v4, even when a stale v3 sits
        // on disk
        let resp = serve_partyhub_path(&state, "offline/entry/owen-bridge.js");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
