use super::*;
use crate::{ActorLastSeen, engine};
const ID: f64 = 800.;
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into(); r.depth=1.; r.tick=0.; r.next_id=2000.;
    r.actors=Default::default(); r.ground.clear(); r.inventory.clear();
    if revision>=52. {r.floors.truncate(3);}
    r.player.x=14.; r.player.y=10.; r.player.hp=100.; r.player.max_hp=100.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order=Field::Missing; r.player.equipment_abilities=Field::Missing;
    for floor in &mut r.floors {
        let f=crate::mutation_journal::make_mut(floor); let w=f.width();
        f.tiles.fill("wall".into()); f.lights.fill(0.); f.seen.fill(false); f.dirty=false; f.last_tick=0.;
        if f.depth==1. {
            for y in 8..=12 { for x in 8..=18 { f.tiles[y*w+x]="floor".into(); } }
            f.tiles[10*w+14]="chasm".into();
        } else {
            for y in 18..=22 { for x in 20..=24 { f.tiles[y*w+x]="floor".into(); } }
            f.tiles[8*w+8]="up".into(); f.tiles[22*w+24]="down".into();
        }
    }
    for depth in 1..=r.floors.len() {r.actors_at_mut(depth as f64);r.ground_at_mut(depth as f64);}
    let mut a=Actor {id:ID,name:"Flying hunter".into(),x:10.,y:10.,hp:100.,max_hp:100.,energy:100.,
        stats:crate::Stats{max_energy:100.,perception:200.,..Default::default()},
        state:"hunting".into(),abilities:vec!["enemy:beast:flight".into()],
        last_seen:ActorLastSeen{x:13.,y:10.}.into(),..Default::default()};
    a.conditions.insert("targetId".into(),r.player.id);
    a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
    a.conditions.insert("paleTraitsInitialized".into(),1.);
    r.actors_at_mut(1.).push(a); r
}
fn source(r:&Run)->&Actor {r.actors_at(1.).iter().find(|a|a.id==ID).unwrap()}
fn source_mut(r:&mut Run)->&mut Actor {r.actors_at_mut(1.).iter_mut().find(|a|a.id==ID).unwrap()}
fn transfer(r:&Run)->Transfer {Transfer{id:ID,from:1.,to:2.,tick:r.tick,fall:true}}
fn depart(r:&mut Run) {
    assert!(crate::travel::jump_chasm(r).unwrap());
    assert_eq!((r.depth,r.player.x,r.player.y),(2.,20.,18.));
}
fn at_exit(r:&mut Run) {let a=source_mut(r); a.x=14.;a.y=10.;}
fn block_landing(r:&mut Run) {
    for (i,(x,y)) in [(21.,18.),(20.,19.),(21.,19.)].into_iter().enumerate() {
        r.actors_at_mut(2.).push(Actor{id:900.+i as f64,x,y,hp:100.,max_hp:100.,state:"neutral".into(),..Default::default()});
    }
}
fn tick_source(r:&mut Run) {r.tick+=1.;catch_up_floor(r,1.).unwrap();}
#[test]
fn departure_records_actual_fall_landing_and_does_not_teleport_distant_hunters() {
    let mut r=room(52.);let rng=r.rng;
    assert_eq!(crate::vision::detection_chance(&r,source(&r),&r.player),100.);
    depart(&mut r);
    let a=source(&r);assert_eq!((a.x,a.y,a.hp),(10.,10.,100.));
    for (key,value) in [("chaseDepth",2.),("chaseExitX",14.),("chaseExitY",10.),
        ("chaseLandingX",20.),("chaseLandingY",18.),("chaseChasm",1.)] {assert_eq!(c(a,key),value,"{key}");}
    assert_eq!(a.last_seen,ActorLastSeen{x:14.,y:10.}.into());
    assert!(r.actors_at(2.).is_empty());assert_eq!(r.rng,rng);
}
#[test]
fn unrelated_unaware_dead_and_grounded_creatures_are_not_new_flying_pursuers() {
    for mode in ["no target","other target","wandering","sleeping","neutral","dead","zero hp","grounded","occluded","invisible"] {
        let mut r=room(52.);
        match mode {
            "no target"=>{source_mut(&mut r).conditions.shift_remove("targetId");},
            "other target"=>{source_mut(&mut r).conditions.insert("targetId".into(),999.);},
            "wandering"|"sleeping"|"neutral"=>source_mut(&mut r).state=mode.into(),
            "dead"=>{let a=source_mut(&mut r);a.dead=true.into();a.hp=0.;},
            "zero hp"=>source_mut(&mut r).hp=0.,
            "grounded"=>source_mut(&mut r).abilities.clear(),
            "occluded"=>{let f=r.floor_mut().unwrap();let w=f.width();for y in 8..=12 {f.tiles[y*w+12]="wall".into();}},
            "invisible"=>{r.player.conditions.insert("invisible".into(),5.);},
            _=>unreachable!(),
        }
        depart(&mut r);assert!(!source(&r).conditions.contains_key("chaseChasm"),"{mode}");
        assert!(!source(&r).conditions.contains_key("chaseDepth"),"{mode}");
    }
}
#[test]
fn forced_and_expired_flight_cohorts_record_the_reserved_player_destination() {
    let mut r=room(52.);let player=r.player.id;
    // Including the flyer in an accidental-fall cohort must not drop it.
    crate::travel::fall_actors_one_level(&mut r,1.,&[player,ID],None).unwrap();
    assert_eq!(r.depth,2.);let a=source(&r);
    assert_eq!((c(a,"chaseLandingX"),c(a,"chaseLandingY")),(r.player.x,r.player.y));
    assert_eq!(c(a,"chaseChasm"),1.);assert_eq!(a.hp,100.);
    assert!(r.actors_at(2.).is_empty());
}
#[test]
fn normal_fast_haste_and_combined_offscreen_pursuit_reach_exit_before_transfer() {
    for (fast,haste,turns) in [(false,false,4),(true,false,2),(false,true,2),(true,true,1)] {
        let mut r=room(52.);
        if fast {source_mut(&mut r).abilities.push("enemy:beast:fast".into());}
        if haste {source_mut(&mut r).conditions.insert("hasted".into(),20.);}
        depart(&mut r);
        for turn in 1..=turns {
            tick_source(&mut r);
            assert_eq!(r.actors_at(2.).iter().any(|a|a.id==ID),turn==turns,"fast={fast} haste={haste} turn={turn}");
        }
        let a=r.actors_at(2.).iter().find(|a|a.id==ID).unwrap();
        assert_eq!((a.x,a.y,a.hp),(21.,18.,100.));
        assert!(!a.conditions.contains_key("chaseDepth"));assert!(!a.conditions.contains_key("chaseChasm"));
        assert_eq!(a.last_seen,ActorLastSeen{x:20.,y:18.}.into());
        assert!(r.actors_at(1.).is_empty());
        let actors=r.actors_at(2.).to_vec();catch_up_floor(&mut r,1.).unwrap();assert_eq!(r.actors_at(2.),actors);
    }
}
#[test]
fn player_normal_and_haste_dispatch_preserve_pursuit() {
    for hasted in [false,true] {
        let mut r=room(52.);if hasted {r.player.conditions.insert("hasted".into(),5.);}
        engine::dispatch(&mut r,&crate::Command::Descend).unwrap();
        assert_eq!(r.depth,2.);assert_eq!(c(source(&r),"chaseChasm"),1.);
        for _ in 0..10 {
            if r.actors_at(2.).iter().any(|a|a.id==ID) {break;}
            engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
        }
        assert!(r.actors_at(2.).iter().any(|a|a.id==ID && a.hp==100.));
    }
}
#[test]
fn occupied_landings_defer_without_damage_and_retry_without_overlap() {
    let mut r=room(52.);depart(&mut r);at_exit(&mut r);block_landing(&mut r);
    let t=transfer(&r);let rng=r.rng;let next_id=r.next_id;
    apply_transfer(&mut r,t).unwrap();assert_eq!(source(&r).hp,100.);assert_eq!(c(source(&r),"chaseChasm"),1.);
    assert!(!r.actors_at(2.).iter().any(|a|a.id==ID));assert_eq!((r.rng,r.next_id),(rng,next_id));
    r.actors_at_mut(2.).retain(|a|a.id!=900.);apply_transfer(&mut r,t).unwrap();
    let a=r.actors_at(2.).iter().find(|a|a.id==ID).unwrap();assert_eq!((a.x,a.y,a.hp),(21.,18.,100.));
    assert_eq!(r.actors_at(2.).iter().filter(|a|(a.x,a.y)==(21.,18.)).count(),1);
}
#[test]
fn landing_respects_portal_alias_occupancy_stairs_and_corner_blocks() {
    let mut r=room(52.);depart(&mut r);at_exit(&mut r);block_landing(&mut r);
    // Free the physical east cell, but reserve its portal-equivalent endpoint.
    let a=r.actors_at_mut(2.).iter_mut().find(|a|a.id==900.).unwrap();a.x=23.;a.y=20.;
    let f=r.floor_mut().unwrap();let w=f.width();
    f.portals=vec![crate::portals_generation::PortalPair{a:18*w+21,b:20*w+23,color_index:0}].into();
    let t=transfer(&r);apply_transfer(&mut r,t).unwrap();assert_eq!(source(&r).id,ID);
    let f=r.floor_mut().unwrap();f.portals=Field::Missing;f.tiles[18*w+21]="up".into();
    apply_transfer(&mut r,t).unwrap();assert_eq!(source(&r).id,ID);
    r.actors_at_mut(2.).retain(|a|a.id!=902.);
    let f=r.floor_mut().unwrap();f.tiles[18*w+21]="wall".into();f.tiles[19*w+20]="wall".into();
    apply_transfer(&mut r,t).unwrap();assert_eq!(source(&r).id,ID);
}
#[test]
fn missing_or_invalid_destination_and_closed_chasm_never_transfer() {
    for mode in ["missing floor","no landing","closed chasm","lost flight","wrong depth"] {
        let mut r=room(52.);depart(&mut r);at_exit(&mut r);let mut t=transfer(&r);
        match mode {
            "missing floor"=>r.floors.retain(|f|f.depth!=2.),
            "no landing"=>{let f=r.floor_mut().unwrap();f.tiles.fill("wall".into());},
            "closed chasm"=>{let f=crate::mutation_journal::make_mut(&mut r.floors[0]);let w=f.width();f.tiles[10*w+14]="floor".into();},
            "lost flight"=>source_mut(&mut r).abilities.clear(),
            "wrong depth"=>t.to=3.,_=>unreachable!(),
        }
        let rng=r.rng;let count=r.floors.len();apply_transfer(&mut r,t).unwrap();
        assert_eq!(source(&r).hp,100.,"{mode}");assert_eq!((r.rng,r.floors.len()),(rng,count),"{mode}");
    }
}
#[test]
fn save_snapshot_and_future_restore_the_pending_sweep_and_retry_identically() {
    let mut r=room(52.);depart(&mut r);at_exit(&mut r);block_landing(&mut r);
    let t=transfer(&r);apply_transfer(&mut r,t).unwrap();
    let mut loaded=crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();
    let snap=crate::temporal::Snapshot::capture(&r);
    let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    let mut future=anchor.native_state().unwrap().clone();
    r.actors_at_mut(1.).clear();snap.restore(&mut r).unwrap();
    for state in [&mut r,&mut loaded,&mut future] {
        state.actors_at_mut(2.).retain(|a|a.id!=900.);apply_transfer(state,t).unwrap();
        let a=state.actors_at(2.).iter().find(|a|a.id==ID).unwrap();
        assert_eq!((a.x,a.y,a.hp),(21.,18.,100.));assert!(!a.conditions.contains_key("chaseChasm"));
    }
    assert_eq!(r.actors,loaded.actors);assert_eq!(r.actors,future.actors);assert_eq!(r.rng,loaded.rng);
}
#[test]
fn legacy_departure_and_unmarked_legacy_fall_damage_are_unchanged() {
    let mut r=room(51.);depart(&mut r);assert!(!source(&r).conditions.contains_key("chaseDepth"));
    // Old saves have no new flag; retain their established transfer/damage.
    for revision in [51.,52.] {
        let mut r=room(revision);depart(&mut r);at_exit(&mut r);
        let a=source_mut(&mut r);a.conditions.shift_remove("chaseChasm");
        for (k,v) in [("chaseDepth",2.),("chaseExitX",14.),("chaseExitY",10.),("chaseLandingX",20.),("chaseLandingY",18.)] {a.conditions.insert(k.into(),v);}
        let t=transfer(&r);apply_transfer(&mut r,t).unwrap();
        assert!(r.actors_at(2.).iter().find(|a|a.id==ID).unwrap().hp<100.);
    }
}
#[test]
fn airborne_dragon_does_not_fold_beside_chasm_or_buy_more_flight() {
    let mut r=room(52.);
    let a=source_mut(&mut r);a.creature_kind="dragon".to_string().into();a.abilities.clear();
    a.conditions.insert("flying".into(),6.);a.conditions.insert(crate::revision52_enemy_followups::FLIGHT.into(),6.);
    depart(&mut r);let energy=source(&r).energy;
    tick_source(&mut r);assert_eq!(source(&r).x,11.);assert_eq!(source(&r).energy,energy);
    assert_eq!(c(source(&r),"flying"),5.);
    for _ in 0..3 {tick_source(&mut r);}
    let a=r.actors_at(2.).iter().find(|a|a.id==ID).unwrap();
    assert_eq!(a.hp,100.);assert_eq!(a.energy,energy);assert_eq!(c(a,"flying"),2.);
}
#[test]
fn player_fall_extends_one_floor_ahead_but_pursuer_transfer_does_not_generate_more() {
    for forced in [false,true] {
        let mut r=room(52.);r.floors.truncate(2);r.actors.shift_remove("3");r.ground.shift_remove("3");
        if forced {
            // The isolated upstairs tile would correctly be the nearest player
            // landing but leave no adjacent enemy arrival. Test the horizon
            // with a usable shared destination; blocked arrivals are above.
            let f=crate::mutation_journal::make_mut(&mut r.floors[1]);let w=f.width();f.tiles[8*w+8]="wall".into();
            let player=r.player.id;crate::travel::fall_actors_one_level(&mut r,1.,&[player],None).unwrap();
        }
        else {depart(&mut r);}
        assert_eq!(r.floors.len(),3);assert_eq!(r.floors.last().unwrap().depth,3.);
        assert_eq!((c(source(&r),"chaseLandingX"),c(source(&r),"chaseLandingY")),(r.player.x,r.player.y));
        let prefetched=r.floors[2].clone();let next_id=r.next_id;
        at_exit(&mut r);let t=transfer(&r);apply_transfer(&mut r,t).unwrap();
        assert!(r.actors_at(2.).iter().any(|a|a.id==ID));assert_eq!(r.floors.len(),3);
        assert_eq!(r.floors[2],prefetched);assert_eq!(r.next_id,next_id);
    }
}
#[test]
fn chase_marker_is_bounded_and_cleared_with_other_chase_state() {
    assert!(crate::revision52_enemy_followups::valid_condition("chaseChasm",1.));
    for value in [0.,2.,f64::NAN,f64::INFINITY] {assert!(!crate::revision52_enemy_followups::valid_condition("chaseChasm",value));}
    for value in [0.,2.,-1.,0.5] {
        let mut r=room(52.);depart(&mut r);source_mut(&mut r).conditions.insert("chaseChasm".into(),value);
        let before=r.clone();assert!(crate::storage::validate(&r).is_err());
        assert!(engine::dispatch(&mut r,&crate::Command::Wait).is_err());assert_eq!(r,before);
    }
    let mut r=room(52.);depart(&mut r);crate::travel::clear_floor_chase(source_mut(&mut r));
    assert!(!source(&r).conditions.keys().any(|key|key.starts_with("chase")));
}

#[test]
fn finite_flight_expiry_and_blocked_routes_cannot_sweep_from_a_distance() {
    for blocked in [false,true] {
        let mut r=room(52.);
        if !blocked {let a=source_mut(&mut r);a.abilities.clear();a.conditions.insert("flying".into(),2.);}
        depart(&mut r);
        if blocked {let f=crate::mutation_journal::make_mut(&mut r.floors[0]);let w=f.width();for y in 8..=12 {f.tiles[y*w+12]="wall".into();}}
        for _ in 0..6 {tick_source(&mut r);}
        assert!(r.actors_at(2.).iter().all(|a|a.id!=ID));
        assert_eq!(source(&r).hp,100.);assert!(source(&r).x<14.);
        if !blocked {assert_eq!(c(source(&r),"flying"),0.);}
    }
}
#[test]
fn pending_chase_is_admitted_if_player_returns_to_source_before_arrival() {
    let mut r=room(52.);depart(&mut r);r.depth=1.;r.player.x=18.;r.player.y=10.;
    engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
    assert!(!source(&r).conditions.contains_key("chaseChasm"));
    assert_eq!(source(&r).conditions.get("targetId"),Some(&r.player.id));
}
#[test]
fn flight_immunity_and_expiry_over_chasm_keep_ordinary_fall_rules() {
    let mut r=room(52.);r.player.x=18.;
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles[10*w+10]="chasm".into();
    crate::travel::fall_actors_one_level(&mut r,1.,&[ID],None).unwrap();
    assert_eq!((source(&r).x,source(&r).y,source(&r).hp),(10.,10.,100.));
    assert!(r.actors_at(2.).is_empty());assert_eq!(r.depth,1.);
    // Finite flight ending over the gap remains an accidental fall, not a
    // free sweep. Only the newly enrolled, still-airborne chase avoids impact.
    source_mut(&mut r).abilities.clear();
    crate::travel::fall_actors_one_level(&mut r,1.,&[ID],None).unwrap();
    assert!(r.actors_at(1.).is_empty());assert_eq!(r.depth,1.);
    let a=r.actors_at(2.).iter().find(|a|a.id==ID).unwrap();assert!(a.hp<100.);
    assert!(!a.conditions.contains_key("chaseChasm"));
}

#[test]
fn enemy_flight_expiry_wait_reserves_row_major_landings_and_applies_impact() {
    let mut r = room(52.);
    r.player.x = 17.;
    let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles[10 * w + 10] = "chasm".into();
    let source = source_mut(&mut r);
    source.abilities.clear();
    source.conditions.insert("flying".into(), 1.);
    source.conditions.insert("paralyzed".into(), 2.);
    source.state = "sleeping".into();
    let destination = crate::mutation_journal::make_mut(&mut r.floors[1]);
    // All four are one tile from the directly aligned (ineligible) wall.
    for (x, y) in [(10, 9), (9, 10), (11, 10), (10, 11)] {
        destination.tiles[y * w + x] = "floor".into();
    }
    r.actors_at_mut(2.).push(Actor {
        id: ID + 1., name: "Blocker".into(), x: 10., y: 9., hp: 100., max_hp: 100.,
        state: "sleeping".into(), ..Default::default()
    });
    let outcome = engine::dispatch(&mut r, &crate::Command::Wait).unwrap();
    assert!(outcome.took_turn);
    assert_eq!(r.depth, 1.);
    assert!(r.actors_at(1.).iter().all(|a| a.id != ID));
    let landed = r.actors_at(2.).iter().find(|a| a.id == ID).unwrap();
    assert_eq!((landed.x, landed.y, landed.hp), (9., 10., 94.));
    assert_eq!(landed.conditions.get("offscreenActionTick"), Some(&r.tick));
    assert!(!r.log.iter().any(|line| line == "You descend."));
}
