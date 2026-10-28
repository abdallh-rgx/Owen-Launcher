//! The route table — the same paths the game and the original C server
//! speak, evaluated top-to-bottom with wildcard support.

use crate::{serve_file, Context, Request, Response};

type Handler = fn(&Context, &Request, &[String]) -> Response;

struct Route {
    method: &'static str,
    pattern: &'static str,
    handler: Handler,
}

fn route(ctx: &Context, req: &Request) -> Response {
    // Fixed table (order matters; wildcards last).
    let routes: &[Route] = &[
        Route { method: "POST", pattern: "/", handler: root_post },
        Route { method: "GET", pattern: "/login", handler: login },
        Route { method: "GET", pattern: "/eulatracking/api/shared/agreements/fn*", handler: eula },
        Route { method: "GET", pattern: "/fortnite/api/game/v2/friendcodes/*/epic", handler: friendcodes },
        Route { method: "GET", pattern: "/launcher/api/public/distributionpoints/", handler: distributionpoints },
        Route { method: "GET", pattern: "/launcher/api/public/assets/:platform/:catalogItemId/:appName", handler: assets },
        Route { method: "GET", pattern: "/Builds/Fortnite/Content/CloudDir/*.manifest", handler: cloud_manifest },
        Route { method: "GET", pattern: "/Builds/Fortnite/Content/CloudDir/*.chunk", handler: cloud_chunk },
        Route { method: "GET", pattern: "/Builds/Fortnite/Content/CloudDir/*.ini", handler: cloud_ini },
        Route { method: "GET", pattern: "/fortnite/api/cloudstorage/system", handler: cloudstorage_list },
        Route { method: "GET", pattern: "/fortnite/api/cloudstorage/system/*", handler: cloudstorage_file },
        Route { method: "GET", pattern: "/fortnite/api/keychain", handler: keychain },
        Route { method: "POST", pattern: "/fortnite/api/game/v2/grant_access/*", handler: ok_empty },
        Route { method: "POST", pattern: "/fortnite/api/game/v2/profile/*/client/*", handler: mcp },
        Route { method: "GET", pattern: "/fortnite/api/storefront/v2/catalog", handler: storefront },
        Route { method: "GET", pattern: "/fortnite/api/storefront/v2/keychain", handler: keychain },
        Route { method: "GET", pattern: "/waitingroom/api/waitingroom", handler: waitingroom },
        Route { method: "GET", pattern: "/fortnite/api/lightswitch/api/service/bulk/status", handler: lightswitch },
        Route { method: "GET", pattern: "/fortnite/api/contentpages/pageConfig", handler: contentpages },
        Route { method: "GET", pattern: "/fortnite/api/calendar/v1/timeline", handler: timeline },
        Route { method: "GET", pattern: "/socialban/api/public/v1/*", handler: socialban },
        Route { method: "GET", pattern: "/fortnite/api/game/v2/leaderboards/*", handler: leaderboards },
        Route { method: "POST", pattern: "/fortnite/api/game/v2/chat/*", handler: ok_empty },
        Route { method: "GET", pattern: "/fortnite/api/game/v2/world/info", handler: world_info },
        Route { method: "GET", pattern: "/api/v1/user/setting", handler: ok_empty_array },
        Route { method: "POST", pattern: "/api/v1/user/setting", handler: ok_empty },
        Route { method: "POST", pattern: "/datarouter/api/v1/public/data/*", handler: ok_empty },
        Route { method: "GET", pattern: "/fortnite/api/game/v2/enabled_features", handler: enabled_features },
        Route { method: "GET", pattern: "/fortnite/api/receipts/v1/account/*/receipts", handler: ok_empty_array },
        Route { method: "GET", pattern: "/fortnite/api/matchmaking/session/findPlayer/*", handler: ok_empty_array },
        Route { method: "POST", pattern: "/fortnite/api/matchmaking/session", handler: matchmaking },
        Route { method: "GET", pattern: "/fortnite/api/friends/v1/*/friends", handler: friends },
        Route { method: "GET", pattern: "/fortnite/api/friends/v1/*/summary", handler: friends_summary },
        Route { method: "GET", pattern: "/party/api/v1/Fortnite/user/*", handler: party },
        Route { method: "GET", pattern: "/fortnite/api/savedata/*/client.settings", handler: ok_empty },
        Route { method: "GET", pattern: "/fortnite/api/discovery/row/*", handler: discovery },
    ];

    for r in routes {
        if r.method != req.method && !(r.method == "GET" && req.method == "HEAD") {
            continue;
        }
        if let Some(captures) = pattern_match(r.pattern, &req.path) {
            return (r.handler)(ctx, req, &captures);
        }
    }
    eprintln!("[owen-lawin] NO ROUTE: {} {}", req.method, req.path);
    Response::not_found()
}

pub fn dispatch(ctx: &Context, req: &Request) -> Response {
    let res = route(ctx, req);
    log_line(req, &res);
    res
}

fn log_line(req: &Request, res: &Response) {
    println!(
        "[owen-lawin] {} {} -> {} ({} bytes)",
        req.method,
        req.path,
        res.status,
        res.body.len()
    );
}

/// Match `:name` segments, `*` tails and in-segment globs such as
/// `*.manifest`. Returns captured segments.
fn pattern_match(pattern: &str, path: &str) -> Option<Vec<String>> {
    let pp: Vec<&str> = pattern.split('/').collect();
    let sp: Vec<&str> = path.split('/').collect();
    let mut caps = Vec::new();
    let mut i = 0;
    while i < pp.len() {
        let seg = pp[i];
        if seg == "*" {
            // Tail wildcard: match the rest (may span segments).
            let rest = sp[i.min(sp.len())..].join("/");
            if rest.is_empty() && !pattern.ends_with('*') {
                return None;
            }
            caps.push(rest);
            return Some(caps);
        }
        if i >= sp.len() {
            return None;
        }
        if seg.starts_with(':') {
            if sp[i].is_empty() {
                return None;
            }
            caps.push(sp[i].to_string());
        } else if seg.contains('*') {
            // In-segment glob: `<prefix>*<suffix>` (either side may be empty).
            let (prefix, suffix) = seg.split_once('*')?;
            let cand = sp[i];
            if cand.len() < prefix.len() + suffix.len()
                || !cand.starts_with(prefix)
                || !cand.ends_with(suffix)
            {
                return None;
            }
            // Capture the full segment (handlers build filenames from it).
            caps.push(cand.to_string());
        } else if seg != sp[i] {
            return None;
        }
        i += 1;
    }
    if sp.len() != pp.len() {
        return None;
    }
    Some(caps)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// The Owen handshake: `GameVersion = "<version>"`.
fn root_post(ctx: &Context, req: &Request, _c: &[String]) -> Response {
    let body = req.body_str();
    let version = body
        .split('"')
        .nth(1)
        .unwrap_or("unknown")
        .to_string();
    println!(
        "[owen-lawin] Owen Redirecter connected: {version} (backend {})",
        ctx.opts.display_name
    );
    Response::text(200, "OK")
}

/// The login page: mirrors the original LoginWebServer behavior — serve a
/// minimal page that immediately redirects into the game's authorize
/// deep-link with the player name as the code.
fn login(ctx: &Context, req: &Request, _c: &[String]) -> Response {
    let name = req
        .query_param("name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| ctx.opts.display_name.clone())
        .replace('"', "");
    let page = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Sign in</title>
<style>body{{background:#0f0f10;color:#e4e4e7;font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}.card{{background:#18181b;border:1px solid #27272a;border-radius:12px;padding:32px 40px;text-align:center}}a{{color:#34d399}}</style></head>
<body><div class="card"><h2>Owen Backend</h2><p>Signing in as <b>{name}</b>...</p>
<p><a href="com.epicgames.fortnite://authorize/?code={name}">Continue to Fortnite</a></p>
<script>location.href="com.epicgames.fortnite://authorize/?code={name}";</script>
</div></body></html>"#
    );
    Response::bytes("text/html; charset=utf-8", page.into_bytes())
}

fn eula(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(
        200,
        r#"[{"key":"epic_games_eula","version":1,"revocation":null,"localizedBody":null,"localizedTitle":null,"locale":"en","created":{},"lastModified":null,"status":"ACTIVE"}]"#.replace("{}", "1580000000000"),
    )
}

fn friendcodes(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(200, r#""FORTNITE""#)
}

fn distributionpoints(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    let _ = ctx;
    Response::json(
        200,
        r#"[{"name":"FortniteContent","path":"https://download.epicgames.com/fortnite/"}]"#,
    )
}

/// `/launcher/api/public/assets/Windows/4fe75bbc5a674f4f9b3569e73c52e7e2/Fortnite%2B12.50-CL-13070927-Android`
/// The Android client asks for the manifest of its build; we answer with the
/// injected manifest so the game installs its content from our CloudDir.
fn assets(ctx: &Context, req: &Request, c: &[String]) -> Response {
    let app_name = c
        .get(2)
        .map(|s| crate::url_decode(s))
        .unwrap_or_default();
    let is_android = req.user_agent().contains("Android")
        || app_name.contains("Android")
        || req.query_param("Label").is_some();

    // injected_manifests.json maps the game's version string to a manifest id.
    let manifests = ctx.asset("Config/injected_manifests.json");
    if let Some(text) = manifests {
        if let Some(value) = extract_manifest_for(text.as_str(), &app_name) {
            let catalog_item = c.get(1).cloned().unwrap_or_default();
            return Response::json(
                200,
                format!(
                    r#"{{"appName":"{app_name}","catalogItemId":"{catalog_item}","buildVersion":"{}","label":"Live","items":{{}},"assetId":"{value}"}}"#,
                    app_name.replace('+', "%2B"),
                ),
            );
        }
    }

    if is_android {
        // Generic fallback: serve the first manifest entry.
        let generic = ctx.asset("Config/injected_manifests.json").map(|t| first_manifest(t.as_str())).flatten();
        if let Some(value) = generic {
            return Response::json(200, format!(r#"{{"appName":"{app_name}","catalogItemId":"{}","buildVersion":"{app_name}","label":"Live","items":{{}},"assetId":"{value}"}}"#, c.get(1).cloned().unwrap_or_default()));
        }
    }
    Response::not_found()
}

/// Minimal string scraping of the injected manifest table (kept dependency-free).
fn extract_manifest_for(table: &str, app_name: &str) -> Option<String> {
    let needle = app_name.split('-').next().unwrap_or(app_name);
    let mut idx = 0usize;
    while let Some(p) = table[idx..].find(needle) {
        let start = idx + p;
        // find the following "manifest_hash": "value"
        let window = &table[start..(start + 512).min(table.len())];
        if let Some(h) = extract_json_string(window, "manifest_hash") {
            return Some(h);
        }
        idx = start + needle.len();
    }
    None
}

fn first_manifest(table: &str) -> Option<String> {
    extract_json_string(table, "manifest_hash")
}

fn extract_json_string(src: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let p = src.find(&pat)?;
    let rest = &src[p + pat.len()..];
    let colon = rest.find(':')?;
    let rest = &rest[colon + 1..];
    let rest = rest.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let end = rest[1..].find('"')? + 1;
    Some(rest[1..end].to_string())
}

fn cloud_manifest(ctx: &Context, _req: &Request, c: &[String]) -> Response {
    serve_file(ctx, &format!("responses/CloudDir/{}", c.first().cloned().unwrap_or_default()))
}
fn cloud_chunk(ctx: &Context, _req: &Request, c: &[String]) -> Response {
    serve_file(ctx, &format!("responses/CloudDir/{}", c.first().cloned().unwrap_or_default()))
}
fn cloud_ini(ctx: &Context, _req: &Request, c: &[String]) -> Response {
    serve_file(ctx, &format!("responses/CloudDir/{}", c.first().cloned().unwrap_or_default()))
}

fn cloudstorage_list(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    let entries = ["DefaultEngine.ini", "DefaultGame.ini", "DefaultInput.ini", "DefaultRuntimeOptions.ini"];
    let items: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            let bytes = ctx.asset_bytes(&format!("CloudStorage/{e}"))?;
            Some(format!(
                r#"{{"uniqueFilename":"{e}","filename":"{e}","hash":"{}","hashType":"MD5","size":{},"contentType":"text/plain-charset=utf-8","uploaded":"2024-01-01T00:00:00.000Z","storageType":"S3"}}"#,
                crate::mcp::md5_hex(&bytes),
                bytes.len()
            ))
        })
        .collect();
    Response::json(200, format!("[{}]", items.join(",")))
}

fn cloudstorage_file(ctx: &Context, _req: &Request, c: &[String]) -> Response {
    serve_file(ctx, &format!("CloudStorage/{}", c.first().cloned().unwrap_or_default()))
}

fn keychain(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/keychain.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(Response::not_found)
}

fn ok_empty(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::new(204)
}

fn ok_empty_array(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(200, "[]")
}

fn mcp(ctx: &Context, req: &Request, c: &[String]) -> Response {
    let account = c.first().cloned().unwrap_or_default();
    let command = c.get(1).cloned().unwrap_or_default();
    let profile_id = req
        .query_param("profileId")
        .unwrap_or("common_core")
        .to_string();
    crate::mcp::handle(ctx, req, &account, &command, &profile_id)
}

fn storefront(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/catalog.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, r#"{"storefronts":[]}"#))
}

fn waitingroom(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::text(200, "OK")
}

fn lightswitch(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    let _ = ctx;
    Response::json(
        200,
        r#"[{"serviceInstanceId":"fortnite","status":"UP","message":"fortnite is up","maintenanceUri":null,"overrideCatalogIds":["a7f138b2e51945ffbfdacc1af0543537"],"allowedToPlay":true,"banned":false,"launcherInfoDTO":{}}]"#,
    )
}

fn contentpages(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/contentpages.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, r#"{"contentPages":[]}"#))
}

fn timeline(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/Athena/Tournament/timeline.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, r#"{"channels":{},"events":[]}"#))
}

fn socialban(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(200, r#"{"bans":[],"warnings":[]}"#)
}

fn leaderboards(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(200, "[]")
}

fn world_info(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/Athena/Discovery/discovery_frontend.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, "[]"))
}

fn enabled_features(_ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    Response::json(200, "[]")
}

fn matchmaking(ctx: &Context, req: &Request, _c: &[String]) -> Response {
    // The session ticket the game needs to connect to the gameserver from
    // config.ini (ip/port).
    let body = req.body_str();
    let _ = body;
    let cfg = ctx.asset("Config/config.ini");
    let (ip, port) = cfg
        .map(|c| parse_gameserver(c.as_str()))
        .unwrap_or(("127.0.0.1".to_string(), 7777u16));
    Response::json(
        200,
        format!(
            r#"{{"id":"{}","sessionId":"owen-session","bucketId":"Fortnite_1v1_Athena_STW_PvE_Campaign_NoBR","attributes":{{"REGION_squadSize":1}},"created":{},"launcherInfoDTO":{{}},"bucketPath":"fortnite-env:fortnite:A1.PUBLIC.STANDARD:Fortnite_1v1_Athena_STW_PvE_Campaign_NoBR:PC:World:play_world"}} "#,
            "00000000-0000-0000-0000-000000000001",
            1580000000000u64
        )
        .replace("PLAYWORLD_ADDR", &format!("{ip}:{port}")),
    )
}

fn parse_gameserver(ini: &str) -> (String, u16) {
    let mut ip = "127.0.0.1".to_string();
    let mut port = 7777u16;
    for line in ini.lines() {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            match k.as_str() {
                "ip" => ip = v.to_string(),
                "port" => port = v.parse().unwrap_or(7777),
                _ => {}
            }
        }
    }
    (ip, port)
}

fn friends(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/friendslist.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, "[]"))
}

fn friends_summary(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/friendslist2.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, r#"{"friends":[],"incoming":[],"outgoing":[],"suggested":[]}"#))
}

fn party(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    let _ = ctx;
    Response::json(
        200,
        r#"{"current":null,"invites":[],"pings":[],"displayNames":{},"parties":[]}"#,
    )
}

fn discovery(ctx: &Context, _req: &Request, _c: &[String]) -> Response {
    ctx.asset("responses/Athena/Discovery/discovery_api_assets.json")
        .map(|t| Response::bytes("application/json", t.as_bytes().to_vec()))
        .unwrap_or_else(|| Response::json(200, "[]"))
}
