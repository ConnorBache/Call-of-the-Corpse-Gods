use corpse_core::{Run,Field,Command,engine,inventory,population,vision};
use std::sync::Arc;
fn room(revision:f64)->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=revision.into();r.player.x=40.;r.player.y=10.;r.player.stats.perception=0.;
 r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
 r.inventory.clear();r.offers.clear();r.history.clear();r.replay.clear();r.future=None;r.phase=Field::Missing;r.haste_snapshot=Field::Missing;
 for a in r.actors.values_mut(){*a=Arc::new(vec![]);}for g in r.ground.values_mut(){*g=Arc::new(vec![]);}
 let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(0.);f.seen.fill(false);
 f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;
 f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;
 f.ichor=Field::Missing;f.light_sources=Field::Missing;f.permanent_fire=Field::Missing;
 f.snow=Field::Missing;f.ice=Field::Missing;f.hellfire=Field::Missing;r
}
fn rat(r:&mut Run,x:f64,state:&str)->f64 {
 let id=r.next_id;r.next_id+=1.;let mut a=population::create_actor_revision("rat",id,x,10.,false,51.).unwrap();
 a.state=state.into();a.stats.perception=0.;a.conditions.insert("stealth".into(),0.);a.stats.max_energy=100.;a.energy=0.;
 r.actors_at_mut(r.depth).push(a);id
}
#[test]
fn hunter_reaches_stored_location_before_forgetting_without_live_position_tracking() {
 let mut r=room(51.);let id=rat(&mut r,20.,"hunting");let pid=r.player.id;
 let a=engine::actor_mut(&mut r,id).unwrap();a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();a.conditions.insert("targetId".into(),pid);a.conditions.insert("monsterUnseenTurns".into(),20.);
 assert!(!vision::plausibly_visible_actor(&r,engine::actor(&r,id).unwrap(),&r.player));
 for _ in 0..20 {
  let a=engine::actor(&r,id).unwrap();if (a.x,a.y)==(10.,10.){break;}
  engine::dispatch(&mut r,&Command::Wait).unwrap();
  let a=engine::actor(&r,id).unwrap();assert_eq!(a.state,"hunting");assert_eq!(a.last_seen.as_ref().map(|p|(p.x,p.y)),Some((10.,10.)));
 }
 assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),(10.,10.));
 engine::dispatch(&mut r,&Command::Wait).unwrap();let a=engine::actor(&r,id).unwrap();assert_eq!(a.state,"wandering");assert!(a.last_seen.as_ref().is_none());
}
#[test]
fn hunter_refreshes_awareness_at_old_location_if_player_is_plausibly_visible() {
 let mut r=room(51.);r.player.x=16.;let id=rat(&mut r,10.,"hunting");let pid=r.player.id;
 let a=engine::actor_mut(&mut r,id).unwrap();a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();a.conditions.insert("targetId".into(),pid);
 engine::dispatch(&mut r,&Command::Wait).unwrap();let a=engine::actor(&r,id).unwrap();assert_eq!(a.state,"hunting");assert_eq!(a.last_seen.as_ref().map(|p|(p.x,p.y)),Some((16.,10.)));assert_eq!(engine::condition(a,"seesPlayer"),1.);
}
#[test]
fn successful_sleep_detection_records_awareness_without_an_extra_wandering_roll() {
 let mut original=room(51.);original.player.x=18.;let id=rat(&mut original,10.,"sleeping");
 let mut witnessed=false;
 for seed in 1..=100 {
  let mut r=original.clone();r.rng=seed as f64;engine::dispatch(&mut r,&Command::Wait).unwrap();
  let a=engine::actor(&r,id).unwrap();if a.state=="hunting" {
   assert_eq!((a.x,a.y),(10.,10.));assert_eq!(a.last_seen.as_ref().map(|p|(p.x,p.y)),Some((18.,10.)));assert_eq!(engine::condition(a,"targetId"),r.player.id);witnessed=true;break;
  }
 }
 assert!(witnessed,"expected at least one seeded successful 10% wake");
}
#[test]
fn approved_heart_rewards_keep_existing_bonuses_and_only_change_new_revision_drops() {
 for (kind,bonus) in [("cave-bat",1.),("tiger",2.),("sabretooth",3.),("dragon",4.)] {
  let a=population::create_actor_revision(kind,100.,10.,10.,false,51.).unwrap();
  let old=inventory::heart_for(&a,200.).unwrap();let old_versioned=inventory::heart_for_revision(&a,200.,50.).unwrap();assert_eq!(old,old_versioned);
  let new=inventory::heart_for_revision(&a,200.,51.).unwrap();assert_eq!(new.bonus.as_ref().unwrap().perception.value_or(0.),bonus);
  assert_eq!((new.energy.clone(),new.exp.clone()),(old.energy.clone(),old.exp.clone()));
  let mut stats=new.bonus.clone();stats.as_mut().unwrap().perception=old.bonus.as_ref().unwrap().perception.clone();assert_eq!(stats,old.bonus);
  let mut r=room(51.);r.inventory.push(new);inventory::command(&mut r,&Command::Use{id:200.}).unwrap();assert_eq!(r.player.stats.perception,bonus);
 }
 let a=population::create_actor_revision("banshee",100.,10.,10.,false,51.).unwrap();assert_eq!(inventory::heart_for_revision(&a,200.,51.).unwrap(),inventory::heart_for(&a,200.).unwrap());
}
#[test]
fn observation_and_queries_do_not_mutate_saved_run_or_rng() {
 let mut r=room(51.);rat(&mut r,20.,"wandering");let before=serde_json::to_string(&r).unwrap();
 for a in r.actors_at(r.depth){let _=vision::detection_chance(&r,a,&r.player);let _=vision::visible_actor(&r,a);}
 let _=vision::observe_terrain(&r);assert_eq!(serde_json::to_string(&r).unwrap(),before);
}
#[test]
fn unseen_stair_goal_finishes_at_legal_edge_but_visible_target_is_reacquired() {
 for visible in [false,true] {
  let mut r=room(51.);r.player.x=if visible {16.} else {40.};let id=rat(&mut r,11.,"hunting");let pid=r.player.id;
  let f=r.floor_mut().unwrap();let w=f.width();f.tiles[10*w+10]="down".into();
  let a=engine::actor_mut(&mut r,id).unwrap();a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();a.conditions.insert("targetId".into(),pid);a.conditions.insert("monsterUnseenTurns".into(),20.);
  engine::dispatch(&mut r,&Command::Wait).unwrap();let a=engine::actor(&r,id).unwrap();
  assert_ne!((a.x,a.y),(10.,10.));assert_eq!(a.state,if visible {"hunting"} else {"wandering"});
  assert_eq!(a.last_seen.as_ref().map(|p|(p.x,p.y)),if visible {Some((16.,10.))} else {None});
 }
}
#[test]
fn offscreen_low_energy_hunt_persists_then_finishes_at_stair_edge() {
 for revision in [50.,51.] {
  let mut r=room(revision);let id=rat(&mut r,20.,"hunting");
  let a=engine::actor_mut(&mut r,id).unwrap();a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();a.conditions.insert("monsterUnseenTurns".into(),20.);
  let f=r.floor_mut().unwrap();let w=f.width();f.tiles[10*w+10]="down".into();f.last_tick=0.;f.dirty=true;
  r.depth=2.;r.tick=1.;corpse_core::offscreen::catch_up_floor(&mut r,1.).unwrap();
  let a=r.actors_at(1.).iter().find(|a|a.id==id).unwrap();
  if revision<51. {assert_eq!(a.state,"sleeping");assert!(a.last_seen.as_ref().is_none());continue;}
  assert_eq!(a.state,"hunting");assert!(a.x<20.);assert!(a.last_seen.as_ref().is_some());
  for tick in 2..=15 {
   r.tick=tick as f64;corpse_core::offscreen::catch_up_floor(&mut r,1.).unwrap();
   let a=r.actors_at(1.).iter().find(|a|a.id==id).unwrap();assert_ne!((a.x,a.y),(10.,10.));
   if a.last_seen.as_ref().is_none(){assert_eq!(a.state,"wandering");break;}
  }
  let a=r.actors_at(1.).iter().find(|a|a.id==id).unwrap();assert!(a.last_seen.as_ref().is_none());
  assert_eq!((a.x-10.).abs().max((a.y-10.).abs()),1.);
 }
}
#[test]
fn offscreen_low_energy_stair_follower_keeps_chase_and_arrives_off_stair() {
 let mut r=room(51.);let id=rat(&mut r,11.,"hunting");
 let a=engine::actor_mut(&mut r,id).unwrap();a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();
 for (k,v) in [("monsterUnseenTurns",20.),("chaseDepth",2.),("chaseExitX",10.),("chaseExitY",10.),("chaseLandingX",10.),("chaseLandingY",10.)]{a.conditions.insert(k.into(),v);}
 let f=r.floor_mut().unwrap();let w=f.width();f.tiles[10*w+10]="down".into();f.last_tick=0.;f.dirty=true;
 r.depth=2.;let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("floor".into());f.tiles[10*w+10]="up".into();f.last_tick=0.;
 r.tick=1.;corpse_core::offscreen::catch_up_floor(&mut r,1.).unwrap();
 assert!(r.actors_at(1.).iter().all(|a|a.id!=id));let a=r.actors_at(2.).iter().find(|a|a.id==id).unwrap();
 assert_eq!(a.state,"hunting");assert_eq!((a.x-10.).abs().max((a.y-10.).abs()),1.);assert!(!a.conditions.contains_key("chaseDepth"));
}
