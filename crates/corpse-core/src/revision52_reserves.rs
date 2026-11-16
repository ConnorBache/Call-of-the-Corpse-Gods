//! R52 reserve ceiling. Load/restore migration never grants energy or touches the
//! player's energy bar. Low-level R50 APIs remain available for frozen replays.
use crate::{Actor, Item, Run};
use crate::revision50_ability_energy::{self as abilities, AbilityReserve};
pub const CAPACITY: f64 = 10.;
pub fn capacity(revision: f64) -> f64 { if revision >= 52. { CAPACITY } else { 50. } }
/// Pure display/payment projection, including pre-amendment R52 saved values.
/// Callers validate first; invalid state is never repaired into usable energy.
pub fn project_reserve(mut reserve: AbilityReserve, revision: f64) -> AbilityReserve {
    if revision >= 52. && abilities::valid_reserve(&reserve) {
        reserve.capacity = reserve.capacity.min(CAPACITY);
        reserve.energy = reserve.energy.min(reserve.capacity);
    }
    reserve
}
pub fn normalize_actor_reserves(actor: &mut Actor, revision: f64) {
    if revision < 52. { return; }
    if let Some(reserves) = actor.ability_reserves.as_mut() {
        for reserve in reserves.values_mut() { *reserve = project_reserve(*reserve, revision); }
    }
}
pub fn normalize_item(item: &mut Item, revision: f64) {
    if revision >= 52. && crate::revision50_amulets::valid_item_state(item, revision) {
        if let Some(state) = item.amulet_state.as_mut() { state.energy = state.energy.min(CAPACITY); }
    }
}
fn check_item(item: &Item) -> Result<(), String> {
    crate::revision50_amulets::valid_item_state(item, 52.).then_some(())
        .ok_or_else(|| "Invalid amulet reserve at R52 normalization".into())
}
fn check_actor(actor: &Actor) -> Result<(), String> {
    abilities::validate_reserves(actor).map_err(|e| format!("Invalid ability reserve at R52 normalization: {e:?}"))?;
    for item in actor.copied_inventory.as_ref().into_iter().flatten() { check_item(item)?; }
    if let Some(state) = actor.monster_state.as_ref() {
        for item in state.carried_items.as_ref().into_iter().flatten() { check_item(item)?; }
        if let Some(pilot) = state.pilot.as_ref() { check_actor(pilot)?; }
    }
    Ok(())
}
fn item_needs_normalize(item: &Item) -> bool {
    item.amulet_state.as_ref().is_some_and(|s| s.energy > CAPACITY)
}
fn actor_needs_normalize(actor: &Actor) -> bool {
    actor.ability_reserves.as_ref().is_some_and(|m| m.values().any(|s| s.capacity > CAPACITY))
        || actor.copied_inventory.as_ref().is_some_and(|v| v.iter().any(item_needs_normalize))
        || actor.monster_state.as_ref().is_some_and(|s|
            s.carried_items.as_ref().is_some_and(|v| v.iter().any(item_needs_normalize))
            || s.pilot.as_ref().is_some_and(|p| actor_needs_normalize(p)))
}
fn normalize_actor(actor: &mut Actor) {
    normalize_actor_reserves(actor, 52.);
    for item in actor.copied_inventory.as_mut().into_iter().flatten() { normalize_item(item, 52.); }
    if let Some(state) = actor.monster_state.as_mut() {
        for item in state.carried_items.as_mut().into_iter().flatten() { normalize_item(item, 52.); }
        if let Some(pilot) = state.pilot.as_mut() { normalize_actor(pilot); }
    }
}
/// Boundary-only O(A+B+I+K) scan, A actors (including pilots), B reserve entries,
/// I physical/carried/debug items and K remembered/detected items. Invalid later
/// entries reject before any write. COW actor/ground blocks detach only if their
/// reserve values change; cost includes copying each changed block's payload.
/// Future anchors stay immutable and normalize when restored, not recursively.
pub fn normalize_run(run: &mut Run) -> Result<(), String> {
    if run.simulation_revision.value_or(29.) < 52. { return Ok(()); }
    check_actor(&run.player)?;
    for actors in run.actors.values() { for actor in actors.iter() { check_actor(actor)?; } }
    for item in &run.inventory { check_item(item)?; }
    for items in run.ground.values() { for item in items.iter() { check_item(&item.item)?; } }
    for items in run.remembered_items.as_ref().into_iter().flat_map(|m| m.values()) {
        for item in items { check_item(&item.item)?; }
    }
    for items in run.detected_items.as_ref().into_iter().flat_map(|m| m.values()) {
        for item in items { check_item(&item.item)?; }
    }
    if let Some(profile) = run.playtest.as_ref() {
        for item in &profile.candidates { check_item(item)?; }
        if let Some(debug) = &profile.debug {
            for reward in debug.pending.iter().chain(&debug.stash) { check_item(&reward.item)?; }
        }
    }
    normalize_actor(&mut run.player);
    for actors in run.actors.values_mut() {
        if actors.iter().any(actor_needs_normalize) {
            for actor in crate::mutation_journal::make_mut(actors) { normalize_actor(actor); }
        }
    }
    for item in &mut run.inventory { normalize_item(item, 52.); }
    for items in run.ground.values_mut() {
        if items.iter().any(|item| item_needs_normalize(&item.item)) {
            for item in crate::mutation_journal::make_mut(items) { normalize_item(&mut item.item, 52.); }
        }
    }
    for items in run.remembered_items.as_mut().into_iter().flat_map(|m| m.values_mut()) {
        for item in items { normalize_item(&mut item.item, 52.); }
    }
    for items in run.detected_items.as_mut().into_iter().flat_map(|m| m.values_mut()) {
        for item in items { normalize_item(&mut item.item, 52.); }
    }
    if let Some(profile) = run.playtest.as_mut() {
        for item in &mut profile.candidates { normalize_item(item, 52.); }
        if let Some(debug) = &mut profile.debug {
            for reward in debug.pending.iter_mut().chain(&mut debug.stash) { normalize_item(&mut reward.item, 52.); }
        }
    }
    Ok(())
}
