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
