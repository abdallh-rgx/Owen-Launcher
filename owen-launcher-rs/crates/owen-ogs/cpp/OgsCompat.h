// =============================================================================
// OgsCompat.h — OGS 12.41 → Android port compatibility layer.
//
// Everything the ported mod needs that used to come from <windows.h> /
// MinHook / the MSVC runtime lives here:
//
//   * Ogs::Init / Ogs::Cfg / Ogs::CfgHas — the tiny flat-JSON config reader
//     for `ogsen.json` ({"features":{...},"vt_indices":{...},"game":"..."}).
//   * Ogs::Log / OGS_LOG — __android_log_print(ANDROID_LOG_INFO,"OwenOGS")
//     on Android, fprintf(stderr) on the host, plus a best-effort append to
//     ogsen_log.txt in the game files dir (mirrors the original OGS_log.txt).
//   * Ogs::Base / Ogs::Resolve — dl_iterate_phdr lookup of the libUE4
//     module base (cached).  Replaces InSDKUtils::GetImageBase()/ImageBase.
//   * Ogs::SleepMs / Ogs::MProtectRW / OGS_RETURN_ADDRESS.
//   * Ogs::Off — the offset registry built from OgsOffsetsGen.h (generated
//     by build.rs from offsets/ogs-1241-android.json; the checked-in
//     cpp/host_stubs/OgsOffsetsGen.h provides all-found-false defaults for
//     host syntax checks and is overridden in CI because $OUT_DIR comes
//     first on the include path).
//   * Ogs::Text / OGS_TEXT — UTF-8 → SDK::FString, generic over the SDK
//     container element type (wchar_t on the Windows dump, char16_t on the
//     Android dump).  Replaces every L"..." literal in the original.
//
// HOST SYNTAX CHECK: compile with -DHOST_SYNTAX_CHECK to get inert stubs for
// android/log.h, dl_iterate_phdr and mprotect so g++ -fsyntax-only works.
// =============================================================================

#pragma once

#include <cstdint>
#include <cstddef>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <string>
#include <vector>
#include <map>
#include <atomic>

// --- Android vs host ---------------------------------------------------------
#if !defined(HOST_SYNTAX_CHECK) && defined(__ANDROID__)
  #include <android/log.h>
  #include <dlfcn.h>
  #include <link.h>      // dl_iterate_phdr
  #include <sys/mman.h>  // mprotect
  #include <unistd.h>    // usleep
  #define OGS_ON_ANDROID 1
#else
  #define OGS_ON_ANDROID 0
  #include <thread>
  #include <chrono>
#endif

// --- the game SDK ------------------------------------------------------------
// Include dirs are "-I<parent> -I<root> -I$OUT_DIR [-I cpp/host_stubs]" so
// both "SDK/SDK.hpp" (parent-relative) and "SDK.hpp" (root-relative) resolve
// to the SAME dump the CI build compiles.
#include "SDK/SDK.hpp"

// =============================================================================
// Offset registry — OgsOffsetsGen.h
// -----------------------------------------------------------------------------
// build.rs generates $OUT_DIR/OgsOffsetsGen.h from
// offsets/ogs-1241-android.json ("entries": { key: {found, rva} }) with one
// pair of macros per scanner key:
//     #define OGSOFF_<KEY>_FOUND 1/0
//     #define OGSOFF_<KEY>_RVA  0x...
// The host fallback (cpp/host_stubs/OgsOffsetsGen.h) is guarded with #ifndef
// so the generated header always wins in CI (OUT_DIR precedes host_stubs on
// the include path).
//
// MOD-ONLY KEYS (documented found:false — do NOT add these to the scanner
// JSON; they are the raw function-pointer/table offsets the original mod
// baked in from the Windows 12.41 binary; on Android each is an RVA into
// libUE4.so that the scanner has NOT resolved yet):
//   create_netdriver            (win 0x4573990)  UEngine::CreateNetDriver
//   init_listen                 (win 0x0D44C40)  UNetDriver::InitListen
//   set_world                   (win 0x42C2B20)  UNetDriver::SetWorld
//   server_replicate_actors     (win 0x1023F60)  FReplicationDriver::
//                                                 ServerReplicateActors
//   give_ability                (win 0x6B19E0)   UAbilitySystemComponent::
//                                                 GiveAbility
//   ability_spec_constructor    (win 0x6D6DD0)   FGameplayAbilitySpec ctor
//   internal_try_activate_ability (win 0x6B33F0) UAbilitySystemComponent::
//                                                 InternalTryActivateAbility
//   give_ability_and_activate_once (win 0x6B1B00)
//   pick_supply_drop_location   (win 0x18848F0)  AFortAthenaMapInfo::
//                                                 PickSupplyDropLocation
//   replace_building_actor      (win 0x1B951B0)  ABuildingSMActor replace
//   cant_build                  (win 0x1E57790)  building placement check
//   apply_character_customization (win 0x2363FF0)
//   bot_manager_setup           (win 0x19D93F0)  bot manager setup entry
//   register_component_with_world (win 0x3FF94E0)
//   remove_from_alive_players   (win 0x18ECBB0)
//   add_to_alive_players        (win 0x18C35B0)
//   static_load_object          (win 0x2E1D7A0)  StaticLoadObject
//       (static_find_object win 0x2E1C4B0 is REPLACED by the SDK's own
//        UObject::FindObject<T>() helper and needs no offset at all)
//   fframe_step_a               (win 0x2E1DD00)  FFrame::StepCompiledIn
//   fframe_step_b               (win 0x2E1DD30)  FFrame::StepCompiledInExplicitProp
//   get_interface               (win 0x2E0D910)  cast-interface resolver
//   gadget_add                  (win 0x1CFC810)  gadget equip path
//   gadget_remove               (win 0x1CFC8D0)  gadget unequip path
//   tick_flush                  (win 0x42C3ED0)  UNetDriver::TickFlush
//   get_max_tick_rate           (win 0x4576310)  skipped on Android (logged)
//   on_reload                   (win 0x260C490)
//   k2_remove_item_from_player  (win 0x1E69600)
//   spawn_loot                  (win 0x1B46D00)
//   on_damage_server            (win 0x2683F80)
//   client_on_pawn_died         (win 0x29B5C80)
//   server_attempt_interact     (win 0x1C8EA00)
//   apply_cost                  (win 0x1583360)
//   on_capsule_begin_overlap    (win 0x22A08C0)
//   server_play_squad_quick_chat (win 0x197F6D0)
//   rebooting_delegate          (win 0x1B6C232)
//   moving_emote_stopped        (win 0x6853B0)
//   on_exploded                 (win 0x3EA2740)
//   ready_to_start_match        (win 0x4640A30)
//   spawn_default_pawn_for      (win 0x18F6250)
//   start_aircraft_phase        (win 0x18F9BB0)
//   on_aircraft_exited_drop_zone (win 0x18E07D0)
//   on_aircraft_entered_drop_zone (win 0x18E0730)
//   storm                       (win 0x18FD350)
//   spawn_bot                   (win 0x19E9B10)
//   set_up_inventory_bot        (win 0x1637F50)
//   on_possessed_pawn_died      (win 0x163C760)
//   on_perception_sensed        (win 0x163C300)
//   on_actor_bump               (win 0x49932C0)
//   process_event               (win 0x2E13BF0)  UObject::ProcessEvent
//   misc_collect_garbage        (win 0x2D95E00)
//   misc_kick_player            (win 0x4155600)
//   misc_change_gamesession_id  (win 0x1E23840)
//   misc_dispatch_request       (win 0x108D740)
//   misc_can_create_context     (win 0x2DBCBA0)
//   misc_nullfunc_0             (win 0x3CA10C0)
//   misc_nullfunc_1             (win 0x3262100)
//   misc_nullfunc_2             (win 0x2D95DC0)
//   k2_destroy_actor            (win 0x7F0220)
//   pick_team                   (win 0x18E6B20)
//   spawnbot_self_call_rva      (win 0x1A4153F)  _ReturnAddress() guard in
//                                                 Bots::SpawnBot (NOT an
//                                                 address — an RVA compared
//                                                 against the caller)
//   struct_faction_tag          (win 0xB0)  FGameplayTag faction write on
//                                 actors whose class is NOT in the dump
//                                 (B_HidingProp_PhantomBooth_C et al.);
//                                 NOT an RVA — a struct offset.
// =============================================================================
#include "OgsOffsetsGen.h"

// ---- registry fallbacks for every mod-only key (host + future-proof) --------
#ifndef OGSOFF_create_netdriver_RVA
  #define OGSOFF_create_netdriver_RVA 0
  #define OGSOFF_create_netdriver_FOUND 0
#endif
#ifndef OGSOFF_init_listen_RVA
  #define OGSOFF_init_listen_RVA 0
  #define OGSOFF_init_listen_FOUND 0
#endif
#ifndef OGSOFF_set_world_RVA
  #define OGSOFF_set_world_RVA 0
  #define OGSOFF_set_world_FOUND 0
#endif
#ifndef OGSOFF_server_replicate_actors_RVA
  #define OGSOFF_server_replicate_actors_RVA 0
  #define OGSOFF_server_replicate_actors_FOUND 0
#endif
#ifndef OGSOFF_give_ability_RVA
  #define OGSOFF_give_ability_RVA 0
  #define OGSOFF_give_ability_FOUND 0
#endif
#ifndef OGSOFF_ability_spec_constructor_RVA
  #define OGSOFF_ability_spec_constructor_RVA 0
  #define OGSOFF_ability_spec_constructor_FOUND 0
#endif
#ifndef OGSOFF_internal_try_activate_ability_RVA
  #define OGSOFF_internal_try_activate_ability_RVA 0
  #define OGSOFF_internal_try_activate_ability_FOUND 0
#endif
#ifndef OGSOFF_give_ability_and_activate_once_RVA
  #define OGSOFF_give_ability_and_activate_once_RVA 0
  #define OGSOFF_give_ability_and_activate_once_FOUND 0
#endif
#ifndef OGSOFF_pick_supply_drop_location_RVA
  #define OGSOFF_pick_supply_drop_location_RVA 0
  #define OGSOFF_pick_supply_drop_location_FOUND 0
#endif
#ifndef OGSOFF_replace_building_actor_RVA
  #define OGSOFF_replace_building_actor_RVA 0
  #define OGSOFF_replace_building_actor_FOUND 0
#endif
#ifndef OGSOFF_cant_build_RVA
  #define OGSOFF_cant_build_RVA 0
  #define OGSOFF_cant_build_FOUND 0
#endif
#ifndef OGSOFF_apply_character_customization_RVA
  #define OGSOFF_apply_character_customization_RVA 0
  #define OGSOFF_apply_character_customization_FOUND 0
#endif
#ifndef OGSOFF_bot_manager_setup_RVA
  #define OGSOFF_bot_manager_setup_RVA 0
  #define OGSOFF_bot_manager_setup_FOUND 0
#endif
#ifndef OGSOFF_register_component_with_world_RVA
  #define OGSOFF_register_component_with_world_RVA 0
  #define OGSOFF_register_component_with_world_FOUND 0
#endif
#ifndef OGSOFF_remove_from_alive_players_RVA
  #define OGSOFF_remove_from_alive_players_RVA 0
  #define OGSOFF_remove_from_alive_players_FOUND 0
#endif
#ifndef OGSOFF_add_to_alive_players_RVA
  #define OGSOFF_add_to_alive_players_RVA 0
  #define OGSOFF_add_to_alive_players_FOUND 0
#endif
#ifndef OGSOFF_static_load_object_RVA
  #define OGSOFF_static_load_object_RVA 0
  #define OGSOFF_static_load_object_FOUND 0
#endif
#ifndef OGSOFF_fframe_step_a_RVA
  #define OGSOFF_fframe_step_a_RVA 0
  #define OGSOFF_fframe_step_a_FOUND 0
#endif
#ifndef OGSOFF_fframe_step_b_RVA
  #define OGSOFF_fframe_step_b_RVA 0
  #define OGSOFF_fframe_step_b_FOUND 0
#endif
#ifndef OGSOFF_get_interface_RVA
  #define OGSOFF_get_interface_RVA 0
  #define OGSOFF_get_interface_FOUND 0
#endif
#ifndef OGSOFF_gadget_add_RVA
  #define OGSOFF_gadget_add_RVA 0
  #define OGSOFF_gadget_add_FOUND 0
#endif
#ifndef OGSOFF_gadget_remove_RVA
  #define OGSOFF_gadget_remove_RVA 0
  #define OGSOFF_gadget_remove_FOUND 0
#endif
#ifndef OGSOFF_tick_flush_RVA
  #define OGSOFF_tick_flush_RVA 0
  #define OGSOFF_tick_flush_FOUND 0
#endif
#ifndef OGSOFF_get_max_tick_rate_RVA
  #define OGSOFF_get_max_tick_rate_RVA 0
  #define OGSOFF_get_max_tick_rate_FOUND 0
#endif
#ifndef OGSOFF_on_reload_RVA
  #define OGSOFF_on_reload_RVA 0
  #define OGSOFF_on_reload_FOUND 0
#endif
#ifndef OGSOFF_k2_remove_item_from_player_RVA
  #define OGSOFF_k2_remove_item_from_player_RVA 0
  #define OGSOFF_k2_remove_item_from_player_FOUND 0
#endif
#ifndef OGSOFF_spawn_loot_RVA
  #define OGSOFF_spawn_loot_RVA 0
  #define OGSOFF_spawn_loot_FOUND 0
#endif
#ifndef OGSOFF_on_damage_server_RVA
  #define OGSOFF_on_damage_server_RVA 0
  #define OGSOFF_on_damage_server_FOUND 0
#endif
#ifndef OGSOFF_client_on_pawn_died_RVA
  #define OGSOFF_client_on_pawn_died_RVA 0
  #define OGSOFF_client_on_pawn_died_FOUND 0
#endif
#ifndef OGSOFF_server_attempt_interact_RVA
  #define OGSOFF_server_attempt_interact_RVA 0
  #define OGSOFF_server_attempt_interact_FOUND 0
#endif
#ifndef OGSOFF_apply_cost_RVA
  #define OGSOFF_apply_cost_RVA 0
  #define OGSOFF_apply_cost_FOUND 0
#endif
#ifndef OGSOFF_on_capsule_begin_overlap_RVA
  #define OGSOFF_on_capsule_begin_overlap_RVA 0
  #define OGSOFF_on_capsule_begin_overlap_FOUND 0
#endif
#ifndef OGSOFF_server_play_squad_quick_chat_RVA
  #define OGSOFF_server_play_squad_quick_chat_RVA 0
  #define OGSOFF_server_play_squad_quick_chat_FOUND 0
#endif
#ifndef OGSOFF_rebooting_delegate_RVA
  #define OGSOFF_rebooting_delegate_RVA 0
  #define OGSOFF_rebooting_delegate_FOUND 0
#endif
#ifndef OGSOFF_moving_emote_stopped_RVA
  #define OGSOFF_moving_emote_stopped_RVA 0
  #define OGSOFF_moving_emote_stopped_FOUND 0
#endif
#ifndef OGSOFF_on_exploded_RVA
  #define OGSOFF_on_exploded_RVA 0
  #define OGSOFF_on_exploded_FOUND 0
#endif
#ifndef OGSOFF_ready_to_start_match_RVA
  #define OGSOFF_ready_to_start_match_RVA 0
  #define OGSOFF_ready_to_start_match_FOUND 0
#endif
#ifndef OGSOFF_spawn_default_pawn_for_RVA
  #define OGSOFF_spawn_default_pawn_for_RVA 0
  #define OGSOFF_spawn_default_pawn_for_FOUND 0
#endif
#ifndef OGSOFF_start_aircraft_phase_RVA
  #define OGSOFF_start_aircraft_phase_RVA 0
  #define OGSOFF_start_aircraft_phase_FOUND 0
#endif
#ifndef OGSOFF_on_aircraft_exited_drop_zone_RVA
  #define OGSOFF_on_aircraft_exited_drop_zone_RVA 0
  #define OGSOFF_on_aircraft_exited_drop_zone_FOUND 0
#endif
#ifndef OGSOFF_on_aircraft_entered_drop_zone_RVA
  #define OGSOFF_on_aircraft_entered_drop_zone_RVA 0
  #define OGSOFF_on_aircraft_entered_drop_zone_FOUND 0
#endif
#ifndef OGSOFF_storm_RVA
  #define OGSOFF_storm_RVA 0
  #define OGSOFF_storm_FOUND 0
#endif
#ifndef OGSOFF_spawn_bot_RVA
  #define OGSOFF_spawn_bot_RVA 0
  #define OGSOFF_spawn_bot_FOUND 0
#endif
#ifndef OGSOFF_set_up_inventory_bot_RVA
  #define OGSOFF_set_up_inventory_bot_RVA 0
  #define OGSOFF_set_up_inventory_bot_FOUND 0
#endif
#ifndef OGSOFF_on_possessed_pawn_died_RVA
  #define OGSOFF_on_possessed_pawn_died_RVA 0
  #define OGSOFF_on_possessed_pawn_died_FOUND 0
#endif
#ifndef OGSOFF_on_perception_sensed_RVA
  #define OGSOFF_on_perception_sensed_RVA 0
  #define OGSOFF_on_perception_sensed_FOUND 0
#endif
#ifndef OGSOFF_on_actor_bump_RVA
  #define OGSOFF_on_actor_bump_RVA 0
  #define OGSOFF_on_actor_bump_FOUND 0
#endif
#ifndef OGSOFF_process_event_RVA
  #define OGSOFF_process_event_RVA 0
  #define OGSOFF_process_event_FOUND 0
#endif
#ifndef OGSOFF_misc_collect_garbage_RVA
  #define OGSOFF_misc_collect_garbage_RVA 0
  #define OGSOFF_misc_collect_garbage_FOUND 0
#endif
#ifndef OGSOFF_misc_kick_player_RVA
  #define OGSOFF_misc_kick_player_RVA 0
  #define OGSOFF_misc_kick_player_FOUND 0
#endif
#ifndef OGSOFF_misc_change_gamesession_id_RVA
  #define OGSOFF_misc_change_gamesession_id_RVA 0
  #define OGSOFF_misc_change_gamesession_id_FOUND 0
#endif
#ifndef OGSOFF_misc_dispatch_request_RVA
  #define OGSOFF_misc_dispatch_request_RVA 0
  #define OGSOFF_misc_dispatch_request_FOUND 0
#endif
#ifndef OGSOFF_misc_can_create_context_RVA
  #define OGSOFF_misc_can_create_context_RVA 0
  #define OGSOFF_misc_can_create_context_FOUND 0
#endif
#ifndef OGSOFF_misc_nullfunc_0_RVA
  #define OGSOFF_misc_nullfunc_0_RVA 0
  #define OGSOFF_misc_nullfunc_0_FOUND 0
#endif
#ifndef OGSOFF_misc_nullfunc_1_RVA
  #define OGSOFF_misc_nullfunc_1_RVA 0
  #define OGSOFF_misc_nullfunc_1_FOUND 0
#endif
#ifndef OGSOFF_misc_nullfunc_2_RVA
  #define OGSOFF_misc_nullfunc_2_RVA 0
  #define OGSOFF_misc_nullfunc_2_FOUND 0
#endif
#ifndef OGSOFF_k2_destroy_actor_RVA
  #define OGSOFF_k2_destroy_actor_RVA 0
  #define OGSOFF_k2_destroy_actor_FOUND 0
#endif
#ifndef OGSOFF_pick_team_RVA
  #define OGSOFF_pick_team_RVA 0
  #define OGSOFF_pick_team_FOUND 0
#endif
#ifndef OGSOFF_spawnbot_self_call_rva_RVA
  #define OGSOFF_spawnbot_self_call_rva_RVA 0
  #define OGSOFF_spawnbot_self_call_rva_FOUND 0
#endif
#ifndef OGSOFF_struct_faction_tag_RVA
  #define OGSOFF_struct_faction_tag_RVA 0
  #define OGSOFF_struct_faction_tag_FOUND 0
#endif
#ifndef OGSOFF_gis_client_RVA
  #define OGSOFF_gis_client_RVA 0
  #define OGSOFF_gis_client_FOUND 0
#endif
#ifndef OGSOFF_gis_server_RVA
  #define OGSOFF_gis_server_RVA 0
  #define OGSOFF_gis_server_FOUND 0
#endif
#ifndef OGSOFF_uworld_getnetmode_RVA
  #define OGSOFF_uworld_getnetmode_RVA 0
  #define OGSOFF_uworld_getnetmode_FOUND 0
#endif
#ifndef OGSOFF_aactor_getnetmode_RVA
  #define OGSOFF_aactor_getnetmode_RVA 0
  #define OGSOFF_aactor_getnetmode_FOUND 0
#endif

namespace Ogs
{
	// =========================================================================
	// Config — tiny flat JSON reader (NO dependencies).
	// Recognised shape (anything else is ignored, defaults win):
	//   {"features":{"netmode_hooks":true,...,"max_bots":100},
	//    "vt_indices":{"UNavigationSystemV1_InitializeForWorld":83},
	//    "game":"12.41"}
	// =========================================================================
	struct Cfg
	{
		std::map<std::string, bool> Features;
		std::map<std::string, long long> Numbers;
		std::map<std::string, long long> VtIndices;
		std::string Game;

		// Spec defaults.  netmode_hooks defaults to "the scanner found
		// uworld_getnetmode" via the generated header.
		bool DefaultFeature(const std::string& name) const
		{
			if (name == "netmode_hooks")  return OGSOFF_uworld_getnetmode_FOUND != 0;
			if (name == "gis_flip")       return false;
			if (name == "listen_server")  return false;
			if (name == "bots")           return true;
			if (name == "bosses")         return true;
			if (name == "booths")         return true;
			if (name == "vehicles")       return true;
			if (name == "late_game")      return false;
			if (name == "exec_hooks")     return true;
			if (name == "bots_tick_thread") return true;
			if (name == "legacy_ai_mang") return true;
			return false;
		}

		long long DefaultNumber(const std::string& name) const
		{
			if (name == "max_bots")                return 100;
			if (name == "min_players_early_start") return 100;
			return 0;
		}

		/// Ogs::Cfg.has("flag") — explicit true/false from ogsen.json or the
		/// baked default.  "flag":<number> counts as true when nonzero.
		bool Has(const char* flag) const
		{
			std::string name(flag);
			auto it = Features.find(name);
			if (it != Features.end())
				return it->second;
			auto n = Numbers.find(name);
			if (n != Numbers.end())
				return n->second != 0;
			return DefaultFeature(name);
		}

		long long Num(const char* key) const
		{
			auto n = Numbers.find(key);
			if (n != Numbers.end())
				return n->second;
			auto f = Features.find(key);
			if (f != Features.end())
				return f->second ? 1 : 0;
			return DefaultNumber(key);
		}

		/// vtable index by name (from "vt_indices"); -1 when unknown.
		long long VtIndex(const char* name, long long def) const
		{
			auto it = VtIndices.find(name);
			return it != VtIndices.end() ? it->second : def;
		}
	};

	/// Parse the config JSON (the string handed to ogs_main).  Never throws;
	/// on any error the result simply keeps its defaults.
	void Init(const char* configJson);
	Cfg& CfgRef();

	// =========================================================================
	// Logging
	// =========================================================================
	void Log(const char* fmt, ...);
	/// Append a line to ogsen_log.txt in the game files dir (best effort).
	void LogFile(const char* line);

#define OGS_LOG(...) ::Ogs::Log(__VA_ARGS__)

	// =========================================================================
	// Module base / memory helpers
	// =========================================================================
	/// Base address of the libUE4-containing module (dl_iterate_phdr,
	/// cached).  Host: dummy 0x1000 so Resolve() is inert but non-null.
	uintptr_t Base();
	/// Base() + rva.
	inline uintptr_t Resolve(uintptr_t rva) { return Base() + rva; }
	void SleepMs(int ms);
	/// mprotect [ptr, ptr+len) to RW; host no-op.  Returns true on success.
	bool MProtectRW(void* ptr, size_t len);

#define OGS_RETURN_ADDRESS() (__builtin_return_address(0))

	// =========================================================================
	// Offset registry
	// =========================================================================
	struct OffEntry
	{
		uintptr_t Rva;
		bool Found;
	};

	struct OffRegistry
	{
		// --- scanner keys (offsets/ogs-1241-android.json, verbatim) --------
		OffEntry gis_client{OGSOFF_gis_client_RVA, OGSOFF_gis_client_FOUND != 0};
		OffEntry gis_server{OGSOFF_gis_server_RVA, OGSOFF_gis_server_FOUND != 0};
		OffEntry uworld_getnetmode{OGSOFF_uworld_getnetmode_RVA, OGSOFF_uworld_getnetmode_FOUND != 0};
		OffEntry aactor_getnetmode{OGSOFF_aactor_getnetmode_RVA, OGSOFF_aactor_getnetmode_FOUND != 0};
		OffEntry world_wrapper_getnetmode_leaf{0, false};
		OffEntry uworld_listen_7777{0, false};

		// --- mod-only keys (all found:false until the scanner grows) -------
		OffEntry create_netdriver{OGSOFF_create_netdriver_RVA, OGSOFF_create_netdriver_FOUND != 0};
		OffEntry init_listen{OGSOFF_init_listen_RVA, OGSOFF_init_listen_FOUND != 0};
		OffEntry set_world{OGSOFF_set_world_RVA, OGSOFF_set_world_FOUND != 0};
		OffEntry server_replicate_actors{OGSOFF_server_replicate_actors_RVA, OGSOFF_server_replicate_actors_FOUND != 0};
		OffEntry give_ability{OGSOFF_give_ability_RVA, OGSOFF_give_ability_FOUND != 0};
		OffEntry ability_spec_constructor{OGSOFF_ability_spec_constructor_RVA, OGSOFF_ability_spec_constructor_FOUND != 0};
		OffEntry internal_try_activate_ability{OGSOFF_internal_try_activate_ability_RVA, OGSOFF_internal_try_activate_ability_FOUND != 0};
		OffEntry give_ability_and_activate_once{OGSOFF_give_ability_and_activate_once_RVA, OGSOFF_give_ability_and_activate_once_FOUND != 0};
		OffEntry pick_supply_drop_location{OGSOFF_pick_supply_drop_location_RVA, OGSOFF_pick_supply_drop_location_FOUND != 0};
		OffEntry replace_building_actor{OGSOFF_replace_building_actor_RVA, OGSOFF_replace_building_actor_FOUND != 0};
		OffEntry cant_build{OGSOFF_cant_build_RVA, OGSOFF_cant_build_FOUND != 0};
		OffEntry apply_character_customization{OGSOFF_apply_character_customization_RVA, OGSOFF_apply_character_customization_FOUND != 0};
		OffEntry bot_manager_setup{OGSOFF_bot_manager_setup_RVA, OGSOFF_bot_manager_setup_FOUND != 0};
		OffEntry register_component_with_world{OGSOFF_register_component_with_world_RVA, OGSOFF_register_component_with_world_FOUND != 0};
		OffEntry remove_from_alive_players{OGSOFF_remove_from_alive_players_RVA, OGSOFF_remove_from_alive_players_FOUND != 0};
		OffEntry add_to_alive_players{OGSOFF_add_to_alive_players_RVA, OGSOFF_add_to_alive_players_FOUND != 0};
		OffEntry static_load_object{OGSOFF_static_load_object_RVA, OGSOFF_static_load_object_FOUND != 0};
		OffEntry fframe_step_a{OGSOFF_fframe_step_a_RVA, OGSOFF_fframe_step_a_FOUND != 0};
		OffEntry fframe_step_b{OGSOFF_fframe_step_b_RVA, OGSOFF_fframe_step_b_FOUND != 0};
		OffEntry get_interface{OGSOFF_get_interface_RVA, OGSOFF_get_interface_FOUND != 0};
		OffEntry gadget_add{OGSOFF_gadget_add_RVA, OGSOFF_gadget_add_FOUND != 0};
		OffEntry gadget_remove{OGSOFF_gadget_remove_RVA, OGSOFF_gadget_remove_FOUND != 0};
		OffEntry tick_flush{OGSOFF_tick_flush_RVA, OGSOFF_tick_flush_FOUND != 0};
		OffEntry get_max_tick_rate{OGSOFF_get_max_tick_rate_RVA, OGSOFF_get_max_tick_rate_FOUND != 0};
		OffEntry on_reload{OGSOFF_on_reload_RVA, OGSOFF_on_reload_FOUND != 0};
		OffEntry k2_remove_item_from_player{OGSOFF_k2_remove_item_from_player_RVA, OGSOFF_k2_remove_item_from_player_FOUND != 0};
		OffEntry spawn_loot{OGSOFF_spawn_loot_RVA, OGSOFF_spawn_loot_FOUND != 0};
		OffEntry on_damage_server{OGSOFF_on_damage_server_RVA, OGSOFF_on_damage_server_FOUND != 0};
		OffEntry client_on_pawn_died{OGSOFF_client_on_pawn_died_RVA, OGSOFF_client_on_pawn_died_FOUND != 0};
		OffEntry server_attempt_interact{OGSOFF_server_attempt_interact_RVA, OGSOFF_server_attempt_interact_FOUND != 0};
		OffEntry apply_cost{OGSOFF_apply_cost_RVA, OGSOFF_apply_cost_FOUND != 0};
		OffEntry on_capsule_begin_overlap{OGSOFF_on_capsule_begin_overlap_RVA, OGSOFF_on_capsule_begin_overlap_FOUND != 0};
		OffEntry server_play_squad_quick_chat{OGSOFF_server_play_squad_quick_chat_RVA, OGSOFF_server_play_squad_quick_chat_FOUND != 0};
		OffEntry rebooting_delegate{OGSOFF_rebooting_delegate_RVA, OGSOFF_rebooting_delegate_FOUND != 0};
		OffEntry moving_emote_stopped{OGSOFF_moving_emote_stopped_RVA, OGSOFF_moving_emote_stopped_FOUND != 0};
		OffEntry on_exploded{OGSOFF_on_exploded_RVA, OGSOFF_on_exploded_FOUND != 0};
		OffEntry ready_to_start_match{OGSOFF_ready_to_start_match_RVA, OGSOFF_ready_to_start_match_FOUND != 0};
		OffEntry spawn_default_pawn_for{OGSOFF_spawn_default_pawn_for_RVA, OGSOFF_spawn_default_pawn_for_FOUND != 0};
		OffEntry start_aircraft_phase{OGSOFF_start_aircraft_phase_RVA, OGSOFF_start_aircraft_phase_FOUND != 0};
		OffEntry on_aircraft_exited_drop_zone{OGSOFF_on_aircraft_exited_drop_zone_RVA, OGSOFF_on_aircraft_exited_drop_zone_FOUND != 0};
		OffEntry on_aircraft_entered_drop_zone{OGSOFF_on_aircraft_entered_drop_zone_RVA, OGSOFF_on_aircraft_entered_drop_zone_FOUND != 0};
		OffEntry storm{OGSOFF_storm_RVA, OGSOFF_storm_FOUND != 0};
		OffEntry spawn_bot{OGSOFF_spawn_bot_RVA, OGSOFF_spawn_bot_FOUND != 0};
		OffEntry set_up_inventory_bot{OGSOFF_set_up_inventory_bot_RVA, OGSOFF_set_up_inventory_bot_FOUND != 0};
		OffEntry on_possessed_pawn_died{OGSOFF_on_possessed_pawn_died_RVA, OGSOFF_on_possessed_pawn_died_FOUND != 0};
		OffEntry on_perception_sensed{OGSOFF_on_perception_sensed_RVA, OGSOFF_on_perception_sensed_FOUND != 0};
		OffEntry on_actor_bump{OGSOFF_on_actor_bump_RVA, OGSOFF_on_actor_bump_FOUND != 0};
		OffEntry process_event{OGSOFF_process_event_RVA, OGSOFF_process_event_FOUND != 0};
		OffEntry misc_collect_garbage{OGSOFF_misc_collect_garbage_RVA, OGSOFF_misc_collect_garbage_FOUND != 0};
		OffEntry misc_kick_player{OGSOFF_misc_kick_player_RVA, OGSOFF_misc_kick_player_FOUND != 0};
		OffEntry misc_change_gamesession_id{OGSOFF_misc_change_gamesession_id_RVA, OGSOFF_misc_change_gamesession_id_FOUND != 0};
		OffEntry misc_dispatch_request{OGSOFF_misc_dispatch_request_RVA, OGSOFF_misc_dispatch_request_FOUND != 0};
		OffEntry misc_can_create_context{OGSOFF_misc_can_create_context_RVA, OGSOFF_misc_can_create_context_FOUND != 0};
		OffEntry misc_nullfunc_0{OGSOFF_misc_nullfunc_0_RVA, OGSOFF_misc_nullfunc_0_FOUND != 0};
		OffEntry misc_nullfunc_1{OGSOFF_misc_nullfunc_1_RVA, OGSOFF_misc_nullfunc_1_FOUND != 0};
		OffEntry misc_nullfunc_2{OGSOFF_misc_nullfunc_2_RVA, OGSOFF_misc_nullfunc_2_FOUND != 0};
		OffEntry k2_destroy_actor{OGSOFF_k2_destroy_actor_RVA, OGSOFF_k2_destroy_actor_FOUND != 0};
		OffEntry pick_team{OGSOFF_pick_team_RVA, OGSOFF_pick_team_FOUND != 0};
		// NOT an RVA: caller-return-address guard value from Bots::SpawnBot.
		OffEntry spawnbot_self_call_rva{OGSOFF_spawnbot_self_call_rva_RVA, OGSOFF_spawnbot_self_call_rva_FOUND != 0};
		// NOT an RVA: raw struct offset of the FGameplayTag faction field on
		// actor classes that are NOT in the dump (PhantomBooth et al.).
		OffEntry struct_faction_tag{OGSOFF_struct_faction_tag_RVA, OGSOFF_struct_faction_tag_FOUND != 0};
	};

	/// The single registry instance (populated once at static init).
	const OffRegistry& Off();

	// =========================================================================
	// Ogs::Text — UTF-8 char* → SDK::FString
	// -----------------------------------------------------------------------------
	// The Windows dump's FString derives TArray<wchar_t>; MobileDumper-7's
	// Android dump derives TArray<char16_t>.  Both expose TArray's public
	// Add()/Num() and operator[], so the element type is deduced from
	// operator[] and the UTF-8 input is encoded to fit (UTF-16 with surrogate
	// pairs for 16-bit elements, UTF-32 for wider ones).  Every L"..." in the
	// original becomes OGS_TEXT("...").
	// =========================================================================
	SDK::FString Text(const char* utf8);
}

#define OGS_TEXT(s) (::Ogs::Text(s))
