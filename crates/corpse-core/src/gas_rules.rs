//! Gas status refresh amendment with deterministic old command prefixes.
use crate::{Field, Run};
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && !matches!(r.gas_rules_from, Field::Null)
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.gas_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || r.gas_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid gas rules command boundary".into());
    }
    Ok(())
}
/// O(1). Live old saves adopt new rules at the next command, retaining their ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.gas_rules_from.is_missing() {
        r.gas_rules_from = r.replay.len().into();
    }
    Ok(())
}
/// O(1). Apply before startup and every recorded transition to preserve gas-contact rules.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) { r.gas_rules_from = cursor.into(); }
    else if cursor == 0 { r.gas_rules_from = Field::Null; }
}

#[cfg(test)]
#[path = "gas_rules_tests.rs"]
mod tests;
