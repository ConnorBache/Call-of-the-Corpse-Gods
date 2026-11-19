//! Contact with spent embers is harmless from a recorded command boundary.
use crate::{Field, Run};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 50. && !matches!(r.ember_rules_from, Field::Null)
}

pub fn validate(r: &Run) -> Result<(), String> {
    if !r.ember_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 50.
        || r.ember_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid ember rules command boundary".into());
    }
    Ok(())
}

/// Old live saves adopt the rule after their existing command ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 50. && r.ember_rules_from.is_missing() {
        r.ember_rules_from = r.replay.len().into();
    }
    Ok(())
}

pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 50. { return; }
    if from.as_ref() == Some(&cursor) { r.ember_rules_from = cursor.into(); }
    else if cursor == 0 { r.ember_rules_from = Field::Null; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, engine, environment, temporal};

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.ember_rules_from = 0usize.into();
        r.player.x = 10.; r.player.y = 10.;
        r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
        r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
        r.actors.clear(); r.ground.clear(); r.inventory.clear();
        for depth in r.floors.iter().map(|f| f.depth).collect::<Vec<_>>() {
            r.actors_at_mut(depth); r.ground_at_mut(depth);
        }
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into()); f.lights.fill(100.);
        f.doors = Field::Missing; f.portals = Field::Missing; f.non_euclidean = Field::Missing;
        f.environment = Field::Missing; f.vents = Field::Missing;
        f.hellfire = Field::Missing; f.permanent_fire = Field::Missing;
        r
    }

    fn cell(r: &mut Run, x: usize, kind: Option<&str>, intensity: f64) -> usize {
        let f = r.floor_mut().unwrap();
        let i = 10 * f.width() + x;
        let e = environment::initialize(f).unwrap();
        e.fire_kind.as_mut().unwrap()[i] = kind.map(str::to_owned);
        e.fire[i] = intensity;
        i
    }

    #[test]
    fn spent_embers_are_safe_on_arrival_and_while_standing_even_if_heat_view_lags() {
        for intensity in [0., 1.] {
            let mut r = room();
            let id = r.player.id;
            let at = cell(&mut r, 11, Some("embers"), intensity);
            assert!(!environment::creature_hazard_at(&r, &r.player, at));
            let rng = r.rng;
            assert!(engine::move_actor(&mut r, id, 1., 0., false).unwrap());
            assert_eq!(r.player.x, 11.);
            assert_eq!(r.rng, rng);
            assert_eq!(engine::condition(&r.player, "burning"), 0.);
            environment::tick_environment(&mut r).unwrap();
            assert_eq!(r.player.hp, 100.);
            assert_eq!(engine::condition(&r.player, "burning"), 0.);
        }
    }

    #[test]
    fn actual_fire_burns_and_spent_embers_do_not_refresh_existing_burning() {
        let mut r = room();
        let id = r.player.id;
        cell(&mut r, 10, Some("creature"), 1.);
        crate::travel::enemy_moved(&mut r, id).unwrap();
        let duration = engine::condition(&r.player, "burning");
        assert!(duration > 0.);
        cell(&mut r, 10, Some("embers"), 1.);
        let rng = r.rng;
        environment::refresh_burning_from_fire(&mut r, id).unwrap();
        assert_eq!(r.rng, rng);
        assert_eq!(engine::condition(&r.player, "burning"), duration);
        environment::tick_environment(&mut r).unwrap();
        assert!(r.player.hp < 100.);
        assert_eq!(engine::condition(&r.player, "burning"), (duration - 1.).max(0.));
    }

    #[test]
    fn hellfire_still_burns_during_world_exposure() {
        let mut r = room();
        let f = r.floor_mut().unwrap();
        let i = 10 * f.width() + 10;
        let mut hellfire = vec![false; f.tiles.len()];
        hellfire[i] = true;
        f.hellfire = hellfire.into();
        assert!(environment::creature_hazard_at(&r, &r.player, i));
        environment::tick_environment(&mut r).unwrap();
        assert!(r.player.hp < 100.);
    }

    #[test]
    fn policy_preserves_historical_contact_and_live_save_boundary() {
        let mut r = room();
        let id = r.player.id;
        let at = cell(&mut r, 10, Some("embers"), 1.);
        let from = Field::Value(1);
        apply_replay_policy(&mut r, &from, 0);
        assert!(environment::creature_hazard_at(&r, &r.player, at));
        environment::refresh_burning_from_fire(&mut r, id).unwrap();
        assert!(engine::condition(&r.player, "burning") > 0.);
        apply_replay_policy(&mut r, &from, 1);
        assert!(!environment::creature_hazard_at(&r, &r.player, at));

        r.replay.push(Command::Wait);
        r.ember_rules_from = Field::Missing;
        let e = temporal::Engine::new(r).unwrap();
        assert_eq!(e.run.ember_rules_from, from);
        let restored = temporal::Engine::load(&e.save().unwrap()).unwrap();
        assert_eq!(restored.run.ember_rules_from, from);
        let record = crate::replay::ReplayRecord::from_run(&restored.run);
        assert_eq!(record.ember_rules_from, from);
        let snap = temporal::Snapshot::capture(&restored.run);
        assert!(snap.value().unwrap().get("emberRulesFrom").is_none());
        let mut rewind = restored.run.clone();
        rewind.ember_rules_from = Field::Null;
        snap.restore(&mut rewind).unwrap();
        assert_eq!(rewind.ember_rules_from, Field::Null);
    }
}
