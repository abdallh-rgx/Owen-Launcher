// =============================================================================
// ogs_all.cpp — the OGS 12.41 mod ported to Android (single TU).
//
// Source files (kept verbatim under cpp/orig/ for reference) and the porting
// transforms applied here (see the task report for the full mapping table):
//   orig/dllmain.cpp            → ogs_main() at the bottom of this file
//   orig/framework.h            → OgsFn registry + helpers below (dissolved)
//   orig/Globals.h              → namespace Globals (config-driven)
//   orig/BotNames.h             → BotDisplayNames (byte-faithful)
//   orig/POI_Locs.h             → DropZoneLocations (byte-faithful)
//   orig/LateGameLoot.h         → weapon pools (byte-faithful)
//   orig/BehaviourTree_System.h → BT framework classes
//   orig/BT_MANG2.h             → namespace BT_MANG2
//   orig/Bosses.h               → FactionBot + Bosses
//   orig/PlayerBots.h           → PlayerBot + PlayerBots
//   orig/Bots.h                 → namespace Bots
//   orig/Inventory.h            → namespace Inventory
//   orig/Looting.h              → namespace Looting
//   orig/Building.h             → namespace Building
//   orig/Abilities.h            → namespace Abilities
//   orig/Quests.h               → namespace Quests
//   orig/Vehicles.h             → namespace Vehicles
//   orig/PhantomBoothSpawner.h  → namespace PhantomBoothSpawner
//   orig/Creative.h             → namespace Creative
//   orig/PE.h                   → namespace PE
//   orig/Misc.h                 → namespace Misc
//   orig/Net.h                  → namespace Net
//   orig/Tick.h                 → namespace Tick + OgsStandaloneTick thread
//   orig/GameMode.h             → namespace GameMode
//   orig/PC.h                   → namespace PC
//
// Global transforms (applied to EVERYTHING):
//   * __fastcall/__int64 stripped (plain types, <cstdint> int64_t)
//   * L"..."                    → OGS_TEXT("...")
//   * TEXT("...")               → OGS_TEXT("...")
//   * Sleep(x)                  → Ogs::SleepMs(x)
//   * _ReturnAddress()          → OGS_RETURN_ADDRESS()
//   * ImageBase+0x...           → Ogs::Off() registry keys
//   * MultiByteToWideChar       → Ogs::Text
//   * MH_CreateHook             → OGS_HOOK_NATIVE_OFF / OgsHook::NativeOff
//   * HookVTable(CDO, idx, ...) → OgsHook::ByExec(CDO, class, func, ...) /
//                                 OgsHook::ByIndex(CDO, cfg vt index, ...)
//   * ExecHook                  → OgsHook::ExecSwap (gated by features.exec_hooks)
//   * windows.h/tlhelp32/intrin are gone; MinHook is gone.
//   * every SDK pointer deref is guarded (OGS_CHECK) — a broken optional
//     feature logs and continues, it must not kill the mod.
// =============================================================================

#include "OgsCompat.h"
#include "ogshook.h"

#include <string>
#include <vector>
#include <map>
#include <set>
#include <array>
#include <thread>
#include <mutex>
#include <chrono>
#include <algorithm>
#include <numeric>
#include <sstream>
#include <random>
#include <functional>
#include <atomic>
#include <ctime>
#include <cmath>
#include <fstream>

using namespace SDK;

// Greppable phase markers (spec):
//   OGS: phase=login_wait / phase=hooks / phase=flip / phase=loadworld
//   OGS: match world up

// =============================================================================
// Phase-check macro — "null → log + abort the phase cleanly".  Use in void
// functions; value-returning paths use explicit null checks.
// =============================================================================
#define OGS_CHECK(x)                                                            \
        do                                                                          \
        {                                                                           \
                if (!(x))                                                               \
                {                                                                       \
                        ::Ogs::Log("OGS: check failed: %s (%s:%d)", #x, __FILE__, __LINE__); \
                        return;                                                             \
                }                                                                       \
        } while (0)

// std::string overload of the original Log(std::string) from framework.h.
inline void Log(const std::string& msg) { ::Ogs::Log("%s", msg.c_str()); }
inline void Log(const char* msg) { ::Ogs::Log("%s", msg); }

// =============================================================================
// OgsSDK — the global "Statics/Math/GameState/Gamemode/..." helpers the
// original initialized at DLL load (Bosses.h namespace-scope auto globals).
// Those would crash at .so load on Android (no world yet); these evaluate
// lazily on every call, exactly like the original's per-function locals
// (auto Statics = (UGameplayStatics*)...DefaultObject).
// =============================================================================
namespace OgsSDK
{
        UGameplayStatics* Statics() { return (UGameplayStatics*)UGameplayStatics::StaticClass()->DefaultObject; }
        UKismetMathLibrary* Math() { return (UKismetMathLibrary*)UKismetMathLibrary::StaticClass()->DefaultObject; }
        UKismetStringLibrary* StringLib() { return (UKismetStringLibrary*)UKismetStringLibrary::StaticClass()->DefaultObject; }
        UKismetSystemLibrary* SystemLib() { return (UKismetSystemLibrary*)UKismetSystemLibrary::StaticClass()->DefaultObject; }
        AFortGameStateAthena* GameState() { UWorld* w = UWorld::GetWorld(); return w ? (AFortGameStateAthena*)w->GameState : nullptr; }
        AFortGameModeAthena* Gamemode() { UWorld* w = UWorld::GetWorld(); return w ? (AFortGameModeAthena*)w->AuthorityGameMode : nullptr; }
}

// =============================================================================
// Globals (orig/Globals.h) — now driven by ogsen.json via Ogs::Cfg.
// =============================================================================
namespace Globals
{
        bool bIsProdServer = false;

        bool bCreativeEnabled = false;
        bool bSTWEnabled = false;
        bool bEventEnabled = false;

        bool bBossesEnabled = true;
        bool bBotsEnabled = true;

        bool bUseLegacyAI_MANG = true; // Keep this on true bro, I am NOT doing this!

        bool LateGame = false;
        bool Automatics = true;
        bool BattleLab = false;
        bool Blitz = false;
        bool StormKing = false;
        bool Arsenal = false;
        bool TeamRumble = false;
        bool SolidGold = false;
        bool UnVaulted = false;
        bool Siphon = false;
        bool Arena = false;

        int MaxBotsToSpawn = 100;
        int MinPlayersForEarlyStart = 100;

        void InitFromCfg()
        {
                const Ogs::Cfg& cfg = Ogs::CfgRef();
                bCreativeEnabled = cfg.Has("creative") || cfg.Has("stw");
                bSTWEnabled = cfg.Has("stw");
                bEventEnabled = cfg.Has("event");
                bBossesEnabled = cfg.Has("bosses");
                bBotsEnabled = cfg.Has("bots");
                bUseLegacyAI_MANG = cfg.Has("legacy_ai_mang");
                LateGame = cfg.Has("late_game");
                Automatics = cfg.Has("automatics");
                BattleLab = cfg.Has("battle_lab");
                Blitz = cfg.Has("blitz");
                StormKing = cfg.Has("storm_king");
                Arsenal = cfg.Has("arsenal");
                TeamRumble = cfg.Has("team_rumble");
                SolidGold = cfg.Has("solid_gold");
                UnVaulted = cfg.Has("unvaulted");
                Siphon = cfg.Has("siphon");
                Arena = cfg.Has("arena");
                MaxBotsToSpawn = (int)cfg.Num("max_bots");
                MinPlayersForEarlyStart = (int)cfg.Num("min_players_early_start");
        }
}

// =============================================================================
// BotDisplayNames (orig/BotNames.h) — byte-faithful.
// =============================================================================
inline std::vector<std::string> BotDisplayNames = {
        "ObsessedTech.",
        "Timeless.",
        "Grampa_Joe",
        "Agua_Cat",
        "AGenuineHuman",
        "NooooMyBueno",
        "Pongo_x8.6",
        "xx-AnimeGirl-xx",
        "90cranker",
        "ColonelChugJug",
        "Gatordile81",
        "JonseyForever35",
        "PositiveFeels",
        "TimeToGo80",
        "QueenBeet74",
        "AimLikeIdaho",
        "CrazyPea96",
        "GetItGotItGood",
        "JustABitEpic",
        "PrancingPwnee",
        "TooManyBeets",
        "MintElephant26",
        "AngryDuck51",
        "CrepeSalad",
        "GhostChicken12",
        "KittyCat80",
        "PrinceWombat",
        "WalkInThePark66",
        "BliceCake",
        "AthenaOrApollo",
        "DoctorLobby92",
        "Gooddoggo80",
        "Kregore73",
        "SergentSummer",
        "WildCactusBob",
        "BraunyBanana",
        "AtTheBeach321",
        "DoubleDaring",
        "Goosezilla13",
        "LetsBePals23",
        "ShadowArrow58",
        "Wondertail",
        "SoggyCookie26",
        "BagelBoy82",
        "DoubleDuel75",
        "Grandma40",
        "LewtGoblin7",
        "Shepard52",
        "Yeetman57",
        "AboveMule633",
        "BellyFlop40",
        "DoubleRainbow96",
        "HashtagToad57",
        "McCucumber71",
        "ShieldHorse63",
        "Beebitme",
        "PurpleCrayon85",
        "Blackjack31",
        "DrPlanet",
        "HeliumHog",
        "Meshuncle",
        "ShootyMcGee40",
        "SweetPenguin16",
        "SpiffyPowder6866",
        "BlinkImGone44",
        "DrumGunnar",
        "HeyThereFriend81",
        "Mouthful95",
        "SilverySilver",
        "PortableOx",
        "LootTrooper51",
        "BoatingIsLife",
        "ElPollo85",
        "Hoodwinked12",
        "N0nDa1ry",
        "SirTricksALot21",
        "A_Sweaty_Dog",
        "LousyCentaur",
        "BoldPrediction",
        "FlavorCaptain",
        "HotelBlankets",
        "NotAPalidome",
        "SteelGoose18",
        "&darkBeast&",
        "Notapiggy",
        "BrainInvader",
        "FlimsyGoat",
        "HowAreMy90s",
        "Number141",
        "TAgYOuRIt9",
        "BobDobaleena",
        "CactusDad80",
        "FlossPatrol82",
        "iHazHighGround",
        "ParanoidCactus",
        "ThermalDragon39",
        "OldWaterBottle28",
        "LazarLazar.",
        "SypherPK.",
        "Tejo-TV.",
};

// =============================================================================
// DropZoneLocations (orig/POI_Locs.h) — byte-faithful.
// =============================================================================
std::vector<FVector> DropZoneLocations = {
        { 106902, -84901, -1834 }, // The Shark (Complete)
        { 107834, -78584, -783 }, // Top of The Shark
        { 120009, -84032, -3370 }, // The Shark Prison
        { 112255, -91220, -3011 }, // The Shark Choppa Spawn
        { 99820, -82200, -3370 }, // The Shark Basketball Court

        { 96764, 29166, -2226 }, // Craggy Cliffs
        { 104078, 26670, -1834 }, // Fishsticks restruant

        { 94032, 51513, 4693 }, // Shadow RadioTower thing idk

        { 76334, 90345, -1450 }, // Steamy Stacks
        { 81669, 90762, 469 }, // Kevolution Energy
        { 80258, 95433, 69 },
        { 86691, 102076, 69 },

        { 113312, 113547, -1837 }, // The Yacht (Complete)
        { 116567, 113665, -2602 }, // The Yacht
        { 109895, 113636, -2986 }, // The Yacht
        { 106233, 108428, -3762 }, // The Yacht Island

        { 64240, -16323, 69 }, // Pleasant Park
        { 60290, -16240, 69 }, // PP Shadow Henchmen Base

        { 31257, -77599, 69 }, // Sweaty Sands

        { 13637, -24219, 69 }, // Salty Springs

        { 30808, 10669, 69 }, // Risky Reels

        { 30536, 42295, 69 }, // Frenzy Farm

        { 30437, 69691, 69 }, // Shadow Henchmen Gas Station

        { 17963, 112955, 69 }, // Dirty Docks

        { 6364, 4866, 69 }, // Agency
        { 6426, 7100, 69 },
        { 6245, 1741, 69 },
        { 11188, 4863, 69 },
        { 657, 4334, 69 },
        { 4084, 7167, 69 },

        { -13311, -81033, 69 }, // Holly Hedges

        { -28997, -31787, 69 }, // Weeping Woods

        { -64881, -46495, 69 }, // Slurpy Swamp

        { -77143, -80634, 69 }, // The Rig
        { -73305, -90910, 69 },

        { -88282, 23038, 69 }, // Misty Meadows
        { -88329, 31859, 69 },

        { -68041, 29906, 69 }, // Misty henchmen house

        { -43937, 52030, 69 }, // Lazy Lake
        { -54818, 57400, 69 },

        { -68550, 80804, 69 }, // Catty Corner

        { -92971, 78709, 69 }, // Mountain henchmen base

        { -39867, 91323, 69 }, // Retail Row
        { -38750, 102176, 69 },

        { -19544, 105594, 69 }, // Grotto
        { -17383, 112993, 69 },
        { -22500, 112479, 69 },
};

// =============================================================================
// Late-game weapon pools (orig/LateGameLoot.h) — byte-faithful (including the
// original's missing comma quirk, folded into adjacent entries verbatim).
// =============================================================================
std::vector<std::string> Assault_rifle = {
        "/Game/Athena/Items/Weapons/WID_Assault_AutoHigh_Athena_VR_Ore_T03.WID_Assault_AutoHigh_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_AutoHigh_Athena_SR_Ore_T03.WID_Assault_AutoHigh_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_SemiAuto_Athena_VR_Ore_T03.WID_Assault_SemiAuto_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Heavy_Athena_VR_Ore_T03.WID_Assault_Heavy_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Heavy_Athena_SR_Ore_T03.WID_Assault_Heavy_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Heavy_Athena_R_Ore_T03.WID_Assault_Heavy_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Surgical_Athena_R_Ore_T03.WID_Assault_Surgical_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Suppressed_Athena_VR_Ore_T03.WID_Assault_Suppressed_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Suppressed_Athena_SR_Ore_T03.WID_Assault_Suppressed_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_PistolCaliber_AR_Athena_VR_Ore_T03.WID_Assault_PistolCaliber_AR_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_PistolCaliber_AR_Athena_SR_Ore_T03.WID_Assault_PistolCaliber_AR_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_PistolCaliber_AR_Athena_R_Ore_T03.WID_Assault_PistolCaliber_AR_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Suppressed_Athena_R_Ore_T03.WID_Assault_Suppressed_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/Boss/WID_Boss_Adventure_AR.WID_Boss_Adventure_AR",
        "/Game/Athena/Items/Weapons/WID_Assault_Surgical_Athena_VR_Ore_T03.WID_Assault_Surgical_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Surgical_Athena_SR_Ore_T03.WID_Assault_Surgical_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Surgical_Thermal_Athena_VR_Ore_T03.WID_Assault_Surgical_Thermal_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Surgical_Thermal_Athena_SR_Ore_T03.WID_Assault_Surgical_Thermal_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_SemiAuto_Athena_SR_Ore_T03.WID_Assault_SemiAuto_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_SemiAuto_Athena_R_Ore_T03.WID_Assault_SemiAuto_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/Boss/WID_Boss_Meow_HA.WID_Boss_Meow_HA",
        "/Game/Athena/Items/Weapons/WID_Assault_Auto_Athena_R_Ore_T03.WID_Assault_Auto_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_LMGSAW_Athena_VR_Ore_T03.WID_Assault_LMGSAW_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_LMGSAW_Athena_R_Ore_T03.WID_Assault_LMGSAW_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_Infantry_Athena_VR.WID_Assault_Infantry_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Assault_Infantry_Athena_R.WID_Assault_Infantry_Athena_R",
        "/Game/Athena/Items/Weapons/WID_Assault_Infantry_Athena_SR.WID_Assault_Infantry_Athena_SR"
};

std::vector<std::string> Shotgun = {
        "/Game/Athena/Items/Weapons/WID_Shotgun_BreakBarrel_Athena_VR_Ore_T03.WID_Shotgun_BreakBarrel_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_Combat_Athena_VR_Ore_T03.WID_Shotgun_Combat_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_Combat_Athena_SR_Ore_T03.WID_Shotgun_Combat_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_Combat_Athena_R_Ore_T03.WID_Shotgun_Combat_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_SlugFire_Athena_VR.WID_Shotgun_SlugFire_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Shotgun_SlugFire_Athena_SR.WID_Shotgun_SlugFire_Athena_SR",
        "/Game/Athena/Items/Weapons/WID_Shotgun_SlugFire_Athena_R.WID_Shotgun_SlugFire_Athena_R",
        "/Game/Athena/Items/Weapons/WID_Shotgun_SemiAuto_Athena_VR_Ore_T03.WID_Shotgun_SemiAuto_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_HighSemiAuto_Athena_VR_Ore_T03.WID_Shotgun_HighSemiAuto_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_HighSemiAuto_Athena_SR_Ore_T03.WID_Shotgun_HighSemiAuto_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_AutoDrum_Athena_R_Ore_T03.WID_Shotgun_AutoDrum_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_BreakBarrel_Athena_SR_Ore_T03.WID_Shotgun_BreakBarrel_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_Standard_Athena_VR_Ore_T03.WID_Shotgun_Standard_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Shotgun_Standard_Athena_SR_Ore_T03.WID_Shotgun_Standard_Athena_SR_Ore_T03",
};

std::vector<std::string> Mixed = {
        "/Game/Athena/Items/Weapons/WID_Sniper_Auto_Suppressed_Scope_Athena_VR.WID_Sniper_Auto_Suppressed_Scope_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Pistol_Donut.WID_Pistol_Donut",
        "/Game/Athena/Items/Weapons/Boss/WID_Boss_Adventure_GH.WID_Boss_Adventure_GH",
        "/Game/Athena/Items/Weapons/WID_Pistol_BurstFireSMG_Athena_R_Ore_T03.WID_Pistol_BurstFireSMG_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_BoltAction_Scope_Athena_VR_Ore_T03.WID_Sniper_BoltAction_Scope_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavyPDW_Athena_VR_Ore_T03.WID_Pistol_AutoHeavyPDW_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/Seasonal/WID_Sniper_Valentine_Athena_VR_Ore_T03.WID_Sniper_Valentine_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavyPDW_Athena_SR_Ore_T03.WID_Pistol_AutoHeavyPDW_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_DualPistol_Suppresseed_Athena_VR_T01.WID_DualPistol_Suppresseed_Athena_VR_T01",
        "/Game/Athena/Items/Weapons/WID_DualPistol_Suppresseed_Athena_R_T01.WID_DualPistol_Suppresseed_Athena_R_T01",
        "/Game/Athena/Items/Weapons/WID_Sniper_BoltAction_Scope_Athena_SR_Ore_T03.WID_Sniper_BoltAction_Scope_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavy_Athena_R_Ore_T03.WID_Pistol_AutoHeavy_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Rocket_Athena_VR_Ore_T03.WID_Launcher_Rocket_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Rocket_Athena_SR_Ore_T03.WID_Launcher_Rocket_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Rocket_Athena_R_Ore_T03.WID_Launcher_Rocket_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/Seasonal/WID_Launcher_Pumpkin_Athena_VR_Ore_T03.WID_Launcher_Pumpkin_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/Seasonal/WID_Launcher_Pumpkin_Athena_SR_Ore_T03.WID_Launcher_Pumpkin_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/Seasonal/WID_Launcher_Pumpkin_Athena_R_Ore_T03.WID_Launcher_Pumpkin_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavySuppressed_Athena_R_Ore_T03.WID_Pistol_AutoHeavySuppressed_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavySuppressed_Athena_VR_Ore_T03.WID_Pistol_AutoHeavySuppressed_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavySuppressed_Athena_SR_Ore_T03.WID_Pistol_AutoHeavySuppressed_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_BoltAction_Scope_Athena_R_Ore_T03.WID_Sniper_BoltAction_Scope_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Flintlock_Athena_C.WID_Pistol_Flintlock_Athena_C",
        "/Game/Athena/Items/Weapons/WID_DualPistol_SemiAuto_Athena_VR_Ore_T03.WID_DualPistol_SemiAuto_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_LMG_Athena_VR_Ore_T03.WID_Assault_LMG_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_LMG_Athena_SR_Ore_T03.WID_Assault_LMG_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_LMG_Athena_R_Ore_T03.WID_Assault_LMG_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Scavenger_Athena_VR_Ore_T03.WID_Pistol_Scavenger_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Scavenger_Athena_R_Ore_T03.WID_Pistol_Scavenger_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Scavenger_Athena_SR_Ore_T03.WID_Pistol_Scavenger_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Standard_Scope_Athena_VR_Ore_T03.WID_Sniper_Standard_Scope_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Standard_Scope_Athena_VeryRare_Ore_T03.WID_Sniper_Standard_Scope_Athena_VeryRare_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Standard_Scope_Athena_SuperRare_Ore_T03.WID_Sniper_Standard_Scope_Athena_SuperRare_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Standard_Scope_Athena_SR_Ore_T03.WID_Sniper_Standard_Scope_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_SemiAuto_Athena_VR_Ore_T03.WID_Pistol_SemiAuto_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Suppressed_Scope_Athena_VR_Ore_T03.WID_Sniper_Suppressed_Scope_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Suppressed_Scope_Athena_SR_Ore_T03.WID_Sniper_Suppressed_Scope_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Suppressed_Scope_Athena_R_Ore_T03.WID_Sniper_Suppressed_Scope_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_SemiAuto_Athena_SR_Ore_T03.WID_Pistol_SemiAuto_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Assault_AutoDrum_Athena_R_Ore_T03.WID_Assault_AutoDrum_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_SemiAuto_Suppressed_Athena_SR_Ore_T03.WID_Pistol_SemiAuto_Suppressed_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Military_Athena_VR_Ore_T03.WID_Launcher_Military_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Military_Athena_SR_Ore_T03.WID_Launcher_Military_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_AutoHeavyPDW_Athena_R_Ore_T03.WID_Pistol_AutoHeavyPDW_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Athena_SMG_VR.WID_Athena_SMG_VR",
        "/Game/Athena/Items/Weapons/WID_Athena_SMG_SR.WID_Athena_SMG_SR",
        "/Game/Athena/Items/Gameplay/Boss/AGID_Boss_Tina.AGID_Boss_Tina",
        "/Game/Athena/Items/Weapons/WID_Launcher_Grenade_Athena_VR_Ore_T03.WID_Launcher_Grenade_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Revolver_SingleAction_Athena_VR.WID_Pistol_Revolver_SingleAction_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Pistol_Revolver_SingleAction_Athena_R.WID_Pistol_Revolver_SingleAction_Athena_R",
        "/Game/Athena/Items/Weapons/WID_Launcher_Grenade_Athena_SR_Ore_T03.WID_Launcher_Grenade_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Launcher_Grenade_Athena_R_Ore_T03.WID_Launcher_Grenade_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_DualPistol_SemiAuto_Athena_SR_Ore_T03.WID_DualPistol_SemiAuto_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_BoltAction_Scope_Athena_C_Ore_T03.WID_Sniper_BoltAction_Scope_Athena_C_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Auto_Suppressed_Scope_Athena_SR.WID_Sniper_Auto_Suppressed_Scope_Athena_SR",
        "/Game/Athena/Items/Weapons/Boss/WID_Boss_Hos_MG.WID_Boss_Hos_MG",
        "/Game/Athena/Items/Weapons/WID_Sniper_Auto_Suppressed_Scope_Athena_R.WID_Sniper_Auto_Suppressed_Scope_Athena_R",
        "/Game/Athena/Items/Weapons/LTM/WID_Hook_Gun_Slide.WID_Hook_Gun_Slide",
        "/Game/Athena/Items/Weapons/WID_Sniper_Heavy_Athena_VR_Ore_T03.WID_Sniper_Heavy_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Heavy_Athena_SR_Ore_T03.WID_Sniper_Heavy_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Sniper_Heavy_Athena_R_Ore_T03.WID_Sniper_Heavy_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_HandCannon_Athena_VR_Ore_T03.WID_Pistol_HandCannon_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_HandCannon_Athena_SR_Ore_T03.WID_Pistol_HandCannon_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_GrenadeLauncher_Prox_Athena_VR.WID_GrenadeLauncher_Prox_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Pistol_Scoped_Athena_VR_Ore_T03.WID_Pistol_Scoped_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Scoped_Athena_SR_Ore_T03.WID_Pistol_Scoped_Athena_SR_Ore_T03"
        "/Game/Athena/Items/Weapons/WID_GrenadeLauncher_Prox_Athena_SR.WID_GrenadeLauncher_Prox_Athena_SR",
        "/Game/Athena/Items/Weapons/WID_Sniper_NoScope_Athena_R_Ore_T03.WID_Sniper_NoScope_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/Boss/WID_Boss_Midas.WID_Boss_Midas",
        "/Game/Athena/Items/Weapons/WID_Pistol_Standard_Athena_VR.WID_Pistol_Standard_Athena_VR",
        "/Game/Athena/Items/Weapons/WID_Pistol_Standard_Athena_SR.WID_Pistol_Standard_Athena_SR",
        "/Game/Athena/Items/Weapons/WID_Pistol_SemiAuto_Athena_R_Ore_T03.WID_Pistol_SemiAuto_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_RapidFireSMG_Athena_R_Ore_T03.WID_Pistol_RapidFireSMG_Athena_R_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_RapidFireSMG_Athena_VR_Ore_T03.WID_Pistol_RapidFireSMG_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_RapidFireSMG_Athena_SR_Ore_T03.WID_Pistol_RapidFireSMG_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_SixShooter_Athena_VR_Ore_T03.WID_Pistol_SixShooter_Athena_VR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_Sixshooter_Athena_SR_Ore_T03.WID_Pistol_SixShooter_Athena_SR_Ore_T03",
        "/Game/Athena/Items/Weapons/WID_Pistol_SixShooter_Athena_R_Ore_T03.WID_Pistol_SixShooter_Athena_R_Ore_T03",
};

std::vector<std::string> Consumables = {
        "/Game/Athena/Items/Consumables/Shields/Athena_Shields.Athena_Shields",
        "/Game/Athena/Items/Consumables/ShieldSmall/Athena_ShieldSmall.Athena_ShieldSmall",
        "/Game/Athena/Items/Consumables/Balloons/Athena_Balloons_Consumable.Athena_Balloons_Consumable",
        "/Game/Athena/Items/Consumables/Bandage/Athena_Bandage.Athena_Bandage",
        "/Game/Athena/Items/Consumables/DanceGrenade/Athena_DanceGrenade.Athena_DanceGrenade",
        "/Game/Athena/Items/Consumables/Bush/Athena_Bush.Athena_Bush",
        "/Game/Athena/Items/Consumables/ChillBronco/Athena_ChillBronco.Athena_ChillBronco",
        "/Game/Athena/Items/Consumables/StickyGrenade/Athena_StickyGrenade.Athena_StickyGrenade",
        "/Game/Athena/Items/Consumables/ShockwaveGrenade/Athena_ShockGrenade.Athena_ShockGrenade",
        "/Game/Athena/Items/Consumables/AppleSun/WID_Athena_AppleSun.WID_Athena_AppleSun",
        "/Game/Athena/Items/Consumables/DangerGrape/WID_Athena_DangerGrape.WID_Athena_DangerGrape",
        "/Game/Athena/Items/Consumables/TNT/Athena_TNT.Athena_TNT",
        "/Game/Athena/Items/Consumables/Flopper/WID_Athena_Flopper.WID_Athena_Flopper",
        "/Game/Athena/Items/Consumables/DeployableStorm/Athena_DogSweater.Athena_DogSweater",
        "/Game/Athena/Items/Consumables/GasGrenade/Athena_GasGrenade.Athena_GasGrenade",
        "/Game/Athena/Items/Consumables/Grenade/Athena_Grenade.Athena_Grenade",
        "/Game/Athena/Items/Consumables/PurpleStuff/Athena_PurpleStuff.Athena_PurpleStuff",
        "/Game/Athena/Items/Consumables/KnockGrenade/Athena_KnockGrenade.Athena_KnockGrenade",
        "/Game/Athena/Items/Consumables/Medkit/Athena_Medkit.Athena_Medkit",
        "/Game/Athena/Items/Consumables/TowerGrenade/Athena_TowerGrenade.Athena_TowerGrenade",
        "/Game/Athena/Items/Consumables/SmokeGrenade/Athena_SmokeGrenade.Athena_SmokeGrenade",
        "/Game/Athena/Items/Consumables/SuperTowerGrenade/Levels/PortAFort_A/Athena_SuperTowerGrenade_A.Athena_SuperTowerGrenade_A",
        "/Game/Athena/Items/Weapons/Prototype/PurpleMouse/WID_Athena_PurpleMouse.WID_Athena_PurpleMouse",
        "/Game/Athena/Items/Consumables/C4/Athena_C4.Athena_C4",
        "/Game/Athena/Items/Consumables/RiftItem/Athena_Rift_Item.Athena_Rift_Item",
        "/Game/Athena/Items/Consumables/Flopper/Effective/WID_Athena_Flopper_Effective.WID_Athena_Flopper_Effective",
        "/Game/Athena/Items/Consumables/SilverBlazer/Athena_SilverBlazer_V2.Athena_SilverBlazer_V2",
};

std::vector<std::string> Traps = {
        "/Game/Athena/Items/Traps/TID_Context_BouncePad_Athena.TID_Context_BouncePad_Athena",
        "/Game/Athena/Items/Traps/TID_ContextTrap_Athena.TID_ContextTrap_Athena",
        "/Game/Items/Traps/Floor/TID_Floor_Player_Launch_Pad_Athena.TID_Floor_Player_Launch_Pad_Athena",
};

// =============================================================================
// framework.h — dissolved into this TU.  The raw ImageBase+0x... function
// pointer table became the typed OgsFn registry accessors (all currently
// found:false — every call site null-checks and logs "skipped (offset
// missing)").  static_find_object is REPLACED by the SDK's own
// UObject::FindObject<T>() helper (Basic.hpp) and needs no offset.
// =============================================================================

// --- text manipulation utils -------------------------------------------------
namespace TextManipUtils
{
        // Found this from stack overflow :fire:
        std::vector<std::string> SplitWhitespace(std::string const& input)
        {
                std::istringstream buffer(input);
                std::vector<std::string> ret;

                std::copy(std::istream_iterator<std::string>(buffer),
                        std::istream_iterator<std::string>(),
                        std::back_inserter(ret));
                return ret;
        }
}

// --- the OgsFn registry (typed accessors over Ogs::Off()) --------------------
namespace OgsFn
{
        // UNetDriver related (listen_server feature — all found:false today)
        using CreateNetDriverFn = UNetDriver * (*)(void*, void*, FName);
        using InitListenFn = bool (*)(void*, void*, FURL&, bool, FString&);
        using SetWorldFn = void (*)(void*, void*);
        using ServerReplicateActorsFn = void (*)(void*);

        // GameplayAbilities
        using GiveAbilityFn = void (*)(UAbilitySystemComponent*, FGameplayAbilitySpecHandle*, FGameplayAbilitySpec);
        using AbilitySpecConstructorFn = void (*)(FGameplayAbilitySpec*, UGameplayAbility*, int, int, UObject*);
        using InternalTryActivateAbilityFn = bool (*)(UAbilitySystemComponent*, FGameplayAbilitySpecHandle, FPredictionKey, UGameplayAbility**, void*, const FGameplayEventData*);
        using GiveAbilityAndActivateOnceFn = FGameplayAbilitySpecHandle (*)(UAbilitySystemComponent*, FGameplayAbilitySpecHandle*, FGameplayAbilitySpec);

        // World / building
        using PickSupplyDropLocationFn = FVector * (*)(AFortAthenaMapInfo*, FVector*, int64_t, float);
        using ReplaceBuildingActorFn = ABuildingSMActor * (*)(ABuildingSMActor*, unsigned int, UObject*, unsigned int, int, bool, AFortPlayerControllerAthena*);
        using CantBuildFn = int64_t (*)(UWorld*, UObject*, FVector, FRotator, char, void*, char*);

        // Character / bots
        using ApplyCharacterCustomizationFn = void* (*)(void*, void*);
        using BotManagerSetupFn = void (*)(int64_t, int64_t, int64_t, int64_t, uint32_t*, int64_t, int64_t, int64_t, int64_t, uint8_t*, uint8_t, uint8_t*, FFortAthenaAIBotRunTimeCustomizationData);
        using RegisterComponentWithWorldFn = char (*)(UObject*, UObject*);
        using RemoveFromAlivePlayersFn = void (*)(AFortGameModeAthena*, AFortPlayerControllerAthena*, APlayerState*, AFortPlayerPawn*, UFortWeaponItemDefinition*, uint8_t, char);
        using AddToAlivePlayersFn = void (*)(AFortGameModeAthena*, AFortPlayerControllerAthena*);

        // Object loading (find is the SDK's FindObject; load stays registry-gated)
        using StaticLoadObjectFn = void* (*)(UClass*, UObject*, const void*, const void*, uint32_t, UObject*, bool, void*);

        // FFrame stepping
        using FFrameStepAFn = void (*)(void*, UObject*, void* const);
        using FFrameStepBFn = void (*)(void*, void* const, FField*);

        // Interface / gadget plumbing
        using GetInterfaceFn = void* (*)(UObject*, UClass*);
        using GadgetAddFn = bool (*)(UFortGadgetItemDefinition*, void*, UFortWorldItem*, uint8_t);
        using GadgetRemoveFn = bool (*)(UFortGadgetItemDefinition*, void*, UFortWorldItem*);

#define OGSFN_DEFINE(name, Type)                                                    \
        inline Type name()                                                             \
        {                                                                              \
                static Type cached = nullptr;                                              \
                static bool resolved = false;                                              \
                if (!resolved)                                                             \
                {                                                                          \
                        resolved = true;                                                       \
                        const ::Ogs::OffEntry& e = ::Ogs::Off().name;                          \
                        if (e.Found)                                                           \
                                cached = reinterpret_cast<Type>(::Ogs::Resolve(e.Rva));            \
                }                                                                          \
                return cached;                                                             \
        }

        OGSFN_DEFINE(create_netdriver, CreateNetDriverFn)
        OGSFN_DEFINE(init_listen, InitListenFn)
        OGSFN_DEFINE(set_world, SetWorldFn)
        OGSFN_DEFINE(server_replicate_actors, ServerReplicateActorsFn)
        OGSFN_DEFINE(give_ability, GiveAbilityFn)
        OGSFN_DEFINE(ability_spec_constructor, AbilitySpecConstructorFn)
        OGSFN_DEFINE(internal_try_activate_ability, InternalTryActivateAbilityFn)
        OGSFN_DEFINE(give_ability_and_activate_once, GiveAbilityAndActivateOnceFn)
        OGSFN_DEFINE(pick_supply_drop_location, PickSupplyDropLocationFn)
        OGSFN_DEFINE(replace_building_actor, ReplaceBuildingActorFn)
        OGSFN_DEFINE(cant_build, CantBuildFn)
        OGSFN_DEFINE(apply_character_customization, ApplyCharacterCustomizationFn)
        OGSFN_DEFINE(bot_manager_setup, BotManagerSetupFn)
        OGSFN_DEFINE(register_component_with_world, RegisterComponentWithWorldFn)
        OGSFN_DEFINE(remove_from_alive_players, RemoveFromAlivePlayersFn)
        OGSFN_DEFINE(add_to_alive_players, AddToAlivePlayersFn)
        OGSFN_DEFINE(static_load_object, StaticLoadObjectFn)
        OGSFN_DEFINE(fframe_step_a, FFrameStepAFn)
        OGSFN_DEFINE(fframe_step_b, FFrameStepBFn)
        OGSFN_DEFINE(get_interface, GetInterfaceFn)
        OGSFN_DEFINE(gadget_add, GadgetAddFn)
        OGSFN_DEFINE(gadget_remove, GadgetRemoveFn)
#undef OGSFN_DEFINE
} // namespace OgsFn

// --- shared mod state (framework.h globals) ----------------------------------
AFortAthenaMutator_Bots* BotMutator = nullptr;
TArray<FVector> PickedSupplyDropLocations;
TArray<APlayerController*> GivenLootPlayers;

static TArray<AActor*> PlayerStarts;

bool bFirstElimTriggered = false;
bool bFirstEliminated = false;
bool DontPlayAnimation = false;

bool bFirstChestSearched = false;
bool bFirstSupplyDropSearched = false;

// --- FFrame (framework.h) — StepCompiledIn goes through OgsFn ----------------
class FOutputDevice
{
public:
        bool bSuppressEventTag;
        bool bAutoEmitLineTerminator;
        uint8_t _Padding1[0x6];
};

class FFrame : public FOutputDevice
{
public:
        void** VTable;
        UFunction* Node;
        UObject* Object;
        uint8* Code;
        uint8* Locals;
        void* MostRecentProperty;
        uint8_t* MostRecentPropertyAddress;
        uint8_t _Padding1[0x40];
        FField* PropertyChainForCompiledIn;

public:
        void StepCompiledIn(void* const Result, bool ForceExplicitProp = false)
        {
                auto stepA = OgsFn::fframe_step_a();
                auto stepB = OgsFn::fframe_step_b();
                if (!stepA || !stepB)
                {
                        OGS_LOG("FFrame::StepCompiledIn skipped (offset missing)");
                        return;
                }

                if (Code && !ForceExplicitProp)
                {
                        stepA(this, Object, Result);
                }
                else
                {
                        FField* _Prop = PropertyChainForCompiledIn;
                        if (!_Prop)
                                return;
                        PropertyChainForCompiledIn = _Prop->Next;
                        stepB(this, Result, _Prop);
                }
        }

        template <typename T>
        T& StepCompiledInRef()
        {
                static T TempVal{};
                MostRecentPropertyAddress = nullptr;

                auto stepA = OgsFn::fframe_step_a();
                auto stepB = OgsFn::fframe_step_b();
                if (stepA && Code)
                {
                        stepA(this, Object, &TempVal);
                }
                else if (stepB && PropertyChainForCompiledIn)
                {
                        FField* _Prop = PropertyChainForCompiledIn;
                        PropertyChainForCompiledIn = _Prop->Next;
                        stepB(this, &TempVal, _Prop);
                }

                return MostRecentPropertyAddress ? *(T*)MostRecentPropertyAddress : TempVal;
        }
};

// --- math helpers (framework.h) ----------------------------------------------
inline FQuat RotatorToQuat(FRotator Rotation)
{
        FQuat Quat;
        const float DEG_TO_RAD = 3.14159f / 180.0f;
        const float DIVIDE_BY_2 = DEG_TO_RAD / 2.0f;

        float SP = sin(Rotation.Pitch * DIVIDE_BY_2);
        float CP = cos(Rotation.Pitch * DIVIDE_BY_2);
        float SY = sin(Rotation.Yaw * DIVIDE_BY_2);
        float CY = cos(Rotation.Yaw * DIVIDE_BY_2);
        float SR = sin(Rotation.Roll * DIVIDE_BY_2);
        float CR = cos(Rotation.Roll * DIVIDE_BY_2);

        Quat.X = CR * SP * SY - SR * CP * CY;
        Quat.Y = -CR * SP * CY - SR * CP * SY;
        Quat.Z = CR * CP * SY - SR * SP * CY;
        Quat.W = CR * CP * CY + SR * SP * SY;

        return Quat;
}

static inline FQuat FRotToQuat(FRotator Rotation)
{
        return RotatorToQuat(Rotation);
}

// --- object finding/loading (framework.h) ------------------------------------
// StaticFindObjectOG (ImageBase+0x2E1C4B0) is replaced by the SDK's own
// UObject::FindObject<T>() helper shipped in Basic.hpp — no offset needed.
template <typename T = void*>
static inline T* StaticFindObject(const std::string& ObjectName)
{
        return UObject::FindObject<T>(ObjectName);
}

template<typename T>
inline T* Cast(UObject* Object)
{
        if (!Object || !Object->IsA(T::StaticClass()))
                return nullptr;
        return (T*)Object;
}

// StaticLoadObject: SDK FindObject first (like the original), then the raw
// loader through the registry (found:false → nullptr + log).
template<typename T = UObject>
static inline T* StaticLoadObject(const std::string& Name)
{
        T* Object = StaticFindObject<T>(Name);

        if (!Object)
        {
                auto loader = OgsFn::static_load_object();
                if (!loader)
                {
                        OGS_LOG("StaticLoadObject skipped (offset missing): %s", Name.c_str());
                        return nullptr;
                }
                FString wide = Ogs::Text(Name.c_str());
                Object = (T*)loader(T::StaticClass(), nullptr, wide.CStr(), nullptr, 0, nullptr, false, nullptr);
        }

        return Object;
}

template<typename T>
T* GetDefaultObject()
{
        return (T*)T::StaticClass()->DefaultObject;
}

// --- actor spawning (framework.h) --------------------------------------------
template<typename T>
inline T* SpawnActor(FVector Loc, FRotator Rot = FRotator(), AActor* Owner = nullptr, SDK::UClass* Class = T::StaticClass(), ESpawnActorCollisionHandlingMethod Handle = ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn)
{
        FTransform Transform{};
        Transform.Scale3D = FVector{ 1,1,1 };
        Transform.Translation = Loc;
        Transform.Rotation = FRotToQuat(Rot);

        return (T*)UGameplayStatics::FinishSpawningActor(UGameplayStatics::BeginDeferredActorSpawnFromClass(UWorld::GetWorld(), Class, Transform, Handle, Owner), Transform);
}

template<typename T>
static inline T* SpawnAActor(FVector Loc = { 0,0,0 }, FRotator Rot = { 0,0,0 }, AActor* Owner = nullptr)
{
        FTransform Transform{};
        Transform.Scale3D = { 1,1,1 };
        Transform.Translation = Loc;
        Transform.Rotation = FRotToQuat(Rot);

        AActor* NewActor = UGameplayStatics::BeginDeferredActorSpawnFromClass(UWorld::GetWorld(), T::StaticClass(), Transform, ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn, Owner);
        return (T*)UGameplayStatics::FinishSpawningActor(NewActor, Transform);
}

template<typename T>
static inline T* SpawnActorClass(FVector Loc = { 0,0,0 }, FRotator Rot = { 0,0,0 }, UClass* Class = nullptr, AActor* Owner = nullptr)
{
        FTransform Transform{};
        Transform.Scale3D = { 1,1,1 };
        Transform.Translation = Loc;
        Transform.Rotation = RotatorToQuat(Rot);

        AActor* NewActor = UGameplayStatics::BeginDeferredActorSpawnFromClass(UWorld::GetWorld(), Class, Transform, ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn, Owner);
        return (T*)UGameplayStatics::FinishSpawningActor(NewActor, Transform);
}

template<typename T>
T* Actors(UClass* Class = T::StaticClass(), FVector Loc = {}, FRotator Rot = {}, AActor* Owner = nullptr)
{
        return SpawnActor<T>(Loc, Rot, Owner, Class);
}

// --- DuplicateActor (framework.h; unused by the rest of the mod but ported) --
AActor* DuplicateActor(AActor* Original)
{
        if (!Original) return nullptr;

        const FVector Location = Original->K2_GetActorLocation();
        const FRotator Rotation = Original->K2_GetActorRotation();
        const FTransform Transform = Original->GetTransform();

        auto* Class = Original->Class;
        auto* Owner = Original->GetOwner();

        auto* Duplicated = (AActor*)UGameplayStatics::FinishSpawningActor(
                UGameplayStatics::BeginDeferredActorSpawnFromClass(UWorld::GetWorld(), Class, Transform, ESpawnActorCollisionHandlingMethod::AlwaysSpawn, Owner),
                Transform
        );

        if (Duplicated)
        {
                Duplicated->Name = Original->Name;
                Duplicated->Tags = Original->Tags;
        }

        // +0xB0 faction tag: the target classes are NOT in the dump → the
        // offset is registry-guarded (struct_faction_tag, found:false today).
        const auto& factionOff = Ogs::Off().struct_faction_tag;
        if (factionOff.Found)
        {
                FGameplayTag* OriginalFactionTag = (FGameplayTag*)((uintptr_t)Original + factionOff.Rva);
                FGameplayTag* DuplicateFactionTag = (FGameplayTag*)((uintptr_t)Duplicated + factionOff.Rva);

                if (OriginalFactionTag && DuplicateFactionTag)
                {
                        *DuplicateFactionTag = *OriginalFactionTag;
                }
        }
        else
        {
                OGS_LOG("DuplicateActor: faction tag copy skipped (offset missing)");
        }

        return Duplicated;
}

// --- pickups (framework.h) ----------------------------------------------------
AFortPickupAthena* SpawnPickup(UFortItemDefinition* ItemDef, int OverrideCount, int LoadedAmmo, FVector Loc, EFortPickupSourceTypeFlag SourceType, EFortPickupSpawnSource Source, AFortPawn* Pawn = nullptr)
{
        if (!ItemDef)
                return nullptr;

        auto SpawnedPickup = Actors<AFortPickupAthena>(AFortPickupAthena::StaticClass(), Loc);
        if (!SpawnedPickup)
                return nullptr;
        SpawnedPickup->bRandomRotation = true;

        auto& PickupEntry = SpawnedPickup->PrimaryPickupItemEntry;
        PickupEntry.ItemDefinition = ItemDef;
        PickupEntry.Count = OverrideCount;
        PickupEntry.LoadedAmmo = LoadedAmmo;
        PickupEntry.ReplicationKey++;
        SpawnedPickup->OnRep_PrimaryPickupItemEntry();
        SpawnedPickup->PawnWhoDroppedPickup = Pawn;

        SpawnedPickup->TossPickup(Loc, Pawn, -1, true, false, SourceType, Source);

        SpawnedPickup->SetReplicateMovement(true);
        SpawnedPickup->MovementComponent = (UProjectileMovementComponent*)GetDefaultObject<UGameplayStatics>()->SpawnObject(UProjectileMovementComponent::StaticClass(), SpawnedPickup);

        if (SourceType == EFortPickupSourceTypeFlag::Container)
        {
                SpawnedPickup->bTossedFromContainer = true;
                SpawnedPickup->OnRep_TossedFromContainer();
        }

        return SpawnedPickup;
}

std::map<AFortPickup*, float> PickupLifetimes;
AFortPickupAthena* SpawnStack(APlayerPawn_Athena_C* Pawn, UFortItemDefinition* Def, int Count, bool giveammo = false, int ammo = 0)
{
        auto Statics = OgsSDK::Statics();

        if (!Pawn || !Def)
                return nullptr;

        FVector Loc = Pawn->K2_GetActorLocation();
        AFortPickupAthena* Pickup = Actors<AFortPickupAthena>(AFortPickupAthena::StaticClass(), Loc);
        if (!Pickup)
                return nullptr;
        Pickup->bReplicates = true;
        PickupLifetimes[Pickup] = Statics->GetTimeSeconds(UWorld::GetWorld());
        Pickup->PawnWhoDroppedPickup = Pawn;
        Pickup->PrimaryPickupItemEntry.Count = Count;
        Pickup->PrimaryPickupItemEntry.ItemDefinition = Def;
        if (giveammo)
        {
                Pickup->PrimaryPickupItemEntry.LoadedAmmo = ammo;
        }
        Pickup->PrimaryPickupItemEntry.ReplicationKey++;

        Pickup->OnRep_PrimaryPickupItemEntry();
        Pickup->TossPickup(Loc, Pawn, 6, true, true, EFortPickupSourceTypeFlag::Other, EFortPickupSpawnSource::Unset);

        Pickup->MovementComponent = (UProjectileMovementComponent*)Statics->SpawnObject(UProjectileMovementComponent::StaticClass(), Pickup);
        Pickup->MovementComponent->bReplicates = true;
        ((UProjectileMovementComponent*)Pickup->MovementComponent)->SetComponentTickEnabled(true);

        return Pickup;
}

static AFortPickupAthena* SpawnPickup(FFortItemEntry ItemEntry, FVector Location, EFortPickupSourceTypeFlag PickupSource = EFortPickupSourceTypeFlag::Other, EFortPickupSpawnSource SpawnSource = EFortPickupSpawnSource::Unset)
{
        auto Pickup = SpawnPickup(ItemEntry.ItemDefinition, ItemEntry.Count, ItemEntry.LoadedAmmo, Location, PickupSource, SpawnSource);
        return Pickup;
}

// --- foundations (framework.h) -------------------------------------------------
inline void ShowFoundation(ABuildingFoundation* BuildingFoundation)
{
        if (!BuildingFoundation)
                return;

        BuildingFoundation->bServerStreamedInLevel = true;
        BuildingFoundation->DynamicFoundationType = EDynamicFoundationType::Static;
        BuildingFoundation->OnRep_ServerStreamedInLevel();

        BuildingFoundation->FoundationEnabledState = EDynamicFoundationEnabledState::Enabled;
        BuildingFoundation->DynamicFoundationRepData.EnabledState = EDynamicFoundationEnabledState::Enabled;
        BuildingFoundation->DynamicFoundationTransform = BuildingFoundation->GetTransform();
        BuildingFoundation->OnRep_DynamicFoundationRepData();
}

// --- supply drop location (framework.h, via OgsFn) -----------------------------
FVector PickSupplyDropLocation(SDK::AFortAthenaMapInfo* MapInfo, SDK::FVector Center, float Radius)
{
        auto PickSupplyDropLocationOG = OgsFn::pick_supply_drop_location();
        if (!PickSupplyDropLocationOG)
        {
                OGS_LOG("PickSupplyDropLocation skipped (offset missing)");
                return SDK::FVector(0, 0, 0);
        }

        const float MinDistance = 10000.0f;

        for (int i = 0; i < 20; i++)
        {
                SDK::FVector loc = FVector(0, 0, 0);
                PickSupplyDropLocationOG(MapInfo, &loc, (int64_t)&Center, Radius);

                bool bTooClose = false;
                for (const auto& other : PickedSupplyDropLocations)
                {
                        float dx = loc.X - other.X;
                        float dy = loc.Y - other.Y;
                        float dz = loc.Z - other.Z;

                        float distSquared = dx * dx + dy * dy + dz * dz;

                        if (distSquared < MinDistance * MinDistance)
                        {
                                bTooClose = true;
                                break;
                        }
                }

                if (!bTooClose)
                {
                        PickedSupplyDropLocations.Add(loc);
                        return loc;
                }
        }

        return SDK::FVector(0, 0, 0);
}

// --- object enumeration (framework.h) ------------------------------------------
template<typename T>
inline std::vector<T*> GetAllObjectsOfClass(UClass* Class = T::StaticClass())
{
        std::vector<T*> Objects{};

        for (int i = 0; i < UObject::GObjects->Num(); ++i)
        {
                UObject* Object = UObject::GObjects->GetByIndex(i);

                if (!Object)
                        continue;

                if (Object->GetFullName().find("Default") != std::string::npos)
                        continue;

                if (Object->GetFullName().find("Test") != std::string::npos)
                        continue;

                if (Object->IsA(Class) && !Object->IsDefaultObject())
                {
                        Objects.push_back((T*)Object);
                }
        }

        return Objects;
}

int CountActorsWithName(FName TargetName, UClass* Class)
{
        TArray<AActor*> FoundActors;
        auto* Statics = OgsSDK::Statics();
        if (!Class)
                return 0;
        Statics->GetAllActorsOfClass(UWorld::GetWorld(), Class, &FoundActors);

        int Count = 0;
        for (AActor* Actor : FoundActors)
        {
                if (Actor && Actor->GetName() == TargetName.ToString())
                        Count++;
        }
        return Count;
}

AFortPlayerControllerAthena* GetPCFromId(FUniqueNetIdRepl& ID)
{
        UWorld* World = UWorld::GetWorld();
        OGS_CHECK(World && World->GameState);

        for (auto& PlayerState : World->GameState->PlayerArray)
        {
                if (!PlayerState)
                        continue;
                auto PlayerStateAthena = Cast<AFortPlayerStateAthena>(PlayerState);
                if (!PlayerStateAthena)
                        continue;
                if (PlayerStateAthena->AreUniqueIDsIdentical(ID, PlayerState->UniqueId))
                        return Cast<AFortPlayerControllerAthena>(PlayerState->Owner);
        }

        return nullptr;
}

enum class EAccoladeEvent : uint8
{
        Kill,
        Search,
        MAX
};

inline UFortAccoladeItemDefinition* GetDefFromEvent(EAccoladeEvent Event, int Count, UObject* Object = nullptr)
{
        UFortAccoladeItemDefinition* Def = nullptr;

        switch (Event)
        {
        case EAccoladeEvent::Kill:
                if (Count == 1)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_014_Elimination_Bronze.AccoladeId_014_Elimination_Bronze");
                }
                else if (Count == 4)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_015_Elimination_Silver.AccoladeId_015_Elimination_Silver");
                }
                else if (Count == 8)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_016_Elimination_Gold.AccoladeId_016_Elimination_Gold");
                }
                break;
        case EAccoladeEvent::Search:
                if (Count == 3)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_008_SearchChests_Bronze.AccoladeId_008_SearchChests_Bronze");
                }
                else if (Count == 7)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_009_SearchChests_Silver.AccoladeId_009_SearchChests_Silver");
                }
                else if (Count == 12)
                {
                        Def = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_010_SearchChests_Gold.AccoladeId_010_SearchChests_Gold");
                }
                break;
        case EAccoladeEvent::MAX:
                break;
        default:
                break;
        }

        return Def;
}

// BT helper enums (framework.h tail) — EBTExecutionMode etc. only exist as
// SDK enums for the reflected parts; the mod-local ones stay mod-local.
namespace EBTExecutionMode
{
        enum Type
        {
                SingleRun,
                Looped,
        };
}

// =============================================================================
// BehaviourTree_System.h (orig) — pure C++ BT framework, verbatim port.
// =============================================================================
struct BTContext
{
        AFortAthenaAIBotController* Controller;
        AFortPlayerPawnAthena* Pawn;
};

class BTDecorator {
public:
        // All of these strings dont need to have a value its just good for runtime data
        std::string Name;
        std::string CachedDescription;
        std::string NodeName;

public:
        virtual bool Evaluate(BTContext Context) = 0;
};

class BTService {
public:
        std::string Name;
        std::string NodeName;

        float Interval = 0.f;
        float NextTickTime = 0.f;
public:
        virtual void TickService(BTContext Context) = 0;
};

class BTNode
{
private:
        std::vector<BTDecorator*> Decorators;
        std::vector<BTService*> Services;
public:
        virtual EBTNodeResult ChildTask(BTContext Context) = 0;
public:
        void AddDecorator(BTDecorator* Decorator) {
                Decorators.push_back(Decorator);
        }

        void AddService(BTService* Service) {
                Services.push_back(Service);
        }

        EBTNodeResult Tick(BTContext& Context) {
                // If a decorator fails then we shouldnt execute the task
                for (BTDecorator* Decorator : Decorators) {
                        if (!Decorator->Evaluate(Context)) {
                                return EBTNodeResult::Failed;
                        }
                }

                for (BTService* Service : Services) {
                        Service->TickService(Context);
                }

                // Run the task once all of the decorators pass
                return ChildTask(Context);
        }
};

class BTComposite_Selector
{
private:
        std::vector<BTNode*> Children;
        std::vector<BTDecorator*> Decorators;
        std::vector<BTService*> Services;

public:
        std::string Name;
public:
        void AddChild(BTNode* Node) {
                Children.push_back(Node);
        }

        void AddDecorator(BTDecorator* Decorator) {
                Decorators.push_back(Decorator);
        }

        void AddService(BTService* Service) {
                Services.push_back(Service);
        }

        virtual EBTNodeResult Tick(BTContext Context) {
                // If a global selector decorator fails we shouldnt execute anything inside of the selector (shouldnt be on the root)
                for (BTDecorator* Decorator : Decorators) {
                        if (!Decorator->Evaluate(Context)) {
                                return EBTNodeResult::Failed;
                        }
                }

                for (BTService* Service : Services) {
                        Service->TickService(Context);
                }

                // Run all of the selectors children then if all fail then return faliure
                for (BTNode* Child : Children)
                {
                        EBTNodeResult Result = Child->Tick(Context);
                        if (Result == EBTNodeResult::Succeeded || Result == EBTNodeResult::InProgress)
                                return Result;
                }
                return EBTNodeResult::Failed;
        }
};

class BehaviorTree
{
public:
        std::string Name;

        std::vector<BTComposite_Selector*> AllNodes;
        BTComposite_Selector* RootNode = nullptr;
        UBlackboardData* BlackboardAsset = nullptr;

        BTComposite_Selector* FindSelectorByName(std::string Name) {
                for (BTComposite_Selector* Selector : AllNodes) {
                        if (Selector->Name == Name) {
                                return Selector;
                        }
                }
                return nullptr;
        }

        void Tick(BTContext Context) {
                if (RootNode)
                        RootNode->Tick(Context);
        }

        ~BehaviorTree() {
                for (auto* Node : AllNodes)
                        delete Node;
        }
};

// =============================================================================
// Bosses.h prerequisites: BT_MANG_Context + FactionBot forward decl + the
// FactionBots registry (used by Bots/Tick/PlayerBots).
// =============================================================================
enum class EBossesStrafeType {
        StrafeLeft,
        StrafeRight
};

struct BT_MANG_Context : BTContext
{
        class FactionBot* bot;
};

std::vector<class FactionBot*> FactionBots{};

// =============================================================================
// PlayerBots.h enums (needed by GameMode/Bots/Tick before PlayerBot itself).
// =============================================================================
enum class EBotState : uint8 {
        Warmup,
        PreBus, // Dont handle this, just there to stop bots from killing eachother before bus enters dropzone
        Bus,
        Skydiving,
        Gliding,
        Landed,
        Fleeing,
        Looting,
        LookingForPlayers,
        MovingToSafeZone,
        Stuck
};

enum class EBotWarmupChoice {
        Emote,
        MoveToPlayerEmote,
        MAX
};

enum class EBotStrafeType {
        StrafeLeft,
        StrafeRight
};

enum class ELootableType {
        None = -1,
        Chest = 0,
        Pickup = 1
};

// =============================================================================
// Quests.h (orig)
// =============================================================================
namespace Quests {
        void GiveAccolade(AFortPlayerControllerAthena* PC, UFortAccoladeItemDefinition* Def, UFortQuestItemDefinition* QuestDef = nullptr)
        {
                if (!PC || !Def) return;

                FAthenaAccolades Accolade{};
                Accolade.AccoladeDef = Def;
                Accolade.Count = 1;
                std::string DefName = Def->GetName();
                Accolade.TemplateId = Ogs::Text(DefName.c_str());

                auto ID = UKismetSystemLibrary::GetDefaultObj()->GetPrimaryAssetIdFromObject(Def);

                FXPEventInfo EventInfo{};
                EventInfo.Accolade = ID;
                EventInfo.EventName = Def->Name;
                EventInfo.EventXpValue = Def->GetAccoladeXpValue();
                EventInfo.Priority = Def->Priority;
                if (QuestDef) {
                        EventInfo.QuestDef = QuestDef;
                }
                EventInfo.SimulatedText = Def->GetShortDescription();
                EventInfo.RestedValuePortion = EventInfo.EventXpValue;
                EventInfo.RestedXPRemaining = EventInfo.EventXpValue;
                EventInfo.SeasonBoostValuePortion = 20;
                if (!PC->XPComponent)
                {
                        OGS_LOG("GiveAccolade: XPComponent missing — accolade skipped");
                        return;
                }
                EventInfo.TotalXpEarnedInMatch = EventInfo.EventXpValue + PC->XPComponent->TotalXpEarned;

                PC->XPComponent->MedalBonusXP += 1250;
                PC->XPComponent->MatchXp += EventInfo.EventXpValue;
                PC->XPComponent->TotalXpEarned += EventInfo.EventXpValue + 1250;

                PC->XPComponent->PlayerAccolades.Add(Accolade);
                PC->XPComponent->MedalsEarned.Add(Def);

                PC->XPComponent->ClientMedalsRecived(PC->XPComponent->PlayerAccolades);
                PC->XPComponent->OnXPEvent(EventInfo);
        }

        void ProgressQuest(AFortPlayerControllerAthena* PC, UFortQuestItemDefinition* QuestDef, FName BackendName)
        {
                if (!PC || !QuestDef)
                        return;
                AFortPlayerStateAthena* PlayerState = (AFortPlayerStateAthena*)PC->PlayerState;
                OGS_CHECK(PlayerState && PlayerState->PlayerTeam);

                if (auto QuestMgr = PC->GetQuestManager(ESubGame::Athena))
                        QuestMgr->SelfCompletedUpdatedQuest(PC, QuestDef, BackendName, 1, 1, nullptr, true, false);
                for (size_t i = 0; i < PlayerState->PlayerTeam->TeamMembers.Num(); i++)
                {
                        auto pc = (AFortPlayerControllerAthena*)PlayerState->PlayerTeam->TeamMembers[i];
                        if (pc && pc != PC)
                        {
                                if (auto QuestMgr = pc->GetQuestManager(ESubGame::Athena))
                                        QuestMgr->SelfCompletedUpdatedQuest(PC, QuestDef, BackendName, 1, 1, PlayerState, true, false);
                        }
                }
                auto QuestMgr = PC->GetQuestManager(ESubGame::Athena);
                if (!QuestMgr)
                        return;
                auto QuestItem = QuestMgr->GetQuestWithDefinition(QuestDef);

                int32 XPCount = 20;

                if (auto RewardsTable = QuestDef->RewardsTable)
                {
                        static auto Name = FName(OGS_TEXT("Default"));
                        auto DefaultRow = RewardsTable->Search([](FName& RName, uint8* Row) { return RName == Name; });
                        XPCount = DefaultRow ? (*(FFortQuestRewardTableRow**)DefaultRow)->Quantity : XPCount;
                }

                FXPEventEntry Entry{};
                Entry.EventXpValue = XPCount;
                Entry.QuestDef = QuestDef;
                Entry.Time = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());
                PC->XPComponent->ChallengeXp += Entry.EventXpValue;
                PC->XPComponent->TotalXpEarned += Entry.EventXpValue;
                Entry.TotalXpEarnedInMatch = PC->XPComponent->TotalXpEarned;
                Entry.SimulatedXpEvent = QuestDef->GetSingleLineDescription();
                PC->XPComponent->RestXP += Entry.EventXpValue;
                PC->XPComponent->InMatchProfileVer++;
                PC->XPComponent->OnInMatchProfileUpdate(PC->XPComponent->InMatchProfileVer);
                PC->XPComponent->OnProfileUpdated();
                PC->XPComponent->OnXpUpdated(PC->XPComponent->CombatXp, PC->XPComponent->SurvivalXp, PC->XPComponent->MedalBonusXP, PC->XPComponent->ChallengeXp, PC->XPComponent->MatchXp, PC->XPComponent->TotalXpEarned);
                PC->XPComponent->WaitingQuestXp.Add(Entry);

                PC->XPComponent->HighPrioXPEvent(Entry);
        }

        bool ContainsTag(FGameplayTagContainer Container, FName Tag)
        {
                for (auto tag : Container.GameplayTags)
                {
                        if (tag.TagName.ComparisonIndex == Tag.ComparisonIndex)
                                return true;
                }
                return false;
        }

        void SendStatEvent(UFortQuestManager* ManagerComp, UObject* TargetObject, FGameplayTagContainer& AdditionalSourceTags, FGameplayTagContainer& TargetTags, int32 Count, EFortQuestObjectiveStatEvent StatEvent)
        {
                if (!ManagerComp || !TargetObject) {
                        Log("ManagerComp or TargetObject is nullptr!");
                }

                if (TargetTags.GameplayTags.IsValid())
                {
                        Log("Valid");
                        for (int i = 0; i < TargetTags.GameplayTags.Num(); i++)
                        {
                                Log("TargetTags GameplayTags: %s", TargetTags.GameplayTags[i].TagName.ToString().c_str());
                        }
                }

                if (TargetTags.ParentTags.IsValid())
                {
                        Log("Valid");
                        for (int i = 0; i < TargetTags.ParentTags.Num(); i++)
                        {
                                Log("TargetTags ParentTags: %s", TargetTags.ParentTags[i].TagName.ToString().c_str());
                        }
                }

                auto PC = (AFortPlayerControllerAthena*)ManagerComp->GetPlayerControllerBP();
                if (!PC) {
                        Log("PC doesent exist!");
                        return;
                }
                else {
                        Log("PC does exist!");
                }

                TArray<UFortQuestItem*> CurrentQuests;
                ManagerComp->GetCurrentQuests(&CurrentQuests);

                if (CurrentQuests.IsValid())
                {
                        for (int i = 0; i < CurrentQuests.Num(); i++)
                        {
                                UFortQuestItem* CurrentQuest = CurrentQuests[i];
                                if (!CurrentQuest)
                                        continue;

                                UFortQuestItemDefinition* QuestDefinition = CurrentQuest->GetQuestDefinitionBP();
                                if (!QuestDefinition)
                                        continue;

                                for (int oi = 0; oi < QuestDefinition->Objectives.Num(); oi++)
                                {
                                        FFortMcpQuestObjectiveInfo ObjectiveInfo = QuestDefinition->Objectives[oi];
                                        FDataTableRowHandle ObjectStatHandle = ObjectiveInfo.ObjectiveStatHandle;
                                        UDataTable* DataTable = ObjectStatHandle.DataTable;
                                        if (!DataTable)
                                                continue;

                                        auto& RowMap = DataTable->RowMap;
                                        FFortQuestObjectiveStatTableRow* Row = nullptr;

                                        if (RowMap.Num() <= 0)
                                                continue;

                                        for (int ri = 0; ri < RowMap.Num(); ++ri)
                                        {
                                                auto& Pair = RowMap[ri];
                                                if (Pair.Key() == ObjectStatHandle.RowName)
                                                {
                                                        Row = reinterpret_cast<FFortQuestObjectiveStatTableRow*>(Pair.Second);
                                                        if (Row->TargetTagContainer.GameplayTags.IsValid() && Row->TargetTagContainer.GameplayTags.IsValidIndex(0)
                                                                && TargetTags.GameplayTags.IsValid() && TargetTags.GameplayTags.IsValidIndex(0))
                                                        {
                                                                if (Row->TargetTagContainer.GameplayTags[0].TagName == TargetTags.GameplayTags[0].TagName)
                                                                {
                                                                        for (int ci = 0; ci < CurrentQuest->Objectives.Num(); ci++)
                                                                        {
                                                                                UFortQuestObjectiveInfo* Objective = CurrentQuest->Objectives[ci];
                                                                                if (!Objective)
                                                                                        continue;

                                                                                FName BackendName = Objective->BackendName;

                                                                                Objective->AchievedCount += Count;
                                                                                Objective->bActive = true;
                                                                                Objective->DisplayDynamicQuestUpdate();
                                                                        }
                                                                        Log("Wow");
                                                                        ManagerComp->ForceTriggerQuestsUpdated();
                                                                        return ProgressQuest(PC, QuestDefinition, ObjectiveInfo.BackendName);
                                                                }
                                                        }
                                                }
                                        }
                                }
                        }
                }
        }

        static inline void (*SendComplexCustomStatEventOG)(UObject* TargetObject, FGameplayTagContainer& AdditionalSourceTags, FGameplayTagContainer& TargetTags, int32 Count);
        void SendComplexCustomStatEvent(UObject* TargetObject, FGameplayTagContainer& AdditionalSourceTags, FGameplayTagContainer& TargetTags, int32 Count)
        {
                Log("SendComplexCustomStateEvent Called!");
                if (!TargetObject) {
                        Log("No TargetObject!");
                }
                UFortQuestManager* ManagerComp = Cast<UFortQuestManager>(TargetObject);
                if (!ManagerComp) {
                        Log("No ManagerComp!");
                        return;
                }

                SendStatEvent(ManagerComp, TargetObject, AdditionalSourceTags, TargetTags, Count, EFortQuestObjectiveStatEvent::ComplexCustom);
        }

        static bool GetQuestContext(UFortQuestManager* QuestManager, EFortQuestObjectiveStatEvent SE) {
                if (!QuestManager || !QuestManager->GetPlayerControllerBP() || !QuestManager->GetPlayerControllerBP()->InteractionComponent
                        || !QuestManager->GetPlayerControllerBP()->InteractionComponent->InteractActor)
                {
                        Log("GetQuestContext: missing chain — skipping");
                        return true;
                }
                Log("test!");
                Log("%s", QuestManager->GetPlayerControllerBP()->InteractionComponent->InteractActor->GetName().c_str());
                FGameplayTagContainer SourceTags;
                FGameplayTagContainer ContextTags;
                FGameplayTagContainer PlaylistContextTags;

                QuestManager->GetSourceAndContextTags(&SourceTags, &ContextTags);
                for (auto& Tag : SourceTags.GameplayTags)
                        Log("%s", Tag.TagName.ToString().c_str());
                return true;
        }

        void Hook() {
                // Both original MH_CreateHook calls in Quests::Hook were commented out
                // in the source; keep it log-only.
                Log("Quests Hooked!");
        }
}

// =============================================================================
// Looting.h (orig) — Credits: MGS
// =============================================================================
namespace Looting {
        void SpawnLlamas()
        {
                int LlamasSpawned = 0;
                auto LlamasToSpawn = (rand() % 3) + 3;
                Log(std::string("Spawned ") + std::to_string(LlamasToSpawn) + " Llamas");

                auto GameState = OgsSDK::GameState();
                OGS_CHECK(GameState && GameState->MapInfo);

                auto MapInfo = GameState->MapInfo;

                for (int i = 0; i < LlamasToSpawn; i++)
                {
                        int Radius = 100000;
                        FVector Location = PickSupplyDropLocation(MapInfo, FVector(1, 1, 10000), (float)Radius);

                        FRotator RandomYawRotator{};
                        RandomYawRotator.Yaw = (float)rand() * 0.010986663f;

                        FTransform InitialSpawnTransform{};
                        InitialSpawnTransform.Translation = Location;
                        InitialSpawnTransform.Rotation = FRotToQuat(RandomYawRotator);
                        InitialSpawnTransform.Scale3D = FVector(1, 1, 1);

                        auto LlamaStart = SpawnActor<AFortAthenaSupplyDrop>(Location, RandomYawRotator, nullptr, MapInfo->LlamaClass.Get());

                        if (!LlamaStart)
                                continue;

                        auto GroundLocation = LlamaStart->FindGroundLocationAt(InitialSpawnTransform.Translation);

                        LlamaStart->K2_DestroyActor();

                        auto Llama = SpawnActor<AFortAthenaSupplyDrop>(GroundLocation, RandomYawRotator, nullptr, MapInfo->LlamaClass.Get());

                        if (!Llama)
                                continue;
                        Llama->bCanBeDamaged = false;
                        LlamasSpawned++;
                }
        }

        static FFortLootTierData* GetLootTierData(std::vector<FFortLootTierData*>& LootTierData)
        {
                float TotalWeight = 0;

                for (auto Item : LootTierData)
                        TotalWeight += Item->Weight;

                float RandomNumber = UKismetMathLibrary::RandomFloatInRange(0, TotalWeight);

                FFortLootTierData* SelectedItem = nullptr;

                for (auto Item : LootTierData)
                {
                        if (RandomNumber <= Item->Weight)
                        {
                                SelectedItem = Item;
                                break;
                        }

                        RandomNumber -= Item->Weight;
                }

                if (!SelectedItem)
                        return GetLootTierData(LootTierData);

                return SelectedItem;
        }

        static FFortLootPackageData* GetLootPackage(std::vector<FFortLootPackageData*>& LootPackages)
        {
                float TotalWeight = 0;

                for (auto Item : LootPackages)
                        TotalWeight += Item->Weight;

                float RandomNumber = UKismetMathLibrary::RandomFloatInRange(0, TotalWeight);

                FFortLootPackageData* SelectedItem = nullptr;

                for (auto Item : LootPackages)
                {
                        if (RandomNumber <= Item->Weight)
                        {
                                SelectedItem = Item;
                                break;
                        }

                        RandomNumber -= Item->Weight;
                }

                if (!SelectedItem)
                        return GetLootPackage(LootPackages);

                return SelectedItem;
        }

        int GetClipSize(UFortItemDefinition* ItemDef) {
                if (auto RangedDef = Cast<UFortWeaponRangedItemDefinition>(ItemDef)) {
                        auto DataTable = RangedDef->WeaponStatHandle.DataTable;
                        auto RowName = RangedDef->WeaponStatHandle.RowName;

                        if (DataTable && RowName.ComparisonIndex) {
                                // +0x30 raw RowMap access → reflected UDataTable::RowMap
                                auto& RowMap = DataTable->RowMap;

                                for (auto& Pair : RowMap) {
                                        FName CurrentRowName = Pair.Key();
                                        FFortRangedWeaponStats* PackageData = (FFortRangedWeaponStats*)Pair.Value();

                                        if (CurrentRowName == RowName && PackageData) {
                                                return PackageData->ClipSize;
                                        }
                                }
                        }
                }

                return 0;
        }

        std::vector<FFortItemEntry> PickLootDrops(FName TierGroupName, int recursive = 0)
        {
                std::vector<FFortItemEntry> LootDrops;

                if (recursive >= 5)
                {
                        return LootDrops;
                }

                auto TierGroupFName = TierGroupName;

                static std::vector<UDataTable*> LTDTables;
                static std::vector<UDataTable*> LPTables;

                static bool First = false;

                if (!First)
                {
                        First = true;

                        auto GameState = OgsSDK::GameState();

                        UDataTable* MainLTD = nullptr;
                        UDataTable* MainLP = nullptr;

                        if (GameState && GameState->CurrentPlaylistInfo.BasePlaylist)
                        {
                                MainLTD = StaticLoadObject<UDataTable>(UKismetStringLibrary::Conv_NameToString(GameState->CurrentPlaylistInfo.BasePlaylist->LootTierData.ObjectID.AssetPathName).ToString());
                                MainLP = StaticLoadObject<UDataTable>(UKismetStringLibrary::Conv_NameToString(GameState->CurrentPlaylistInfo.BasePlaylist->LootPackages.ObjectID.AssetPathName).ToString());
                        }

                        if (!MainLTD)
                                MainLTD = UObject::FindObject<UDataTable>("AthenaLootTierData_Client");

                        if (!MainLP)
                                MainLP = UObject::FindObject<UDataTable>("AthenaLootPackages_Client");

                        LTDTables.push_back(MainLTD);

                        LPTables.push_back(MainLP);
                }

                std::vector<FFortLootTierData*> TierGroupLTDs;

                for (int p = 0; p < LTDTables.size(); p++)
                {
                        auto LTD = LTDTables[p];
                        if (!LTD)
                                continue;
                        auto& LTDRowMap = LTD->RowMap;

                        auto LTDRowMapNum = LTDRowMap.Elements.Num();

                        for (int i = 0; i < LTDRowMapNum; i++)
                        {
                                auto& CurrentLTD = LTDRowMap.Elements[i];
                                auto TierData = (FFortLootTierData*)CurrentLTD.Value();

                                if (TierGroupName == TierData->TierGroup && TierData->Weight != 0)
                                {
                                        TierGroupLTDs.push_back(TierData);
                                }
                        }
                }

                if (TierGroupLTDs.empty())
                        return LootDrops;

                FFortLootTierData* ChosenRowLootTierData = GetLootTierData(TierGroupLTDs);

                if (ChosenRowLootTierData->NumLootPackageDrops <= 0)
                        return PickLootDrops(TierGroupName, ++recursive);

                if (ChosenRowLootTierData->LootPackageCategoryMinArray.Num() != ChosenRowLootTierData->LootPackageCategoryWeightArray.Num() ||
                        ChosenRowLootTierData->LootPackageCategoryMinArray.Num() != ChosenRowLootTierData->LootPackageCategoryMaxArray.Num())
                        return PickLootDrops(TierGroupName, ++recursive);

                int MinimumLootDrops = 0;
                float NumLootPackageDrops = std::floor(ChosenRowLootTierData->NumLootPackageDrops);

                if (ChosenRowLootTierData->LootPackageCategoryMinArray.Num())
                {
                        for (int i = 0; i < ChosenRowLootTierData->LootPackageCategoryMinArray.Num(); i++)
                        {
                                if (ChosenRowLootTierData->LootPackageCategoryMinArray[i] > 0)
                                {
                                        MinimumLootDrops += ChosenRowLootTierData->LootPackageCategoryMinArray[i];
                                }
                        }
                }

                int SumLootPackageCategoryWeightArray = 0;

                for (int i = 0; i < ChosenRowLootTierData->LootPackageCategoryWeightArray.Num(); i++)
                {
                        auto CategoryWeight = ChosenRowLootTierData->LootPackageCategoryWeightArray[i];

                        if (CategoryWeight > 0)
                        {
                                auto CategoryMaxArray = ChosenRowLootTierData->LootPackageCategoryMaxArray[i];

                                if (CategoryMaxArray < 0)
                                {
                                        SumLootPackageCategoryWeightArray += CategoryWeight;
                                }
                        }
                }

                int SumLootPackageCategoryMinArray = 0;

                for (int i = 0; i < ChosenRowLootTierData->LootPackageCategoryMinArray.Num(); i++)
                {
                        auto CategoryWeight = ChosenRowLootTierData->LootPackageCategoryMinArray[i];

                        if (CategoryWeight > 0)
                        {
                                auto CategoryMaxArray = ChosenRowLootTierData->LootPackageCategoryMaxArray[i];

                                if (CategoryMaxArray < 0)
                                {
                                        SumLootPackageCategoryMinArray += CategoryWeight;
                                }
                        }
                }

                if (SumLootPackageCategoryWeightArray > SumLootPackageCategoryMinArray)
                        return PickLootDrops(TierGroupName, ++recursive);

                std::vector<FFortLootPackageData*> TierGroupLPs;

                for (int p = 0; p < LPTables.size(); p++)
                {
                        auto LP = LPTables[p];
                        if (!LP)
                                continue;
                        auto& LPRowMap = LP->RowMap;

                        for (int i = 0; i < LPRowMap.Elements.Num(); i++)
                        {
                                auto& CurrentLP = LPRowMap.Elements[i];
                                auto LootPackage = (FFortLootPackageData*)CurrentLP.Value();

                                if (!LootPackage)
                                        continue;

                                if (LootPackage->LootPackageID == ChosenRowLootTierData->LootPackage && LootPackage->Weight != 0)
                                {
                                        TierGroupLPs.push_back(LootPackage);
                                }
                        }
                }

                auto ChosenLootPackageName = ChosenRowLootTierData->LootPackage.ToString();

                bool bIsWorldList = ChosenLootPackageName.find("WorldList") != std::string::npos;

                LootDrops.reserve((size_t)NumLootPackageDrops);

                for (float i = 0; i < NumLootPackageDrops; i++)
                {
                        if (i >= TierGroupLPs.size())
                                break;

                        auto TierGroupLP = TierGroupLPs.at((size_t)i);
                        auto TierGroupLPStr = TierGroupLP->LootPackageCall.ToString();

                        if (TierGroupLPStr.find(".Empty") != std::string::npos)
                        {
                                NumLootPackageDrops++;
                                continue;
                        }

                        std::vector<FFortLootPackageData*> lootPackageCalls;

                        if (bIsWorldList)
                        {
                                for (int j = 0; j < TierGroupLPs.size(); j++)
                                {
                                        auto& CurrentLP = TierGroupLPs.at(j);

                                        if (CurrentLP->Weight != 0)
                                                lootPackageCalls.push_back(CurrentLP);
                                }
                        }
                        else
                        {
                                for (int p = 0; p < LPTables.size(); p++)
                                {
                                        auto LP = LPTables[p];
                                        if (!LP)
                                                continue;
                                        auto& LPRowMap = LP->RowMap;

                                        for (int j = 0; j < LPRowMap.Elements.Num(); j++)
                                        {
                                                auto& CurrentLP = LPRowMap.Elements[j];

                                                auto LootPackage = (FFortLootPackageData*)CurrentLP.Value();

                                                if (LootPackage->LootPackageID.ToString() == TierGroupLPStr && LootPackage->Weight != 0)
                                                {
                                                        lootPackageCalls.push_back(LootPackage);
                                                }
                                        }
                                }
                        }

                        if (lootPackageCalls.size() == 0)
                        {
                                NumLootPackageDrops++;
                                continue;
                        }

                        FFortLootPackageData* LootPackageCall = GetLootPackage(lootPackageCalls);

                        if (!LootPackageCall)
                                continue;

                        auto ItemDef = LootPackageCall->ItemDefinition.Get();

                        if (!ItemDef)
                        {
                                NumLootPackageDrops++;
                                continue;
                        }

                        FFortItemEntry LootDropEntry{};

                        LootDropEntry.ItemDefinition = ItemDef;
                        LootDropEntry.LoadedAmmo = GetClipSize(Cast<UFortWeaponItemDefinition>(ItemDef));
                        LootDropEntry.Count = LootPackageCall->Count;

                        LootDrops.push_back(LootDropEntry);
                }

                return LootDrops;
        }

        char SpawnLoot(ABuildingContainer* BuildingContainer)
        {
                OGS_CHECK(BuildingContainer && BuildingContainer->Class);

                std::string ClassName = BuildingContainer->Class->GetName();

                auto SearchLootTierGroup = BuildingContainer->SearchLootTierGroup;
                EFortPickupSpawnSource SpawnSource = EFortPickupSpawnSource::Unset;

                EFortPickupSourceTypeFlag PickupSourceTypeFlags = EFortPickupSourceTypeFlag::Container;

                static auto Loot_Treasure = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_Treasure"));
                static auto Loot_Ammo = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_Ammo"));
                static auto Loot_AthenaFloorLoot = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_AthenaFloorLoot"));
                static auto Loot_AthenaFloorLoot_Warmup = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_AthenaFloorLoot_Warmup"));

                if (SearchLootTierGroup == Loot_AthenaFloorLoot || SearchLootTierGroup == Loot_AthenaFloorLoot_Warmup)
                {
                        PickupSourceTypeFlags = EFortPickupSourceTypeFlag::FloorLoot;
                }

                if (!Globals::LateGame || !Globals::Arsenal)
                {
                        BuildingContainer->bAlreadySearched = true;
                        BuildingContainer->SearchBounceData.SearchAnimationCount++;
                        BuildingContainer->OnRep_bAlreadySearched();
                }
                else if (Globals::LateGame && SearchLootTierGroup == Loot_AthenaFloorLoot_Warmup || Globals::Arsenal && SearchLootTierGroup == Loot_AthenaFloorLoot_Warmup)
                {
                        BuildingContainer->bAlreadySearched = true;
                        BuildingContainer->SearchBounceData.SearchAnimationCount++;
                        BuildingContainer->OnRep_bAlreadySearched();

                        auto LootDrops = PickLootDrops(SearchLootTierGroup);

                        auto CorrectLocation = BuildingContainer->K2_GetActorLocation() + (BuildingContainer->GetActorForwardVector() * BuildingContainer->LootSpawnLocation_Athena.X) + (BuildingContainer->GetActorRightVector() * BuildingContainer->LootSpawnLocation_Athena.Y) + (BuildingContainer->GetActorUpVector() * BuildingContainer->LootSpawnLocation_Athena.Z);

                        for (auto& LootDrop : LootDrops)
                        {
                                if (LootDrop.Count > 0)
                                {
                                        SpawnPickup(LootDrop.ItemDefinition, LootDrop.Count, LootDrop.LoadedAmmo, CorrectLocation, PickupSourceTypeFlags, SpawnSource);

                                        if (SearchLootTierGroup == Loot_AthenaFloorLoot || SearchLootTierGroup == Loot_AthenaFloorLoot_Warmup)
                                        {
                                                if (LootDrop.ItemDefinition->GetName() == "WID_Athena_HappyGhost")
                                                {
                                                        return 0;
                                                }

                                                UFortAmmoItemDefinition* AmmoDef = (UFortAmmoItemDefinition*)((UFortWeaponRangedItemDefinition*)LootDrop.ItemDefinition)->GetAmmoWorldItemDefinition_BP();

                                                if (AmmoDef && LootDrop.ItemDefinition != AmmoDef && AmmoDef->DropCount > 0)
                                                {
                                                        SpawnPickup(AmmoDef, AmmoDef->DropCount, 0, CorrectLocation, PickupSourceTypeFlags, SpawnSource);
                                                }
                                        }
                                }

                        }
                }

                if (!Globals::LateGame || !Globals::Arsenal)
                {
                        if (SearchLootTierGroup == Loot_Treasure)
                        {
                                EFortPickupSourceTypeFlag PickupSourceTypeFlags = EFortPickupSourceTypeFlag::Container;

                                SearchLootTierGroup = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_AthenaTreasure"));
                                SpawnSource = EFortPickupSpawnSource::Chest;
                        }

                        if (SearchLootTierGroup == Loot_Ammo)
                        {
                                EFortPickupSourceTypeFlag PickupSourceTypeFlags = EFortPickupSourceTypeFlag::Container;

                                SearchLootTierGroup = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Loot_AthenaAmmoLarge"));
                                SpawnSource = EFortPickupSpawnSource::AmmoBox;
                        }

                        if (ClassName.find("Tiered_Chest_Athena_FactionChest") != std::string::npos)
                        {
                                for (int i = 0; i < 2; i++)
                                {
                                        auto LootDrops = PickLootDrops(SearchLootTierGroup);

                                        auto CorrectLocation = BuildingContainer->K2_GetActorLocation() + (BuildingContainer->GetActorForwardVector() * BuildingContainer->LootSpawnLocation_Athena.X) + (BuildingContainer->GetActorRightVector() * BuildingContainer->LootSpawnLocation_Athena.Y) + (BuildingContainer->GetActorUpVector() * BuildingContainer->LootSpawnLocation_Athena.Z);

                                        for (auto& LootDrop : LootDrops)
                                        {
                                                SpawnPickup(LootDrop.ItemDefinition, LootDrop.Count, LootDrop.LoadedAmmo, CorrectLocation, PickupSourceTypeFlags, SpawnSource);

                                                UFortAmmoItemDefinition* AmmoDef = (UFortAmmoItemDefinition*)((UFortWeaponRangedItemDefinition*)LootDrop.ItemDefinition)->GetAmmoWorldItemDefinition_BP();

                                                if (AmmoDef && LootDrop.ItemDefinition != AmmoDef && AmmoDef->DropCount > 0)
                                                {
                                                        SpawnPickup(AmmoDef, AmmoDef->DropCount, 0, CorrectLocation, PickupSourceTypeFlags, SpawnSource);
                                                }
                                        }
                                }
                                return true;
                        }

                        else if (ClassName.find("Tiered_Chest") != std::string::npos)
                        {
                                auto LootDrops = PickLootDrops(SearchLootTierGroup);

                                auto CorrectLocation = BuildingContainer->K2_GetActorLocation() + (BuildingContainer->GetActorForwardVector() * BuildingContainer->LootSpawnLocation_Athena.X) + (BuildingContainer->GetActorRightVector() * BuildingContainer->LootSpawnLocation_Athena.Y) + (BuildingContainer->GetActorUpVector() * BuildingContainer->LootSpawnLocation_Athena.Z);

                                for (auto& LootDrop : LootDrops)
                                {
                                        SpawnPickup(LootDrop.ItemDefinition, LootDrop.Count, LootDrop.LoadedAmmo, CorrectLocation, PickupSourceTypeFlags, SpawnSource);
                                }

                                static auto Wood = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/WoodItemData.WoodItemData");
                                static auto Metal = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/MetalItemData.MetalItemData");
                                static auto Stone = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/StoneItemData.StoneItemData");

                                UFortItemDefinition* Mats = (rand() % 40 > 20) ? ((rand() % 20 > 10) ? Wood : Stone) : Metal;

                                SpawnPickup(Mats, 30, 0, CorrectLocation, PickupSourceTypeFlags, SpawnSource);

                                return true;
                        }

                        else
                        {
                                auto LootDrops = PickLootDrops(SearchLootTierGroup);

                                auto CorrectLocation = BuildingContainer->K2_GetActorLocation() + (BuildingContainer->GetActorForwardVector() * BuildingContainer->LootSpawnLocation_Athena.X) + (BuildingContainer->GetActorRightVector() * BuildingContainer->LootSpawnLocation_Athena.Y) + (BuildingContainer->GetActorUpVector() * BuildingContainer->LootSpawnLocation_Athena.Z);

                                for (auto& LootDrop : LootDrops)
                                {
                                        if (LootDrop.Count > 0)
                                        {
                                                SpawnPickup(LootDrop.ItemDefinition, LootDrop.Count, LootDrop.LoadedAmmo, CorrectLocation, PickupSourceTypeFlags, SpawnSource);

                                                if (SearchLootTierGroup == Loot_AthenaFloorLoot || SearchLootTierGroup == Loot_AthenaFloorLoot_Warmup)
                                                {
                                                        if (LootDrop.ItemDefinition->GetName() == "WID_Athena_HappyGhost")
                                                        {
                                                                return 0;
                                                        }

                                                        UFortAmmoItemDefinition* AmmoDef = (UFortAmmoItemDefinition*)((UFortWeaponRangedItemDefinition*)LootDrop.ItemDefinition)->GetAmmoWorldItemDefinition_BP();

                                                        if (AmmoDef && LootDrop.ItemDefinition != AmmoDef && AmmoDef->DropCount > 0)
                                                        {
                                                                SpawnPickup(AmmoDef, AmmoDef->DropCount, 0, CorrectLocation, PickupSourceTypeFlags, SpawnSource);
                                                        }
                                                }
                                        }

                                }
                        }
                }

                return true;
        }

        void SpawnFloorLoot()
        {
                auto Statics = OgsSDK::Statics();

                TArray<AActor*> FloorLootSpawners;
                UClass* SpawnerClass = StaticLoadObject<UClass>("/Game/Athena/Environments/Blueprints/Tiered_Athena_FloorLoot_Warmup.Tiered_Athena_FloorLoot_Warmup_C");
                Statics->GetAllActorsOfClass(UWorld::GetWorld(), SpawnerClass, &FloorLootSpawners);

                for (size_t i = 0; i < FloorLootSpawners.Num(); i++)
                {
                        FloorLootSpawners[i]->K2_DestroyActor();
                }

                FloorLootSpawners.Free();

                SpawnerClass = StaticLoadObject<UClass>("/Game/Athena/Environments/Blueprints/Tiered_Athena_FloorLoot_01.Tiered_Athena_FloorLoot_01_C");
                Statics->GetAllActorsOfClass(UWorld::GetWorld(), SpawnerClass, &FloorLootSpawners);

                for (size_t i = 0; i < FloorLootSpawners.Num(); i++)
                {
                        FloorLootSpawners[i]->K2_DestroyActor();
                }

                FloorLootSpawners.Free();
        }

        void Hook() {
                OGS_HOOK_NATIVE_OFF(Ogs::Off().spawn_loot, SpawnLoot, nullptr);

                Log("Looting Hooked!");
        }
}

// =============================================================================
// Inventory.h (orig)
// =============================================================================
namespace Inventory {

        bool DontPlayAnimation = false;

        UFortItemDefinition* FindDefFromGuid(AFortPlayerControllerAthena* PC, FGuid Guid) {
                OGS_CHECK(PC && PC->WorldInventory);

                for (int32 i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemGuid == Guid) {
                                return (UFortItemDefinition*)PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemDefinition;
                        }
                }

                return nullptr;
        }

        EFortQuickBars GetQuickBars(UFortItemDefinition* ItemDefinition)
        {
                OGS_CHECK(ItemDefinition);
                if (!ItemDefinition->IsA(UFortWeaponMeleeItemDefinition::StaticClass()) && !ItemDefinition->IsA(UFortEditToolItemDefinition::StaticClass()) &&
                        !ItemDefinition->IsA(UFortBuildingItemDefinition::StaticClass()) && !ItemDefinition->IsA(UFortAmmoItemDefinition::StaticClass()) && !ItemDefinition->IsA(UFortResourceItemDefinition::StaticClass()) && !ItemDefinition->IsA(UFortTrapItemDefinition::StaticClass()) && !ItemDefinition->IsA(AFortWeaponRangedForVehicle::StaticClass()))
                        return EFortQuickBars::Primary;

                return EFortQuickBars::Secondary;
        }

        bool IsPrimaryQuickbar(UFortItemDefinition* Def)
        {
                return Def->IsA(UFortConsumableItemDefinition::StaticClass()) || Def->IsA(UFortWeaponRangedItemDefinition::StaticClass()) || Def->IsA(UFortGadgetItemDefinition::StaticClass());
        }

        bool IsInventoryFull(AFortPlayerController* PC)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                int ItemNb = 0;
                auto InstancesPtr = &PC->WorldInventory->Inventory.ItemInstances;
                for (int i = 0; i < InstancesPtr->Num(); i++)
                {
                        if (InstancesPtr->operator[](i))
                        {
                                if (GetQuickBars(InstancesPtr->operator[](i)->ItemEntry.ItemDefinition) == EFortQuickBars::Primary)
                                {
                                        ItemNb++;

                                        if (ItemNb >= 5)
                                        {
                                                break;
                                        }
                                }
                        }
                }

                return ItemNb >= 5;
        }

        inline int GetMagSize(UFortWeaponItemDefinition* Def)
        {
                if (!Def)
                        return 0;

                UDataTable* Table = Def->WeaponStatHandle.DataTable;
                if (!Table)
                        return 0;
                int Ret = 0;
                // +0x30 raw RowMap access → reflected UDataTable::RowMap
                auto& RowMap = Table->RowMap;
                for (auto& Pair : RowMap)
                {
                        if (Pair.Key().ToString() == Def->WeaponStatHandle.RowName.ToString())
                        {
                                Ret = ((FFortRangedWeaponStats*)Pair.Value())->ClipSize;
                        }
                }
                return Ret;
        }

        inline int GetMaxStackSize(UFortItemDefinition* Def) // doesnt work with consumables for some reason
        {
                OGS_CHECK(Def);
                // Original read *(float*)(__int64(Def) + 0x178).  0x178 is exactly
                // &Def->MaxStackSize.Curve.RowName (MaxStackSize@0x168, Curve@+0x8,
                // RowName@+0x8) — preserved byte-faithfully through the SDK member.
                float Value = *reinterpret_cast<float*>(&Def->MaxStackSize.Curve.RowName);
                auto Table = Def->MaxStackSize.Curve.CurveTable;
                EEvaluateCurveTableResult Res;
                float Out = 0.f;
                UDataTableFunctionLibrary::EvaluateCurveTableRow(Table, Def->MaxStackSize.Curve.RowName, 0, &Res, &Out, FString());
                if (!Table || Out <= 0)
                        Out = Value;
                return Out;
        }

        //having 2 of these seems wrong
        float GetMaxStack(UFortItemDefinition* Def) //consumables
        {
                OGS_CHECK(Def);
                if (!Def->MaxStackSize.Curve.CurveTable)
                        return Def->MaxStackSize.Value;
                EEvaluateCurveTableResult Result;
                float Ret = 0.f;
                ((UDataTableFunctionLibrary*)UDataTableFunctionLibrary::StaticClass()->DefaultObject)->EvaluateCurveTableRow(Def->MaxStackSize.Curve.CurveTable, Def->MaxStackSize.Curve.RowName, 0, &Result, &Ret, FString());
                return Ret;
        }

        inline void UpdateInventory(AFortPlayerController* PC, FFortItemEntry& Entry)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                for (size_t i = 0; i < PC->WorldInventory->Inventory.ItemInstances.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ItemInstances[i]->ItemEntry.ItemGuid == Entry.ItemGuid)
                        {
                                PC->WorldInventory->Inventory.ItemInstances[i]->ItemEntry = Entry;
                                PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                break;
                        }
                }
        }

        void ModifyEntry(AFortPlayerControllerAthena* PC, FFortItemEntry& Entry)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                for (int32 i = 0; i < PC->WorldInventory->Inventory.ItemInstances.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ItemInstances[i]->ItemEntry.ItemGuid == Entry.ItemGuid)
                        {
                                PC->WorldInventory->Inventory.ItemInstances[i]->ItemEntry = Entry;
                                PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                break;
                        }
                }
        }

        void UpdateLoadedAmmo(AFortPlayerController* PC, AFortWeapon* Weapon)
        {
                OGS_CHECK(PC && PC->WorldInventory && Weapon);
                for (int32 i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemGuid == Weapon->ItemEntryGuid)
                        {
                                PC->WorldInventory->Inventory.ReplicatedEntries[i].LoadedAmmo = Weapon->AmmoCount;
                                UpdateInventory((AFortPlayerControllerAthena*)PC, PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                break;
                        }
                }
        }

        void UpdateLoadedAmmo(AFortPlayerController* PC, AFortWeapon* Weapon, int AmountToAdd)
        {
                OGS_CHECK(PC && PC->WorldInventory && Weapon);
                for (int32 i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemGuid == Weapon->ItemEntryGuid)
                        {
                                PC->WorldInventory->Inventory.ReplicatedEntries[i].LoadedAmmo += AmountToAdd;
                                ModifyEntry((AFortPlayerControllerAthena*)PC, PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                UpdateInventory((AFortPlayerControllerAthena*)PC, PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                break;
                        }
                }
        }

        // GetInterface (framework.h): the raw ImageBase+0x2E0D910 resolver is
        // registry-gated; null → callers log "skipped (offset missing)".
        template <typename _It>
        static _It* GetInterface(UObject* Object)
        {
                auto resolver = OgsFn::get_interface();
                if (!resolver)
                {
                        OGS_LOG("GetInterface skipped (offset missing)");
                        return nullptr;
                }
                return (_It*)resolver(Object, _It::StaticClass());
        }

        inline void GiveItem(AFortPlayerController* PC, UFortItemDefinition* Def, int Count, int LoadedAmmo)
        {
                OGS_CHECK(PC && PC->WorldInventory && Def);

                UFortWorldItem* Item = Cast<UFortWorldItem>(Def->CreateTemporaryItemInstanceBP(Count, 0));
                OGS_CHECK(Item);
                Item->SetOwningControllerForTemporaryItem(PC);
                Item->OwnerInventory = PC->WorldInventory;
                Item->ItemEntry.LoadedAmmo = LoadedAmmo;

                PC->WorldInventory->Inventory.ReplicatedEntries.Add(Item->ItemEntry);
                PC->WorldInventory->Inventory.ItemInstances.Add(Item);

                if (auto Gadget = Cast<UFortGadgetItemDefinition>(Def))
                {
                        auto gadgetAdd = OgsFn::gadget_add();
                        if (gadgetAdd)
                                gadgetAdd(Gadget, GetInterface<IFortInventoryOwnerInterface>(PC), Item, 1);
                        else
                                OGS_LOG("gadget add skipped (offset missing)");
                }

                PC->WorldInventory->Inventory.MarkItemDirty(Item->ItemEntry);
                PC->WorldInventory->HandleInventoryLocalUpdate();
        }

        void UpdateStack(AFortPlayerController* PC, bool Update, FFortItemEntry* EntryToUpdate = nullptr)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                PC->WorldInventory->bRequiresLocalUpdate = true;
                PC->WorldInventory->HandleInventoryLocalUpdate();
                PC->HandleWorldInventoryLocalUpdate();
                PC->ClientForceUpdateQuickbar(EFortQuickBars::Primary);
                PC->ClientForceUpdateQuickbar(EFortQuickBars::Secondary);

                if (Update)
                {
                        PC->WorldInventory->Inventory.MarkItemDirty(*EntryToUpdate);
                }
                else
                {
                        PC->WorldInventory->Inventory.MarkArrayDirty();
                }
        }

        FFortItemEntry* GiveStack(AFortPlayerControllerAthena* PC, UFortItemDefinition* Def, int Count = 1, bool GiveLoadedAmmo = false, int LoadedAmmo = 0, bool Toast = false)
        {
                OGS_CHECK(PC && PC->WorldInventory && Def);
                UFortWorldItem* Item = (UFortWorldItem*)Def->CreateTemporaryItemInstanceBP(Count, 0);
                OGS_CHECK(Item);

                Item->SetOwningControllerForTemporaryItem(PC);
                Item->OwnerInventory = PC->WorldInventory;
                Item->ItemEntry.ItemDefinition = Def;
                Item->ItemEntry.Count = Count;

                if (GiveLoadedAmmo)
                {
                        Item->ItemEntry.LoadedAmmo = LoadedAmmo;
                }
                Item->ItemEntry.ReplicationKey++;

                PC->WorldInventory->Inventory.ReplicatedEntries.Add(Item->ItemEntry);
                PC->WorldInventory->Inventory.ItemInstances.Add(Item);

                UpdateStack(PC, false);
                return &Item->ItemEntry;
        }

        void GiveItemStack(AFortPlayerController* PC, UFortItemDefinition* Def, int Count, int LoadedAmmo)
        {
                OGS_CHECK(PC && PC->WorldInventory && Def);
                EEvaluateCurveTableResult Result;
                float OutXY = 0;
                UDataTableFunctionLibrary::EvaluateCurveTableRow(Def->MaxStackSize.Curve.CurveTable, Def->MaxStackSize.Curve.RowName, 0, &Result, &OutXY, FString());
                if (!Def->MaxStackSize.Curve.CurveTable || OutXY <= 0)
                        OutXY = Def->MaxStackSize.Value;
                FFortItemEntry* Found = nullptr;
                for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemDefinition == Def)
                        {
                                Found = &PC->WorldInventory->Inventory.ReplicatedEntries[i];
                                PC->WorldInventory->Inventory.ReplicatedEntries[i].Count += Count;
                                if (PC->WorldInventory->Inventory.ReplicatedEntries[i].Count > OutXY)
                                {
                                        PC->WorldInventory->Inventory.ReplicatedEntries[i].Count = (int)OutXY;
                                }
                                PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                UpdateInventory(PC, PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                PC->WorldInventory->HandleInventoryLocalUpdate();
                                return;
                        }
                }

                if (!Found)
                {
                        GiveItem(PC, Def, Count, LoadedAmmo);
                }
        }

        void RemoveItem(AFortPlayerController* PC, UFortItemDefinition* Def, int Count)
        {
                OGS_CHECK(PC && PC->WorldInventory && Def);
                bool Remove = false;
                FGuid guid;
                for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        auto& Entry = PC->WorldInventory->Inventory.ReplicatedEntries[i];
                        if (Entry.ItemDefinition == Def)
                        {
                                Entry.Count -= Count;
                                if (Entry.Count <= 0)
                                {
                                        PC->WorldInventory->Inventory.ReplicatedEntries[i].StateValues.Free();
                                        PC->WorldInventory->Inventory.ReplicatedEntries.RemoveSingle((int)i);
                                        Remove = true;
                                        guid = Entry.ItemGuid;
                                }
                                else
                                {
                                        PC->WorldInventory->Inventory.MarkItemDirty(PC->WorldInventory->Inventory.ReplicatedEntries[i]);
                                        UpdateInventory(PC, Entry);
                                        PC->WorldInventory->HandleInventoryLocalUpdate();
                                        return;
                                }
                                break;
                        }
                }

                if (Remove)
                {
                        for (size_t i = 0; i < PC->WorldInventory->Inventory.ItemInstances.Num(); i++)
                        {
                                if (PC->WorldInventory->Inventory.ItemInstances[i]->GetItemGuid() == guid)
                                {
                                        if (auto Gadget = Cast<UFortGadgetItemDefinition>(PC->WorldInventory->Inventory.ItemInstances[i]->ItemEntry.ItemDefinition))
                                        {
                                                auto gadgetRemove = OgsFn::gadget_remove();
                                                if (gadgetRemove)
                                                        gadgetRemove(Gadget, GetInterface<IFortInventoryOwnerInterface>(PC), PC->WorldInventory->Inventory.ItemInstances[i]);
                                                else
                                                        OGS_LOG("gadget remove skipped (offset missing)");
                                        }
                                        PC->WorldInventory->Inventory.ItemInstances.RemoveSingle((int)i);
                                        break;
                                }
                        }
                }

                PC->WorldInventory->Inventory.MarkArrayDirty();
                PC->WorldInventory->HandleInventoryLocalUpdate();
        }

        inline void RemoveItem(AFortPlayerController* PC, FGuid Guid, int Count)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                for (auto& Entry : PC->WorldInventory->Inventory.ReplicatedEntries)
                {
                        if (Entry.ItemGuid == Guid)
                        {
                                RemoveItem(PC, Entry.ItemDefinition, Count);
                                break;
                        }
                }
        }

        inline FFortItemEntry* FindEntry(AFortPlayerController* PC, FGuid Guid)
        {
                OGS_CHECK(PC && PC->WorldInventory);
                for (auto& Entry : PC->WorldInventory->Inventory.ReplicatedEntries)
                {
                        if (Entry.ItemGuid == Guid)
                        {
                                return &Entry;
                        }
                }
                return nullptr;
        }

        FFortItemEntry* FindItemEntry(AFortPlayerController* PC, UFortItemDefinition* ItemDef)
        {
                if (!PC || !PC->WorldInventory || !ItemDef)
                        return nullptr;
                for (int i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); ++i)
                {
                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemDefinition == ItemDef)
                        {
                                return &PC->WorldInventory->Inventory.ReplicatedEntries[i];
                        }
                }
                return nullptr;
        }

        UFortWorldItem* FindItemInstance(AFortInventory* inv, UFortItemDefinition* ItemDefinition)
        {
                if (!inv)
                        return nullptr;
                auto& ItemInstances = inv->Inventory.ItemInstances;

                for (int i = 0; i < ItemInstances.Num(); i++)
                {
                        auto ItemInstance = ItemInstances[i];

                        if (ItemInstance->ItemEntry.ItemDefinition == ItemDefinition)
                                return ItemInstance;
                }

                return nullptr;
        }

        UFortWorldItem* FindItemInstance(AFortInventory* inv, const FGuid& Guid)
        {
                if (!inv)
                        return nullptr;
                auto& ItemInstances = inv->Inventory.ItemInstances;

                for (int i = 0; i < ItemInstances.Num(); i++)
                {
                        auto ItemInstance = ItemInstances[i];

                        if (ItemInstance->ItemEntry.ItemGuid == Guid)
                                return ItemInstance;
                }

                return nullptr;
        }

        inline void ServerExecuteInventoryItem(AFortPlayerControllerAthena* PC, FGuid Guid)
        {
                OGS_CHECK(PC);
                if (!PC->MyFortPawn || !PC->Pawn)
                        return;

                UFortItemDefinition* ItemDef = FindDefFromGuid(PC, Guid);
                if (ItemDef) {
                        UFortWeaponItemDefinition* DefToEquip = (UFortWeaponItemDefinition*)ItemDef;
                        if (ItemDef->IsA(UFortGadgetItemDefinition::StaticClass()))
                        {
                                DefToEquip = ((UFortGadgetItemDefinition*)ItemDef)->GetWeaponItemDefinition();
                        }
                        else if (ItemDef->IsA(UFortDecoItemDefinition::StaticClass())) {
                                auto DecoItemDefinition = (UFortDecoItemDefinition*)ItemDef;
                                PC->MyFortPawn->PickUpActor(nullptr, DecoItemDefinition);
                                PC->MyFortPawn->CurrentWeapon->ItemEntryGuid = Guid;
                                static auto FortDecoTool_ContextTrapStaticClass = StaticLoadObject<UClass>("/Script/FortniteGame.FortDecoTool_ContextTrap");
                                if (PC->MyFortPawn->CurrentWeapon->IsA(FortDecoTool_ContextTrapStaticClass))
                                {
                                        reinterpret_cast<AFortDecoTool_ContextTrap*>(PC->MyFortPawn->CurrentWeapon)->ContextTrapItemDefinition = (UFortContextTrapItemDefinition*)ItemDef;
                                }
                                return;
                        }
                        PC->MyFortPawn->EquipWeaponDefinition(DefToEquip, Guid);
                }
        }

        inline void (*ServerHandlePickupOG)(AFortPlayerPawn* Pawn, AFortPickup* Pickup, float InFlyTime, FVector InStartDirection, bool bPlayPickupSound);
        inline void ServerHandlePickup(AFortPlayerPawnAthena* Pawn, AFortPickup* Pickup, float InFlyTime, const FVector& InStartDirection, bool bPlayPickupSound)
        {
                if (!Pickup || !Pawn || !Pawn->Controller || Pickup->bPickedUp)
                        return;

                AFortPlayerControllerAthena* PC = (AFortPlayerControllerAthena*)Pawn->Controller;

                UFortItemDefinition* Def = Pickup->PrimaryPickupItemEntry.ItemDefinition;
                if (!Def)
                        return;
                FFortItemEntry* FoundEntry = nullptr;
                FFortItemEntry& PickupEntry = Pickup->PrimaryPickupItemEntry;
                float MaxStackSize = GetMaxStack(Def);
                bool Stackable = Def->IsStackable();
                UFortItemDefinition* PickupItemDef = PickupEntry.ItemDefinition;
                bool Found = false;
                FFortItemEntry* GaveEntry = nullptr;

                if (IsInventoryFull(PC))
                {
                        if (Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortAmmoItemDefinition::StaticClass()) || Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortResourceItemDefinition::StaticClass()) || Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortTrapItemDefinition::StaticClass()))
                        {
                                GiveItemStack(PC, Pickup->PrimaryPickupItemEntry.ItemDefinition, Pickup->PrimaryPickupItemEntry.Count, Pickup->PrimaryPickupItemEntry.LoadedAmmo);

                                Pickup->PickupLocationData.bPlayPickupSound = true;
                                Pickup->PickupLocationData.FlyTime = 0.3f;
                                Pickup->PickupLocationData.ItemOwner = Pawn;
                                Pickup->PickupLocationData.PickupGuid = Pickup->PrimaryPickupItemEntry.ItemGuid;
                                Pickup->PickupLocationData.PickupTarget = Pawn;
                                Pickup->OnRep_PickupLocationData();

                                Pickup->bPickedUp = true;
                                Pickup->OnRep_bPickedUp();
                                return;
                        }

                        if (!Pawn->CurrentWeapon || !Pawn->CurrentWeapon->WeaponData)
                                return;

                        if (!Pawn->CurrentWeapon->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass()))
                        {
                                if (Stackable)
                                {
                                        for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                        {
                                                FFortItemEntry& Entry = PC->WorldInventory->Inventory.ReplicatedEntries[i];

                                                if (Entry.ItemDefinition == PickupItemDef)
                                                {
                                                        Found = true;
                                                        if ((MaxStackSize - Entry.Count) > 0)
                                                        {
                                                                Entry.Count += PickupEntry.Count;

                                                                if (Entry.Count > MaxStackSize)
                                                                {
                                                                        SpawnStack((APlayerPawn_Athena_C*)PC->Pawn, PickupItemDef, (int)(Entry.Count - MaxStackSize));
                                                                        Entry.Count = (int)MaxStackSize;
                                                                }

                                                                PC->WorldInventory->Inventory.MarkItemDirty(Entry);
                                                        }
                                                        else
                                                        {
                                                                if (IsPrimaryQuickbar(PickupItemDef))
                                                                {
                                                                        GaveEntry = GiveStack(PC, PickupItemDef, PickupEntry.Count);
                                                                }
                                                        }
                                                        break;
                                                }
                                        }
                                        if (!Found)
                                        {
                                                for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                                {
                                                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemGuid == Pawn->CurrentWeapon->GetInventoryGUID())
                                                        {
                                                                PC->ServerAttemptInventoryDrop(Pawn->CurrentWeapon->GetInventoryGUID(), PC->WorldInventory->Inventory.ReplicatedEntries[i].Count, false);
                                                                break;
                                                        }
                                                }
                                                GaveEntry = GiveStack(PC, PickupItemDef, PickupEntry.Count, false, 0, true);
                                        }

                                        Pickup->PickupLocationData.bPlayPickupSound = true;
                                        Pickup->PickupLocationData.FlyTime = 0.3f;
                                        Pickup->PickupLocationData.ItemOwner = Pawn;
                                        Pickup->PickupLocationData.PickupGuid = Pickup->PrimaryPickupItemEntry.ItemGuid;
                                        Pickup->PickupLocationData.PickupTarget = Pawn;
                                        Pickup->OnRep_PickupLocationData();

                                        Pickup->bPickedUp = true;
                                        Pickup->OnRep_bPickedUp();
                                        return;
                                }

                                for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                {
                                        if (PC->WorldInventory->Inventory.ReplicatedEntries[i].ItemGuid == Pawn->CurrentWeapon->GetInventoryGUID())
                                        {
                                                PC->ServerAttemptInventoryDrop(Pawn->CurrentWeapon->GetInventoryGUID(), PC->WorldInventory->Inventory.ReplicatedEntries[i].Count, false);
                                                break;
                                        }
                                }
                        }
                }

                if (!IsInventoryFull(PC))
                {
                        if (Stackable && !Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortAmmoItemDefinition::StaticClass()) || Stackable && !Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortResourceItemDefinition::StaticClass()) || Stackable && Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortTrapItemDefinition::StaticClass()))
                        {
                                for (size_t i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                {
                                        FFortItemEntry& Entry = PC->WorldInventory->Inventory.ReplicatedEntries[i];

                                        if (Entry.ItemDefinition == PickupItemDef)
                                        {
                                                Found = true;
                                                if ((MaxStackSize - Entry.Count) > 0)
                                                {
                                                        Entry.Count += PickupEntry.Count;

                                                        if (Entry.Count > MaxStackSize)
                                                        {
                                                                SpawnStack((APlayerPawn_Athena_C*)PC->Pawn, PickupItemDef, (int)(Entry.Count - MaxStackSize));
                                                                Entry.Count = (int)MaxStackSize;
                                                        }

                                                        PC->WorldInventory->Inventory.MarkItemDirty(Entry);
                                                }
                                                else
                                                {
                                                        if (IsPrimaryQuickbar(PickupItemDef))
                                                        {
                                                                GaveEntry = GiveStack(PC, PickupItemDef, PickupEntry.Count);
                                                        }
                                                }
                                                break;
                                        }
                                }
                                if (!Found)
                                {
                                        GaveEntry = GiveStack(PC, PickupItemDef, PickupEntry.Count, false, 0, true);
                                }

                                Pickup->PickupLocationData.bPlayPickupSound = true;
                                Pickup->PickupLocationData.FlyTime = 0.3f;
                                Pickup->PickupLocationData.ItemOwner = Pawn;
                                Pickup->PickupLocationData.PickupGuid = Pickup->PrimaryPickupItemEntry.ItemGuid;
                                Pickup->PickupLocationData.PickupTarget = Pawn;
                                Pickup->OnRep_PickupLocationData();

                                Pickup->bPickedUp = true;
                                Pickup->OnRep_bPickedUp();
                                return;
                        }

                        if (Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortAmmoItemDefinition::StaticClass()) || Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortResourceItemDefinition::StaticClass()) || Pickup->PrimaryPickupItemEntry.ItemDefinition->IsA(UFortTrapItemDefinition::StaticClass()))
                        {
                                GiveItemStack(PC, Pickup->PrimaryPickupItemEntry.ItemDefinition, Pickup->PrimaryPickupItemEntry.Count, Pickup->PrimaryPickupItemEntry.LoadedAmmo);
                        }
                        else {
                                GiveItem(PC, Pickup->PrimaryPickupItemEntry.ItemDefinition, Pickup->PrimaryPickupItemEntry.Count, Pickup->PrimaryPickupItemEntry.LoadedAmmo);
                        }
                }

                Pickup->PickupLocationData.bPlayPickupSound = true;
                Pickup->PickupLocationData.FlyTime = 0.3f;
                Pickup->PickupLocationData.ItemOwner = Pawn;
                Pickup->PickupLocationData.PickupGuid = Pickup->PrimaryPickupItemEntry.ItemGuid;
                Pickup->PickupLocationData.PickupTarget = Pawn;
                Pickup->OnRep_PickupLocationData();

                Pickup->bPickedUp = true;
                Pickup->OnRep_bPickedUp();
        }

        inline void ServerAttemptInventoryDrop(AFortPlayerControllerAthena* PC, FGuid ItemGuid, int Count, bool bTrash)
        {
                if (Globals::Arsenal)
                        return;

                FFortItemEntry* Entry = FindEntry(PC, ItemGuid);
                AFortPlayerPawn* Pawn = (AFortPlayerPawn*)PC->Pawn;
                if (!Entry)
                        return;
                SpawnPickup(Entry->ItemDefinition, Count, Entry->LoadedAmmo, PC->Pawn->K2_GetActorLocation(), EFortPickupSourceTypeFlag::Player, EFortPickupSpawnSource::Unset, Pawn);

                if (Entry->ItemDefinition == StaticLoadObject<UFortItemDefinition>("/Game/Athena/Items/Consumables/Shields/Athena_Shields.Athena_Shields") || Entry->ItemDefinition == StaticLoadObject<UFortItemDefinition>("/Game/Athena/Items/Consumables/Medkit/Athena_Medkit.Athena_Medkit"))
                {
                        DontPlayAnimation = true;
                }

                RemoveItem(PC, ItemGuid, Count);
        }

        int64_t(*OnReloadOG)(AFortWeapon* Weapon, int RemoveCount);
        int64_t OnReload(AFortWeapon* Weapon, int RemoveCount)
        {
                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->AuthorityGameMode && World->GameState);
                AFortGameModeAthena* GameMode = (AFortGameModeAthena*)World->AuthorityGameMode;

                auto GameState = (AFortGameStateAthena*)GameMode->GameState;

                auto Ret = OnReloadOG(Weapon, RemoveCount);
                auto WeaponDef = Weapon->WeaponData;
                if (!WeaponDef)
                        return Ret;

                auto AmmoDef = WeaponDef->GetAmmoWorldItemDefinition_BP();
                if (!AmmoDef)
                        return Ret;

                AFortPlayerPawnAthena* Pawn = (AFortPlayerPawnAthena*)Weapon->GetOwner();
                if (!Pawn)
                        return Ret;
                AFortPlayerControllerAthena* PC = (AFortPlayerControllerAthena*)Pawn->Controller;
                AFortPlayerStateAthena* PlayerState = (AFortPlayerStateAthena*)(PC ? PC->PlayerState : nullptr);

                if (!PC || !PC->Pawn || !PC->IsA(AFortPlayerControllerAthena::StaticClass()) || !PC->WorldInventory || GameState->GamePhase >= EAthenaGamePhase::EndGame)
                        return Ret;

                if (PC->bInfiniteAmmo) {
                        UpdateLoadedAmmo(PC, Weapon, RemoveCount);
                        return Ret;
                }

                int AmmoCount = 0;
                FFortItemEntry* FoundEntry = nullptr;
                for (int32 i = 0; i < PC->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        FFortItemEntry& Entry = PC->WorldInventory->Inventory.ReplicatedEntries[i];

                        if (Entry.ItemDefinition == AmmoDef) {
                                AmmoCount = Entry.Count;
                                FoundEntry = &Entry;
                                break;
                        }
                }

                int AmmoToRemove = (RemoveCount < AmmoCount) ? RemoveCount : AmmoCount;

                if (AmmoToRemove > 0) {
                        RemoveItem(PC, AmmoDef, AmmoToRemove);
                        UpdateLoadedAmmo(PC, Weapon, AmmoToRemove);
                }

                if (PlayerState && PlayerState->AbilitySystemComponent)
                {
                        if (WeaponDef == StaticLoadObject<UFortItemDefinition>("/Game/Athena/Items/Consumables/Shields/Athena_Shields.Athena_Shields") && !DontPlayAnimation)
                        {
                                FGameplayEffectContextHandle Handle = PlayerState->AbilitySystemComponent->MakeEffectContext();
                                FGameplayTag tag{};
                                static auto name = UKismetStringLibrary::GetDefaultObj()->Conv_StringToName(OGS_TEXT("GameplayCue.Shield.PotionConsumed"));
                                tag.TagName = name;

                                PlayerState->AbilitySystemComponent->NetMulticast_InvokeGameplayCueExecuted(tag, FPredictionKey(), Handle);
                        }

                        if (WeaponDef == StaticLoadObject<UFortItemDefinition>("/Game/Athena/Items/Consumables/Medkit/Athena_Medkit.Athena_Medkit") && !DontPlayAnimation)
                        {
                                FGameplayEffectContextHandle Handle = PlayerState->AbilitySystemComponent->MakeEffectContext();
                                FGameplayTag tag;
                                static auto name = UKismetStringLibrary::GetDefaultObj()->Conv_StringToName(OGS_TEXT("GameplayCue.Athena.Health.HealUsed"));
                                tag.TagName = name;

                                PlayerState->AbilitySystemComponent->NetMulticast_InvokeGameplayCueExecuted(tag, FPredictionKey(), Handle);
                        }
                }

                PC->WorldInventory->bRequiresLocalUpdate = true;
                PC->WorldInventory->HandleInventoryLocalUpdate();

                if (DontPlayAnimation)
                {
                        DontPlayAnimation = false;
                }

                return Ret;
        }

        inline void (*NetMulticastDamageCuesOG)(AFortPlayerPawnAthena* Pawn, FAthenaBatchedDamageGameplayCues_Shared SharedData, FAthenaBatchedDamageGameplayCues_NonShared NonSharedData);
        inline void NetMulticastDamageCues(AFortPlayerPawnAthena* Pawn, FAthenaBatchedDamageGameplayCues_Shared SharedData, FAthenaBatchedDamageGameplayCues_NonShared NonSharedData)
        {
                if (!Pawn || (Pawn->Controller && Pawn->Controller->IsA(ABP_PhoebePlayerController_C::StaticClass())))
                        return;

                if (Pawn->CurrentWeapon)
                        UpdateLoadedAmmo((AFortPlayerController*)Pawn->Controller, ((AFortPlayerPawn*)Pawn)->CurrentWeapon);

                return NetMulticastDamageCuesOG(Pawn, SharedData, NonSharedData);
        }

        static inline void (*GiveItemToInventoryOwnerOG)(UObject* Object, FFrame& Stack);
        static void GiveItemToInventoryOwner(UObject* Object, FFrame& Stack) {
                TScriptInterface<class IFortInventoryOwnerInterface> InventoryOwner;
                UFortWorldItemDefinition* ItemDefinition;
                int32 NumberToGive;
                bool bNotifyPlayer;
                int32 ItemLevel;
                int32 PickupInstigatorHandle;
                Stack.StepCompiledIn(&InventoryOwner);
                Stack.StepCompiledIn(&ItemDefinition);
                Stack.StepCompiledIn(&NumberToGive);
                Stack.StepCompiledIn(&bNotifyPlayer);
                Stack.StepCompiledIn(&ItemLevel);
                Stack.StepCompiledIn(&PickupInstigatorHandle);

                Log("Well well well!");

                auto PC = (AFortPlayerControllerAthena*)InventoryOwner.ObjectPointer;
                GiveItem(PC, ItemDefinition, 1, 1);
                if (GiveItemToInventoryOwnerOG)
                        return GiveItemToInventoryOwnerOG(Object, Stack);
        }

        bool ServerRemoveInventoryItem(AFortPlayerControllerAthena* PlayerController, FGuid ItemGuid, int Count, bool bForceRemoval, bool bForcePersistWhenEmpty)
        {
                Log("ServerRemoveInventoryItem!");
                RemoveItem(PlayerController, ItemGuid, Count);

                return true;
        }

        static inline int32(*K2_RemoveItemFromPlayerOG)(AFortPlayerControllerAthena* PC, UFortWorldItemDefinition* ItemDefinition, int32 AmountToRemove, bool bForceRemoval);
        static int32 K2_RemoveItemFromPlayer(AFortPlayerControllerAthena* PC, UFortWorldItemDefinition* ItemDefinition, int32 AmountToRemove, bool bForceRemoval) {
                Log("K2_RemoveItemFromPlayer!");
                RemoveItem(PC, ItemDefinition, INT_MAX);
                return AmountToRemove;
        }

        static inline int32(*K2_RemoveItemFromPlayerByGuidOG)(UObject* Object, FFrame& Stack, int32* Ret);
        int32 K2_RemoveItemFromPlayerByGuid(UObject* Context, FFrame& Stack, int32* Ret)
        {
                class AFortPlayerControllerAthena* PlayerController;
                struct FGuid ItemGuid;
                int32 AmountToRemove;
                bool bForceRemoval;
                Stack.StepCompiledIn(&PlayerController);
                Stack.StepCompiledIn(&ItemGuid);
                Stack.StepCompiledIn(&AmountToRemove);
                Stack.StepCompiledIn(&bForceRemoval);

                auto RemoveCount = AmountToRemove - 1;

                Log("wow wow wow!");
                RemoveItem(PlayerController, ItemGuid, 1);

                return RemoveCount;
        }

        //lategame
        std::string GetRandomWeapon(const std::vector<std::string>& list) {
                if (list.empty()) return "";
                static std::random_device rd;
                static std::mt19937 gen(rd());
                std::uniform_int_distribution<> dist(0, (int)list.size() - 1);
                return list[dist(gen)];
        }

        UFortItemDefinition* LoadWeapon(const std::vector<std::string>& Pool)// fixes issue where deff is somehow null
        {
                for (int i = 0; i < 2; ++i)
                {
                        std::string WeaponPath = GetRandomWeapon(Pool);
                        UFortItemDefinition* Def = StaticLoadObject<UFortItemDefinition>(WeaponPath);
                        if (Def)
                                return Def;
                }
                return nullptr;
        }

        void GiveLoadout(AFortPlayerController* PC)
        {
                if (UFortItemDefinition* AssaultRifleDef = LoadWeapon(Assault_rifle))
                {
                        GiveItem(PC, AssaultRifleDef, 1, Looting::GetClipSize(AssaultRifleDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)AssaultRifleDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItem(PC, AmmoDef, 200, 0);
                        }
                }

                if (UFortItemDefinition* ShotGunDef = LoadWeapon(Shotgun))
                {
                        GiveItem(PC, ShotGunDef, 1, Looting::GetClipSize(ShotGunDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)ShotGunDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItem(PC, AmmoDef, 120, 0);
                        }
                }

                if (UFortItemDefinition* RandomDef = LoadWeapon(Mixed))
                {
                        GiveItem(PC, RandomDef, 1, Looting::GetClipSize(RandomDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)RandomDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItem(PC, AmmoDef, 120, 0);
                        }
                }

                if (auto Consumable1Def = LoadWeapon(Consumables))
                        GiveItemStack(PC, Consumable1Def, 3, 0);

                if (auto Consumable2Def = LoadWeapon(Consumables))
                        GiveItemStack(PC, Consumable2Def, 3, 0);

                if (auto TrapDef = LoadWeapon(Traps))
                        GiveItem(PC, TrapDef, 3, 0);

                static UFortItemDefinition* WoodDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/WoodItemData.WoodItemData");
                GiveItem(PC, WoodDef, 500, 0);

                static UFortItemDefinition* StoneDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/StoneItemData.StoneItemData");
                GiveItem(PC, StoneDef, 500, 0);

                static UFortItemDefinition* MetalDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/MetalItemData.MetalItemData");
                GiveItem(PC, MetalDef, 500, 0);
        }

        void Hook() {
                // HookVTable(AFortPlayerControllerAthena, 0x20D/0x209, ...) → ByExec
                OgsHook::ByExec(AFortPlayerControllerAthena::GetDefaultObj(), "FortPlayerControllerAthena", "ServerExecuteInventoryItem", (void*)ServerExecuteInventoryItem, nullptr);

                OgsHook::ByExec(APlayerPawn_Athena_C::GetDefaultObj(), "PlayerPawn_Athena_C", "ServerHandlePickup", (void*)ServerHandlePickup, (void**)&ServerHandlePickupOG);

                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerAttemptInventoryDrop", (void*)ServerAttemptInventoryDrop, nullptr);

                OgsHook::ByExec(AFortPlayerPawnAthena::GetDefaultObj(), "FortPlayerPawnAthena", "NetMulticastDamageCues", (void*)NetMulticastDamageCues, (void**)&NetMulticastDamageCuesOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_reload, OnReload, (void**)&OnReloadOG);

                if (Ogs::CfgRef().Has("exec_hooks"))
                {
                        OgsHook::ExecSwap("/Script/FortniteGame.FortKismetLibrary.GiveItemToInventoryOwner", (void*)GiveItemToInventoryOwner, (void**)&GiveItemToInventoryOwnerOG);
                }
                else
                {
                        Ogs::Log("exec_hooks off — GiveItemToInventoryOwner not swapped");
                }

                OgsHook::ByExec(AFortPlayerControllerAthena::GetDefaultObj(), "FortPlayerControllerAthena", "ServerRemoveInventoryItem", (void*)ServerRemoveInventoryItem, nullptr);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().k2_remove_item_from_player, K2_RemoveItemFromPlayer, nullptr);

                if (Ogs::CfgRef().Has("exec_hooks"))
                {
                        OgsHook::ExecSwap("/Script/FortniteGame.FortKismetLibrary.K2_RemoveItemFromPlayerByGuid", (void*)K2_RemoveItemFromPlayerByGuid, (void**)&K2_RemoveItemFromPlayerByGuidOG);
                }

                Log("Inventory Hooked!");
        }
}

// =============================================================================
// Abilities.h (orig)
// =============================================================================
namespace Abilities {
        inline void InitAbilitiesForPlayer(AFortPlayerController* PC)
        {
                OGS_CHECK(PC);
                auto PlayerState = (AFortPlayerStateAthena*)PC->PlayerState;
                static auto AbilitySet = StaticLoadObject<UFortAbilitySet>("/Game/Abilities/Player/Generic/Traits/DefaultPlayer/GAS_AthenaPlayer.GAS_AthenaPlayer");

                if (PlayerState && AbilitySet)
                {
                        auto specCtor = OgsFn::ability_spec_constructor();
                        auto giveAbility = OgsFn::give_ability();
                        if (!specCtor || !giveAbility)
                        {
                                OGS_LOG("InitAbilitiesForPlayer skipped (offset missing)");
                                return;
                        }
                        for (size_t i = 0; i < AbilitySet->GameplayAbilities.Num(); i++)
                        {
                                FGameplayAbilitySpec Spec{};
                                specCtor(&Spec, (UGameplayAbility*)AbilitySet->GameplayAbilities[i].Get()->DefaultObject, 1, -1, nullptr);
                                giveAbility(PlayerState->AbilitySystemComponent, &Spec.Handle, Spec);
                        }
                }
        }

        inline FGameplayAbilitySpec* FindAbilitySpecFromHandle(UFortAbilitySystemComponentAthena* ASC, FGameplayAbilitySpecHandle& Handle)
        {
                OGS_CHECK(ASC);
                for (size_t i = 0; i < ASC->ActivatableAbilities.Items.Num(); i++)
                {
                        if (ASC->ActivatableAbilities.Items[i].Handle.Handle == Handle.Handle)
                                return &ASC->ActivatableAbilities.Items[i];
                }
                return nullptr;
        }

        FGameplayAbilitySpec* FindAbilitySpec(UAbilitySystemComponent* AbilitySystemComponent, FGameplayAbilitySpecHandle Handle) {
                OGS_CHECK(AbilitySystemComponent);

                for (FGameplayAbilitySpec& Spec : AbilitySystemComponent->ActivatableAbilities.Items) {
                        if (Spec.Handle.Handle == Handle.Handle) {
                                if (!Spec.PendingRemove) {
                                        return &Spec;
                                }
                        }
                }

                return nullptr;
        }

        void InternalServerTryActivateAbility(UAbilitySystemComponent* AbilitySystemComponent, FGameplayAbilitySpecHandle Handle, bool InputPressed, const struct FPredictionKey& InPredictionKey, FGameplayEventData* TriggerEventData)
        {
                OGS_CHECK(AbilitySystemComponent);
                FGameplayAbilitySpec* Spec = FindAbilitySpec(AbilitySystemComponent, Handle);
                if (!Spec) {
                        return AbilitySystemComponent->ClientActivateAbilityFailed(Handle, InPredictionKey.Current);
                }
                UGameplayAbility* AbilityToActivate = Spec->Ability;

                if (!AbilityToActivate)
                {
                        return AbilitySystemComponent->ClientActivateAbilityFailed(Handle, InPredictionKey.Current);
                }

                UGameplayAbility* InstancedAbility = nullptr;
                Spec->InputPressed = true;

                auto tryActivate = OgsFn::internal_try_activate_ability();
                if (!tryActivate)
                {
                        OGS_LOG("InternalServerTryActivateAbility skipped (offset missing)");
                        return;
                }
                if (tryActivate(AbilitySystemComponent, Handle, InPredictionKey, &InstancedAbility, nullptr, TriggerEventData)) {}
                else {
                        AbilitySystemComponent->ClientActivateAbilityFailed(Handle, InPredictionKey.Current);
                        Spec->InputPressed = false;

                        AbilitySystemComponent->ActivatableAbilities.MarkItemDirty(*Spec);
                }
        }

        void Hook()
        {
                // HookVTable(..., 0xFA, InternalServerTryActivateAbility, ...) on the
                // three ASC CDOs → ByExec on the reflected RPC.
                OgsHook::ByExec(UAbilitySystemComponent::GetDefaultObj(), "AbilitySystemComponent", "InternalServerTryActivateAbility", (void*)InternalServerTryActivateAbility, nullptr);
                OgsHook::ByExec(UFortAbilitySystemComponent::GetDefaultObj(), "FortAbilitySystemComponent", "InternalServerTryActivateAbility", (void*)InternalServerTryActivateAbility, nullptr);
                OgsHook::ByExec(UFortAbilitySystemComponentAthena::GetDefaultObj(), "FortAbilitySystemComponentAthena", "InternalServerTryActivateAbility", (void*)InternalServerTryActivateAbility, nullptr);

                Log("Abilities Hooked!");
        }
}

// =============================================================================
// Building.h (orig)
// =============================================================================
namespace Building {
        inline void ServerCreateBuildingActor(AFortPlayerControllerAthena* PC, FCreateBuildingActorData CreateBuildingData)
        {
                if (!PC || PC->IsInAircraft())
                        return;
                OGS_CHECK(PC->BroadcastRemoteClientInfo);

                UClass* BuildingClass = PC->BroadcastRemoteClientInfo->RemoteBuildableClass.Get();
                char a7;
                TArray<AActor*> BuildingsToRemove;
                auto cantBuild = OgsFn::cant_build();
                if (!cantBuild)
                {
                        OGS_LOG("ServerCreateBuildingActor skipped (offset missing: cant_build)");
                        return;
                }
                if (!cantBuild(UWorld::GetWorld(), BuildingClass, CreateBuildingData.BuildLoc, CreateBuildingData.BuildRot, CreateBuildingData.bMirrored, &BuildingsToRemove, &a7))
                {
                        OGS_CHECK(BuildingClass && BuildingClass->DefaultObject);
                        auto ResDef = UFortKismetLibrary::GetDefaultObj()->K2_GetResourceItemDefinition(((ABuildingSMActor*)BuildingClass->DefaultObject)->ResourceType);
                        Inventory::RemoveItem(PC, ResDef, 10);

                        ABuildingSMActor* NewBuilding = SpawnActor<ABuildingSMActor>(CreateBuildingData.BuildLoc, CreateBuildingData.BuildRot, PC, BuildingClass);
                        OGS_CHECK(NewBuilding);

                        NewBuilding->bPlayerPlaced = true;
                        NewBuilding->InitializeKismetSpawnedBuildingActor(NewBuilding, PC, true);
                        NewBuilding->TeamIndex = ((AFortPlayerStateAthena*)PC->PlayerState)->TeamIndex;
                        NewBuilding->Team = EFortTeam(NewBuilding->TeamIndex);

                        for (size_t i = 0; i < BuildingsToRemove.Num(); i++)
                        {
                                BuildingsToRemove[i]->K2_DestroyActor();
                        }
                        UKismetArrayLibrary::Array_Clear((TArray<int32>&)BuildingsToRemove);
                }
        }

        inline void ServerBeginEditingBuildingActor(AFortPlayerControllerAthena* PC, ABuildingSMActor* BuildingActorToEdit)
        {
                if (!BuildingActorToEdit || !BuildingActorToEdit->bPlayerPlaced || !PC->MyFortPawn)
                        return;
                OGS_CHECK(PC->WorldInventory);

                AFortPlayerStateAthena* PlayerState = (AFortPlayerStateAthena*)PC->PlayerState;
                BuildingActorToEdit->SetNetDormancy(ENetDormancy::DORM_Awake);
                BuildingActorToEdit->EditingPlayer = PlayerState;
                for (size_t i = 0; i < PC->WorldInventory->Inventory.ItemInstances.Num(); i++)
                {
                        auto Item = PC->WorldInventory->Inventory.ItemInstances[i];
                        if (Item->GetItemDefinitionBP()->IsA(UFortEditToolItemDefinition::StaticClass()))
                        {
                                PC->MyFortPawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)Item->GetItemDefinitionBP(), Item->GetItemGuid());
                                break;
                        }
                }
                if (!PC->MyFortPawn->CurrentWeapon || !PC->MyFortPawn->CurrentWeapon->WeaponData || !PC->MyFortPawn->CurrentWeapon->IsA(AFortWeap_EditingTool::StaticClass()))
                        return;

                AFortWeap_EditingTool* EditTool = (AFortWeap_EditingTool*)PC->MyFortPawn->CurrentWeapon;
                EditTool->EditActor = BuildingActorToEdit;
                EditTool->OnRep_EditActor();
        }

        inline void ServerEndEditingBuildingActor(AFortPlayerControllerAthena* PC, ABuildingSMActor* BuildingActorToEdit)
        {
                if (!BuildingActorToEdit || !PC->MyFortPawn || BuildingActorToEdit->bDestroyed == 1 || BuildingActorToEdit->EditingPlayer != PC->PlayerState)
                        return;
                OGS_CHECK(PC->WorldInventory);

                BuildingActorToEdit->SetNetDormancy(ENetDormancy::DORM_DormantAll);
                BuildingActorToEdit->EditingPlayer = nullptr;
                for (size_t i = 0; i < PC->WorldInventory->Inventory.ItemInstances.Num(); i++)
                {
                        auto Item = PC->WorldInventory->Inventory.ItemInstances[i];
                        if (Item->GetItemDefinitionBP()->IsA(UFortEditToolItemDefinition::StaticClass()))
                        {
                                PC->MyFortPawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)Item->GetItemDefinitionBP(), Item->GetItemGuid());
                                break;
                        }
                }
                if (!PC->MyFortPawn->CurrentWeapon || !PC->MyFortPawn->CurrentWeapon->WeaponData || !PC->MyFortPawn->CurrentWeapon->IsA(AFortWeap_EditingTool::StaticClass()))
                        return;

                AFortWeap_EditingTool* EditTool = (AFortWeap_EditingTool*)PC->MyFortPawn->CurrentWeapon;
                EditTool->EditActor = nullptr;
                EditTool->OnRep_EditActor();
        }

        inline void ServerEditBuildingActor(AFortPlayerControllerAthena* PC, ABuildingSMActor* BuildingActorToEdit, TSubclassOf<ABuildingSMActor> NewBuildingClass, uint8 RotationIterations, bool bMirrored)
        {
                if (!BuildingActorToEdit || BuildingActorToEdit->EditingPlayer != PC->PlayerState || !NewBuildingClass.Get() || BuildingActorToEdit->bDestroyed == 1)
                        return;

                auto replaceBuilding = OgsFn::replace_building_actor();
                if (!replaceBuilding)
                {
                        OGS_LOG("ServerEditBuildingActor skipped (offset missing)");
                        return;
                }

                BuildingActorToEdit->SetNetDormancy(ENetDormancy::DORM_DormantAll);
                BuildingActorToEdit->EditingPlayer = nullptr;
                ABuildingSMActor* NewActor = replaceBuilding(BuildingActorToEdit, 1, NewBuildingClass.Get(), 0, RotationIterations, bMirrored, PC);
                if (NewActor)
                        NewActor->bPlayerPlaced = true;
        }

        void ServerRepairBuildingActor(AFortPlayerController* PC, ABuildingSMActor* BuildingActorToRepair)
        {
                auto FortKismet = (UFortKismetLibrary*)UFortKismetLibrary::StaticClass()->DefaultObject;
                if (!BuildingActorToRepair)
                        return;

                if (BuildingActorToRepair->EditingPlayer)
                {
                        return;
                }

                float BuildingHealthPercent = BuildingActorToRepair->GetHealthPercent();

                float BuildingCost = 10;
                float RepairCostMultiplier = 0.75;

                float BuildingHealthPercentLost = 1.0f - BuildingHealthPercent;
                float RepairCostUnrounded = (BuildingCost * BuildingHealthPercentLost) * RepairCostMultiplier;
                float RepairCost = std::floor(RepairCostUnrounded > 0 ? RepairCostUnrounded < 1 ? 1 : RepairCostUnrounded : 0);

                if (RepairCost < 0)
                        return;

                auto ResDef = FortKismet->K2_GetResourceItemDefinition(BuildingActorToRepair->ResourceType);

                if (!ResDef)
                        return;

                if (!PC->bBuildFree)
                {
                        Inventory::RemoveItem(PC, ResDef, (int)RepairCost);
                }

                BuildingActorToRepair->RepairBuilding(PC, (int)RepairCost);
        }

        inline int64_t (*OnDamageServerOG)(ABuildingSMActor* Actor, float Damage, FGameplayTagContainer DamageTags, FVector Momentum, FHitResult HitInfo, AFortPlayerControllerAthena* InstigatedBy, AActor* DamageCauser, FGameplayEffectContextHandle EffectContext);
        inline int64_t OnDamageServer(ABuildingSMActor* Actor, float Damage, FGameplayTagContainer DamageTags, FVector Momentum, FHitResult HitInfo, AFortPlayerControllerAthena* InstigatedBy, AActor* DamageCauser, FGameplayEffectContextHandle EffectContext)
        {
                if (!Actor || !InstigatedBy || !InstigatedBy->IsA(AFortPlayerControllerAthena::StaticClass()) || !DamageCauser || !DamageCauser->IsA(AFortWeapon::StaticClass()) || !((AFortWeapon*)DamageCauser)->WeaponData || !((AFortWeapon*)DamageCauser)->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass()) || Actor->bPlayerPlaced)
                        return OnDamageServerOG(Actor, Damage, DamageTags, Momentum, HitInfo, InstigatedBy, DamageCauser, EffectContext);

                auto Def = UFortKismetLibrary::K2_GetResourceItemDefinition(Actor->ResourceType);
                static std::map<AFortPlayerControllerAthena*, int> NumWeakSpots{};

                if (Def)
                {
                        auto& BuildingResourceAmountOverride = Actor->BuildingResourceAmountOverride;
                        if (!BuildingResourceAmountOverride.CurveTable) {
                                return OnDamageServerOG(Actor, Damage, DamageTags, Momentum, HitInfo, InstigatedBy, DamageCauser, EffectContext);
                        }
                        auto GameState = (AFortGameStateAthena*)UWorld::GetWorld()->GameState;

                        FString CurveTableAssetPath = UKismetStringLibrary::Conv_NameToString(GameState->CurrentPlaylistInfo.BasePlaylist->ResourceRates.ObjectID.AssetPathName);
                        static auto CurveTable = StaticLoadObject<UCurveTable>(CurveTableAssetPath.ToString());
                        if (!CurveTable)
                                CurveTable = StaticLoadObject<UCurveTable>("/Game/Athena/Balance/DataTables/AthenaResourceRates.AthenaResourceRates");

                        float Average = 1;
                        EEvaluateCurveTableResult OutCurveTable;
                        UDataTableFunctionLibrary::EvaluateCurveTableRow(CurveTable, BuildingResourceAmountOverride.RowName, 0.f, &OutCurveTable, &Average, FString());
                        float FinalResourceCount = round(Average / (Actor->GetMaxHealth() / Damage));

                        if (FinalResourceCount > 0)
                        {
                                if (Damage == 100.f && GameState->GamePhase != EAthenaGamePhase::Warmup)
                                {
                                        UFortAccoladeItemDefinition* AccoladeDef = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_066_WeakSpotsInARow.AccoladeId_066_WeakSpotsInARow");

                                        NumWeakSpots[InstigatedBy]++;
                                        if (NumWeakSpots[InstigatedBy] == 2) {
                                                Quests::GiveAccolade(InstigatedBy, AccoladeDef);
                                                NumWeakSpots[InstigatedBy] = 0;
                                        }
                                }

                                InstigatedBy->ClientReportDamagedResourceBuilding(Actor, Actor->ResourceType, FinalResourceCount, false, Damage == 100.f);
                                Inventory::GiveItemStack(InstigatedBy, Def, (int)FinalResourceCount, 0);
                        }
                }

                return OnDamageServerOG(Actor, Damage, DamageTags, Momentum, HitInfo, InstigatedBy, DamageCauser, EffectContext);
        }

        void Hook() {
                // HookVTable(AAthena_PlayerController_C, 0x230/0x237/0x235/0x232/0x22C)
                // → ByExec on the reflected ServerRPCs.
                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerCreateBuildingActor", (void*)ServerCreateBuildingActor, nullptr);

                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerBeginEditingBuildingActor", (void*)ServerBeginEditingBuildingActor, nullptr);

                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerEndEditingBuildingActor", (void*)ServerEndEditingBuildingActor, nullptr);

                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerEditBuildingActor", (void*)ServerEditBuildingActor, nullptr);

                OgsHook::ByExec(AAthena_PlayerController_C::GetDefaultObj(), "Athena_PlayerController_C", "ServerRepairBuildingActor", (void*)ServerRepairBuildingActor, nullptr);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_damage_server, OnDamageServer, (void**)&OnDamageServerOG);

                Log("Hooked Building!");
        }
}

// =============================================================================
// Bosses.h (orig) — FactionBot + henchmen AI.
// NOTE: the original had namespace-scope `auto GameState/Math/Gamemode/
// Statics = ...` initialised at DLL load; they are OgsSDK::* calls here.
// =============================================================================
class FactionBot
{
public:
        // The behaviortree for the new ai system
        BehaviorTree* BT_MANG = nullptr;

        // The context that should be sent to the behaviortree
        BT_MANG_Context Context = {};

        // The playercontroller of the bot
        ABP_PhoebePlayerController_C* PC;

        // The Pawn of the bot
        AFortPlayerPawnAthena* Pawn;

        // Are we ticking the bot?
        bool bTickEnabled = false;

        // So we can track the current tick that the bot is doing
        uint64_t tick_counter = 0;

        // Is the bot stressed, will be determined every tick.
        bool bIsStressed = false;

        // The current target that the bot is focusing on
        AActor* CurrentTarget = nullptr;

        // The current alertlevel of the bot, should have different behaviour for each one
        EAlertLevel AlertLevel = EAlertLevel::Unaware;

        // The bots should only have one weapon, so why not make it easier for us to switch back to it if we switch to pickaxe
        UFortWeaponItemDefinition* Weapon = nullptr;
        FGuid WeaponGuid;

        // So we can easily switch to the pickaxe when we need to
        FGuid PickaxeGuid;
        UFortWeaponMeleeItemDefinition* Pickaxe = nullptr;

        // The cid (pretty self explanatory)
        std::string CID = "";

        // The actual name of the bots class
        std::string Name = "";

        // The patrol path that the henchmen/bot is assigned to
        AFortAthenaPatrolPath* PatrolPath = nullptr;

        // The location of the current patrol point
        FVector CurrentPatrolPointLoc;

        // The current patrol point the bot is on
        int CurrentPatrolPoint = 0;

        // the amount of patrol points the bot patrol path has
        int MaxPatrolPoints = 0;

        // Are we patrolling?
        bool bIsPatrolling = false;

        // patrol points decremental
        bool bIsPatrollingBack = false;

        // bot is paused for some time for next patrol
        bool bIsWaitingForNextPatrol = false;

        // Time for the bot to go to the next patrol point
        float TimeWaitingForNextPatrol = 0.f;

        // Time for the bot to go back to being unaware
        float TimeUntilUnaware = 0.f;

        // Is the bot currently strafing (combat technique used by the bots)
        bool bIsCurrentlyStrafing = false;

        // The strafe type used by the bot, determines what direction
        EBossesStrafeType StrafeType = EBossesStrafeType::StrafeLeft;

        // When should the current strafe end?
        float StrafeEndTime = 0.0f;

        // the bot this bot will revive when downed, or the other way round
        FactionBot* CurrentAssignedDBNOBot;

        bool bIsReviving = false;

        float ReviveTime = 0.f;

        bool bWasReportedStuck = false;

public:
        FactionBot(AFortPlayerPawnAthena* Pawn)
        {
                this->Pawn = Pawn;
                PC = (ABP_PhoebePlayerController_C*)(Pawn ? Pawn->Controller : nullptr);

                Context.Controller = (AFortAthenaAIBotController*)PC;
                Context.Pawn = Pawn;
                Context.bot = this;

                FactionBots.push_back(this);
        }

public:
        // We shouldnt need to use this but we have it anyway
        FGuid GetItem(UFortItemDefinition* Def)
        {
                OGS_CHECK(PC && PC->Inventory);
                for (int32 i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition == Def)
                                return PC->Inventory->Inventory.ReplicatedEntries[i].ItemGuid;
                }
                return FGuid();
        }

        // only really should be called when initialisint the bot
        void GiveItem(UFortItemDefinition* ODef, int Count = 1, bool equip = true)
        {
                OGS_CHECK(PC && PC->Inventory && ODef);
                UFortItemDefinition* Def = ODef;
                if (Def->GetName() == "AGID_Boss_Tina") {
                        Def = StaticLoadObject<UFortItemDefinition>("/Game/Athena/Items/Weapons/Boss/WID_Boss_Tina.WID_Boss_Tina");
                }
                OGS_CHECK(Def);
                UFortWorldItem* Item = (UFortWorldItem*)Def->CreateTemporaryItemInstanceBP(Count, 0);
                OGS_CHECK(Item);
                Item->OwnerInventory = PC->Inventory;

                if (Def->IsA(UFortWeaponRangedItemDefinition::StaticClass()))
                {
                        Item->ItemEntry.LoadedAmmo = Looting::GetClipSize((UFortWeaponItemDefinition*)Def);
                }

                PC->Inventory->Inventory.ReplicatedEntries.Add(Item->ItemEntry);
                PC->Inventory->Inventory.ItemInstances.Add(Item);
                PC->Inventory->Inventory.MarkItemDirty(Item->ItemEntry);
                PC->Inventory->HandleInventoryLocalUpdate();
                if (Def->IsA(UFortWeaponRangedItemDefinition::StaticClass()) && equip)
                {
                        this->Weapon = (UFortWeaponItemDefinition*)ODef;
                        this->WeaponGuid = GetItem(Def);
                        if (Pawn)
                                Pawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)Def, GetItem(Def));
                }
                if (Def->GetName() == "WID_Harvest_Pickaxe_Athena_C_T01") {
                        this->Pickaxe = (UFortWeaponMeleeItemDefinition*)Def;
                        this->PickaxeGuid = GetItem(Def);
                }
        }

        // set the currenttarget so that we can have the bot react appropriately to the target
        virtual void OnPerceptionSensed(AActor* SourceActor, FAIStimulus& Stimulus)
        {
                if (PC && Stimulus.bSuccessfullySensed == 1 && PC->LineOfSightTo(SourceActor, FVector(), true) && Pawn && Pawn->GetDistanceTo(SourceActor) < 5000)
                {
                        CurrentTarget = SourceActor;
                }
        }
};

namespace BossesBTService_Patrolling {
        inline FVector GetPatrolLocation(FactionBot* bot) {
                auto Math = OgsSDK::Math();
                FVector Loc{};
                if (!bot || !bot->PatrolPath || bot->PatrolPath->PatrolPoints.Num() == 0)
                        return Loc;

                bot->MaxPatrolPoints = (bot->PatrolPath->PatrolPoints.Num() - 1);

                if (bot->MaxPatrolPoints < 1)
                {
                        Log("Not enough patrol points for " + (bot->PC ? bot->PC->GetFullName() : std::string("?")) + ", cancelling patrol.");
                        bot->PatrolPath = nullptr;
                        return Loc;
                }

                FVector PatrolPointLoc = bot->PatrolPath->PatrolPoints[bot->CurrentPatrolPoint]->K2_GetActorLocation();
                if (PatrolPointLoc.IsZero())
                {
                        Log("No Patrol Point Location For Index: " + std::to_string(bot->CurrentPatrolPoint));
                }
                else
                {
                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                        auto TestRot = Math->FindLookAtRotation(BotPos, PatrolPointLoc);

                        bot->PC->SetControlRotation(TestRot);
                        bot->PC->K2_SetActorRotation(TestRot, true);
                        Loc = PatrolPointLoc;
                }

                Log("Started Patrolling " + std::to_string(bot->MaxPatrolPoints) + " Patrol Points for bot: " + bot->Name);
                bot->CurrentPatrolPointLoc = Loc;
                bot->bIsPatrolling = true;

                return Loc;
        }

        inline FVector DetermineNextPatrolPointLoc(FactionBot* bot) {
                auto Math = OgsSDK::Math();
                FVector Loc{};
                if (!bot || !bot->PatrolPath || bot->PatrolPath->PatrolPoints.Num() == 0)
                        return Loc;

                if (!bot->bIsPatrollingBack) {
                        if ((bot->CurrentPatrolPoint + 1) >= bot->MaxPatrolPoints) {
                                bot->bIsPatrollingBack = true;
                        }
                        bot->CurrentPatrolPoint = bot->CurrentPatrolPoint + 1;
                        if (!bot->PatrolPath->PatrolPoints[bot->CurrentPatrolPoint]) {
                                bot->bIsPatrollingBack = true;
                        }
                        else {
                                FVector PatrolPointLoc = bot->PatrolPath->PatrolPoints[bot->CurrentPatrolPoint]->K2_GetActorLocation();
                                if (PatrolPointLoc.IsZero())
                                {
                                        Log("No Patrol Point Location For Index: " + std::to_string(bot->CurrentPatrolPoint));
                                }
                                else
                                {
                                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                                        auto TestRot = Math->FindLookAtRotation(BotPos, PatrolPointLoc);

                                        bot->PC->SetControlRotation(TestRot);
                                        bot->PC->K2_SetActorRotation(TestRot, true);
                                        Loc = PatrolPointLoc;
                                }
                        }
                }
                else {
                        if ((bot->CurrentPatrolPoint - 1) <= 0) {
                                bot->bIsPatrollingBack = false;
                        }
                        bot->CurrentPatrolPoint = bot->CurrentPatrolPoint - 1;
                        if (!bot->PatrolPath->PatrolPoints[bot->CurrentPatrolPoint]) {
                                bot->bIsPatrollingBack = false;
                        }
                        else {
                                FVector PatrolPointLoc = bot->PatrolPath->PatrolPoints[bot->CurrentPatrolPoint]->K2_GetActorLocation();
                                if (PatrolPointLoc.IsZero())
                                {
                                        Log("No Patrol Point Location For Index: " + std::to_string(bot->CurrentPatrolPoint));
                                }
                                else
                                {
                                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                                        auto TestRot = Math->FindLookAtRotation(BotPos, PatrolPointLoc);

                                        bot->PC->SetControlRotation(TestRot);
                                        bot->PC->K2_SetActorRotation(TestRot, true);
                                        Loc = PatrolPointLoc;
                                }
                        }
                }

                if (bot->PC && bot->PC->Blackboard)
                        bot->PC->Blackboard->SetValueAsVector(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Patrolling_Destination")), Loc);
                return Loc;
        }
}

class BossesBTService_AIEvaluator {
public:
        // When stressed the bot will handle combat situations with players or playerbots differently
        bool IsStressed(AFortPlayerPawnAthena* Pawn, ABP_PhoebePlayerController_C* PC) {
                // If the bots shield is below 0 and they are threatened then they are stressed
                if (Pawn && PC && Pawn->GetShield() <= 0 && PC->CurrentAlertLevel == EAlertLevel::Threatened) {
                        return true;
                }
                return false;
        }

public:
        void Tick(FactionBot* bot) {
                auto Math = OgsSDK::Math();
                auto Statics = OgsSDK::Statics();
                OGS_CHECK(bot && bot->Pawn && bot->PC);

                FVector BotPos = bot->Pawn->K2_GetActorLocation();

                bot->bIsStressed = IsStressed(bot->Pawn, bot->PC);

                if (bot->bIsReviving && bot->CurrentAssignedDBNOBot && bot->CurrentAssignedDBNOBot->PC) {
                        bot->PC->ReviveTarget = bot->CurrentAssignedDBNOBot->Pawn;
                        if (Statics->GetTimeSeconds(UWorld::GetWorld()) >= bot->ReviveTime) {
                                auto PlayerState = (AFortPlayerStateAthena*)bot->CurrentAssignedDBNOBot->PC->PlayerState;
                                OGS_CHECK(PlayerState && PlayerState->AbilitySystemComponent);
                                auto AbilitySystemComp = (UFortAbilitySystemComponentAthena*)PlayerState->AbilitySystemComponent;

                                FGameplayEventData Data{};
                                Data.EventTag = bot->CurrentAssignedDBNOBot->Pawn->EventReviveTag;
                                Data.ContextHandle = PlayerState->AbilitySystemComponent->MakeEffectContext();
                                Data.Instigator = bot->PC;
                                Data.Target = bot->CurrentAssignedDBNOBot->Pawn;
                                Data.TargetData = UAbilitySystemBlueprintLibrary::AbilityTargetDataFromActor(bot->CurrentAssignedDBNOBot->Pawn);
                                Data.TargetTags = bot->CurrentAssignedDBNOBot->Pawn->GameplayTags;
                                UAbilitySystemBlueprintLibrary::SendGameplayEventToActor(bot->CurrentAssignedDBNOBot->Pawn, bot->CurrentAssignedDBNOBot->Pawn->EventReviveTag, Data);

                                for (auto& Ability : AbilitySystemComp->ActivatableAbilities.Items)
                                {
                                        if (!Ability.Ability || !Ability.Ability->Class)
                                                continue;
                                        const std::string AbilityName = Ability.Ability->Class->GetName();

                                        if (AbilityName.find("GAB_AthenaDBNO") != std::string::npos || AbilityName.find("GAB_AthenaDBNORevive") != std::string::npos)
                                        {
                                                AbilitySystemComp->ServerCancelAbility(Ability.Handle, Ability.ActivationInfo);
                                                AbilitySystemComp->ServerEndAbility(Ability.Handle, Ability.ActivationInfo, Ability.ActivationInfo.PredictionKeyWhenActivated);
                                                AbilitySystemComp->ClientCancelAbility(Ability.Handle, Ability.ActivationInfo);
                                                AbilitySystemComp->ClientEndAbility(Ability.Handle, Ability.ActivationInfo);
                                        }
                                }

                                bot->CurrentAssignedDBNOBot->Pawn->bIsDBNO = false;
                                bot->Pawn->bPlayedDying = false;
                                bot->Pawn->bIsDying = false;
                                bot->Pawn->DBNORevivalStacking = 0;
                                bot->CurrentAssignedDBNOBot->Pawn->OnRep_IsDBNO();
                                bot->CurrentAssignedDBNOBot->Pawn->SetHealth(30);
                                PlayerState->DeathInfo = {};
                                PlayerState->OnRep_DeathInfo();

                                bot->CurrentAssignedDBNOBot->Pawn->EquipWeaponDefinition(bot->Weapon, bot->WeaponGuid);
                        }
                }

                if (!bot->bIsStressed && bot->PatrolPath && bot->CurrentPatrolPointLoc.IsZero() && !bot->bIsPatrolling) {
                        BossesBTService_Patrolling::GetPatrolLocation(bot);
                }

                if (bot->bIsPatrolling && bot->bIsWaitingForNextPatrol && Statics->GetTimeSeconds(UWorld::GetWorld()) >= bot->TimeWaitingForNextPatrol) {
                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                        auto PatrolRot = Math->FindLookAtRotation(BotPos, bot->CurrentPatrolPointLoc);

                        bot->PC->SetControlRotation(PatrolRot);
                        bot->PC->K2_SetActorRotation(PatrolRot, true);

                        bot->bIsWaitingForNextPatrol = false;
                }

                if (bot->PC->CurrentAlertLevel != EAlertLevel::Threatened && bot->PC->CurrentAlertLevel != EAlertLevel::Alerted
                        && bot->PatrolPath && bot->bIsPatrolling && !bot->bIsWaitingForNextPatrol && !bot->CurrentPatrolPointLoc.IsZero()) {
                        float Distance = Math->Vector_Distance(BotPos, bot->CurrentPatrolPointLoc);

                        if (Distance < 175.0f) {
                                bot->bIsWaitingForNextPatrol = true;
                                bot->TimeWaitingForNextPatrol = Statics->GetTimeSeconds(UWorld::GetWorld()) + Math->RandomFloatInRange(1.5f, 3.0f);
                                bot->CurrentPatrolPointLoc = BossesBTService_Patrolling::DetermineNextPatrolPointLoc(bot);
                        }
                }

                if (bot->tick_counter % 30 == 0) {
                        float ClosestDistance = FLT_MAX;
                        FactionBot* ClosestBot = nullptr;

                        for (int i = 0; i < (int)FactionBots.size(); i++) {
                                if (!FactionBots[i] || !FactionBots[i]->PC || !FactionBots[i]->Pawn || !FactionBots[i]->PC->PlayerState)
                                        continue;
                                if (((AFortPlayerStateAthena*)bot->PC->PlayerState)->PlayerTeam == ((AFortPlayerStateAthena*)FactionBots[i]->PC->PlayerState)->PlayerTeam) {
                                        if (bot->Pawn->bIsDBNO) {
                                                if (FactionBots[i]->Pawn->bIsDBNO)
                                                        continue;
                                        }
                                        if (FactionBots[i]->Pawn == bot->Pawn)
                                                continue;

                                        float Dist = Math->Vector_Distance(bot->Pawn->K2_GetActorLocation(), FactionBots[i]->Pawn->K2_GetActorLocation());

                                        if (Dist < ClosestDistance) {
                                                ClosestDistance = Dist;
                                                ClosestBot = FactionBots[i];
                                        }
                                }
                        }

                        if (ClosestBot) {
                                bot->CurrentAssignedDBNOBot = ClosestBot;
                        }
                }
        }
};

namespace Bosses {
        inline void TickBots(float DeltaTime)
        {
                auto Math = OgsSDK::Math();
                auto Statics = OgsSDK::Statics();

                auto block = [](FactionBot* bot, std::function<void(FactionBot* bot)> const& SetUnaware, bool Alerted, bool Threatened, bool LKP) {
                        if (!bot || !bot->PC || !bot->Pawn)
                                return;

                        auto Math = OgsSDK::Math();

                        BossesBTService_AIEvaluator Evaluator;
                        Evaluator.Tick(bot);
                        //return;

                        if (bot->CurrentAssignedDBNOBot && bot->CurrentAssignedDBNOBot->Pawn && bot->CurrentAssignedDBNOBot->Pawn->bIsDBNO && !Threatened) {
                                FVector BotPos = bot->Pawn->K2_GetActorLocation();
                                float Distance = Math->Vector_Distance(BotPos, bot->CurrentAssignedDBNOBot->Pawn->K2_GetActorLocation());
                                auto TestRot = Math->FindLookAtRotation(BotPos, bot->CurrentAssignedDBNOBot->Pawn->K2_GetActorLocation());

                                bot->PC->SetControlRotation(TestRot);
                                bot->PC->K2_SetActorRotation(TestRot, true);

                                if (Distance < 200.0f) {
                                        if (!bot->bIsReviving) {
                                                bot->ReviveTime = OgsSDK::Statics()->GetTimeSeconds(UWorld::GetWorld()) + 9.9;
                                                bot->bIsReviving = true;
                                        }
                                        return;
                                }
                                else {
                                        if (bot->bIsReviving) {
                                                bot->bIsReviving = false;
                                        }
                                        //bot->Pawn->AddMovementInput(bot->Pawn->GetActorForwardVector(), 1.1f, true);
                                        bot->PC->MoveToActor(bot->CurrentAssignedDBNOBot->PC, 100, true, false, true, nullptr, true);
                                }

                                bot->tick_counter++;
                                return;
                        }

                        if (bot->bIsReviving) {
                                bot->bIsReviving = false;
                        }

                        if (bot->Pawn->bIsDBNO && bot->CurrentAssignedDBNOBot) {
                                FVector BotPos = bot->Pawn->K2_GetActorLocation();
                                float Distance = Math->Vector_Distance(BotPos, bot->CurrentAssignedDBNOBot->Pawn->K2_GetActorLocation());
                                auto TestRot = Math->FindLookAtRotation(BotPos, bot->CurrentAssignedDBNOBot->Pawn->K2_GetActorLocation());

                                bot->PC->SetControlRotation(TestRot);
                                bot->PC->K2_SetActorRotation(TestRot, true);

                                if (Distance < 100.0f) {}
                                else {
                                        //bot->Pawn->AddMovementInput(bot->Pawn->GetActorForwardVector(), 1.1f, true);
                                        bot->PC->MoveToActor(bot->CurrentAssignedDBNOBot->PC, 100, true, false, true, nullptr, true);
                                }

                                bot->tick_counter++;
                                return;
                        }

                        if ((bot->tick_counter % 30 == 0) && bot->bWasReportedStuck) {
                                bot->Pawn->EquipWeaponDefinition(bot->Weapon, bot->WeaponGuid);
                                bot->bWasReportedStuck = false;

                                if (bot->Pawn->bIsCrouched) {
                                        bot->Pawn->UnCrouch(false);
                                }
                        }

                        if (!Threatened && !Alerted && !LKP && bot->PatrolPath && bot->bIsPatrolling && !bot->bIsWaitingForNextPatrol && !bot->CurrentPatrolPointLoc.IsZero()) {
                                bot->PC->Blackboard->SetValueAsBool(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Patrolling_ShouldMove")), true);
                                bot->PC->MoveToLocation(bot->CurrentPatrolPointLoc, 100.f, false, false, false, true, nullptr, true);
                        }

                        if (!Threatened && !Alerted && !LKP) {
                                if (bot->CurrentAssignedDBNOBot && bot->CurrentAssignedDBNOBot->bIsStressed) {
                                        bot->PC->MoveToActor(bot->CurrentAssignedDBNOBot->PC, Math->RandomFloatInRange(400, 1000), true, false, true, nullptr, true);
                                }
                        }

                        if (Alerted) {
                                if (bot->CurrentTarget) {
                                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                                        auto TargetPos = bot->CurrentTarget->K2_GetActorLocation();

                                        bot->Pawn->PawnStopFire(0);

                                        auto Rot = Math->FindLookAtRotation(BotPos, TargetPos);

                                        if (Math->RandomBoolWithWeight(0.25f)) {
                                                bot->PC->SetControlRotation(Rot);
                                                bot->PC->K2_SetActorRotation(Rot, true);
                                        }

                                        bot->PC->MoveToLocation(TargetPos, 25.f, true, false, false, true, nullptr, true);

                                        if (Math->RandomBoolWithWeight(0.25f)) {
                                                float RandomXmod = Math->RandomFloatInRange(-360000, 360000);
                                                float RandomYmod = Math->RandomFloatInRange(-360000, 360000);
                                                //float RandomZmod = Math->RandomFloatInRange(-360000, 360000);

                                                FVector TargetPosMod{ TargetPos.X + RandomXmod, TargetPos.Y + RandomYmod, TargetPos.Z };

                                                Rot = Math->FindLookAtRotation(BotPos, TargetPosMod);

                                                bot->PC->SetControlRotation(Rot);
                                                bot->PC->K2_SetActorRotation(Rot, true);
                                        }
                                }
                        }
                        else if (Threatened) {
                                if (bot->CurrentTarget) {
                                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                                        auto TargetPos = bot->CurrentTarget->K2_GetActorLocation();

                                        if (!bot->Pawn->bIsCrouched && Math->RandomBoolWithWeight(0.01f)) {
                                                bot->Pawn->Crouch(false);
                                        }
                                        if (bot->Pawn->bIsCrouched && (bot->tick_counter % 30) == 0) {
                                                bot->Pawn->UnCrouch(false);
                                        }

                                        if (Math->RandomBoolWithWeight(0.001f)) {
                                                bot->Pawn->UnCrouch(false);
                                                bot->Pawn->Jump();
                                        }

                                        if (!bot->bIsStressed) {
                                                bot->PC->MoveToActor(bot->CurrentTarget, Math->RandomFloatInRange(400, 1000), true, false, true, nullptr, true);
                                        }
                                        else {
                                                bot->Pawn->AddMovementInput((bot->Pawn->GetActorForwardVector() * -1.0f), 1.2f, true);
                                        }

                                        if (true) { // wouldve had a tickcounter condition here but no
                                                float RandomXmod = Math->RandomFloatInRange(-180, 180);
                                                float RandomYmod = Math->RandomFloatInRange(-180, 180);
                                                float RandomZmod = Math->RandomFloatInRange(-180, 180);

                                                FVector TargetPosMod{ TargetPos.X + RandomXmod, TargetPos.Y + RandomYmod, TargetPos.Z + RandomZmod };

                                                FRotator Rot = Math->FindLookAtRotation(BotPos, TargetPosMod);

                                                bot->PC->SetControlRotation(Rot);
                                                bot->PC->K2_SetActorRotation(Rot, true);

                                                //bot->PC->K2_SetFocalPoint(TargetPosMod); doesent fix the issue with them not aiming up or down
                                        }

                                        if (!bot->bIsCurrentlyStrafing)
                                        {
                                                if (UKismetMathLibrary::RandomBoolWithWeight(0.05))
                                                {
                                                        bot->bIsCurrentlyStrafing = true;
                                                        if (UKismetMathLibrary::RandomBool()) {
                                                                bot->StrafeType = EBossesStrafeType::StrafeLeft;
                                                        }
                                                        else {
                                                                bot->StrafeType = EBossesStrafeType::StrafeRight;
                                                        }
                                                        bot->StrafeEndTime = OgsSDK::Statics()->GetTimeSeconds(UWorld::GetWorld()) + Math->RandomFloatInRange(2.0f, 4.0f);
                                                }
                                        }
                                        else
                                        {
                                                if (OgsSDK::Statics()->GetTimeSeconds(UWorld::GetWorld()) < bot->StrafeEndTime)
                                                {
                                                        if (bot->StrafeType == EBossesStrafeType::StrafeLeft) {
                                                                bot->Pawn->AddMovementInput((bot->Pawn->GetActorRightVector() * -1.0f), 1.2f, true);
                                                        }
                                                        else {
                                                                bot->Pawn->AddMovementInput(bot->Pawn->GetActorRightVector(), 1.2f, true);
                                                        }
                                                }
                                                else
                                                {
                                                        bot->bIsCurrentlyStrafing = false;
                                                }
                                        }

                                        if (bot->PC->LineOfSightTo(bot->CurrentTarget, BotPos, true)
                                                && Math->RandomBoolWithWeight(0.5f)) {
                                                bool HasMinigun = bot->Pawn->CurrentWeapon && bot->Pawn->CurrentWeapon->Name.ToString().find("Minigun") != std::string::npos;
                                                if (!HasMinigun && (bot->tick_counter % 2 != 0)) {
                                                        bot->Pawn->PawnStopFire(0);
                                                }
                                                else if (HasMinigun && (bot->tick_counter % 150 != 0)) {
                                                        bot->Pawn->PawnStopFire(0);
                                                }
                                                else {
                                                        bot->Pawn->PawnStartFire(0);
                                                }
                                        }
                                        else {
                                                bot->Pawn->PawnStopFire(0);
                                        }
                                }
                        }
                        else if (LKP) {
                                if (bot->CurrentTarget) {
                                        auto BotPos = bot->Pawn->K2_GetActorLocation();
                                        auto TargetPos = bot->CurrentTarget->K2_GetActorLocation();

                                        auto Rot = Math->FindLookAtRotation(BotPos, TargetPos);

                                        if (Math->RandomBoolWithWeight(0.25f)) {
                                                bot->PC->SetControlRotation(Rot);
                                                bot->PC->K2_SetActorRotation(Rot, true);
                                        }

                                        bot->PC->MoveToActor(bot->CurrentTarget, 150, true, false, true, nullptr, true);

                                        if (Math->RandomBoolWithWeight(0.25f)) {
                                                float RandomXmod = Math->RandomFloatInRange(-360000, 360000);
                                                float RandomYmod = Math->RandomFloatInRange(-360000, 360000);
                                                //float RandomZmod = Math->RandomFloatInRange(-360000, 360000);

                                                FVector TargetPosMod{ TargetPos.X + RandomXmod, TargetPos.Y + RandomYmod, TargetPos.Z };

                                                Rot = Math->FindLookAtRotation(BotPos, TargetPosMod);

                                                bot->PC->SetControlRotation(Rot);
                                                bot->PC->K2_SetActorRotation(Rot, true);
                                        }
                                }
                        }
                        else {
                                SetUnaware(bot);
                        }

                        bot->tick_counter++;
                };
                auto SetUnaware = [](FactionBot* bot) {
                        if (!bot || !bot->PC || !bot->Pawn)
                                return;
                        bot->PC->CurrentAlertLevel = EAlertLevel::Unaware;
                        bot->PC->OnAlertLevelChanged(bot->AlertLevel, EAlertLevel::Unaware);
                        bot->Pawn->PawnStopFire(0);
                        if (bot->PatrolPath) {
                                bot->bIsPatrolling = true;
                        }
                };
                for (auto bot : FactionBots)
                {
                        if (!bot || !bot->PC)
                                continue;
                        if (Globals::bUseLegacyAI_MANG) {
                                auto Alerted = bot->PC->CurrentAlertLevel == EAlertLevel::Alerted;
                                auto Threatened = bot->PC->CurrentAlertLevel == EAlertLevel::Threatened;
                                auto LKP = bot->PC->CurrentAlertLevel == EAlertLevel::LKP;
                                block(bot, SetUnaware, Alerted, Threatened, LKP);
                        }
                        else {
                                if (bot->BT_MANG) {
                                        bot->BT_MANG->Tick(bot->Context);
                                }
                        }
                }
        }
}

// =============================================================================
// BT_MANG2.h (orig) — the "2.0" behaviour tree.
// =============================================================================
namespace BT_MANG2_Tasks {
        class BTTask_Wait : public BTNode {
        public:
                float WaitTime = 0.f;
                float WorldWaitTime = 0.f;
                bool bFinishedWait = false;
        public:
                BTTask_Wait(float InWaitTime) {
                        WaitTime = InWaitTime;
                        WorldWaitTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) + InWaitTime;
                }

                virtual EBTNodeResult ChildTask(BTContext Context) override {
                        if (WaitTime == 0.f || WorldWaitTime == 0.f) {
                                return EBTNodeResult::Failed;
                        }
                        if (UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) >= WorldWaitTime) {
                                // Reset the wait if the wait is completed
                                if (bFinishedWait) {
                                        WorldWaitTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) + WaitTime;
                                        bFinishedWait = false;
                                        return EBTNodeResult::InProgress;
                                }
                                bFinishedWait = true;
                                return EBTNodeResult::Succeeded;
                        }
                        // If it is still waiting then return inprogress
                        return EBTNodeResult::InProgress;
                }
        };

        class BTTask_StartPatrolling : public BTNode {
        public:
                BTComposite_Selector* SelectorToRun = nullptr;
        public:
                virtual EBTNodeResult ChildTask(BTContext Context) override {
                        if (SelectorToRun) {
                                return SelectorToRun->Tick(Context);
                        }
                        return EBTNodeResult::Failed;
                }
        };
}

namespace BT_MANG_Decorators {
        class BTDecorator_Loop : public BTDecorator {
        public:
                virtual bool Evaluate(BTContext Context) override {
                        // All of the loop decorators that i saw are infinite loops so...
                        return true;
                }
        };

        class BTDecorator_Blackboard_5 : public BTDecorator {
        public:
                BTDecorator_Blackboard_5() {
                        Name = "BTDecorator_Blackboard_5";
                        CachedDescription = "AIEvaluator_Global_IsMovementBlocked is Is Set";
                        NodeName = "Is Movement Blocked?";
                }

                virtual bool Evaluate(BTContext Context) override {
                        auto* BB = Context.Controller ? Context.Controller->Blackboard : nullptr;
                        if (!BB)
                                return false;
                        return BB->GetValueAsBool(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_IsMovementBlocked")));
                }
        };
}

namespace BT_MANG_Services {
        class BTService_AIEvaluator_4 : public BTService {
        public:
                BTService_AIEvaluator_4() {
                        Name = "FortAthenaBTService_AIEvaluator_4";
                        NodeName = "Focus Scan Around Only";

                        Interval = 0.03f;
                        NextTickTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) + Interval;
                }

                virtual void TickService(BTContext Context) override {
                        float CurrentTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());

                        if (CurrentTime >= NextTickTime) {
                                //Log("BTService_AIEvaluator_4");
                                if (!Context.Pawn)
                                        return;
                                FVector CurrentLoc = Context.Pawn->K2_GetActorLocation();
                                FVector LookLoc = FVector((CurrentLoc.X + (UKismetMathLibrary::RandomFloatInRange(-3.f, 3.f))),
                                        (CurrentLoc.Y + (UKismetMathLibrary::RandomFloatInRange(-3.f, 3.f))), CurrentLoc.Z);

                                if (Context.Controller)
                                        Context.Controller->K2_SetFocalPoint(LookLoc);

                                NextTickTime = CurrentTime + Interval;
                        }
                }
        };
}

namespace BT_MANG2 {
        // Cached Constructed BT_MANG2 BT
        static BehaviorTree* BT_MANG2 = nullptr;

        // Construct the behaviortree and cache it
        BehaviorTree* ConstructBehaviorTree() {
                auto* Tree = new BehaviorTree();

                auto* RootSelector = new BTComposite_Selector();
                RootSelector->Name = "BTComposite_Selector_Root";

                {
                        auto* Selector = new BTComposite_Selector();
                        Selector->Name = "BTComposite_Selector_Patrolling";
                        (void)Selector;
                }

                {
                        auto* Task = new BT_MANG2_Tasks::BTTask_Wait(2.f);
                        Task->AddDecorator(new BT_MANG_Decorators::BTDecorator_Blackboard_5());
                        Task->AddDecorator(new BT_MANG_Decorators::BTDecorator_Loop());
                        RootSelector->AddChild(Task);
                }

                {
                        auto* Task = new BT_MANG2_Tasks::BTTask_StartPatrolling();
                        Task->SelectorToRun = Tree->FindSelectorByName("BTComposite_Selector_Patrolling");
                        RootSelector->AddChild(Task);
                }

                Tree->RootNode = RootSelector;
                Tree->AllNodes.push_back(RootSelector);

                BT_MANG2 = Tree;
                return Tree;
        }
}

// =============================================================================
// PlayerBots.h (orig)
// =============================================================================
static std::vector<UAthenaCharacterItemDefinition*> CIDs{};
static std::vector<UAthenaPickaxeItemDefinition*> Pickaxes{};
static std::vector<UAthenaBackpackItemDefinition*> Backpacks{};
static std::vector<UAthenaGliderItemDefinition*> Gliders{};
static std::vector<UAthenaSkyDiveContrailItemDefinition*> Contrails{};
inline std::vector<UAthenaDanceItemDefinition*> Dances{};

std::vector<class PlayerBot*> PlayerBotArray{};
struct PlayerBot
{
public:
        // Additional State Enums : Seprated so its cleaner
        EBotWarmupChoice BotWarmupChoice = EBotWarmupChoice::MAX;

public:
        // So we can track the current tick that the bot is doing
        uint64_t tick_counter = 0;

        // The Pawn of the bot
        AFortPlayerPawnAthena* Pawn = nullptr;

        // The playercontroller of the bot
        ABP_PhoebePlayerController_C* PC = nullptr;

        // The playerstate of the bot
        AFortPlayerStateAthena* PlayerState = nullptr;

        // The current botstate, has different behaviours for different states
        EBotState BotState = EBotState::Warmup;

        // The cached botstate, usually if a botstate needs to revert back to the previous one
        EBotState CachedBotState = EBotState::Warmup;

        // The building that the bot collided with (doesent even works smd)
        ABuildingActor* StuckBuildingActor = nullptr;

        // Reservation of lootables, stops pileup and tracks current lootable
        AActor* TargetLootable = nullptr;

        // Just incase we have to do specific stuff based on the type of lootable it is
        ELootableType TargetLootableType = ELootableType::None;

        // The nearest player, bot, factionbot
        AActor* NearestPlayerActor = nullptr;

        // The displayname of the bot
        FString DisplayName = OGS_TEXT("Bot");

        // Is the bot stressed, will be determined every tick.
        bool bIsStressed = false;

        // Is the bot dead, basically marking the bot to be removed from the playerbotarray later
        bool bIsDead = false;

        // Has the bot thanked the bus driver
        bool bHasThankedBusDriver = false;

        // Has the bot jumped from the bus, if not then set the botstate to bus.
        bool bHasJumpedFromBus = false;

        // The dropzone that the bot will attempt to land at
        FVector TargetDropZone = FVector();

        // The closest distance achieved to the targetdropzone, will be used mostly for determining bus drop
        float ClosestDistToDropZone = FLT_MAX;

        // Is the bot currently strafing (combat technique & Unstuck method)
        bool bIsCurrentlyStrafing = false;

        // The strafe type used by the bot, determines what direction
        EBotStrafeType StrafeType = EBotStrafeType::StrafeLeft;

        // When should the current strafe end?
        float StrafeEndTime = 0.0f;

        // General purpose timer
        float TimeToNextAction = 0.f;

        // The start time of this current lootable
        float LootTargetStartTime = 0.f;

        // The distance between the bot and the lootable
        float LastLootTargetDistance = 0.f;

public:
        void OnDied(AFortPlayerStateAthena* KillerState, AActor* DamageCauser, FName BoneName)
        {
                static auto Acc_DistanceShot = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_DistanceShot.AccoladeId_DistanceShot");
                static auto Acc_LongShot = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_051_LongShot.AccoladeId_051_LongShot");
                static auto Acc_LudicrousShot = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_052_LudicrousShot.AccoladeId_052_LudicrousShot");
                static auto Acc_ImpossibleShot = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_053_ImpossibleShot.AccoladeId_053_ImpossibleShot");
                static auto Acc_Headshot = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_048_HeadshotElim.AccoladeId_048_HeadshotElim");

                static auto Acc_Survival_Bronze = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_026_Survival_Default_Bronze.AccoladeId_026_Survival_Default_Bronze");
                static auto Acc_Survival_Silver = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_027_Survival_Default_Silver.AccoladeId_027_Survival_Default_Silver");
                static auto Acc_Survival_Gold = StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_028_Survival_Default_Gold.AccoladeId_028_Survival_Default_Gold");

                Log("Bot Died");

                bIsDead = true;

                if (!KillerState || !DamageCauser || !PlayerState || !Pawn)
                        return;

                FDeathInfo& DeathInfo = PlayerState->DeathInfo;

                DeathInfo.bDBNO = Pawn->bWasDBNOOnDeath;
                DeathInfo.DeathLocation = Pawn->K2_GetActorLocation();
                DeathInfo.DeathTags = Pawn->DeathTags;
                DeathInfo.Downer = KillerState ? KillerState : nullptr;
                AFortPawn* KillerPawn = KillerState ? KillerState->GetCurrentPawn() : nullptr;
                DeathInfo.Distance = (KillerPawn && Pawn) ? KillerPawn->GetDistanceTo(Pawn) : 0.f;
                DeathInfo.FinisherOrDowner = KillerState ? KillerState : nullptr;
                DeathInfo.DeathCause = PlayerState->ToDeathCause(DeathInfo.DeathTags, DeathInfo.bDBNO);
                DeathInfo.bInitialized = true;
                PlayerState->OnRep_DeathInfo();

                bool bIsKillerABot = KillerState->bIsABot;

                if (!PC || !PC->Inventory)
                        return;

                auto KillerPC = (AFortPlayerControllerAthena*)KillerState->GetOwner();

                if (KillerPC && KillerPC->IsA(AFortPlayerControllerAthena::StaticClass()) && !bIsKillerABot)
                {
                        KillerState->KillScore++;

                        Quests::GiveAccolade(KillerPC, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_012_Elimination.AccoladeId_012_Elimination"));
                        Quests::GiveAccolade(KillerPC, GetDefFromEvent(EAccoladeEvent::Kill, KillerState->KillScore));

                        // giving assist accolade cuz idfk how to track assists
                        Quests::GiveAccolade((AFortPlayerControllerAthena*)KillerState->Owner, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_013_Assist.AccoladeId_013_Assist"));

                        if (KillerState->PlayerTeam)
                        {
                                for (size_t i = 0; i < KillerState->PlayerTeam->TeamMembers.Num(); i++)
                                {
                                        if (KillerState->PlayerTeam->TeamMembers[i] && KillerState->PlayerTeam->TeamMembers[i]->PlayerState)
                                        {
                                                ((AFortPlayerStateAthena*)KillerState->PlayerTeam->TeamMembers[i]->PlayerState)->TeamKillScore++;
                                                ((AFortPlayerStateAthena*)KillerState->PlayerTeam->TeamMembers[i]->PlayerState)->OnRep_TeamKillScore();
                                        }
                                }
                        }

                        KillerState->ClientReportKill(PlayerState);
                        KillerState->ClientReportTeamKill(KillerState->KillScore);
                        KillerState->OnRep_Kills();
                }

                if (bIsKillerABot) {
                        Log("got to the end!");
                        return;
                }

                if (!bFirstElimTriggered) {
                        Quests::GiveAccolade(KillerPC, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_017_First_Elimination.AccoladeId_017_First_Elimination"));
                        bFirstElimTriggered = true;
                }

                float Distance = DeathInfo.Distance / 100.0f;

                if (Distance >= 100.0f && Distance < 150.0f)
                {
                        Quests::GiveAccolade(KillerPC, Acc_DistanceShot); // 100-149m accolade
                }
                else if (Distance >= 150.0f && Distance < 200.0f)
                {
                        Quests::GiveAccolade(KillerPC, Acc_LongShot); // 150-199m accolade
                }
                else if (Distance >= 200.0f && Distance < 250.0f)
                {
                        Quests::GiveAccolade(KillerPC, Acc_LudicrousShot); // 200-249m accolade
                }
                else if (Distance >= 250.0f)
                {
                        Quests::GiveAccolade(KillerPC, Acc_ImpossibleShot); // 250+m accolade
                }

                if (BoneName.ToString() == "head")
                {
                        KillerPC->HeadShots++;
                        Quests::GiveAccolade(KillerPC, Acc_Headshot); // headshot accolade

                        if (KillerPC->HeadShots == 4) {
                                Quests::GiveAccolade(KillerPC, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_049_Headhunter.AccoladeId_049_Headhunter"));
                        }
                }

                if (KillerPC) {
                        float CurrentTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());

                        if (UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) > KillerPC->LastKillTimeWindow) {
                                KillerPC->KillsInKillTimeWindow = 0;
                                KillerPC->LastRecordedKillsInKillTimeWindow = 0;
                        }
                        KillerPC->LastKillTimeWindow = CurrentTime + 10.f;
                        KillerPC->KillsInKillTimeWindow++;
                }

                Log("Got to the end");
        }

public:
        void SetStuck(AActor* OtherActor, FHitResult& Hit) {
                if (BotState < EBotState::Landed)
                        return;

                if (BotState == EBotState::Stuck)
                        return;

                if (OtherActor && OtherActor->IsA(ABuildingSMActor::StaticClass()))
                {
                        ABuildingSMActor* Actor = (ABuildingSMActor*)OtherActor;
                        float Health = Actor->GetHealth();
                        FFindFloorResult Res;

                        if (Actor->bCanBeDamaged == 1 && Health > 1 && Health < 2500)
                        {
                                if (!Pawn || !Pawn->CharacterMovement || !Pawn->CapsuleComponent)
                                        return;
                                Pawn->CharacterMovement->K2_FindFloor(Pawn->CapsuleComponent->K2_GetComponentLocation(), &Res);
                                if (Res.HitResult.Actor.Get() == OtherActor)
                                        return;
                                StuckBuildingActor = Actor;
                        }
                }

                CachedBotState = BotState;
                BotState = EBotState::Stuck;
        }

public:
        // Give an item to the bot
        void GiveItemBot(UFortItemDefinition* Def, int Count = 1, int LoadedAmmo = 0)
        {
                if (!Def || !PC || !PC->Inventory) {
                        return;
                }

                UFortWorldItem* Item = (UFortWorldItem*)Def->CreateTemporaryItemInstanceBP(Count, 0);
                if (!Item)
                        return;
                Item->OwnerInventory = PC->Inventory;
                Item->ItemEntry.LoadedAmmo = LoadedAmmo;
                PC->Inventory->Inventory.ReplicatedEntries.Add(Item->ItemEntry);
                PC->Inventory->Inventory.ItemInstances.Add(Item);
                PC->Inventory->Inventory.MarkItemDirty(Item->ItemEntry);
                PC->Inventory->HandleInventoryLocalUpdate();
        }

        FFortItemEntry* GetEntry(UFortItemDefinition* Def)
        {
                OGS_CHECK(PC && PC->Inventory);
                for (size_t i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition == Def)
                                return &PC->Inventory->Inventory.ReplicatedEntries[i];
                }

                return nullptr;
        }

        void Emote()
        {
                if (Dances.empty())
                        return;
                auto EmoteDef = Dances[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)Dances.size() - 1)];
                if (!EmoteDef)
                        return;

                static UClass* EmoteAbilityClass = StaticLoadObject<UClass>("/Game/Abilities/Emotes/GAB_Emote_Generic.GAB_Emote_Generic_C");

                auto specCtor = OgsFn::ability_spec_constructor();
                auto giveAndActivate = OgsFn::give_ability_and_activate_once();
                if (!EmoteAbilityClass || !EmoteAbilityClass->DefaultObject || !specCtor || !giveAndActivate || !PC || !PC->PlayerState)
                {
                        OGS_LOG("Emote skipped (offset missing)");
                        return;
                }

                FGameplayAbilitySpec Spec{};
                specCtor(&Spec, reinterpret_cast<UGameplayAbility*>(EmoteAbilityClass->DefaultObject), 1, -1, EmoteDef);
                giveAndActivate(reinterpret_cast<AFortPlayerStateAthena*>(PC->PlayerState)->AbilitySystemComponent, &Spec.Handle, Spec);
        }

        void Run()
        {
                OGS_CHECK(PlayerState && PlayerState->AbilitySystemComponent);
                for (size_t i = 0; i < PlayerState->AbilitySystemComponent->ActivatableAbilities.Items.Num(); i++)
                {
                        auto& Item = PlayerState->AbilitySystemComponent->ActivatableAbilities.Items[i];
                        if (!Item.Ability)
                                continue;
                        if (Item.Ability->IsA(UFortGameplayAbility_Sprint::StaticClass()))
                        {
                                if (Item.ActivationInfo.PredictionKeyWhenActivated.bIsStale) {
                                        continue;
                                }
                                if (PlayerState->AbilitySystemComponent->CanActivateAbilityWithMatchingTag(Item.Ability->AbilityTags)) {
                                        PlayerState->AbilitySystemComponent->ServerTryActivateAbility(Item.Handle, Item.InputPressed, Item.ActivationInfo.PredictionKeyWhenActivated);
                                }
                                break;
                        }
                }
        }

        void ForceStrafe(bool doOverride) {
                auto Math = OgsSDK::Math();
                auto Statics = OgsSDK::Statics();
                if (!bIsCurrentlyStrafing && doOverride)
                {
                        bIsCurrentlyStrafing = true;
                        if (UKismetMathLibrary::RandomBool()) {
                                StrafeType = EBotStrafeType::StrafeLeft;
                        }
                        else {
                                StrafeType = EBotStrafeType::StrafeRight;
                        }
                        StrafeEndTime = Statics->GetTimeSeconds(UWorld::GetWorld()) + Math->RandomFloatInRange(2.0f, 5.0f);
                }
                else
                {
                        if (Statics->GetTimeSeconds(UWorld::GetWorld()) < StrafeEndTime)
                        {
                                if (StrafeType == EBotStrafeType::StrafeLeft) {
                                        Pawn->AddMovementInput((Pawn->GetActorRightVector() * -1.0f), 1.5f, true);
                                }
                                else {
                                        Pawn->AddMovementInput(Pawn->GetActorRightVector(), 1.5f, true);
                                }
                        }
                        else
                        {
                                bIsCurrentlyStrafing = false;
                        }
                }
        }

        void LookAt(AActor* Actor)
        {
                if (!Pawn || (PC && PC->GetFocusActor() == Actor))
                        return;

                if (!Actor)
                {
                        if (PC) PC->K2_ClearFocus();
                        return;
                }

                if (PC) PC->K2_SetFocus(Actor);
        }

        bool IsPickaxeEquiped() {
                if (!Pawn || !Pawn->CurrentWeapon)
                        return false;

                if (Pawn->CurrentWeapon->WeaponData && Pawn->CurrentWeapon->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass()))
                {
                        return true;
                }
                return false;
        }

        bool HasGun()
        {
                OGS_CHECK(PC && PC->Inventory);
                for (size_t i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        auto& Entry = PC->Inventory->Inventory.ReplicatedEntries[i];
                        if (Entry.ItemDefinition) {
                                std::string ItemName = Entry.ItemDefinition->Name.ToString();
                                if (ItemName.find("Shotgun") != std::string::npos || ItemName.find("SMG") != std::string::npos || ItemName.find("Assault") != std::string::npos
                                        || ItemName.find("Sniper") != std::string::npos || ItemName.find("Rocket") != std::string::npos || ItemName.find("Pistol") != std::string::npos) {
                                        return true;
                                        break;
                                }
                        }
                }
                return false;
        }

        void EquipPickaxe()
        {
                OGS_CHECK(PC && PC->Inventory);
                if (!Pawn || !Pawn->CurrentWeapon)
                        return;

                if (!Pawn->CurrentWeapon->WeaponData || !Pawn->CurrentWeapon->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass()))
                {
                        for (size_t i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                        {
                                if (PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition
                                        && PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition->IsA(UFortWeaponMeleeItemDefinition::StaticClass()))
                                {
                                        Pawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition, PC->Inventory->Inventory.ReplicatedEntries[i].ItemGuid);
                                        break;
                                }
                        }
                }
        }

        void SimpleSwitchToWeapon() {
                if (!HasGun()) {
                        return;
                }

                if (!Pawn || !Pawn->CurrentWeapon || !Pawn->CurrentWeapon->WeaponData || !PC || !PC->Inventory || bIsDead)
                        return;

                if (!Pawn->CurrentWeapon->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass())) {
                        return;
                }

                if (Pawn->CurrentWeapon->WeaponData->IsA(UFortWeaponMeleeItemDefinition::StaticClass()))
                {
                        for (size_t i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                        {
                                auto& Entry = PC->Inventory->Inventory.ReplicatedEntries[i];
                                if (Entry.ItemDefinition) {
                                        std::string ItemName = Entry.ItemDefinition->Name.ToString();
                                        if (ItemName.find("Shotgun") != std::string::npos || ItemName.find("SMG") != std::string::npos || ItemName.find("Assault") != std::string::npos
                                                || ItemName.find("Grenade") != std::string::npos || ItemName.find("Sniper") != std::string::npos || ItemName.find("Rocket") != std::string::npos || ItemName.find("Pistol") != std::string::npos) {
                                                Pawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)Entry.ItemDefinition, Entry.ItemGuid);
                                                break;
                                        }
                                }
                        }
                }
        }

        // I will add more stuff to this over time based on what the bot needs/dont needs
        bool ShouldPickup(AFortPickup* Pickup) {
                if (!Pickup) {
                        return false;
                }
                auto Def = Pickup->PrimaryPickupItemEntry.ItemDefinition;
                if (!Def) return false;

                // Dont bother with ammo cuz bots dont use it and bots usually pickup stuff in range anyway
                if (Def->IsA(UFortAmmoItemDefinition::StaticClass()))
                        return false;

                return true;
        }

        void Pickup(AFortPickup* Pickup) {
                if (!Pickup)
                        return;

                GiveItemBot(Pickup->PrimaryPickupItemEntry.ItemDefinition, Pickup->PrimaryPickupItemEntry.Count, Pickup->PrimaryPickupItemEntry.LoadedAmmo);

                Pickup->PickupLocationData.bPlayPickupSound = true;
                Pickup->PickupLocationData.FlyTime = 0.3f;
                Pickup->PickupLocationData.ItemOwner = Pawn;
                Pickup->PickupLocationData.PickupGuid = Pickup->PrimaryPickupItemEntry.ItemGuid;
                Pickup->PickupLocationData.PickupTarget = Pawn;
                Pickup->OnRep_PickupLocationData();

                Pickup->bPickedUp = true;
                Pickup->OnRep_bPickedUp();
        }

        void PickupAllItemsInRange(float Range = 320.f) {
                if (!Pawn || !PC) {
                        return;
                }

                static auto PickupClass = AFortPickupAthena::StaticClass();
                TArray<AActor*> Array;
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), PickupClass, &Array);

                for (size_t i = 0; i < Array.Num(); i++)
                {
                        if (!Array[i] || Array[i]->bHidden)
                                continue;

                        if (!ShouldPickup((AFortPickupAthena*)Array[i])) {
                                continue;
                        }

                        if (Array[i]->GetDistanceTo(Pawn) < Range)
                        {
                                Pickup((AFortPickupAthena*)Array[i]);
                        }
                }

                Array.Free();
        }

        ABuildingActor* FindNearestChest()
        {
                static auto ChestClass = StaticLoadObject<UClass>("/Game/Building/ActorBlueprints/Containers/Tiered_Chest_Athena.Tiered_Chest_Athena_C");
                static auto FactionChestClass = StaticLoadObject<UClass>("/Game/Building/ActorBlueprints/Containers/Tiered_Chest_Athena_FactionChest.Tiered_Chest_Athena_FactionChest_C");
                TArray<AActor*> Array;
                TArray<AActor*> FactionChests;
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), ChestClass, &Array);
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), FactionChestClass, &FactionChests);

                AActor* NearestChest = nullptr;

                for (size_t i = 0; i < Array.Num(); i++)
                {
                        AActor* Chest = Array[i];
                        if (!Chest || (Chest->bHidden && Chest != TargetLootable))
                                continue;

                        if (!NearestChest || Chest->GetDistanceTo(Pawn) < NearestChest->GetDistanceTo(Pawn))
                        {
                                NearestChest = Array[i];
                        }
                }

                for (size_t i = 0; i < FactionChests.Num(); i++)
                {
                        AActor* Chest = FactionChests[i];
                        if (!Chest || (Chest->bHidden && Chest != TargetLootable))
                                continue;

                        if (!NearestChest || Chest->GetDistanceTo(Pawn) < NearestChest->GetDistanceTo(Pawn))
                        {
                                NearestChest = FactionChests[i];
                        }
                }
                Array.Free();
                FactionChests.Free();
                return (ABuildingActor*)NearestChest;
        }

        AFortPickupAthena* FindNearestPickup()
        {
                static auto PickupClass = AFortPickupAthena::StaticClass();
                TArray<AActor*> Array;
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), PickupClass, &Array);
                AActor* NearestPickup = nullptr;

                for (size_t i = 0; i < Array.Num(); i++)
                {
                        if (!Array[i] || (Array[i]->bHidden && Array[i] != TargetLootable))
                                continue;

                        if (!ShouldPickup((AFortPickupAthena*)Array[i])) {
                                continue;
                        }

                        if (!NearestPickup || Array[i]->GetDistanceTo(Pawn) < NearestPickup->GetDistanceTo(Pawn))
                        {
                                NearestPickup = Array[i];
                        }
                }

                Array.Free();
                return (AFortPickupAthena*)NearestPickup;
        }

        bool GetNearestLootable() {
                static auto ChestClass = StaticLoadObject<UClass>("/Game/Building/ActorBlueprints/Containers/Tiered_Chest_Athena.Tiered_Chest_Athena_C");
                static auto FactionChestClass = StaticLoadObject<UClass>("/Game/Building/ActorBlueprints/Containers/Tiered_Chest_Athena_FactionChest.Tiered_Chest_Athena_FactionChest_C");
                TArray<AActor*> ChestArray;
                TArray<AActor*> FactionChests;
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), ChestClass, &ChestArray);
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), FactionChestClass, &FactionChests);

                static auto PickupClass = AFortPickupAthena::StaticClass();
                TArray<AActor*> PickupArray;
                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), PickupClass, &PickupArray);

                AActor* NearestPickup = nullptr;
                AActor* NearestChest = nullptr;

                for (size_t i = 0; i < ChestArray.Num(); i++)
                {
                        AActor* Chest = ChestArray[i];
                        if (!Chest || (Chest->bHidden && Chest != TargetLootable))
                                continue;

                        if (!NearestChest || Chest->GetDistanceTo(Pawn) < NearestChest->GetDistanceTo(Pawn))
                        {
                                NearestChest = ChestArray[i];
                        }
                }

                for (size_t i = 0; i < FactionChests.Num(); i++)
                {
                        AActor* Chest = FactionChests[i];
                        if (!Chest || (Chest->bHidden && Chest != TargetLootable))
                                continue;

                        if (!NearestChest || Chest->GetDistanceTo(Pawn) < NearestChest->GetDistanceTo(Pawn))
                        {
                                NearestChest = FactionChests[i];
                        }
                }

                for (size_t i = 0; i < PickupArray.Num(); i++)
                {
                        if (!PickupArray[i] || (PickupArray[i]->bHidden && PickupArray[i] != TargetLootable))
                                continue;

                        if (!ShouldPickup((AFortPickupAthena*)PickupArray[i])) {
                                continue;
                        }

                        if (!NearestPickup || PickupArray[i]->GetDistanceTo(Pawn) < NearestPickup->GetDistanceTo(Pawn))
                        {
                                NearestPickup = PickupArray[i];
                        }
                }

                ChestArray.Free();
                FactionChests.Free();
                PickupArray.Free();
                if (!NearestPickup || !NearestChest)
                        return false; // original dereferenced nulls here; fail-safe instead
                return NearestPickup->GetDistanceTo(Pawn) > NearestChest->GetDistanceTo(Pawn);
        }

        ABuildingSMActor* FindNearestBuildingSMActor()
        {
                static TArray<AActor*> Array;
                static bool PopulatedArray = false;
                if (!PopulatedArray)
                {
                        PopulatedArray = true;
                        UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), ABuildingSMActor::StaticClass(), &Array);
                }

                AActor* NearestActor = nullptr;

                for (size_t i = 0; i < Array.Num(); i++)
                {
                        if (!NearestActor || (((ABuildingSMActor*)NearestActor)->GetHealth() < 1500 && ((ABuildingSMActor*)NearestActor)->GetHealth() > 1 && Array[i]->GetDistanceTo(Pawn) < NearestActor->GetDistanceTo(Pawn)))
                        {
                                NearestActor = Array[i];
                        }
                }

                return (ABuildingSMActor*)NearestActor;
        }

        FVector FindNearestPlayerOrBot() {
                auto GameMode = OgsSDK::Gamemode();
                OGS_CHECK(GameMode && Pawn);

                AActor* NearestPlayer = nullptr;

                for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                {
                        if (GameMode->AlivePlayers[i] && (!NearestPlayer || (GameMode->AlivePlayers[i]->Pawn && GameMode->AlivePlayers[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn))))
                        {
                                AFortPlayerStateAthena* PS = (AFortPlayerStateAthena*)GameMode->AlivePlayers[i]->PlayerState;
                                if (PS && PS->TeamIndex != PlayerState->TeamIndex) {
                                        NearestPlayer = GameMode->AlivePlayers[i]->Pawn;
                                }
                        }
                }

                for (size_t i = 0; i < GameMode->AliveBots.Num(); i++)
                {
                        if (GameMode->AliveBots[i] && GameMode->AliveBots[i]->Pawn != Pawn)
                        {
                                if (!NearestPlayer || (GameMode->AliveBots[i]->Pawn && GameMode->AliveBots[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn)))
                                {
                                        AFortPlayerStateAthena* PS = (AFortPlayerStateAthena*)GameMode->AlivePlayers[i]->PlayerState;
                                        if (PS && PS->TeamIndex != PlayerState->TeamIndex) {
                                                NearestPlayer = GameMode->AliveBots[i]->Pawn;
                                        }
                                }
                        }
                }

                for (size_t i = 0; i < FactionBots.size(); i++) {
                        if (FactionBots[i] && FactionBots[i]->Pawn && (!NearestPlayer || (FactionBots[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn))))
                        {
                                NearestPlayer = FactionBots[i]->Pawn;
                        }
                }

                OGS_CHECK(NearestPlayer);
                return NearestPlayer->K2_GetActorLocation();
        }

        AActor* GetNearestPlayerActor() {
                auto GameMode = OgsSDK::Gamemode();
                OGS_CHECK(GameMode && Pawn && PlayerState);

                AActor* NearestPlayer = nullptr;

                for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                {
                        if (GameMode->AlivePlayers[i] && (!NearestPlayer || (GameMode->AlivePlayers[i]->Pawn && GameMode->AlivePlayers[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn))))
                        {
                                AFortPlayerStateAthena* PS = (AFortPlayerStateAthena*)GameMode->AlivePlayers[i]->PlayerState;
                                if (PS && PS->TeamIndex != PlayerState->TeamIndex) {
                                        NearestPlayer = GameMode->AlivePlayers[i]->Pawn;
                                }
                        }
                }

                for (size_t i = 0; i < GameMode->AliveBots.Num(); i++)
                {
                        if (GameMode->AliveBots[i] && GameMode->AliveBots[i]->Pawn != Pawn)
                        {
                                if (!NearestPlayer || (GameMode->AliveBots[i]->Pawn && GameMode->AliveBots[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn)))
                                {
                                        AFortPlayerStateAthena* PS = (AFortPlayerStateAthena*)GameMode->AliveBots[i]->PlayerState;
                                        if (PS && PS->TeamIndex != PlayerState->TeamIndex) {
                                                NearestPlayer = GameMode->AliveBots[i]->Pawn;
                                        }
                                }
                        }
                }

                for (size_t i = 0; i < FactionBots.size(); i++) {
                        if (FactionBots[i] && FactionBots[i]->Pawn && (!NearestPlayer || (FactionBots[i]->Pawn->GetDistanceTo(Pawn) < NearestPlayer->GetDistanceTo(Pawn))))
                        {
                                NearestPlayer = FactionBots[i]->Pawn;
                        }
                }

                return NearestPlayer;
        }

        void UpdateLootableReservation(AActor* Lootable, bool RemoveReservation) {
                if (RemoveReservation && !TargetLootable) {
                        return;
                }

                if (!RemoveReservation) {
                        if (!Lootable) {
                                return;
                        }
                        TargetLootable = Lootable;
                        Lootable->bHidden = true;
                }
                else {
                        TargetLootable->bHidden = false;
                        TargetLootable = nullptr;
                }
        }

        void GiveLategameLoadout()
        {
                if (UFortItemDefinition* AssaultRifleDef = Inventory::LoadWeapon(Assault_rifle))
                {
                        GiveItemBot(AssaultRifleDef, 1, Looting::GetClipSize(AssaultRifleDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)AssaultRifleDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItemBot(AmmoDef, 200, 0);
                        }
                }

                if (UFortItemDefinition* ShotGunDef = Inventory::LoadWeapon(Shotgun))
                {
                        GiveItemBot(ShotGunDef, 1, Looting::GetClipSize(ShotGunDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)ShotGunDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItemBot(AmmoDef, 120, 0);
                        }
                }

                if (UFortItemDefinition* RandomDef = Inventory::LoadWeapon(Mixed))
                {
                        GiveItemBot(RandomDef, 1, Looting::GetClipSize(RandomDef));

                        if (auto* RangedDef = (UFortWeaponRangedItemDefinition*)RandomDef)
                        {
                                UFortWorldItemDefinition* AmmoDef = RangedDef->GetAmmoWorldItemDefinition_BP();
                                if (AmmoDef)
                                        GiveItemBot(AmmoDef, 120, 0);
                        }
                }

                if (auto Consumable1Def = Inventory::LoadWeapon(Consumables))
                        GiveItemBot(Consumable1Def, 3, 0);

                if (auto Consumable2Def = Inventory::LoadWeapon(Consumables))
                        GiveItemBot(Consumable2Def, 3, 0);

                if (auto TrapDef = Inventory::LoadWeapon(Traps))
                        GiveItemBot(TrapDef, 3, 0);

                static UFortItemDefinition* WoodDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/WoodItemData.WoodItemData");
                GiveItemBot(WoodDef, 500, 0);

                static UFortItemDefinition* StoneDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/StoneItemData.StoneItemData");
                GiveItemBot(StoneDef, 500, 0);

                static UFortItemDefinition* MetalDef = StaticLoadObject<UFortItemDefinition>("/Game/Items/ResourcePickups/MetalItemData.MetalItemData");
                GiveItemBot(MetalDef, 500, 0);
        }
};

class BotsBTService_AIEvaluator {
public:
        // When stressed the bot will handle combat situations with players or other bots differently
        bool IsStressed(AFortPlayerPawnAthena* Pawn, ABP_PhoebePlayerController_C* PC) {
                // If the bots health is 75 or below then they are stressed
                if (Pawn && Pawn->GetHealth() <= 75) {
                        return true;
                }
                return false;
        }

public:
        void Tick(PlayerBot* bot) {
                auto Math = OgsSDK::Math();
                OGS_CHECK(bot && bot->Pawn && bot->PC);

                FVector BotPos = bot->Pawn->K2_GetActorLocation();
                FVector Vel = bot->Pawn->GetVelocity();
                float Speed = sqrtf(Vel.X * Vel.X + Vel.Y * Vel.Y);

                if (bot->tick_counter % 60 == 0) {
                        // Lets update the reservation every 2 seconds because its cleaner
                        AActor* NearestChest = bot->FindNearestChest();
                        AActor* NearestPickup = (AActor*)bot->FindNearestPickup();
                        if (!NearestChest || !NearestPickup) {}
                        else {
                                AActor* Nearest = NearestChest;
                                ELootableType NearestLootable = ELootableType::Chest;
                                if (NearestChest->GetDistanceTo(bot->Pawn) > NearestPickup->GetDistanceTo(bot->Pawn)) {
                                        NearestLootable = ELootableType::Pickup;
                                        Nearest = NearestPickup;
                                }
                                if (bot->TargetLootable) {
                                        if (Nearest != bot->TargetLootable) {
                                                bot->UpdateLootableReservation(nullptr, true);
                                                bot->UpdateLootableReservation(Nearest, false);
                                        }
                                }
                                else {
                                        bot->UpdateLootableReservation(Nearest, false);
                                }
                                bot->TargetLootableType = NearestLootable;
                        }

                        bot->NearestPlayerActor = bot->GetNearestPlayerActor();
                }

                if (bot->tick_counter % 60 == 0) {
                        // Every 2 seconds clear the focus just incase the bot is doing something else
                        bot->PC->K2_ClearFocus();
                }

                if (bot->Pawn->bIsCrouched && (bot->tick_counter % 30) == 0) {
                        bot->Pawn->UnCrouch(false);
                }

                bot->bIsStressed = IsStressed(bot->Pawn, bot->PC);

                if (bot->bIsCurrentlyStrafing) {
                        bot->ForceStrafe(false);
                }

                if ((bot->tick_counter % 90 == 0) && Speed >= 100 && bot->BotState > EBotState::Landed) { // Works but mostly just spams error logs
                        bot->Run();
                }
        }
};

class BotsBTService_Warmup{
public:
        void DetermineBotWarmupChoice(PlayerBot* bot) {
                if (UKismetMathLibrary::GetDefaultObj()->RandomBool()) {
                        bot->BotWarmupChoice = EBotWarmupChoice::Emote;
                }
                else {
                        bot->BotWarmupChoice = EBotWarmupChoice::MoveToPlayerEmote;
                }
        }

public:
        void Tick(PlayerBot* bot) {
                auto Math = OgsSDK::Math();
                OGS_CHECK(bot && bot->Pawn && bot->PC);

                if (bot->BotWarmupChoice == EBotWarmupChoice::MAX) {
                        DetermineBotWarmupChoice(bot);
                }
                else if (bot->BotWarmupChoice == EBotWarmupChoice::Emote) {
                        if (bot->tick_counter % 300 == 0) {
                                bot->Emote();
                        }
                }
                else {
                        if (bot->tick_counter % 150 == 0) {
                                bot->NearestPlayerActor = bot->GetNearestPlayerActor();
                                auto BotPos = bot->Pawn->K2_GetActorLocation();
                                if (bot->NearestPlayerActor) {
                                        FVector Nearest = bot->NearestPlayerActor->K2_GetActorLocation();
                                        if (!Nearest.IsZero()) {
                                                float Dist = Math->Vector_Distance(BotPos, Nearest);
                                                if (Dist < 200.f + rand() % 300) {
                                                        bot->LookAt(bot->NearestPlayerActor);
                                                        if (UKismetMathLibrary::GetDefaultObj()->RandomBool()) {
                                                                bot->Emote();
                                                        }
                                                }
                                                else {
                                                        bot->LookAt(bot->NearestPlayerActor);
                                                        bot->PC->MoveToActor(bot->NearestPlayerActor, 100, true, false, true, nullptr, true);
                                                }
                                        }
                                }
                        }
                }
        }
};

class BotsBTService_AIDropZone {
public:
        void ChooseDropZone(PlayerBot* bot) {
                auto Math = OgsSDK::Math();
                if (DropZoneLocations.empty()) return;

                bot->TargetDropZone = DropZoneLocations[rand() % DropZoneLocations.size()];

                // makes it more realisetic cause they dont clutter together
                bot->TargetDropZone.X += Math->RandomFloatInRange(-800.f, 800.f);
                bot->TargetDropZone.Y += Math->RandomFloatInRange(-800.f, 800.f);
        }

public:
        void Tick(PlayerBot* bot) {
                auto GameState = OgsSDK::GameState();
                auto Math = OgsSDK::Math();
                OGS_CHECK(bot && bot->Pawn && bot->PC && GameState);

                if (bot->TargetDropZone.IsZero()) {
                        ChooseDropZone(bot);
                        return;
                }

                if (bot->BotState == EBotState::Bus) {
                        bot->Pawn->SetShield(0);
                        if (bot->bHasJumpedFromBus) {
                                bot->BotState = EBotState::Skydiving;
                                return;
                        }

                        if (!bot->bHasThankedBusDriver && GameState->GamePhase == EAthenaGamePhase::Aircraft && Math->RandomBoolWithWeight(0.0005f))
                        {
                                bot->bHasThankedBusDriver = true;
                                bot->PC->ThankBusDriver();
                        }

                        AActor* Bus = GameState->GetAircraft(0);
                        if (!Bus) {
                                return;
                        }

                        FVector BusLocation = Bus->K2_GetActorLocation();
                        FVector DropTarget = bot->TargetDropZone;
                        DropTarget.Z = BusLocation.Z;

                        if (GameState->GamePhase > EAthenaGamePhase::Aircraft) {
                                Log("Force Jump");
                                bot->Pawn->K2_TeleportTo(DropTarget, {});
                                bot->Pawn->BeginSkydiving(true);
                                bot->BotState = EBotState::Skydiving;

                                bot->bHasJumpedFromBus = true;

                                return;
                        }

                        float DistanceToDrop = Math->Vector_Distance(BusLocation, DropTarget);
                        if (DistanceToDrop < bot->ClosestDistToDropZone) {
                                bot->ClosestDistToDropZone = DistanceToDrop;
                        }
                        else {
                                if (!bot->bHasThankedBusDriver && Math->RandomBoolWithWeight(0.5f)) {
                                        bot->bHasThankedBusDriver = true;
                                        bot->PC->ThankBusDriver();
                                }
                                if (Math->RandomBoolWithWeight(0.75)) {
                                        bot->Pawn->K2_TeleportTo(GameState->GetAircraft(0)->K2_GetActorLocation(), {});
                                        bot->Pawn->BeginSkydiving(true);
                                        bot->BotState = EBotState::Skydiving;

                                        bot->bHasJumpedFromBus = true;
                                }
                        }

                        return;
                }

                auto BotPos = bot->Pawn->K2_GetActorLocation();
                if (!bot->TargetDropZone.IsZero()) {
                        bot->TargetDropZone.Z = BotPos.Z;
                }

                if (bot->BotState == EBotState::Skydiving) {
                        if (!bot->Pawn->bIsSkydiving) {
                                bot->BotState = EBotState::Gliding;
                        }

                        if (!bot->TargetDropZone.IsZero()) {
                                // Dont know a better way to skydive
                                bot->Pawn->AddMovementInput(Math->NegateVector(bot->Pawn->GetActorUpVector()), 1, true);

                                float Dist = Math->Vector_Distance(BotPos, bot->TargetDropZone);
                                auto TestRot = Math->FindLookAtRotation(BotPos, bot->TargetDropZone);

                                bot->PC->SetControlRotation(TestRot);
                                bot->PC->K2_SetActorRotation(TestRot, true);

                                bot->PC->MoveToLocation(bot->TargetDropZone, 200.f, true, false, false, true, nullptr, true);
                        }
                }
                else if (bot->BotState == EBotState::Gliding) {
                        if (bot->Pawn->bIsSkydiving) {
                                bot->BotState = EBotState::Skydiving;
                        }

                        FVector Vel = bot->Pawn->GetVelocity();
                        float Speed = Vel.Z;
                        if (Speed == 0.f || bot->Pawn->bIsInWaterVolume) {
                                bot->BotState = EBotState::Landed;
                        }

                        if (!bot->TargetDropZone.IsZero()) {
                                float Dist = Math->Vector_Distance(BotPos, bot->TargetDropZone);
                                auto TestRot = Math->FindLookAtRotation(BotPos, bot->TargetDropZone);

                                bot->PC->SetControlRotation(TestRot);
                                bot->PC->K2_SetActorRotation(TestRot, true);

                                bot->PC->MoveToLocation(bot->TargetDropZone, 200.f, true, false, false, true, nullptr, true);
                        }
                }
        }
};

class BotsBTService_Loot {
public:

public:
        void Tick(PlayerBot* bot) {
                auto Math = OgsSDK::Math();
                OGS_CHECK(bot && bot->Pawn && bot->PC);

                if (bot->TargetLootable) {
                        if (bot->HasGun()) {
                                bot->BotState = EBotState::LookingForPlayers;
                                return;
                        }
                        FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                        if (!BotLoc.IsZero()) {
                                float Dist = Math->Vector_Distance(BotLoc, bot->TargetLootable->K2_GetActorLocation());

                                auto BotPos = bot->Pawn->K2_GetActorLocation();
                                auto TestRot = Math->FindLookAtRotation(BotPos, bot->TargetLootable->K2_GetActorLocation());

                                bot->PC->SetControlRotation(TestRot);
                                bot->PC->K2_SetActorRotation(TestRot, true);
                                bot->LookAt(bot->TargetLootable);

                                float CurrentTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());

                                if (bot->TargetLootable) {
                                        FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                                        float Dist = Math->Vector_Distance(BotLoc, bot->TargetLootable->K2_GetActorLocation());

                                        if (bot->LootTargetStartTime == 0.f) {
                                                bot->LootTargetStartTime = CurrentTime;
                                                bot->LastLootTargetDistance = Dist;
                                        }

                                        float Elapsed = CurrentTime - bot->LootTargetStartTime;

                                        // if the bot is not getting closer or stuck for more than 8 seconds then we should try go for another lootable
                                        if ((Elapsed > 8.f && Dist > bot->LastLootTargetDistance - 100.f)) {
                                                if (Math->RandomBool()) {
                                                        bot->Pawn->K2_TeleportTo(bot->TargetLootable->K2_GetActorLocation(), {});
                                                }
                                                else {
                                                        bot->TargetLootable = nullptr;
                                                        bot->LootTargetStartTime = 0.f;
                                                }
                                                return;
                                        }

                                        if (Dist < 300.f) {
                                                bot->LootTargetStartTime = 0.f;
                                        }

                                        bot->LastLootTargetDistance = Dist;
                                }

                                if (Dist < 300.f) {
                                        bot->PC->StopMovement();
                                        bot->Pawn->PawnStopFire(0);
                                        if (!bot->TimeToNextAction || !bot->Pawn->bStartedInteractSearch && bot->TargetLootableType == ELootableType::Chest) {
                                                bot->TimeToNextAction = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());
                                                bot->Pawn->bStartedInteractSearch = true;
                                                bot->Pawn->OnRep_StartedInteractSearch();
                                        }
                                        else if (UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld()) - bot->TimeToNextAction >= 1.5f && bot->TargetLootableType == ELootableType::Chest) {
                                                Looting::SpawnLoot((ABuildingContainer*)bot->TargetLootable);
                                                bot->TargetLootable->bHidden = true;
                                                bot->TargetLootable = nullptr;
                                                AFortPickup* Pickup = bot->FindNearestPickup();
                                                if (Pickup)
                                                {
                                                        bot->PickupAllItemsInRange();
                                                        bot->SimpleSwitchToWeapon();
                                                }

                                                bot->Pawn->bStartedInteractSearch = false;
                                                bot->Pawn->OnRep_StartedInteractSearch();
                                                bot->TimeToNextAction = 0;
                                                bot->BotState = EBotState::LookingForPlayers;
                                        }
                                        else if (bot->TargetLootableType == ELootableType::Pickup) {
                                                //bot->PickupAllItemsInRange(400.f);
                                                AFortPickup* Pickup = bot->FindNearestPickup();
                                                if (Pickup)
                                                {
                                                        bot->Pickup(Pickup);
                                                }
                                                bot->TimeToNextAction = 0;
                                                bot->BotState = EBotState::LookingForPlayers;
                                        }
                                }
                                else if (Dist < 2000.f) {
                                        bot->Pawn->PawnStartFire(0);
                                        bot->PC->MoveToActor(bot->TargetLootable, 50, true, false, true, nullptr, true);
                                        //bot->Pawn->AddMovementInput(bot->Pawn->GetActorForwardVector(), 1.1f, true);
                                }
                                else {
                                        bot->PC->MoveToActor(bot->TargetLootable, 50, true, false, true, nullptr, true);
                                        //bot->Pawn->AddMovementInput(bot->Pawn->GetActorForwardVector(), 1.1f, true);
                                }
                        }
                }
                else {
                        //Log("No targetlootable dummy!");
                }
        }
};

namespace PlayerBots {
        void FreeDeadBots() {
                for (size_t i = 0; i < PlayerBotArray.size();)
                {
                        if (PlayerBotArray[i]->bIsDead) {
                                delete PlayerBotArray[i];
                                PlayerBotArray.erase(PlayerBotArray.begin() + i);
                                Log("Freed a dead bot from the array!");
                        }
                        else {
                                ++i;
                        }
                }
        }

        void SpawnPlayerBots(AActor* SpawnLocator, EBotState StartingState = EBotState::Warmup, AFortPlayerControllerAthena* TeamPC = nullptr)
        {
                if (!Globals::bBotsEnabled)
                        return;

                auto GameState = OgsSDK::GameState();
                OGS_CHECK(GameState && SpawnLocator);

                static auto BotBP = StaticLoadObject<UClass>("/Game/Athena/AI/Phoebe/BP_PlayerPawn_Athena_Phoebe.BP_PlayerPawn_Athena_Phoebe_C");
                static UBehaviorTree* BehaviorTree = StaticLoadObject<UBehaviorTree>("/Game/Athena/AI/Phoebe/BehaviorTrees/BT_Phoebe.BT_Phoebe");
                static UBlackboardData* BlackboardData = StaticLoadObject<UBlackboardData>("/Game/Athena/AI/Phoebe/BehaviorTrees/BB_Phoebe.BB_Phoebe");

                if (!BotBP || !BehaviorTree)
                        return;

                PlayerBot* bot = new PlayerBot{};

                AFortGameModeAthena* GameMode = OgsSDK::Gamemode();
                OGS_CHECK(GameMode && GameMode->ServerBotManager);

                static auto BotMutator = (AFortAthenaMutator_Bots*)GameMode->ServerBotManager->CachedBotMutator;
                OGS_CHECK(BotMutator);

                bot->Pawn = BotMutator->SpawnBot(BotBP, SpawnLocator, SpawnLocator->K2_GetActorLocation(), SpawnLocator->K2_GetActorRotation(), false);

                if (!bot->Pawn || !bot->Pawn->Controller)
                        return;

                bot->PC = Cast<ABP_PhoebePlayerController_C>(bot->Pawn->Controller);
                bot->PlayerState = Cast<AFortPlayerStateAthena>(bot->PC ? bot->PC->PlayerState : nullptr);

                if (!bot->PC || !bot->PlayerState)
                        return;

                if (!CIDs.empty() && UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(1)) { // use the random bool so that some bots will be defaults (more realistic)
                        // as you could probably tell, i did not write this!
                        auto CID = CIDs[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)CIDs.size() - 1)];
                        if (CID && CID->HeroDefinition)
                        {
                                if (CID->HeroDefinition->Specializations.IsValid())
                                {
                                        for (size_t i = 0; i < CID->HeroDefinition->Specializations.Num(); i++)
                                        {
                                                UFortHeroSpecialization* Spec = StaticLoadObject<UFortHeroSpecialization>(UKismetStringLibrary::GetDefaultObj()->Conv_NameToString(CID->HeroDefinition->Specializations[i].ObjectID.AssetPathName).ToString());
                                                if (Spec)
                                                {
                                                        for (size_t j = 0; j < Spec->CharacterParts.Num(); j++)
                                                        {
                                                                UCustomCharacterPart* Part = StaticLoadObject<UCustomCharacterPart>(UKismetStringLibrary::GetDefaultObj()->Conv_NameToString(Spec->CharacterParts[j].ObjectID.AssetPathName).ToString());
                                                                if (Part)
                                                                {
                                                                        bot->PlayerState->CharacterData.Parts[(uintptr_t)Part->CharacterPartType] = Part;
                                                                }
                                                        }
                                                }
                                        }
                                }
                        }
                        if (CID) {
                                bot->PC->CosmeticLoadoutBC.Character = CID;
                        }
                }
                if (!Backpacks.empty() && UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.5)) { // less likely to equip than skin cause lots of ppl prefer not to use backpack
                        auto Backpack = Backpacks[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)Backpacks.size() - 1)];
                        if (Backpack)
                        {
                                for (size_t j = 0; j < Backpack->CharacterParts.Num(); j++)
                                {
                                        UCustomCharacterPart* Part = Backpack->CharacterParts[j];
                                        if (Part)
                                        {
                                                bot->PlayerState->CharacterData.Parts[(uintptr_t)Part->CharacterPartType] = Part;
                                        }
                                }
                        }
                }
                if (!Gliders.empty()) {
                        auto Glider = Gliders[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)Gliders.size() - 1)];
                        bot->PC->CosmeticLoadoutBC.Glider = Glider;
                }
                if (!Contrails.empty() && UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.95)) {
                        auto Contrail = Contrails[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)Contrails.size() - 1)];
                        bot->PC->CosmeticLoadoutBC.SkyDiveContrail = Contrail;
                }
                for (size_t i = 0; i < Dances.size(); i++)
                {
                        bot->PC->CosmeticLoadoutBC.Dances.Add(Dances.at(i));
                }

                bot->Pawn->CosmeticLoadout = bot->PC->CosmeticLoadoutBC;
                bot->Pawn->OnRep_CosmeticLoadout();

                if (Pickaxes.empty()) {
                        Log("Pickaxes array is empty!");
                        UFortWeaponMeleeItemDefinition* PickDef = StaticLoadObject<UFortWeaponMeleeItemDefinition>("/Game/Athena/Items/Weapons/WID_Harvest_Pickaxe_Athena_C_T01.WID_Harvest_Pickaxe_Athena_C_T01");
                        if (PickDef) {
                                bot->GiveItemBot(PickDef);
                                auto Entry = bot->GetEntry(PickDef);
                                if (Entry)
                                        bot->Pawn->EquipWeaponDefinition((UFortWeaponMeleeItemDefinition*)Entry->ItemDefinition, Entry->ItemGuid);
                        }
                        else {
                                Log("Default Pickaxe dont exist!");
                        }
                }
                else {
                        auto PickDef = Pickaxes[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, (int)Pickaxes.size() - 1)];
                        if (!PickDef)
                        {
                                Log("Cooked!");
                                UFortWeaponMeleeItemDefinition* PickDef2 = StaticLoadObject<UFortWeaponMeleeItemDefinition>("/Game/Athena/Items/Weapons/WID_Harvest_Pickaxe_Athena_C_T01.WID_Harvest_Pickaxe_Athena_C_T01");
                                if (PickDef2) {
                                        bot->GiveItemBot(PickDef2);
                                        auto Entry = bot->GetEntry(PickDef2);
                                        if (Entry)
                                                bot->Pawn->EquipWeaponDefinition((UFortWeaponMeleeItemDefinition*)Entry->ItemDefinition, Entry->ItemGuid);
                                }
                                else {
                                        Log("Default Pickaxe dont exist!");
                                }
                        }
                        else {
                                if (PickDef && PickDef->WeaponDefinition)
                                {
                                        bot->GiveItemBot(PickDef->WeaponDefinition);
                                }

                                auto Entry = bot->GetEntry(PickDef->WeaponDefinition);
                                if (Entry)
                                        bot->Pawn->EquipWeaponDefinition((UFortWeaponItemDefinition*)Entry->ItemDefinition, Entry->ItemGuid);
                        }
                }

                if (BotDisplayNames.size() != 0) {
                        std::srand(static_cast<unsigned int>(std::time(0)));
                        int randomIndex = std::rand() % (int)BotDisplayNames.size();
                        std::string rdName = BotDisplayNames[randomIndex];
                        BotDisplayNames.erase(BotDisplayNames.begin() + randomIndex);

                        // MultiByteToWideChar(CP_UTF8, ...) → Ogs::Text (same UTF-8 →
                        // wide conversion, implemented generically over the SDK element
                        // type).
                        FString CVName = Ogs::Text(rdName.c_str());
                        GameMode->ChangeName(bot->PC, CVName, true);
                        bot->DisplayName = CVName;

                        bot->PlayerState->OnRep_PlayerName();
                }

                for (auto SkillSet : bot->PC->DigestedBotSkillSets)
                {
                        if (!SkillSet)
                                continue;

                        if (auto AimingSkill = Cast<UFortAthenaAIBotAimingDigestedSkillSet>(SkillSet))
                                bot->PC->CacheAimingDigestedSkillSet = AimingSkill;

                        if (auto AttackingSkill = Cast<UFortAthenaAIBotAttackingDigestedSkillSet>(SkillSet))
                                bot->PC->CacheAttackingSkillSet = AttackingSkill;

                        if (auto HarvestSkill = Cast<UFortAthenaAIBotHarvestDigestedSkillSet>(SkillSet))
                                bot->PC->CacheHarvestDigestedSkillSet = HarvestSkill;

                        if (auto InventorySkill = Cast<UFortAthenaAIBotInventoryDigestedSkillSet>(SkillSet))
                                bot->PC->CacheInventoryDigestedSkillSet = InventorySkill;

                        if (auto LootingSkill = Cast<UFortAthenaAIBotLootingDigestedSkillSet>(SkillSet))
                                bot->PC->CacheLootingSkillSet = LootingSkill;

                        if (auto MovementSkill = Cast<UFortAthenaAIBotMovementDigestedSkillSet>(SkillSet))
                                bot->PC->CacheMovementSkillSet = MovementSkill;

                        if (auto PerceptionSkill = Cast<UFortAthenaAIBotPerceptionDigestedSkillSet>(SkillSet))
                                bot->PC->CachePerceptionDigestedSkillSet = PerceptionSkill;

                        if (auto PlayStyleSkill = Cast<UFortAthenaAIBotPlayStyleDigestedSkillSet>(SkillSet))
                                bot->PC->CachePlayStyleSkillSet = PlayStyleSkill;
                }

                bot->PC->BehaviorTree = BehaviorTree;
                bot->PC->RunBehaviorTree(BehaviorTree);
                bot->PC->UseBlackboard(BehaviorTree->BlackboardAsset, &bot->PC->Blackboard);
                bot->PC->UseBlackboard(BehaviorTree->BlackboardAsset, &bot->PC->Blackboard1);

                static auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                static auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                bot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::Warmup);
                bot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Warmup);

                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->NavigationSystem);
                bot->PC->PathFollowingComponent->MyNavData = ((UAthenaNavSystem*)World->NavigationSystem)->MainNavData;
                bot->PC->PathFollowingComponent->OnNavDataRegistered(((UAthenaNavSystem*)World->NavigationSystem)->MainNavData);
                if (((UAthenaNavSystem*)World->NavigationSystem)->MainNavData) {
                        //Log("NavData!");
                }
                else {
                        Log("No NavData!");
                }

                bot->PlayerState->OnRep_CharacterData();

                bot->Pawn->CapsuleComponent->SetGenerateOverlapEvents(true);
                bot->Pawn->CharacterMovement->bCanWalkOffLedges = true;

                bot->Pawn->SetMaxHealth(100);
                bot->Pawn->SetHealth(100);
                bot->Pawn->SetMaxShield(100);
                bot->Pawn->SetShield(0);

                bot->BotState = StartingState;

                if (TeamPC) {
                        AFortPlayerStateAthena* TeamPS = (AFortPlayerStateAthena*)TeamPC->PlayerState;

                        uint8 OldTeamIdx = bot->PlayerState->TeamIndex;

                        bot->PlayerState->bIsABot = false;

                        bot->PlayerState->TeamIndex = TeamPS->TeamIndex;
                        bot->PlayerState->SquadId = TeamPS->SquadId;
                        bot->PlayerState->OnRep_TeamIndex(OldTeamIdx);
                        bot->PlayerState->OnRep_SquadId();

                        TeamPS->PlayerTeam->TeamMembers.Add(bot->PC);
                        bot->PlayerState->PlayerTeam = TeamPS->PlayerTeam;

                        bot->PlayerState->OnRep_PlayerTeam();
                        TeamPS->OnRep_PlayerTeam();

                        bot->PlayerState->bIsABot = true;
                }

                PlayerBotArray.push_back(bot); // gotta do this so we can tick them all
                //Log("Bot Spawned With DisplayName: " + bot->DisplayName.ToString());
        }

        void Tick() {
                if (!PlayerBotArray.empty()) {
                        if (UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.001f))
                        {
                                FreeDeadBots();
                        }
                }
                else {
                        return;
                }

                auto GameState = OgsSDK::GameState();
                auto Math = OgsSDK::Math();
                OGS_CHECK(GameState && Math);

                for (auto bot : PlayerBotArray) {
                        if (!bot || !bot->Pawn || !bot->PC || !bot->PlayerState)
                                continue;

                        if (GameState->GamePhase <= EAthenaGamePhase::Warmup) {
                                if (bot->tick_counter <= 150) {
                                        bot->tick_counter++;
                                        continue;
                                }
                        }

                        if (bot->BotState > EBotState::Bus) {
                                BotsBTService_AIEvaluator Evaluator;
                                Evaluator.Tick(bot); // tick the evaluator after the bot is out of the bus so we dont mess up anything or cause potential crash
                        }

                        if (bot->BotState == EBotState::Warmup) {
                                BotsBTService_Warmup Warmup;
                                Warmup.Tick(bot);
                        }
                        else if (bot->BotState == EBotState::PreBus) {
                                bot->Pawn->SetHealth(100);
                                bot->Pawn->SetShield(100);
                                if (!bot->bHasThankedBusDriver && UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.0005f))
                                {
                                        bot->bHasThankedBusDriver = true;
                                        bot->PC->ThankBusDriver();
                                }
                        }
                        else if (bot->BotState == EBotState::Bus || bot->BotState == EBotState::Skydiving || bot->BotState == EBotState::Gliding) {
                                if (Globals::LateGame && UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.5f)) {
                                        if (bot->bHasJumpedFromBus) {
                                                bot->BotState = EBotState::LookingForPlayers;
                                                return;
                                        }

                                        bot->Pawn->K2_TeleportTo(GameState->GetAircraft(0)->K2_GetActorLocation(), {});
                                        bot->Pawn->BeginSkydiving(true);
                                        bot->GiveLategameLoadout();
                                        bot->BotState = EBotState::LookingForPlayers;

                                        bot->bHasJumpedFromBus = true;

                                        return;
                                }

                                BotsBTService_AIDropZone DropZoneEv;
                                DropZoneEv.Tick(bot);
                        }
                        else if (bot->BotState == EBotState::Landed) {
                                FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                                if (bot->NearestPlayerActor) {
                                        float Dist = Math->Vector_Distance(BotLoc, bot->NearestPlayerActor->K2_GetActorLocation());
                                        if (Dist < 6000.f) {
                                                // When the bot lands the bot will have no guns so we need to get away before dying
                                                //bot->BotState = EBotState::Fleeing; // Fleeing is kinda cooked atm

                                                bot->BotState = EBotState::Looting;
                                        }
                                        else {
                                                bot->BotState = EBotState::Looting;
                                        }
                                }
                                else {
                                        bot->BotState = EBotState::Looting;
                                }
                        }
                        else if (bot->BotState == EBotState::Fleeing) {
                                FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                                FVector Nearest = bot->FindNearestPlayerOrBot();
                                if (!Nearest.IsZero()) {
                                        float Dist = Math->Vector_Distance(BotLoc, Nearest);
                                        if (bot->HasGun()) {
                                                bot->BotState = EBotState::LookingForPlayers;
                                                continue;
                                        }

                                        if (Dist < 3000.f) {
                                                auto TestRot = Math->FindLookAtRotation(Nearest, BotLoc);

                                                bot->PC->SetControlRotation(TestRot);
                                                bot->PC->K2_SetActorRotation(TestRot, true);
                                                bot->Pawn->AddMovementInput(bot->Pawn->GetActorForwardVector(), 1.2f, true);
                                        }
                                        else {
                                                bot->BotState = EBotState::Looting;
                                        }
                                }
                                else {
                                        bot->BotState = EBotState::Looting;
                                }
                        }
                        else if (bot->BotState == EBotState::Looting) {
                                BotsBTService_Loot LootAI;
                                LootAI.Tick(bot);
                        }
                        else if (bot->BotState == EBotState::LookingForPlayers) {
                                if (!bot->HasGun()) {
                                        bot->BotState = EBotState::Looting;
                                }
                                if (bot->IsPickaxeEquiped()) {
                                        bot->SimpleSwitchToWeapon();
                                }

                                FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                                if (bot->NearestPlayerActor) {
                                        FVector Nearest = bot->NearestPlayerActor->K2_GetActorLocation();

                                        FRotator TestRot;
                                        FVector TargetPosmod = Nearest;

                                        if (!Nearest.IsZero()) {
                                                float Dist = Math->Vector_Distance(BotLoc, Nearest);

                                                if (bot->PC->LineOfSightTo(bot->NearestPlayerActor, BotLoc, true)) {
                                                        if (true) {
                                                                float RandomXmod = Math->RandomFloatInRange(-180, 180);
                                                                float RandomYmod = Math->RandomFloatInRange(-180, 180);
                                                                float RandomZmod = Math->RandomFloatInRange(-180, 180);

                                                                FVector TargetPosMod{ Nearest.X + RandomXmod, Nearest.Y + RandomYmod, Nearest.Z + RandomZmod };

                                                                FRotator Rot = Math->FindLookAtRotation(BotLoc, TargetPosMod);

                                                                bot->PC->SetControlRotation(Rot);
                                                                bot->PC->K2_SetActorRotation(Rot, true);

                                                                //bot->PC->K2_SetFocalPoint(TargetPosMod);
                                                        }

                                                        if (UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.001)) {
                                                                TestRot = Math->FindLookAtRotation(BotLoc, Nearest);

                                                                bot->PC->SetControlRotation(TestRot);
                                                                bot->PC->K2_SetActorRotation(TestRot, true);
                                                        }

                                                        if (!bot->Pawn->bIsCrouched && Math->RandomBoolWithWeight(0.025f)) {
                                                                bot->Pawn->Crouch(false);
                                                        }

                                                        bot->ForceStrafe(true);

                                                        if (Dist < 1000) {
                                                                FVector BackVector = bot->Pawn->GetActorForwardVector() * -1.0f;
                                                                bot->Pawn->AddMovementInput(BackVector, 1.1f, true);
                                                        }

                                                        if (!bot->bIsStressed) {
                                                                bot->PC->MoveToActor(bot->NearestPlayerActor, Math->RandomFloatInRange(400, 1500), true, false, true, nullptr, true);
                                                        }
                                                        else {
                                                                // Somewhat retreating
                                                                bot->Pawn->AddMovementInput((bot->Pawn->GetActorForwardVector() * -1.0f), 1.2f, true);
                                                        }

                                                        //bot->PC->StopMovement();
                                                        if (bot->PC->LineOfSightTo(bot->NearestPlayerActor, BotLoc, true)) {
                                                                bot->Pawn->PawnStartFire(0);
                                                        }
                                                        else {
                                                                bot->Pawn->PawnStopFire(0);
                                                        }
                                                }
                                                else {
                                                        bot->BotState = EBotState::MovingToSafeZone;
                                                }
                                        }
                                        else {}
                                }
                        }
                        else if (bot->BotState == EBotState::MovingToSafeZone) {
                                FVector BotLoc = bot->Pawn->K2_GetActorLocation();
                                if (bot->NearestPlayerActor) {
                                        float Dist = Math->Vector_Distance(BotLoc, bot->NearestPlayerActor->K2_GetActorLocation());

                                        if (Dist < 4000.f) {
                                                if (!bot->HasGun()) {
                                                        //bot->BotState = EBotState::Fleeing;
                                                        bot->BotState = EBotState::Looting;
                                                }
                                                else {
                                                        bot->BotState = EBotState::LookingForPlayers;
                                                }
                                        }
                                }

                                if (GameState && GameState->SafeZoneIndicator)
                                {
                                        if (Math->RandomBoolWithWeight(0.025f)) {
                                                bot->ForceStrafe(true);
                                        }
                                        bot->PC->MoveToLocation(GameState->SafeZoneIndicator->NextCenter, GameState->SafeZoneIndicator->Radius, true, false, false, true, nullptr, true);
                                }
                        }
                        else if (bot->BotState == EBotState::Stuck) {
                                if (!bot->IsPickaxeEquiped()) {
                                        bot->EquipPickaxe();
                                }
                                bot->Pawn->PawnStartFire(0);

                                if (!bot->Pawn->bIsCrouched && Math->RandomBoolWithWeight(0.025f)) {
                                        bot->Pawn->Crouch(false);
                                }

                                if (Math->RandomBoolWithWeight(0.05f)) {
                                        bot->Pawn->UnCrouch(false);
                                        bot->Pawn->Jump();
                                }

                                if (bot->StuckBuildingActor) {
                                        if (bot->StuckBuildingActor->GetHealth() <= 1) {
                                                bot->Pawn->PawnStopFire(0);
                                                bot->StuckBuildingActor = nullptr;
                                                bot->BotState = bot->CachedBotState;
                                        }
                                        else {
                                                if (!bot->IsPickaxeEquiped()) {
                                                        bot->EquipPickaxe();
                                                }
                                                bot->LookAt(bot->StuckBuildingActor);
                                                bot->Pawn->PawnStartFire(0);
                                        }
                                }
                                else {
                                        bot->ForceStrafe(true);
                                        bot->BotState = bot->CachedBotState;
                                }

                                bot->Pawn->PawnStopFire(0);
                        }

                        bot->tick_counter++;
                }
        }
}

// =============================================================================
// Bots.h (orig)
// =============================================================================
namespace Bots {
        void StartTree(UBehaviorTreeComponent* BTComp, UBehaviorTree* BTAsset, EBTExecutionMode::Type Mode = EBTExecutionMode::Looped)
        {
                if (!BTComp)
                {
                        Log("StartTree: Unable to run NULL BTComp");
                        return;
                }

                BTComp->DefaultBehaviorTreeAsset = BTAsset;

                BTComp->StartLogic();
                return;
        }

        // I have suspicion this is stripped so i will recreate it
        bool RunBehaviorTree(AFortAthenaAIBotController* PC, UBehaviorTree* BTAsset)
        {
                // @todo: find BrainComponent and see if it's BehaviorTreeComponent
                // Also check if BTAsset requires BlackBoardComponent, and if so
                // check if BB type is accepted by BTAsset.
                // Spawn BehaviorTreeComponent if none present.
                // Spawn BlackBoardComponent if none present, but fail if one is present but is not of compatible class
                if (BTAsset == NULL)
                {
                        Log("RunBehaviorTree: Unable to run NULL behavior tree");
                        return false;
                }

                bool bSuccess = true;

                // see if need a blackboard component at all
                UBlackboardComponent* BlackboardComp = PC ? PC->Blackboard : nullptr;
                if (BTAsset->BlackboardAsset && PC->Blackboard == nullptr)
                {
                        bSuccess = PC->UseBlackboard(BTAsset->BlackboardAsset, &BlackboardComp);
                }

                if (bSuccess)
                {
                        UBehaviorTreeComponent* BTComp = Cast<UBehaviorTreeComponent>(PC->BrainComponent);
                        if (BTComp == NULL)
                        {
                                Log("RunBehaviorTree: spawning BehaviorTreeComponent..");

                                BTComp = (UBehaviorTreeComponent*)((UGameplayStatics*)UGameplayStatics::StaticClass()->DefaultObject)->SpawnObject(UBehaviorTreeComponent::StaticClass(), PC);
                                auto registerComp = OgsFn::register_component_with_world();
                                if (registerComp && BTComp)
                                        registerComp(BTComp, UWorld::GetWorld());
                                else
                                        OGS_LOG("RegisterComponentWithWorld skipped (offset missing)");
                        }
                        if (!BTComp)
                                return false;

                        // make sure BrainComponent points at the newly created BT component
                        PC->BrainComponent = BTComp;

                        //BTComp->StartTree(*BTAsset, EBTExecutionMode::Looped);
                        if (PC->RunBehaviorTree(BTAsset)) {
                                Log("BehaviourTree Ran Successfully!");
                        }
                        else {
                                Log("RunBehaviorTree returned false, manually running...");
                                StartTree(BTComp, BTAsset, EBTExecutionMode::Looped);
                        }
                }

                return bSuccess;
        }

        AFortPlayerPawnAthena* (*SpawnBotOG)(UFortServerBotManagerAthena* BotManager, FVector SpawnLoc, FRotator SpawnRot, UFortAthenaAIBotCustomizationData* BotData, FFortAthenaAIBotRunTimeCustomizationData RuntimeBotData);
        AFortPlayerPawnAthena* SpawnBot(UFortServerBotManagerAthena* BotManager, FVector SpawnLoc, FRotator SpawnRot, UFortAthenaAIBotCustomizationData* BotData, FFortAthenaAIBotRunTimeCustomizationData RuntimeBotData)
        {
                // Original: if (__int64(_ReturnAddress()) - __int64(GetModuleHandleA(0)) == 0x1A4153F)
                // The caller-RVA guard is registry-gated (spawnbot_self_call_rva);
                // missing → never pass through to the original here.
                const auto& selfCall = Ogs::Off().spawnbot_self_call_rva;
                if (selfCall.Found &&
                        (uintptr_t)OGS_RETURN_ADDRESS() - Ogs::Base() == selfCall.Rva) {
                        return SpawnBotOG(BotManager, SpawnLoc, SpawnRot, BotData, RuntimeBotData);
                }

                OGS_CHECK(BotData && BotData->CharacterCustomization);
                std::string BotName = BotData->Name.ToString();

                if (BotData->GetFullName().find("MANG_POI_Yacht") != std::string::npos)
                {
                        BotData = StaticLoadObject<UFortAthenaAIBotCustomizationData>("/Game/Athena/AI/MANG/BotData/BotData_MANG_POI_HDP.BotData_MANG_POI_HDP");
                        OGS_CHECK(BotData && BotData->CharacterCustomization);
                }

                if (BotData->CharacterCustomization->CustomizationLoadout.Character
                        && BotData->CharacterCustomization->CustomizationLoadout.Character->GetName() == "CID_556_Athena_Commando_F_RebirthDefaultA")
                {
                        std::string Tag = RuntimeBotData.PredefinedCosmeticSetTag.TagName.ToString();
                        if (Tag == "Athena.Faction.Alter") {
                                BotData->CharacterCustomization->CustomizationLoadout.Character = StaticLoadObject<UAthenaCharacterItemDefinition>("/Game/Athena/Items/Cosmetics/Characters/CID_NPC_Athena_Commando_M_HenchmanBad.CID_NPC_Athena_Commando_M_HenchmanBad");
                        }
                        else if (Tag == "Athena.Faction.Ego") {
                                BotData->CharacterCustomization->CustomizationLoadout.Character = StaticLoadObject<UAthenaCharacterItemDefinition>("/Game/Athena/Items/Cosmetics/Characters/CID_NPC_Athena_Commando_M_HenchmanGood.CID_NPC_Athena_Commando_M_HenchmanGood");
                        }
                }

                AActor* SpawnLocator = SpawnActor<ADefaultPawn>(SpawnLoc, SpawnRot);
                OGS_CHECK(SpawnLocator && BotMutator);
                AFortPlayerPawnAthena* Ret = BotMutator->SpawnBot(BotData->PawnClass, SpawnLocator, SpawnLoc, SpawnRot, true);
                OGS_CHECK(Ret && Ret->Controller);
                AFortAthenaAIBotController* PC = (AFortAthenaAIBotController*)Ret->Controller;

                PC->CosmeticLoadoutBC = BotData->CharacterCustomization->CustomizationLoadout;
                PC->CachedPatrollingComponent = (UFortAthenaNpcPatrollingComponent*)UGameplayStatics::SpawnObject(UFortAthenaNpcPatrollingComponent::StaticClass(), PC);
                OGS_CHECK(BotData->CharacterCustomization->CustomizationLoadout.Character && BotData->CharacterCustomization->CustomizationLoadout.Character->HeroDefinition);
                for (int32 i = 0; i < BotData->CharacterCustomization->CustomizationLoadout.Character->HeroDefinition->Specializations.Num(); i++)
                {
                        UFortHeroSpecialization* Spec = StaticLoadObject<UFortHeroSpecialization>(UKismetStringLibrary::Conv_NameToString(BotData->CharacterCustomization->CustomizationLoadout.Character->HeroDefinition->Specializations[i].ObjectID.AssetPathName).ToString());

                        if (Spec)
                        {
                                for (int32 j = 0; j < Spec->CharacterParts.Num(); j++)
                                {
                                        UCustomCharacterPart* Part = StaticLoadObject<UCustomCharacterPart>(UKismetStringLibrary::Conv_NameToString(Spec->CharacterParts[j].ObjectID.AssetPathName).ToString());
                                        if (Part)
                                                Ret->ServerChoosePart(Part->CharacterPartType, Part);
                                }
                        }
                }

                Ret->CosmeticLoadout = BotData->CharacterCustomization->CustomizationLoadout;
                Ret->OnRep_CosmeticLoadout();

                SpawnLocator->K2_DestroyActor();
                uint32_t CustomSquadId = RuntimeBotData.CustomSquadId;
                uint8_t TrueByte = 1;
                uint8_t FalseByte = 0;

                static UBehaviorTree* BehaviorTree = StaticLoadObject<UBehaviorTree>("/Game/Athena/AI/MANG/BehaviorTree/BT_MANG2.BT_MANG2");
                static UBlackboardData* Blackboard = StaticLoadObject<UBlackboardData>("/Game/Athena/AI/MANG/BehaviorTree/BB_MANG2.BB_MANG2");
                if (!BehaviorTree)
                {
                        Log("Behaviourtree does not exist!");
                }
                if (!Blackboard)
                {
                        Log("Blackboard doesent exist!");
                }

                auto botManagerSetup = OgsFn::bot_manager_setup();
                if (botManagerSetup)
                {
                        botManagerSetup((int64_t)BotManager, (int64_t)Ret, (int64_t)BotData->BehaviorTree, 0, &CustomSquadId, 0,
                                (int64_t)BotData->StartupInventory, (int64_t)BotData->BotNameSettings, 0, &FalseByte, 0, &TrueByte, RuntimeBotData);
                }
                else
                {
                        OGS_LOG("BotManagerSetup skipped (offset missing) — faction bot is half-initialised");
                }

                FactionBot* bot = new FactionBot(Ret);

                bot->CID = BotData->CharacterCustomization->CustomizationLoadout.Character->GetName();

                if (BotData->StartupInventory)
                {
                        for (int32 i = 0; i < BotData->StartupInventory->Items.Num(); i++)
                        {
                                bool equip = true;

                                if (BotData->StartupInventory->Items[i]
                                        && (BotData->StartupInventory->Items[i]->GetName().rfind("WID_Athena_FloppingRabbit", 0) == 0 || BotData->StartupInventory->Items[i]->GetName().rfind("WID_Boss_Adventure_GH", 0) == 0)) {
                                        equip = false;
                                }
                                bot->GiveItem(BotData->StartupInventory->Items[i], 1, equip);
                                if (auto Data = Cast<UFortWeaponItemDefinition>(BotData->StartupInventory->Items[i]))
                                {
                                        if (Data->GetAmmoWorldItemDefinition_BP() && Data->GetAmmoWorldItemDefinition_BP() != Data)
                                        {
                                                bot->GiveItem(Data->GetAmmoWorldItemDefinition_BP(), 99999);
                                        }
                                }
                        }
                }

                TArray<AFortAthenaPatrolPath*> PatrolPaths;

                auto Statics = OgsSDK::Statics();
                Statics->GetAllActorsOfClass(UWorld::GetWorld(), AFortAthenaPatrolPath::StaticClass(), (TArray<AActor*>*)&PatrolPaths);

                bot->Name = BotName;
                for (int i = 0; i < PatrolPaths.Num(); i++) {
                        if (PatrolPaths[i] && PatrolPaths[i]->PatrolPoints.Num() > 0 && PatrolPaths[i]->PatrolPoints[0]
                                && PatrolPaths[i]->PatrolPoints[0]->K2_GetActorLocation() == SpawnLoc) {
                                //Log("Found patrol path for bot: " + BotData->GetFullName());
                                bot->PC->CachedPatrollingComponent->SetPatrolPath(PatrolPaths[i]);
                                PC->CachedPatrollingComponent->SetPatrolPath(PatrolPaths[i]);
                                bot->PatrolPath = PatrolPaths[i];
                        }
                }

                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->NavigationSystem && PC->PathFollowingComponent);
                PC->PathFollowingComponent->MyNavData = ((UAthenaNavSystem*)World->NavigationSystem)->MainNavData;
                PC->PathFollowingComponent->OnNavDataRegistered(PC->PathFollowingComponent->MyNavData);
                PC->PathFollowingComponent->Activate(true);
                PC->PathFollowingComponent->SetActive(true, true);
                PC->PathFollowingComponent->OnRep_IsActive();

                if (!PC->BrainComponent) {
                        PC->BrainComponent = (UBrainComponent*)UGameplayStatics::SpawnObject(UBrainComponent::StaticClass(), PC);
                        PC->BrainComponent->Activate(false);
                        PC->BrainComponent->SetActive(true, false);
                        PC->BrainComponent->OnRep_IsActive();
                }

                PC->UseBlackboard(Blackboard, &PC->Blackboard);
                PC->OnUsingBlackBoard(PC->Blackboard, Blackboard);

                PC->Blackboard->SetValueAsBool(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_IsMovementBlocked")), false);

                PC->BehaviorTree = BehaviorTree;
                /*if (Globals::bUseLegacyAI_MANG) {
                        if (RunBehaviorTree(PC, StaticLoadObject<UBehaviorTree>("/Game/Athena/AI/MANG/BehaviorTree/BT_MANG2.BT_MANG2"))) {
                                Log("Hi!");
                        }
                        else {
                                Log("Bye!");
                        }
                        PC->BlueprintOnBehaviorTreeStarted();
                }*/

                static auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                static auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::Warmup);
                PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Warmup);

                PC->Blackboard->SetValueAsBool(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_IsMovementBlocked")), false);
                PC->Blackboard->SetValueAsEnum(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_AvoidThreat_ExecutionStatus")), (uint8)EExecutionStatus::ExecutionAllowed);
                PC->Blackboard->SetValueAsEnum(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Leash_ExecutionStatus")), (uint8)EExecutionStatus::ExecutionAllowed);
                PC->Blackboard->SetValueAsEnum(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Patrolling_ExecutionStatus")), (uint8)EExecutionStatus::ExecutionAllowed);
                PC->Blackboard->SetValueAsEnum(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_DynamicBlueprint_ExecutionStatus")), (uint8)EExecutionStatus::ExecutionAllowed);
                PC->Blackboard->SetValueAsEnum(UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_CharacterLaunched_ExecutionStatus")), (uint8)EExecutionStatus::ExecutionAllowed);

                OGS_CHECK(Ret->Mesh);
                Ret->Mesh->AnimBlueprintGeneratedClass = StaticLoadObject<UClass>("/Game/Athena/AI/MANG/AnimSet/MANG_PatrolLayerAnimBP.MANG_PatrolLayerAnimBP_C");
                Ret->OnRep_AnimBPOverride();

                PC->OnRep_Pawn();

                PC->BrainComponent->RestartLogic();

                bot->BT_MANG = BT_MANG2::ConstructBehaviorTree();

                return Ret;
        }

        inline char (*SetUpInventoryBotOG)(AActor* Pawn, int64_t a2);
        inline char SetUpInventoryBot(AActor* Pawn, int64_t a2)
        {
                // Original: *(AActor**)(__int64(Pawn) + 1352) = SpawnActor<AFortInventory>(...)
                // 1352 == 0x548 == offsetof(AFortAthenaAIBotController, Inventory)
                // (reflected member) — the "Pawn" parameter is the bot controller.
                reinterpret_cast<AFortAthenaAIBotController*>(Pawn)->Inventory = SpawnActor<AFortInventory>({}, {}, Pawn);

                return SetUpInventoryBotOG(Pawn, a2);
        }

        void (*OnPossessedPawnDiedOG)(ABP_PhoebePlayerController_C* PC, AActor* DamagedActor, float Damage, AController* InstigatedBy, AActor* DamageCauser, FVector HitLocation, UPrimitiveComponent* HitComp, FName BoneName, FVector Momentum);
        void OnPossessedPawnDied(ABP_PhoebePlayerController_C* PC, AActor* DamagedActor, float Damage, AController* InstigatedBy, AActor* DamageCauser, FVector HitLocation, UPrimitiveComponent* HitComp, FName BoneName, FVector Momentum)
        {
                if (!PC) {
                        return;
                }

                FactionBot* KilledBot = nullptr;
                for (size_t i = 0; i < FactionBots.size(); i++)
                {
                        auto bot = FactionBots[i];
                        if (bot && bot->PC == PC)
                        {
                                if (bot->Pawn && bot->Pawn->GetName().rfind("BP_Pawn_DangerGrape_", 0) == 0) {
                                        goto nodrop;
                                }
                                else {
                                        KilledBot = bot;
                                }
                        }
                }

                if (KilledBot && InstigatedBy && InstigatedBy->PlayerState && !InstigatedBy->PlayerState->bIsABot) {
                        std::string DisguiseName;
                        AFortPlayerControllerAthena* PlayerController = (AFortPlayerControllerAthena*)InstigatedBy;
                        if (PlayerController && PlayerController->WorldInventory && PlayerController->WorldInventory->Inventory.ReplicatedEntries) {
                                for (size_t i = 0; i < PlayerController->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                {
                                        auto& Entry = PlayerController->WorldInventory->Inventory.ReplicatedEntries[i];
                                        if (Entry.ItemDefinition) {
                                                std::string ItemName = Entry.ItemDefinition->Name.ToString();
                                                if (ItemName.find("Disguise") != std::string::npos) {
                                                        DisguiseName = ItemName;
                                                }
                                        }
                                }
                        }

                        Log(KilledBot->Name);
                        if (KilledBot->Name.find("POI") != std::string::npos) {
                                Quests::GiveAccolade((AFortPlayerControllerAthena*)InstigatedBy, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_078_ElimBoss.AccoladeId_078_ElimBoss"));
                        }
                        else if (KilledBot->Name.find("Ego") != std::string::npos && DisguiseName.find("Alter") != std::string::npos) {
                                Quests::GiveAccolade((AFortPlayerControllerAthena*)InstigatedBy, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_081_ElimGhost.AccoladeId_081_ElimGhost"));
                        }
                        else if (KilledBot->Name.find("Alter") != std::string::npos && DisguiseName.find("Ego") != std::string::npos) {
                                Quests::GiveAccolade((AFortPlayerControllerAthena*)InstigatedBy, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_082_ElimShadow.AccoladeId_082_ElimShadow"));
                        }
                        else {
                                Quests::GiveAccolade((AFortPlayerControllerAthena*)InstigatedBy, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_075_ElimMang.AccoladeId_075_ElimMang"));
                        }
                }

                if (InstigatedBy && DamageCauser) {
                        for (auto& bot : PlayerBotArray)
                        {
                                if (bot && bot->PC && bot->PC == PC && !bot->bIsDead)
                                {
                                        bot->OnDied((AFortPlayerStateAthena*)InstigatedBy->PlayerState, DamageCauser, BoneName);
                                        break;
                                }
                        }
                }

                PC->PlayerBotPawn->SetMaxShield(0);
                for (int32 i = 0; i < PC->Inventory->Inventory.ReplicatedEntries.Num(); i++)
                {
                        if (PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition->IsA(UFortWeaponMeleeItemDefinition::StaticClass()) || PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition->IsA(UFortAmmoItemDefinition::StaticClass()))
                                continue;
                        auto Def = PC->Inventory->Inventory.ReplicatedEntries[i].ItemDefinition;
                        if (Def->GetName() == "AGID_Athena_Keycard_Yacht") {
                                goto nodrop;
                        }
                        if (Def->GetName() == "WID_Boss_Tina") {
                                Def = KilledBot ? KilledBot->Weapon : Def;
                        }
                        SpawnPickup(Def, PC->Inventory->Inventory.ReplicatedEntries[i].Count, PC->Inventory->Inventory.ReplicatedEntries[i].LoadedAmmo, PC->Pawn->K2_GetActorLocation(), EFortPickupSourceTypeFlag::Other, EFortPickupSpawnSource::PlayerElimination);
                        int Ammo = 0;
                        int AmmoC = 0;
                        if (Def->GetName() == "WID_Boss_Hos_MG") {
                                Ammo = 60;
                                AmmoC = 60;
                        }
                        else if (Def->GetName().rfind("WID_Assault_LMG_Athena_", 0) == 0) {
                                Ammo = 45;
                                AmmoC = 45;
                        }
                        if (Def->IsA(UFortWeaponRangedItemDefinition::StaticClass()))
                        {
                                UFortWeaponItemDefinition* Def2 = (UFortWeaponItemDefinition*)Def;
                                SpawnPickup(Def2->GetAmmoWorldItemDefinition_BP(), AmmoC != 0 ? AmmoC : Looting::GetClipSize(Def2), Ammo != 0 ? Ammo : Looting::GetClipSize(Def2), PC->Pawn->K2_GetActorLocation(), EFortPickupSourceTypeFlag::Other, EFortPickupSpawnSource::PlayerElimination);
                        }
                }

        nodrop:
                for (size_t i = 0; i < FactionBots.size(); i++)
                {
                        auto bot = FactionBots[i];
                        if (bot && bot->PC == PC)
                        {
                                FactionBots.erase(FactionBots.begin() + i);
                                break;
                        }
                }

                return OnPossessedPawnDiedOG(PC, DamagedActor, Damage, InstigatedBy, DamageCauser, HitLocation, HitComp, BoneName, Momentum);
        }

        wchar_t* (*OnPerceptionSensedOG)(ABP_PhoebePlayerController_C* PC, AActor* SourceActor, FAIStimulus& Stimulus);
        wchar_t* OnPerceptionSensed(ABP_PhoebePlayerController_C* PC, AActor* SourceActor, FAIStimulus& Stimulus)
        {
                OGS_CHECK(SourceActor);

                if (SourceActor->IsA(AFortPlayerPawnAthena::StaticClass()))
                {
                        std::string DisguiseName;
                        AFortPlayerController* PlayerController = Cast<AFortPlayerController>(SourceActor->GetOwner());
                        if (PlayerController && PlayerController->WorldInventory && PlayerController->WorldInventory->Inventory.ReplicatedEntries) {
                                for (size_t i = 0; i < PlayerController->WorldInventory->Inventory.ReplicatedEntries.Num(); i++)
                                {
                                        auto& Entry = PlayerController->WorldInventory->Inventory.ReplicatedEntries[i];
                                        if (Entry.ItemDefinition) {
                                                std::string ItemName = Entry.ItemDefinition->Name.ToString();
                                                if (ItemName.find("Disguise") != std::string::npos) {
                                                        DisguiseName = ItemName;
                                                }
                                        }
                                }
                        }

                        for (auto bot : FactionBots)
                        {
                                if (bot->PC == PC)
                                {
                                        if (!DisguiseName.empty()) {
                                                Log(bot->Name);
                                                if (bot->Name.find("Ego") != std::string::npos && DisguiseName.find("Ego") != std::string::npos) {
                                                        return nullptr;
                                                }
                                                else if (bot->Name.find("Alter") != std::string::npos && DisguiseName.find("Alter") != std::string::npos) {
                                                        return nullptr;
                                                }
                                        }
                                        bot->OnPerceptionSensed(SourceActor, Stimulus);
                                }
                        }
                }

                return OnPerceptionSensedOG(PC, SourceActor, Stimulus);
        }

        void (*OnActorBumpOG)(UPathFollowingComponent* Comp, AActor* SelfActor, AActor* OtherActor, const FVector& NormalImpulse, FHitResult& Hit);
        void OnActorBump(UFortAthenaAIBotPathFollowingComponent* Comp, AActor* SelfActor, AActor* OtherActor, const FVector& NormalImpulse, FHitResult& Hit)
        {
                OGS_CHECK(Comp);
                auto Math = OgsSDK::Math();

                //Log("OnActorBump Called!");
                for (auto& bot : FactionBots)
                {
                        if (bot && bot->PC == Comp->BotController)
                        {
                                bot->bWasReportedStuck = true;

                                if (!bot->Pawn->bIsCrouched && Math->RandomBoolWithWeight(0.1f)) {
                                        bot->Pawn->Crouch(false);
                                }
                                if (bot->Pawn->bIsCrouched && (bot->tick_counter % 30) == 0) {
                                        bot->Pawn->UnCrouch(false);
                                }

                                if (Math->RandomBoolWithWeight(0.025f)) {
                                        bot->Pawn->UnCrouch(false);
                                        bot->Pawn->Jump();
                                }

                                bot->Pawn->EquipWeaponDefinition(bot->Pickaxe, bot->PickaxeGuid);

                                bot->Pawn->PawnStartFire(0);

                                if (UKismetMathLibrary::GetDefaultObj()->RandomBool()) {
                                        bot->Pawn->AddMovementInput((bot->Pawn->GetActorRightVector() * -1.0f), 1.5f, true);
                                }
                                else {
                                        bot->Pawn->AddMovementInput(bot->Pawn->GetActorRightVector(), 1.5f, true);
                                }

                                bot->Pawn->PawnStopFire(0);
                                break;
                        }
                }

                for (auto& bot : PlayerBotArray)
                {
                        if (bot && bot->PC == Comp->BotController)
                        {
                                bot->SetStuck(OtherActor, Hit);
                                break;
                        }
                }

                return OnActorBumpOG(Comp, SelfActor, OtherActor, NormalImpulse, Hit);
        }

        // Pathfinding
        inline void (*InitializeForWorldOG)(UNavigationSystemV1* NavSystem, UWorld* World, EFNavigationSystemRunMode Mode);
        void InitializeForWorld(UNavigationSystemV1* NavSystem, UWorld* World, EFNavigationSystemRunMode Mode)
        {
                Log("InitializeForWorld Called!");
                auto AthenaNavSystem = (UAthenaNavSystem*)NavSystem;
                AthenaNavSystem->bAutoCreateNavigationData = true;
                return InitializeForWorldOG(NavSystem, World, Mode);
        }

        void Hook() {
                OGS_HOOK_NATIVE_OFF(Ogs::Off().spawn_bot, SpawnBot, (void**)&SpawnBotOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().set_up_inventory_bot, SetUpInventoryBot, (void**)&SetUpInventoryBotOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_possessed_pawn_died, OnPossessedPawnDied, (void**)&OnPossessedPawnDiedOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_perception_sensed, OnPerceptionSensed, (void**)&OnPerceptionSensedOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_actor_bump, OnActorBump, (void**)&OnActorBumpOG);

                // HookVTable(UAthenaNavSystem::GetDefaultObj(), 0x53, InitializeForWorld, ...)
                // → ByExec first (reflected), ByIndex fallback from vt_indices.
                if (!OgsHook::ByExec(UAthenaNavSystem::GetDefaultObj(), "AthenaNavSystem", "InitializeForWorld", (void*)InitializeForWorld, (void**)&InitializeForWorldOG))
                {
                        long long idx = Ogs::CfgRef().VtIndex("UNavigationSystemV1_InitializeForWorld", 0x53);
                        OgsHook::ByIndex(UAthenaNavSystem::GetDefaultObj(), (int)idx, (void*)InitializeForWorld, (void**)&InitializeForWorldOG);
                }

                Log("Hooked Bots!");
        }
}

// =============================================================================
// Misc.h (orig)
// =============================================================================
namespace Misc {
        void nullFunc() {}

        int True() {
                return 1;
        }

        int False() {
                return 0;
        }

        static inline void (*KickPlayerOG)(AGameSession*, AController*);
        static void KickPlayer(AGameSession*, AController*) {
                return;
        }

        void (*DispatchRequestOG)(int64_t a1, uint64_t* a2, int a3);
        void DispatchRequest(int64_t a1, uint64_t* a2, int a3)
        {
                return DispatchRequestOG(a1, a2, 3);
        }

        void (*K2_DestroyActorOG)(AActor* This, int64_t a2);
        void K2_DestroyActor(AActor* This, int64_t a2)
        {
                OGS_CHECK(This);
                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->AuthorityGameMode && World->GameState);
                AFortGameModeAthena* GameMode = (AFortGameModeAthena*)World->AuthorityGameMode;
                AFortGameStateAthena* GameState = (AFortGameStateAthena*)World->GameState;

                std::string Name = This->GetName();
                //Log(Name);

                if (GameState->GamePhase <= EAthenaGamePhase::Warmup) {
                        if (Name.find("BGA") != std::string::npos || Name.find("StaticGenerator") != std::string::npos) {
                                This->bActorIsBeingDestroyed = true;
                        }
                }
                else {
                        if (Name.find("BGA") != std::string::npos || Name.find("StaticGenerator") != std::string::npos) {
                                if (This->bActorIsBeingDestroyed) {
                                        This->bActorIsBeingDestroyed = false;
                                }
                        }
                }

                return K2_DestroyActorOG(This, a2);
        }

        void LateGameAircraftThread(FVector BattleBusLocation)
        {
                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->GameState && World->AuthorityGameMode);
                auto GameState = (AFortGameStateAthena*)World->GameState;
                auto GameMode = (AFortGameModeAthena*)World->AuthorityGameMode;

                Ogs::SleepMs(2500);

                GameState->bAircraftIsLocked = false;

                GameMode->OnAircraftExitedDropZone(GameState->GetAircraft(0));

                GameState->GamePhase = EAthenaGamePhase::SafeZones;

                GameState->SafeZonesStartTime = 1;
        }

        uint8 NextIdx = 3;
        int CurrentPlayersOnTeam = 0;
        int MaxPlayersOnTeam;

        inline int64_t PickTeam(int64_t a1, uint8_t a2, int64_t a3)
        {
                uint8_t Ret = NextIdx;
                CurrentPlayersOnTeam++;

                if (CurrentPlayersOnTeam == MaxPlayersOnTeam)
                {
                        NextIdx++;
                        CurrentPlayersOnTeam = 0;
                }
                return Ret;
        };

        void Hook() {
                // collectgarbage (0x2D95E00) + duplicate nullFunc hook
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_collect_garbage, False, nullptr);
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_kick_player, KickPlayer, (void**)&KickPlayerOG); // Kickplayer
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_change_gamesession_id, False, nullptr);// change gamesession id
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_dispatch_request, DispatchRequest, (void**)&DispatchRequestOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_can_create_context, True, nullptr); // CanCreateContext

                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_nullfunc_0, nullFunc, nullptr);
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_nullfunc_1, nullFunc, nullptr);
                OGS_HOOK_NATIVE_OFF(Ogs::Off().misc_nullfunc_2, nullFunc, nullptr);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().k2_destroy_actor, K2_DestroyActor, (void**)&K2_DestroyActorOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().pick_team, PickTeam, nullptr);

                Log("Misc Hooked!");
        }
}

// =============================================================================
// Net.h (orig) — only uworld_getnetmode is available on Android today; the
// aactor_getnetmode hook is skipped + logged (scanner: found=false).
// =============================================================================
struct LocRot {
        FVector Location;
        FRotator Rotation;
};

TArray<LocRot> CameraSpawners;
TArray<AActor*> CameraActors;

namespace Net {
        enum class ENetMode
        {
                Standalone,
                DedicatedServer,
                ListenServer,
                Client,

                MAX,
        };

        ENetMode (*WorldGetNetModeOG)(UWorld*);
        ENetMode WorldGetNetMode(UWorld* a1)
        {
                OGS_CHECK(a1);
                std::string Name = a1->GetName();

                ENetMode OriginalNetMode = WorldGetNetModeOG(a1);
                //Log("WorldNetMode: " + std::to_string(static_cast<int>(OriginalNetMode)));
                return ENetMode::DedicatedServer;
        }

        ENetMode (*AActorGetNetModeOG)(AActor*);
        ENetMode AActorGetNetMode(AActor* a1)
        {
                OGS_CHECK(a1);
                std::string Name = a1->GetName();

                ENetMode OriginalNetMode = AActorGetNetModeOG(a1);
                //Log("AActorNetMode: " + std::to_string(static_cast<int>(OriginalNetMode)));
                return ENetMode::DedicatedServer;
        }

        void Hook() {
                if (Ogs::CfgRef().Has("netmode_hooks"))
                {
                        OGS_HOOK_NATIVE_OFF(Ogs::Off().uworld_getnetmode, WorldGetNetMode, (void**)&WorldGetNetModeOG);
                }
                else
                {
                        Ogs::Log("netmode_hooks off (config)");
                }

                // aactor_getnetmode: scanner found=false on 12.41-android → skip+log.
                if (Ogs::Off().aactor_getnetmode.Found)
                {
                        OgsHook::Native(Ogs::Resolve(Ogs::Off().aactor_getnetmode.Rva), (void*)AActorGetNetMode, (void**)&AActorGetNetModeOG);
                }
                else
                {
                        Ogs::Log("feature off (offset missing): aactor_getnetmode");
                }

                Log("Hooked Net!");
        }
}

// =============================================================================
// GameMode.h (orig)
// =============================================================================
namespace GameMode {
        namespace Event {
                static inline UClass* Starter;
                static inline UObject* JerkyLoader;
        }

        inline bool (*ReadyToStartMatchOG)(AFortGameModeAthena* GameMode);
        inline bool ReadyToStartMatch(AFortGameModeAthena* GameMode)
        {
                OGS_CHECK(GameMode);
                ReadyToStartMatchOG(GameMode);

                static bool SetupServer = false;
                static bool ServerListening = false;

                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->GameState);
                AFortGameStateAthena* GameState = (AFortGameStateAthena*)World->GameState;

                if (!SetupServer) {
                        static bool LoadedPlaylist = false;
                        if (!LoadedPlaylist) {
                                LoadedPlaylist = true;
                                UFortPlaylistAthena* Playlist;
                                if (Globals::bCreativeEnabled) {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Creative/Playlist_PlaygroundV2.Playlist_PlaygroundV2");
                                }
                                else if (Globals::bEventEnabled) {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Music/Playlist_Music_High.Playlist_Music_High");
                                }
                                else if (Globals::Arena) {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Showdown/Playlist_ShowdownAlt_Solo.Playlist_ShowdownAlt_Solo");
                                }
                                else if (Globals::Automatics)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Auto/Playlist_DefaultSolo.Playlist_DefaultSolo");
                                }
                                else if (Globals::BattleLab)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/BattleLab/Playlist_BattleLab.Playlist_BattleLab");
                                }
                                else if (Globals::Blitz)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Blitz/Playlist_Blitz_Solo.Playlist_Blitz_Solo");
                                }
                                else if (Globals::StormKing)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/DADBRO/Playlist_DADBRO_Squads.Playlist_DADBRO_Squads");
                                }
                                else if (Globals::Arsenal)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/gg/Playlist_Gg_Reverse.Playlist_Gg_Reverse");
                                }
                                else if (Globals::TeamRumble)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Respawn/Playlist_Respawn_Solo.Playlist_Respawn_Solo");
                                }
                                else if (Globals::SolidGold)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/SolidGold/Playlist_SolidGold_Solo.Playlist_SolidGold_Solo");
                                }
                                else if (Globals::UnVaulted)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Unvaulted/Playlist_Unvaulted_Solo.Playlist_Unvaulted_Solo");
                                }
                                else if (Globals::Siphon)
                                {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Vamp/Playlist_Vamp_Solo.Playlist_Vamp_Solo");
                                }
                                else {
                                        Playlist = StaticLoadObject<UFortPlaylistAthena>("/Game/Athena/Playlists/Playlist_DefaultSquad.Playlist_DefaultSquad");
                                }
                                if (!Playlist) {
                                        Log("Could not find playlist!");
                                        return false;
                                }
                                else {
                                        Log("Found Playlist!");
                                }

                                GameState->CurrentPlaylistInfo.BasePlaylist = Playlist;
                                GameState->CurrentPlaylistInfo.OverridePlaylist = Playlist;
                                GameState->CurrentPlaylistInfo.PlaylistReplicationKey++;
                                GameState->CurrentPlaylistInfo.MarkArrayDirty();
                                GameState->OnRep_CurrentPlaylistInfo();

                                GameMode->CurrentPlaylistName = Playlist->PlaylistName;
                                GameMode->WarmupRequiredPlayerCount = 1;

                                Misc::NextIdx = Playlist->DefaultFirstTeam;
                                Misc::MaxPlayersOnTeam = Playlist->MaxSquadSize;

                                bool bDBNO = Misc::MaxPlayersOnTeam > 1;

                                GameState->bDBNOEnabledForGameMode = bDBNO;
                                GameState->bDBNODeathEnabled = bDBNO;

                                GameMode->bAlwaysDBNO = bDBNO;
                                GameMode->bDBNOEnabled = bDBNO;

                                GameState->CurrentPlaylistInfo.BasePlaylist = Playlist;
                                GameState->CurrentPlaylistInfo.OverridePlaylist = Playlist;
                                GameState->CurrentPlaylistInfo.PlaylistReplicationKey++;
                                GameState->CurrentPlaylistId = Playlist->PlaylistId;
                                GameState->CurrentPlaylistInfo.MarkArrayDirty();

                                OGS_CHECK(GameMode->GameSession);
                                GameMode->GameSession->MaxPlayers = Playlist->MaxPlayers;
                                GameMode->GameSession->MaxSpectators = 0;
                                GameMode->GameSession->MaxPartySize = Playlist->MaxSquadSize;
                                GameMode->GameSession->MaxSplitscreensPerConnection = 2;
                                GameMode->GameSession->bRequiresPushToTalk = false;
                                GameMode->GameSession->SessionName = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("GameSession"));

                                auto TS = UGameplayStatics::GetTimeSeconds(UWorld::GetWorld());
                                auto DR = 90.f;

                                GameState->WarmupCountdownEndTime = TS + DR;
                                GameMode->WarmupCountdownDuration = DR;
                                GameState->WarmupCountdownStartTime = TS;
                                GameMode->WarmupEarlyCountdownDuration = DR;

                                GameState->CurrentPlaylistId = Playlist->PlaylistId;
                                GameState->OnRep_CurrentPlaylistId();

                                GameState->bGameModeWillSkipAircraft = Playlist->bSkipAircraft;
                                GameState->CachedSafeZoneStartUp = Playlist->SafeZoneStartUp;
                                GameState->AirCraftBehavior = Playlist->AirCraftBehavior;
                                GameState->OnRep_Aircraft();

                                GameState->WorldLevel = Playlist->LootLevel;
                                GameMode->AISettings = Playlist->AISettings;
                                GameMode->bSpawnAllStuff = true;
                                GameState->DefaultRebootMachineHotfix = 1;

                                if (Globals::bEventEnabled) {
                                        Log("Event is loaded!");

                                        TArray<AActor*> BuildingFoundations;
                                        UGameplayStatics::GetAllActorsOfClass(UWorld::GetWorld(), ABuildingFoundation::StaticClass(), &BuildingFoundations);

                                        for (AActor*& BuildingFoundation : BuildingFoundations) {
                                                ABuildingFoundation* Foundation = (ABuildingFoundation*)BuildingFoundation;
                                                if (!Foundation) continue;

                                                if (Foundation->GetName().find("Jerky") != std::string::npos ||
                                                        Foundation->GetName().find("LF_Athena_POI_19x19") != std::string::npos) {
                                                        ShowFoundation((ABuildingFoundation*)BuildingFoundation);
                                                }
                                        }

                                        BuildingFoundations.Free();
                                }
                                else {
                                        ABuildingFoundation* BuildingFound = StaticFindObject<ABuildingFoundation>("/Game/Athena/Apollo/Maps/Apollo_POI_Foundations.Apollo_POI_Foundations.PersistentLevel.LF_Athena_POI_19x19_2");
                                        ShowFoundation((ABuildingFoundation*)BuildingFound);
                                }

                                Log("Setup Playlist!");
                        }

                        if (!GameState->MapInfo) {
                                //Log("Map isnt fully loaded yet, ReadyToStartMatch return false!"); //this spams so why log
                                return false;
                        }

                        static bool InitialisedBots = false;

                        if (!InitialisedBots) {
                                if (Globals::bEventEnabled) {
                                        InitialisedBots = true;
                                }
                                else if (auto BotManager = (UFortServerBotManagerAthena*)UGameplayStatics::SpawnObject(UFortServerBotManagerAthena::StaticClass(), GameMode))
                                {
                                        InitialisedBots = true;

                                        GameMode->ServerBotManager = BotManager;
                                        GameMode->ServerBotManagerClass = UFortServerBotManagerAthena::StaticClass();
                                        BotManager->CachedGameState = GameState;
                                        BotManager->CachedGameMode = GameMode;

                                        BotMutator = SpawnActor<AFortAthenaMutator_Bots>({});
                                        OGS_CHECK(BotMutator);
                                        BotManager->CachedBotMutator = BotMutator;
                                        BotMutator->CachedGameMode = GameMode;
                                        BotMutator->CachedGameState = GameState;

                                        if (!GameMode->AIDirector) {
                                                Log("No AIDirector, Creating one automatically...");
                                                AAthenaAIDirector* Director = SpawnActor<AAthenaAIDirector>({});
                                                GameMode->AIDirector = Director;
                                                GameMode->AIDirector->Activate();
                                        }

                                        AFortAIGoalManager* GoalManager = SpawnActor<AFortAIGoalManager>({});
                                        GameMode->AIGoalManager = GoalManager;

                                        if (Globals::bBotsEnabled) {
                                                CIDs = GetAllObjectsOfClass<UAthenaCharacterItemDefinition>();
                                                Pickaxes = GetAllObjectsOfClass<UAthenaPickaxeItemDefinition>();
                                                Backpacks = GetAllObjectsOfClass<UAthenaBackpackItemDefinition>();
                                                Gliders = GetAllObjectsOfClass<UAthenaGliderItemDefinition>();
                                                Contrails = GetAllObjectsOfClass<UAthenaSkyDiveContrailItemDefinition>();
                                                Dances = GetAllObjectsOfClass<UAthenaDanceItemDefinition>();
                                        }

                                        Log("Initialised Bots!");
                                }
                                else
                                {
                                        Log("BotManager is nullptr!");
                                }
                        }

                        if (!ServerListening) {
                                ServerListening = true;

                                if (Globals::bEventEnabled) {
                                        Event::Starter = StaticLoadObject<UClass>("/CycloneJerky/Gameplay/BP_Jerky_Scripting.BP_Jerky_Scripting_C");
                                        Event::JerkyLoader = UObject::FindObject<UObject>("BP_Jerky_Loader_C JerkyLoaderLevel.JerkyLoaderLevel.PersistentLevel.BP_Jerky_Loader_2");
                                }

                                GameState->OnRep_CurrentPlaylistId();
                                GameState->OnRep_CurrentPlaylistInfo();

                                for (int i = 0; i < GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevels.Num(); i++)
                                {
                                        FVector Loc{};
                                        FRotator Rot{};
                                        bool bSuccess = false;
                                        ((ULevelStreamingDynamic*)ULevelStreamingDynamic::StaticClass()->DefaultObject)->LoadLevelInstanceBySoftObjectPtr(UWorld::GetWorld(), GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevels[i], Loc, Rot, &bSuccess, FString());
                                        FAdditionalLevelStreamed NewLevel{};
                                        NewLevel.LevelName = GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevels[i].ObjectID.AssetPathName;
                                        NewLevel.bIsServerOnly = false;
                                        GameState->AdditionalPlaylistLevelsStreamed.Add(NewLevel);
                                }

                                for (int i = 0; i < GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevelsServerOnly.Num(); i++)
                                {
                                        FVector Loc{};
                                        FRotator Rot{};
                                        bool bSuccess = false;
                                        ((ULevelStreamingDynamic*)ULevelStreamingDynamic::StaticClass()->DefaultObject)->LoadLevelInstanceBySoftObjectPtr(UWorld::GetWorld(), GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevelsServerOnly[i], Loc, Rot, &bSuccess, FString());
                                        FAdditionalLevelStreamed NewLevel{};
                                        NewLevel.LevelName = GameState->CurrentPlaylistInfo.BasePlaylist->AdditionalLevelsServerOnly[i].ObjectID.AssetPathName;
                                        NewLevel.bIsServerOnly = true;
                                        GameState->AdditionalPlaylistLevelsStreamed.Add(NewLevel);
                                }
                                GameState->OnRep_AdditionalPlaylistLevelsStreamed();
                                GameState->OnFinishedStreamingAdditionalPlaylistLevel();
                                GameMode->HandleAllPlaylistLevelsVisible();

                                // listen_server path: create the GameNetDriver and listen on
                                // 7777 (all offsets registry-gated; with listen_server=false
                                // or offsets missing this block is skipped + logged).
                                bool wantListen = Ogs::CfgRef().Has("listen_server");
                                auto createNetDriver = OgsFn::create_netdriver();
                                auto initListen = OgsFn::init_listen();
                                auto setWorld = OgsFn::set_world();
                                if (wantListen && createNetDriver && initListen && setWorld)
                                {
                                        FName NetDriverDef = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("GameNetDriver"));

                                        UNetDriver* NetDriver = createNetDriver(UEngine::GetEngine(), UWorld::GetWorld(), NetDriverDef);
                                        if (NetDriver)
                                        {
                                                NetDriver->NetDriverName = NetDriverDef;
                                                NetDriver->World = UWorld::GetWorld();

                                                FString Error;
                                                FURL url = FURL();
                                                url.Port = 7777;

                                                initListen(NetDriver, UWorld::GetWorld(), url, false, Error);
                                                setWorld(NetDriver, UWorld::GetWorld());

                                                UWorld::GetWorld()->NetDriver = NetDriver;

                                                for (size_t i = 0; i < UWorld::GetWorld()->LevelCollections.Num(); i++) {
                                                        UWorld::GetWorld()->LevelCollections[i].NetDriver = NetDriver;
                                                }

                                                setWorld(UWorld::GetWorld()->NetDriver, UWorld::GetWorld());
                                        }

                                        GameMode->bWorldIsReady = true;

                                        Log("Listening: 7777");
                                }
                                else
                                {
                                        if (wantListen)
                                                Ogs::Log("listen_server on but create_netdriver/init_listen/set_world offset missing — standalone mode");
                                        else
                                                Ogs::Log("listen_server off — standalone mode (no netdriver)");
                                        GameMode->bWorldIsReady = true;
                                }
                        }
                }

                if (GameState->PlayersLeft > 0)
                {
                        return true;
                }
                else
                {
                        auto TS = UGameplayStatics::GetTimeSeconds(UWorld::GetWorld());
                        auto DR = 90.f;

                        GameState->WarmupCountdownEndTime = TS + DR;
                        GameMode->WarmupCountdownDuration = DR;
                        GameState->WarmupCountdownStartTime = TS;
                        GameMode->WarmupEarlyCountdownDuration = DR;
                }

                return false;
        }

        inline APawn* SpawnDefaultPawnFor(AFortGameModeAthena* GameMode, AFortPlayerController* Player, AActor* StartingLoc)
        {
                OGS_CHECK(GameMode && Player && StartingLoc && Player->PlayerState);
                if (Player->Pawn)
                        return 0;

                AFortPlayerControllerAthena* PC = (AFortPlayerControllerAthena*)Player;
                AFortPlayerStateAthena* PlayerState = (AFortPlayerStateAthena*)PC->PlayerState;
                AFortGameStateAthena* GameState = (AFortGameStateAthena*)UWorld::GetWorld()->GameState;

                auto Transform = StartingLoc->GetTransform();
                auto Pawn = (AFortPlayerPawn*)GameMode->SpawnDefaultPawnAtTransform(Player, Transform);

                Abilities::InitAbilitiesForPlayer(PC);

                PC->XPComponent->bRegisteredWithQuestManager = true;
                PC->XPComponent->OnRep_bRegisteredWithQuestManager();

                PlayerState->SeasonLevelUIDisplay = PC->XPComponent->CurrentLevel;
                PlayerState->OnRep_SeasonLevelUIDisplay();

                PC->GetQuestManager(ESubGame::Athena)->InitializeQuestAbilities(PC->Pawn);

                UFortKismetLibrary::UpdatePlayerCustomCharacterPartsVisualization(PlayerState);
                PlayerState->OnRep_CharacterData();

                PlayerState->SquadId = PlayerState->TeamIndex - 3;
                PlayerState->OnRep_SquadId();

                FGameMemberInfo Member;
                Member.MostRecentArrayReplicationKey = -1;
                Member.ReplicationID = -1;
                Member.ReplicationKey = -1;
                Member.TeamIndex = PlayerState->TeamIndex;
                Member.SquadId = PlayerState->SquadId;
                Member.MemberUniqueId = PlayerState->UniqueId;

                GameState->GameMemberInfoArray.Members.Add(Member);
                GameState->GameMemberInfoArray.MarkItemDirty(Member);

                UAthenaPickaxeItemDefinition* PickDef;
                FFortAthenaLoadout& CosmecticLoadoutPC = PC->CosmeticLoadoutPC;
                PickDef = CosmecticLoadoutPC.Pickaxe != nullptr ? CosmecticLoadoutPC.Pickaxe : StaticLoadObject<UAthenaPickaxeItemDefinition>("/Game/Athena/Items/Weapons/WID_Harvest_Pickaxe_Athena_C_T01.WID_Harvest_Pickaxe_Athena_C_T01");
                if (PickDef) {
                        Inventory::GiveItem(PC, PickDef->WeaponDefinition, 1, 0);
                }
                else {
                        Log("Pick Doesent Exist!");
                }
                Pawn->CosmeticLoadout = PC->CosmeticLoadoutPC;
                Pawn->OnRep_CosmeticLoadout();

                for (size_t i = 0; i < GameMode->StartingItems.Num(); i++)
                {
                        if (GameMode->StartingItems[i].Count > 0)
                        {
                                Inventory::GiveItem(PC, GameMode->StartingItems[i].Item, GameMode->StartingItems[i].Count, 0);
                        }
                }

                GameState->OnRep_SafeZoneIndicator();
                GameState->OnRep_SafeZonePhase();

                PlayerState->ForceNetUpdate();
                Pawn->ForceNetUpdate();
                PC->ForceNetUpdate();

                auto applyCustomization = OgsFn::apply_character_customization();
                if (applyCustomization)
                        applyCustomization(PlayerState, Pawn);
                else
                        OGS_LOG("ApplyCharacterCustomization skipped (offset missing)");

                return Pawn;
        }

        static int64_t (*StartAircraftPhaseOG)(AFortGameModeAthena* GameMode, char a2) = nullptr;
        int64_t StartAircraftPhase(AFortGameModeAthena* GameMode, char a2)
        {
                for (auto FactionBot : FactionBots) // idek if im supposed to be setting these for the henchmens
                {
                        static auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                        static auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                        if (!FactionBot || !FactionBot->PC || !FactionBot->PC->Blackboard)
                                continue;
                        FactionBot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::BusLocked);
                        FactionBot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Aircraft);
                }

                for (auto PlayerBot : PlayerBotArray)
                {
                        if (!PlayerBot || !PlayerBot->PC || !PlayerBot->PC->Blackboard)
                                continue;

                        if (!Globals::LateGame) {
                                auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                                auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::BusLocked);
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Aircraft);
                        }
                        else {
                                auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                                auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::BusFlying);
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Aircraft);
                        }

                        static auto Name4 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_JumpOffBus_ExecutionStatus"));
                        static auto Name3 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_IsInBus"));
                        PlayerBot->PC->Blackboard->SetValueAsBool(Name3, true);
                        PlayerBot->PC->Blackboard->SetValueAsEnum(Name4, (uint8)EExecutionStatus::ExecutionAllowed);

                        if (!Globals::LateGame) {
                                PlayerBot->BotState = EBotState::PreBus; // Proper!
                        }
                        else {
                                PlayerBot->BotState = EBotState::Bus;
                        }
                }

                return StartAircraftPhaseOG(GameMode, a2);
        }

        static inline void (*OriginalOnAircraftExitedDropZone)(AFortGameModeAthena* GameMode, AFortAthenaAircraft* FortAthenaAircraft);
        void OnAircraftExitedDropZone(AFortGameModeAthena* GameMode, AFortAthenaAircraft* FortAthenaAircraft)
        {
                Log("OnAircraftExitedDropZone!");

                if (Globals::bBotsEnabled) { // kick all bots out of the bus
                        for (auto PlayerBot : PlayerBotArray) {
                                if (PlayerBot && PlayerBot->BotState == EBotState::Bus) {
                                        AFortGameStateAthena* GameState = (AFortGameStateAthena*)UWorld::GetWorld()->GameState;

                                        PlayerBot->Pawn->K2_TeleportTo(GameState->GetAircraft(0)->K2_GetActorLocation(), {});
                                        PlayerBot->Pawn->BeginSkydiving(true);
                                        PlayerBot->BotState = EBotState::Skydiving;
                                        Log("Kicked a bot!");
                                }
                        }
                }

                if (Globals::bEventEnabled)
                {
                        UFunction* StartEventFunc = Event::JerkyLoader && Event::JerkyLoader->Class
                                ? Event::JerkyLoader->Class->GetFunction("BP_Jerky_Loader_C", "startevent") : nullptr;

                        float ToStart = 0.f;
                        if (StartEventFunc)
                                Event::JerkyLoader->ProcessEvent(StartEventFunc, &ToStart);
                }

                return OriginalOnAircraftExitedDropZone(GameMode, FortAthenaAircraft);
        }

        int64_t (*OnAircraftEnteredDropZoneOG)(AFortGameModeAthena*);
        int64_t OnAircraftEnteredDropZone(AFortGameModeAthena* a1)
        {
                Log("OnAircraftEnteredDropZone Called!");

                for (auto FactionBot : FactionBots)
                {
                        if (!FactionBot || !FactionBot->PC || !FactionBot->PC->Blackboard)
                                continue;
                        static auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                        static auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                        FactionBot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::BusFlying);
                        FactionBot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Aircraft);
                }

                if (!Globals::LateGame) {
                        for (auto PlayerBot : PlayerBotArray)
                        {
                                if (!PlayerBot || !PlayerBot->PC || !PlayerBot->PC->Blackboard)
                                        continue;
                                static auto Name1 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhaseStep"));
                                static auto Name2 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_GamePhase"));
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name1, (uint8)EAthenaGamePhaseStep::BusFlying);
                                PlayerBot->PC->Blackboard->SetValueAsEnum(Name2, (uint8)EAthenaGamePhase::Aircraft);

                                static auto Name9 = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("AIEvaluator_Global_IsInBus"));

                                PlayerBot->PC->Blackboard->SetValueAsBool(Name9, true);

                                PlayerBot->BotState = EBotState::Bus;
                        }
                }

                return OnAircraftEnteredDropZoneOG(a1);
        }

        void (*StormOG)(AFortGameModeAthena* GameMode, int32 ZoneIndex);
        void Storm(AFortGameModeAthena* GameMode, int32 ZoneIndex)
        {
                OGS_CHECK(GameMode && GameMode->GameState);
                auto GameState = (AFortGameStateAthena*)GameMode->GameState;

                static bool First = true;

                if (Globals::LateGame && !First) //fixes crashing
                {
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeID_SurviveStormCircle.AccoladeID_SurviveStormCircle"));
                        }
                }
                else if (!Globals::LateGame)
                {
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeID_SurviveStormCircle.AccoladeID_SurviveStormCircle"));
                        }
                }

                static bool initstorm = false;

                if (Globals::LateGame && !initstorm)
                {
                        initstorm = true;
                        GameMode->SafeZonePhase = 4;
                        GameState->SafeZonePhase = 4;
                        GameState->SafeZonesStartTime = 0;
                        ZoneIndex = 4;
                        First = false;

                        if (Globals::bBotsEnabled) {
                                for (size_t i = 0; i < PlayerBotArray.size(); i++)
                                {
                                        PlayerBotArray[i]->BotState = EBotState::Bus;
                                }
                        }
                }

                if (Globals::LateGame && initstorm)
                {
                        int newPhase = GameState->SafeZonePhase + 1;

                        GameMode->SafeZonePhase = newPhase;
                        GameState->SafeZonePhase = newPhase;
                }

                if (Globals::bBotsEnabled && !Globals::LateGame) {
                        for (size_t i = 0; i < PlayerBotArray.size(); i++)
                        {
                                // Lets actually make sure the bot landed first before moving to zone
                                if (PlayerBotArray[i]->BotState < EBotState::Landed)
                                        continue;
                                Log("SafeZone!");
                                PlayerBotArray[i]->BotState = EBotState::MovingToSafeZone; // i dont know the best way to get the bots to move to zone tbh
                        }
                }

                return StormOG(GameMode, ZoneIndex);
        }

        void Hook() {
                OGS_HOOK_NATIVE_OFF(Ogs::Off().ready_to_start_match, ReadyToStartMatch, (void**)&ReadyToStartMatchOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().spawn_default_pawn_for, SpawnDefaultPawnFor, nullptr);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().start_aircraft_phase, StartAircraftPhase, (void**)&StartAircraftPhaseOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_aircraft_exited_drop_zone, OnAircraftExitedDropZone, (void**)&OriginalOnAircraftExitedDropZone);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().on_aircraft_entered_drop_zone, OnAircraftEnteredDropZone, (void**)&OnAircraftEnteredDropZoneOG);

                OGS_HOOK_NATIVE_OFF(Ogs::Off().storm, Storm, (void**)&StormOG);

                Log("Gamemode Hooked!");
        }
}

// =============================================================================
// Tick.h (orig) + the standalone 30 Hz tick thread (OgsStandaloneTick).
//
// On Windows the mod hooked UNetDriver::TickFlush (netdriver tick).  On
// Android with listen_server=false there is no netdriver, so the ported
// TickFlush BODY (early-start checks, bot drip-spawn, accolade ticking,
// Bosses::TickBots, PlayerBots::Tick) is driven by OgsStandaloneTick — a
// 30 Hz std::thread with a synthetic DeltaTime, running only while the
// match world is up.  GetMaxTickRate is skipped (logged) per spec.
// =============================================================================
namespace AccoladeTickingService {
        float NextPlaytimeXPDrop = 0.f;

        // Survival Accolades
        bool AccoladeId_026_Survival_Default_Bronze = false;
        bool AccoladeId_027_Survival_Default_Silver = false;
        bool AccoladeId_028_Survival_Default_Gold = false;
        //arena
        bool Placement25 = false;
        bool Placement5 = false;

        // first player to land accolade
        bool AccoladeId_018_First_Landing = false;

        void Tick(AFortGameModeAthena* GameMode, AFortGameStateAthena* GameState) {
                OGS_CHECK(GameMode && GameState);
                float CurrentTime = UGameplayStatics::GetDefaultObj()->GetTimeSeconds(UWorld::GetWorld());

                if (NextPlaytimeXPDrop == 0.f) {
                        NextPlaytimeXPDrop = CurrentTime + 120.f;
                }

                if (CurrentTime >= NextPlaytimeXPDrop) { // Broken or smth idk
                        NextPlaytimeXPDrop = CurrentTime + 120.f; // 2 Minute interval
                        Log("Playtime XP!");
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_054_Playtime.AccoladeId_054_Playtime"));
                        }
                }

                int32 AliveCount = GameMode->AlivePlayers.Num() + GameMode->AliveBots.Num();

                if (AliveCount == 50 && !AccoladeId_026_Survival_Default_Bronze)
                {
                        AccoladeId_026_Survival_Default_Bronze = true;
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_026_Survival_Default_Bronze.AccoladeId_026_Survival_Default_Bronze"));

                        }
                }


                if (AliveCount == 25 && !AccoladeId_027_Survival_Default_Silver)
                {
                        AccoladeId_027_Survival_Default_Silver = true;
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_027_Survival_Default_Silver.AccoladeId_027_Survival_Default_Silver"));

                        }
                }
                if (AliveCount == 10 && !AccoladeId_028_Survival_Default_Gold)
                {
                        AccoladeId_028_Survival_Default_Gold = true;
                        for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                        {
                                Quests::GiveAccolade(GameMode->AlivePlayers[i], StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_028_Survival_Default_Gold.AccoladeId_028_Survival_Default_Gold"));
                        }
                }

                if (Globals::Arena)
                {
                        if (AliveCount == 25 && !Placement25)
                        {
                                Placement25 = true;
                                for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                                {
                                        GameMode->AlivePlayers[i]->ClientReportTournamentPlacementPointsScored(25, 60);

                                }
                        }

                        else if (AliveCount == 5 && !Placement5)
                        {
                                Placement5 = true;
                                for (size_t i = 0; i < GameMode->AlivePlayers.Num(); i++)
                                {
                                        GameMode->AlivePlayers[i]->ClientReportTournamentPlacementPointsScored(5, 30);

                                }
                        }
                }

                for (AFortPlayerControllerAthena* Player : GameMode->AlivePlayers) {
                        if (!Player)
                                continue;
                        // KillStreak Accolades!
                        if (CurrentTime <= Player->LastKillTimeWindow && Player->KillsInKillTimeWindow != Player->LastRecordedKillsInKillTimeWindow) {
                                if (Player->KillsInKillTimeWindow == 2) {
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_002_DoubleKill.AccoladeId_002_DoubleKill"));
                                }
                                else if (Player->KillsInKillTimeWindow == 3) {
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_003_TrippleKill.AccoladeId_003_TrippleKill"));
                                }
                                else if (Player->KillsInKillTimeWindow == 4) {
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_004_QuadraKill.AccoladeId_004_QuadraKill"));
                                }
                                else if (Player->KillsInKillTimeWindow == 5) {
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_005_PentaKill.AccoladeId_005_PentaKill"));
                                }
                                else if (Player->KillsInKillTimeWindow == 6) {
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_006_MonsterKill.AccoladeId_006_MonsterKill"));
                                }

                                Player->LastRecordedKillsInKillTimeWindow = Player->KillsInKillTimeWindow;
                        }

                        // Maybe proper?
                        if (!AccoladeId_018_First_Landing && !Player->IsInAircraft() && Player->Pawn) {
                                FVector Vel = Player->Pawn->GetVelocity();
                                float Speed = Vel.Z;
                                if (Speed == 0.f) {
                                        AccoladeId_018_First_Landing = true;
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_018_First_Landing.AccoladeId_018_First_Landing"));

                                        // Eh give discover landmark and poi accolade cuz idk how to track it either (for ones idk how to track i will put them in the proper places when i figure it out)
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_032_DiscoverLandmark.AccoladeId_032_DiscoverLandmark"));
                                        Quests::GiveAccolade(Player, StaticLoadObject<UFortAccoladeItemDefinition>("/Game/Athena/Items/Accolades/AccoladeId_033_DiscoverPOI.AccoladeId_033_DiscoverPOI"));
                                }
                        }
                }
        }
}

namespace Tick {
        // The ported TickFlush BODY.  Driver/replication-driver work only happens
        // when a netdriver exists (listen_server path); the standalone thread
        // drives the same body with a synthetic DeltaTime.
        void TickFlushBody(UNetDriver* Driver, float DeltaTime)
        {
                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->AuthorityGameMode && World->GameState);
                AFortGameModeAthena* GameMode = (AFortGameModeAthena*)World->AuthorityGameMode;
                AFortGameStateAthena* GameState = (AFortGameStateAthena*)World->GameState;

                if (Driver)
                {
                        if (!Driver->ReplicationDriver)
                        {
                                Log("ReplicationDriver Doesent Exist!");
                        }

                        auto serverReplicateActors = OgsFn::server_replicate_actors();
                        if (serverReplicateActors && Driver->ReplicationDriver)
                                serverReplicateActors(Driver->ReplicationDriver);
                        else
                                OGS_LOG("ServerReplicateActors skipped (offset missing)");
                }

                if (GameState->GamePhase == EAthenaGamePhase::Warmup
                        && (GameMode->AlivePlayers.Num() + GameMode->AliveBots.Num()) >= Globals::MinPlayersForEarlyStart
                        && GameState->WarmupCountdownEndTime > UGameplayStatics::GetTimeSeconds(UWorld::GetWorld()) + 10.f) {

                        auto TS = UGameplayStatics::GetTimeSeconds(UWorld::GetWorld());
                        auto DR = 10.f;

                        GameState->WarmupCountdownEndTime = TS + DR;
                        GameMode->WarmupCountdownDuration = DR;
                        GameState->WarmupCountdownStartTime = TS;
                        GameMode->WarmupEarlyCountdownDuration = DR;
                }

                if (Globals::bBotsEnabled && !Globals::bEventEnabled) {
                        static bool InitialisedPlayerStarts = false;
                        if (!InitialisedPlayerStarts)
                        {
                                UGameplayStatics::GetDefaultObj()->GetAllActorsOfClass(UWorld::GetWorld(), AFortPlayerStartWarmup::StaticClass(), &PlayerStarts);
                                InitialisedPlayerStarts = true;
                        }

                        if (GameState->GamePhase == EAthenaGamePhase::Warmup
                                && GameMode->AlivePlayers.Num() > 0
                                && GameMode->GameSession
                                && (GameMode->AlivePlayers.Num() + GameMode->AliveBots.Num()) < GameMode->GameSession->MaxPlayers
                                && GameMode->AliveBots.Num() < Globals::MaxBotsToSpawn
                                && GameState->WarmupCountdownEndTime > UGameplayStatics::GetTimeSeconds(UWorld::GetWorld()))
                        {
                                if (UKismetMathLibrary::GetDefaultObj()->RandomBoolWithWeight(0.05f))
                                {
                                        AActor* SpawnLocator = PlayerStarts.Num() > 0
                                                ? PlayerStarts[UKismetMathLibrary::GetDefaultObj()->RandomIntegerInRange(0, PlayerStarts.Num() - 1)]
                                                : nullptr;

                                        if (SpawnLocator)
                                        {
                                                PlayerBots::SpawnPlayerBots(SpawnLocator);
                                        }
                                }
                        }
                }

                if (GameState->WarmupCountdownEndTime - UGameplayStatics::GetTimeSeconds(UWorld::GetWorld()) <= 0 && GameState->GamePhase == EAthenaGamePhase::Warmup)
                {
                        GameMode::StartAircraftPhase(GameMode, 0);
                }

                if (GameState->GamePhase > EAthenaGamePhase::Warmup && !Globals::bCreativeEnabled) {
                        AccoladeTickingService::Tick(GameMode, GameState);
                }

                if (Globals::bBossesEnabled && !Globals::bEventEnabled && GameState->GamePhase > EAthenaGamePhase::Warmup)
                {
                        Bosses::TickBots(DeltaTime);
                }

                if (Globals::bBotsEnabled && !Globals::bEventEnabled) {
                        PlayerBots::Tick();
                }
        }

        inline void (*TickFlushOG)(UNetDriver*, float);
        void TickFlush(UNetDriver* Driver, float DeltaTime)
        {
                if (!Driver)
                        return;

                TickFlushBody(Driver, DeltaTime);
                return TickFlushOG(Driver, DeltaTime);
        }

        inline float GetMaxTickRate()
        {
                return 30.f;
        }

        void Hook() {
                OGS_HOOK_NATIVE_OFF(Ogs::Off().tick_flush, TickFlush, (void**)&TickFlushOG);

                // GetMaxTickRate: SKIPPED on Android (spec) — the standalone tick
                // thread already runs at 30 Hz.
                Ogs::Log("Tick: GetMaxTickRate hook skipped (standalone 30Hz thread drives ticking)");

                Log("Tick Hooked!");
        }
}

// --- OgsStandaloneTick: the 30 Hz driver when there is no netdriver ---------
namespace OgsStandaloneTick
{
        std::atomic<bool> g_MatchWorldUp{ false };
        std::atomic<bool> g_Running{ false };

        void ThreadMain()
        {
                const auto Frame = std::chrono::milliseconds(33); // ~30 Hz
                const float SyntheticDeltaTime = 1.0f / 30.0f;
                auto Next = std::chrono::steady_clock::now();

                Ogs::Log("OgsStandaloneTick: thread running (waiting for match world)");

                while (g_Running.load())
                {
                        std::this_thread::sleep_until(Next);
                        Next += Frame;

                        if (!g_MatchWorldUp.load())
                                continue;

                        UWorld* World = UWorld::GetWorld();
                        if (!World || !World->AuthorityGameMode || !World->GameState)
                                continue;

                        Tick::TickFlushBody(nullptr, SyntheticDeltaTime);
                }

                Ogs::Log("OgsStandaloneTick: thread exiting");
        }

        void MarkMatchWorldUp()
        {
                g_MatchWorldUp.store(true);
        }

        void Start()
        {
                if (g_Running.exchange(true))
                        return;
                std::thread(ThreadMain).detach();
        }

        void Stop()
        {
                g_Running.store(false);
        }
}

// =============================================================================
// Creative.h (orig)
// =============================================================================
namespace Creative {
        void ServerTeleportToPlaygroundLobbyIsland(AFortPlayerController* PC)
        {
                auto GameState = OgsSDK::GameState();
                auto Math = OgsSDK::Math();
                auto GameMode = OgsSDK::Gamemode();
                auto Statics = OgsSDK::Statics();
                OGS_CHECK(GameState && GameMode && Statics);

                if (PC)
                {
                        Log("test");
                        auto State = (AFortPlayerStateAthena*)PC->PlayerState;
                        OGS_CHECK(State);
                        if (GameState->IsTeleportToCreativeHubAllowed(State))
                        {
                                auto PlayerStartTEST = GameMode->FindPlayerStart(PC, OGS_TEXT(""));

                                AActor* StartSpot = nullptr;
                                if (PlayerStartTEST)
                                {
                                        StartSpot = PlayerStartTEST;
                                }
                                else
                                {
                                        TArray<AActor*> StartSpotsC;
                                        Statics->GetAllActorsOfClass(UWorld::GetWorld(), AFortPlayerStartCreative::StaticClass(), &StartSpotsC);

                                        StartSpot = StartSpotsC.Num() > 0 ? StartSpotsC[rand() % StartSpotsC.Num()] : nullptr;

                                        StartSpotsC.Free();
                                }

                                if (StartSpot && PC->Pawn)
                                {
                                        PC->Pawn->K2_TeleportTo(StartSpot->K2_GetActorLocation(), StartSpot->K2_GetActorRotation());
                                }
                        }
                }
                else {
                        Log("PlayerController Doesent Exist!");
                }
        }

        void Hook() {
                //HookVTable(AFortPlayerControllerAthena::GetDefaultObj(), 0x4E6, ServerTeleportToPlaygroundLobbyIsland, nullptr);
                //MH_CreateHook((LPVOID)(ImageBase + 0x2CB1710), ServerTeleportToPlaygroundLobbyIsland, nullptr);

                Log("Hooked Creative!");
        }
}

// =============================================================================
// PE.h (orig) — ProcessEvent hook (registry-gated).
// =============================================================================
namespace PE {
        static void* (*ProcessEventOG)(UObject*, UFunction*, void*);
        void* ProcessEvent(UObject* Obj, UFunction* Function, void* Params)
        {
                if (Function) {

                }

                return ProcessEventOG(Obj, Function, Params);
        }

        void Hook() {
                OGS_HOOK_NATIVE_OFF(Ogs::Off().process_event, ProcessEvent, (void**)&ProcessEventOG);

                Log("PE Hooked!");
        }
}

// =============================================================================
// Vehicles.h (orig) — feature-gated by "vehicles".
// =============================================================================
namespace Vehicles {

        void ServerMove(AFortPhysicsPawn* PhysicsPawn, FReplicatedPhysicsPawnState InState) {
                OGS_CHECK(PhysicsPawn && PhysicsPawn->RootComponent);

                UPrimitiveComponent* RootComponent = static_cast<UPrimitiveComponent*>(PhysicsPawn->RootComponent);

                InState.Rotation.X -= 2.5f;
                InState.Rotation.Y /= 0.3f;
                InState.Rotation.Z += 2.0f;
                InState.Rotation.W /= -1.2f;

                FTransform Transform;
                Transform.Translation = InState.Translation;
                Transform.Rotation = InState.Rotation;
                Transform.Scale3D = FVector(1.0f, 1.0f, 1.0f);

                RootComponent->K2_SetWorldTransform(Transform, false, nullptr, true);
                RootComponent->bComponentToWorldUpdated = true;

                RootComponent->SetPhysicsLinearVelocity(InState.LinearVelocity, false, FName());
                RootComponent->SetPhysicsAngularVelocity(InState.AngularVelocity, false, FName());
        }

        void SpawnVehicles()
        {
                if (Globals::bEventEnabled)
                        return;

                UWorld* World = UWorld::GetWorld();
                OGS_CHECK(World && World->AuthorityGameMode && World->GameState);
                AFortGameModeAthena* GameMode = (AFortGameModeAthena*)World->AuthorityGameMode;
                AFortGameStateAthena* GameState = (AFortGameStateAthena*)World->GameState;
                static bool bHasSpawnedSharkChopper = false;

                TArray<AActor*> Spawners;
                UGameplayStatics::GetAllActorsOfClass(UWorld::GetWorld(), AFortAthenaVehicleSpawner::StaticClass(), &Spawners);

                for (AActor* ActorSp : Spawners) {
                        AFortAthenaVehicleSpawner* Spawner = Cast<AFortAthenaVehicleSpawner>(ActorSp);
                        if (!Spawner) continue;

                        std::string Name = Spawner->GetName();


                        if (Name.rfind("Apollo_Hoagie_Spawner", 0) != 0 && Name.rfind("Athena_Meatball_L_Spawner", 0) != 0) continue;

                        if (!Spawner->GetVehicleClass())
                                continue;

                        AActor* Vehicle = SpawnActorClass<AActor>(Spawner->K2_GetActorLocation(), Spawner->K2_GetActorRotation(), Spawner->GetVehicleClass());

                        if (!bHasSpawnedSharkChopper && Name.rfind("Apollo_Hoagie_Spawner", 0) == 0) {
                                bHasSpawnedSharkChopper = true;

                                AActor* Shark_Chopper = SpawnActorClass<AActor>(FVector(113665, -91120, -3065), {}, Spawner->GetVehicleClass());

                                // HookVTable(Shark_Chopper->VTable, 0xEE, ServerMove, nullptr)
                                // → ByExec on the reflected ServerMove RPC of the vehicle.
                                if (Shark_Chopper)
                                        OgsHook::ByExec(Shark_Chopper, "FortPhysicsPawn", "ServerMove", (void*)ServerMove, nullptr);
                        }

                        // HookVTable(Vehicle->VTable, 0xEE, ServerMove, nullptr)
                        if (Vehicle)
                                OgsHook::ByExec(Vehicle, "FortPhysicsPawn", "ServerMove", (void*)ServerMove, nullptr);
                }

                Spawners.Free();
        }
}

// =============================================================================
// PhantomBoothSpawner.h (orig) — ported (registry-gated faction-tag write).
// =============================================================================
namespace PhantomBoothSpawner {
        std::vector<FVector> PhantomBoothLocations = {
                // Agency Booths
                { 3481.255127, 2394.962891, -1917.627686 },
                { 7685, 5974, -1535 },
                { -1168.074707, 7682.558594, -2302.662598 },

                // The Shark Booths
                { 89218.656250, -75777.187500, -3829 },
                { 90732.578125, -96676.492188, -3846 },
                { 98479.625, -66667.960938, -3856 },
                { 106736, -86832, -3452 },
                { 108865, -85892, -2680 },

                // The Yacht Booths
                { 112502, 114296, -3832 },
                { 109492, 114338, -3063 },
                { 105059, 108490.992188, -3840 },
                { 116696, 114325, -3448 },

                // The Rig Booths
                { -70218.710938, -78991.226563, -3843.156250 },
                { -77443.039063, -88644.937500, -3064 },
                { -82059.859375, -88710.59375, -3448 },
                { -84055, -71100, -3858 },

                // The Grotto Booths
                { -14165.263672, 111691.804688, -3840 },
                { -21847, 112260, 2680 },

                // Pleasant Park Booths
                { 55664.054688, -19288.376953, -2296 },
        };

        std::vector<FRotator> PhantomBoothRotations = {
                { 0, 89.999954, 0 },
                { 0, 90.000420, 0 },
                { 0, 0.000085, 0 },

                { 0, 114.999809, 0 },
                { 0, 155.000092, 0 },
                { 0, 84.999954, 0 },
                { 0, -90.000114, 0 },
                { 0, 0, 0 },

                { 0, -44.999969, 0 },
                { 0, -90.000023, 0 },
                { 0, -90.000023, 0 },
                { 0, -90.000023, 0 },

                { 0, 89.907799, 0 },
                { 0, -19.034946, 0 },
                { 0, 39.999893, 0 },
                { 0, -35, 0 },

                { 0, 179.999969, 0 },
                { 0, 180, 0 },

                { 0, -90.000053, 0 },
        };

        void SpawnBooths() {
                if (PhantomBoothLocations.size() != PhantomBoothRotations.size()) {
                        Log("Sizes for spawndata dont match!");
                        return;
                }

                static auto PhantomBooth = StaticLoadObject<UClass>("/Game/Athena/Items/EnvironmentalItems/HidingProps/Props/B_HidingProp_PhantomBooth.B_HidingProp_PhantomBooth_C");
                for (int i = 0; i < (int)PhantomBoothLocations.size(); i++) {
                        FVector Location = PhantomBoothLocations[i];
                        FRotator Rotation = PhantomBoothRotations[i];

                        if (PhantomBooth) {
                                AActor* SpawnedBooth = SpawnActor<AActor>(Location, Rotation, nullptr, PhantomBooth);
                                FGameplayTag FactionTag{};
                                // Remember to shift the index for this if addded or removed from the std::vector
                                if (i >= 12) {
                                        FactionTag.TagName = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Athena.Faction.Alter"));
                                }
                                else {
                                        FactionTag.TagName = UKismetStringLibrary::Conv_StringToName(OGS_TEXT("Athena.Faction.Ego"));
                                }

                                // orig: *(FGameplayTag*)(SpawnedBooth + 0xB0) — now registry-gated.
                                const auto& tagOff = Ogs::Off().struct_faction_tag;
                                if (tagOff.Found && SpawnedBooth) {
                                        *(FGameplayTag*)(Ogs::Resolve(tagOff.Rva) + (uintptr_t)SpawnedBooth) = FactionTag;
                                }
                                else if (SpawnedBooth) {
                                        Log("faction tag write skipped (offset missing)");
                                }

                                if (SpawnedBooth && SpawnedBooth->Class) {
                                        UFunction* FactionSelectedFn = SpawnedBooth->Class->GetFunction("B_HidingProp_PhantomBooth_C", "BndEvt__Athena_FactionSelection_AlterEgo_K2Node_ComponentBoundEvent_0_OnFactionSelected__DelegateSignature");
                                        if (FactionSelectedFn) {
                                                struct {
                                                        FGameplayTag FactionTagOut;
                                                } Params;
                                                Params.FactionTagOut = FactionTag;
                                                SpawnedBooth->ProcessEvent(FactionSelectedFn, &Params);
                                        }
                                }
                        }
                        else {
                                Log("PhantomBooth does not exist!");
                        }
                }

                if (PhantomBooth) {
                        Log("Spawned: " + std::to_string(PhantomBoothLocations.size()) + " PhantomBooths");
                }
        }
}

// =============================================================================
// ogs_main — orig/dllmain.cpp Main() (the boot orchestrator), ported.
//
// Phase markers (greppable from logcat — the CI driver watches them):
//   "OGS: phase=login_wait"  waiting for the Frontend world, InProgress
//   "OGS: phase=hooks"        installing every namespace hook
//   "OGS: phase=flip"         GIsClient=false / GIsServer=true
//   "OGS: phase=loadworld"    console `open Apollo_Terrain` (+ the optional
//                             "world_open_delay" hold for lobby screenshots)
//   "OGS: match world up"     the BR map is live — the in-match proof marker
// =============================================================================

namespace {

bool WaitForEngine(int timeout_s)
{
        for (int i = 0; i < timeout_s; ++i) {
                if (UEngine::GetEngine())
                        return true;
                Ogs::SleepMs(1000);
        }
        return false;
}

bool WorldIsFrontendInProgress()
{
        UWorld* World = UWorld::GetWorld();
        if (!World)
                return false;
        if (World->GetName() != "Frontend")
                return false;
        auto GM = OgsSDK::Gamemode();
        if (!GM)
                return false;
        return GM->GetMatchState().ToString() == "InProgress";
}

bool WaitForLogin(int timeout_s)
{
        for (int i = 0; i < timeout_s; ++i) {
                if (WorldIsFrontendInProgress())
                        return true;
                Ogs::SleepMs(1000);
        }
        return false;
}

/// orig: *(bool*)(ImageBase + 0x804B659) = false; //GIsClient
///       *(bool*)(ImageBase + 0x804B65A) = true;  //GIsServer
/// Registry-gated; GIsServer falls back to GIsClient+1 (the PC layout) but
/// only after validating the byte actually reads 0 like a client build.
bool FlipGIS()
{
        const auto& reg = Ogs::Off();
        uintptr_t base = Ogs::Base();
        if (!base || !reg.gis_client.Found) {
                Log("gis_flip off (gis_client offset missing)");
                return false;
        }
        volatile uint8_t* isClient = (volatile uint8_t*)Ogs::Resolve(reg.gis_client.Rva);
        uintptr_t serverRva = reg.gis_server.Found ? reg.gis_server.Rva : reg.gis_client.Rva + 1;
        volatile uint8_t* isServer = (volatile uint8_t*)(base + serverRva);
        if (!reg.gis_server.Found) {
                if (*isServer != 0) {
                        Log("gis_server heuristic rejected (byte at +1 reads %d, expected 0) — flip skipped", (int)*isServer);
                        return false;
                }
                Log("gis_server: gis_client+1 heuristic validated");
        }
        if (!Ogs::MProtectRW((void*)isClient, 1) || !Ogs::MProtectRW((void*)isServer, 1)) {
                Log("gis_flip: mprotect failed");
                return false;
        }
        *isClient = 0;
        *isServer = 1;
        Log("GIsClient=false GIsServer=true (client @0x%llx, server @0x%llx)",
            (unsigned long long)(base + reg.gis_client.Rva),
            (unsigned long long)(base + serverRva));
        return true;
}

void LoadWorld()
{
        Log("Loading World!");
        const char16_t* map = Globals::bCreativeEnabled
            ? OGS_TEXT("open Creative_NoApollo_Terrain")
            : OGS_TEXT("open Apollo_Terrain");
        auto SL = OgsSDK::SystemLib();
        if (SL)
                SL->ExecuteConsoleCommand(UWorld::GetWorld(), map, nullptr);
        else
                Log("SystemLib unavailable — console open skipped");

        // orig: UWorld::GetWorld()->OwningGameInstance->LocalPlayers.Remove(0);
        UWorld* World = UWorld::GetWorld();
        if (World && World->OwningGameInstance && World->OwningGameInstance->LocalPlayers.Num() > 0)
                World->OwningGameInstance->LocalPlayers.Remove(0);
}

} // namespace

extern "C" void ogs_main(const char* config_json)
{
        Ogs::Init(config_json);
        OGS_LOG("OGS: boot — libOwenOGS 12.41-android (safe-Rust host + C++ mod)");
        Globals::InitFromCfg();

        if (!WaitForEngine(180)) {
                OGS_LOG("OGS: engine never appeared — mod idle");
                return;
        }
        OGS_LOG("OGS: phase=login_wait");
        if (!WaitForLogin(900)) {
                OGS_LOG("OGS: login wait timed out (no Frontend/InProgress) — mod idle");
                return;
        }
        Ogs::SleepMs(1000);

        OGS_LOG("OGS: phase=hooks");
        GameMode::Hook();
        Looting::Hook();
        Inventory::Hook();
        Abilities::Hook();
        Building::Hook();
        Quests::Hook();
        Misc::Hook();
        Net::Hook();
        Tick::Hook();
        Bots::Hook();
        Creative::Hook();
        PE::Hook();
        OGS_LOG("OGS: hooks installed");

        if (Ogs::CfgRef().Has("gis_flip")) {
                OGS_LOG("OGS: phase=flip");
                FlipGIS();
        }
        else {
                Log("gis_flip disabled by config — staying a client");
        }

        // orig: wait for a world with a local player
        for (int i = 0; i < 300; ++i) {
                UWorld* World = UWorld::GetWorld();
                if (World && World->OwningGameInstance && World->OwningGameInstance->LocalPlayers.Num() > 0)
                        break;
                Ogs::SleepMs(1000);
        }

        // Optional hold BEFORE the map opens: the E2E driver takes its lobby
        // screenshot during this window (ogsen.json "numbers":
        // {"world_open_delay": <seconds>}).
        long long delay = Ogs::CfgRef().Num("world_open_delay");
        if (delay > 0) {
                Log("world_open_delay=%llds (lobby window)", delay);
                for (long long t = delay; t > 0; t -= 5) {
                        Ogs::SleepMs((int)(t >= 5 ? 5000 : t * 1000));
                        long long left = delay - ((delay - t) + 5);
                        (void)left;
                }
        }

        OGS_LOG("OGS: phase=loadworld");
        LoadWorld();

        // wait for the travel to land on the BR map, then emit the proof marker
        for (int i = 0; i < 600; ++i) {
                UWorld* World = UWorld::GetWorld();
                if (World) {
                        std::string n = World->GetName();
                        if (n.find("Apollo") != std::string::npos || n.find("Terrain") != std::string::npos) {
                                OGS_LOG("OGS: match world up (%s)", n.c_str());
                                return;
                        }
                }
                Ogs::SleepMs(1000);
        }
        OGS_LOG("OGS: world never became Apollo_Terrain (loadworld issued, travel pending)");
}
