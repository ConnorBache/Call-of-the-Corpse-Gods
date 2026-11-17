// Generated from frozen v48 TypeScript declarations; reviewed boundary schema.
use super::Field;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub type ItemMagicNature = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialStats {
    #[serde(rename = "offense", default, skip_serializing_if = "Field::is_missing")]
    pub offense: Field<f64>,
    #[serde(rename = "defense", default, skip_serializing_if = "Field::is_missing")]
    pub defense: Field<f64>,
    #[serde(
        rename = "accuracy",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub accuracy: Field<f64>,
    #[serde(rename = "dodge", default, skip_serializing_if = "Field::is_missing")]
    pub dodge: Field<f64>,
    #[serde(rename = "dr", default, skip_serializing_if = "Field::is_missing")]
    pub dr: Field<f64>,
    #[serde(
        rename = "perception",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub perception: Field<f64>,
    #[serde(
        rename = "maxEnergy",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub max_energy: Field<f64>,
    #[serde(rename = "maxHp", default, skip_serializing_if = "Field::is_missing")]
    pub max_hp: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEnhancementsEntry {
    #[serde(rename = "craft")]
    pub craft: String,
    #[serde(rename = "bonus")]
    pub bonus: PartialStats,
    #[serde(rename = "active", default, skip_serializing_if = "Field::is_missing")]
    pub active: Field<bool>,
    #[serde(rename = "id", default, skip_serializing_if = "Field::is_missing")]
    pub id: Field<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// R52 birth identity. Missing is always base tier one, including old saves.
    #[serde(rename = "amuletTier", default, skip_serializing_if = "Field::is_missing")]
    pub amulet_tier: Field<f64>,
    /// Explicit R50 amulet resource state; absent until authored by a caller.
    #[serde(rename = "amuletState", default, skip_serializing_if = "Field::is_missing")]
    pub amulet_state: Field<crate::revision50_amulets::AmuletState>,
    /// r39 ground ownership; only a dropped dream-plane Shard carries this marker.
    #[serde(rename = "dreamGround", default, skip_serializing_if = "Field::is_missing")]
    pub dream_ground: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub blessed: Field<bool>,
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "glyph")]
    pub glyph: String,
    #[serde(
        rename = "quantity",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub quantity: Field<f64>,
    #[serde(
        rename = "grantedAbility",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub granted_ability: Field<String>,
    #[serde(rename = "additionalGrantedAbilities", default, skip_serializing_if = "Vec::is_empty")]
    pub additional_granted_abilities: Vec<String>,
    #[serde(
        rename = "tinkered",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub tinkered: Field<bool>,
    #[serde(
        rename = "heartCreatureType",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub heart_creature_type: Field<String>,
    /// Exact source of a heart selected by a Borge event; never a species identity.
    #[serde(rename = "sourceActorId", default, skip_serializing_if = "Field::is_missing")]
    pub source_actor_id: Field<f64>,
    #[serde(rename = "borgeDepth", default, skip_serializing_if = "Field::is_missing")]
    pub borge_depth: Field<f64>,
    #[serde(
        rename = "potionNature",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub potion_nature: Field<String>,
    #[serde(
        rename = "magicNature",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub magic_nature: Field<ItemMagicNature>,
    #[serde(rename = "damage", default, skip_serializing_if = "Field::is_missing")]
    pub damage: Field<f64>,
    #[serde(
        rename = "offenseMultiplier",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub offense_multiplier: Field<f64>,
    #[serde(
        rename = "baseBonus",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub base_bonus: Field<PartialStats>,
    #[serde(
        rename = "enchantment",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub enchantment: Field<f64>,
    #[serde(rename = "hands", default, skip_serializing_if = "Field::is_missing")]
    pub hands: Field<f64>,
    #[serde(rename = "thrown", default, skip_serializing_if = "Field::is_missing")]
    pub thrown: Field<bool>,
    #[serde(
        rename = "rangedWeapon",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub ranged_weapon: Field<String>,
    #[serde(
        rename = "identified",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub identified: Field<bool>,
    #[serde(rename = "diluted", default, skip_serializing_if = "Field::is_missing")]
    pub diluted: Field<bool>,
    #[serde(rename = "effect", default, skip_serializing_if = "Field::is_missing")]
    pub effect: Field<String>,
    #[serde(rename = "bonus", default, skip_serializing_if = "Field::is_missing")]
    pub bonus: Field<PartialStats>,
    #[serde(rename = "energy", default, skip_serializing_if = "Field::is_missing")]
    pub energy: Field<f64>,
    #[serde(rename = "exp", default, skip_serializing_if = "Field::is_missing")]
    pub exp: Field<f64>,
    #[serde(rename = "enhancements")]
    pub enhancements: Vec<ItemEnhancementsEntry>,
    #[serde(
        rename = "equipped",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub equipped: Field<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundItem {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "item")]
    pub item: Item,
    #[serde(
        rename = "autoPickupBlocked",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub auto_pickup_blocked: Field<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunDeath {
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(
        rename = "sourceName",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub source_name: Field<String>,
    #[serde(rename = "cause", default, skip_serializing_if = "Field::is_missing")]
    pub cause: Field<String>,
    #[serde(rename = "depth")]
    pub depth: f64,
    #[serde(rename = "tick")]
    pub tick: f64,
    #[serde(rename = "unpreventable")]
    pub unpreventable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStatistics {
    #[serde(rename = "version")]
    pub version: f64,
    #[serde(rename = "blessingsReceived")]
    pub blessings_received: f64,
    #[serde(rename = "blessingsComplete")]
    pub blessings_complete: bool,
    #[serde(rename = "heartsEaten")]
    pub hearts_eaten: IndexMap<String, f64>,
    #[serde(rename = "heartsComplete")]
    pub hearts_complete: bool,
    #[serde(rename = "death", default, skip_serializing_if = "Field::is_missing")]
    pub death: Field<RunDeath>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunTimeCosts {
    #[serde(rename = "energy")]
    pub energy: f64,
    #[serde(rename = "charges")]
    pub charges: f64,
}

pub type Mode = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceMeterHistory {
    #[serde(rename = "start")]
    pub start: f64,
    #[serde(rename = "max")]
    pub max: f64,
    #[serde(rename = "loss")]
    pub loss: f64,
    #[serde(
        rename = "displayReference",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub display_reference: Field<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTurnHistory {
    #[serde(rename = "tick")]
    pub tick: f64,
    #[serde(rename = "depth")]
    pub depth: f64,
    #[serde(rename = "actors")]
    pub actors: IndexMap<String, IndexMap<String, ResourceMeterHistory>>,
}

pub type ZillenniaQualityId = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorMonsterState {
    #[serde(
        rename = "consumedCorpseIds",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub consumed_corpse_ids: Field<Vec<f64>>,
    #[serde(rename = "pilot", default, skip_serializing_if = "Field::is_missing")]
    pub pilot: Field<Box<Actor>>,
    #[serde(
        rename = "carriedItems",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub carried_items: Field<Vec<Item>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    #[serde(rename = "offense")]
    pub offense: f64,
    #[serde(rename = "defense")]
    pub defense: f64,
    #[serde(rename = "accuracy")]
    pub accuracy: f64,
    #[serde(rename = "dodge")]
    pub dodge: f64,
    #[serde(rename = "dr")]
    pub dr: f64,
    #[serde(rename = "perception")]
    pub perception: f64,
    #[serde(rename = "maxEnergy")]
    pub max_energy: f64,
    #[serde(rename = "maxHp", default, skip_serializing_if = "Field::is_missing")]
    pub max_hp: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorLastSeen {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// Fortune refill deadlines; absent in older saves and initialized when needed.
    #[serde(rename = "fortuneSchedule", default, skip_serializing_if = "Field::is_missing")]
    pub fortune_schedule: Field<crate::turn_schedule::Schedule<()>>,
    #[serde(rename = "reapingLedger", default, skip_serializing_if = "Field::is_missing")]
    pub reaping_ledger: Field<crate::revision50_amulets::ReapingLedger>,
    #[serde(rename = "amuletLedger", default, skip_serializing_if = "Field::is_missing")]
    pub amulet_ledger: Field<crate::revision50_amulets::ActivationLedger>,
    #[serde(rename = "abilityReserves", default, skip_serializing_if = "Field::is_missing")]
    pub ability_reserves: Field<IndexMap<String, crate::revision50_ability_energy::AbilityReserve>>,
    /// Detached Revision 50 Mini-Me item state; never aliases the player's pack.
    #[serde(rename = "copiedInventory", default, skip_serializing_if = "Field::is_missing")]
    pub copied_inventory: Field<Vec<Item>>,
    #[serde(
        rename = "creatureKind",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub creature_kind: Field<String>,
    #[serde(
        rename = "zillenniaQuality",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub zillennia_quality: Field<ZillenniaQualityId>,
    #[serde(
        rename = "monsterState",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub monster_state: Field<ActorMonsterState>,
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "glyph")]
    pub glyph: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "hp")]
    pub hp: f64,
    #[serde(rename = "maxHp")]
    pub max_hp: f64,
    #[serde(
        rename = "baseMaxHp",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub base_max_hp: Field<f64>,
    #[serde(rename = "energy")]
    pub energy: f64,
    #[serde(rename = "stats")]
    pub stats: Stats,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "perfect", default, skip_serializing_if = "Field::is_missing")]
    pub perfect: Field<bool>,
    #[serde(rename = "order", default, skip_serializing_if = "Field::is_missing")]
    pub order: Field<String>,
    #[serde(
        rename = "lastSeen",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub last_seen: Field<ActorLastSeen>,
    #[serde(
        rename = "heartless",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub heartless: Field<bool>,
    #[serde(
        rename = "bornFromBirther",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub born_from_birther: Field<bool>,
    #[serde(rename = "dead", default, skip_serializing_if = "Field::is_missing")]
    pub dead: Field<bool>,
    #[serde(
        rename = "corpseAge",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub corpse_age: Field<f64>,
    #[serde(rename = "abilities")]
    pub abilities: Vec<String>,
    #[serde(
        rename = "equipmentAbilities",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub equipment_abilities: Field<Vec<String>>,
    #[serde(
        rename = "abilityOrder",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub ability_order: Field<Vec<String>>,
    #[serde(rename = "active")]
    pub active: Vec<String>,
    #[serde(rename = "conditions")]
    pub conditions: IndexMap<String, f64>,
    #[serde(rename = "blood", default, skip_serializing_if = "Field::is_missing")]
    pub blood: Field<f64>,
    #[serde(
        rename = "damageBank",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub damage_bank: Field<f64>,
    #[serde(
        rename = "nightmares",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub nightmares: Field<f64>,
    #[serde(
        rename = "reactions",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub reactions: Field<f64>,
    #[serde(
        rename = "adaptiveDr",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub adaptive_dr: Field<f64>,
    #[serde(
        rename = "adaptiveHearts",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub adaptive_hearts: Field<IndexMap<String, f64>>,
}

pub type GasType = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasPresence {
    #[serde(rename = "type")]
    pub r#type: GasType,
    #[serde(rename = "concentration")]
    pub concentration: f64,
}

pub type FireKind = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootObservation {
    #[serde(rename = "glyph")]
    pub glyph: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "active")]
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardSnapshotVent {
    #[serde(rename = "gas")]
    pub gas: GasType,
    #[serde(rename = "countdown")]
    pub countdown: f64,
    #[serde(rename = "phase")]
    pub phase: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardSnapshotFeature {
    #[serde(rename = "glyph")]
    pub glyph: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardSnapshot {
    #[serde(rename = "gas", default, skip_serializing_if = "Field::is_missing")]
    pub gas: Field<GasPresence>,
    #[serde(rename = "fire", default, skip_serializing_if = "Field::is_missing")]
    pub fire: Field<FireKind>,
    #[serde(
        rename = "permanentFire",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub permanent_fire: Field<bool>,
    #[serde(rename = "slime", default, skip_serializing_if = "Field::is_missing")]
    pub slime: Field<bool>,
    #[serde(rename = "mud", default, skip_serializing_if = "Field::is_missing")]
    pub mud: Field<bool>,
    #[serde(rename = "ichor", default, skip_serializing_if = "Field::is_missing")]
    pub ichor: Field<bool>,
    #[serde(
        rename = "razorleaves",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub razorleaves: Field<bool>,
    #[serde(rename = "roots", default, skip_serializing_if = "Field::is_missing")]
    pub roots: Field<bool>,
    #[serde(rename = "root", default, skip_serializing_if = "Field::is_missing")]
    pub root: Field<RootObservation>,
    #[serde(rename = "blood", default, skip_serializing_if = "Field::is_missing")]
    pub blood: Field<bool>,
    #[serde(rename = "bloodyWall", default, skip_serializing_if = "Field::is_missing")]
    pub bloody_wall: Field<bool>,
    /// Revision 33: observed dissolved blood concentration, not a floor stain.
    #[serde(rename = "waterBlood", default, skip_serializing_if = "Field::is_missing")]
    pub water_blood: Field<f64>,
    #[serde(rename = "current", default, skip_serializing_if = "Field::is_missing")]
    pub current: Field<f64>,
    #[serde(rename = "vent", default, skip_serializing_if = "Field::is_missing")]
    pub vent: Field<HazardSnapshotVent>,
    #[serde(rename = "bulwark", default, skip_serializing_if = "Field::is_missing")]
    pub bulwark: Field<f64>,
    #[serde(
        rename = "lightSource",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub light_source: Field<String>,
    #[serde(rename = "feature", default, skip_serializing_if = "Field::is_missing")]
    pub feature: Field<HazardSnapshotFeature>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorSphinxRoom {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "cells")]
    pub cells: Vec<f64>,
    #[serde(rename = "rewardCells")]
    pub reward_cells: Vec<f64>,
}

pub type EncounterDanger = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterRecord {
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "danger")]
    pub danger: EncounterDanger,
    #[serde(rename = "score")]
    pub score: f64,
    #[serde(rename = "floorThreat")]
    pub floor_threat: f64,
    #[serde(rename = "sleeping")]
    pub sleeping: bool,
    #[serde(rename = "actorIds")]
    pub actor_ids: Vec<f64>,
    #[serde(rename = "setupIndices")]
    pub setup_indices: Vec<f64>,
    #[serde(rename = "kind", default, skip_serializing_if = "Field::is_missing")]
    pub kind: Field<String>,
    #[serde(
        rename = "leaderId",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub leader_id: Field<f64>,
    #[serde(
        rename = "followerIds",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub follower_ids: Field<Vec<f64>>,
}

pub type GenerationStage = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationStageRecord {
    #[serde(rename = "id")]
    pub id: GenerationStage,
    #[serde(rename = "tileHash")]
    pub tile_hash: f64,
    #[serde(rename = "openCells")]
    pub open_cells: f64,
    #[serde(rename = "rooms")]
    pub rooms: f64,
    #[serde(rename = "connectors")]
    pub connectors: f64,
    #[serde(rename = "loops")]
    pub loops: f64,
    #[serde(rename = "details", default, skip_serializing_if = "Field::is_missing")]
    pub details: Field<IndexMap<String, f64>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StairRecess {
    #[serde(rename = "at")]
    pub at: f64,
    #[serde(rename = "facing")]
    pub facing: String,
}

pub type GeologicalKind = String;
pub type GeologicalScale = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeologicalLandmarkCenter {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeologicalLandmarkBounds {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "width")]
    pub width: f64,
    #[serde(rename = "height")]
    pub height: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeologicalLandmark {
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(
        rename = "parentId",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub parent_id: Field<f64>,
    #[serde(rename = "kind")]
    pub kind: GeologicalKind,
    #[serde(rename = "scale")]
    pub scale: GeologicalScale,
    #[serde(rename = "center")]
    pub center: GeologicalLandmarkCenter,
    #[serde(rename = "bounds")]
    pub bounds: GeologicalLandmarkBounds,
    #[serde(rename = "cells")]
    pub cells: Vec<f64>,
    #[serde(rename = "roomIds")]
    pub room_ids: Vec<f64>,
    #[serde(rename = "connectorIds")]
    pub connector_ids: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationMetadataTerrain {
    #[serde(rename = "version")]
    pub version: f64,
    #[serde(
        rename = "archetype",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub archetype: Field<String>,
    #[serde(rename = "orientation")]
    pub orientation: String,
    #[serde(rename = "phase")]
    pub phase: f64,
    #[serde(rename = "changedCells")]
    pub changed_cells: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationMetadata {
    #[serde(rename = "version")]
    pub version: f64,
    #[serde(rename = "stages")]
    pub stages: Vec<GenerationStageRecord>,
    #[serde(
        rename = "stairRecesses",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub stair_recesses: Field<Vec<StairRecess>>,
    #[serde(
        rename = "landmarks",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub landmarks: Field<Vec<GeologicalLandmark>>,
    #[serde(
        rename = "naturalWalls",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub natural_walls: Field<Vec<f64>>,
    #[serde(
        rename = "terrainSpine",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub terrain_spine: Field<Vec<f64>>,
    #[serde(rename = "terrain", default, skip_serializing_if = "Field::is_missing")]
    pub terrain: Field<GenerationMetadataTerrain>,
}

pub type DungeonRoomKind = String;
pub type RoomSizeClass = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DungeonPoint {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomMetadataBounds {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "width")]
    pub width: f64,
    #[serde(rename = "height")]
    pub height: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomMetadata {
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "kind")]
    pub kind: DungeonRoomKind,
    #[serde(rename = "cells")]
    pub cells: Vec<f64>,
    #[serde(
        rename = "sizeClass",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub size_class: Field<RoomSizeClass>,
    #[serde(rename = "center")]
    pub center: DungeonPoint,
    #[serde(rename = "bounds")]
    pub bounds: RoomMetadataBounds,
    #[serde(rename = "parentRoomId")]
    pub parent_room_id: Option<f64>,
    #[serde(rename = "connectorId")]
    pub connector_id: Option<f64>,
    #[serde(rename = "doors")]
    pub doors: Vec<f64>,
}

pub type PassageStyle = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorMetadata {
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "style", default, skip_serializing_if = "Field::is_missing")]
    pub style: Field<PassageStyle>,
    #[serde(rename = "path", default, skip_serializing_if = "Field::is_missing")]
    pub path: Field<Vec<f64>>,
    #[serde(rename = "cells")]
    pub cells: Vec<f64>,
    #[serde(rename = "roomIds")]
    pub room_ids: Vec<f64>,
    #[serde(rename = "doors")]
    pub doors: Vec<f64>,
    #[serde(rename = "parentRoomId")]
    pub parent_room_id: f64,
    #[serde(
        rename = "parentConnectorId",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub parent_connector_id: Field<f64>,
    #[serde(rename = "purpose")]
    pub purpose: String,
    #[serde(
        rename = "routeBefore",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub route_before: Field<f64>,
    #[serde(
        rename = "routeAfter",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub route_after: Field<f64>,
    #[serde(
        rename = "routeEndpoints",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub route_endpoints: Field<(f64, f64)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorRoomGraph {
    #[serde(rename = "rooms")]
    pub rooms: Vec<RoomMetadata>,
    #[serde(rename = "connectors")]
    pub connectors: Vec<ConnectorMetadata>,
}

pub type EnemyFamily = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorRememberedDoorsEntry {
    #[serde(rename = "open")]
    pub open: bool,
    #[serde(rename = "burning", default, skip_serializing_if = "Field::is_missing")]
    pub burning: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorDoorsEntry {
    #[serde(rename = "open")]
    pub open: bool,
    #[serde(rename = "burning", default, skip_serializing_if = "Field::is_missing")]
    pub burning: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasVent {
    #[serde(rename = "at")]
    pub at: f64,
    #[serde(rename = "gas")]
    pub gas: GasType,
    #[serde(rename = "countdown")]
    pub countdown: f64,
    #[serde(rename = "phase")]
    pub phase: String,
    #[serde(rename = "pulse", default, skip_serializing_if = "Field::is_missing")]
    pub pulse: Field<IndexMap<String, f64>>,
    #[serde(rename = "amount", default, skip_serializing_if = "Field::is_missing")]
    pub amount: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FungalRoot {
    #[serde(rename = "fungusId")]
    pub fungus_id: f64,
    #[serde(rename = "corpseId")]
    pub corpse_id: f64,
    #[serde(rename = "path")]
    pub path: Vec<f64>,
    #[serde(rename = "active")]
    pub active: bool,
    #[serde(rename = "color")]
    pub color: String,
    /// Absent on historical and already-grown links. The path remains the
    /// original direct route; only its grown prefix is physical root material.
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub growth: Field<FungalRootGrowth>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FungalRootGrowth {
    pub grown: usize,
    pub turns: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorSetupSpawnsEntry {
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(
        rename = "setupIndex",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub setup_index: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupBounds {
    #[serde(rename = "left")]
    pub left: f64,
    #[serde(rename = "right")]
    pub right: f64,
    #[serde(rename = "top")]
    pub top: f64,
    #[serde(rename = "bottom")]
    pub bottom: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorSetupsEntry {
    #[serde(rename = "id")]
    pub id: f64,
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "bounds", default, skip_serializing_if = "Field::is_missing")]
    pub bounds: Field<SetupBounds>,
    #[serde(
        rename = "footprint",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub footprint: Field<Vec<f64>>,
}

pub type Substrate = String;
pub type Tile = String;
pub type GodbeastKind = String;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GodbeastEventRoom {
    #[serde(rename = "left")]
    pub left: f64,
    #[serde(rename = "top")]
    pub top: f64,
    #[serde(rename = "right")]
    pub right: f64,
    #[serde(rename = "bottom")]
    pub bottom: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GodbeastEventWell {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GodbeastEventLever {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GodbeastEventTap {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GodbeastEvent {
    #[serde(rename="epiditeAge", default, skip_serializing_if = "Field::is_missing")]
    pub epidite_age: Field<f64>,
    #[serde(rename = "kind")]
    pub kind: GodbeastKind,
    #[serde(rename = "displayName", default, skip_serializing_if = "Field::is_missing")]
    pub display_name: Field<String>,
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "room")]
    pub room: GodbeastEventRoom,
    #[serde(rename = "well", default, skip_serializing_if = "Field::is_missing")]
    pub well: Field<GodbeastEventWell>,
    #[serde(rename = "lever", default, skip_serializing_if = "Field::is_missing")]
    pub lever: Field<GodbeastEventLever>,
    #[serde(rename = "tap", default, skip_serializing_if = "Field::is_missing")]
    pub tap: Field<GodbeastEventTap>,
    #[serde(rename = "altar", default, skip_serializing_if = "Field::is_missing")]
    pub altar: Field<GodbeastEventTap>,
    #[serde(rename = "sacrificeActorId", default, skip_serializing_if = "Field::is_missing")]
    pub sacrifice_actor_id: Field<f64>,
    #[serde(rename = "sacrificeName", default, skip_serializing_if = "Field::is_missing")]
    pub sacrifice_name: Field<String>,
    #[serde(
        rename = "activated",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub activated: Field<bool>,
    #[serde(
        rename = "rewardCreated",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub reward_created: Field<bool>,
    #[serde(rename = "tapped", default, skip_serializing_if = "Field::is_missing")]
    pub tapped: Field<bool>,
    #[serde(
        rename = "floodAge",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub flood_age: Field<f64>,
    #[serde(
        rename = "floodFrontier",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub flood_frontier: Field<Vec<f64>>,
    #[serde(
        rename = "floodVisited",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub flood_visited: Field<Vec<f64>>,
    #[serde(
        rename = "fishSpawned",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub fish_spawned: Field<f64>,
    #[serde(
        rename = "nextFishAge",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub next_fish_age: Field<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorLightSourcesEntry {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "power")]
    pub power: f64,
    #[serde(rename = "falloff")]
    pub falloff: f64,
    #[serde(rename = "kind", default, skip_serializing_if = "Field::is_missing")]
    pub kind: Field<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorAlteredChasmsEntry {
    #[serde(rename = "tile")]
    pub tile: Tile,
    #[serde(rename = "age")]
    pub age: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentalStateProvenanceEntry {
    #[serde(rename = "owner")]
    pub owner: Vec<f64>,
    #[serde(rename = "player")]
    pub player: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentalState {
    #[serde(rename = "bloodMist", default, skip_serializing_if = "Field::is_missing")]
    pub blood_mist: Field<Vec<f64>>,
    #[serde(rename = "bloodMistTurns", default, skip_serializing_if = "Field::is_missing")]
    pub blood_mist_turns: Field<Vec<f64>>,

    #[serde(rename = "gasType", default, skip_serializing_if = "Field::is_missing")]
    pub gas_type: Field<Vec<Option<GasType>>>,
    #[serde(
        rename = "gasVolume",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub gas_volume: Field<Vec<f64>>,
    #[serde(
        rename = "gasOwner",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub gas_owner: Field<Vec<f64>>,
    #[serde(
        rename = "gasPlayer",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub gas_player: Field<Vec<f64>>,
    #[serde(
        rename = "fireKind",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub fire_kind: Field<Vec<Option<FireKind>>>,
    #[serde(
        rename = "fireFresh",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub fire_fresh: Field<Vec<bool>>,
    #[serde(
        rename = "fireReaction",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub fire_reaction: Field<Vec<f64>>,
    #[serde(
        rename = "nextFireReaction",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub next_fire_reaction: Field<f64>,
    #[serde(
        rename = "provenance",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub provenance: Field<IndexMap<String, EnvironmentalStateProvenanceEntry>>,
    #[serde(
        rename = "confusion",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub confusion: Field<Vec<f64>>,
    #[serde(rename = "hallucinogenic", default, skip_serializing_if = "Field::is_missing")]
    pub hallucinogenic: Field<Vec<f64>>,
    #[serde(rename = "steam", default, skip_serializing_if = "Field::is_missing")]
    pub steam: Field<Vec<f64>>,
    #[serde(
        rename = "flammable",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub flammable: Field<Vec<f64>>,
    #[serde(
        rename = "explosive",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub explosive: Field<Vec<f64>>,
    #[serde(
        rename = "hydrogen",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub hydrogen: Field<Vec<f64>>,
    #[serde(rename = "caustic")]
    pub caustic: Vec<f64>,
    #[serde(rename = "breather")]
    pub breather: Vec<f64>,
    #[serde(rename = "paralysis")]
    pub paralysis: Vec<f64>,
    #[serde(rename = "fire")]
    pub fire: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Floor {
    #[serde(rename = "revision50Darkness", default, skip_serializing_if = "Field::is_missing")]
    pub revision50_darkness: Field<Vec<crate::revision50_potions::Darkness>>,
    #[serde(rename = "revision50TemporaryTerrain", default, skip_serializing_if = "Field::is_missing")]
    pub revision50_temporary_terrain: Field<Vec<crate::revision50_control_amulets::TemporaryTerrain>>,
    #[serde(rename = "revision50Terrain", default, skip_serializing_if = "Field::is_missing")]
    pub revision50_terrain: Field<Vec<crate::revision50_terrain::Modification>>,
    /// Revision 51: Speed-up topology is active only while its terrain exists.
    #[serde(rename = "speedUpTerrain", default, skip_serializing_if = "Field::is_missing")]
    pub speed_up_terrain: Field<bool>,
    #[serde(rename = "speedUp", default, skip_serializing_if = "Field::is_missing")]
    pub speed_up: Field<Vec<crate::non_euclidean_generation::NonEuclideanPocket>>,
    #[serde(rename = "nonEuclidean", default, skip_serializing_if = "Field::is_missing")]
    pub non_euclidean: Field<Vec<crate::non_euclidean_generation::NonEuclideanPocket>>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub portals: Field<Vec<crate::portals_generation::PortalPair>>,
    #[serde(rename = "pocketBiomes", default, skip_serializing_if = "Field::is_missing")]
    pub pocket_biomes: Field<Vec<crate::pocket_biomes::PocketBiome>>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub entrance: Field<crate::entrance::Entrance>,
    #[serde(
        rename = "rememberedHazards",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_hazards: Field<IndexMap<String, HazardSnapshot>>,
    #[serde(
        rename = "monsterEntryTick",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub monster_entry_tick: Field<f64>,
    #[serde(rename = "ice", default, skip_serializing_if = "Field::is_missing")]
    pub ice: Field<Vec<bool>>,
    #[serde(rename = "puddles", default, skip_serializing_if = "Field::is_missing")]
    pub puddles: Field<Vec<bool>>,
    #[serde(rename = "hellfire", default, skip_serializing_if = "Field::is_missing")]
    pub hellfire: Field<Vec<bool>>,
    #[serde(rename = "rememberedIce", default, skip_serializing_if = "Field::is_missing")]
    pub remembered_ice: Field<Vec<bool>>,
    #[serde(rename = "rememberedPuddles", default, skip_serializing_if = "Field::is_missing")]
    pub remembered_puddles: Field<Vec<bool>>,
    #[serde(rename = "rememberedHellfire", default, skip_serializing_if = "Field::is_missing")]
    pub remembered_hellfire: Field<Vec<bool>>,
    #[serde(rename = "snow", default, skip_serializing_if = "Field::is_missing")]
    pub snow: Field<Vec<bool>>,
    #[serde(
        rename = "rememberedSnow",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_snow: Field<Vec<bool>>,
    #[serde(
        rename = "sphinxRoom",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub sphinx_room: Field<FloorSphinxRoom>,
    #[serde(rename = "width", default, skip_serializing_if = "Field::is_missing")]
    pub width: Field<f64>,
    #[serde(rename = "height", default, skip_serializing_if = "Field::is_missing")]
    pub height: Field<f64>,
    #[serde(
        rename = "encounters",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub encounters: Field<Vec<EncounterRecord>>,
    #[serde(
        rename = "enemyBlessingBudget",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub enemy_blessing_budget: Field<f64>,
    #[serde(
        rename = "generation",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub generation: Field<GenerationMetadata>,
    #[serde(
        rename = "roomGraph",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub room_graph: Field<FloorRoomGraph>,
    #[serde(
        rename = "basicVentCells",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub basic_vent_cells: Field<Vec<f64>>,
    #[serde(
        rename = "generationBridges",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub generation_bridges: Field<Vec<f64>>,
    #[serde(
        rename = "dungeonRooms",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub dungeon_rooms: Field<Vec<RoomMetadata>>,
    #[serde(
        rename = "dungeonConnectors",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub dungeon_connectors: Field<Vec<ConnectorMetadata>>,
    #[serde(
        rename = "currents",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub currents: Field<Vec<f64>>,
    #[serde(
        rename = "rememberedCurrents",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_currents: Field<Vec<f64>>,
    #[serde(
        rename = "rememberedIchor",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_ichor: Field<Vec<f64>>,
    #[serde(
        rename = "enemyFamilies",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub enemy_families: Field<Vec<EnemyFamily>>,
    #[serde(rename = "mud", default, skip_serializing_if = "Field::is_missing")]
    pub mud: Field<Vec<f64>>,
    #[serde(
        rename = "rememberedDoors",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_doors: Field<IndexMap<String, FloorRememberedDoorsEntry>>,
    #[serde(rename = "doors", default, skip_serializing_if = "Field::is_missing")]
    pub doors: Field<IndexMap<String, FloorDoorsEntry>>,
    #[serde(rename = "grates", default, skip_serializing_if = "Field::is_missing")]
    pub grates: Field<Vec<f64>>,
    #[serde(
        rename = "permanentFire",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub permanent_fire: Field<Vec<f64>>,
    #[serde(
        rename = "bulwarks",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub bulwarks: Field<crate::deadline_map::DeadlineMap>,
    #[serde(rename = "vents", default, skip_serializing_if = "Field::is_missing")]
    pub vents: Field<Vec<GasVent>>,
    #[serde(
        rename = "fungalRoots",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub fungal_roots: Field<Vec<FungalRoot>>,
    #[serde(rename = "roots", default, skip_serializing_if = "Field::is_missing")]
    pub roots: Field<Vec<f64>>,
    #[serde(
        rename = "setupSpawns",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub setup_spawns: Field<Vec<FloorSetupSpawnsEntry>>,
    #[serde(rename = "setups", default, skip_serializing_if = "Field::is_missing")]
    pub setups: Field<Vec<FloorSetupsEntry>>,
    #[serde(
        rename = "substrates",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub substrates: Field<Vec<Substrate>>,
    #[serde(
        rename = "rememberedTiles",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_tiles: Field<Vec<Tile>>,
    #[serde(
        rename = "rememberedSubstrates",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_substrates: Field<Vec<Substrate>>,
    #[serde(rename = "slime", default, skip_serializing_if = "Field::is_missing")]
    pub slime: Field<Vec<f64>>,
    #[serde(
        rename = "razorleaves",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub razorleaves: Field<Vec<f64>>,
    #[serde(rename = "blood", default, skip_serializing_if = "Field::is_missing")]
    pub blood: Field<Vec<bool>>,
    /// Edible wall overlay; actual terrain remains wall for collision/LOS/fire.
    #[serde(rename = "bloodyWalls", default, skip_serializing_if = "Field::is_missing")]
    pub bloody_walls: Field<Vec<bool>>,
    /// Revision 33: bounded dissolved blood; absent in legacy runs.
    #[serde(rename = "waterBlood", default, skip_serializing_if = "Field::is_missing")]
    pub water_blood: Field<Vec<f64>>,
    /// Permanent Kelpie spawnpoint sources, independent of living actors.
    #[serde(rename = "kelpieBloodSources", default, skip_serializing_if = "Field::is_missing")]
    pub kelpie_blood_sources: Field<Vec<f64>>,
    /// Revision 33: authored river-landmark water sources for the feeding cycle.
    #[serde(rename = "bloodRiverSources", default, skip_serializing_if = "Field::is_missing")]
    pub blood_river_sources: Field<Vec<f64>>,
    #[serde(rename = "ichor", default, skip_serializing_if = "Field::is_missing")]
    pub ichor: Field<Vec<f64>>,
    #[serde(
        rename = "godbeastEvent",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub godbeast_event: Field<GodbeastEvent>,
    #[serde(
        rename = "lightSources",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub light_sources: Field<Vec<FloorLightSourcesEntry>>,
    #[serde(
        rename = "alteredChasms",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub altered_chasms: Field<IndexMap<String, FloorAlteredChasmsEntry>>,
    #[serde(
        rename = "environment",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub environment: Field<EnvironmentalState>,
    #[serde(rename = "depth")]
    pub depth: f64,
    #[serde(rename = "domain")]
    pub domain: String,
    #[serde(rename = "tiles")]
    pub tiles: Vec<Tile>,
    #[serde(rename = "lights")]
    pub lights: Vec<f64>,
    #[serde(rename = "seen")]
    pub seen: Vec<bool>,
    #[serde(rename = "godbeast")]
    pub godbeast: bool,
    #[serde(rename = "dirty")]
    pub dirty: bool,
    #[serde(rename = "lastTick")]
    pub last_tick: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunGroundEntryEntry {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "item")]
    pub item: Item,
    #[serde(
        rename = "autoPickupBlocked",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub auto_pickup_blocked: Field<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRally {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogArchive {
    #[serde(rename = "version")]
    pub version: f64,
    #[serde(rename = "messages")]
    pub messages: Vec<String>,
    #[serde(rename = "events")]
    pub events: Vec<f64>,
    #[serde(rename = "incomplete")]
    pub incomplete: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRememberedItemsEntryEntry {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "item")]
    pub item: Item,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunDetectedItemsEntryEntry {
    #[serde(rename = "x")]
    pub x: f64,
    #[serde(rename = "y")]
    pub y: f64,
    #[serde(rename = "item")]
    pub item: Item,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Derived geometry cache; excluded from saves and rebuilt after cloning.
    #[serde(skip)]
    pub optical_cache: crate::optical_cache::OpticalCache,
    /// Derived immutable-view lighting cache, rebuilt after load/clone.
    #[serde(skip)]
    pub lighting_cache: crate::lighting_cache::LightingCache,
    /// Derived health bookkeeping; not canonical save state.
    #[serde(skip)]
    pub health_dirty: crate::health_dirty::HealthDirty,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub dreamwalking: Field<crate::dreamwalking::State>,
    /// Chronological Timeweave with derived cell index; Arc snapshots share until mutation.
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub timeweave: Field<Arc<crate::timeweave::Trail>>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub collapse: Field<std::sync::Arc<crate::finale::Collapse>>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub playtest: Field<crate::playtest::Profile>,
    #[serde(rename = "runId", default, skip_serializing_if = "Field::is_missing")]
    pub run_id: Field<String>,
    #[serde(
        rename = "runStats",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub run_stats: Field<RunStatistics>,
    #[serde(
        rename = "achievements",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub achievements: Field<Vec<String>>,
    #[serde(
        rename = "veridianSapHarvested",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub veridian_sap_harvested: Field<bool>,
    /// Revision 51 lifetime generation budget, independent of current ownership.
    #[serde(rename = "compositeBowGenerated", default, skip_serializing_if = "Field::is_missing")]
    pub composite_bow_generated: Field<bool>,
    #[serde(rename = "version")]
    pub version: f64,
    #[serde(
        rename = "gameplayRevision",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub gameplay_revision: Field<f64>,
    #[serde(
        rename = "divineRevision",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub divine_revision: Field<f64>,
    #[serde(
        rename = "enemyHealthRevision",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub enemy_health_revision: Field<f64>,
    #[serde(
        rename = "simulationRevision",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub simulation_revision: Field<f64>,
    #[serde(
        rename = "resolvingTick",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub resolving_tick: Field<bool>,
    #[serde(
        rename = "timeCosts",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub time_costs: Field<RunTimeCosts>,
    #[serde(rename = "phase", default, skip_serializing_if = "Field::is_missing")]
    pub phase: Field<String>,
    #[serde(rename = "fastActionPhase", default, skip_serializing_if = "Field::is_missing")]
    pub fast_action_phase: Field<crate::model::FastActionPhase>,
    #[serde(
        rename = "hasteSnapshot",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub haste_snapshot: Field<String>,
    #[serde(rename = "seed")]
    pub seed: String,
    #[serde(rename = "rng")]
    pub rng: f64,
    #[serde(rename = "mode")]
    pub mode: Mode,
    /// Opt-in accelerated descent. Missing in all historical normal saves.
    #[serde(rename = "fastMode", default, skip_serializing_if = "Field::is_missing")]
    pub fast_mode: Field<bool>,
    /// Twenty-floor route; absent preserves historical save/replay coordinates.
    #[serde(rename = "compressedDepths", default, skip_serializing_if = "Field::is_missing")]
    pub compressed_depths: Field<bool>,
    #[serde(rename = "gods")]
    pub gods: Vec<f64>,
    #[serde(rename = "tick")]
    pub tick: f64,
    #[serde(
        rename = "damageSinceLastTurn",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub damage_since_last_turn: Field<f64>,
    #[serde(
        rename = "damageSinceLastTurnMaxHp",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub damage_since_last_turn_max_hp: Field<f64>,
    #[serde(
        rename = "resourceHistory",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub resource_history: Field<ResourceTurnHistory>,
    #[serde(rename = "depth")]
    pub depth: f64,
    #[serde(rename = "level")]
    pub level: f64,
    #[serde(rename = "exp")]
    pub exp: f64,
    #[serde(rename = "player")]
    pub player: Actor,
    #[serde(
        rename = "initialPlayer",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub initial_player: Field<Actor>,
    #[serde(rename = "inventory")]
    pub inventory: Vec<Item>,
    #[serde(
        rename = "inventoryDisplayOrder",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub inventory_display_order: Field<Vec<f64>>,
    #[serde(rename = "floors")]
    pub floors: Vec<Arc<Floor>>,
    #[serde(rename = "actors")]
    pub actors: crate::creature_index::ActorFloors,
    #[serde(rename = "ground")]
    pub ground: IndexMap<String, Arc<Vec<GroundItem>>>,
    #[serde(rename = "rally")]
    pub rally: Option<RunRally>,
    #[serde(rename = "log")]
    pub log: Vec<String>,
    #[serde(
        rename = "logKinds",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub log_kinds: Field<Vec<String>>,
    #[serde(
        rename = "logArchive",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub log_archive: Field<LogArchive>,
    #[serde(rename = "nextId")]
    pub next_id: f64,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "offers")]
    pub offers: Vec<String>,
    #[serde(rename = "history")]
    pub history: Vec<String>,
    #[serde(rename = "future")]
    pub future: Option<crate::future_anchor::FutureAnchor>,
    #[serde(rename = "replay")]
    pub replay: Vec<super::Command>,
    /// Seed-ledger policy administration: Missing legacy; Value(N) switches to
    /// snapshot Revert before command N; Null explicitly unknown old trial provenance.
    #[serde(rename = "replaySnapshotFrom", default, skip_serializing_if = "Field::is_missing")]
    pub replay_snapshot_from: Field<usize>,
    /// Firebolt-only command boundary; Null is an explicit legacy playback prefix.
    #[serde(rename = "fireboltTravelFrom", default, skip_serializing_if = "Field::is_missing")]
    pub firebolt_travel_from: Field<usize>,
    /// Amulet rules command boundary; Null denotes a legacy replay prefix.
    #[serde(rename = "amuletRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub amulet_rules_from: Field<usize>,
    #[serde(rename = "metrics")]
    pub metrics: IndexMap<String, f64>,
    #[serde(rename = "dev")]
    pub dev: bool,
    #[serde(rename = "knownPotions")]
    pub known_potions: Vec<String>,
    #[serde(
        rename = "generatedAbilityGrants",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub generated_ability_grants: Field<Vec<String>>,
    #[serde(
        rename = "knownPotionNatures",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub known_potion_natures: Field<IndexMap<String, String>>,
    #[serde(
        rename = "rememberedItems",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub remembered_items: Field<IndexMap<String, Vec<RunRememberedItemsEntryEntry>>>,
    #[serde(
        rename = "detectedItems",
        default,
        skip_serializing_if = "Field::is_missing"
    )]
    pub detected_items: Field<IndexMap<String, Vec<RunDetectedItemsEntryEntry>>>,
}
