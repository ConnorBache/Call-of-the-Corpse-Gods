use corpse_core::{Command,Field,Run,Stats,engine,half_here};
fn room(revision:f64) -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();
    r.player.x=10.;r.player.y=10.;r.player.hp=100.;r.player.max_hp=100.;
    r.player.stats=Stats {accuracy:100.,perception:300.,max_energy:100.,..Default::default()};
    r.player.energy=40.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.amulet_ledger=Field::Missing;r.player.reaping_ledger=Field::Missing;
    r.inventory.clear();r.actors=Default::default();r.ground.clear();
    for depth in 1..=r.floors.len() {r.actors_at_mut(depth as f64);r.ground_at_mut(depth as f64);}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.phase="normal".to_string().into();r.next_id=1000.;r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;f.vents=Field::Missing;
    f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    r
}
fn command(x:Option<f64>,y:Option<f64>)->Command{Command::Ability{id:half_here::ID.into(),target:Field::Missing,x:x.map_or(Field::Missing,Field::Value),y:y.map_or(Field::Missing,Field::Value)}}
#[test] fn successful_placement_takes_one_turn_but_dismissal_remains_free(){
 let mut r=room(52.);r.player.abilities.push(half_here::ID.into());let tick=r.tick;
 engine::dispatch(&mut r,&command(Some(11.),Some(10.))).unwrap();
 assert_eq!(r.tick,tick+1.);assert!(r.player.active.iter().any(|s|s==half_here::ID));
 engine::dispatch(&mut r,&command(None,None)).unwrap();assert_eq!(r.tick,tick+1.);assert!(!r.player.active.iter().any(|s|s==half_here::ID));
}
#[test] fn missing_and_occupied_targets_do_not_advance_time(){
 for (x,y) in [(None,None),(Some(10.),Some(10.)),(Some(-1.),Some(10.))]{
 let mut r=room(52.);r.player.abilities.push(half_here::ID.into());let tick=r.tick;
 engine::dispatch(&mut r,&command(x,y)).unwrap();assert_eq!(r.tick,tick);assert!(!r.player.active.iter().any(|s|s==half_here::ID));
 }
}
#[test] fn legacy_placement_retains_free_action(){
 let mut r=room(51.);r.player.abilities.push(half_here::ID.into());let tick=r.tick;
 engine::dispatch(&mut r,&command(Some(11.),Some(10.))).unwrap();assert_eq!(r.tick,tick);assert!(r.player.active.iter().any(|s|s==half_here::ID));
}
#[test] fn placement_uses_one_haste_action_then_wait_finishes_normal_turn(){
 let mut r=room(52.);r.player.abilities.push(half_here::ID.into());r.player.conditions.insert("hasted".into(),10.);r.phase="haste".to_string().into();let tick=r.tick;
 engine::dispatch(&mut r,&command(Some(11.),Some(10.))).unwrap();assert_eq!(r.tick,tick);assert_eq!(r.phase.as_ref().map(String::as_str),Some("normal"));
 engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.tick,tick+1.);
}

#[test]
fn newer_placements_cannot_be_dismissed_or_replaced_before_five_ticks() {
    let mut r = room(52.);
    r.player.abilities.push(half_here::ID.into());
    half_here::admit_current(&mut r).unwrap();
    let start = r.tick;
    engine::dispatch(&mut r, &command(Some(11.), Some(10.))).unwrap();
    let clone = half_here::clone_id(&r).unwrap();
    for elapsed in 1..5 {
        assert_eq!(r.tick, start + elapsed as f64);
        assert_eq!(half_here::turns_until_dismissal(&r), 5 - elapsed);
        engine::dispatch(&mut r, &command(None, None)).unwrap();
        engine::dispatch(&mut r, &command(Some(10.), Some(11.))).unwrap();
        assert_eq!(r.tick, start + elapsed as f64);
        assert_eq!(half_here::clone_id(&r), Some(clone));
        assert_eq!(engine::actor(&r, clone).map(|a| (a.x, a.y)), Some((11., 10.)));
        engine::dispatch(&mut r, &Command::Wait).unwrap();
    }
    engine::dispatch(&mut r, &command(None, None)).unwrap();
    assert_eq!(r.tick, start + 5.);
    assert!(half_here::clone_id(&r).is_none());
    engine::dispatch(&mut r, &command(Some(10.), Some(11.))).unwrap();
    assert_eq!(half_here::turns_until_dismissal(&r), 4);
}

#[test]
fn fresh_clone_cannot_be_swapped_or_forced_out_of_its_tile() {
    let mut r = room(52.);
    r.player.abilities.push(half_here::ID.into());
    half_here::admit_current(&mut r).unwrap();
    engine::dispatch(&mut r, &command(Some(11.), Some(10.))).unwrap();
    let id = half_here::clone_id(&r).unwrap();
    let step = Command::Move { dx: 1., dy: 0. };
    engine::dispatch(&mut r, &step).unwrap();
    assert_eq!((r.player.x, r.player.y), (10., 10.));
    assert_eq!(engine::actor(&r, id).map(|a| (a.x, a.y)), Some((11., 10.)));
    r.actors_at_mut(r.depth).iter_mut().find(|a| a.id == id).unwrap()
        .conditions.insert("forcedMove".into(), 1.);
    assert!(!engine::move_actor(&mut r, id, 1., 0., true).unwrap());
    assert_eq!(engine::actor(&r, id).unwrap().x, 11.);
    while half_here::turns_until_dismissal(&r) > 0 {
        engine::dispatch(&mut r, &Command::Wait).unwrap();
    }
    engine::dispatch(&mut r, &step).unwrap();
    assert_eq!(r.player.x, 11.);
    assert_eq!(engine::actor(&r, id).unwrap().x, 10.);
}

#[test]
fn saved_and_replayed_haste_placement_preserves_the_lock() {
    use corpse_core::{replay::{ReplayRecord, ReplaySession}, temporal::Engine};
    let mut initial = room(52.);
    initial.player.abilities.push(half_here::ID.into());
    initial.player.conditions.insert("hasted".into(), 10.);
    initial.phase = "haste".to_string().into();
    let mut live = Engine::new(initial.clone()).unwrap();
    live.dispatch(&command(Some(11.), Some(10.))).unwrap();
    assert_eq!(live.run.tick, initial.tick);
    assert_eq!(half_here::turns_until_dismissal(&live.run), 5);
    live.dispatch(&command(None, None)).unwrap();
    assert_eq!(half_here::turns_until_dismissal(&live.run), 5);
    live.dispatch(&Command::Wait).unwrap();
    let loaded = Engine::load(&live.save().unwrap()).unwrap();
    assert_eq!(half_here::turns_until_dismissal(&loaded.run), 4);
    let record = ReplayRecord::try_from_run(&live.run).unwrap();
    let mut replay = ReplaySession::from_initial(initial, record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(half_here::turns_until_dismissal(&replay.engine.run), 4);
    assert_eq!(half_here::clone_id(&replay.engine.run), half_here::clone_id(&live.run));
}

#[test]
fn legacy_r52_replay_dismisses_immediately_until_a_live_upgrade_boundary() {
    use corpse_core::{replay::{ReplayRecord, ReplaySession}, temporal::Engine};
    let mut initial = room(52.);
    initial.player.abilities.push(half_here::ID.into());
    let mut live = Engine::new(initial.clone()).unwrap();
    live.run.half_here_lock_from = Field::Missing;
    let place = command(Some(11.), Some(10.));
    let dismiss = command(None, None);
    live.dispatch(&place).unwrap();
    live.dispatch(&dismiss).unwrap();
    assert!(half_here::clone_id(&live.run).is_none());
    let mut live = Engine::load(&live.save().unwrap()).unwrap();
    assert_eq!(live.run.half_here_lock_from, 2usize.into());
    live.dispatch(&place).unwrap();
    live.dispatch(&dismiss).unwrap();
    let record = ReplayRecord::try_from_run(&live.run).unwrap();
    let mut replay = ReplaySession::from_initial(initial, record).unwrap();
    assert!(replay.step().unwrap());
    assert_eq!(half_here::turns_until_dismissal(&replay.engine.run), 0);
    assert!(replay.step().unwrap());
    assert!(half_here::clone_id(&replay.engine.run).is_none());
    while replay.step().unwrap() {}
    assert_eq!(half_here::turns_until_dismissal(&replay.engine.run), 4);
    assert_eq!(half_here::clone_id(&replay.engine.run), half_here::clone_id(&live.run));
}
