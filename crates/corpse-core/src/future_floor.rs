//! Once-per-floor, floor-entry rewind. The used-floor ledger lives outside the
//! rewind image; capture strips this module's anchor to prevent recursive saves.
use crate::{Field, Run, future_anchor::FutureAnchor};
use serde::{Serialize, Deserialize};
pub const ID: &str = "morae-see-the-future";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
pub struct State {
    pub depth: f64,
    pub anchor: Option<FutureAnchor>,
    #[serde(default)]
    pub used_floors: Vec<f64>,
}
pub fn enabled(r:&Run)->bool {r.simulation_revision.value_or(29.)>=52. && !matches!(r.future_floor_rules_from,Field::Null)}
pub fn used(r:&Run,depth:f64)->bool {r.future_floor.as_ref().is_some_and(|s|s.used_floors.contains(&depth))}
pub fn active(r:&Run)->bool {enabled(r) && r.player.abilities.iter().any(|a|a==ID) && !used(r,r.depth)}
pub fn available(r:&Run)->bool {active(r) && r.future_floor.as_ref().is_some_and(|s|s.depth==r.depth && s.anchor.is_some())}
pub fn validate(r:&Run)->Result<(),String>{
    if !r.future_floor_rules_from.is_missing() && (r.simulation_revision.value_or(29.)<52. || r.future_floor_rules_from.as_ref().is_some_and(|&n|n>r.replay.len())) {return Err("Invalid floor Future rules command boundary".into());}
    if let Some(s)=&r.future_floor {if !s.depth.is_finite() || s.depth<1. || s.depth.fract()!=0. || s.used_floors.iter().any(|d|!d.is_finite()||*d<1.||d.fract()!=0.) {return Err("Invalid floor Future state".into());}}
    Ok(())
}
pub fn admit_current(r:&mut Run)->Result<(),String>{validate(r)?;if r.simulation_revision.value_or(29.)>=52. && r.future_floor_rules_from.is_missing(){r.future_floor_rules_from=r.replay.len().into();}Ok(())}
pub fn apply_replay_policy(r:&mut Run,from:&Field<usize>,cursor:usize){
    if r.simulation_revision.value_or(29.)<52. {return;}
    if from.as_ref()==Some(&cursor){r.future_floor_rules_from=cursor.into();}
    else if cursor==0 {r.future_floor_rules_from=Field::Null;r.future_floor=None;}
}
/// Validate imported checkpoint bytes once at engine admission, not every turn.
pub fn validate_anchor(r:&Run)->Result<(),String>{
    if enabled(r) {if let Some(s)=&r.future_floor {if let Some(anchor)=&s.anchor {
        let saved=anchor.prepare_restore().map_err(|e|e.to_string())?;
        if saved.depth!=s.depth || saved.simulation_revision!=r.simulation_revision {return Err("Floor Future anchor does not match current floor rules".into());}
    }}}
    Ok(())
}
/// O(S) finite validation and metadata cloning per floor entry, sharing large COW
/// blocks. One anchor only; returning to a floor does not reset its spent charge.
pub fn capture_entry(r:&mut Run)->Result<(),String>{
    if !enabled(r) || r.status!="playing" || r.player.hp<=0. || crate::debug_floor::preparing(r) {return Ok(());}
    let used_floors=r.future_floor.as_ref().map(|s|s.used_floors.clone()).unwrap_or_default();
    let anchor=if used_floors.contains(&r.depth){None}else{Some(FutureAnchor::capture41(r).map_err(|e|e.to_string())?)};
    r.future_floor=Some(State{depth:r.depth,anchor,used_floors});r.future=None;
    Ok(())
}
/// Startup/import fallback: pre-amendment saves have no historical floor-entry
/// image, so the loaded position becomes their first checkpoint, not a fake past.
pub fn ensure_entry(r:&mut Run)->Result<(),String>{
    if enabled(r) && r.future_floor.as_ref().is_none_or(|s|s.depth!=r.depth){capture_entry(r)?;}
    Ok(())
}
/// O(S) restore/validation, O(D) spent-floor transfer (D <= number of floors).
/// The replay ledger and policy survive; all world effects and resources rewind.
pub fn restore(r:&mut Run)->Result<bool,String>{
    if !available(r){return Ok(false);}
    let state=r.future_floor.as_ref().unwrap();
    let mut next=state.anchor.as_ref().unwrap().prepare_restore().map_err(|e|e.to_string())?;
    if next.depth!=r.depth || next.simulation_revision!=r.simulation_revision{return Err("Floor Future anchor does not match current floor rules".into());}
    let mut used_floors=state.used_floors.clone();used_floors.push(r.depth);
    next.future_floor=Some(State{depth:r.depth,anchor:None,used_floors});
    next.future=None;
    next.replay=std::mem::take(&mut r.replay);
    next.replay_snapshot_from=r.replay_snapshot_from.clone();
    next.firebolt_travel_from=r.firebolt_travel_from.clone();
    next.amulet_rules_from=r.amulet_rules_from.clone();
    next.potion_rules_from=r.potion_rules_from.clone();
    next.ring_rules_from=r.ring_rules_from.clone();
    next.gas_rules_from=r.gas_rules_from.clone();
    next.commandment_rules_from=r.commandment_rules_from.clone();
    next.screamer_rules_from=r.screamer_rules_from.clone();
    next.ragelight_rules_from=r.ragelight_rules_from.clone();
    next.balance_rules_from=r.balance_rules_from.clone();
    next.collapse_rules_from=r.collapse_rules_from.clone();
    next.blood_mist_fire_rules_from=r.blood_mist_fire_rules_from.clone();
    next.half_here_lock_from=r.half_here_lock_from.clone();
    next.half_here_energy_from=r.half_here_energy_from.clone();
    next.screamer_range_from=r.screamer_range_from.clone();
    next.death_gas_rules_from=r.death_gas_rules_from.clone();
    next.minotaur_bump_from=r.minotaur_bump_from.clone();
    next.borge_throw_from=r.borge_throw_from.clone();
    next.banshee_shriek_from=r.banshee_shriek_from.clone();
    next.local_floor_ticks_from=r.local_floor_ticks_from.clone();
    next.minotaur_straight_from=r.minotaur_straight_from.clone();
    next.mercuria_amulet_from=r.mercuria_amulet_from.clone();
    next.execute_rules_from=r.execute_rules_from.clone();
    next.beckoning_rules_from=r.beckoning_rules_from.clone();
    next.ichor_fire_rules_from=r.ichor_fire_rules_from.clone();
    next.avatar_starter_policy=r.avatar_starter_policy.clone();
    next.ember_rules_from=r.ember_rules_from.clone();
    next.steam_rules_from=r.steam_rules_from.clone();
    next.future_floor_rules_from=r.future_floor_rules_from.clone();
    next.run_id=r.run_id.clone();
    next.actors.adopt_epoch_from(&r.actors);
    *r=next;crate::health_dirty::invalidate(r);
    crate::engine::log(r,"You use See The Future.","player");
    Ok(true)
}
#[cfg(test)] mod tests;
