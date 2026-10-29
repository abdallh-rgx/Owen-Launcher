//! Static Fortnite season registry (Season 1 through 42) plus Battle Pass
//! profile-enrichment helpers used by the MCP athena handler and the
//! `/api/launcher/seasons` endpoints.

use crate::state::Account;
use serde_json::{json, Value};

/// Highest season known to the registry.
pub const MAX_SEASON: u32 = 42;

/// Static metadata for one Fortnite season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeasonMeta {
    pub season: u32,
    pub chapter: u32,
    pub chapter_season: u32,
    pub name: &'static str,
    pub start: &'static str,
    pub end: &'static str,
}

/// (season, name, start, end) — chapter mapping is computed once per entry.
const TABLE: &[(u32, &'static str, &'static str, &'static str)] = &[
    (1, "", "2017-10-25", "2017-12-13"),
    (2, "", "2017-12-14", "2018-02-21"),
    (3, "", "2018-02-22", "2018-04-30"),
    (4, "Blockbuster", "2018-05-01", "2018-07-11"),
    (5, "Worlds Collide", "2018-07-12", "2018-09-26"),
    (6, "Darkness Rises", "2018-09-27", "2018-12-05"),
    (7, "The Iceberg", "2018-12-06", "2019-02-27"),
    (8, "X Marks the Spot", "2019-02-28", "2019-05-08"),
    (9, "The Future Is Yours", "2019-05-09", "2019-07-31"),
    (10, "Out of Time", "2019-08-01", "2019-10-13"),
    (11, "Chapter 2", "2019-10-15", "2020-02-19"),
    (12, "Top Secret", "2020-02-20", "2020-06-16"),
    (13, "Splash Down", "2020-06-17", "2020-08-26"),
    (14, "Nexus War", "2020-08-27", "2020-12-01"),
    (15, "Zero Point", "2020-12-02", "2021-03-15"),
    (16, "Primal", "2021-03-16", "2021-06-07"),
    (17, "Invasion", "2021-06-08", "2021-09-12"),
    (18, "Cubed", "2021-09-13", "2021-12-04"),
    (19, "Flipped", "2021-12-05", "2022-03-19"),
    (20, "Resistance", "2022-03-20", "2022-06-04"),
    (21, "Vibin'", "2022-06-05", "2022-09-17"),
    (22, "Chrome", "2022-09-18", "2022-12-03"),
    (23, "A New Beginning", "2022-12-04", "2023-03-09"),
    (24, "Most Wanted", "2023-03-10", "2023-06-08"),
    (25, "Wilds", "2023-06-09", "2023-08-25"),
    (26, "Last Resort", "2023-08-26", "2023-11-02"),
    (27, "Fortnite OG", "2023-11-03", "2023-12-02"),
    (28, "Underground", "2023-12-03", "2024-03-07"),
    (29, "Myths & Mortals", "2024-03-08", "2024-05-23"),
    (30, "Wrecked", "2024-05-24", "2024-08-15"),
    (31, "Absolute Doom", "2024-08-16", "2024-11-30"),
    (32, "Demon Hunters", "2024-12-01", "2025-02-20"),
    (33, "", "2025-02-21", "2025-06-07"),
    (34, "", "2025-06-08", "2025-09-15"),
    (35, "", "2025-09-16", "2025-12-15"),
    (36, "", "2025-12-16", "2026-03-15"),
    (37, "", "2026-03-16", "2026-06-15"),
    (38, "", "2026-06-16", "2026-09-15"),
    (39, "", "2026-09-16", "2026-12-15"),
    (40, "", "2026-12-16", "2027-03-15"),
    (41, "", "2027-03-16", "2027-06-15"),
    (42, "", "2027-06-16", "2027-09-15"),
];

/// Chapter mapping: 1-10 → C1, 11-18 → C2, 19-22 → C3, 23-26 → C4,
/// 27 → OG (chapter 1), 28-31 → C5, 32-35 → C6, 36-42 → C7.
pub fn chapter_of(season: u32) -> (u32, u32) {
    match season {
        1..=10 => (1, season),
        11..=18 => (2, season - 10),
        19..=22 => (3, season - 18),
        23..=26 => (4, season - 22),
        27 => (1, 7), // Fortnite OG — chapter 1 remix
        28..=31 => (5, season - 27),
        32..=35 => (6, season - 31),
        36..=42 => (7, season - 35),
        _ => (1, season.min(10)),
    }
}

fn meta_of(row: &(u32, &'static str, &'static str, &'static str)) -> SeasonMeta {
    let (chapter, chapter_season) = chapter_of(row.0);
    SeasonMeta {
        season: row.0,
        chapter,
        chapter_season,
        name: row.1,
        start: row.2,
        end: row.3,
    }
}

/// Look up static metadata for a season number.
pub fn season_info(n: u32) -> Option<SeasonMeta> {
    TABLE.iter().find(|(s, ..)| *s == n).map(meta_of)
}

/// Like [`season_info`] but falls back to season 1 for unknown numbers
/// (used by handlers that must always answer).
pub fn season_info_or_default(n: u32) -> SeasonMeta {
    season_info(n).unwrap_or_else(|| season_info(1).unwrap_or(meta_of(&TABLE[0])))
}

/// All known seasons, in order.
pub fn all_seasons() -> Vec<SeasonMeta> {
    TABLE.iter().map(meta_of).collect()
}

/// Season banner image path — all 42 seasons ship an original folder image
/// (S1-4, S11, S34-42 generated for this release; the rest are the original
/// app's asset set).
pub fn season_image(n: u32) -> Option<&'static str> {
    if !(1..=42).contains(&n) {
        return None;
    }
    const IMAGES: [&str; 42] = [
        "/owen/seasons/season_1_folder.png", "/owen/seasons/season_2_folder.png",
        "/owen/seasons/season_3_folder.png", "/owen/seasons/season_4_folder.png",
        "/owen/seasons/season_5_folder.png", "/owen/seasons/season_6_folder.png",
        "/owen/seasons/season_7_folder.png", "/owen/seasons/season_8_folder.png",
        "/owen/seasons/season_9_folder.png", "/owen/seasons/season_10_folder.png",
        "/owen/seasons/season_11_folder.png", "/owen/seasons/season_12_folder.png",
        "/owen/seasons/season_13_folder.png", "/owen/seasons/season_14_folder.png",
        "/owen/seasons/season_15_folder.png", "/owen/seasons/season_16_folder.png",
        "/owen/seasons/season_17_folder.png", "/owen/seasons/season_18_folder.png",
        "/owen/seasons/season_19_folder.png", "/owen/seasons/season_20_folder.png",
        "/owen/seasons/season_21_folder.png", "/owen/seasons/season_22_folder.png",
        "/owen/seasons/season_23_folder.png", "/owen/seasons/season_24_folder.png",
        "/owen/seasons/season_25_folder.png", "/owen/seasons/season_26_folder.png",
        "/owen/seasons/season_27_folder.png", "/owen/seasons/season_28_folder.png",
        "/owen/seasons/season_29_folder.png", "/owen/seasons/season_30_folder.png",
        "/owen/seasons/season_31_folder.png", "/owen/seasons/season_32_folder.png",
        "/owen/seasons/season_33_folder.png", "/owen/seasons/season_34_folder.png",
        "/owen/seasons/season_35_folder.png", "/owen/seasons/season_36_folder.png",
        "/owen/seasons/season_37_folder.png", "/owen/seasons/season_38_folder.png",
        "/owen/seasons/season_39_folder.png", "/owen/seasons/season_40_folder.png",
        "/owen/seasons/season_41_folder.png", "/owen/seasons/season_42_folder.png",
    ];
    IMAGES.get((n - 1) as usize).copied()
}

/// Resolve the effective season: `X-Season` header > `?season=` query >
/// account selection > current default. Values outside 1..=MAX_SEASON are
/// ignored.
pub fn resolve_season(
    header: Option<&str>,
    query: Option<&str>,
    selected: Option<u32>,
    current: u32,
) -> u32 {
    let parsed = |v: Option<&str>| {
        v.and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|n| (1..=MAX_SEASON).contains(n))
    };
    parsed(header)
        .or_else(|| parsed(query))
        .or_else(|| selected.filter(|n| (1..=MAX_SEASON).contains(n)))
        .unwrap_or_else(|| current.clamp(1, MAX_SEASON))
}

/// (paidRewards len, freeRewards len) of a battle pass JSON value.
pub fn battlepass_tiers(bp: &Value) -> (usize, usize) {
    let paid = bp
        .get("paidRewards")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let free = bp
        .get("freeRewards")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    (paid, free)
}

/// Flatten battle pass tiers into profile items (Reload-Backend
/// semantics): grant `paidRewards` tiers up to `min(level, tiers.len())`,
/// and additionally `freeRewards` when the pass was purchased. Each tier
/// is a map of `templateId -> quantity`; empty maps produce nothing.
pub fn flatten_battlepass_items(bp: &Value, level: u32, purchased: bool) -> Vec<Value> {
    let mut out = Vec::new();
    let mut add_track = |key: &str| {
        let Some(tiers) = bp.get(key).and_then(|v| v.as_array()) else {
            return;
        };
        let up_to = (level as usize).min(tiers.len());
        for tier in tiers.iter().take(up_to) {
            let Some(entries) = tier.as_object() else {
                continue;
            };
            for (template_id, quantity) in entries {
                out.push(json!({
                    "templateId": template_id,
                    "quantity": quantity,
                    "attributes": { "item_seen": true },
                }));
            }
        }
    };
    add_track("paidRewards");
    if purchased {
        add_track("freeRewards");
    }
    out
}

/// Enrich an athena profile with account stats and battle pass items
/// before it is returned by the MCP handler.
pub fn enrich_athena(profile: &mut Value, account: &Account, battlepass: &Value) {
    // stats.attributes: ensure level / book_level / battlepass_purchased.
    if !profile.get("stats").map(|s| s.is_object()).unwrap_or(false) {
        profile["stats"] = json!({});
    }
    if !profile["stats"].get("attributes").map(|a| a.is_object()).unwrap_or(false) {
        profile["stats"]["attributes"] = json!({});
    }
    if let Some(attrs) = profile["stats"]["attributes"].as_object_mut() {
        attrs.insert("level".into(), json!(account.level));
        attrs.insert("book_level".into(), json!(account.level.min(100)));
        attrs.insert("battlepass_purchased".into(), json!(account.battle_pass_purchased));
    }
    // items: flattened battle pass tiers for the player's level.
    profile["items"] = Value::Array(flatten_battlepass_items(
        battlepass,
        account.level,
        account.battle_pass_purchased,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_season_2_to_42_has_meta() {
        for n in 2..=42 {
            let meta = season_info(n).unwrap_or_else(|| panic!("season {n} missing"));
            assert_eq!(meta.season, n);
            assert!(meta.start.len() == 10 && meta.end.len() == 10);
            assert!(meta.start < meta.end, "season {n}: start must precede end");
        }
        assert!(season_info(1).is_some());
        assert!(season_info(0).is_none());
        assert!(season_info(43).is_none());
    }

    #[test]
    fn chapter_mapping_spot_checks() {
        assert_eq!(chapter_of(12), (2, 2)); // C2S2 Top Secret
        assert_eq!(chapter_of(20), (3, 2)); // C3S2 Resistance
        assert_eq!(chapter_of(28), (5, 1)); // C5S1 Underground
        let og = season_info(27).unwrap();
        assert_eq!(og.name, "Fortnite OG");
        assert_eq!(og.chapter, 1);
        assert_eq!(chapter_of(11), (2, 1));
        assert_eq!(chapter_of(19), (3, 1));
        assert_eq!(chapter_of(23), (4, 1));
        assert_eq!(chapter_of(32), (6, 1));
        assert_eq!(chapter_of(36), (7, 1));
        assert_eq!(chapter_of(42), (7, 7));
    }

    #[test]
    fn all_seasons_returns_full_ordered_table() {
        let all = all_seasons();
        assert_eq!(all.len(), 42);
        assert!(all.windows(2).all(|w| w[0].season + 1 == w[1].season));
        assert_eq!(all[0].name, "");
        assert_eq!(all[11].name, "Top Secret");
        assert_eq!(all[26].name, "Fortnite OG");
        assert_eq!(all[27].name, "Underground");
    }

    #[test]
    fn image_range_is_1_to_42() {
        for n in 1..=42 {
            assert_eq!(
                season_image(n).map(|s| s.to_string()),
                Some(format!("/owen/seasons/season_{n}_folder.png")),
                "season {n}"
            );
        }
        for n in [0u32, 43, 99] {
            assert_eq!(season_image(n), None, "season {n} must have no image");
        }
    }

    #[test]
    fn resolve_season_precedence() {
        // header > query > selected > current
        assert_eq!(resolve_season(Some("12"), Some("20"), Some(7), 42), 12);
        assert_eq!(resolve_season(None, Some("20"), Some(7), 42), 20);
        assert_eq!(resolve_season(None, None, Some(7), 42), 7);
        assert_eq!(resolve_season(None, None, None, 42), 42);
        // invalid values fall through
        assert_eq!(resolve_season(Some("99"), Some("20"), None, 42), 20);
        assert_eq!(resolve_season(Some("abc"), None, None, 42), 42);
        assert_eq!(resolve_season(None, None, Some(99), 42), 42);
        assert_eq!(resolve_season(None, None, None, 0), 1);
        // whitespace tolerated
        assert_eq!(resolve_season(Some(" 12 "), None, None, 42), 12);
    }

    #[test]
    fn flatten_battlepass_items_slices_by_level() {
        let bp = json!({
            "paidRewards": [
                { "AthenaCharacter:cid_a": 1, "AthenaPickaxe:pick_a": 1 },
                {},
                { "AthenaBackpack:bid_a": 1 }
            ],
            "freeRewards": [
                { "AthenaDance:spid_a": 1 },
                { "Currency:mtxgiveaway": 100 }
            ]
        });
        // level 1 → only tier 0 (one item), no free track
        let items = flatten_battlepass_items(&bp, 1, false);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["templateId"], "AthenaCharacter:cid_a");
        assert_eq!(items[0]["quantity"], 1);
        assert_eq!(items[0]["attributes"]["item_seen"], true);
        // level 2 → tiers 0..2, tier 1 empty → still 2 items
        assert_eq!(flatten_battlepass_items(&bp, 2, false).len(), 2);
        // level 5 → clamped to 3 tiers
        let items = flatten_battlepass_items(&bp, 5, false);
        assert_eq!(items.len(), 3);
        assert_eq!(items[2]["templateId"], "AthenaBackpack:bid_a");
        // purchased → free track included too
        let items = flatten_battlepass_items(&bp, 5, true);
        assert_eq!(items.len(), 5);
        assert_eq!(items[4]["templateId"], "Currency:mtxgiveaway");
        assert_eq!(items[4]["quantity"], 100);
    }

    #[test]
    fn enrich_athena_sets_stats_and_items() {
        let mut account = crate::state::Account::new("Tester".into());
        account.level = 150;
        account.battle_pass_purchased = true;
        let bp = json!({
            "paidRewards": [{ "AthenaCharacter:cid_a": 1 }],
            "freeRewards": [{ "AthenaDance:spid_a": 1 }]
        });
        let mut profile = json!({ "_id": account.account_id, "items": [] });
        enrich_athena(&mut profile, &account, &bp);
        assert_eq!(profile["stats"]["attributes"]["level"], 150);
        assert_eq!(profile["stats"]["attributes"]["book_level"], 100);
        assert_eq!(profile["stats"]["attributes"]["battlepass_purchased"], true);
        assert_eq!(profile["items"].as_array().unwrap().len(), 2);

        // works even when stats is entirely absent
        let mut bare = json!({});
        account.level = 2;
        account.battle_pass_purchased = false;
        enrich_athena(&mut bare, &account, &bp);
        assert_eq!(bare["stats"]["attributes"]["book_level"], 2);
        assert_eq!(bare["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn battlepass_tiers_counts() {
        let bp = json!({
            "paidRewards": Value::Array(vec![json!({}); 100]),
            "freeRewards": Value::Array(vec![json!({}); 50])
        });
        assert_eq!(battlepass_tiers(&bp), (100, 50));
        assert_eq!(battlepass_tiers(&json!({})), (0, 0));
    }
}
