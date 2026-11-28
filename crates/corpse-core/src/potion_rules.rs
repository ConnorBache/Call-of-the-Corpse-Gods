//! Potion roster/mechanics amendment with deterministic old command prefixes.
use crate::{Field, Run};
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && !matches!(r.potion_rules_from, Field::Null)
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.potion_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || r.potion_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid potion rules command boundary".into());
    }
    if !r.potion_retirements_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.potion_retirements_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid potion retirement command boundary".into());
    }
    Ok(())
}
/// O(1). Live old saves adopt new rules at the next command, retaining their ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.potion_rules_from.is_missing() {
        r.potion_rules_from = r.replay.len().into();
    }
    Ok(())
}
/// O(1). Must also run BEFORE seed-world generation, since the loot roster differs.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) { r.potion_rules_from = cursor.into(); }
    else if cursor == 0 { r.potion_rules_from = Field::Null; }
}

#[cfg(test)]
#[path = "potion_rules_tests.rs"]
mod tests;

/// Separate policy so old runs preserve both the previous roster and RNG mapping.
pub fn retirements_current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.potion_retirements_from, Field::Value(_))
}
pub fn admit_retirements(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.potion_retirements_from.is_missing() {
        r.potion_retirements_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_retirement_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.potion_retirements_from = cursor.into(); }
    else if cursor == 0 { r.potion_retirements_from = Field::Missing; }
}
