//! R52 species follow-ups. Identity works on imported bodies, not birth markers.
use crate::{Actor, Run, engine::{self, EngineError}};

pub const BLEED: &str = "killspawnBleeding";
pub const BLEED_PREFIX: &str = "killspawnBleed:";
pub const BLEED_PACKET: &str = "killspawnBleed";
pub const FLIGHT: &str = "dragonFlight";
pub const FLIGHT_COST: f64 = 8.;
pub const FLIGHT_TURNS: f64 = 6.;
pub const BLEED_TURNS: usize = 3;

pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) == 52. }
pub fn species(a: &Actor, kind: &str) -> bool {
    crate::enemy_traits::definition(a).is_some_and(|d| d.id == kind)
}
pub fn condition_key(k: &str) -> bool {
    // Pending travel can become active when the player returns before a
    // pursuer arrives. These existing chase values retain their old semantics;
    // the new explicit chasm marker alone adds a constrained value shape.
    k == BLEED || k == FLIGHT || k.starts_with(BLEED_PREFIX)
        || matches!(k,"chaseDepth"|"chaseExitX"|"chaseExitY"|"chaseLandingX"|"chaseLandingY"|"chaseChasm")
}
/// Validate syntax and bounded timer shapes, not deadlines against global time:
/// an inactive floor or temporal copy may carry its own older normal clock.
pub fn valid_condition(k: &str, value: f64) -> bool {
    if k == "chaseChasm" { return value == 1.; }
    let count = |n:f64| n.is_finite() && n>=0. && n.fract()==0. && n<=9_007_199_254_740_991.;
    if k==BLEED { return count(value) && value<=BLEED_TURNS as f64; }
    if k==FLIGHT { return count(value) && value<=FLIGHT_TURNS; }
    if k.starts_with(BLEED_PREFIX) {
        return parse_bucket(k).is_some_and(|(due,source)| count(due) && count(source))
            && count(value) && value>0.;
    }
    true
}
/// O(all persisted actor conditions), including carried pilots. Pure admission
/// shared by command and save/load boundaries; no repair or silent dropping.
pub fn validate_run(r:&Run)->Result<(),String> {
    if !enabled(r) {return Ok(());}
    fn validate_actor(a:&Actor)->Result<(),String> {
        if a.conditions.iter().any(|(k,&v)|!valid_condition(k,v)) {
            return Err("Invalid Revision 52 enemy follow-up condition".into());
        }
        let has_wound=a.conditions.keys().any(|k|k.starts_with(BLEED_PREFIX));
        if has_wound != (engine::condition(a,BLEED)>0.) {
            return Err("Invalid Revision 52 wound timer/budget pair".into());
        }
        if let Some(pilot)=a.monster_state.as_ref().and_then(|s|s.pilot.as_ref()) {validate_actor(pilot)?;}
        Ok(())
    }
    validate_actor(&r.player)?;
    for actors in r.actors.values() {for a in actors.iter() {validate_actor(a)?;}}
    Ok(())
}
pub fn clear_bleed(a: &mut Actor) {
    a.conditions.retain(|k, _| k != BLEED && !k.starts_with(BLEED_PREFIX));
}
/// O(C) condition scan, O(1) new scheduled buckets per hit. Each wound commits
/// its whole resisted budget once, then three integer installments. Repeated
/// hits add, never reset another wound's remaining damage or deadline.
pub fn after_melee(r: &mut Run, source: &Actor, target: f64, dealt: f64, melee: bool) -> Result<(), EngineError> {
    if !enabled(r) || !melee || !dealt.is_finite() || dealt <= 0. { return Ok(()); }
    let Some(body) = engine::actor(r, target).cloned().filter(|b| b.hp > 0. && !b.dead.value_or(false)) else { return Ok(()); };
    if species(source, "blorb") {
        // No cloud or environmental deposition. The usual caustic pipeline
        // retains corrosion resistance, gas immunity, absorption and death.
        engine::damage_direct_with_source_snapshot(r, target, 1., source, "caustic", true)?;
    } else if species(source, "killspawn") {
        let budget = crate::combat::nearest_damage(crate::revision50_rings::resist_damage(r, &body, "bleeding", dealt * 0.5));
        if budget <= 0. { return Ok(()); }
        let tick = r.tick;
        let a = engine::actor_mut_trusted(r, target).unwrap();
        let base = (budget / BLEED_TURNS as f64).floor();
        let extra = budget as usize % BLEED_TURNS;
        for offset in 1..=BLEED_TURNS {
            let amount = base + f64::from(offset <= extra);
            if amount > 0. {
                *a.conditions.entry(format!("{BLEED_PREFIX}{}:{}", tick + offset as f64, source.id)).or_default() += amount;
            }
        }
        refresh_bleed(a, tick);
        let a = a.clone();
        crate::animation_enemy::mark(r, &a, "E71", "droplet-red", crate::events::Stage::Special);
    }
    Ok(())
}
fn parse_bucket(k: &str) -> Option<(f64, f64)> {
    let (due, source) = k.strip_prefix(BLEED_PREFIX)?.split_once(':')?;
    Some((due.parse().ok()?, source.parse().ok()?))
}
fn refresh_bleed(a: &mut Actor, tick: f64) {
    let remaining = a.conditions.iter().filter(|(_, v)| **v > 0.)
        .filter_map(|(k, _)| parse_bucket(k).map(|(due, _)| due - tick)).fold(0., f64::max);
    if remaining > 0. { a.conditions.insert(BLEED.into(), remaining); }
    else { a.conditions.shift_remove(BLEED); }
}
/// Normal-world-upkeep only, shared by visible and offscreen simulation. O(C)
/// buckets plus the ordinary packet/lifecycle cost per due source. Removing a
/// bucket before dispatch makes lethal rescues and repeated calls idempotent.
pub fn tick_bleed(r: &mut Run, id: f64) -> Result<(), EngineError> {
    if !enabled(r) { return Ok(()); }
    let Some(body) = engine::actor(r, id) else { return Ok(()); };
    let due: Vec<_> = body.conditions.iter().filter_map(|(k, &amount)| {
        let (tick, source) = parse_bucket(k)?;
        (tick <= r.tick).then_some((k.clone(), amount, source))
    }).collect();
    for (key, amount, source) in due {
        let Some(body) = engine::actor_mut_trusted(r, id) else { break; };
        body.conditions.shift_remove(&key);
        if body.dead.value_or(false) || body.hp <= 0. { clear_bleed(body); break; }
        engine::damage_environment(r, id, amount, Some(source), BLEED_PACKET, true)?;
        if r.status != "playing" { break; }
    }
    let tick = r.tick;
    if let Some(body) = engine::actor_mut_trusted(r, id) {
        if body.dead.value_or(false) || body.hp <= 0. { clear_bleed(body); }
        else { refresh_bleed(body, tick); }
    }
    Ok(())
}

#[cfg(test)]
#[path = "revision52_enemy_followups_tests.rs"]
mod tests;
