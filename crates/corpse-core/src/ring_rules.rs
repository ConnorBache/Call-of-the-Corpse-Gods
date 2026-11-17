//! Ring roster/mechanics amendment with deterministic old command prefixes.
use crate::{Field, Run};
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && !matches!(r.ring_rules_from, Field::Null)
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.ring_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || r.ring_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid ring rules command boundary".into());
    }
    Ok(())
}
/// O(1). Live old saves adopt new rules at the next command, retaining their ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.ring_rules_from.is_missing() {
        r.ring_rules_from = r.replay.len().into();
    }
    Ok(())
}
/// O(1). Must also run BEFORE seed-world generation, since the loot roster differs.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) { r.ring_rules_from = cursor.into(); }
    else if cursor == 0 { r.ring_rules_from = Field::Null; }
}

#[cfg(test)]
#[path = "ring_rules_tests.rs"]
mod tests;
