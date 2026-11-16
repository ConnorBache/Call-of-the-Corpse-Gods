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
pub fn consume_blood_weapon(r: &mut Run, a: &Actor) {consume_blood_weapon_at(r,a,a)}
pub fn consume_blood_weapon_at(r: &mut Run, a: &Actor,target:&Actor) {
    let owner = half_here::owner_id(r, a);
    if let Some(owner) = engine::actor_mut_trusted(r, owner) {
        let consumed=owner.conditions.shift_remove("bloodWeaponReady").is_some();
        if consumed {crate::animation_player::local(r,target,"P41","slash",crate::events::Stage::Impact);}
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
    heal_melee_damage_from(r,a,None,actual_hp_damage,melee);
}
/// The source is the actual damaged endpoint, not a guessed route. Unknown
/// sources retain only the recipient's actual-heal pulse. No new gameplay work.
pub fn heal_melee_damage_from(r:&mut Run,a:&Actor,victim:Option<&Actor>,actual_hp_damage:f64,melee:bool) {
    if !melee || actual_hp_damage <= 0.0 || healing_suppressed(r, a) { return; }
    let fight_long = has(a, FIGHT_LONG);
    let vampirism = crate::revision50_rings::stacking_count(r, a, crate::revision50_rings::VAMPIRISM);
    // Preserve historical division exactly: multiplying by 0.2 changes saved
    // floating-point traces even though the two expressions are mathematically equal.
    let healing = match (fight_long, vampirism) {
        (true, 0) => actual_hp_damage / 5.0,
        (true, copies) => actual_hp_damage * (0.2 + 0.05 * copies as f64),
        (false, 0) => return,
        (false, copies) => actual_hp_damage * (0.05 * copies as f64),
    };
    let owner = half_here::owner_id(r, a);
    let hp_before=engine::actor(r,owner).map_or(0.,|h|h.hp);
    if let Some(healer) = engine::actor_mut_trusted(r, owner) {
        if healer.hp <= 0.0 || healer.dead.value_or(false) {
            return;
        }
        healer.hp = (healer.hp + healing).min(healer.max_hp);
    }
    half_here::sync(r);
    if fight_long {if let Some(h)=engine::actor(r,owner).filter(|h|h.hp>hp_before) {
        crate::animation_player::local(r,h,"P04","droplet",crate::events::Stage::Health);
        if let Some(victim)=victim {crate::events::link(r,victim,h,"P04","droplet-return",crate::events::Stage::Health);}
    }}
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

#[cfg(test)] mod animation_fidelity_tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.hp=50.;r.player.max_hp=100.;
        r.player.abilities=vec![FIGHT_LONG.into()];r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r
    }
    #[test] fn real_heal_links_known_victim_and_preserves_simulation(){
        let mut r=room();let mut expected=r.clone();let a=r.player.clone();let mut b=a.clone();b.id=900.;b.x=3.;b.state="hunting".into();
        crate::events::begin();heal_melee_damage_from(&mut r,&a,Some(&b),20.,true);
        let events=crate::events::finish();assert!(events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="droplet-return")));
        assert!(events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="droplet")));
        heal_melee_damage(&mut expected,&a,20.,true);assert_eq!(r,expected);
    }
    #[test] fn full_hp_hidden_victim_and_legacy_do_not_emit_a_return(){
        let mut r=room();let mut b=r.player.clone();b.id=900.;b.x=3.;b.state="hunting".into();
        r.player.hp=100.;let a=r.player.clone();crate::events::begin();heal_melee_damage_from(&mut r,&a,Some(&b),20.,true);assert!(crate::events::finish().is_empty());
        r.player.hp=50.;b.conditions.insert("invisible".into(),10.);let a=r.player.clone();crate::events::begin();heal_melee_damage_from(&mut r,&a,Some(&b),20.,true);
        let events=crate::events::finish();assert!(!events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="droplet-return")));
        assert_eq!(events.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.style=="droplet")).count(),1);
        r.simulation_revision=51.0.into();let a=r.player.clone();crate::events::begin();heal_melee_damage_from(&mut r,&a,Some(&b),20.,true);assert!(crate::events::finish().is_empty());
    }
}
