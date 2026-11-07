//! Source-ordered reaction decisions. Recursive attacks and damage stay in engine.
use crate::{Actor, Run, engine, half_here, rng};
pub const SURRENDER: &str = "pelian-never-surrender";
pub const QUICK_HAND: &str = "isheilah-quick-hand";
pub const HIT_BACK: &str = "shrail-i-hit-back";
pub const DEFLECT: &str = "isheilah-are-you-sure-about-that";
pub const NIGHTMARE: &str = "ursa-waking-nightmare";
pub const BLEED: &str = "lichor-bleed-them";
fn owner<'a>(r: &'a Run, a: &'a Actor) -> &'a Actor {
    engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a)
}
fn active(r: &Run, a: &Actor, id: &str) -> bool {
    crate::abilities::endpoint_active(r, a, id)
}
fn allied(r: &Run, a: &Actor) -> bool {
    a.id == r.player.id || a.state == "ally"
}
fn hostile(r: &Run, a: &Actor, b: &Actor) -> bool {
    a.id != b.id && allied(r, a) != allied(r, b)
        && crate::dreamwalking::same_plane(r, a, b)
}
fn adjacent(r: &Run, a: &Actor, b: &Actor) -> bool {
    crate::portals::actor_distance(r, a, b) <= std::f64::consts::SQRT_2
}
/// The source compares missing surrenderTick distinctly from tick zero.
pub fn surrender_bonus(r: &Run, b: &Actor, defense: f64, apply: bool) -> f64 {
    let o = owner(r, b);
    if apply
        && active(r, b, SURRENDER)
        && crate::revision50_ability_energy::can_pay(r, b.id, SURRENDER, 8.0)
        && o.conditions.get("surrenderTick") != Some(&r.tick)
    {
        defense
    } else {
        0.0
    }
}
/// Call after rolling a landed strike, using damage without the Surrender bonus.
pub fn pay_surrender(r: &mut Run, b: &Actor, ordinary_damage: f64) {
    if ordinary_damage <= 0.0
        || !active(r, b, SURRENDER)
        || owner(r, b).conditions.get("surrenderTick") == Some(&r.tick)
    {
        return;
    }
    let id = half_here::owner_id(r, b);
    if engine::actor(r, id).is_none() {
        return;
    }
    let endpoint = if engine::actor(r, b.id).is_some() {
        b.id
    } else {
        id
    };
    if crate::revision50_ability_energy::spend(r, endpoint, SURRENDER, 8.0) {
        let tick = r.tick;
        engine::actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("surrenderTick".into(), tick);
    } else {
        engine::actor_mut_trusted(r, id)
            .unwrap()
            .active
            .retain(|s| s != SURRENDER);
    }
}
/// Landed zero-damage hits also trigger Quick Hand while the endpoint lives.
pub fn quick_hand_after_hit(r: &mut Run, b: &Actor) {
    if b.hp <= 0.0
        || b.dead.value_or(false)
        || !active(r, b, QUICK_HAND)
        || owner(r, b).reactions.value_or(0.0) <= 0.0
    {
        return;
    }
    let id = half_here::owner_id(r, b);
    let resolving = r.resolving_tick.value_or(false);
    if let Some(o) = engine::actor_mut_trusted(r, id) {
        o.reactions = (o.reactions.value_or(0.0) - 1.0).into();
        let h = engine::condition(o, "hasted").max(1.0);
        o.conditions.insert("hasted".into(), h);
        if resolving {
            o.conditions.shift_remove("hastedFresh");
        } else {
            o.conditions.insert("hastedFresh".into(), 1.0);
        }
    }
}
/// Counter may still fire after a terminal strike. Use source's post-impact body
/// position and pre-strike defender Offense; the engine supplies current attacker's DR.
pub fn prepare_hit_back(
    r: &mut Run,
    a: &Actor,
    b: &Actor,
    defender_offense: f64,
    attacker_dr: f64,
    adaptive_dr: f64,
    secondary: bool,
) -> Option<f64> {
    if r.simulation_revision.value_or(29.) >= 50.
        || secondary
        || (crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(b))
        || !crate::dreamwalking::same_plane(r, a, b)
        || !b.abilities.iter().any(|s| s == HIT_BACK)
        || engine::condition(owner(r, b), "hitBack") != 0.0
        || !adjacent(r, a, b)
    {
        return None;
    }
    let id = half_here::owner_id(r, b);
    if let Some(o) = engine::actor_mut_trusted(r, id) {
        o.conditions.insert("hitBack".into(), 1.0);
    }
    Some((defender_offense - attacker_dr - adaptive_dr).max(0.0))
}
/// Candidate order is player then current-floor actor order. No invented LOS or
/// path filter: adjacency/hostility are the authored tests. Original attacker may win.
pub fn select_deflection_target(r: &mut Run, b: &Actor, secondary: bool) -> Option<f64> {
    if secondary || !active(r, b, DEFLECT) || owner(r, b).reactions.value_or(0.0) <= 0.0 {
        return None;
    }
    let candidates: Vec<f64> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|c| !c.dead.value_or(false) && c.hp > 0.0 && hostile(r, b, c) && adjacent(r, b, c))
        .map(|c| c.id)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let id = half_here::owner_id(r, b);
    if let Some(o) = engine::actor_mut_trusted(r, id) {
        o.reactions = (o.reactions.value_or(0.0) - 1.0).into();
    }
    let index = (rng::random(r) * candidates.len() as f64).floor() as usize;
    Some(candidates[index])
}
pub fn nightmare_absorbs(r: &Run, a: &Actor, source: Option<&Actor>) -> bool {
    r.simulation_revision.value_or(29.) < 50.
        && source.is_some_and(|s| hostile(r, s, a))
        && active(r, a, NIGHTMARE)
        && owner(r, a).nightmares.value_or(0.0) > 0.0
}
/// Invoke only after finite positive packet/immunity checks, before HP loss/logging.
pub fn absorb_nightmare(r: &mut Run, a: &Actor, source: Option<&Actor>) -> bool {
    if !nightmare_absorbs(r, a, source) {
        return false;
    }
    let id = half_here::owner_id(r, a);
    if let Some(o) = engine::actor_mut_trusted(r, id) {
        o.nightmares = (o.nightmares.value_or(0.0) - 1.0).into();
    }
    half_here::sync(r);
    true
}
/// O(1) stack lookup/arithmetic, no allocation or RNG. Bleed Them is the only
/// gameplay producer of this status; duration/reapplication remain unchanged.
/// The boolean bypasses only generic damage scaling, not packet protections.
pub fn bleeding_packet(revision: f64, body: &Actor) -> (f64, bool) {
    if revision >= 44.0 {
        let stacks = engine::condition(body, "bleeding");
        (if stacks > 0.0 { 1.0 + (stacks / 5.0).floor() } else { 0.0 }, true)
    } else {
        (body.max_hp * 0.02, false)
    }
}
pub fn apply_bleeding(r: &mut Run, a: &Actor, target_id: f64, dealt: f64, offense: f64) {
    if dealt <= 0.0 || !a.abilities.iter().any(|s| s == BLEED) {
        return;
    }
    if engine::actor(r, target_id)
        .is_some_and(|b| !crate::dreamwalking::same_plane(r, a, b))
    {
        return;
    }
    if let Some(b) = engine::actor_mut_trusted(r, target_id) {
        if b.dead.value_or(false) {
            return;
        }
        let duration = engine::condition(b, "bleeding").max(offense.max(0.0));
        b.conditions.insert("bleeding".into(), duration);
        b.conditions.insert("bleedSourceId".into(), a.id);
    }
}

#[cfg(test)]
mod dream_plane_tests {
    use super::*;
    #[test]
    fn sleeping_body_reactions_are_inert_and_deflection_stays_on_its_endpoint_plane() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: r.depth, ..Default::default() }.into();
        r.player.active = vec![SURRENDER.into(), QUICK_HAND.into(), DEFLECT.into(), NIGHTMARE.into()];
        r.player.abilities.push(HIT_BACK.into());
        r.player.reactions = 3.0.into(); r.player.nightmares = 2.0.into();
        r.player.energy = 100.; r.player.conditions.clear();
        let mut body = r.player.clone(); body.id = 90.; body.active.clear();
        body.conditions.insert("phantasmiiBody".into(), 1.);
        body.conditions.insert("halfHereOwner".into(), r.player.id);
        let mut enemy = r.actors_at(1.)[0].clone();
        enemy.x = r.player.x + 1.; enemy.y = r.player.y;
        *r.actors_at_mut(1.).first_mut().unwrap() = enemy.clone();
        assert_eq!(surrender_bonus(&r, &body, 20., true), 0.);
        pay_surrender(&mut r, &body, 10.);
        quick_hand_after_hit(&mut r, &body);
        assert_eq!(r.player.energy, 100.);
        assert_eq!(r.player.reactions.value_or(0.), 3.);
        assert!(!r.player.conditions.contains_key("hasted"));
        assert!(!nightmare_absorbs(&r, &body, Some(&enemy)));
        assert_eq!(prepare_hit_back(&mut r, &enemy, &body, 20., 0., 0., false), None);
        assert_eq!(select_deflection_target(&mut r, &body, false), None);
        let player = r.player.clone();
        assert_eq!(select_deflection_target(&mut r, &player, false), None);
        assert_eq!(r.player.reactions.value_or(0.), 3.);
        crate::engine::actor_mut_trusted(&mut r, enemy.id).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        assert_eq!(select_deflection_target(&mut r, &player, false), Some(enemy.id));
        assert_eq!(r.player.reactions.value_or(0.), 2.);
    }
}
