//! Commandment landed-hit amendment with deterministic old command prefixes.
use crate::{Field, Run};
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 50. && !matches!(r.commandment_rules_from, Field::Null)
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.commandment_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 50.
        || r.commandment_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid Commandment rules command boundary".into());
    }
    Ok(())
}
/// O(1). Live old saves adopt new rules at the next command, retaining their ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 50. && r.commandment_rules_from.is_missing() {
        r.commandment_rules_from = r.replay.len().into();
    }
    Ok(())
}
/// O(1). Apply before startup and every recorded transition to preserve historical attack outcomes.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 50. { return; }
    if from.as_ref() == Some(&cursor) { r.commandment_rules_from = cursor.into(); }
    else if cursor == 0 { r.commandment_rules_from = Field::Null; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewind_does_not_rewind_commandment_policy() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.commandment_rules_from = Field::Null;
        let snapshot = crate::temporal::Snapshot::capture(&r);
        assert!(snapshot.value().unwrap().get("commandmentRulesFrom").is_none());
        r.commandment_rules_from = 0usize.into();
        snapshot.restore(&mut r).unwrap();
        assert_eq!(r.commandment_rules_from, Field::Value(0));
        assert!(current(&r));
    }
}
