//! A real adjacent strike identifies its hostile source, even on a miss.
//! This is creature knowledge only; optics, telepathy, and map memory are unchanged.
use crate::{Actor, Field, Run, engine};

const THROUGH: &str = "r52AdjacentAttackerThrough";
const COMMAND: &str = "r52AdjacentAttackerCommand";

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && r.adjacent_attack_sight_from.as_ref().is_some()
}

pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.adjacent_attack_sight_from.is_missing()
        && (r.simulation_revision.value_or(29.) < 52.
            || r.adjacent_attack_sight_from.as_ref().is_some_and(|&n| n > r.replay.len()))
    {
        return Err("Invalid adjacent attacker sight command boundary".into());
    }
    Ok(())
}

pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.adjacent_attack_sight_from.is_missing() {
        r.adjacent_attack_sight_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, policy: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if policy.as_ref() == Some(&cursor) { r.adjacent_attack_sight_from = cursor.into(); }
    else if cursor == 0 { r.adjacent_attack_sight_from = Field::Null; }
}

pub(crate) fn condition_key(key: &str) -> bool { matches!(key, THROUGH | COMMAND) }

fn command_number(r: &Run) -> f64 { r.metrics.get("commands").copied().unwrap_or(0.) }

/// Warm two condition lookups, plus existing plane classification. Does not
/// make a sensed attacker optically visible or add a telepathic terrain halo.
pub(crate) fn visible(r: &Run, attacker: &Actor) -> bool {
    current(r) && attacker.id != r.player.id && attacker.hp > 0.
        && !attacker.dead.value_or(false)
        && attacker.conditions.get(THROUGH).is_some_and(|&until| until >= r.tick)
        && crate::dreamwalking::same_plane(r, &r.player, attacker)
}

/// Must run only after ordinary attack admission; a rejection is not a strike.
/// A pre-clock attack can finish in either the current or next tick; carry
/// only a strike made within the actual command that crosses that boundary.
pub(crate) fn strike(r: &mut Run, source: &Actor, target: &Actor) {
    if !current(r) || target.id != r.player.id || source.id == target.id
        || source.hp <= 0. || source.dead.value_or(false)
        || !crate::dreamwalking::same_plane(r, source, target)
        || !crate::revision50_control_amulets::hostile(r, source, target)
        || ((source.x-target.x).abs().max((source.y-target.y).abs()) > 1.
            && crate::portals::actor_chebyshev(r, source, target) > 1.)
    {
        return;
    }
    let tick = r.tick;
    let command = (!r.resolving_tick.value_or(false)).then(|| command_number(r));
    if let Some(a) = engine::actor_mut_trusted(r, source.id) {
        a.conditions.insert(THROUGH.into(), tick);
        if let Some(n) = command { a.conditions.insert(COMMAND.into(), n); }
    }
    if let Some(n) = command { r.player.conditions.insert(COMMAND.into(), n); }
}

/// O(A) only if attacks occurred before this command's normal tick starts;
/// other turns are O(1). No historical-floor scan or RNG. Expired receipts
/// remain inert and may be overwritten on a later adjacent strike.
pub(crate) fn carry_pre_turn_attacks(r: &mut Run) {
    if !current(r) { return; }
    let command = command_number(r);
    if r.player.conditions.shift_remove(COMMAND) != Some(command) { return; }
    let tick = r.tick;
    let ids: Vec<_> = r.actors_at(r.depth).iter()
        .filter(|a| a.conditions.get(COMMAND) == Some(&command)
            && a.conditions.get(THROUGH) == Some(&tick))
        .map(|a| a.id).collect();
    for id in ids {
        if let Some(a) = engine::actor_mut_trusted(r, id) {
            a.conditions.shift_remove(COMMAND);
            a.conditions.insert(THROUGH.into(), tick+1.);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorLastSeen, Command, replay::{ReplayRecord,ReplaySession}, temporal::Engine, vision};

    fn room() -> (Run, f64, f64) {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=52.0.into();
        run.adjacent_attack_sight_from=0usize.into();
        run.player.x=10.;run.player.y=10.;run.player.hp=1000.;run.player.max_hp=1000.;
        run.player.base_max_hp=1000.0.into();run.player.stats.perception=300.;
        run.player.conditions.clear();run.player.active.clear();run.player.abilities.clear();
        run.offers.clear();run.inventory.clear();
        run.phase="normal".to_owned().into();
        run.actors_at_mut(run.depth).clear();
        let floor=run.floor_mut().unwrap();
        floor.tiles.fill("floor".into());floor.lights.fill(0.);floor.doors=Field::Missing;
        floor.portals=Field::Missing;floor.non_euclidean=Field::Missing;floor.speed_up=Field::Missing;
        let mut spawn=|x,y,acting:bool| {
            let id=run.next_id;run.next_id+=1.;
            let mut enemy=crate::population::create_actor_revision("rat",id,x,y,false,52.).unwrap();
            enemy.state="hunting".into();enemy.last_seen=ActorLastSeen{x:10.,y:10.}.into();
            enemy.stats.perception=300.;
            enemy.conditions.insert("targetId".into(),run.player.id);
            enemy.conditions.insert("invisible".into(),100.);
            enemy.conditions.insert("distracted".into(),1.);
            if !acting { enemy.conditions.insert("paralyzed".into(),100.); }
            run.actors_at_mut(run.depth).push(enemy);
            id
        };
        let attacker=spawn(11.,11.,true);
        let bystander=spawn(9.,10.,false);
        (run,attacker,bystander)
    }

    fn assert_perceived(run: &Run, id: f64, expected: bool) {
        let enemy=engine::actor(run,id).unwrap();
        assert_eq!(vision::visible_actor(run,enemy),expected);
        assert_eq!(vision::VisibilityProjection::new(run).visible_actor(enemy),expected);
        assert!(!vision::directly_visible_actor(run,enemy));
        assert!(!vision::telepathic(run,enemy));
    }

    #[test]
    fn adjacent_miss_reveals_only_its_source_until_next_normal_turn() {
        let (run,attacker,bystander)=room();let mut engine=Engine::new(run).unwrap();
        assert_perceived(&engine.run,attacker,false);
        assert_perceived(&engine.run,bystander,false);
        let mask=vision::terrain_visibility(&engine.run);
        assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);
        assert!(!engine::actor(&engine.run,attacker).unwrap().conditions.contains_key("distracted"),"attempt resolved");
        assert!(engine.run.log.iter().any(|line|line.contains("but misses")));
        assert_perceived(&engine.run,attacker,true);
        assert_perceived(&engine.run,bystander,false);
        assert_eq!(vision::terrain_visibility(&engine.run),mask);
        assert!(!engine.dispatch(&Command::Rally{x:8.,y:10.}).unwrap().took_turn);
        assert_perceived(&engine.run,attacker,true);
        let loaded=Engine::load(&engine.save().unwrap()).unwrap();
        assert_perceived(&loaded.run,attacker,true);
        engine::actor_mut(&mut engine.run,attacker).unwrap().conditions.insert("paralyzed".into(),100.);
        assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);
        assert_perceived(&engine.run,attacker,false);
    }

    #[test]
    fn pre_turn_strike_survives_haste_but_expires_when_world_advances() {
        let (mut run,attacker,_)=room();
        run.phase="haste".to_owned().into();
        run.player.conditions.insert("hasted".into(),100.);
        engine::actor_mut(&mut run,attacker).unwrap().conditions.insert("hasted".into(),100.);
        let mut engine=Engine::new(run).unwrap();
        assert!(!engine.dispatch(&Command::Wait).unwrap().took_turn);
        assert_perceived(&engine.run,attacker,true);
        engine::actor_mut(&mut engine.run,attacker).unwrap().conditions.insert("paralyzed".into(),100.);
        assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);
        assert_perceived(&engine.run,attacker,false);
    }

    #[test]
    fn no_reveal_for_blocked_different_plane_or_distant_attack() {
        for (x,y,plane,protected) in [(13.,10.,false,false),(11.,11.,true,false),(11.,11.,false,true)] {
            let (mut run,id,_)=room();
            let a=engine::actor_mut(&mut run,id).unwrap();a.x=x;a.y=y;
            if plane { a.conditions.insert("phantasmiiPlane".into(),1.); }
            if protected { run.player.active.push("morae-turning-the-wheel".into()); }
            let player=run.player.id;
            engine::attack(&mut run,id,player,1.).unwrap();
            assert_perceived(&run,id,false);
        }
    }

    #[test]
    fn a_portal_neighbor_is_adjacent_for_a_real_strike() {
        let (mut run,id,_)=room();
        let a=engine::actor_mut(&mut run,id).unwrap();
        a.x=18.;a.y=10.;
        let f=run.floor_mut().unwrap();let w=f.width();
        f.portals=vec![crate::portals_generation::PortalPair {a:10*w+11,b:10*w+18,color_index:0}].into();
        assert_eq!(crate::portals::actor_chebyshev(&run,&run.player,engine::actor(&run,id).unwrap()),1.);
        let player=run.player.id;
        engine::attack(&mut run,id,player,1.).unwrap();
        assert_perceived(&run,id,true);
    }

    #[test]
    fn historical_replay_prefix_stays_hidden_and_live_load_enables_new_rule() {
        let (mut run,attacker,_)=room();run.adjacent_attack_sight_from=Field::Null;
        let mut old=Engine::new(run.clone()).unwrap();
        old.dispatch(&Command::Wait).unwrap();
        assert_perceived(&old.run,attacker,false);
        let record=ReplayRecord::try_from_run(&old.run).unwrap();
        assert!(record.adjacent_attack_sight_from.is_missing());
        let mut replay=ReplaySession::from_initial(run.clone(),record).unwrap();
        replay.step().unwrap();assert_perceived(&replay.engine.run,attacker,false);
        let mut live=Engine::load(&old.save().unwrap()).unwrap();
        assert_eq!(live.run.adjacent_attack_sight_from,1usize.into());
        live.dispatch(&Command::Wait).unwrap();
        assert_perceived(&live.run,attacker,true);
        let mut replay=ReplaySession::from_initial(run,ReplayRecord::try_from_run(&live.run).unwrap()).unwrap();
        replay.step().unwrap();assert_perceived(&replay.engine.run,attacker,false);
        replay.step().unwrap();assert_perceived(&replay.engine.run,attacker,true);
        replay.restart().unwrap();
        replay.step().unwrap();assert_perceived(&replay.engine.run,attacker,false);
    }
}
