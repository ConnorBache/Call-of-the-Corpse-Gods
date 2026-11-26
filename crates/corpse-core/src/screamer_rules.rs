//! Screamer scheduled scream lockout amendment with deterministic old command prefixes.
use crate::{Field, Run};

pub const RADIUS: f64 = 10.;

pub fn range_required(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.screamer_range_from, Field::Value(_))
}

/// A player placeholder in offscreen simulation is not a target. Use the
/// same plane and portal-aware distance as the scream's actual effects.
pub(crate) fn can_reach_player(r: &Run, screamer: &crate::Actor) -> bool {
    !range_required(r) || (r.player.x >= 0. && r.player.hp > 0.
        && !r.player.dead.value_or(false) && crate::dreamwalking::same_plane(r, screamer, &r.player)
        && crate::portals::actor_distance(r, screamer, &r.player) <= RADIUS)
}

pub(crate) fn validate_range(r: &Run) -> Result<(), String> {
    if !r.screamer_range_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.screamer_range_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Screamer range command boundary".into());
    }
    Ok(())
}

pub fn admit_range(r: &mut Run) -> Result<(), String> {
    validate_range(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.screamer_range_from.is_missing() {
        r.screamer_range_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_range_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) {
        r.screamer_range_from = cursor.into();
    } else if cursor == 0 {
        r.screamer_range_from = Field::Missing;
    }
}
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 50. && !matches!(r.screamer_rules_from, Field::Null)
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.screamer_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 50.
        || r.screamer_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid Screamer rules command boundary".into());
    }
    Ok(())
}
/// O(1). Live old saves adopt new rules at the next command, retaining their ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 50. && r.screamer_rules_from.is_missing() {
        r.screamer_rules_from = r.replay.len().into();
    }
    Ok(())
}
/// O(1). Apply before startup and every recorded transition to preserve historical scream outcomes.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 50. { return; }
    if from.as_ref() == Some(&cursor) { r.screamer_rules_from = cursor.into(); }
    else if cursor == 0 { r.screamer_rules_from = Field::Null; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewind_does_not_rewind_screamer_policy() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.screamer_rules_from = Field::Null;
        let snapshot = crate::temporal::Snapshot::capture(&r);
        assert!(snapshot.value().unwrap().get("screamerRulesFrom").is_none());
        r.screamer_rules_from = 0usize.into();
        snapshot.restore(&mut r).unwrap();
        assert_eq!(r.screamer_rules_from, Field::Value(0));
        assert!(current(&r));
    }
}
