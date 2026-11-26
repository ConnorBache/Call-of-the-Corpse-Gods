//! Banshee shriek command boundary for live saves and historical replays.
use crate::{Field, Run};

pub const WHITE_WAVE: &str = "wave-reveal-banshee-white";
pub const PURPLE_WAVE: &str = "wave-banshee-purple";
pub fn damage(current_hp: f64) -> f64 { current_hp * 0.5 + 100. }

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.banshee_shriek_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.banshee_shriek_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.banshee_shriek_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Banshee shriek command boundary".into());
    }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.banshee_shriek_from.is_missing() {
        r.banshee_shriek_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.banshee_shriek_from = cursor.into(); }
    else if cursor == 0 { r.banshee_shriek_from = Field::Missing; }
}
