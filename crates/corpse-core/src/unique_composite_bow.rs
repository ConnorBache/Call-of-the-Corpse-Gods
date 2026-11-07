//! Revision 51's one generated Composite bow per run, not per floor or owner.
use crate::{Actor, Item, Run};

pub(crate) fn is_composite(item: &Item) -> bool {
    item.kind == "weapon" && item.ranged_weapon.as_ref().is_some_and(|s| s == "composite-bow")
}

fn actor_has_bow(actor: &Actor) -> bool {
    actor.copied_inventory.as_ref().is_some_and(|items| items.iter().any(is_composite))
        || actor.monster_state.as_ref().is_some_and(|state|
            state.carried_items.as_ref().is_some_and(|items| items.iter().any(is_composite))
            || state.pilot.as_ref().is_some_and(|pilot| actor_has_bow(pilot)))
}

/// O(I+A+G+M+D) once for missing-marker imports: all inventory items, actors and
/// nested pilots, ground/memory items and debug candidates. O(pilot depth) stack;
/// no cloning, RNG, item mutation or removal. Normal persisted runs are O(1).
pub(crate) fn initialize(run: &mut Run) {
    if run.simulation_revision.value_or(29.) < 51. || run.composite_bow_generated.as_ref().is_some() {
        return;
    }
    let found = run.inventory.iter().any(is_composite)
        || std::iter::once(&run.player).chain(run.initial_player.as_ref())
            .chain(run.actors.values().flat_map(|actors| actors.iter())).any(actor_has_bow)
        || run.ground.values().flat_map(|items| items.iter()).any(|g| is_composite(&g.item))
        || run.remembered_items.as_ref().is_some_and(|floors|
            floors.values().flatten().any(|g| is_composite(&g.item)))
        || run.detected_items.as_ref().is_some_and(|floors|
            floors.values().flatten().any(|g| is_composite(&g.item)))
        || run.playtest.as_ref().is_some_and(|profile|
            profile.candidates.iter().any(is_composite)
            || profile.debug.as_ref().is_some_and(|debug|
                debug.stash.iter().chain(&debug.pending).any(|reward| is_composite(&reward.item))));
    run.composite_bow_generated = found.into();
}

#[cfg(test)]
#[path = "unique_composite_bow_tests.rs"]
mod tests;
