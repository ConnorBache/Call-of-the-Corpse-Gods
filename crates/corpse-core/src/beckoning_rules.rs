//! Successful player-side Beckoning consumes the target enemy next action in live R52.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && !matches!(run.beckoning_rules_from, Field::Null)
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.beckoning_rules_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.beckoning_rules_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid Beckoning command boundary".into());
    }
    Ok(())
}

/// An old live save adopts Beckoning action-skip rules after its existing command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if current(run) && run.beckoning_rules_from.is_missing() {
        run.beckoning_rules_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.beckoning_rules_from = cursor.into();
    } else if cursor == 0 {
        run.beckoning_rules_from = Field::Null;
    }
}


#[cfg(test)]mod tests {
 use super::*;
 #[test]fn beckoning_policy_survives_floor_future_snapshot_and_import(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.beckoning_rules_from=Field::Null;r.inventory.clear();
  r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
  r.player.ability_reserves=Field::Missing;r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
  crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
  let snapshot=crate::temporal::Snapshot::capture(&r);
  assert!(snapshot.value().unwrap().get("beckoningRulesFrom").is_none());
  for marker in [serde_json::Value::Null,serde_json::json!(1)] {
   let mut v=snapshot.value().unwrap();v["beckoningRulesFrom"]=marker;
   let imported=crate::temporal::Snapshot::from_value(v).unwrap();
   assert!(imported.value().unwrap().get("beckoningRulesFrom").is_none());
  }
  r.player.abilities.push(crate::future_floor::ID.into());crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
  crate::future_floor::ensure_entry(&mut r).unwrap();r.replay.push(crate::Command::Wait);r.beckoning_rules_from=Field::Value(1);
  assert!(crate::future_floor::restore(&mut r).unwrap());assert_eq!(r.beckoning_rules_from,Field::Value(1));
  snapshot.restore(&mut r).unwrap();assert_eq!(r.beckoning_rules_from,Field::Value(1));
 }
}
