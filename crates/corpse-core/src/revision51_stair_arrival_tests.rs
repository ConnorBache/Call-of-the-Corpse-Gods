use super::*;
const ID:f64=101.;
fn room(revision:f64,upward:bool)->(Run,Transfer) {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.actors=Default::default();r.next_id=1000.;r.tick=4.;
    let (from,to)=if upward{(2.,1.)}else{(1.,2.)};r.depth=to;
    r.player.x=10.;r.player.y=9.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    for depth in [from,to] {
        let index=r.floors.iter().position(|f|f.depth==depth).unwrap();
        let f=crate::mutation_journal::make_mut(&mut r.floors[index]);let w=f.width();
        f.tiles.fill("wall".into());f.last_tick=3.;
        for y in 8..=12 {for x in 8..=12 {f.tiles[y*w+x]="floor".into();}}
        f.tiles[10*w+10]=if (depth==from)^upward {"down"}else{"up"}.into();
    }
    let mut a=Actor{id:ID,name:"Stair follower".into(),x:if revision>=51.{9.}else{10.},y:10.,hp:100.,max_hp:100.,energy:100.,
        state:"hunting".into(),stats:crate::Stats{max_energy:100.,..Default::default()},..Default::default()};
    a.last_seen=crate::ActorLastSeen{x:10.,y:10.}.into();
    for (k,v) in [("chaseDepth",to),("chaseExitX",10.),("chaseExitY",10.),("chaseLandingX",10.),("chaseLandingY",10.)] {a.conditions.insert(k.into(),v);}
    r.actors_at_mut(from).push(a);
    (r,Transfer{id:ID,from,to,tick:4.,fall:false})
}
fn seal_adjacent(r:&mut Run,t:Transfer,except:Option<(i32,i32)>){
    for (i,d) in crate::movement::DIRECTIONS.iter().enumerate(){
        let (x,y)=(10+d.x,10+d.y);if except==Some((x,y))||(x,y)==(10,9){continue;}
        r.actors_at_mut(t.to).push(Actor{id:200.+i as f64,x:x as f64,y:y as f64,hp:100.,max_hp:100.,state:"sleeping".into(),..Default::default()});
    }
}
#[test]
fn player_blocks_exit_but_arrivals_use_another_adjacent_square_in_both_directions(){
    for upward in [false,true]{let(mut r,t)=room(51.,upward);seal_adjacent(&mut r,t,Some((11,10)));let rng=r.rng;
        apply_transfer(&mut r,t).unwrap();let a=r.actors_at(t.to).iter().find(|a|a.id==ID).unwrap();
        assert_eq!((a.x,a.y),(11.,10.));assert!(r.actors_at(t.from).is_empty());assert!(!a.conditions.contains_key("chaseDepth"));assert_eq!(r.rng,rng);
    }
}
#[test]
fn all_adjacent_blocked_defers_and_retries_after_save_and_temporal_restore(){
    let(mut r,t)=room(51.,false);seal_adjacent(&mut r,t,None);let rng=r.rng;
    apply_transfer(&mut r,t).unwrap();assert_eq!(r.actors_at(t.from)[0].id,ID);assert!(r.actors_at(t.to).iter().all(|a|a.id!=ID));
    assert_eq!(r.actors_at(t.from)[0].conditions.get("chaseDepth"),Some(&t.to));assert_eq!(r.rng,rng);
    let mut saved:Run=serde_json::from_str(&crate::storage::serialize(&r).unwrap()).unwrap();
    let snapshot=crate::temporal::Snapshot::capture(&r);let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();let mut anchored=anchor.native_state().unwrap().clone();
    r.actors_at_mut(t.from).clear();snapshot.restore(&mut r).unwrap();
    for run in [&mut r,&mut saved,&mut anchored]{
        run.actors_at_mut(t.to).retain(|a|(a.x,a.y)!=(11.,10.));
        apply_transfer(run,Transfer{tick:5.,..t}).unwrap();let a=run.actors_at(t.to).iter().find(|a|a.id==ID).unwrap();
        assert_eq!((a.x,a.y),(11.,10.));assert_eq!(run.rng,rng);
    }
}
#[test]
fn stair_center_is_never_a_fallback_but_revision50_retains_it(){
    for revision in [50.,51.]{let(mut r,t)=room(revision,false);seal_adjacent(&mut r,t,None);
        apply_transfer(&mut r,t).unwrap();let a=r.actors_at(t.to).iter().find(|a|a.id==ID);
        assert_eq!(a.is_some(),revision<51.);if let Some(a)=a{assert_eq!((a.x,a.y),(10.,10.));}
    }
}
#[test]
fn source_approach_preserves_final_stair_action_without_ever_standing_on_stair(){
    let(mut r,t)=room(51.,false);r.depth=t.from;r.player.x=-999.;r.player.y=-999.;
    r.actors_at_mut(t.from)[0].x=8.;let mut pending=vec![];
    precise_actor_phase(&mut r,&mut pending).unwrap();assert!(pending.is_empty());
    assert_eq!((r.actors_at(t.from)[0].x,r.actors_at(t.from)[0].y),(9.,10.));
    r.tick+=1.;precise_actor_phase(&mut r,&mut pending).unwrap();assert_eq!(pending.len(),1);
    assert_eq!((r.actors_at(t.from)[0].x,r.actors_at(t.from)[0].y),(9.,10.));
}
#[test]
fn arrival_checks_walls_diagonal_corners_and_second_stairs(){
    let(mut r,t)=room(51.,false);seal_adjacent(&mut r,t,Some((11,9)));
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles[9*w+10]="wall".into();f.tiles[10*w+11]="down".into();
    apply_transfer(&mut r,t).unwrap();assert!(r.actors_at(t.from).iter().any(|a|a.id==ID));
}
#[test]
fn dead_creatures_on_stairs_do_not_revive_in_precise_or_aggregate_offscreen_simulation(){
    for revision in [50.,51.]{for precise in [false,true]{let(mut r,t)=room(revision,false);
        let a=&mut r.actors_at_mut(t.from)[0];a.x=10.;a.y=10.;a.dead=true.into();a.hp=0.;a.corpse_age=400.0.into();a.conditions.clear();
        if precise {let mut head=proxy(&r,t.from,4.);precise_actor_phase(&mut head,&mut vec![]).unwrap();r.actors.replace_from(head.actors);}
        else {approximate(&mut r,t.from,1.).unwrap();}
        assert_eq!(r.actors_at(t.from)[0].dead.value_or(false),revision>=51.);
    }}
}
