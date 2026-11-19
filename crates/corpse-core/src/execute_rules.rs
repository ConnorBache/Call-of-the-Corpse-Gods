//! Current Execute triggers on an ordinary HP threshold crossing, not pre-hit HP.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && !matches!(run.execute_rules_from, Field::Null)
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.execute_rules_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.execute_rules_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid Execute command boundary".into());
    }
    Ok(())
}

/// Adopt the crossing rule after the existing live command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if current(run) && run.execute_rules_from.is_missing() {
        run.execute_rules_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.execute_rules_from = cursor.into();
    } else if cursor == 0 {
        run.execute_rules_from = Field::Null;
    }
}

