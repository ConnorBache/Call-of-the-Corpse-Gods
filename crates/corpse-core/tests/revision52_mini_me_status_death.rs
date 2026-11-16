use corpse_core::{Command,Field,Run,Stats,engine};
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

fn scenario(effect: &str, hp: f64) -> Run {
 let mut r=room(52.); r.player.abilities.push("kative-mini-me".into());
 let mut a=corpse_core::mini_me::preview(&r,15.,10.); a.hp=hp; a.max_hp=172.;
 let id=a.id;
 match effect {
  "venom" => {a.conditions.insert("envenomed".into(),2.);a.conditions.insert(format!("venom:{}:999",r.tick+2.),2.);},
  "bleed" => {a.conditions.insert("killspawnBleeding".into(),1.);a.conditions.insert(format!("killspawnBleed:{}:999",r.tick+1.),2.);},
  _ => unreachable!()
 }
 r.player.conditions.insert("miniMeId".into(),id);r.player.active.push("kative-mini-me".into());r.actors_at_mut(r.depth).push(a); r
}
#[test] fn mini_me_lethal_venom_during_wait_and_move_does_not_panic() {
 for command in [Command::Wait,Command::Move{dx:1.,dy:0.}] {
 let mut r=scenario("venom",1.);let id=r.next_id;let tick=r.tick;
 engine::dispatch(&mut r,&command).unwrap();
 assert!(engine::actor(&r,id).is_none());assert_eq!(r.tick,tick+1.);assert_eq!(r.status,"playing");
 engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.tick,tick+2.);
 }
}
#[test] fn mini_me_lethal_scheduled_bleed_does_not_panic() {
 let mut r=scenario("bleed",1.);let id=r.next_id;
 engine::dispatch(&mut r,&Command::Wait).unwrap();assert!(engine::actor(&r,id).is_none());
}
#[test] fn mini_me_nonlethal_status_still_regenerates_and_acts() {
 let mut r=scenario("venom",100.);let id=r.next_id;let tick=r.tick;
 engine::dispatch(&mut r,&Command::Wait).unwrap();let a=engine::actor(&r,id).unwrap();
 assert!(a.hp>98. && a.hp<100.);assert!(!a.dead.value_or(false));assert_eq!(r.tick,tick+1.);
}
#[test] fn loaded_pre_death_state_can_advance_safely() {
 let r=scenario("venom",1.);let raw=serde_json::to_string(&r).unwrap();
 let mut loaded:Run=serde_json::from_str(&raw).unwrap();let id=loaded.next_id;
 engine::dispatch(&mut loaded,&Command::Move{dx:1.,dy:0.}).unwrap();
 assert!(engine::actor(&loaded,id).is_none());assert_eq!(loaded.status,"playing");
}
