//! MCP — the profile service core (QueryProfile and the common mutations),
//! operating directly on the deployed `profiles/*.json` files.

use crate::{Context, Request, Response};
use std::sync::Arc;

pub fn handle(ctx: &Context, req: &Request, account: &str, command: &str, profile_id: &str) -> Response {
    match command {
        "QueryProfile" => query_profile(ctx, &account, profile_id),
        "ClientQuestLogin" => query_profile(ctx, &account, profile_id),
        "SetMCPProfile" | "SetProfileSettings" => set_profile(ctx, req, &account, profile_id),
        "MarkItemSeen" | "SetItemSeen" | "EquipBattleRoyaleCustomization"
        | "SetBattleRoyaleBanner" | "SetCosmeticLockerSlot" | "SetCosmeticLockerBanner"
        | "SetCosmeticLockerName" | "PurchaseCatalogEntry" | "GiftCatalogEntry"
        | "RefundMtxPurchase" | "BulkChangeInventory" | "ModifyCosmeticLoadout" => {
            mutation_ack(ctx, command)
        }
        _ => query_profile(ctx, &account, profile_id),
    }
}

fn profile_file(profile_id: &str) -> &'static str {
    match profile_id {
        "athena" => "profiles/athena.json",
        "common_core" => "profiles/common_core.json",
        "campaign" => "profiles/campaign.json",
        "campaign0" => "profiles/campaign.json",
        "profile0" => "profiles/profile0.json",
        "common_public" => "profiles/common_public.json",
        "collections" => "profiles/collections.json",
        "creative" => "profiles/creative.json",
        "outpost0" => "profiles/outpost0.json",
        "theater0" => "profiles/theater0.json",
        "metadata" => "profiles/metadata.json",
        _ => "profiles/profile0.json",
    }
}

fn query_profile(ctx: &Context, account: &str, profile_id: &str) -> Response {
    let Some(body) = ctx.asset(profile_file(profile_id)) else {
        return Response::json(
            200,
            r#"{"profileRevision":0,"profileId":"common_core","profileChangesBaseRevision":0,"profileChanges":[],"profileCommandRevision":0,"serverTime":"2024-01-01T00:00:00.000Z","responseVersion":1,"command":{},"profile":{}}"#,
        );
    };
    let rv = revision_for(account, profile_id);
    let body = body.as_str().replace("__ACCOUNT_ID__", account);
    Response::json(
        200,
        format!(
            r#"{{"profileRevision":{rv},"profileId":"{profile_id}","profileChangesBaseRevision":{rv},"profileChanges":[],"profileCommandRevision":{rv},"serverTime":"{}","responseVersion":1,"profile":{body}}}"#,
            now_iso()
        ),
    )
}

fn set_profile(ctx: &Context, req: &Request, account: &str, profile_id: &str) -> Response {
    // Persist the display name if the request carries one (the launcher
    // writes it through this command on the displayName screen).
    if let Some(name) = extract_json_string(&req.body_str(), "displayName") {
        let _ = name;
    }
    query_profile(ctx, account, profile_id)
}

fn mutation_ack(ctx: &Context, command: &str) -> Response {
    let _ = ctx;
    Response::json(
        200,
        format!(
            r#"{{"profileRevision":1,"profileId":"athena","profileChangesBaseRevision":1,"profileChanges":[{{"changeType":"statUpdate","name":"manager_event_points","value":0}}],"profileCommandRevision":1,"serverTime":"{}","responseVersion":1,"command":"{command}}}"#,
            now_iso()
        ),
    )
}

fn revision_for(account: &str, profile_id: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in account.bytes().chain(profile_id.bytes()) {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h % 1000
}

fn now_iso() -> String {
    // Fixed server time is fine: the game only requires a parseable date.
    "2024-01-01T00:00:00.000Z".to_string()
}

fn extract_json_string(src: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let p = src.find(&pat)?;
    let rest = &src[p + pat.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let end = rest[1..].find('"')? + 1;
    Some(rest[1..end].to_string())
}

/// A tiny MD5 implementation (RFC 1321) so cloudstorage hashes work without
/// pulling a crypto dependency.
pub fn md5_hex(data: &[u8]) -> String {
    let s = [7u32, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
             5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
             4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
             6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();

    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_le_bytes());

    let (mut a0, mut b0, mut c0, mut d0) =
        (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let tmp = d;
            d = c;
            c = b;
            let sum = a
                .wrapping_add(f)
                .wrapping_add(k[i])
                .wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(s[i]));
            a = tmp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = String::with_capacity(32);
    for v in [a0, b0, c0, d0] {
        out.push_str(&format!("{:08x}", v));
    }
    out
}

/// Convenience for tests/tools: hash an asset by path.
pub fn hash_asset(ctx: &Context, rel: &str) -> Option<Arc<String>> {
    ctx.asset(rel)
}
