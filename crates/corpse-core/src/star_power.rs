//! Opt-in Star Power for current runs; legacy command prefixes keep suppression.
use crate::{Actor, Field, Run};

pub const ID: &str = "anavani-star-power";
pub const DESCRIPTION: &str =
    "You are immune to fire. Activate to start burning; deactivate to stop.";
pub(crate) const LIT: &str = "starPowerLit";

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && r.star_power_from.as_ref().is_some()
}

pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.star_power_from.is_missing()
        && (r.simulation_revision.value_or(29.) < 52.
            || r.star_power_from.as_ref().is_some_and(|&n| n > r.replay.len()))
    {
        return Err("Invalid Star Power command boundary".into());
    }
    Ok(())
}

fn clear_burning(a: &mut Actor) {
    for key in ["burning", "burningOwner", "burningPlayer"] {
        a.conditions.shift_remove(key);
    }
}

/// O(A+C) in the restored player's ability/condition counts. A legacy
/// suppression bit must never become a positive ignition after a rewind.
pub(crate) fn reconcile(r: &mut Run) {
    if !current(r) || !r.player.abilities.iter().any(|id| id == ID)
        || r.player.conditions.contains_key(LIT) { return; }
    r.player.active.retain(|id| id != ID);
    if !r.player.conditions.contains_key("burningOwner") && !r.player.dead.value_or(false)
        && !crate::corpses::has_permanent_burning(&r.player) {
        clear_burning(&mut r.player);
    }
}

pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.star_power_from.is_missing() {
        r.star_power_from = r.replay.len().into();
        reconcile(r);
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        r.star_power_from = cursor.into();
        reconcile(r);
    } else if cursor == 0 {
        r.star_power_from = Field::Null;
    }
}

pub fn burning(r: &Run, a: &Actor) -> bool {
    !a.dead.value_or(false) && !crate::dreamwalking::is_body(a)
        && a.abilities.iter().any(|id| id == ID)
        && (a.active.iter().any(|id| id == ID) == current(r))
}

/// The free toggle changes the status immediately; normal world ticks handle
/// spread. Reuse the burn helper, including external extinguishing effects.
pub(crate) fn toggled(r: &mut Run, on: bool) {
    if !current(r) { return; }
    r.player.conditions.insert(LIT.into(), f64::from(u8::from(on)));
    if on {
        crate::corpses::apply_burning_run(r, r.player.id, r.simulation_revision.value_or(29.),
            0., None, None, None);
    } else if !crate::corpses::has_permanent_burning(&r.player) {
        clear_burning(&mut r.player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::condition, replay::{ReplayRecord, ReplaySession}, temporal::Engine, Command};

    fn scene() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.actors_at_mut(r.depth).clear();
        r.player.abilities.clear();
        r.player.active.clear();
        r.player.conditions.clear();
        r.player.abilities.push(ID.into());
        r.player.energy = 100.;
        r.player.stats.max_energy = 100.;
        r.offers.clear();
        r.inventory.clear();
        r.player.x = 10.;
        r.player.y = 10.;
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into());
        f.doors = Field::Missing;
        f.portals = Field::Missing;
        r
    }
    fn toggle() -> Command {
        Command::Ability { id: ID.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing }
    }
    fn lit(r: &Run) -> bool { condition(&r.player, "burning") > 0. }

    #[test]
    fn star_power_is_opt_in_and_free_and_keeps_passive_fire_immunity() {
        let mut e = Engine::new(scene()).unwrap();
        assert!(!lit(&e.run));
        e.dispatch(&Command::Wait).unwrap();
        assert!(!lit(&e.run));
        let (tick, energy) = (e.run.tick, e.run.player.energy);
        e.dispatch(&toggle()).unwrap();
        assert!(lit(&e.run) && burning(&e.run, &e.run.player));
        assert_eq!((e.run.tick, e.run.player.energy), (tick, energy));
        e.dispatch(&Command::Wait).unwrap();
        assert!(lit(&e.run));
        let save = e.save().unwrap();
        let mut restored = Engine::load(&save).unwrap();
        assert!(lit(&restored.run) && burning(&restored.run, &restored.run.player));
        restored.dispatch(&toggle()).unwrap();
        assert!(!lit(&restored.run) && !burning(&restored.run, &restored.run.player));
        let hp = restored.run.player.hp;
        let id = restored.run.player.id;
        crate::engine::damage_environment(&mut restored.run, id, 12., None, "fire", true).unwrap();
        assert_eq!(restored.run.player.hp, hp);
        restored.dispatch(&Command::Wait).unwrap();
        assert!(!lit(&restored.run));
    }

    #[test]
    fn star_power_replay_and_legacy_save_switch_rules_only_at_boundary() {
        let initial = scene();
        let mut legacy = initial.clone();
        legacy.star_power_from = Field::Null;
        let mut live = Engine::new_for_replay(legacy).unwrap();
        live.dispatch(&Command::Wait).unwrap();
        assert!(lit(&live.run));
        let historic = live.run.clone();
        live.dispatch(&toggle()).unwrap();
        let boundary = live.run.replay.len();
        live.run.star_power_from = Field::Missing;
        admit_current(&mut live.run).unwrap();
        assert!(!lit(&live.run) && !burning(&live.run, &live.run.player));
        live.dispatch(&toggle()).unwrap();
        assert!(lit(&live.run));
        live.dispatch(&Command::Wait).unwrap();
        let record = ReplayRecord::from_run(&live.run);
        assert_eq!(record.star_power_from, Field::Value(boundary));
        let mut replay = ReplaySession::from_initial(initial, record).unwrap();
        assert!(replay.step().unwrap());
        assert!(!current(&replay.engine.run));
        assert_eq!(replay.engine.run.rng, historic.rng);
        assert!(lit(&replay.engine.run));
        assert!(replay.step().unwrap());
        assert!(current(&replay.engine.run));
        assert!(!lit(&replay.engine.run));
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.run.rng, live.run.rng);
        assert_eq!(replay.engine.run.player.active, live.run.player.active);
        assert_eq!(replay.engine.run.player.conditions, live.run.player.conditions);
    }
}
