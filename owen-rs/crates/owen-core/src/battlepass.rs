//! Battle Pass data extraction from pak listings.
//!
//! Fortnite stores its cosmetic item definitions inside the game paks under
//! predictable paths:
//!
//! ```text
//! FortniteGame/Content/Athena/Items/Cosmetics/Characters/CID_533_Athena_Commando_M_Donut.ucase
//! FortniteGame/Content/Athena/Items/Cosmetics/Backpacks/BID_...
//! FortniteGame/Content/Athena/Items/Cosmetics/Dances/EID_...
//! FortniteGame/Content/Athena/Items/Cosmetics/Pickaxes/PID_...
//! FortniteGame/Content/Athena/Items/Cosmetics/Sprays/Spid_...  (etc)
//! ```
//!
//! Each pak belongs to one build; combining the build version (season
//! mapping via [`crate::version::FortniteVersion::season`]) with the item
//! listing produces per-season Battle Pass and item shop data.

use crate::pak::PakIndex;
use crate::version::FortniteVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CosmeticKind {
    Character,
    Backpack,
    Pickaxe,
    Dance,
    Glider,
    Wrap,
    Emoji,
    Spray,
    LoadingScreen,
    Music,
    Contrail,
    Pet,
    Toy,
    Banner,
    Other,
}

impl CosmeticKind {
    pub fn from_path(path: &str) -> Self {
        let p = path.to_ascii_lowercase();
        if p.contains("characters/cid_") {
            CosmeticKind::Character
        } else if p.contains("backpacks/bid_") {
            CosmeticKind::Backpack
        } else if p.contains("pickaxes/pid_") {
            CosmeticKind::Pickaxe
        } else if p.contains("dances/eid_") {
            CosmeticKind::Dance
        } else if p.contains("gliders/glider") || p.contains("gliders/gid_") {
            CosmeticKind::Glider
        } else if p.contains("wrap") {
            CosmeticKind::Wrap
        } else if p.contains("emoji") {
            CosmeticKind::Emoji
        } else if p.contains("spray") {
            CosmeticKind::Spray
        } else if p.contains("loadingscreen") {
            CosmeticKind::LoadingScreen
        } else if p.contains("musicpack") || p.contains("music") {
            CosmeticKind::Music
        } else if p.contains("contrails") {
            CosmeticKind::Contrail
        } else if p.contains("pets") {
            CosmeticKind::Pet
        } else if p.contains("toys") {
            CosmeticKind::Toy
        } else if p.contains("banners") {
            CosmeticKind::Banner
        } else {
            CosmeticKind::Other
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CosmeticKind::Character => "Outfit",
            CosmeticKind::Backpack => "Back Bling",
            CosmeticKind::Pickaxe => "Harvesting Tool",
            CosmeticKind::Dance => "Emote",
            CosmeticKind::Glider => "Glider",
            CosmeticKind::Wrap => "Wrap",
            CosmeticKind::Emoji => "Emoji",
            CosmeticKind::Spray => "Spray",
            CosmeticKind::LoadingScreen => "Loading Screen",
            CosmeticKind::Music => "Music",
            CosmeticKind::Contrail => "Contrail",
            CosmeticKind::Pet => "Pet",
            CosmeticKind::Toy => "Toy",
            CosmeticKind::Banner => "Banner",
            CosmeticKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticItem {
    /// Asset name, e.g. `CID_533_Athena_Commando_M_Donut`.
    pub id: String,
    pub kind: CosmeticKind,
    /// Full path inside the pak.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonBattlePass {
    /// e.g. "Chapter 2 Season 2".
    pub title: String,
    pub chapter: u32,
    pub season: u32,
    pub version: String,
    pub items: Vec<CosmeticItem>,
    pub total_files: usize,
}

/// Extract the cosmetic listing from a parsed pak index for a version.
pub fn extract(index: &PakIndex, version: &FortniteVersion) -> SeasonBattlePass {
    let (chapter, season) = version.season();
    let mut items = Vec::new();
    for e in &index.entries {
        let lower = e.name.to_ascii_lowercase();
        if !lower.contains("/athena/items/cosmetics/") {
            continue;
        }
        let id = e
            .name
            .rsplit('/')
            .next()
            .unwrap_or(&e.name)
            .trim_end_matches(".ucase")
            .to_string();
        items.push(CosmeticItem {
            kind: CosmeticKind::from_path(&e.name),
            path: e.name.clone(),
            id,
        });
    }
    items.sort_by(|a, b| a.kind.label().cmp(b.kind.label()).then(a.id.cmp(&b.id)));
    SeasonBattlePass {
        title: format!("Chapter {chapter} Season {season}"),
        chapter,
        season,
        version: version.to_string(),
        total_files: index.entries.len(),
        items,
    }
}
