//! Revision 50 attack and source-effect predicates. No extra attack rolls.
use crate::{Actor, Run, abilities, engine, half_here};
pub const EXECUTE: &str = "shrail-execute";
pub fn execute_cost(revision:f64)->f64 {if revision>=52. {10.}else{20.}}
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
/// Pre-damage terminal effects. Current Execute waits for actual ordinary HP loss.
fn pay_execute(r: &mut Run, a: &Actor, b: &Actor) -> bool {
    let killed = abilities::endpoint_active(r, a, EXECUTE)
        && crate::revision50_ability_energy::spend(r, a.id, EXECUTE, execute_cost(r.simulation_revision.value_or(29.)));
    if killed {crate::animation_shapes::nova(r,b,"P02","nova-red-small",2);}
    killed
}
pub fn lethal_hit(r: &mut Run, a: &Actor, b: &Actor) -> bool {
    if !enabled(r) || !enemies(r, a, b) { return false; }
    if witch_senses(r, a, b) { crate::animation_shapes::area(r,b,"P54","box-expand",2,"#501060",crate::events::Stage::Special); return true; }
    if crate::execute_rules::current(r) {return false;}
    let health = owner(r, b);
    let eligible = health.hp > 0. && health.hp < health.max_hp * 0.25;
    eligible && pay_execute(r, a, b)
}

/// The caller has already applied ordinary HP loss, including redirection.
/// Neither ordinary overkill nor targets already below the threshold spend energy.
pub fn execute_crossing(r: &mut Run, a: &Actor, b: &Actor, hp_before: f64, quarter: f64, hp_after: f64) -> bool {
    crate::execute_rules::current(r) && hp_before >= quarter && hp_after > 0. && hp_after < quarter
        && enemies(r, a, b) && pay_execute(r, a, b)
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
/// its square to Unstoppable Force. Commandment runs after the advance only
/// after a landed hit under current rules.
/// Maximum push visits at most V distinct cells (portal loops stop); O(V*A*P)
/// collision work, O(V) memory, plus existing arrival/plane/portal helper costs.
pub fn after_attack(r: &mut Run, attacker: &Actor, target: &Actor, landed: bool) -> Result<(), engine::EngineError> {
    if !enabled(r) || !enemies(r, attacker, target) { return Ok(()); }
    let depth = r.depth;
    let Some(a) = engine::actor(r, attacker.id).cloned().filter(|a| a.hp > 0. && !a.dead.value_or(false)) else { return Ok(()); };
    if !crate::dreamwalking::same_plane(r, &a, target) { return Ok(()); }
    let (dx,dy) = crate::portals::direction_toward(r, attacker, target.x, target.y);
    if dx == 0. && dy == 0. { return Ok(()); }
    if abilities::endpoint_active(r, &a, FORCE) && cell_open(r, &a, target.x, target.y, Some(target.id)) {
        let living = engine::actor(r, target.id).cloned().filter(|b| b.hp > 0. && !b.dead.value_or(false));
        let standing = living.as_ref().filter(|b| b.x == target.x && b.y == target.y);
        let displacement = standing.filter(|b| !crate::half_here::position_locked(r, b.id)).and_then(|b| push_destination(r, b, dx, dy)
            .or_else(|| cell_open(r, b, a.x, a.y, Some(a.id)).then_some((a.x,a.y))));
        if standing.is_none() || displacement.is_some() {
            let grapple = crate::revision50_grappler::snapshot(r);
            // Atomic placement before callbacks: swapping never marks an actor
            // dead, and a pushed player cannot change floors halfway through it.
            if let Some((x,y)) = displacement {
                let b = engine::actor_mut_trusted(r, target.id).unwrap(); b.x = x; b.y = y;
            }
            let moved = engine::actor_mut_trusted(r, a.id).unwrap(); moved.x = target.x; moved.y = target.y;
            let swapped=displacement==Some((a.x,a.y));
            crate::events::link(r,&a,engine::actor(r,a.id).unwrap(),"P05",if swapped {"arc-swap-forward"}else{"wave-advance"},crate::events::Stage::Travel);
            if displacement.is_some() {crate::events::link(r,target,engine::actor(r,target.id).unwrap(),"P05",if swapped {"arc-swap-reverse"}else{"speed-push"},crate::events::Stage::Travel);}
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
    if (!landed && crate::commandment_rules::current(r)) || !has(owner(r, &a), COMMANDMENT) { return Ok(()); }
    let Some(b) = engine::actor(r, target.id).cloned().filter(|b| b.hp > 0. && !b.dead.value_or(false)) else { return Ok(()); };
    let Some(current_attacker) = engine::actor(r, a.id).filter(|a| a.hp > 0. && !a.dead.value_or(false)) else { return Ok(()); };
    let (dx,dy) = crate::portals::direction_toward(r, current_attacker, b.x, b.y);
    if dx == 0. && dy == 0. { return Ok(()); }
    let maximum = abilities::endpoint_active(r, &a, COMMANDMENT) && crate::revision50_ability_energy::spend(r, a.id, COMMANDMENT, 20.);
    let limit = if maximum { r.floor().map_or(0, |f| f.tiles.len()) } else { 1 };
    let mut seen = std::collections::HashSet::new();
    seen.insert((b.x as i32,b.y as i32));
    let mut route=crate::events::TravelRoute::default();
    for _ in 0..limit {
        let before=engine::actor(r,b.id).cloned();
        if !push_step(r, b.id, dx, dy)? { break; }
        if maximum && r.depth==depth {if let (Some(before),Some(after))=(before,engine::actor(r,b.id)) {
            let glyph=crate::actor_presentation::apparent_kind(r,&before)
                .and_then(|kind|crate::population::catalog().actors.get(kind)).map(|a|a.glyph.as_str()).unwrap_or(&before.glyph);
            route.capture(r,&before,&[(&before).into(),after.into()],glyph);
        }}
        if r.depth != depth || r.status != "playing" || engine::actor(r, a.id).is_none() { break; }
        let Some(b) = engine::actor(r, b.id) else { break; };
        if !seen.insert((b.x as i32,b.y as i32)) { break; }
    }
    route.finish();
    Ok(())
}

/// Returns true only when the melee attack is replaced by a successful retreat.
/// Fixed eight-neighbor scan, farthest legal square first with stable direction
/// tie-breaking. A trapped endpoint spends no reaction and the attack proceeds.
pub fn missed_me(r: &mut Run, attacker: &Actor, defender: &Actor, melee: bool) -> Result<bool, engine::EngineError> {
    if !enabled(r) || crate::hanged::enabled(r.simulation_revision.value_or(29.))
        || !melee || !abilities::endpoint_active(r, defender, "isheilah-missed-me")
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
    let before_depth=r.depth;
    if !forced_step(r, defender, x - defender.x, y - defender.y)? { return Ok(false); }
    let owner_id = half_here::owner_id(r, defender);
    if let Some(o) = engine::actor_any_mut(r, owner_id) {
        o.reactions = (o.reactions.value_or(0.) - 1.).into();
    }
    if owner_id==r.player.id {crate::events::inventory(r,"P13","reaction-spent-missed-me",None);}
    if let Some(after)=engine::actor(r,defender.id) {if r.depth==before_depth {crate::animation_player::moved(r,defender,after,"P13","unfold");}else{crate::animation_player::local(r,after,"P13","unfold",crate::events::Stage::Travel);}}
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
    let duration = crate::monster_statuses::status_duration(r,a,10.);
    let resolving = r.resolving_tick.value_or(false);
    let Some(a) = engine::actor_mut_trusted(r, id) else { return; };
    let duration = engine::condition(a, "hasted").max(duration);
    a.conditions.insert("hasted".into(), duration);
    if resolving { a.conditions.shift_remove("hastedFresh"); }
    else { a.conditions.insert("hastedFresh".into(), 1.); }
    crate::animation_player::local(r,engine::actor(r,id).unwrap(),"P06","speed",crate::events::Stage::Special);
    if id==r.player.id {crate::events::inventory(r,"P06","health-slash",None);}
}

#[cfg(test)] mod force_animation_fidelity_tests {
 use super::*;
 fn room()->(Run,Actor) {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.conditions.clear();r.player.abilities=vec![FORCE.into()];r.player.active=vec![FORCE.into()];r.inventory.clear();
  let mut b=r.player.clone();b.id=900.;b.x=3.;b.state="hunting".into();b.abilities.clear();b.active.clear();
  r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(b.clone());
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;
  (r,b)
 }
 #[test] fn committed_push_and_swap_have_distinct_oriented_styles(){
  for swap in [false,true] {
   let (mut r,b)=room();if swap {let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+4]="wall".into();}
   let a=r.player.clone();crate::events::begin();after_attack(&mut r,&a,&b,true).unwrap();let events=crate::events::finish();
   let styles:Vec<_>=events.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="P05").map(|a|a.style.as_str()).collect();
   assert!(styles.contains(&if swap{"arc-swap-forward"}else{"wave-advance"}),"{styles:?}");
   assert!(styles.contains(&if swap{"arc-swap-reverse"}else{"speed-push"}),"{styles:?}");
   assert_eq!(engine::actor(&r,b.id).map(|b|(b.x,b.y)),Some(if swap{(2.,2.)}else{(4.,2.)}));
  }
 }
 #[test] fn dead_target_has_no_displacement_mark(){
  let (mut r,mut b)=room();b.hp=0.;b.dead=true.into();*engine::actor_mut_trusted(&mut r,b.id).unwrap()=b.clone();
  let a=r.player.clone();crate::events::begin();after_attack(&mut r,&a,&b,true).unwrap();let events=crate::events::finish();
  assert!(!events.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.style=="speed-push"||a.style=="arc-swap-reverse"));
 }
}

#[cfg(test)] mod animation_commandment_style_tests {
 use super::*;
 #[test] fn only_paid_push_has_one_grouped_observed_route(){
  for mode in [0,1,2] {
   let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.conditions.clear();r.player.abilities=vec![COMMANDMENT.into()];r.player.active=if mode==0{vec![]}else{r.player.abilities.clone()};r.player.energy=0.;r.player.ability_reserves=crate::Field::Missing;r.inventory.clear();
   // The paid push still costs 20: ten reserve plus ten main energy in R52.
   if mode==1 {r.player.energy=10.;}
   crate::revision50_ability_energy::initialize_reserve(&mut r.player,COMMANDMENT,10.,if mode==1{10.}else{0.}).unwrap();
   let mut target=r.player.clone();target.id=900.;target.x=3.;target.state="hunting".into();target.abilities.clear();target.active.clear();target.ability_reserves=crate::Field::Missing;r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(target.clone());
   let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;let w=f.width();f.tiles[2*w+6]="wall".into();
   let a=r.player.clone();crate::events::begin();after_attack(&mut r,&a,&target,true).unwrap();let e=crate::events::finish();let styles:Vec<_>=e.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="P60").map(|a|a.style.as_str()).collect();
   assert_eq!(styles,if mode==1{vec!["moving-actor-commandment"]}else{vec![]});
   if mode==1 {let cue=e.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal=="P60").unwrap();assert_eq!(cue.cells.iter().map(|p|p.x).collect::<Vec<_>>(),vec![3.,4.,5.]);}
   assert_eq!(engine::actor(&r,900.).unwrap().x,if mode==1{5.}else{4.});
   let reserve=&r.player.ability_reserves.as_ref().unwrap()[COMMANDMENT];assert_eq!((reserve.capacity,reserve.energy,r.player.energy),(10.,0.,0.));
  }
 }
}

#[cfg(test)] mod commandment_route_receipt_tests {
 use super::*;
 #[test] fn grouped_receipt_keeps_only_observed_route_cells_and_portal_gap(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.conditions.clear();
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);
  let p=|x|crate::events::EventPoint{x,y:2.};
  crate::events::begin();let mut route=crate::events::TravelRoute::default();
  route.capture(&r,&r.player,&[p(2.),p(3.)],"@");
  route.capture(&r,&r.player,&[p(3.),p(-1.),p(8.)],"@");
  // The old-floor receipt remains valid even if an arrival changes floors.
  let depth=r.depth;r.depth+=1.;route.finish();let events=crate::events::finish();
  assert_eq!(events.len(),1);assert_eq!(events[0].depth,depth);
  assert_eq!(events[0].animation.as_ref().unwrap().cells.iter().map(|p|p.x).collect::<Vec<_>>(),vec![2.,3.,8.]);
 }
}

#[cfg(test)] mod execute_ten_energy_tests {
    use super::*;
    #[test] fn execute_spends_ten_reserve_first_and_preserves_threshold_and_legacy_cost(){
        for(reserve,energy,expected,remain)in[(10.,0.,true,0.),(5.,5.,true,0.),(0.,10.,true,0.),(5.,4.,false,4.)]{
            let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision=52.0.into();r.execute_rules_from=crate::Field::Null;r.player.abilities=vec![EXECUTE.into()];r.player.active=vec![EXECUTE.into()];r.player.energy=energy;
            crate::revision50_ability_energy::initialize_reserve(&mut r.player,EXECUTE,10.,reserve).unwrap();
            let a=r.player.clone();let mut b=Actor{id:9999.,state:"hunting".into(),hp:24.,max_hp:100.,..Default::default()};
            assert_eq!(lethal_hit(&mut r,&a,&b),expected);assert_eq!(r.player.energy,remain);
            assert_eq!(r.player.ability_reserves.as_ref().unwrap()[EXECUTE].energy,if expected{0.}else{reserve});
            b.hp=25.;assert!(!lethal_hit(&mut r,&a,&b));
        }
        assert_eq!(execute_cost(51.),20.);assert_eq!(execute_cost(52.),10.);
    }
}

#[cfg(test)] mod slippery_presentation_tests {
 use super::*;
 #[test] fn activation_box_remains_but_paid_block_has_no_repeat(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let id="isheilah-make-slippery";
  r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.conditions.clear();r.player.active.clear();r.player.abilities=vec![id.into()];r.inventory.clear();
  r.player.energy=100.;crate::revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,0.).unwrap();
  let mut target=r.player.clone();target.id=990.;target.x=3.;target.state="hunting".into();target.abilities.clear();target.active.clear();
  r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(target.clone());
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;
  let before=crate::animation_player::Snapshot::capture(&r).unwrap();
  r.player.active.push(id.into());r.player.conditions.insert("slipperyTarget".into(),target.id);
  crate::events::begin();before.finish(&r);let activation=crate::events::finish();
  assert!(activation.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.proposal=="P15"&&a.style=="box-contract"));
  crate::events::begin();assert!(!engine::move_actor(&mut r,target.id,0.,1.,true).unwrap());let blocked=crate::events::finish();
  assert!(!blocked.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.proposal=="P15"));
  let after=engine::actor(&r,target.id).unwrap();assert_eq!((after.x,after.y),(target.x,target.y));assert_eq!(r.player.energy,92.);
 }
}
