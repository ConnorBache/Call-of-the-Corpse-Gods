//! Straight Minotaur charge admission at a replay-compatible command boundary.
use crate::{Field, Run};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.minotaur_straight_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.minotaur_straight_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.minotaur_straight_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Minotaur straight charge command boundary".into());
    }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.minotaur_straight_from.is_missing() {
        r.minotaur_straight_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.minotaur_straight_from = cursor.into(); }
    else if cursor == 0 { r.minotaur_straight_from = Field::Missing; }
}
