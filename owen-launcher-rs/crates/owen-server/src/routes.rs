//! HTTP routes: the game-facing API surface (LawinServer v2 compatible,
//! plus Reload-Backend features) and the Owen-specific endpoints.

use crate::login_page::LOGIN_PAGE;
use crate::seasons::{self, MAX_SEASON, SeasonMeta};
use crate::state::{iso_now, today, Account, AppState, Session};
use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn root_router(state: Arc<AppState>) -> Router {
    let app: Router<Arc<AppState>> = Router::new()
        // ---- Owen-specific ----
        .route("/", get(index))
        .route("/login", get(login_page))
        .merge(crate::partyhub::partyhub_router())
        .route("/api/launcher/register", post(launcher_register))
        .route("/api/launcher/me", get(launcher_me))
        .route("/api/launcher/me/season", post(launcher_me_season))
        .route("/api/launcher/logout", post(launcher_logout))
        .route("/api/launcher/seasons", get(launcher_seasons))
        .route("/api/launcher/seasons/:n", get(launcher_season))
        .route("/api/launcher/manifests", get(launcher_manifests))
        .route("/launcher/api/public/assets/:platform/:label/:app/:build", get(launcher_assets))
        .route("/manifest/:rest", get(manifest_file))
        // ---- admin (web panel) ----
        .merge(admin_router())
        // ---- account / oauth ----
        .route("/account/api/oauth/token", post(oauth_token))
        .route("/account/api/oauth/exchange", get(oauth_exchange))
        .route("/account/api/oauth/verify", get(oauth_verify))
        .route("/account/api/oauth/sessions/kill", post(sessions_kill))
        .route("/account/api/oauth/sessions/kill/:sess", post(sessions_kill))
        .route("/account/api/public/account", get(account_lookup))
        .route("/account/api/public/account/:id", get(account_info))
        .route("/account/api/public/account/:id/externalAuths", get(account_external_auths))
        .route("/account/api/epicdomains/ssodomains", get(ssodomains))
        // ---- fortnite api ----
        .route("/fortnite/api/game/v2/enabled_features", get(enabled_features))
        .route("/fortnite/api/game/v2/world/info", get(world_info))
        .route("/fortnite/api/game/v2/privacy/account/:id", get(privacy))
        .route("/fortnite/api/game/v2/grant_access/:id", post(grant_access))
        .route("/fortnite/api/game/v2/tryPlayOnPlatform/account/:id", post(try_play))
        .route("/fortnite/api/game/v2/profile/:account/client/:operation", post(mcp))
        .route("/fortnite/api/storefront/v2/catalog", get(storefront))
        .route("/fortnite/api/storefront/v2/keychain", get(keychain))
        .route("/fortnite/api/calendar/v1/timeline", get(timeline))
        .route("/fortnite/api/cloudstorage/system", get(cloudstorage_system))
        .route("/fortnite/api/cloudstorage/system/:file", get(cloudstorage_file))
        .route("/fortnite/api/cloudstorage/user/:account", get(cloudstorage_user))
        .route("/fortnite/api/cloudstorage/user/:account/:file", get(cloudstorage_user_file))
        .route("/fortnite/api/matchmaking/session/:id", get(get_session))
        .route("/fortnite/api/matchmaking/session/:id/join", post(join_session))
        .route("/fortnite/api/matchmaking/session/matchMakingRequest", post(matchmaking_request))
        // ---- friends ----
        .route("/friends/api/v1/:id/friends", get(friends_list))
        .route("/friends/api/v1/:id/settings", get(friends_settings))
        .route("/friends/api/v1/:id/block-list", get(block_list))
        // ---- content ----
        .route("/content/api/pages/fortnite-game", get(contentpages))
        .route("/content/api/pages/fortnite-game/:rest", get(contentpages_sub))
        // ---- misc passthrough ----
        .route("/lightswitch/api/service/bulk/status", get(lightswitch))
        .route("/datarouter/api/v1/public/data", post(datarouter))
        .route("/eulatracking/api/shared/agreements/fn", get(eula))
        .route("/socialban/api/public/v1/:id", get(socialban))
        .fallback(fallback);
    app.with_state(state)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn parse_form(body: &Bytes) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let text = String::from_utf8_lossy(body);
    for pair in text.split('&') {
        let mut kv = pair.splitn(2, '=');
        let k = urldecode(kv.next().unwrap_or(""));
        let v = urldecode(kv.next().unwrap_or(""));
        if !k.is_empty() {
            out.insert(k, v);
        }
    }
    out
}

fn urldecode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(b as char);
                    i += 3;
                } else {
                    out.push(bytes[i] as char);
                    i += 1;
                }
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

fn jresp(code: StatusCode, v: Value) -> Response {
    (code, Json(v)).into_response()
}

fn ok_json(v: Value) -> Response {
    jresp(StatusCode::OK, v)
}

// ---------------------------------------------------------------------------
// Owen-specific endpoints
// ---------------------------------------------------------------------------

async fn index(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.config.read().unwrap().clone();
    let uptime = state.started.elapsed().as_secs();
    let (accounts, registered) = {
        let accounts = state.accounts.read().unwrap();
        (
            accounts.len(),
            accounts.values().filter(|a| !a.email.is_empty()).count(),
        )
    };
    ok_json(json!({
        "server": "owen-server",
        "version": env!("CARGO_PKG_VERSION"),
        "backend_url": cfg.backend_url,
        "uptime_seconds": uptime,
        "login_url": cfg.login_url(),
        "partyhub_graphql": cfg.partyhub_graphql_url(),
        "accounts": accounts,
        "registered": registered,
        "seasons": state.battlepass_seasons.read().unwrap().len(),
        "partyhub": state.partyhub_dir.is_some(),
        "manifests": state.manifests_count(),
        "xmpp_port": cfg.xmpp_port,
    }))
}

async fn login_page(Query(q): Query<HashMap<String, String>>) -> Html<&'static str> {
    let _ = q;
    Html(LOGIN_PAGE)
}

// ---------------------------------------------------------------------------
// launcher auth API
// ---------------------------------------------------------------------------

/// Register a launcher account (email + username + password) and return an
/// auto-login session.
async fn launcher_register(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    state.log_request("POST /api/launcher/register".into());
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return jresp(
                StatusCode::BAD_REQUEST,
                json!({"errorCode": "errors.com.owen.account.invalid_payload",
                       "message": "Expected a JSON body {email, username, displayName?, password}"}),
            )
        }
    };
    let email = payload.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let username = payload.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let display_name = payload
        .get("displayName")
        .or_else(|| payload.get("display_name"))
        .and_then(|v| v.as_str())
        .unwrap_or(username);
    let password = payload.get("password").and_then(|v| v.as_str()).unwrap_or("");

    match state.register(email, username, display_name, password) {
        Ok(acc) => {
            let session = state.create_session(&acc.account_id);
            ok_json(json!({
                "accountId": acc.account_id,
                "username": acc.username,
                "displayName": acc.display_name,
                "email": acc.email,
                "created": acc.created,
                "access_token": session.access_token,
                "refresh_token": session.refresh_token,
                "expires_in": session.expires_in,
            }))
        }
        Err(e) => jresp(
            StatusCode::BAD_REQUEST,
            json!({"errorCode": e.error_code(), "message": e.message()}),
        ),
    }
}

/// Launcher account summary (Bearer).
async fn launcher_me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match state.auth_account(&headers) {
        Some(acc) => ok_json(account_summary(&state, &acc)),
        None => jresp(StatusCode::UNAUTHORIZED, json!({"error": "invalid_token"})),
    }
}

fn account_summary(state: &AppState, acc: &Account) -> Value {
    json!({
        "accountId": acc.account_id,
        "username": acc.username,
        "displayName": acc.display_name,
        "email": acc.email,
        "created": acc.created,
        "level": acc.level,
        "vbucks": acc.vbucks,
        "battlePassPurchased": acc.battle_pass_purchased,
        "banned": acc.banned,
        "itemsOwned": acc.items.len(),
        "selectedSeason": acc.selected_season.unwrap_or(state.current_season()),
    })
}

/// Select the active battle pass season for the bearer account.
async fn launcher_me_season(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(account_id) = state.auth_account(&headers).map(|a| a.account_id) else {
        return jresp(StatusCode::UNAUTHORIZED, json!({"error": "invalid_token"}));
    };
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return jresp(
                StatusCode::BAD_REQUEST,
                json!({"errorCode": "errors.com.owen.season.invalid",
                       "message": "Expected a JSON body {season: N}"}),
            )
        }
    };
    let season = payload
        .get("season")
        .and_then(|v| v.as_u64())
        .filter(|n| (1..=MAX_SEASON as u64).contains(n));
    let Some(season) = season else {
        return jresp(
            StatusCode::BAD_REQUEST,
            json!({"errorCode": "errors.com.owen.season.invalid",
                   "message": format!("Season must be a number between 1 and {MAX_SEASON}")}),
        );
    };
    match state.set_selected_season(&account_id, season as u32) {
        Some(acc) => ok_json(account_summary(&state, &acc)),
        None => jresp(StatusCode::UNAUTHORIZED, json!({"error": "invalid_token"})),
    }
}

/// Kill the bearer session.
async fn launcher_logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let token = auth
            .strip_prefix("Bearer ")
            .or(auth.strip_prefix("bearer "))
            .unwrap_or(auth);
        state.end_session(token);
    }
    ok_json(json!({}))
}

// ---------------------------------------------------------------------------
// launcher seasons / manifests
// ---------------------------------------------------------------------------

fn season_item(meta: SeasonMeta, bp: Option<&Value>, current: u32) -> Value {
    let (paid, free) = bp.map(seasons::battlepass_tiers).unwrap_or((0, 0));
    let now = today();
    let active = (meta.start <= now.as_str() && now.as_str() <= meta.end) || meta.season == current;
    json!({
        "season": meta.season,
        "chapter": meta.chapter,
        "chapterSeason": meta.chapter_season,
        "name": meta.name,
        "version": format!("v{}.0", meta.season),
        "startDate": meta.start,
        "endDate": meta.end,
        "image": seasons::season_image(meta.season),
        "paidTiers": paid,
        "freeTiers": free,
        "active": active,
    })
}

/// The season catalog for the launcher: every known season with battle
/// pass tier counts, plus the current default season.
async fn launcher_seasons(State(state): State<Arc<AppState>>) -> Response {
    let current = state.current_season();
    let battlepasses = state.battlepass_seasons.read().unwrap();
    let items: Vec<Value> = seasons::all_seasons()
        .into_iter()
        .map(|meta| {
            let bp = battlepasses.get(&format!("Season{}", meta.season));
            season_item(meta, bp, current)
        })
        .collect();
    ok_json(json!({"current": current, "seasons": items}))
}

/// One season with its full battle pass JSON.
async fn launcher_season(State(state): State<Arc<AppState>>, Path(n): Path<String>) -> Response {
    let Ok(n) = n.trim().parse::<u32>() else {
        return jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.owen.season.not_found"}));
    };
    let Some(meta) = seasons::season_info(n) else {
        return jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.owen.season.not_found"}));
    };
    let current = state.current_season();
    let battlepass = state.battlepass_for(n);
    let meta_json = season_item(meta, battlepass.as_ref(), current);
    ok_json(json!({
        "meta": meta_json,
        "battlepass": battlepass.unwrap_or(Value::Null),
    }))
}

/// Serve the launcher manifests catalog (data_dir/manifests.json).
async fn launcher_manifests(State(state): State<Arc<AppState>>) -> Response {
    match state
        .data_file("manifests.json")
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
    {
        Some(v) => ok_json(v),
        None => ok_json(json!([])),
    }
}

/// Auto manifest redirecter: the launcher asset API the game asks after
/// reading `assets/Cloud/cloudcontent.json`.
async fn launcher_assets(
    State(state): State<Arc<AppState>>,
    Path((platform, label, app, build)): Path<(String, String, String, String)>,
) -> Response {
    state.log_request(format!("GET /launcher/api/public/assets/{platform}/{label}/{app}/{build}"));
    let manifests = state.manifests.read().unwrap();
    let entry = manifests
        .find_release(&build)
        .cloned()
        .or_else(|| {
            // Fallback: treat the build token as a live manifest path.
            let version = build
                .strip_prefix("++Fortnite+Release-")
                .and_then(|s| s.split("-CL-").next())
                .unwrap_or(&build);
            Some(owen_core::manifest::ManifestEntry {
                version: version.to_string(),
                release_string: build.clone(),
                manifest_url: String::new(),
                source: owen_core::manifest::ManifestSource::Catalog,
                cloud_content: None,
            })
        });
    match entry {
        Some(e) => {
            let resp = owen_core::manifest::launcher_asset_response(&e, &label);
            ok_json(serde_json::to_value(resp).unwrap_or_default())
        }
        None => jresp(
            StatusCode::NOT_FOUND,
            json!({"errorCode": "errors.com.owen.manifest.not_found"}),
        ),
    }
}

/// Serve a manifest file from the data dir (or redirect to the live CDN).
async fn manifest_file(State(state): State<Arc<AppState>>, Path(rest): Path<String>) -> Response {
    let manifests = state.manifests.read().unwrap();
    if let Some(entry) = manifests.entries.iter().find(|e| !e.manifest_url.is_empty()) {
        return Redirect::temporary(&entry.manifest_url).into_response();
    }
    if let Some(data) = state.data_file(&format!("responses/CloudDir/{rest}")) {
        return (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            data,
        )
            .into_response();
    }
    jresp(StatusCode::NOT_FOUND, json!({"error": "manifest not found"}))
}

// ---------------------------------------------------------------------------
// account / oauth
// ---------------------------------------------------------------------------

async fn oauth_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let form = parse_form(&body);
    let grant = form.get("grant_type").cloned().unwrap_or_default();
    let account = match grant.as_str() {
        "password" => {
            let login = form
                .get("username")
                .cloned()
                .or_else(|| form.get("email").cloned())
                .unwrap_or_else(|| "OwenPlayer".into());
            let password = form.get("password").cloned().unwrap_or_default();
            match state.verify_login(&login, &password) {
                Some(acc) => acc,
                // No account matches the login at all → legacy quick-account
                // creation so the in-game flow keeps working.
                None if !state.login_exists(&login) => state.account_by_name(&login),
                // An account exists (registered, password-protected) but the
                // password is wrong.
                None => {
                    return jresp(
                        StatusCode::BAD_REQUEST,
                        json!({"error": "invalid_grant", "error_description": "Invalid username or password"}),
                    )
                }
            }
        }
        "device_auth" => {
            // form fields: device_id / account_id / secret
            let account_id = form.get("account_id").cloned().unwrap_or_default();
            match state.account_by_id(&account_id) {
                Some(acc) => acc,
                None => {
                    return jresp(
                        StatusCode::BAD_REQUEST,
                        json!({"error": "invalid_grant", "error_description": "Unknown device credentials"}),
                    )
                }
            }
        }
        "authorization_code" => {
            let code = form.get("code").cloned().unwrap_or_default();
            match state.exchange_account(&code) {
                Some(id) => state.account_by_id(&id).unwrap_or_else(|| state.account_by_name("OwenPlayer")),
                None => {
                    return jresp(
                        StatusCode::BAD_REQUEST,
                        json!({"error": "invalid_grant", "error_description": "Unknown exchange code"}),
                    )
                }
            }
        }
        "refresh_token" => {
            let refresh = form.get("refresh_token").cloned().unwrap_or_default();
            let sessions = state.sessions.read().unwrap();
            sessions
                .values()
                .find(|s| s.refresh_token == refresh)
                .and_then(|s| state.account_by_id(&s.account_id))
                .unwrap_or_else(|| state.account_by_name("OwenPlayer"))
        }
        _ => state.account_by_name("OwenPlayer"),
    };

    if account.banned {
        return jresp(
            StatusCode::FORBIDDEN,
            json!({"error": "banned", "error_description": "This account is banned on this server"}),
        );
    }

    let session: Session = state.create_session(&account.account_id);
    state.log_request(format!("POST /account/api/oauth/token grant={grant} user={}", account.display_name));
    let _ = headers;
    ok_json(json!({
        "access_token": session.access_token,
        "expires_in": session.expires_in,
        "expires_at": iso_now(),
        "token_type": "bearer",
        "refresh_token": session.refresh_token,
        "refresh_expires": 79860,
        "refresh_expires_at": iso_now(),
        "account_id": account.account_id,
        "client_id": session.client_id,
        "internal_client": true,
        "client_service": "fortnite",
        "displayName": account.display_name,
        "app": "fortnite",
        "in_app_id": account.account_id,
        "product_id": "prod-fn",
        "application": "fortnite",
    }))
}

async fn oauth_exchange(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(account) = state.auth_account(&headers).clone() else {
        return jresp(StatusCode::UNAUTHORIZED, json!({"error": "invalid_token"}));
    };
    let code = state.create_exchange_code(&account.account_id);
    state.log_request(format!("GET /account/api/oauth/exchange user={}", account.display_name));
    ok_json(json!({
        "expiresInSeconds": 299,
        "code": code,
        "creatingClientId": "owen-launcher",
    }))
}

async fn oauth_verify(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match state.auth_account(&headers) {
        Some(account) => ok_json(json!({
            "token": "valid", "organizationId": "owen",
            "accountId": account.account_id, "displayName": account.display_name,
        })),
        None => jresp(StatusCode::UNAUTHORIZED, json!({"error": "invalid_token"})),
    }
}

async fn sessions_kill(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let token = auth.trim_start_matches("Bearer ").trim_start_matches("bearer ");
        state.sessions.write().unwrap().remove(token);
    }
    ok_json(json!({}))
}

async fn account_lookup(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let names = q
        .get("displayName")
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let accounts = state.accounts.read().unwrap();
    let mut out = Vec::new();
    for name in names {
        if let Some(acc) = accounts.values().find(|a| a.display_name.eq_ignore_ascii_case(&name)) {
            out.push(json!({"id": acc.account_id, "displayName": acc.display_name}));
        }
    }
    ok_json(json!(out))
}

async fn account_info(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let account = state
        .account_by_id(&id)
        .or_else(|| q.get("displayName").map(|n| state.account_by_name(n)));
    match account {
        Some(acc) => ok_json(json!({
            "id": acc.account_id, "displayName": acc.display_name,
            "name": acc.display_name, "firstSeen": acc.created,
            "externalAuths": [], "email_hidden": true,
        })),
        None => jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.epicgames.account.not_found"})),
    }
}

async fn account_external_auths() -> Response {
    ok_json(json!([]))
}

async fn ssodomains() -> Response {
    ok_json(json!([
        "unrealengine.com", "unrealengine.com.tst", "epicgames.com", "epicgames.com.tst",
        "fortnite.com", "fortnite.com.tst", "ol.epicgames.net", "ol.epicgames.net.tst"
    ]))
}

// ---------------------------------------------------------------------------
// fortnite api
// ---------------------------------------------------------------------------

async fn enabled_features() -> Response {
    ok_json(json!([]))
}

async fn world_info() -> Response {
    ok_json(json!({"worlds": []}))
}

async fn privacy(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let _ = state.account_by_id(&id);
    ok_json(json!({"accountId": id, "isOptedIn": true, "isFriendsOnly": false}))
}

async fn grant_access(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    state.log_request(format!("POST /fortnite/api/game/v2/grant_access/{id}"));
    ok_json(json!({}))
}

async fn try_play(State(_state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let _ = id;
    ok_json(json!({}))
}

/// MCP — profile operations (Reload-Backend feature parity: Battle Pass
/// purchase, level purchase, item grants, locker equips).
async fn mcp(
    State(state): State<Arc<AppState>>,
    Path((account_id, operation)): Path<(String, String)>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let account = state
        .auth_account(&headers)
        .or_else(|| state.account_by_id(&account_id))
        .unwrap_or_else(|| state.account_by_name(&account_id))
        .clone();
    let payload: Value =
        serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));

    state.log_request(format!("MCP {operation} for {}", account.display_name));

    // ProfileId travels in the JSON body.
    let profile_id = payload
        .get("profileId")
        .and_then(|v| v.as_str())
        .unwrap_or("common_core")
        .to_string();

    let mut notifications: Vec<Value> = Vec::new();
    let mut changed = false;
    // Read commands always answer with the full profile (LawinServer v2
    // behavior) — the game expects its locker/level/battle-pass data in
    // `profileChanges` on every QueryProfile / ClientQuestLogin.
    let full_update = matches!(
        operation.as_str(),
        "QueryProfile" | "ClientQuestLogin" | "RefreshExpeditions" | "GetMcpTimeForLogin"
    );

    match operation.as_str() {
        "QueryProfile" | "ClientQuestLogin" | "RefreshExpeditions" | "GetMcpTimeForLogin" => {}
        "MarkItemSeen" | "EquipBattleRoyaleCustomization" | "SetBattleRoyaleBanner" => {
            if let Some(map) = state.accounts.write().unwrap().get_mut(&account.account_id) {
                map.locker.insert(operation.clone(), payload.clone());
            }
            changed = true;
        }
        "SetBattlePassPurchased" | "PurchaseBattlePass" => {
            if let Some(map) = state.accounts.write().unwrap().get_mut(&account.account_id) {
                map.battle_pass_purchased = true;
            }
            notifications.push(json!({"type": "battlePassPurchased", "primary": true}));
            changed = true;
        }
        "PurchaseBattlePassLevels" | "BattlePassLevelClaim" => {
            let levels = payload
                .get("purchaseCount")
                .or(payload.get("levels"))
                .and_then(|v| v.as_u64())
                .or_else(|| payload.get("levelNum").and_then(|v| v.as_u64()))
                .unwrap_or(1) as u32;
            if let Some(map) = state.accounts.write().unwrap().get_mut(&account.account_id) {
                map.level += levels;
            }
            changed = true;
        }
        "PurchaseCatalogItem" | "RefundMtxPurchase" => {
            if let Some(map) = state.accounts.write().unwrap().get_mut(&account.account_id) {
                map.vbucks -= 100;
            }
            changed = true;
        }
        _ => {}
    }

    if changed {
        if let Some(map) = state.accounts.write().unwrap().get_mut(&account.account_id) {
            let rvn = map.rvn.entry(profile_id.clone()).or_insert(1);
            *rvn += 1;
        }
        state.persist_accounts();
    }

    // Fresh snapshot so mutations (level, purchase...) are reflected.
    let account = state
        .account_by_id(&account.account_id)
        .unwrap_or(account);

    let mut profile = state.profile_for(&account, &profile_id);

    // Athena enrichment: when the resolved season has a battle pass file,
    // overlay book level and the flattened battle pass items.
    if profile_id == "athena" {
        let current = state.current_season();
        let season = seasons::resolve_season(
            headers.get("x-season").and_then(|v| v.to_str().ok()),
            query.get("season").map(|s| s.as_str()),
            account.selected_season,
            current,
        );
        if let Some(battlepass) = state.battlepass_for(season) {
            seasons::enrich_athena(&mut profile, &account, &battlepass);
        }
    }

    let rvn = account.rvn.get(&profile_id).copied().unwrap_or(1);

    ok_json(json!({
        "profileRevision": rvn,
        "profileId": profile_id,
        "profileChangesBaseRevision": rvn.saturating_sub(1),
        "profileChanges": if changed || full_update {
            serde_json::Value::Array(vec![json!({
                "changeType": "fullProfileUpdate",
                "profile": profile,
            })])
        } else {
            serde_json::Value::Array(vec![])
        },
        "profileCommandRevision": rvn,
        "serverTime": iso_now(),
        "responseVersion": 1,
        "notifications": notifications,
    }))
}

async fn storefront(State(state): State<Arc<AppState>>) -> Response {
    if let Some(catalog) = state
        .data_file("responses/catalog.json")
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
    {
        return ok_json(catalog);
    }
    ok_json(json!({"refreshIntervalHrs": 24, "dailyPurchaseHrs": 24, "expiration": iso_now(), "storefronts": []}))
}

async fn keychain(State(state): State<Arc<AppState>>) -> Response {
    match state
        .data_file("responses/keychain.json")
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
    {
        Some(v) => ok_json(v),
        None => ok_json(json!([])),
    }
}

/// Fortnite calendar/timeline. The season is resolved from the `X-Season`
/// header, the `?season=` query or the bearer account's selection,
/// defaulting to the highest season with a battle pass file.
async fn timeline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let current = state.current_season();
    let selected = state.auth_account(&headers).and_then(|a| a.selected_season);
    let season = seasons::resolve_season(
        headers.get("x-season").and_then(|v| v.to_str().ok()),
        query.get("season").map(|s| s.as_str()),
        selected,
        current,
    );
    let meta = seasons::season_info_or_default(season);
    ok_json(json!({
        "channels": {},
        "clientParameters": {"canvas": "en"},
        "currentTime": iso_now(),
        "events": [],
        "seasons": [{
            "seasonNumber": meta.season,
            "beginTime": format!("{}T00:00:00.000Z", meta.start),
            "endTime": format!("{}T00:00:00.000Z", meta.end),
            "profileChanges": [],
        }],
        "seasonStamp": meta.season.to_string(),
        "stories": {},
        "synergy": {},
        "activeEvents": [],
    }))
}

async fn cloudstorage_system(State(state): State<Arc<AppState>>) -> Response {
    let mut files = Vec::new();
    let dir = state.data_dir.join("CloudStorage");
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".ini") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                files.push(json!({
                    "uniqueFilename": name, "filename": name,
                    "hash": format!("{:x}", crc32(entry.path())),
                    "hashType": "CRC32",
                    "size": meta.len(),
                    "uploaded": iso_now(),
                    "storageType": "S3",
                    "doNotCache": false,
                }));
            }
        }
    }
    ok_json(json!(files))
}

fn crc32(path: std::path::PathBuf) -> u32 {
    let data = std::fs::read(path).unwrap_or_default();
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in &data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 };
        }
    }
    !crc
}

async fn cloudstorage_file(State(state): State<Arc<AppState>>, Path(file): Path<String>) -> Response {
    match state.data_file(&format!("CloudStorage/{file}")) {
        Some(data) => (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            data,
        )
            .into_response(),
        None => jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.owen.not_found"})),
    }
}

async fn cloudstorage_user(State(state): State<Arc<AppState>>, Path(account): Path<String>) -> Response {
    let _ = state.account_by_id(&account);
    ok_json(json!([]))
}

async fn cloudstorage_user_file(
    State(_state): State<Arc<AppState>>,
    Path((account, file)): Path<(String, String)>,
) -> Response {
    let _ = (account, file);
    jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.owen.not_found"}))
}

// ---------------------------------------------------------------------------
// matchmaking
// ---------------------------------------------------------------------------

async fn get_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    state.log_request(format!("GET matchmaking/session/{id}"));
    jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.epicgames.matchmaking.session.not_found"}))
}

async fn join_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    state.log_request(format!("POST matchmaking/session/{id}/join"));
    jresp(StatusCode::NOT_FOUND, json!({"errorCode": "errors.com.epicgames.matchmaking.session.not_found"}))
}

async fn matchmaking_request(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    let _ = body;
    state.log_request("POST matchmaking/matchMakingRequest".into());
    ok_json(json!([]))
}

// ---------------------------------------------------------------------------
// friends
// ---------------------------------------------------------------------------

async fn friends_list(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let _ = state.account_by_id(&id);
    ok_json(json!([]))
}

async fn friends_settings() -> Response {
    ok_json(json!([]))
}

async fn block_list() -> Response {
    ok_json(json!([]))
}

// ---------------------------------------------------------------------------
// content pages
// ---------------------------------------------------------------------------

async fn contentpages(State(state): State<Arc<AppState>>) -> Response {
    if let Some(v) = state
        .data_file("responses/contentpages.json")
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
    {
        return ok_json(v);
    }
    ok_json(json!({}))
}

async fn contentpages_sub(State(state): State<Arc<AppState>>, Path(rest): Path<String>) -> Response {
    let file = match rest.as_str() {
        "motd" => Some("responses/Athena/motd.json"),
        "radio-stations" => Some("responses/Athena/sparkTracks.json"),
        "seasonpasses" => Some("responses/Athena/SeasonData.json"),
        _ => None,
    };
    if let Some(f) = file {
        if let Some(v) = state.data_file(f).and_then(|d| serde_json::from_slice::<Value>(&d).ok()) {
            return ok_json(v);
        }
    }
    ok_json(json!({}))
}

// ---------------------------------------------------------------------------
// misc
// ---------------------------------------------------------------------------

async fn lightswitch() -> Response {
    ok_json(json!([{
        "serviceInstanceId": "fortnite", "status": "UP",
        "message": "Owen backend", "maintenanceUri": null,
    }]))
}

async fn datarouter() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

async fn eula() -> Response {
    ok_json(json!({"id": "fn", "locale": "en", "version": 1, "accepted": true, "location": ""}))
}

async fn socialban(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let _ = state.account_by_id(&id);
    ok_json(json!({"bans": [], "warnings": []}))
}

async fn fallback(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    // Serve public/ assets (motd images, video segments...) like LawinServer.
    let clean = path.trim_start_matches('/');
    let candidate = state.data_dir.join("public").join(clean);
    if method == axum::http::Method::GET && candidate.is_file() {
        if let Ok(data) = std::fs::read(&candidate) {
            let mime = mime_for(clean);
            return (StatusCode::OK, [("content-type", mime)], data).into_response();
        }
    }
    state.log_request(format!("{method} {path} (no route)"));
    jresp(
        StatusCode::NOT_FOUND,
        json!({"errorCode": "errors.com.owen.no_route", "message": format!("no route for {method} {path}")}),
    )
}

fn mime_for(path: &str) -> &'static str {
    if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".mp4") {
        "video/mp4"
    } else if path.ends_with(".m4s") {
        "video/iso.segment"
    } else if path.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

// ---------------------------------------------------------------------------
// admin API (the web control panel — replaces the Discord bot)
// ---------------------------------------------------------------------------

pub fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/admin/status", get(admin_status))
        .route("/api/admin/config", get(admin_get_config).put(admin_put_config))
        .route("/api/admin/rules", get(admin_get_rules).put(admin_put_rules))
        .route("/api/admin/players", get(admin_players).post(admin_create_player))
        .route("/api/admin/players/:id", get(admin_player).put(admin_update_player).delete(admin_delete_player))
        .route("/api/admin/scan", post(admin_scan))
        .route("/api/admin/manifests", get(admin_manifests).post(admin_manifests_add))
        .route("/api/admin/battlepass", get(admin_battlepass))
        .route("/api/admin/battlepass/pak", post(admin_battlepass_pak))
        .route("/api/admin/log", get(admin_log))
        .route("/api/admin/versions", get(admin_versions))
}

async fn admin_status(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.config.read().unwrap().clone();
    let accounts = state.accounts.read().unwrap();
    ok_json(json!({
        "server": "owen-server",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.started.elapsed().as_secs(),
        "requests_served": state.request_counter.load(std::sync::atomic::Ordering::Relaxed),
        "backend_url": cfg.backend_url,
        "login_url": cfg.login_url(),
        "partyhub_graphql_url": cfg.partyhub_graphql_url(),
        "http_port": cfg.http_port,
        "tls_port": cfg.tls_port,
        "xmpp_port": cfg.xmpp_port,
        "partyhub_redirect": cfg.partyhub_redirect,
        "webview_redirect": cfg.webview_redirect,
        "manifest_redirect": cfg.manifest_redirect,
        "accounts_total": accounts.len(),
        "banned": accounts.values().filter(|a| a.banned).count(),
        "battle_pass_seasons": state.battlepass_seasons.read().unwrap().len(),
        "pak_reports": state.pak_reports.read().unwrap().len(),
        "scan_reports": state.scan_reports.read().unwrap().len(),
        "manifest_entries": state.manifests.read().unwrap().entries.len(),
        "data_dir": state.data_dir.display().to_string(),
    }))
}

async fn admin_get_config(State(state): State<Arc<AppState>>) -> Response {
    ok_json(serde_json::to_value(&*state.config.read().unwrap()).unwrap_or_default())
}

async fn admin_put_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Response {
    let mut cfg = state.config.write().unwrap();
    if let Some(u) = body.get("backend_url").and_then(|v| v.as_str()) {
        if let Err(e) = cfg.set_backend_url(u) {
            return jresp(StatusCode::BAD_REQUEST, json!({"error": e}));
        }
    }
    for key in ["partyhub_redirect", "webview_redirect", "manifest_redirect", "display_name"] {
        if let Some(v) = body.get(key) {
            match (key, v) {
                ("display_name", Value::String(s)) => cfg.display_name = s.clone(),
                (_, Value::Bool(b)) => match key {
                    "partyhub_redirect" => cfg.partyhub_redirect = *b,
                    "webview_redirect" => cfg.webview_redirect = *b,
                    "manifest_redirect" => cfg.manifest_redirect = *b,
                    _ => {}
                },
                _ => {}
            }
        }
    }
    if let Some(p) = body.get("http_port").and_then(|v| v.as_u64()) {
        cfg.http_port = p as u16;
    }
    *state.redirect.write().unwrap() = cfg.redirect.clone();
    ok_json(serde_json::to_value(&*cfg).unwrap_or_default())
}

async fn admin_get_rules(State(state): State<Arc<AppState>>) -> Response {
    ok_json(serde_json::to_value(&*state.redirect.read().unwrap()).unwrap_or_default())
}

async fn admin_put_rules(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Response {
    if let Ok(rules) = serde_json::from_value::<owen_core::urlrule::RedirectRules>(body) {
        *state.redirect.write().unwrap() = rules.clone();
        state.config.write().unwrap().redirect = rules.clone();
        ok_json(serde_json::to_value(&rules).unwrap_or_default())
    } else {
        jresp(StatusCode::BAD_REQUEST, json!({"error": "invalid rules payload"}))
    }
}

async fn admin_players(State(state): State<Arc<AppState>>) -> Response {
    let accounts = state.accounts.read().unwrap();
    let mut list: Vec<Value> = accounts.values().cloned().map(|a| serde_json::to_value(&a).unwrap_or_default()).collect();
    list.sort_by(|a, b| {
        a.get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("display_name").and_then(|v| v.as_str()).unwrap_or(""))
    });
    ok_json(json!(list))
}

async fn admin_create_player(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Response {
    let Some(name) = body.get("display_name").and_then(|v| v.as_str()) else {
        return jresp(StatusCode::BAD_REQUEST, json!({"error": "display_name required"}));
    };
    let acc = state.account_by_name(name);
    ok_json(serde_json::to_value(&acc).unwrap_or_default())
}

async fn admin_player(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    match state.account_by_id(&id) {
        Some(acc) => ok_json(serde_json::to_value(&acc).unwrap_or_default()),
        None => jresp(StatusCode::NOT_FOUND, json!({"error": "not found"})),
    }
}

async fn admin_update_player(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let (snapshot, old_name) = {
        let mut accounts = state.accounts.write().unwrap();
        let Some(acc) = accounts.get_mut(&id) else {
            return jresp(StatusCode::NOT_FOUND, json!({"error": "not found"}));
        };
        let old_name = acc.display_name.clone();
        if let Some(v) = body.get("banned").and_then(|v| v.as_bool()) {
            acc.banned = v;
        }
        if let Some(v) = body.get("whitelisted").and_then(|v| v.as_bool()) {
            acc.whitelisted = v;
        }
        if let Some(v) = body.get("vbucks").and_then(|v| v.as_i64()) {
            acc.vbucks = v;
        }
        if let Some(v) = body.get("level").and_then(|v| v.as_u64()) {
            acc.level = v as u32;
        }
        if let Some(v) = body.get("battle_pass_purchased").and_then(|v| v.as_bool()) {
            acc.battle_pass_purchased = v;
        }
        if let Some(v) = body.get("sac_code").and_then(|v| v.as_str()) {
            acc.sac_code = v.to_string();
        }
        if let Some(v) = body.get("display_name").and_then(|v| v.as_str()) {
            acc.display_name = v.to_string();
        }
        (acc.clone(), old_name)
    };
    // keep the display-name index in sync on rename
    if snapshot.display_name != old_name {
        let mut by_name = state.by_name.write().unwrap();
        by_name.remove(&old_name);
        by_name.insert(snapshot.display_name.clone(), snapshot.account_id.clone());
    }
    state.persist_accounts();
    ok_json(serde_json::to_value(&snapshot).unwrap_or_default())
}

async fn admin_delete_player(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let removed = state.accounts.write().unwrap().remove(&id);
    if let Some(acc) = removed {
        state.by_name.write().unwrap().remove(&acc.display_name);
        if !acc.username.is_empty() {
            state
                .by_username
                .write()
                .unwrap()
                .remove(&acc.username.to_lowercase());
        }
        if !acc.email.is_empty() {
            state.by_email.write().unwrap().remove(&acc.email);
        }
        state.persist_accounts();
        ok_json(json!({"deleted": acc.account_id}))
    } else {
        jresp(StatusCode::NOT_FOUND, json!({"error": "not found"}))
    }
}

async fn admin_scan(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field
            .file_name()
            .map(|s| s.to_string())
            .or_else(|| field.name().map(|s| s.to_string()))
            .unwrap_or_else(|| "lib.so".into());
        let data = field.bytes().await.unwrap_or_default();
        if data.is_empty() {
            continue;
        }
        let report = owen_core::scanner::scan_image_data(&data);
        let record = crate::state::ScanRecord {
            ts: iso_now(),
            size: data.len() as u64,
            file_name: name,
            report: report.clone(),
        };
        state.scan_reports.write().unwrap().insert(0, record);
        state.scan_reports.write().unwrap().truncate(50);
        return ok_json(serde_json::to_value(&report).unwrap_or_default());
    }
    jresp(StatusCode::BAD_REQUEST, json!({"error": "no file uploaded"}))
}

async fn admin_manifests(State(state): State<Arc<AppState>>) -> Response {
    let manifests = state.manifests.read().unwrap();
    ok_json(serde_json::to_value(&*manifests).unwrap_or_default())
}

async fn admin_manifests_add(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let text = String::from_utf8_lossy(&body).to_string();
    match owen_core::manifest::CloudContent::from_json(&body) {
        Ok(cc) => {
            let mut manifests = state.manifests.write().unwrap();
            manifests.add_cloud_content(cc);
            ok_json(serde_json::to_value(&*manifests).unwrap_or_default())
        }
        Err(_) => {
            // Try as a bare manifest URL or path text.
            let trimmed = text.trim();
            if trimmed.contains(".manifest") {
                let version = trimmed
                    .split('/')
                    .find(|p| p.contains('.'))
                    .unwrap_or("custom")
                    .to_string();
                let mut manifests = state.manifests.write().unwrap();
                let entry = owen_core::manifest::ManifestEntry {
                    version,
                    release_string: trimmed.to_string(),
                    manifest_url: trimmed.to_string(),
                    source: owen_core::manifest::ManifestSource::Manual,
                    cloud_content: None,
                };
                owen_core::manifest::ManifestCatalog { entries: vec![entry.clone()] }
                    .entries
                    .into_iter()
                    .for_each(|e| {
                        if let Some(existing) = manifests.entries.iter_mut().find(|x| x.release_string == e.release_string) {
                            *existing = e;
                        } else {
                            manifests.entries.push(e);
                        }
                    });
                ok_json(serde_json::to_value(&*manifests).unwrap_or_default())
            } else {
                jresp(StatusCode::BAD_REQUEST, json!({"error": "expected cloudcontent.json or a manifest URL", "received": trimmed.chars().take(120).collect::<String>()}))
            }
        }
    }
}

async fn admin_battlepass(State(state): State<Arc<AppState>>) -> Response {
    let seasons = state.battlepass_seasons.read().unwrap();
    let paks = state.pak_reports.read().unwrap();
    ok_json(json!({
        "seasons": seasons.keys().cloned().collect::<Vec<_>>(),
        "pak_reports": *paks,
    }))
}

async fn admin_battlepass_pak(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let mut version_major: u32 = 12;
    let mut version_minor: u32 = 50;
    let mut aes_key: Option<[u8; 32]> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().map(|s| s.to_string());
        let fname = field
            .file_name()
            .map(|s| s.to_string())
            .or(field_name.clone())
            .unwrap_or_else(|| "pak.pak".into());
        let data = field.bytes().await.unwrap_or_default();
        if data.is_empty() {
            continue;
        }
        if field_name.as_deref() == Some("version") {
            let text = String::from_utf8_lossy(&data).trim().to_string();
            if let Some(v) = owen_core::version::detect_from_bytes(
                format!("++Fortnite+Release-{text}").as_bytes(),
            ) {
                version_major = v.major;
                version_minor = v.minor;
            }
            continue;
        }
        if field_name.as_deref() == Some("key") {
            let text = String::from_utf8_lossy(&data).trim().to_string();
            aes_key = parse_hex_key(&text);
            continue;
        }
        // Parse the pak index from the uploaded bytes.
        let mut cursor = std::io::Cursor::new(&data);
        match owen_core::pak::read_pak_index(&mut cursor, aes_key.as_ref()) {
            Ok(index) => {
                let version = owen_core::version::FortniteVersion {
                    major: version_major,
                    minor: version_minor,
                    cl: 0,
                    release_string: format!("++Fortnite+Release-{version_major}.{version_minor:02}"),
                };
                let report = owen_core::battlepass::extract(&index, &version);
                state.pak_reports.write().unwrap().insert(0, report.clone());
                state.pak_reports.write().unwrap().truncate(20);
                return ok_json(serde_json::to_value(&report).unwrap_or_default());
            }
            Err(e) => {
                return jresp(StatusCode::UNPROCESSABLE_ENTITY, json!({
                    "error": e.to_string(), "file": fname,
                }))
            }
        }
    }
    jresp(StatusCode::BAD_REQUEST, json!({"error": "no pak uploaded"}))
}

fn parse_hex_key(s: &str) -> Option<[u8; 32]> {
    let hex: String = s
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

async fn admin_log(State(state): State<Arc<AppState>>) -> Response {
    let log = state.log.read().unwrap();
    ok_json(json!({ "entries": *log }))
}

async fn admin_versions() -> Response {
    ok_json(owen_core::profiles::version_catalog())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_without_pattern_conflicts() {
        // Fails (panics) if any route pattern is invalid — e.g. a
        // catch-all in the middle of a path, which axum 0.7 rejects.
        let state = Arc::new(crate::state::AppState::load(
            std::env::temp_dir().join("owen-router-smoke"),
            owen_core::config::OwenConfig::default(),
        ));
        let _router = root_router(state);
    }

    #[test]
    fn season_item_shape_and_counts() {
        let meta = seasons::season_info(12).expect("season 12");
        let bp = json!({
            "paidRewards": Value::Array(vec![json!({}); 100]),
            "freeRewards": Value::Array(vec![json!({}); 90])
        });
        let item = season_item(meta, Some(&bp), 12);
        assert_eq!(item["season"], 12);
        assert_eq!(item["chapter"], 2);
        assert_eq!(item["chapterSeason"], 2);
        assert_eq!(item["name"], "Top Secret");
        assert_eq!(item["version"], "v12.0");
        assert_eq!(item["startDate"], "2020-02-20");
        assert_eq!(item["endDate"], "2020-06-16");
        assert_eq!(item["image"], "/owen/seasons/season_12_folder.png");
        assert_eq!(item["paidTiers"], 100);
        assert_eq!(item["freeTiers"], 90);
        assert_eq!(item["active"], true, "season == current is active");

        // no battle pass file → zero counts, image rules still apply
        let item = season_item(meta, None, 42);
        assert_eq!(item["paidTiers"], 0);
        assert_eq!(item["freeTiers"], 0);
        assert_eq!(item["active"], false, "past window and not current");

        // seasons outside 5..=33 have no image
        let meta = seasons::season_info(2).expect("season 2");
        let item = season_item(meta, None, 42);
        assert!(item["image"].is_null());
        let meta = seasons::season_info(34).expect("season 34");
        let item = season_item(meta, None, 42);
        assert!(item["image"].is_null());
    }

    #[test]
    fn account_summary_shape() {
        let (state, _dir) = crate::state::testing::test_state();
        let mut acc = state.account_by_name("SummaryUser");
        state
            .update_account(&acc.account_id, |a| {
                a.level = 33;
                a.vbucks = 1234;
                a.items.push("AthenaCharacter:cid_test".into());
            })
            .expect("update");
        acc = state.account_by_id(&acc.account_id).expect("fresh");
        let summary = account_summary(&state, &acc);
        assert_eq!(summary["accountId"], acc.account_id.as_str());
        assert_eq!(summary["displayName"], "SummaryUser");
        assert_eq!(summary["level"], 33);
        assert_eq!(summary["vbucks"], 1234);
        assert_eq!(summary["battlePassPurchased"], false);
        assert_eq!(summary["banned"], false);
        assert_eq!(summary["itemsOwned"], 1);
        assert_eq!(summary["selectedSeason"], 1, "defaults to current season");
    }

    #[test]
    fn mcp_athena_enrichment_via_state() {
        // End-to-end shape of the MCP athena path: profile_for +
        // enrich_athena with a small inline battle pass.
        let (state, _dir) = crate::state::testing::test_state();
        let mut acc = state.account_by_name("McpUser");
        state
            .update_account(&acc.account_id, |a| {
                a.level = 3;
                a.battle_pass_purchased = true;
            })
            .expect("update");
        acc = state.account_by_id(&acc.account_id).expect("fresh");

        state.battlepass_seasons.write().unwrap().insert(
            "Season12".into(),
            json!({
                "paidRewards": [
                    { "AthenaCharacter:cid_1": 1 },
                    {},
                    { "AthenaBackpack:bid_1": 1 }
                ],
                "freeRewards": [{ "AthenaDance:spid_1": 1 }]
            }),
        );

        let mut profile = state.profile_for(&acc, "athena");
        let bp = state.battlepass_for(12).expect("battle pass");
        seasons::enrich_athena(&mut profile, &acc, &bp);

        // level 3 → all paid tiers (tier 1 empty) + free tier 0
        let items = profile["items"].as_array().expect("items");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["templateId"], "AthenaCharacter:cid_1");
        assert_eq!(items[1]["templateId"], "AthenaBackpack:bid_1");
        assert_eq!(items[2]["templateId"], "AthenaDance:spid_1");
        assert_eq!(profile["stats"]["attributes"]["book_level"], 3);
        assert_eq!(profile["stats"]["attributes"]["battlepass_purchased"], true);
        // synthesized athena template defaults
        assert_eq!(profile["wipeNumber"], 7);
        assert_eq!(profile["version"], 1);
    }
}
