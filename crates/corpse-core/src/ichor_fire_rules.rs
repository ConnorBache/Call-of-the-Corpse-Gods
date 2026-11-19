//! Existing Ichor flames spread to neighbors only every second world tick.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && !matches!(run.ichor_fire_rules_from, Field::Null)
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.ichor_fire_rules_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.ichor_fire_rules_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid Ichor fire command boundary".into());
    }
    Ok(())
}

/// An old live save adopts slower Ichor fire spread after its existing command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if current(run) && run.ichor_fire_rules_from.is_missing() {
        run.ichor_fire_rules_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.ichor_fire_rules_from = cursor.into();
    } else if cursor == 0 {
        run.ichor_fire_rules_from = Field::Null;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Floor, environment, offscreen, replay::{ReplayRecord, ReplaySession}, temporal};

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.tick = 0.;
        r.simulation_revision = 52.0.into();
        r.ichor_fire_rules_from = 0usize.into();
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

    fn at(f: &Floor, x: usize, y: usize) -> usize { y*f.width()+x }

    fn coat(r: &mut Run, xs: &[usize], y: usize) {
        let f=r.floor_mut().unwrap();
        let width=f.width();
        let count=f.tiles.len();
        let ichor=f.ichor.get_or_insert_with(||vec![0.;count]);
        for &x in xs { ichor[y*width+x] = 100.; }
    }

    fn fire(f: &Floor, x: usize, y: usize) -> Option<&str> {
        f.environment.as_ref()?.fire_kind.as_ref()?[at(f,x,y)].as_deref()
    }

    fn first_fire(r: &mut Run) {
        coat(r, &[20,21,22,23], 10);
        environment::ignite(r,20.,10.,None,None).unwrap();
        assert_eq!(fire(r.floor().unwrap(),20,10),Some("ichor"));
    }

    #[test]
    fn ichor_fire_propagates_every_other_world_step_without_slowing_ordinary_fire() {
        let mut r=room();
        r.tick=1.;
        first_fire(&mut r);
        coat(&mut r,&[31,40,41],10);
        let f=r.floor_mut().unwrap();
        let at=at(f,30,10);
        f.tiles[at]="grass".into();
        environment::ignite(&mut r,30.,10.,None,None).unwrap();
        assert_eq!(fire(r.floor().unwrap(),30,10),Some("plain"));

        environment::tick_environment(&mut r).unwrap();
        let f=r.floor().unwrap();
        assert_eq!(fire(f,20,10),Some("ichor"));
        assert_eq!(fire(f,21,10),None);
        assert_eq!(fire(f,31,10),Some("ichor"),"ordinary fire still spreads on odd ticks");
        assert_eq!(fire(f,40,10),None,"unlit Ichor cannot spontaneously spread fire");

        for (tick, last) in [(2.,21),(3.,21),(4.,22)] {
            r.tick=tick;
            environment::tick_environment(&mut r).unwrap();
            for x in 20..=23 {
                assert_eq!(fire(r.floor().unwrap(),x,10), (x<=last).then_some("ichor"),"tick {tick} x {x}");
            }
        }
    }

    #[test]
    fn ichor_fire_keeps_same_cell_reactions_between_spread_steps() {
        let mut r=room();
        r.tick=1.;
        first_fire(&mut r);
        let f=r.floor_mut().unwrap();
        let at=at(f,20,10);
        environment::add_gas(f,at,"flammable",30.,-1.).unwrap();
        let n=f.tiles.len();
        let mut adapter=crate::fire_adapter::FireAdapter::new(&mut r,vec![false;n]).unwrap();
        adapter.advance().unwrap();
        let f=r.floor().unwrap();
        // Typed gas can linger at zero volume; consumption is authoritative.
        assert_eq!(f.environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[at],0.);
        assert_eq!(fire(f,20,10),Some("ichor"));
        assert_eq!(fire(f,21,10),None);
    }

    #[test]
    fn ichor_fire_does_not_throttle_the_mercuria_liquid_flood() {
        let mut r=room();
        let f=r.floor_mut().unwrap();
        f.godbeast_event=crate::GodbeastEvent {
            kind: "Mercuria".into(), activated: true.into(), x:20., y:10., ..Default::default()
        }.into();
        for tick in 1..=2 {
            r.tick=tick as f64;
            environment::tick_environment(&mut r).unwrap();
            let f=r.floor().unwrap();
            assert_eq!(f.ichor.as_ref().unwrap()[at(f,20+tick,10)],100.);
            assert_eq!(fire(f,20+tick,10),None,"flooding is not direct ignition");
        }
    }

    #[test]
    fn ichor_fire_lazy_floor_matches_active_world_tick_parity() {
        let mut r=room();
        r.depth=2.;
        {
            let f=r.floor_mut().unwrap();
            f.tiles.fill("floor".into()); f.lights.fill(100.);
            f.environment=Field::Missing; f.vents=Field::Missing;
            f.ichor=Field::Missing; f.godbeast_event=Field::Missing;
            f.hellfire=Field::Missing; f.permanent_fire=Field::Missing;
            f.last_tick=0.;
        }
        first_fire(&mut r);
        let mut active=r.clone();
        r.depth=1.;
        let mut lazy=r.clone();
        for tick in 1..=4 {
            r.tick=tick as f64;
            active.tick=r.tick;
            environment::tick_environment(&mut active).unwrap();
            // Real, lazy inactive-floor execution. It rebuilds a proxy run at
            // the missing floor tick rather than using the current floor phase.
            offscreen::catch_up_floor(&mut r,2.).unwrap();
            let f=r.floors.iter().find(|f| f.depth==2.).unwrap();
            assert_eq!(f.last_tick,r.tick);
            for x in 20..=23 {
                assert_eq!(fire(f,x,10),fire(active.floor().unwrap(),x,10),"tick {tick} x {x}");
            }
            assert_eq!(fire(f,21,10), (tick>=2).then_some("ichor"));
            assert_eq!(fire(f,22,10), (tick>=4).then_some("ichor"));
        }
        lazy.tick=3.;
        offscreen::catch_up_floor(&mut lazy,2.).unwrap();
        let f=lazy.floors.iter().find(|f| f.depth==2.).unwrap();
        assert_eq!(f.last_tick,3.);
        assert_eq!(fire(f,21,10),Some("ichor"));
        assert_eq!(fire(f,22,10),None,"one lazy catch-up batches three missing world steps");
    }

    #[test]
    fn ichor_fire_historical_prefix_keeps_its_original_spread_and_replays_adoption() {
        let mut initial=room();
        first_fire(&mut initial);
        initial.ichor_fire_rules_from=Field::Null;
        let mut old=temporal::Engine::new(initial.clone()).unwrap();
        old.dispatch(&Command::Wait).unwrap();
        assert_eq!(fire(old.run.floor().unwrap(),21,10),Some("ichor"),"old odd step still spreads");
        let mut live=temporal::Engine::load(&old.save().unwrap()).unwrap();
        assert_eq!(live.run.ichor_fire_rules_from,Field::Value(1));
        for _ in 0..2 { live.dispatch(&Command::Wait).unwrap(); }
        assert_eq!(fire(live.run.floor().unwrap(),22,10),Some("ichor"));
        assert_eq!(fire(live.run.floor().unwrap(),23,10),None,"new odd step is held");
        let mut replay=ReplaySession::from_initial(initial,ReplayRecord::from_run(&live.run)).unwrap();
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),live.save().unwrap());
        replay.restart().unwrap();
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),live.save().unwrap());
    }
}
