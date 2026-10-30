# OGS-12.41 study pack (MarvelcoCode/OGS-12.41)

This folder mirrors the **only data** the upstream project ships that is relevant
to our stack, plus machine-readable derivations. It is a *study pack* — nothing
in `data/owen/` or `data/lawin/` was touched.

## What OGS-12.41 actually is

Upstream: <https://github.com/MarvelcoCode/OGS-12.41> (released publicly by one
of its two authors after a payment dispute; see its README).

It is **NOT a private server**. It is a **client-side C++ (Win64) DLL mod**
(`dllmain.cpp` + `OGS-S12.vcxproj`, Visual Studio solution) that is injected
into the *Fortnite 12.41 Windows process* and uses **MinHook** plus a dumped
**Fortnite 12.41 SDK** (`SDK.rar`, 11 MB — NOT mirrored here) to flip
`GIsClient=false / GIsServer=true` inside the game, turning the local client
into a listen server. Everything it "serves" is implemented **in-process**
through native/SDK calls:

| Module (`OGS-S12/*.h`) | What it does in-process |
|---|---|
| `Bots.h`, `PlayerBots.h`, `BT_MANG2.h`, `BehaviourTree_System.h` | AI bot players (spawn, drop, fight) |
| `Bosses.h` | boss encounters |
| `PhantomBoothSpawner.h` | the "booths" the README mentions |
| `Quests.h` | accolade/XP/quest granting via `XPComponent` |
| `Building.h`, `Looting.h`, `LateGameLoot.h`, `Inventory.h` | build/loot behavior |
| `Creative.h`, `Misc.h`, `Vehicles.h`, `Abilities.h` | creative islands, extras |
| `Net.h`, `Tick.h`, `GameMode.h`, `PC.h`, `PE.h` | net driver + gamemode hooks |
| `Globals.h` | runtime toggles (`MaxBotsToSpawn`, `bBossesEnabled`, …) |

Stack: **C++ / MSVC / Win32** (`AllocConsole`, `ImageBase + 0x…` offsets,
`DWORD Main(LPVOID)`), **x64-only**, no HTTP server, no Node, no C#.

### Consequence for our repo — the honest answer

**OGS-12.41 ships zero server-response data.** There is no MCP JSON, no
keychain, no catalog, no timeline, no cloudstorage INI, no XMPP payload —
nothing an HTTP backend like `owen-server` could serve. Verified by scanning
every file for JSON/INI content: the repo's only non-code artifacts are the
two data headers mirrored below (plus `SDK.rar`, a binary SDK dump we do not
redistribute).

So for *full 12.41 ingame support over HTTP*, `data/owen/` + `data/lawin/`
already cover everything a server can cover:

| Need (12.41) | Already served by | File |
|---|---|---|
| MCP profiles (athena/campaign/common_core) | `POST /fortnite/api/game/v2/profile/{id}/client/:operation` | `data/owen/profiles/*.json` |
| Catalog + keychain | `/fortnite/api/storefront/v2/catalog`, `…/keychain` | `data/owen/responses/catalog.json`, `responses/keychain.json` |
| Timeline (season stamp 12) | `/fortnite/api/calendar/v1/timeline?season=12` | generated per-season by `owen-server/src/seasons.rs` |
| Cloudstorage INIs | `/fortnite/api/cloudstorage/system*` | `data/owen/CloudStorage/*.ini` (incl. `responses/patches/`) |
| Battle pass (S12) | battlepass admin API | `data/owen/battlepass/Season12.json` |
| Friends list | `/friends/api/v1/:id/friends` | `data/lawin/responses/friendslist.json` |
| OAuth + login page | `/account/api/oauth/token`, `GET /login` | `owen-server` native |
| PartyHub web/bridge | `/partyhub/*` | `owen-server/src/partyhub.rs` native |

What OGS-12.41 proves is the part **no HTTP server can provide**: bots,
bosses, booths and quest *gameplay* need code running inside the game process
(the same conclusion our `owen-redirecter`/Erbium native work reaches). The
two data files it *does* ship are mirrored here because our stack can adopt
them.

## Files in this folder

| File | What it is | Where it should be adopted |
|---|---|---|
| `BotNames.h` | Original header — `std::vector<std::string> BotDisplayNames`, 100 community bot display names (kept verbatim, original name) | reference / source of truth |
| `bot-names.json` | Derived JSON of the same 100 names | `owen-server` `friends` + PartyHub presence: a `GET /friends/api/v1/:id/friends` variant that returns a synthetic roster (fake "online" players), and `data/lawin/responses/friendslist.json` refresh |
| `POI_Locs.h` | Original header — `std::vector<FVector> DropZoneLocations`, 47 hand-mapped 12.41 (C2S2 Apollo) drop-zone coordinates | reference / source of truth |
| `poi-dropzones.json` | Derived JSON of the same 47 zones `{name,x,y,z}` | a future `GET /api/launcher/seasons/12/pois` (or creative-discovery payload) so any client-side bot/feature mod we ship can fetch drop zones instead of hardcoding them |

Licensing/attribution: upstream is community-made gameplay data published on
GitHub with no license file; mirrored here for interoperability study with
attribution in each derived JSON's `source` field. Nothing here is Epic
content — the names are the mod authors' inventions, the coordinates are
hand-mapped by the mod authors.
