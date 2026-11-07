//! Revisioned whole-life numerical overlays; frozen rewards and placement stay separate.
//! Only newly created actors use this overlay. Saved actors are never retuned on load.
use crate::Actor;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct PhysicalTuning {
    pub kind: String,
    pub reference_depth: f64,
    pub target_mean_loss: f64,
    pub offense: f64,
    pub defense: f64,
    pub accuracy: f64,
    pub dodge: f64,
    pub dr: f64,
    pub qualification: String,
    #[serde(default)]
    pub gas_passive: Option<f64>,
    #[serde(default)]
    pub gas_boost: Option<f64>,
    #[serde(default)]
    pub gas_shot: Option<f64>,
    #[serde(default)]
    pub validation_model: String,
}

pub fn tuning() -> &'static [PhysicalTuning] {
    static TUNING: OnceLock<Vec<PhysicalTuning>> = OnceLock::new();
    TUNING.get_or_init(|| {
        serde_json::from_str(include_str!("catalog/physical-balance-v31.json"))
            .expect("provisional physical balance metadata")
    })
}

/// Current revision overlays are separate data, leaving every prior replay unchanged.
/// An empty revision32 catalog during authoring falls back to the frozen revision31 row.
pub fn tuning_revision(revision: f64) -> &'static [PhysicalTuning] {
    if revision < 32.0 { return tuning(); }
    static TUNING: OnceLock<Vec<PhysicalTuning>> = OnceLock::new();
    TUNING.get_or_init(|| {
        serde_json::from_str(include_str!("catalog/physical-balance-v32.json"))
            .expect("revision32 monster balance metadata")
    })
}

pub fn reference_depth_revision(kind: &str, revision: f64) -> Option<f64> {
    tuning_revision(revision).iter().find(|t| t.kind == kind)
        .map(|t| t.reference_depth).or_else(|| reference_depth(kind))
}

/// A balance reference is not a spawn bound. Earliest archetypes deliberately use floor1.
/// The O(T + K + P) scan is over the fixed ordinary roster and two paired species, no RNG.
pub fn reference_depth(kind: &str) -> Option<f64> {
    if let Some(t) = tuning().iter().find(|t| t.kind == kind) {
        return Some(t.reference_depth);
    }
    if matches!(kind, "blorb" | "rat" | "cave-bat" | "shellback-beetle"
        | "hemogoblin" | "needleling" | "pale:Jackal"
        | "bastidion" | "fungus" | "vent-fungus") {
        return Some(1.0);
    }
    let leader = crate::population::catalog().pairs.iter()
        .find(|(_, follower)| follower.as_str() == kind)
        .map_or(kind, |(leader, _)| leader.as_str());
    crate::ordinary_roster::species().iter().find(|s| s.id == leader)
        .map(|s| (s.first + s.last) as f64 / 2.0)
}

/// Apply once at a new-actor construction boundary, before damage, growth or buffs.
/// O(T), T fixed tuning records; no RNG, reward, ability or spawn-policy mutation.
pub fn apply_new_actor(actor: &mut Actor, revision: f64) {
    if ![31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0].contains(&revision) { return; }
    let Some(kind) = actor.creature_kind.as_ref().cloned() else { return; };
    apply_new_actor_kind(actor, &kind, revision);
}

/// Explicit construction key supports legacy Pale templates without creatureKind.
/// Identity, rewards, traits and saved actor representation remain unchanged.
pub fn apply_new_actor_kind(actor: &mut Actor, kind: &str, revision: f64) {
    apply_legacy_new_actor_kind(actor, kind, revision);
    crate::accuracy_dodge::apply_new_actor_kind(actor, kind, revision);
    crate::balance42::apply_new_actor_kind(actor, kind, revision);
}

fn apply_legacy_new_actor_kind(actor: &mut Actor, kind: &str, revision: f64) {
    if ![31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0].contains(&revision) { return; }
    let Some(t) = tuning_revision(revision).iter().find(|t| t.kind == kind)
        .or_else(|| tuning().iter().find(|t| t.kind == kind)) else { return; };
    actor.stats.offense = t.offense;
    actor.stats.defense = t.defense;
    actor.stats.accuracy = t.accuracy;
    actor.stats.dodge = t.dodge;
    actor.stats.dr = t.dr;
    actor.hp = 100.0 + 6.0 * t.defense;
    actor.max_hp = actor.hp;
    for (key, volume) in [
        ("monsterGasPassiveVolume", t.gas_passive),
        ("monsterGasBoostVolume", t.gas_boost),
        ("monsterGasShotVolume", t.gas_shot),
    ] {
        if let Some(volume) = volume { actor.conditions.insert(key.into(), volume); }
    }
}

/// Fixed birth-only profiles, distinct from natural species. Existing identity,
/// abilities, energy costs and rewards survive; actual actor stats are serialized.
/// O(T) lookup once per child; no depth-dependent rescaling and no RNG.
pub fn apply_summoned_actor(actor: &mut Actor, parent_greater: bool, child_kind: &str, revision: f64) {
    if revision < 32.0 { return; }
    static SUMMONED: OnceLock<Vec<PhysicalTuning>> = OnceLock::new();
    let rows = SUMMONED.get_or_init(|| serde_json::from_str(include_str!("catalog/summoned-balance-v32.json"))
        .expect("revision32 summoned creature tuning"));
    let key = format!("{}:{child_kind}", if parent_greater { "greater-birther" } else { "birther" });
    let Some(t) = rows.iter().find(|t| t.kind == key) else { return; };
    actor.stats.offense = t.offense;
    actor.stats.defense = t.defense;
    actor.stats.accuracy = t.accuracy;
    actor.stats.dodge = t.dodge;
    actor.stats.dr = t.dr;
    actor.hp = 100.0 + 6.0 * t.defense;
    actor.max_hp = actor.hp;
    for (key, volume) in [
        ("monsterGasPassiveVolume", t.gas_passive),
        ("monsterGasBoostVolume", t.gas_boost),
        ("monsterGasShotVolume", t.gas_shot),
    ] {
        if let Some(volume) = volume { actor.conditions.insert(key.into(), volume); }
    }
    crate::balance42::apply_new_actor_kind(actor, &key, revision);
}
