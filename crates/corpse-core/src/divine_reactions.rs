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
    pay_surrender_from(r,b,ordinary_damage,None);
}
/// Direction is presentation-only and admitted only for an observed attacker.
pub fn pay_surrender_from(r:&mut Run,b:&Actor,ordinary_damage:f64,attacker:Option<&Actor>) {
    if ordinary_damage <= 0.0
        // R52 waits for a source whose damage is reduced by DR. Piercing hits
        // bypass the bonus, so they cannot spend energy or consume the proc.
        || (r.simulation_revision.value_or(29.) >= 52.
            && attacker.is_some_and(|a| a.abilities.iter().any(|id| id == "enemy:beast:armor-piercing")))
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
        if let Some(attacker)=attacker {crate::events::facing(r,b,attacker,"P10","shield-wedge",crate::events::Stage::Impact);}
        else {crate::animation_player::local(r,b,"P10","shield",crate::events::Stage::Impact);}
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
    let spent=if let Some(o) = engine::actor_mut_trusted(r, id) {
        o.reactions = (o.reactions.value_or(0.0) - 1.0).into();
        let h = engine::condition(o, "hasted").max(1.0);
        o.conditions.insert("hasted".into(), h);
        if resolving {
            o.conditions.shift_remove("hastedFresh");
        } else {
            o.conditions.insert("hastedFresh".into(), 1.0);
        }
        true
    } else {false};
    if !spent {return;}
    crate::animation_player::local(r,b,"P16","speed",crate::events::Stage::Special);
    if id==r.player.id {crate::events::inventory(r,"P16","reaction-spent",None);}
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
    if let Some(target)=engine::actor(r,candidates[index]) {
        crate::events::link(r,b,target,"P17","arc-redirect",crate::events::Stage::Special);
    }
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
    let incoming = engine::actor(r,target_id).map(|b| crate::monster_statuses::status_duration(r,b,offense.max(0.0))).unwrap_or(0.);
    if let Some(b) = engine::actor_mut_trusted(r, target_id) {
        if b.dead.value_or(false) {
            return;
        }
        let duration = engine::condition(b, "bleeding").max(incoming);
        b.conditions.insert("bleeding".into(), duration);
        b.conditions.insert("bleedSourceId".into(), a.id);
        if duration>0. {let b=b.clone();crate::animation_player::local(r,&b,"P38","droplet-two",crate::events::Stage::Special);}
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

#[cfg(test)] mod redirect_animation_fidelity_tests {
 use super::*;
 #[test] fn consumed_redirect_has_selected_endpoints_and_empty_reaction_has_none(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  r.player.x=2.;r.player.y=2.;r.player.abilities=vec![DEFLECT.into()];r.player.active=r.player.abilities.clone();r.player.conditions.clear();r.player.reactions=1.0.into();
  let mut enemy=r.player.clone();enemy.id=900.;enemy.x=3.;enemy.state="hunting".into();enemy.abilities.clear();enemy.active.clear();
  r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(enemy.clone());let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;
  let defender=r.player.clone();crate::events::begin();assert_eq!(select_deflection_target(&mut r,&defender,false),Some(enemy.id));
  let events=crate::events::finish();let link=events.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.style=="arc-redirect")).unwrap();assert_eq!(link.from,Some((&defender).into()));assert_eq!(link.to,(&enemy).into());
  let defender=r.player.clone();crate::events::begin();assert_eq!(select_deflection_target(&mut r,&defender,false),None);assert!(crate::events::finish().is_empty());
 }
}

#[cfg(test)] mod animation_reaction_receipt_tests {
 use super::*;
 #[test] fn quick_hand_has_one_map_cue_and_one_owned_hud_receipt_only_on_spend(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.conditions.clear();r.player.abilities=vec![QUICK_HAND.into()];r.player.active=r.player.abilities.clone();r.player.reactions=1.0.into();let a=r.player.clone();
  crate::events::begin();quick_hand_after_hit(&mut r,&a);let e=crate::events::finish();let cues:Vec<_>=e.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="P16").collect();assert_eq!(cues.len(),2);assert_eq!(cues.iter().filter(|a|a.surface.is_none()&&a.style=="speed").count(),1);assert_eq!(cues.iter().filter(|a|a.surface.as_deref()==Some("inventory")&&a.style=="reaction-spent").count(),1);
  let a=r.player.clone();crate::events::begin();quick_hand_after_hit(&mut r,&a);assert!(crate::events::finish().is_empty());
  let mut ally=r.player.clone();ally.id=900.;ally.x+=1.;ally.state="ally".into();ally.reactions=1.0.into();r.actors_at_mut(r.depth).push(ally.clone());crate::events::begin();quick_hand_after_hit(&mut r,&ally);assert!(!crate::events::finish().iter().filter_map(|e|e.animation.as_ref()).any(|a|a.surface.as_deref()==Some("inventory")));
 }
}
