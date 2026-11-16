//! Revision 50 integration boundary. The new revision is not enabled for new runs
//! until every required subsystem is implemented and the native suite is run.
use crate::{Actor, Field, Run, engine::{self, EngineError}};
pub const DUEL: &str = "pelian-fated-duel";
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 50. }
pub fn toggle(id: &str) -> bool {
    matches!(id,"shrail-execute"|"shrail-grappler"|"shrail-unstoppable-force"|"anavani-commandment"|DUEL)
}
pub fn supported(id: &str) -> bool {
    toggle(id) || id==crate::revision50_combat::MASOCHISM || id==crate::revision50_terrain::CLAY_ARMY || id==crate::revision50_terrain::COGWORKS || id==crate::revision50_nightmares::ID || id==crate::revision50_pursuit::ID || matches!(id,"isheilah-spellslinger"|"kative-appropriate"|"kative-bottomless-pack"|"grisha-witch-hatred"|"ursa-oblivious")
}
pub fn condition_key(k: &str) -> bool {
    crate::abilities::upkeep_payment_condition(k) || crate::revision50_potions::condition_key(k) || crate::revision50_amulet_effects::condition_key(k) || crate::revision50_control_amulets::condition_key(k) || k=="r50PursuitCredit" || k==crate::revision50_terrain::MODE_KEY || [crate::revision50_nightmares::MARKER,crate::revision50_nightmares::OWNER].contains(&k) || crate::revision50_pursuit::condition_key(k) || matches!(k,"r50DuelTarget"|"r50DuelX"|"r50DuelY"|"r50DuelDepth")
}
fn has(a: &Actor, id: &str) -> bool { a.abilities.iter().any(|s| s == id) }
fn duel_area(a: &Actor, depth: f64) -> bool {
    let (Some(&anchor_depth),Some(&x),Some(&y))=(a.conditions.get("r50DuelDepth"),
        a.conditions.get("r50DuelX"),a.conditions.get("r50DuelY")) else {return false;};
    anchor_depth==depth && (a.x-x).abs()<=2.0 && (a.y-y).abs()<=2.0
}
fn expire_duel(a: &mut Actor) {
    a.active.retain(|s|s!=DUEL);
    for key in ["r50DuelTarget","r50DuelX","r50DuelY","r50DuelDepth"] {a.conditions.shift_remove(key);}
}
/// Revision-50 callers use this when assigning a detached copy its spawn cell.
pub fn settle_actor_duel(a: &mut Actor, depth: f64) {
    if a.active.iter().any(|s|s==DUEL) && !duel_area(a,depth) {expire_duel(a);}
}
/// O(A+B) scans across persisted owners; only blocks with an expired owner detach
/// from COW snapshots. Each owner retains its own anchor, target and paid upkeep.
pub fn settle_duel(r: &mut Run) {
    if duel_target_rules(r) { settle_current_duels(r); return; }
    if !enabled(r) {return;}
    if r.player.active.iter().any(|s|s==DUEL) && !duel_area(&r.player,r.depth) {expire_duel(&mut r.player);}
    for (depth,block) in &mut r.actors {
        let Ok(depth)=depth.parse::<f64>() else {continue;};
        let expired=|a:&Actor| !crate::half_here::is_clone(a)
            && a.active.iter().any(|s|s==DUEL) && !duel_area(a,depth);
        if block.iter().any(expired) {
            for a in crate::mutation_journal::make_mut(block) {if expired(a) {expire_duel(a);}}
        }
    }
}
/// R52 ends the selected-target tether; older replay revisions retain owner-only expiry.
pub(crate) fn duel_target_rules(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 52. }
fn duel_point(owner: &Actor, depth: f64, x: f64, y: f64) -> bool {
    owner.conditions.get("r50DuelDepth") == Some(&depth)
        && owner.conditions.get("r50DuelX").is_some_and(|anchor| (x-anchor).abs() <= 2.0)
        && owner.conditions.get("r50DuelY").is_some_and(|anchor| (y-anchor).abs() <= 2.0)
}
/// O(d) clean global index lookup; includes the actor's actual persisted floor.
/// Dirty reconciliation has the creature index's documented world/slot costs.
fn duel_actor(r: &Run, id: f64) -> Option<(f64, &Actor)> {
    if id == r.player.id { return Some((r.depth, &r.player)); }
    let (floor, at) = r.actors.position(id, r.player.id, r.depth, false).ok()??;
    let (depth, actors) = r.actors.get_index(floor)?;
    Some((depth.parse().ok()?, actors.get(at)?))
}
fn duel_target_valid(r: &Run, owner: &Actor) -> bool {
    owner.conditions.get("r50DuelTarget").and_then(|id| duel_actor(r, *id))
        .is_some_and(|(depth, target)| target.hp > 0.0 && !target.dead.value_or(false)
            && duel_point(owner, depth, target.x, target.y))
}
fn duel_invalid(r: &Run, owner: &Actor, depth: f64) -> bool {
    !crate::half_here::is_clone(owner) && owner.active.iter().any(|id| id == DUEL)
        && (!duel_area(owner, depth) || !duel_target_valid(r, owner))
}
/// Read-only counterpart to the maintenance settlement gate.
pub fn duel_upkeep_valid(r:&Run,owner:&Actor)->bool {
    !duel_target_rules(r) || !duel_invalid(r,owner,r.depth)
}
fn clear_duel_owner(r: &mut Run, id: f64) {
    if let Some(owner) = engine::actor_any_mut(r, id) { expire_duel(owner); }
    if id == r.player.id { crate::half_here::sync(r); }
}
/// O(d + K + C) before one owner's upkeep, no owner scan. Expiry changes only
/// that owner (plus shared player endpoint projection), never another reserve.
pub(crate) fn settle_duel_owner(r: &mut Run, id: f64) {
    if !duel_target_rules(r) { return; }
    if duel_actor(r, id).is_some_and(|(depth, owner)| duel_invalid(r, owner, depth)) {
        clear_duel_owner(r, id);
    }
}
/// O((D+1)*(d+K+C)) with a clean creature index, D independent Duel candidates;
/// no world scan per move/attack. Structural reconciliation is shared with the
/// existing identity index; COW detaches only blocks with an expiring owner.
/// Call after each committed displacement, including every multi-step segment.
/// No path/terrain work and no RNG. Expiry is irreversible even after reentry.
pub(crate) fn duel_actor_changed(r: &mut Run, changed: f64) {
    if !duel_target_rules(r) { return; }
    let ids = r.actors.duel_owner_ids(r.player.id, r.depth).unwrap_or_default();
    for id in std::iter::once(r.player.id).chain(ids.iter().copied()) {
        let expired = duel_actor(r, id).is_some_and(|(depth, owner)|
            !crate::half_here::is_clone(owner) && owner.active.iter().any(|s| s == DUEL)
            && (owner.id == changed && !duel_area(owner, depth)
                || owner.conditions.get("r50DuelTarget") == Some(&changed) && !duel_target_valid(r, owner)));
        if expired { clear_duel_owner(r, id); }
    }
}
/// Actual death/removal notification, after rescue but before any revival.
/// Same candidate/index/COW bounds as duel_actor_changed; no world owner scan.
pub(crate) fn end_duels_with_target(r: &mut Run, target: f64) {
    if !duel_target_rules(r) { return; }
    let ids = r.actors.duel_owner_ids(r.player.id, r.depth).unwrap_or_default();
    for id in std::iter::once(r.player.id).chain(ids.iter().copied()) {
        if duel_actor(r, id).is_some_and(|(_, owner)| !crate::half_here::is_clone(owner)
            && owner.conditions.get("r50DuelTarget") == Some(&target)) { clear_duel_owner(r, id); }
    }
}
/// Actual-world boundary normalization, O((D+1)*(d+K+C)) clean plus the shared
/// index reconciliation/COW costs. Also normalizes loaded/rewound stale links.
pub(crate) fn settle_current_duels(r: &mut Run) {
    if !duel_target_rules(r) { return; }
    let ids = r.actors.duel_owner_ids(r.player.id, r.depth).unwrap_or_default();
    for id in std::iter::once(r.player.id).chain(ids.iter().copied()) { settle_duel_owner(r, id); }
}
/// Merge hidden-world target results without observing a temporarily staged
/// player position during travel admission. Same bounds as full normalization.
pub(crate) fn settle_duel_targets(r: &mut Run) {
    if !duel_target_rules(r) { return; }
    let ids = r.actors.duel_owner_ids(r.player.id, r.depth).unwrap_or_default();
    for id in std::iter::once(r.player.id).chain(ids.iter().copied()) {
        if duel_actor(r, id).is_some_and(|(_, owner)| !crate::half_here::is_clone(owner)
            && owner.active.iter().any(|s| s == DUEL) && !duel_target_valid(r, owner)) {
            clear_duel_owner(r, id);
        }
    }
}

pub fn duel_blocks(r: &Run, source: &Actor, target: &Actor) -> bool {
    if !enabled(r) {return false;}
    let owner_id=crate::half_here::owner_id(r,target);
    let owner=engine::actor(r,owner_id).unwrap_or(target);
    let allied=|a:&Actor| a.id==r.player.id || a.state=="ally";
    source.id!=owner.id && (allied(source)!=allied(owner) || crate::revision50_control_amulets::discord_hostility(r,source,owner))
        && owner.active.iter().any(|s|s==DUEL) && duel_area(owner,r.depth)
        && (!duel_target_rules(r) || duel_target_valid(r,owner))
        && owner.conditions.get("r50DuelTarget").is_some_and(|id|source.id!=*id)
}
/// O(A) actor selection plus existing visibility/attack cost. Every validation
/// precedes payment, target mutation, RNG or emitted combat events.
fn implant_thought(r:&mut Run,target:&Field<f64>,x:&Field<f64>,y:&Field<f64>)->Result<bool,EngineError>{
    let (Some(&tid),Some(&x),Some(&y))=(target.as_ref(),x.as_ref(),y.as_ref()) else {return Ok(false)};
    let Some(attacker)=engine::actor(r,tid).cloned() else {return Ok(false)};
    if tid==r.player.id || attacker.state=="ally" || attacker.state=="neutral"
        || attacker.hp<=0. || attacker.dead.value_or(false) || crate::ahvamitt::is_actor(&attacker)
        || crate::half_here::is_clone(&attacker) || !crate::vision::visible_actor(r,&attacker)
        || !crate::dreamwalking::same_plane(r,&r.player,&attacker) {return Ok(false)}
    let Some(victim)=std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|a|
        a.id!=tid && a.hp>0. && !a.dead.value_or(false)
        && crate::dreamwalking::same_plane(r,&attacker,a) && crate::portals::same_position(r,a,x,y)
        && crate::portals::actor_chebyshev(r,&attacker,a)<=1.).cloned() else {return Ok(false)};
    if engine::attack_blocker(r,&attacker,&victim,0.,false).is_some() {return Ok(false)}
    if crate::abilities::energy_activation_silenced(r,&r.player) || !crate::revision50_ability_energy::spend(r,r.player.id,"numo-implant-thought",15.) {return Ok(false)}
    crate::events::link(r,&r.player,&attacker,"P19","stitch-thought",crate::events::Stage::Special);
    crate::events::link(r,&attacker,&victim,"P19","notch-target",crate::events::Stage::Special);
    engine::attack(r,tid,victim.id,1.)?;
    engine::log(r,"You use Implant Thought.","player");
    Ok(true)
}
/// Some means this revision owns the branch; None falls through to unchanged
/// implemented mechanics. Unsupported changed powers fail before mutation.
pub fn activate(r:&mut Run,id:&str,target:&Field<f64>,x:&Field<f64>,y:&Field<f64>)->Result<Option<bool>,EngineError>{
    if !enabled(r) {return Ok(None)}
    if id.starts_with("craft:") {return Err(EngineError("Revision 50 removed general god-specific Craft blessings".into()))}
    if !has(&r.player,id) {return Ok(Some(false))}
    if id==crate::revision50_nightmares::ID {
        let (Some(&x),Some(&y))=(x.as_ref(),y.as_ref()) else{return Ok(Some(false))};
        crate::revision50_nightmares::create(r,x,y).map_err(EngineError)?;
        engine::log(r,"You create a waking nightmare.","player");return Ok(Some(false))
    }
    if id=="numo-implant-thought" {return implant_thought(r,target,x,y).map(Some)}
    if id==crate::revision50_terrain::CLAY_ARMY {
        let (Some(&x),Some(&y))=(x.as_ref(),y.as_ref()) else{return Ok(Some(false))};
        return crate::revision50_terrain::activate_clay(r,x,y).map(Some);
    }
    if id==crate::revision50_terrain::COGWORKS {
        if let (Some(&x),Some(&y))=(x.as_ref(),y.as_ref()) {
            return crate::revision50_terrain::activate_cogworks(r,x,y).map(Some);
        }
        if x.is_missing() && y.is_missing() {
            if let Some(&selected)=target.as_ref() {crate::revision50_terrain::select_mode(r,selected);}
        }
        return Ok(Some(false));
    }
    if id==crate::revision50_pursuit::ID {
        return Ok(Some(target.as_ref().is_some_and(|target|crate::revision50_pursuit::activate(r,*target))));
    }
    if toggle(id) {
        if r.player.active.iter().any(|s|s==id) {r.player.active.retain(|s|s!=id);return Ok(Some(false))}
        if id==DUEL {
            let Some(a)=target.as_ref().and_then(|id|engine::actor(r,*id)).cloned() else{return Ok(Some(false))};
            if a.id==r.player.id || a.state=="ally" || a.state=="neutral" || a.hp<=0. || a.dead.value_or(false)
                || !crate::dreamwalking::same_plane(r,&r.player,&a)
                || crate::portals::actor_chebyshev(r,&r.player,&a)>1. {return Ok(Some(false))}
            if crate::abilities::energy_activation_silenced(r,&r.player) || !crate::abilities::prepay_activation_upkeep(r,DUEL,3.) {return Ok(Some(false))}
            r.player.conditions.insert("r50DuelTarget".into(),a.id);
            r.player.conditions.insert("r50DuelX".into(),a.x);
            r.player.conditions.insert("r50DuelY".into(),a.y);
            r.player.conditions.insert("r50DuelDepth".into(),r.depth);
        }
        r.player.active.push(id.into());
        engine::log(r,format!("You activate {}.",crate::progression::definition(id).map_or(id,|a|a.name.as_str())),"player");
        return Ok(Some(false))
    }
    if supported(id) {return Ok(Some(false))}
    let changed = ["shrail-grappler","pelian-relentless-pursuit","isheilah-spellslinger","ursa-waking-nightmares","kative-cogworks"];
    if changed.contains(&id) {return Err(EngineError(format!("Revision 50 integration pending for {id}")))}
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.active.clear();r.player.abilities=vec![DUEL.into()];r
    }
    #[test] fn duel_is_five_by_five_and_expires_when_leaving() {
        let mut r=run();r.player.active.push(DUEL.into());
        let (x,y,depth)=(r.player.x,r.player.y,r.depth);
        r.player.conditions.insert("r50DuelX".into(),x);r.player.conditions.insert("r50DuelY".into(),y);r.player.conditions.insert("r50DuelDepth".into(),depth);
        for dx in -2..=2 {for dy in -2..=2 {r.player.x=x+dx as f64;r.player.y=y+dy as f64;assert!(duel_area(&r.player,r.depth));}}
        r.player.x=x+3.;settle_duel(&mut r);assert!(r.player.active.is_empty());
    }
    #[test] fn invalid_implant_does_not_spend_or_mutate() {
        let mut r=run();r.player.abilities.push("numo-implant-thought".into());let before=r.clone();
        assert!(!implant_thought(&mut r,&Field::Missing,&Field::Missing,&Field::Missing).unwrap());assert_eq!(r,before);
    }
    fn anchored_run()->Run {
        let mut r=run();r.inventory.clear();r.player.conditions.clear();r.actors_at_mut(r.depth).clear();
        r.next_id=900.0;r.player.active=vec![DUEL.into()];
        for (key,value) in [("r50DuelX",r.player.x),("r50DuelY",r.player.y),
            ("r50DuelDepth",r.depth),("r50DuelTarget",1000.0)] {r.player.conditions.insert(key.into(),value);}
        r
    }
    fn independent_duelist(r:&Run,x:f64,y:f64)->Actor {
        let mut a=r.player.clone();a.id=r.next_id;a.x=x;a.y=y;a.state="ally".into();
        settle_actor_duel(&mut a,r.depth);a
    }
    fn insert_duelist(r:&mut Run,a:Actor) {r.next_id+=1.0;r.actors_at_mut(r.depth).push(a);}
    #[test]
    fn independent_duel_preserves_target_and_anchor_but_expires_and_pays_independently() {
        let mut r=anchored_run();let (x,y)=(r.player.x,r.player.y);
        r.player.ability_reserves=Field::Missing;
        crate::revision50_ability_energy::initialize_reserve(&mut r.player,DUEL,6.0,6.0).unwrap();
        let a=independent_duelist(&r,x+1.0,y);let id=a.id;
        assert_eq!(a.conditions.get("r50DuelX"),Some(&x));
        assert_eq!(a.conditions.get("r50DuelTarget"),Some(&1000.0));
        insert_duelist(&mut r,a);
        let source=Actor{id:1001.0,state:"hunting".into(),..Default::default()};
        let chosen=Actor{id:1000.0,state:"hunting".into(),..Default::default()};
        let copy=engine::actor(&r,id).unwrap();
        assert!(duel_blocks(&r,&source,copy));assert!(!duel_blocks(&r,&chosen,copy));
        assert!(!duel_blocks(&r,&r.player,copy));
        crate::abilities::pay_upkeep(&mut r,id).unwrap();
        assert_eq!(engine::actor(&r,id).unwrap().ability_reserves.as_ref().unwrap()[DUEL].energy,3.0);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,6.0);
        r.player.x=x+3.0;settle_duel(&mut r);
        assert!(!r.player.active.iter().any(|s|s==DUEL));
        assert!(duel_blocks(&r,&source,engine::actor(&r,id).unwrap()));
        engine::actor_mut_trusted(&mut r,id).unwrap().x=x+3.0;settle_duel(&mut r);
        let copy=engine::actor(&r,id).unwrap();assert!(!copy.active.iter().any(|s|s==DUEL));
        assert!(!copy.conditions.contains_key("r50DuelTarget"));
    }
    #[test]
    fn shared_endpoint_uses_owner_area_and_outside_spawn_does_not_recenter_anchor() {
        let mut r=anchored_run();let (x,y)=(r.player.x,r.player.y);
        let source=Actor{id:1001.0,state:"hunting".into(),..Default::default()};
        let mut body=Actor{id:901.0,x:x+10.0,y,state:"ally".into(),..Default::default()};
        body.conditions.insert("halfHereClone".into(),1.0);
        body.conditions.insert("halfHereOwner".into(),r.player.id);
        assert!(duel_blocks(&r,&source,&body));
        let outside=independent_duelist(&r,x+3.0,y);
        assert!(!outside.active.iter().any(|s|s==DUEL));
        assert!(r.player.active.iter().any(|s|s==DUEL));
        r.player.x=x+3.0;assert!(!duel_blocks(&r,&source,&body));
        r.simulation_revision=49.0.into();let before=r.clone();settle_duel(&mut r);
        assert_eq!(r,before);assert!(!duel_blocks(&r,&source,&body));
    }
    #[test]
    fn owner_on_another_floor_uses_its_actual_depth_for_expiry() {
        let mut r=anchored_run();let other_depth=r.depth+1.0;
        let mut copy=independent_duelist(&r,r.player.x,r.player.y);
        copy.conditions.insert("r50DuelDepth".into(),other_depth);
        let id=copy.id;r.actors_at_mut(other_depth).push(copy);
        settle_duel(&mut r);
        assert!(r.actors_at(other_depth).iter().find(|a|a.id==id).unwrap().active.iter().any(|s|s==DUEL));
        r.actors_at_mut(other_depth).iter_mut().find(|a|a.id==id).unwrap().x+=3.0;
        settle_duel(&mut r);
        assert!(!r.actors_at(other_depth).iter().find(|a|a.id==id).unwrap().active.iter().any(|s|s==DUEL));
        assert!(r.player.active.iter().any(|s|s==DUEL));
    }
}

#[cfg(test)] mod implant_animation_fidelity_tests {
 use super::*;
 #[test] fn admitted_control_has_thought_and_victim_direction_invalid_target_is_quiet(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  r.player.x=2.;r.player.y=2.;r.player.abilities=vec!["numo-implant-thought".into()];r.player.active.clear();r.player.conditions.clear();r.player.ability_reserves=Field::Missing;r.inventory.clear();
  crate::revision50_ability_energy::initialize_reserve(&mut r.player,"numo-implant-thought",100.,100.).unwrap();
  let mut enemy=r.player.clone();enemy.id=900.;enemy.x=3.;enemy.state="hunting".into();enemy.abilities.clear();enemy.ability_reserves=Field::Missing;
  r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(enemy);let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=Field::Missing;f.non_euclidean=Field::Missing;
  crate::events::begin();assert!(!implant_thought(&mut r,&999.0.into(),&2.0.into(),&2.0.into()).unwrap());assert!(crate::events::finish().is_empty());
  crate::events::begin();assert!(implant_thought(&mut r,&900.0.into(),&2.0.into(),&2.0.into()).unwrap());let events=crate::events::finish();
  for (style,from,to) in [("stitch-thought",(2.,2.),(3.,2.)),("notch-target",(3.,2.),(2.,2.))] {
   let e=events.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P19"&&a.style==style)).unwrap();
   assert_eq!(e.from,Some(crate::events::EventPoint{x:from.0,y:from.1}));assert_eq!(e.to,crate::events::EventPoint{x:to.0,y:to.1});
  }
 }
}

#[cfg(test)]
#[path="revision52_duel_tests.rs"]
mod revision52_duel_tests;
