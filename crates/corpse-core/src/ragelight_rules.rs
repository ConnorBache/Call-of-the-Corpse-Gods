//! Ragelight is automatic in live R52 games. Historical command prefixes retain the toggle.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && !matches!(run.ragelight_rules_from, Field::Null)
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.ragelight_rules_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.ragelight_rules_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid Ragelight command boundary".into());
    }
    Ok(())
}

/// An old live save adopts passive Ragelight after its existing command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if current(run) && run.ragelight_rules_from.is_missing() {
        run.ragelight_rules_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.ragelight_rules_from = cursor.into();
    } else if cursor == 0 {
        run.ragelight_rules_from = Field::Null;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, divine_rescues::{self, RAGE}, engine, temporal::Engine};

    fn room(marker: Field<usize>) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.ragelight_rules_from = marker;
        r.player.abilities = vec![RAGE.into()];
        r.inventory.clear();
        r.player.active.clear();
        r.player.conditions.shift_remove("rageCost");
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        r
    }

    #[test]
    fn ragelight_passive_rescues_automatically_pays_reserve_first_and_resets_at_level_up() {
        let mut r = room(0usize.into());
        let player = r.player.id;
        r.player.ability_reserves.as_mut().unwrap().get_mut(RAGE).unwrap().energy = 1.;
        r.player.energy = 2.;
        for (cost, reserve, energy) in [(1., 1., 2.), (2., 0., 2.), (4., 0., 0.)] {
            r.player.hp = 0.;
            assert!(divine_rescues::try_rescue(&mut r, player, false));
            assert_eq!(r.player.hp, 1.);
            assert_eq!(engine::condition(&r.player, "rageCost"), cost);
            assert_eq!(r.player.energy, energy);
            assert_eq!(r.player.ability_reserves.as_ref().unwrap()[RAGE].energy, reserve);
            assert!(r.player.active.is_empty());
        }
        r.player.hp = 0.;
        assert!(!divine_rescues::try_rescue(&mut r, player, false));
        assert_eq!(engine::condition(&r.player, "rageCost"), 4.);
        r.exp = crate::progression::level_up_cost(&r);
        crate::progression::level_up(&mut r);
        assert_eq!(engine::condition(&r.player, "rageCost"), 0.);
        assert!(!divine_rescues::try_rescue(&mut r, player, true));
        assert_eq!(r.player.hp, 0.);
        assert!(divine_rescues::try_rescue(&mut r, player, false));
        // Ownership, not the stale active list, is authoritative.
        r.player.abilities.clear();
        r.player.active.push(RAGE.into());
        r.player.hp = 0.;
        assert!(!divine_rescues::try_rescue(&mut r, player, false));
    }

    #[test]
    fn ragelight_command_boundary_survives_rewinds() {
        let mut r = room(Field::Null);
        let snapshot = crate::temporal::Snapshot::capture(&r);
        assert!(snapshot.value().unwrap().get("ragelightRulesFrom").is_none());
        for marker in [serde_json::Value::Null, serde_json::json!(1)] {
            let mut encoded = snapshot.value().unwrap();
            encoded["ragelightRulesFrom"] = marker;
            let imported = crate::temporal::Snapshot::from_value(encoded).unwrap();
            assert!(imported.value().unwrap().get("ragelightRulesFrom").is_none());
        }

        r.player.abilities.push(crate::future_floor::ID.into());
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        crate::future_floor::ensure_entry(&mut r).unwrap();
        r.replay.push(Command::Wait);
        r.ragelight_rules_from = Field::Value(1);
        assert!(crate::future_floor::restore(&mut r).unwrap());
        assert_eq!(r.ragelight_rules_from, Field::Value(1));
        snapshot.restore(&mut r).unwrap();
        assert_eq!(r.ragelight_rules_from, Field::Value(1));
    }

    #[test]
    fn ragelight_current_toggle_is_inert_and_legacy_toggle_still_replays() {
        let command = Command::Ability { id: RAGE.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing };
        let legacy = room(Field::Null);
        let mut lethal_legacy = legacy.clone();
        lethal_legacy.player.hp = 0.;
        let player = lethal_legacy.player.id;
        assert!(!divine_rescues::try_rescue(&mut lethal_legacy, player, false));
        let mut old_engine = Engine::new(legacy.clone()).unwrap();
        old_engine.dispatch(&command).unwrap();
        assert!(old_engine.run.player.active.contains(&RAGE.into()));
        let record = crate::replay::ReplayRecord::try_from_run(&old_engine.run).unwrap();
        let mut replay = crate::replay::ReplaySession::from_initial(legacy.clone(), record).unwrap();
        replay.step().unwrap();
        assert_eq!(replay.engine.save().unwrap(), old_engine.save().unwrap());

        let mut current_engine = Engine::load(&old_engine.save().unwrap()).unwrap();
        assert_eq!(current_engine.run.ragelight_rules_from, Field::Value(1));
        let before = current_engine.save().unwrap();
        assert!(!current_engine.dispatch(&command).unwrap().changed);
        assert_eq!(current_engine.save().unwrap(), before);
        let record = crate::replay::ReplayRecord::try_from_run(&current_engine.run).unwrap();
        let mut replay = crate::replay::ReplaySession::from_initial(legacy, record).unwrap();
        replay.step().unwrap();
        assert_eq!(replay.engine.save().unwrap(), before);
    }
}
