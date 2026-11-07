//! Revision 50 attack and source-effect predicates. No extra attack rolls.
use crate::{Actor, Run, abilities, engine, half_here};
pub const EXECUTE: &str = "shrail-execute";
pub const FORCE: &str = "shrail-unstoppable-force";
pub const COMMANDMENT: &str = "anavani-commandment";
pub const WITCH_HATRED: &str = "grisha-witch-hatred";
pub const OBLIVIOUS: &str = "ursa-oblivious";
pub const MAGICKAL_CASTER: &str = "enemy:beast:magickal-caster";
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 50. }
fn owner<'a>(r: &'a Run, a: &'a Actor) -> &'a Actor {
    engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a)
}
fn has(a: &Actor, id: &str) -> bool { a.abilities.iter().any(|v| v == id) }
fn enemies(r: &Run, a: &Actor, b: &Actor) -> bool {
    a.id != b.id && (a.id == r.player.id || a.state == "ally") != (b.id == r.player.id || b.state == "ally")
}
/// Explicit authored trait only: AI caster behavior is not a bestiary classification.
pub fn magickal_caster(a: &Actor) -> bool { has(a, MAGICKAL_CASTER) }
/// Birth-only authored R50 classification. Frozen templates and existing actors
/// are not retuned; summoned profiles keep their independent numeric overlays.
pub fn mark_new_caster(a: &mut Actor, kind: &str, revision: f64) {
    if revision >= 50. && matches!(kind, "witch" | "hemogoblin-bloodmancer" | "dryad"
        | "horn-1" | "horn-2" | "horn-3" | "horn-4" | "horn-5" | "horn-6" | "horn-7"
        | "saidribee-queen" | "valta") && !magickal_caster(a) {
        a.abilities.push(MAGICKAL_CASTER.into());
    }
}

pub fn witch_senses(r: &Run, a: &Actor, b: &Actor) -> bool {
    enabled(r) && has(owner(r, a), WITCH_HATRED) && magickal_caster(b)
        && enemies(r, a, b) && crate::dreamwalking::same_plane(r, a, b)
        && b.hp > 0. && !b.dead.value_or(false)
}
/// Before attack rolls, damage, statuses, displacement or other source effects.
/// Environmental effects with no enemy source are intentionally not blocked.
pub fn oblivious_blocks(r: &Run, target: &Actor, source: Option<&Actor>) -> bool {
    if !enabled(r) || !has(owner(r, target), OBLIVIOUS) { return false; }
    source.is_some_and(|s| enemies(r, target, s) && if half_here::owner_id(r, target) == r.player.id {
        !crate::vision::visible_actor(r, s)
    } else {
        !crate::dreamwalking::same_plane(r, target, s)
            || !crate::vision::line_of_sight(r, target, s)
            || crate::vision::detection_chance(r, target, s) <= 0.
    })
}
/// Call once after a strike lands, before mitigation. Execute does not spend on
/// a miss, a full-quarter-health target, insufficient funds, or Witch-Hatred kill.
pub fn lethal_hit(r: &mut Run, a: &Actor, b: &Actor) -> bool {
    if !enabled(r) || !enemies(r, a, b) { return false; }
    if witch_senses(r, a, b) { return true; }
    let health = owner(r, b);
    let eligible = health.hp > 0. && health.hp < health.max_hp * 0.25;
    eligible && abilities::endpoint_active(r, a, EXECUTE) && crate::revision50_ability_energy::spend(r, a.id, EXECUTE, 20.)
}
fn cell_open(r: &Run, a: &Actor, x: f64, y: f64, ignored: Option<f64>) -> bool {
    if !engine::actor_passable(r, a, x, y) || engine::bulwark_blocks(r, a, x, y) { return false; }
    let Some(f) = r.floor() else { return false; };
    let at = y as usize * f.width() + x as usize;
    if a.id != r.player.id && f.tiles[at] == "chasm" && !engine::flying_in(r, a) { return false; }
    if a.id == r.player.id && crate::dreamwalking::active(r)
        && matches!(f.tiles[at].as_str(), "up" | "down" | "pit") { return false; }
    if f.doors.as_ref().and_then(|d| d.get(&at.to_string())).is_some_and(|d| !d.open) { return false; }
    !std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|b| {
        b.id != a.id && Some(b.id) != ignored && b.hp > 0. && !b.dead.value_or(false)
            && crate::dreamwalking::same_plane(r, a, b) && crate::portals::same_position(r, b, x, y)
    })
}
fn arrive(r: &mut Run, id: f64, old: (f64, f64), grapple: Option<crate::revision50_grappler::DragSnapshot>) -> Result<(), engine::EngineError> {
    let Some(width) = r.floor().map(|f| f.width()) else { return Ok(()); };
    let previous = engine::actor_mut_trusted(r, id).and_then(|a| a.conditions.insert("forcedMove".into(), 1.));
    let result = engine::finish_actor_arrival(r, id, old.1 as usize * width + old.0 as usize, false, grapple);
    if let Some(a) = engine::actor_any_mut(r, id) {
        a.conditions.shift_remove("forcedMove");
        if let Some(value) = previous { a.conditions.insert("forcedMove".into(), value); }
    }
    result
}
/// Validate all outcomes before the serialized portal choice. Used only when
/// Force must stage both actors before either arrival can change floors.
fn push_destination(r: &mut Run, a: &Actor, dx: f64, dy: f64) -> Option<(f64,f64)> {
    let (x,y) = (a.x + dx, a.y + dy);
    if !cell_open(r, a, x, y, None) { return None; }
    let options = crate::portals::step_options(r, a, x, y);
    if options.is_empty() || !options.iter().all(|&(x,y)| cell_open(r, a, x, y, None)) { return None; }
    let destination = crate::portals::commit_step(r, a, x, y);
    (destination != (a.x,a.y)).then_some(destination)
}
/// Existing forced movement owns contact, doors, Grappler, falls and stairs.
/// Return actual displacement, never the action-consumed boolean alone.
fn forced_step(r: &mut Run, a: &Actor, dx: f64, dy: f64) -> Result<bool, engine::EngineError> {
    let depth = r.depth;
    let previous = engine::actor_mut_trusted(r, a.id).and_then(|a| a.conditions.insert("forcedMove".into(), 1.));
    let result = engine::move_actor(r, a.id, dx, dy, true);
    if let Some(current) = engine::actor_any_mut(r, a.id) {
        current.conditions.shift_remove("forcedMove");
        if let Some(value) = previous { current.conditions.insert("forcedMove".into(), value); }
    }
    result?;
    Ok(r.depth != depth || engine::actor(r, a.id).is_some_and(|b| (b.x,b.y) != (a.x,a.y)))
}
fn push_step(r: &mut Run, id: f64, dx: f64, dy: f64) -> Result<bool, engine::EngineError> {
    let Some(a) = engine::actor(r, id).cloned() else { return Ok(false); };
    let (x,y) = (a.x + dx, a.y + dy);
    if !cell_open(r, &a, x, y, None) { return Ok(false); }
    let options = crate::portals::step_options(r, &a, x, y);
    if options.is_empty() || !options.iter().all(|&(x,y)| cell_open(r, &a, x, y, None)) { return Ok(false); }
    forced_step(r, &a, dx, dy)
}
/// Call once for an attempted attack after damage/reactions, including misses.
/// Uses the original attack squares so an enemy killed by the hit still yields
/// its square to Unstoppable Force. Commandment runs after the advance.
/// Maximum push visits at most V distinct cells (portal loops stop); O(V*A*P)
/// collision work, O(V) memory, plus existing arrival/plane/portal helper costs.
pub fn after_attack(r: &mut Run, attacker: &Actor, target: &Actor) -> Result<(), engine::EngineError> {
    if !enabled(r) || !enemies(r, attacker, target) { return Ok(()); }
    let depth = r.depth;
    let Some(a) = engine::actor(r, attacker.id).cloned().filter(|a| a.hp > 0. && !a.dead.value_or(false)) else { return Ok(()); };
    if !crate::dreamwalking::same_plane(r, &a, target) { return Ok(()); }
    let (dx,dy) = crate::portals::direction_toward(r, attacker, target.x, target.y);
    if dx == 0. && dy == 0. { return Ok(()); }
    if abilities::endpoint_active(r, &a, FORCE) && cell_open(r, &a, target.x, target.y, Some(target.id)) {
        let living = engine::actor(r, target.id).cloned().filter(|b| b.hp > 0. && !b.dead.value_or(false));
        let standing = living.as_ref().filter(|b| b.x == target.x && b.y == target.y);
        let displacement = standing.and_then(|b| push_destination(r, b, dx, dy)
            .or_else(|| cell_open(r, b, a.x, a.y, Some(a.id)).then_some((a.x,a.y))));
        if standing.is_none() || displacement.is_some() {
            let grapple = crate::revision50_grappler::snapshot(r);
            // Atomic placement before callbacks: swapping never marks an actor
            // dead, and a pushed player cannot change floors halfway through it.
            if let Some((x,y)) = displacement {
                let b = engine::actor_mut_trusted(r, target.id).unwrap(); b.x = x; b.y = y;
            }
            let moved = engine::actor_mut_trusted(r, a.id).unwrap(); moved.x = target.x; moved.y = target.y;
            // Settle the non-player endpoint first, then player travel last.
            if a.id == r.player.id {
                if displacement.is_some() { arrive(r, target.id, (target.x,target.y), None)?; }
                arrive(r, a.id, (a.x,a.y), grapple)?;
            } else {
                arrive(r, a.id, (a.x,a.y), None)?;
                if displacement.is_some() { arrive(r, target.id, (target.x,target.y), grapple)?; }
                else { crate::revision50_grappler::finish(r, grapple)?; }
            }
        }
    }
    if r.depth != depth || r.status != "playing" { return Ok(()); }
    if !has(owner(r, &a), COMMANDMENT) { return Ok(()); }
    let Some(b) = engine::actor(r, target.id).cloned().filter(|b| b.hp > 0. && !b.dead.value_or(false)) else { return Ok(()); };
    let Some(current_attacker) = engine::actor(r, a.id).filter(|a| a.hp > 0. && !a.dead.value_or(false)) else { return Ok(()); };
    let (dx,dy) = crate::portals::direction_toward(r, current_attacker, b.x, b.y);
    if dx == 0. && dy == 0. { return Ok(()); }
    let maximum = abilities::endpoint_active(r, &a, COMMANDMENT) && crate::revision50_ability_energy::spend(r, a.id, COMMANDMENT, 20.);
    let limit = if maximum { r.floor().map_or(0, |f| f.tiles.len()) } else { 1 };
    let mut seen = std::collections::HashSet::new();
    seen.insert((b.x as i32,b.y as i32));
    for _ in 0..limit {
        if !push_step(r, b.id, dx, dy)? { break; }
        if r.depth != depth || r.status != "playing" || engine::actor(r, a.id).is_none() { break; }
        let Some(b) = engine::actor(r, b.id) else { break; };
        if !seen.insert((b.x as i32,b.y as i32)) { break; }
    }
    Ok(())
}

/// Returns true only when the melee attack is replaced by a successful retreat.
/// Fixed eight-neighbor scan, farthest legal square first with stable direction
/// tie-breaking. A trapped endpoint spends no reaction and the attack proceeds.
pub fn missed_me(r: &mut Run, attacker: &Actor, defender: &Actor, melee: bool) -> Result<bool, engine::EngineError> {
    if !enabled(r) || !melee || !abilities::endpoint_active(r, defender, "isheilah-missed-me")
        || owner(r, defender).reactions.value_or(0.) <= 0.
        || !crate::dreamwalking::same_plane(r, attacker, defender) { return Ok(false); }
    let initial = crate::portals::actor_distance(r, defender, attacker);
    let mut best = initial;
    let mut chosen = None;
    for delta in crate::movement::DIRECTIONS {
        let (x,y) = (defender.x + delta.x as f64, defender.y + delta.y as f64);
        if !cell_open(r, defender, x, y, None) { continue; }
        let Some(floor) = r.floor() else { continue; };
        if !crate::movement::diagonal_step_allowed(&floor.tiles, floor.width(),
            crate::movement::Point { x: defender.x as i32, y: defender.y as i32 },
            crate::movement::Point { x: x as i32, y: y as i32 },
            abilities::endpoint_active(r, defender, "etill-phase-through")) { continue; }
        let options = crate::portals::step_options(r, defender, x, y);
        if options.is_empty() || !options.iter().all(|&(x,y)| cell_open(r, defender, x, y, None)) { continue; }
        let worst = options.iter().map(|&(x,y)| {
            let mut candidate = defender.clone(); candidate.x = x; candidate.y = y;
            crate::portals::actor_distance(r, &candidate, attacker)
        }).fold(f64::INFINITY, f64::min);
        if worst > best { best = worst; chosen = Some((x,y)); }
    }
    let Some((x,y)) = chosen else { return Ok(false); };
    if !forced_step(r, defender, x - defender.x, y - defender.y)? { return Ok(false); }
    let owner_id = half_here::owner_id(r, defender);
    if let Some(o) = engine::actor_any_mut(r, owner_id) {
        o.reactions = (o.reactions.value_or(0.) - 1.).into();
    }
    Ok(true)
}
/// Pure crossing test, intentionally no once-per-life latch: healing rearms it.
/// Runtime composition is ten total turns even when several thresholds cross.
pub fn masochism_crossings(before: f64, after: f64, maximum: f64) -> usize {
    if !before.is_finite() || !after.is_finite() || !maximum.is_finite() || maximum <= 0. || after >= before { return 0; }
    [1.,0.75,0.5,0.25].iter().filter(|&&fraction| before >= maximum * fraction && after < maximum * fraction).count()
}

pub const MASOCHISM: &str = "shrail-masochism";
/// Called once after shared HP is reduced and before endpoint synchronization or
/// death rescue. Any number of crossings refreshes ten turns, never adds ten per
/// threshold; a longer existing haste effect is not shortened. Healing rearms
/// thresholds automatically because no historical crossing latch is persisted.
/// O(A + K + I*E) plus existing effective-stat/health derivation and COW costs,
/// where I is inventory size and E item enhancement/stat entries. No RNG.
pub fn masochism_after_hp_loss(r: &mut Run, victim: f64, before_hp: f64) {
    if !enabled(r) { return; }
    let Some(body) = engine::actor(r, victim) else { return; };
    let id = half_here::owner_id(r, body);
    let Some(a) = engine::actor(r, id) else { return; };
    if !has(a, MASOCHISM) || a.dead.value_or(false) { return; }
    let maximum = crate::divine_combat::health_from_stats(r, a, &crate::combat::effective_stats(r, a));
    if masochism_crossings(before_hp, a.hp, maximum) == 0 { return; }
    let resolving = r.resolving_tick.value_or(false);
    let Some(a) = engine::actor_mut_trusted(r, id) else { return; };
    let duration = engine::condition(a, "hasted").max(10.);
    a.conditions.insert("hasted".into(), duration);
    if resolving { a.conditions.shift_remove("hastedFresh"); }
    else { a.conditions.insert("hastedFresh".into(), 1.); }
}
