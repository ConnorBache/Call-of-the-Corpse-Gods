//! Four authored divine combat hooks, without attack recursion or hidden RNG.
//! Actor-ID lookup is O(A), ability predicates O(K), and clone synchronization O(A).
//! Callers preserve source ordering: damage, live-target debuff, then melee healing.
use crate::{Actor, Run, Stats, combat, engine, half_here};
pub const FIGHT_LONG: &str = "shrail-fight-long";
pub const BEAT_TO_DEATH: &str = "shrail-beat-to-death";
pub const HARDENED_BLOOD: &str = "lichor-hardened-blood";
pub const BLOOD_WEAPON: &str = "lichor-blood-weapon";
fn js_round(n: f64) -> f64 {
    let floor = n.floor();
    floor + if n - floor >= 0.5 { 1.0 } else { 0.0 }
}
fn has(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|s| s == id)
}
pub fn uses_enemy_health(r: &Run, a: &Actor) -> bool {
    a.id != r.player.id
        && engine::condition(a, "miniMe") != 1.0
        && engine::condition(a, "halfHereClone") != 1.0
}
/// Accepts already-derived stats, so Hardened Blood never recursively derives itself.
pub fn health_from_stats(r: &Run, a: &Actor, s: &Stats) -> f64 {
    let owner = engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a);
    if owner.id != a.id {
        return health_from_stats(r, owner, &combat::effective_stats(r, owner));
    }
    if uses_enemy_health(r, owner) {
        return 100.0 + 6.0 * js_round(s.defense);
    }
    let baseline = owner.base_max_hp.value_or(if owner.id == r.player.id {
        100.0
    } else {
        a.max_hp - 6.0 * a.stats.defense - a.stats.max_hp.value_or(0.0)
    });
    (baseline + 6.0 * s.defense + s.max_hp.value_or(0.0)).max(1.0)
}
/// Apply after equipment and enemy-health stat normalization, before energy caps.
pub fn apply_stat_modifiers(r: &Run, a: &Actor, s: &mut Stats) {
    let beaten = if r.simulation_revision.value_or(29.) >= 50. { 0. } else {
        combat::integer_stat(engine::condition(a, "beatToDeathHits")).max(0.0)
    };
    s.dodge -= beaten;
    s.dr -= 5.0 * beaten;
    if has(a, HARDENED_BLOOD) {
        let owner = engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a);
        s.dr += ((health_from_stats(r, a, s) - owner.hp).max(0.0) / 10.0).ceil();
    }
}
pub fn blood_weapon_hp_cost(a: &Actor) -> f64 {
    combat::tuned_damage(a.max_hp / 10.0, None)
}
pub fn blood_weapon_hp_cost_for_run(r: &Run, a: &Actor) -> f64 {
    combat::tuned_damage_for_run(r, a.max_hp / 10.0, None)
}
pub fn blood_weapon_ready(r: &Run, a: &Actor) -> bool {
    let owner = engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a);
    has(owner, BLOOD_WEAPON) && engine::condition(owner, "bloodWeaponReady") > 0.0
}
/// Call only after the source attack has passed pre-roll blocks; misses consume it too.
pub fn consume_blood_weapon(r: &mut Run, a: &Actor) {
    let owner = half_here::owner_id(r, a);
    if let Some(owner) = engine::actor_mut_trusted(r, owner) {
        owner.conditions.shift_remove("bloodWeaponReady");
    }
}
/// A zero-damage landed strike still debuffs a living target; actual death clears it.
pub fn beat_after_landed_hit(r: &mut Run, a: &Actor, target_id: f64) {
    if r.simulation_revision.value_or(29.) >= 50. || !has(a, BEAT_TO_DEATH) {
        return;
    }
    if let Some(target) = engine::actor_mut_trusted(r, target_id) {
        if target.hp > 0.0 && !target.dead.value_or(false) {
            *target
                .conditions
                .entry("beatToDeathHits".into())
                .or_default() += 1.0;
        }
    }
}
pub fn healing_suppressed(r: &Run, a: &Actor) -> bool {
    crate::divine_environment::healing_suppressed(r,a)
}
/// Pass gross HP loss capped by pre-damage owner HP, never overkill or post-rescue loss.
/// Healing retains source fractions. The whole-number restriction applies to damage.
pub fn heal_melee_damage(r: &mut Run, a: &Actor, actual_hp_damage: f64, melee: bool) {
    if !melee || actual_hp_damage <= 0.0 || healing_suppressed(r, a) { return; }
    let fight_long = has(a, FIGHT_LONG);
    let vampirism = crate::revision50_rings::has_effect(r, a, crate::revision50_rings::VAMPIRISM);
    // Preserve historical division exactly: multiplying by 0.2 changes saved
    // floating-point traces even though the two expressions are mathematically equal.
    let healing = match (fight_long, vampirism) {
        (true, false) => actual_hp_damage / 5.0,
        (true, true) => actual_hp_damage * 0.25,
        (false, true) => actual_hp_damage * 0.05,
        (false, false) => return,
    };
    let owner = half_here::owner_id(r, a);
    if let Some(healer) = engine::actor_mut_trusted(r, owner) {
        if healer.hp <= 0.0 || healer.dead.value_or(false) {
            return;
        }
        healer.hp = (healer.hp + healing).min(healer.max_hp);
    }
    half_here::sync(r);
}
/// Source syncHealth unlocks this once, even if maximum health does not change.
pub fn unlock_blood_and_fury(r: &mut Run) {
    if !has(&r.player, FIGHT_LONG) || !has(&r.player, BLOOD_WEAPON) {
        return;
    }
    let achievements = r.achievements.get_or_insert_with(Vec::new);
    if achievements.iter().any(|s| s == "blood-and-fury") {
        return;
    }
    achievements.push("blood-and-fury".into());
    engine::log(r, "Achievement unlocked: Blood and Fury.", "player");
}
