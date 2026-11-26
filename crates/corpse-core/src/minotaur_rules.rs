//! Replay boundary for committed-bump-only Minotaur attacks.
use crate::{Field, Run};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.minotaur_bump_from, Field::Value(_))
}

pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.minotaur_bump_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.minotaur_bump_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Minotaur bump command boundary".into());
    }
    Ok(())
}

pub(crate) fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.minotaur_bump_from.is_missing() {
        r.minotaur_bump_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) {
        r.minotaur_bump_from = cursor.into();
    } else if cursor == 0 {
        r.minotaur_bump_from = Field::Missing;
    }
}

/// Restrict actors driven by Minotaur AI, not the player’s direct controls.
pub(crate) fn only_bump(r: &Run, a: &crate::Actor) -> bool {
    (current(r) || crate::minotaur_straight::current(r)) && a.id != r.player.id && a.abilities.iter().any(|id| id == "enemy:beast:minotaur")
}
