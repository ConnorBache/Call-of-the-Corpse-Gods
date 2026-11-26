//! Local floor simulation horizon and its replay boundary.
use crate::{Field, Run};

#[derive(Clone, Copy)]
pub(crate) struct Window {
    active: f64,
    above: f64,
    below: f64,
    limited: bool,
}
impl Window {
    pub(crate) fn contains(self, depth: f64) -> bool {
        !self.limited || depth == self.active || depth == self.above || depth == self.below
    }
}
/// Legacy routes may skip internal floors, so use physical neighbors.
pub(crate) fn window(r: &Run) -> Window {
    let limited = current(r);
    Window { active: r.depth, limited,
        above: if limited { crate::fast_mode::above(r, r.depth) } else { r.depth },
        below: if limited { crate::fast_mode::below(r, r.depth) } else { r.depth }}
}

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.local_floor_ticks_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.local_floor_ticks_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.local_floor_ticks_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Local floor ticks command boundary".into());
    }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.local_floor_ticks_from.is_missing() {
        r.local_floor_ticks_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.local_floor_ticks_from = cursor.into(); }
    else if cursor == 0 { r.local_floor_ticks_from = Field::Missing; }
}
