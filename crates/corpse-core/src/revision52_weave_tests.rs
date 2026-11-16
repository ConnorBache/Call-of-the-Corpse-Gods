//! R52 arrival-consumption regressions; historical R41 cases remain in engine.rs.
use super::*;
use crate::timeweave::{Cell,Plane,Segment,Trail};

fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;
    r.player.abilities=vec![crate::progression::WEAVE_ID.into()];
    r.player.active.clear();r.player.conditions.clear();r.inventory.clear();r.player.hp=10000.;r.player.max_hp=10000.;r.offers.clear();
    for a in r.actors.values_mut(){*a=std::sync::Arc::new(Vec::new());}
    for g in r.ground.values_mut(){*g=std::sync::Arc::new(Vec::new());}
    let f=r.floor_mut().unwrap();for tile in &mut f.tiles {*tile="floor".into();}
    f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;
    f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    r
}
fn cell(x:u32,y:u32)->Cell{Cell{floor:1,plane:Plane::Waking,tile:y*79+x}}
fn lay(r:&mut Run,from:Cell,to:Cell){
    std::sync::Arc::make_mut(r.timeweave.get_or_insert_with(||std::sync::Arc::new(Trail::default())))
        .append(Segment{cell:from,from,to});
}
fn tiles(r:&Run)->Vec<Cell>{r.timeweave.as_ref().unwrap().iter().map(|s|s.cell).collect()}

#[test]
fn backtracking_unrolls_each_entered_segment_and_can_continue(){
    let mut r=room();
    for _ in 0..3 {assert!(dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);}
    let retained=crate::temporal::Snapshot::capture(&r);
    let tick=r.tick;let rng=r.rng;let phase=r.phase.clone();let history=r.resource_history.clone();
    for x in (10..13).rev(){
        let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();
        assert!(!out.took_turn);assert!(out.changed);assert!(out.pending_snapshot.is_none());
        assert_eq!(r.player.x,x as f64);assert!(r.timeweave.as_ref().unwrap().get(cell(x,10)).is_none());
        assert_eq!(r.timeweave.as_ref().unwrap().len(),(x-10) as usize);
        assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);assert_eq!(r.phase,phase);assert_eq!(r.resource_history,history);
    }
    assert_eq!(retained.state.timeweave.as_ref().unwrap().len(),3);
    // The consumed arrows are gone: forward movement is ordinary, with a fresh trail.
    assert!(dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    assert_eq!(tiles(&r),vec![cell(10,10)]);
}

#[test]
fn correct_middle_entry_preserves_both_older_and_newer_segments(){
    let mut r=room();r.player.x=11.;
    for (from,to) in [(cell(9,10),cell(10,10)),(cell(10,10),cell(11,10)),(cell(20,10),cell(21,10))]{lay(&mut r,from,to);}
    assert!(!dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    assert_eq!(tiles(&r),vec![cell(9,10),cell(20,10)]);
    let bytes=serde_json::to_vec(&r).unwrap();let restored:Run=serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.timeweave,r.timeweave);assert!(crate::timeweave::validate_run(&restored).is_ok());
    assert_eq!(serde_json::to_vec(&restored).unwrap(),bytes);
}

#[test]
fn wrong_angle_entry_destroys_entered_and_older_on_arrival_not_departure(){
    let mut r=room();r.player.x=11.;r.player.y=11.;
    for x in 9..12 {lay(&mut r,cell(x,10),cell(x+1,10));}
    let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:-1.}).unwrap();
    assert!(out.took_turn);assert_eq!((r.player.x,r.player.y),(10.,10.));
    assert_eq!(tiles(&r),vec![cell(11,10),cell(11,11)]);
    assert_eq!(r.timeweave.as_ref().unwrap().get(cell(11,11)).unwrap().to,cell(10,10));
}

#[test]
fn blocked_entry_does_not_destroy_any_segment(){
    let mut r=room();r.player.x=11.;
    lay(&mut r,cell(10,10),cell(11,10));r.floor_mut().unwrap().tiles[10*79+10]="wall".into();
    let trail=r.timeweave.clone();let tick=r.tick;
    let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert_eq!(r.timeweave,trail);assert_eq!(r.tick,tick);
}

#[test]
fn attack_on_weave_is_charged_and_does_not_consume_it(){
    let mut r=room();r.player.x=11.;lay(&mut r,cell(10,10),cell(11,10));
    let mut a=crate::population::create_actor_revision("rat",r.next_id,10.,10.,false,52.).unwrap();
    r.next_id+=1.;a.hp=10000.;a.max_hp=10000.;a.conditions.insert("paralyzed".into(),10.);
    r.actors_at_mut(1.).push(a);let trail=r.timeweave.clone();let tick=r.tick;
    assert!(dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    assert!(r.tick>tick);assert_eq!(r.player.x,11.);assert_eq!(r.timeweave,trail);
}

#[test]
fn pursuit_credit_and_haste_preserve_arrival_consumption(){
    for pursuit in [false,true] {
        let mut r=room();r.player.x=11.;lay(&mut r,cell(10,10),cell(11,10));
        r.player.conditions.insert(if pursuit{"r50PursuitCredit"}else{"hasted"}.into(),5.);
        let tick=r.tick;let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();
        assert!(!out.took_turn);assert_eq!(r.tick,tick);assert!(r.timeweave.as_ref().unwrap().is_empty());
        if pursuit {assert_eq!(condition(&r.player,"r50PursuitCredit"),0.);}
        else {assert_eq!(condition(&r.player,"hasted"),5.);}
    }
}

#[test]
fn dream_weave_consumption_keeps_waking_segments_and_body(){
    let mut r=room();lay(&mut r,cell(10,10),cell(11,10));
    r.dreamwalking=crate::dreamwalking::State{unlocked:true,..Default::default()}.into();
    assert_eq!(crate::dreamwalking::ability(&mut r,crate::dreamwalking::DREAMWALK_ID).unwrap(),Some(true));
    let body=crate::dreamwalking::body(&r).unwrap().clone();
    assert!(dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    let dream=Cell{plane:Plane::Dream,..cell(10,10)};
    assert!(r.timeweave.as_ref().unwrap().get(dream).is_some());
    assert!(!dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    assert!(r.timeweave.as_ref().unwrap().get(dream).is_none());
    assert!(r.timeweave.as_ref().unwrap().get(cell(10,10)).is_some());
    let after=crate::dreamwalking::body(&r).unwrap();assert_eq!((after.x,after.y),(body.x,body.y));
}

#[test]
fn older_save_source_arrow_cannot_delete_newer_unrelated_segments(){
    let mut r=room();r.player.x=11.;r.player.y=11.;
    for (from,to) in [(cell(9,10),cell(10,10)),(cell(10,10),cell(11,10)),
        (cell(20,10),cell(21,10)),(cell(11,11),cell(12,11))]{lay(&mut r,from,to);}
    assert!(dispatch(&mut r,&Command::Move{dx:-1.,dy:-1.}).unwrap().took_turn);
    assert_eq!(tiles(&r),vec![cell(20,10),cell(11,11)]);
    assert_eq!(r.timeweave.as_ref().unwrap().get(cell(11,11)).unwrap().to,cell(10,10));
}
