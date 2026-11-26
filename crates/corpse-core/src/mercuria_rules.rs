//! Mercuria reward policy and its replay boundary.
use crate::{Field, Run};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.mercuria_amulet_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.mercuria_amulet_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.mercuria_amulet_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Mercuria amulet command boundary".into());
    }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.mercuria_amulet_from.is_missing() {
        r.mercuria_amulet_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.mercuria_amulet_from = cursor.into(); }
    else if cursor == 0 { r.mercuria_amulet_from = Field::Missing; }
}
