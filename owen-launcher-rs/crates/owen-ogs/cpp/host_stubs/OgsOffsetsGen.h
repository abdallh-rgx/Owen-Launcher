// =============================================================================
// OgsOffsetsGen.h — HOST SYNTAX-CHECK FALLBACK (all found=0).
//
// In CI, build.rs generates the real $OUT_DIR/OgsOffsetsGen.h from
// offsets/ogs-1241-android.json ("entries": { key: {found, rva} }) as
//
//     #define OGSOFF_<KEY>_FOUND 1        (or 0)
//     #define OGSOFF_<KEY>_RVA  0x...     (0 when not found)
//
// and $OUT_DIR precedes cpp/host_stubs on the include path, so THIS file is
// only seen when no generated header exists (host g++ -fsyntax-only gate).
// Every macro is #ifndef-guarded and OgsCompat.h fills the same defaults for
// its mod-only keys, so both paths agree.
//
// Keys covered here (scanner JSON keys verbatim + the mod-only keys the port
// needs; see OgsCompat.h for the full documented list):
//   gis_client gis_server uworld_getnetmode aactor_getnetmode
//   world_wrapper_getnetmode_leaf uworld_listen_7777
//   xref_GameNetDriver xref_UNetDriver xref_IpNetDriver xref_DemoNetDriver
//   xref_BeaconNetDriver xref_StatelessConnectHandlerComponent xref_InitListen
//   xref_CreateNetDriver xref_ServerReplicateActors xref_ReplicationDriver
//   xref_NetDriverDefinitions
//   create_netdriver init_listen set_world server_replicate_actors
//   give_ability ability_spec_constructor internal_try_activate_ability
//   give_ability_and_activate_once pick_supply_drop_location
//   replace_building_actor cant_build apply_character_customization
//   bot_manager_setup register_component_with_world remove_from_alive_players
//   add_to_alive_players static_find_object static_load_object
//   fframe_step_a fframe_step_b get_interface gadget_add gadget_remove
//   tick_flush get_max_tick_rate on_reload k2_remove_item_from_player
   spawn_loot on_damage_server
//   client_on_pawn_died server_attempt_interact apply_cost
//   on_capsule_begin_overlap server_play_squad_quick_chat rebooting_delegate
//   moving_emote_stopped on_exploded ready_to_start_match
//   spawn_default_pawn_for start_aircraft_phase on_aircraft_exited_drop_zone
//   on_aircraft_entered_drop_zone storm spawn_bot set_up_inventory_bot
//   on_possessed_pawn_died on_perception_sensed on_actor_bump process_event
//   misc_collect_garbage misc_kick_player misc_change_gamesession_id
//   misc_dispatch_request misc_can_create_context misc_nullfunc_0
//   misc_nullfunc_1 misc_nullfunc_2 k2_destroy_actor pick_team
//   spawnbot_self_call_rva struct_faction_tag
// =============================================================================

// ---- scanner keys (offsets/ogs-1241-android.json) — verbatim ---------------
#ifndef OGSOFF_gis_client_FOUND
#define OGSOFF_gis_client_FOUND 0
#define OGSOFF_gis_client_RVA 0
#endif
#ifndef OGSOFF_gis_server_FOUND
#define OGSOFF_gis_server_FOUND 0
#define OGSOFF_gis_server_RVA 0
#endif
#ifndef OGSOFF_uworld_getnetmode_FOUND
#define OGSOFF_uworld_getnetmode_FOUND 0
#define OGSOFF_uworld_getnetmode_RVA 0
#endif
#ifndef OGSOFF_aactor_getnetmode_FOUND
#define OGSOFF_aactor_getnetmode_FOUND 0
#define OGSOFF_aactor_getnetmode_RVA 0
#endif
#ifndef OGSOFF_world_wrapper_getnetmode_leaf_FOUND
#define OGSOFF_world_wrapper_getnetmode_leaf_FOUND 0
#define OGSOFF_world_wrapper_getnetmode_leaf_RVA 0
#endif
#ifndef OGSOFF_uworld_listen_7777_FOUND
#define OGSOFF_uworld_listen_7777_FOUND 0
#define OGSOFF_uworld_listen_7777_RVA 0
#endif
#ifndef OGSOFF_xref_GameNetDriver_FOUND
#define OGSOFF_xref_GameNetDriver_FOUND 0
#define OGSOFF_xref_GameNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_UNetDriver_FOUND
#define OGSOFF_xref_UNetDriver_FOUND 0
#define OGSOFF_xref_UNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_IpNetDriver_FOUND
#define OGSOFF_xref_IpNetDriver_FOUND 0
#define OGSOFF_xref_IpNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_DemoNetDriver_FOUND
#define OGSOFF_xref_DemoNetDriver_FOUND 0
#define OGSOFF_xref_DemoNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_BeaconNetDriver_FOUND
#define OGSOFF_xref_BeaconNetDriver_FOUND 0
#define OGSOFF_xref_BeaconNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_StatelessConnectHandlerComponent_FOUND
#define OGSOFF_xref_StatelessConnectHandlerComponent_FOUND 0
#define OGSOFF_xref_StatelessConnectHandlerComponent_RVA 0
#endif
#ifndef OGSOFF_xref_InitListen_FOUND
#define OGSOFF_xref_InitListen_FOUND 0
#define OGSOFF_xref_InitListen_RVA 0
#endif
#ifndef OGSOFF_xref_CreateNetDriver_FOUND
#define OGSOFF_xref_CreateNetDriver_FOUND 0
#define OGSOFF_xref_CreateNetDriver_RVA 0
#endif
#ifndef OGSOFF_xref_ServerReplicateActors_FOUND
#define OGSOFF_xref_ServerReplicateActors_FOUND 0
#define OGSOFF_xref_ServerReplicateActors_RVA 0
#endif
#ifndef OGSOFF_xref_ReplicationDriver_FOUND
#define OGSOFF_xref_ReplicationDriver_FOUND 0
#define OGSOFF_xref_ReplicationDriver_RVA 0
#endif
#ifndef OGSOFF_xref_NetDriverDefinitions_FOUND
#define OGSOFF_xref_NetDriverDefinitions_FOUND 0
#define OGSOFF_xref_NetDriverDefinitions_RVA 0
#endif

// ---- mod-only keys (see OgsCompat.h header comment for the win RVAs) ------
#ifndef OGSOFF_create_netdriver_FOUND
#define OGSOFF_create_netdriver_FOUND 0
#define OGSOFF_create_netdriver_RVA 0
#endif
#ifndef OGSOFF_init_listen_FOUND
#define OGSOFF_init_listen_FOUND 0
#define OGSOFF_init_listen_RVA 0
#endif
#ifndef OGSOFF_set_world_FOUND
#define OGSOFF_set_world_FOUND 0
#define OGSOFF_set_world_RVA 0
#endif
#ifndef OGSOFF_server_replicate_actors_FOUND
#define OGSOFF_server_replicate_actors_FOUND 0
#define OGSOFF_server_replicate_actors_RVA 0
#endif
#ifndef OGSOFF_give_ability_FOUND
#define OGSOFF_give_ability_FOUND 0
#define OGSOFF_give_ability_RVA 0
#endif
#ifndef OGSOFF_ability_spec_constructor_FOUND
#define OGSOFF_ability_spec_constructor_FOUND 0
#define OGSOFF_ability_spec_constructor_RVA 0
#endif
#ifndef OGSOFF_internal_try_activate_ability_FOUND
#define OGSOFF_internal_try_activate_ability_FOUND 0
#define OGSOFF_internal_try_activate_ability_RVA 0
#endif
#ifndef OGSOFF_give_ability_and_activate_once_FOUND
#define OGSOFF_give_ability_and_activate_once_FOUND 0
#define OGSOFF_give_ability_and_activate_once_RVA 0
#endif
#ifndef OGSOFF_pick_supply_drop_location_FOUND
#define OGSOFF_pick_supply_drop_location_FOUND 0
#define OGSOFF_pick_supply_drop_location_RVA 0
#endif
#ifndef OGSOFF_replace_building_actor_FOUND
#define OGSOFF_replace_building_actor_FOUND 0
#define OGSOFF_replace_building_actor_RVA 0
#endif
#ifndef OGSOFF_cant_build_FOUND
#define OGSOFF_cant_build_FOUND 0
#define OGSOFF_cant_build_RVA 0
#endif
#ifndef OGSOFF_apply_character_customization_FOUND
#define OGSOFF_apply_character_customization_FOUND 0
#define OGSOFF_apply_character_customization_RVA 0
#endif
#ifndef OGSOFF_bot_manager_setup_FOUND
#define OGSOFF_bot_manager_setup_FOUND 0
#define OGSOFF_bot_manager_setup_RVA 0
#endif
#ifndef OGSOFF_register_component_with_world_FOUND
#define OGSOFF_register_component_with_world_FOUND 0
#define OGSOFF_register_component_with_world_RVA 0
#endif
#ifndef OGSOFF_remove_from_alive_players_FOUND
#define OGSOFF_remove_from_alive_players_FOUND 0
#define OGSOFF_remove_from_alive_players_RVA 0
#endif
#ifndef OGSOFF_add_to_alive_players_FOUND
#define OGSOFF_add_to_alive_players_FOUND 0
#define OGSOFF_add_to_alive_players_RVA 0
#endif
#ifndef OGSOFF_static_find_object_FOUND
#define OGSOFF_static_find_object_FOUND 0
#define OGSOFF_static_find_object_RVA 0
#endif
#ifndef OGSOFF_static_load_object_FOUND
#define OGSOFF_static_load_object_FOUND 0
#define OGSOFF_static_load_object_RVA 0
#endif
#ifndef OGSOFF_fframe_step_a_FOUND
#define OGSOFF_fframe_step_a_FOUND 0
#define OGSOFF_fframe_step_a_RVA 0
#endif
#ifndef OGSOFF_fframe_step_b_FOUND
#define OGSOFF_fframe_step_b_FOUND 0
#define OGSOFF_fframe_step_b_RVA 0
#endif
#ifndef OGSOFF_get_interface_FOUND
#define OGSOFF_get_interface_FOUND 0
#define OGSOFF_get_interface_RVA 0
#endif
#ifndef OGSOFF_gadget_add_FOUND
#define OGSOFF_gadget_add_FOUND 0
#define OGSOFF_gadget_add_RVA 0
#endif
#ifndef OGSOFF_gadget_remove_FOUND
#define OGSOFF_gadget_remove_FOUND 0
#define OGSOFF_gadget_remove_RVA 0
#endif
#ifndef OGSOFF_tick_flush_FOUND
#define OGSOFF_tick_flush_FOUND 0
#define OGSOFF_tick_flush_RVA 0
#endif
#ifndef OGSOFF_get_max_tick_rate_FOUND
#define OGSOFF_get_max_tick_rate_FOUND 0
#define OGSOFF_get_max_tick_rate_RVA 0
#endif
#ifndef OGSOFF_on_reload_FOUND
#define OGSOFF_on_reload_FOUND 0
#define OGSOFF_on_reload_RVA 0
#endif
#ifndef OGSOFF_k2_remove_item_from_player_FOUND
#define OGSOFF_k2_remove_item_from_player_FOUND 0
#define OGSOFF_k2_remove_item_from_player_RVA 0
#endif
#ifndef OGSOFF_spawn_loot_FOUND
#define OGSOFF_spawn_loot_FOUND 0
#define OGSOFF_spawn_loot_RVA 0
#endif
#ifndef OGSOFF_on_damage_server_FOUND
#define OGSOFF_on_damage_server_FOUND 0
#define OGSOFF_on_damage_server_RVA 0
#endif
#ifndef OGSOFF_client_on_pawn_died_FOUND
#define OGSOFF_client_on_pawn_died_FOUND 0
#define OGSOFF_client_on_pawn_died_RVA 0
#endif
#ifndef OGSOFF_server_attempt_interact_FOUND
#define OGSOFF_server_attempt_interact_FOUND 0
#define OGSOFF_server_attempt_interact_RVA 0
#endif
#ifndef OGSOFF_apply_cost_FOUND
#define OGSOFF_apply_cost_FOUND 0
#define OGSOFF_apply_cost_RVA 0
#endif
#ifndef OGSOFF_on_capsule_begin_overlap_FOUND
#define OGSOFF_on_capsule_begin_overlap_FOUND 0
#define OGSOFF_on_capsule_begin_overlap_RVA 0
#endif
#ifndef OGSOFF_server_play_squad_quick_chat_FOUND
#define OGSOFF_server_play_squad_quick_chat_FOUND 0
#define OGSOFF_server_play_squad_quick_chat_RVA 0
#endif
#ifndef OGSOFF_rebooting_delegate_FOUND
#define OGSOFF_rebooting_delegate_FOUND 0
#define OGSOFF_rebooting_delegate_RVA 0
#endif
#ifndef OGSOFF_moving_emote_stopped_FOUND
#define OGSOFF_moving_emote_stopped_FOUND 0
#define OGSOFF_moving_emote_stopped_RVA 0
#endif
#ifndef OGSOFF_on_exploded_FOUND
#define OGSOFF_on_exploded_FOUND 0
#define OGSOFF_on_exploded_RVA 0
#endif
#ifndef OGSOFF_ready_to_start_match_FOUND
#define OGSOFF_ready_to_start_match_FOUND 0
#define OGSOFF_ready_to_start_match_RVA 0
#endif
#ifndef OGSOFF_spawn_default_pawn_for_FOUND
#define OGSOFF_spawn_default_pawn_for_FOUND 0
#define OGSOFF_spawn_default_pawn_for_RVA 0
#endif
#ifndef OGSOFF_start_aircraft_phase_FOUND
#define OGSOFF_start_aircraft_phase_FOUND 0
#define OGSOFF_start_aircraft_phase_RVA 0
#endif
#ifndef OGSOFF_on_aircraft_exited_drop_zone_FOUND
#define OGSOFF_on_aircraft_exited_drop_zone_FOUND 0
#define OGSOFF_on_aircraft_exited_drop_zone_RVA 0
#endif
#ifndef OGSOFF_on_aircraft_entered_drop_zone_FOUND
#define OGSOFF_on_aircraft_entered_drop_zone_FOUND 0
#define OGSOFF_on_aircraft_entered_drop_zone_RVA 0
#endif
#ifndef OGSOFF_storm_FOUND
#define OGSOFF_storm_FOUND 0
#define OGSOFF_storm_RVA 0
#endif
#ifndef OGSOFF_spawn_bot_FOUND
#define OGSOFF_spawn_bot_FOUND 0
#define OGSOFF_spawn_bot_RVA 0
#endif
#ifndef OGSOFF_set_up_inventory_bot_FOUND
#define OGSOFF_set_up_inventory_bot_FOUND 0
#define OGSOFF_set_up_inventory_bot_RVA 0
#endif
#ifndef OGSOFF_on_possessed_pawn_died_FOUND
#define OGSOFF_on_possessed_pawn_died_FOUND 0
#define OGSOFF_on_possessed_pawn_died_RVA 0
#endif
#ifndef OGSOFF_on_perception_sensed_FOUND
#define OGSOFF_on_perception_sensed_FOUND 0
#define OGSOFF_on_perception_sensed_RVA 0
#endif
#ifndef OGSOFF_on_actor_bump_FOUND
#define OGSOFF_on_actor_bump_FOUND 0
#define OGSOFF_on_actor_bump_RVA 0
#endif
#ifndef OGSOFF_process_event_FOUND
#define OGSOFF_process_event_FOUND 0
#define OGSOFF_process_event_RVA 0
#endif
#ifndef OGSOFF_misc_collect_garbage_FOUND
#define OGSOFF_misc_collect_garbage_FOUND 0
#define OGSOFF_misc_collect_garbage_RVA 0
#endif
#ifndef OGSOFF_misc_kick_player_FOUND
#define OGSOFF_misc_kick_player_FOUND 0
#define OGSOFF_misc_kick_player_RVA 0
#endif
#ifndef OGSOFF_misc_change_gamesession_id_FOUND
#define OGSOFF_misc_change_gamesession_id_FOUND 0
#define OGSOFF_misc_change_gamesession_id_RVA 0
#endif
#ifndef OGSOFF_misc_dispatch_request_FOUND
#define OGSOFF_misc_dispatch_request_FOUND 0
#define OGSOFF_misc_dispatch_request_RVA 0
#endif
#ifndef OGSOFF_misc_can_create_context_FOUND
#define OGSOFF_misc_can_create_context_FOUND 0
#define OGSOFF_misc_can_create_context_RVA 0
#endif
#ifndef OGSOFF_misc_nullfunc_0_FOUND
#define OGSOFF_misc_nullfunc_0_FOUND 0
#define OGSOFF_misc_nullfunc_0_RVA 0
#endif
#ifndef OGSOFF_misc_nullfunc_1_FOUND
#define OGSOFF_misc_nullfunc_1_FOUND 0
#define OGSOFF_misc_nullfunc_1_RVA 0
#endif
#ifndef OGSOFF_misc_nullfunc_2_FOUND
#define OGSOFF_misc_nullfunc_2_FOUND 0
#define OGSOFF_misc_nullfunc_2_RVA 0
#endif
#ifndef OGSOFF_k2_destroy_actor_FOUND
#define OGSOFF_k2_destroy_actor_FOUND 0
#define OGSOFF_k2_destroy_actor_RVA 0
#endif
#ifndef OGSOFF_pick_team_FOUND
#define OGSOFF_pick_team_FOUND 0
#define OGSOFF_pick_team_RVA 0
#endif
#ifndef OGSOFF_spawnbot_self_call_rva_FOUND
#define OGSOFF_spawnbot_self_call_rva_FOUND 0
#define OGSOFF_spawnbot_self_call_rva_RVA 0
#endif
#ifndef OGSOFF_struct_faction_tag_FOUND
#define OGSOFF_struct_faction_tag_FOUND 0
#define OGSOFF_struct_faction_tag_RVA 0
#endif
