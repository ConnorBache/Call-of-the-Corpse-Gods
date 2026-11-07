//! Explicit actor-local equipment lookup. Stats-only Mini-Me has no inventory.
use crate::{Actor, Item, Run};
/// O(1) slice lookup; no allocation or fallback to another actor's inventory.
pub fn inventory<'a>(r: &'a Run, actor: &'a Actor) -> &'a [Item] {
    if actor.id == r.player.id { &r.inventory }
    else if r.simulation_revision.value_or(29.0)>=50.0 && actor.conditions.get("miniMe")==Some(&1.0) { &[] }
    else if r.simulation_revision.value_or(29.0) >= 50.0 {
        actor.copied_inventory.as_ref().map(Vec::as_slice).unwrap_or(&[])
    } else { &[] }
}
/// O(I+G) grant traversal; snapshot abilities already contain these powers.
pub fn grants<'a>(r: &'a Run, actor: &'a Actor) -> impl Iterator<Item=&'a String> {
    inventory(r,actor).iter().filter(|i|crate::revision50_inventory::active(r,i)).flat_map(crate::tinker::grants)
}
