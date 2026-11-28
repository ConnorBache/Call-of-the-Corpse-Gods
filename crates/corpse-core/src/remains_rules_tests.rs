use super::*;
use crate::{Actor, GroundItem, Item, Command, engine, temporal::Engine};

fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();
    r.remains_rules_from=0usize.into();
    r.actors_at_mut(r.depth).clear();
    r.ground_at_mut(r.depth).clear();
    r.player.abilities.clear();
    r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());
    r
}
fn rat(id:f64,x:f64,y:f64,dead:bool)->Actor{
    let mut a=crate::population::create_actor_revision("rat",id,x,y,false,52.).unwrap();
    a.hp=if dead {0.}else{1.};a.dead=dead.into();a.heartless=dead.into();
    a
}
fn loot(id:f64,x:f64,y:f64)->GroundItem{
    GroundItem{x,y,item:Item{id,name:"Test ring".into(),kind:"ring".into(),..Default::default()},..Default::default()}
}
fn kill(r:&mut Run,id:f64){
    engine::damage_environment(r,id,100.,None,"fire",true).unwrap();
}
fn positions(r:&Run,id:f64)->(f64,f64){
    let a=r.actors_at(r.depth).iter().find(|a|a.id==id).unwrap();
    (a.x,a.y)
}
fn heart(r:&Run)->(f64,f64){
    let a=r.ground[&r.depth.to_string()].iter().find(|g|g.item.kind=="heart").unwrap();
    (a.x,a.y)
}

#[test]
fn death_heart_has_origin_priority_and_corpse_spreads_around_item_and_old_corpse(){
    let mut r=room();let d=r.depth;
    r.actors_at_mut(d).push(rat(990.,15.,10.,true));
    r.actors_at_mut(d).push(rat(991.,15.,10.,false));
    kill(&mut r,991.);
    assert_eq!(heart(&r),(15.,10.));
    assert_eq!(positions(&r,990.),(15.,9.));
    assert_eq!(positions(&r,991.),(14.,10.));

    let mut r=room();let d=r.depth;
    r.ground_at_mut(d).push(loot(992.,15.,10.));
    r.actors_at_mut(d).push(rat(991.,15.,10.,false));
    kill(&mut r,991.);
    assert_eq!(heart(&r),(15.,9.));
    assert_eq!(positions(&r,991.),(14.,10.));
    engine::place_item(&mut r,14.,10.,loot(993.,14.,10.).item).unwrap();
    let item=r.ground[&d.to_string()].iter().find(|g|g.item.id==993.).unwrap();
    assert_ne!((item.x,item.y),positions(&r,991.));
}

#[test]
fn adoption_and_snapshot_repair_keep_the_item_fixed_and_legacy_prefix_intact() {
    let mut r=room();let d=r.depth;
    r.remains_rules_from=Field::Missing;
    r.actors_at_mut(d).push(rat(991.,15.,10.,false));
    kill(&mut r,991.);
    assert_eq!(heart(&r),positions(&r,991.));
    let old=crate::temporal::Snapshot::capture(&r);
    assert!(old.value().unwrap().get("remainsRulesFrom").is_none());
    r.replay.push(Command::Wait);
    let mut live=Engine::new(r.clone()).unwrap();
    assert_eq!(live.run.remains_rules_from,Field::Value(1));
    assert_ne!(heart(&live.run),positions(&live.run,991.));
    old.restore(&mut live.run).unwrap();
    assert_eq!(live.run.remains_rules_from,Field::Value(1));
    assert_ne!(heart(&live.run),positions(&live.run,991.));
    let mut playback=r;
    apply_replay(&mut playback,&1usize.into(),0).unwrap();
    assert_eq!(heart(&playback),positions(&playback,991.));
    apply_replay(&mut playback,&1usize.into(),1).unwrap();
    assert_eq!(positions(&playback,991.),positions(&live.run,991.));
}

#[test]
fn chasm_landing_keeps_heart_and_corpse_apart(){
    let mut r=room();let d=r.depth;
    let (x,y)=(15.,10.);
    let w=r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles[y as usize*w+x as usize]="chasm".into();
    let next=crate::fast_mode::below(&r,d);
    let floor=r.floors.iter_mut().find(|f|f.depth==next).unwrap();
    crate::mutation_journal::make_mut(floor).tiles.fill("floor".into());
    r.actors_at_mut(next).clear();
    r.ground_at_mut(next).clear();
    r.actors_at_mut(d).push(rat(991.,x,y,false));
    kill(&mut r,991.);
    assert!(r.actors_at(d).iter().all(|a|a.id!=991.));
    r.depth=next;
    assert_eq!(heart(&r),(x,y));
    assert_eq!(positions(&r,991.),(x,y-1.));
}

#[test]
fn replay_session_restores_the_same_mixed_policy_run(){
    use crate::replay::{ReplayRecord,ReplaySession};
    let initial=crate::startup::create_avatar_run("02-03-09:remains-replay").unwrap();
    let mut old_record=ReplayRecord::try_from_run(&initial).unwrap();
    old_record.remains_rules_from=Field::Missing;
    let mut old=ReplaySession::create(old_record,true).unwrap();
    assert!(!current(&old.engine.run));
    old.engine.dispatch(&Command::Wait).unwrap();
    let mut live=Engine::load(&old.engine.save().unwrap()).unwrap();
    assert_eq!(live.run.remains_rules_from,Field::Value(1));
    live.dispatch(&Command::Wait).unwrap();
    let mut replay=ReplaySession::create(ReplayRecord::try_from_run(&live.run).unwrap(),true).unwrap();
    assert!(!current(&replay.engine.run));
    assert!(replay.step().unwrap());
    assert!(current(&replay.engine.run));
    while replay.step().unwrap(){}
    assert_eq!(replay.engine.save().unwrap(),live.save().unwrap());
    replay.restart().unwrap();
    while replay.step().unwrap(){}
    assert_eq!(replay.engine.save().unwrap(),live.save().unwrap());
}
