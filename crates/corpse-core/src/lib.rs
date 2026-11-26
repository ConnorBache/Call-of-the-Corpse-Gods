pub mod hanged;
pub mod revision51_perception;
pub mod revision50_creature_identity;
pub mod revision50_item_knowledge;
pub mod revision50_rings;
pub mod revision52_swimming;
pub mod revision50_player_attack;
pub mod revision52_reserves;
pub mod revision50_ability_energy;
pub mod revision50_amulets;
pub mod revision50_amulet_effects;
pub mod revision50_control_amulets;
pub mod revision50_wave_teleport;
pub mod revision50_grappler;
pub mod actor_equipment;
pub mod revision50_nightmares;
pub mod revision50;
pub mod revision50_inventory;
pub mod revision50_combat;
pub mod revision50_terrain;
pub mod revision50_pursuit;
mod revision51_enemy_reroute;
pub mod revision51_ichor_fire;
pub mod timeweave;
pub mod capture_pilot;
pub mod mutation_journal;
pub mod optical_cache;
pub mod creature_trie;
pub mod creature_index;
pub mod deadline_map;
pub mod lighting_cache;
pub mod health_dirty;
pub mod turn_schedule;
pub mod model;
pub mod rng;
pub mod storage;
pub mod temporal;
pub use model::*;
pub mod corpses;
pub mod generation;
pub mod movement;
pub mod propagation;

pub mod abilities;
pub mod ai;
pub mod combat;
mod attack_text;
pub mod engine;
pub mod environment;
pub mod water_blood;
pub mod equipment;
mod unique_composite_bow;
pub mod gas_reactions;
pub mod inventory;
pub mod progression;
pub mod resources;
pub mod vision;

pub mod fire_adapter;

pub mod replay;

pub mod hazard_memory;

pub mod fungi;

pub mod currents;
pub mod square_currents;

pub mod validation;

pub mod migrations;

pub mod offscreen;

pub mod crafting;

pub mod population;

pub mod ichor;

pub mod spawn_heatmap;

pub mod item_placement;

pub mod events;

pub mod sphinx_generation;

pub mod startup;
pub mod fast_mode;

pub mod projectiles;

pub mod travel;

pub mod corpse_placement;
pub(crate) mod corpse_currents;

pub mod lifecycle;
pub mod monster_statuses;
pub mod profile;

pub mod half_here;

pub mod monster_vampire;

pub mod divine_combat;

pub mod monster_wendigo;

pub mod divine_environment;

pub mod divine_rescues;

pub mod monster_bombardier;
pub mod monster_caster;
pub mod monster_grab;
pub mod monster_horn;
pub mod monster_inventor;
pub mod monster_moon;
pub mod monster_spells;
pub mod monster_sphinx;
pub mod monster_stalking;

pub mod maintained_links;

pub mod divine_reactions;

pub mod equipment_abilities;

pub mod tinker;

pub mod enemy_traits;

pub mod mini_me;

pub mod monster_chill;

pub mod potions;
mod potion_animation;
pub mod revision50_potions;

pub mod godbeast_interactions;

pub mod migration_pipeline;

pub mod monster_ruin;

pub mod ordinary_roster;

pub mod entrance;
pub mod playtest;
pub mod debug_floor;

pub mod physical_balance;
pub mod balance42;
pub mod accuracy_dodge;

pub mod finale;

pub mod flavor_placement;

pub mod combat_audit;

pub mod new_creatures;
pub mod witch;

pub mod creature_feeding;

pub mod borge;

pub mod saidribee_queen;
pub mod pocket_biomes;
pub mod royal_honey;
pub mod zillennia_ground;

pub mod actor_presentation;

pub mod dreamwalking;
pub mod phantasmii_generation;

pub mod epidite;

pub mod ahvamitt;

pub mod portals;
pub mod portals_generation;

pub mod non_euclidean_generation;

pub mod terrain_runtime;
pub mod sub_biomes_generation;
pub mod sub_biomes_validation;

pub mod timing;

/// Revision40 bank-reaching currents; legacy current generation remains unchanged.
pub mod shore_currents;

/// Immutable typed Future anchors; legacy strings remain boundary-compatible.
pub mod future_anchor;

#[cfg(test)]
mod attack_text_revision46;

/// Revision51 depth-appropriate, fully rooted fungus feast encounters.
pub mod fungus_feasts;

/// Revision51 hostile stair exclusion and adjacent cross-floor arrivals.
pub mod revision51_stairs;

/// Revision51 water extinguishes ordinary and Sparkle fire sources.
pub mod revision51_fire_water;

/// Detached revision52 enemy animation outcome cues.
pub mod animation_enemy;

pub mod animation_amulets;
/// Firebolt-only travel amendment and command-ledger compatibility policy.
pub mod firebolt_travel;
pub mod amulet_rules;
pub mod potion_rules;
pub mod ring_rules;
pub mod gas_rules;
pub mod commandment_rules;
pub mod screamer_rules;
pub mod death_gas_rules;
pub mod ember_rules;
/// Ragelight passive rescue amendment and legacy replay boundary.
pub mod ragelight_rules;
pub mod balance_rules;
pub mod steam_rules;
pub mod execute_rules;
pub mod beckoning_rules;
pub mod ichor_fire_rules;
mod beckoning_skip;
pub mod screamer_schedule;
pub mod future_floor;
pub mod animation_player;
pub mod animation_shapes;
#[cfg(test)] mod animation_contract_tests;

pub mod animation_observation;
#[cfg(test)] mod animation_inventory_tests;

/// Explicit revision52 floor-aware natural Fighter birth tuning.
pub mod balance52;
/// R52 paid Dragon flight and precise contact follow-ups.
pub mod revision52_enemy_followups;
/// Immutable item-birth tiers and captured amulet effect profiles.
pub mod revision52_amulet_tiers;
pub mod revision52_dragon_flight;

pub(crate) mod revision52_speed;

#[cfg(test)]
mod fast_mode_tests;

#[cfg(test)]
mod fast_mode_boss_tests;

pub mod fortune_schedule;

pub mod ability_sight;

pub mod blood_mist;

mod verdania_guard;

#[cfg(test)] mod animation_video_fixtures;

pub mod avatar_starters;
