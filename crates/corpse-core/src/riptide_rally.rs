//! Riptide follows the live rally flag after a versioned command boundary.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && run.riptide_rally_from.as_ref().is_some()
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.riptide_rally_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.riptide_rally_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid Riptide rally command boundary".into());
    }
    Ok(())
}

/// An old live save adopts moving rally points after its existing command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if run.simulation_revision.value_or(29.) >= 52. && run.riptide_rally_from.is_missing() {
        run.riptide_rally_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.riptide_rally_from = cursor.into();
    } else if cursor == 0 {
        run.riptide_rally_from = Field::Null;
    }
}

pub fn placed(run: &Run) -> Option<(f64, f64)> {
    let rally = run.rally.as_ref()?;
    crate::portals::cell(run.floor()?, rally.x, rally.y)?;
    Some((rally.x, rally.y))
}

/// Both simulation and previews read the same destination. A legacy active
/// cast without a rally keeps its old point until the player sets a flag.
pub fn goal(run: &Run, caster: &crate::Actor) -> Option<(f64, f64)> {
    if current(run) {
        placed(run).or_else(|| {
            Some((*caster.conditions.get("riptideX")?, *caster.conditions.get("riptideY")?))
        })
    } else {
        Some((crate::engine::condition(caster, "riptideX"), crate::engine::condition(caster, "riptideY")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, engine, replay::{ReplayRecord, ReplaySession}, temporal::Engine};
    const ID: &str = "etill-riptide";
    const ENEMY: f64 = 901.;

    fn scene() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.next_id = 1000.;
        r.player.x = 10.;
        r.player.y = 10.;
        r.player.energy = 100.;
        r.player.stats.max_energy = 100.;
        r.player.abilities = vec![ID.into()];
        r.player.active.clear();
        r.player.conditions.clear();
        r.player.conditions.insert("telepathic".into(),100.);
        r.actors_at_mut(r.depth).clear();
        r.inventory.clear();
        r.offers.clear();
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into());
        f.lights.fill(0.);
        f.doors = Field::Missing;
        f.portals = Field::Missing;
        let mut a = crate::population::create_actor_revision("bear", ENEMY, 14., 10., false, 52.).unwrap();
        a.state = "hunting".into();
        a.active.clear();
        a.abilities.clear();
        r.actors_at_mut(r.depth).push(a);
        r
    }
    fn cast(goal: Option<(f64, f64)>) -> Command {
        Command::Ability { id: ID.into(), target: ENEMY.into(),
            x: goal.map_or(Field::Missing, |(x,_)| x.into()),
            y: goal.map_or(Field::Missing, |(_,y)| y.into()) }
    }
    fn enemy(run: &Run) -> (f64, f64) {
        let a = engine::actor(run, ENEMY).unwrap();
        (a.x, a.y)
    }

    #[test]
    fn current_cast_uses_existing_flag_and_retargets_next_turn_without_recast() {
        let mut e = Engine::new(scene()).unwrap();
        assert!(current(&e.run));
        let energy = e.run.player.energy;
        e.dispatch(&cast(None)).unwrap();
        assert_eq!(energy, e.run.player.energy);
        assert!(!e.run.player.active.iter().any(|id| id == ID));

        e.dispatch(&Command::Rally { x: 18., y: 10. }).unwrap();
        // The coordinates in a targeted command describe the clicked enemy,
        // not the goal. They cannot override the flag.
        e.dispatch(&cast(Some((14., 10.)))).unwrap();
        e.dispatch(&Command::Wait).unwrap();
        let (x,y) = enemy(&e.run);
        assert!(x > 14.);
        assert_eq!(y, 10.);
        let tick = e.run.tick;
        let energy = e.run.player.energy;
        e.dispatch(&Command::Rally { x: x-4., y: y }).unwrap();
        assert_eq!(e.run.tick, tick);
        assert_eq!(e.run.player.energy, energy);
        assert_eq!(enemy(&e.run), (x,y));

        let save = e.save().unwrap();
        let mut reloaded = Engine::load(&save).unwrap();
        e.dispatch(&Command::Wait).unwrap();
        reloaded.dispatch(&Command::Wait).unwrap();
        assert_eq!(enemy(&e.run), (x-1.,y));
        assert_eq!(enemy(&reloaded.run), enemy(&e.run));
        assert!(e.run.player.active.iter().any(|id| id == ID));
    }

    #[test]
    fn replay_does_not_redirect_old_cast_until_command_boundary() {
        let initial = scene();
        let mut r = initial.clone();
        r.riptide_rally_from = Field::Null;
        let mut live = Engine::new_for_replay(r).unwrap();
        live.dispatch(&Command::Rally { x: 2., y: 10. }).unwrap();
        live.dispatch(&cast(Some((18., 10.)))).unwrap();
        live.dispatch(&Command::Wait).unwrap();
        let old = enemy(&live.run);
        assert!(old.0 > 14.);
        let boundary = live.run.replay.len();
        live.run.riptide_rally_from = Field::Missing;
        admit_current(&mut live.run).unwrap();
        live.dispatch(&Command::Rally { x: old.0, y: 6. }).unwrap();
        live.dispatch(&Command::Wait).unwrap();
        assert_eq!(enemy(&live.run), (old.0, 9.));
        let record = ReplayRecord::from_run(&live.run);
        assert_eq!(record.riptide_rally_from, Field::Value(boundary));
        let mut replay = ReplaySession::from_initial(initial, record).unwrap();
        for _ in 0..boundary-1 { assert!(replay.step().unwrap()); }
        assert!(!current(&replay.engine.run));
        assert!(replay.step().unwrap());
        assert_eq!(enemy(&replay.engine.run), old);
        assert!(current(&replay.engine.run));
        while replay.step().unwrap() {}
        assert!(current(&replay.engine.run));
        assert_eq!(enemy(&replay.engine.run), enemy(&live.run));
    }
}
