use super::*;
use crate::{Field, ActorLastSeen};
const ID: f64 = 801.;
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x=14.; r.player.y=10.; r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.inventory.clear();r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;
    r.actors_at_mut(r.depth).clear();
    let f=r.floor_mut().unwrap(); f.tiles.fill("wall".into());
    f.portals=Field::Missing; f.non_euclidean=Field::Missing; f.speed_up=Field::Missing;
    let w=f.width(); for x in 10..=14 {f.tiles[10*w+x]="floor".into();}
    for x in 11..=12 {f.tiles[10*w+x]="chasm".into();}
    let mut a=Actor { id:ID,x:10.,y:10.,name:"Dragon".into(),creature_kind:"dragon".to_string().into(),hp:952.,max_hp:952.,energy:50.,
        state:"hunting".into(),abilities:vec!["enemy:beast:fast".into(),"enemy:beast:fire-breath".into()],
        last_seen:ActorLastSeen{x:14.,y:10.}.into(),..Default::default() };
    a.stats.max_energy=50.; a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
    r.actors_at_mut(r.depth).push(a); r
}
fn a(r:&Run)->&Actor {engine::actor(r,ID).unwrap()}
fn tick_timer(r:&mut Run) {r.tick+=1.;for key in ["flying",FLIGHT] {engine::tick_condition(engine::actor_mut_trusted(r,ID).unwrap(),key);}}
#[test]
fn pays_once_normal_action_and_fast_moves_do_not_tick_or_reactivate() {
    let mut r=room();let player=r.player.id;let rng=r.rng;
    assert!(!turn(&mut r,ID,Some(player),true).unwrap());assert_eq!(a(&r).energy,50.);
    assert!(turn(&mut r,ID,Some(player),false).unwrap());assert_eq!(a(&r).energy,42.);
    assert_eq!((a(&r).x,a(&r).y),(10.,10.));assert_eq!(condition(a(&r),"flying"),6.);
    assert!(turn(&mut r,ID,Some(player),true).unwrap());assert_eq!(a(&r).x,11.);assert_eq!(condition(a(&r),"flying"),6.);
    tick_timer(&mut r);assert!(turn(&mut r,ID,Some(player),false).unwrap());assert_eq!(a(&r).x,12.);
    assert!(turn(&mut r,ID,Some(player),true).unwrap());assert_eq!(a(&r).x,13.);
    assert!(!engine::flying_in(&r,a(&r)));assert_eq!(a(&r).energy,42.);assert_eq!(r.rng,rng);
}
#[test]
fn fifty_energy_allows_six_activations_without_recharge() {
    let mut r=room(); let player=r.player.id;
    assert_eq!(crate::combat::effective_stats(&r,a(&r)).max_energy,50.);
    for _ in 0..6 {
        assert!(turn(&mut r,ID,Some(player),false).unwrap());land(&mut r,ID);
    }
    assert_eq!(a(&r).energy,2.);assert!(!turn(&mut r,ID,Some(player),false).unwrap());
}
#[test]
fn no_payment_when_ground_is_as_fast_legacy_silenced_or_no_safe_landing() {
    for mode in ["ground","legacy","silenced","landing"] {
        let mut r=room();let player=r.player.id;
        match mode {
            "ground"=>{let f=r.floor_mut().unwrap();let w=f.width();for x in 11..=12 {f.tiles[10*w+x]="floor".into();}},
            "legacy"=>r.simulation_revision=51.0.into(),
            "silenced"=>{engine::actor_mut_trusted(&mut r,ID).unwrap().conditions.insert("silenced".into(),1.);},
            _=>{let f=r.floor_mut().unwrap();let w=f.width();f.tiles[10*w+13]="chasm".into();},
        }
        assert!(!turn(&mut r,ID,Some(player),false).unwrap(),"{mode}");assert_eq!(a(&r).energy,50.);
    }
}
#[test]
fn lost_target_uses_ground_landing_and_saved_flight_resumes_exactly() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    engine::actor_mut_trusted(&mut r,ID).unwrap().last_seen=Field::Missing;
    let mut loaded:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    tick_timer(&mut r);tick_timer(&mut loaded);
    turn(&mut r,ID,None,false).unwrap();turn(&mut loaded,ID,None,false).unwrap();
    assert_eq!(r,loaded);assert_eq!(a(&r).x,10.);assert!(!engine::flying_in(&r,a(&r)));
}
#[test]
fn new_blockade_returns_to_safe_ground_without_an_extra_payment() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    r.actors_at_mut(r.depth).push(Actor{id:802.,x:13.,y:10.,hp:100.,max_hp:100.,state:"sleeping".into(),..Default::default()});
    tick_timer(&mut r);assert!(turn(&mut r,ID,Some(player),false).unwrap());
    assert_eq!(a(&r).x,10.);assert!(!engine::flying_in(&r,a(&r)));assert_eq!(a(&r).energy,42.);
}
#[test]
fn foreign_levitation_survives_landing_and_cleansed_flight_does_not_reappear() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();
    engine::actor_mut_trusted(&mut r,ID).unwrap().conditions.insert("flying".into(),20.);
    land(&mut r,ID);assert_eq!(condition(a(&r),"flying"),20.);
    assert_eq!(condition(a(&r),FLIGHT),0.);assert!(!turn(&mut r,ID,Some(player),false).unwrap());
    crate::abilities::clear_status_conditions(engine::actor_mut_trusted(&mut r,ID).unwrap());
    assert!(!engine::flying_in(&r,a(&r)));assert_eq!(condition(a(&r),FLIGHT),0.);
}
#[test]
fn offscreen_airborne_dragon_lands_using_normal_actions_only() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    let depth=r.depth;let tick=r.tick;
    let f=r.floor_mut().unwrap();f.dirty=true;f.last_tick=tick;
    r.depth=2.;r.tick+=5.;crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    let a=r.actors_at(depth).iter().find(|a|a.id==ID).unwrap();
    assert!(!a.dead.value_or(false));assert_eq!(a.y,10.);assert!(a.x>=13.);
    assert_eq!(condition(a,FLIGHT),0.);assert_eq!(condition(a,"flying"),0.);assert_eq!(a.energy,42.);
}
#[test]
fn physical_activation_uses_silence_but_not_spell_scaling_and_can_recharge() {
    let mut r=room();let player=r.player.id;
    engine::actor_mut_trusted(&mut r,ID).unwrap().conditions.insert("magicSensitive".into(),20.);
    turn(&mut r,ID,Some(player),false).unwrap();assert_eq!(a(&r).energy,42.);land(&mut r,ID);
    let mut body=a(&r).clone();body.energy=2.;body.state="sleeping".into();body.last_seen=Field::Missing;
    for _ in 0..48 {crate::ai::advance_rest(&r,&mut body,false);}
    assert_eq!(body.energy,50.);
}
#[test]
fn offscreen_ally_finishes_paid_landing_before_entering_rest() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    let a=engine::actor_mut_trusted(&mut r,ID).unwrap();a.state="ally".into();a.last_seen=Field::Missing;
    let depth=r.depth;let tick=r.tick;let f=r.floor_mut().unwrap();f.dirty=true;f.last_tick=tick;
    r.depth=2.;r.tick+=1.;crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    let a=r.actors_at(depth).iter().find(|a|a.id==ID).unwrap();
    assert!(!a.dead.value_or(false));assert_eq!((a.x,a.y),(10.,10.));assert_eq!(condition(a,FLIGHT),0.);assert_eq!(a.energy,42.);
}
#[test]
fn dispatch_resting_ally_lands_before_sleep_return() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    r.player.x=10.;r.player.y=10.;r.next_id=900.;
    let body=engine::actor_mut_trusted(&mut r,ID).unwrap();body.state="ally".into();body.last_seen=Field::Missing;
    body.conditions.insert("monsterAllyRest".into(),1.);
    engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
    assert_eq!((a(&r).x,a(&r).y),(13.,10.));assert_eq!(condition(a(&r),FLIGHT),0.);
    assert!(!engine::flying_in(&r,a(&r)));assert_eq!(a(&r).energy,42.);
}
#[test]
fn already_paid_sticky_struggle_is_not_charged_again_in_route_comparison() {
    let mut r=room();let player=r.player.id;
    let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
    for x in 10..=14 {f.tiles[10*w+x]="floor".into();}
    let mud=f.mud.get_or_insert_with(||vec![0.;n]);mud[10*w+10]=1.;mud[10*w+11]=1.;
    engine::actor_mut_trusted(&mut r,ID).unwrap().conditions.insert("stuck".into(),1.);
    assert!(!turn(&mut r,ID,Some(player),false).unwrap());assert_eq!(a(&r).energy,50.);
}
// Execute each NPC full action separately by keeping the player genuinely
// Hasted. This makes bonus/normal starts observable without calling the AI
// kernel directly or treating two movement attempts as unconditional Fast.
fn next_actor_action(r:&mut Run,hasted:bool) {
    loop {
        let normal=r.phase.as_ref().is_some_and(|p|p=="normal");
        engine::dispatch(r,&crate::Command::Wait).unwrap();
        if normal || hasted {return;}
    }
}
fn route_fixture(fast:bool,hasted:bool,bonus:bool,two_sticky:bool)->Run {
    let mut r=room();r.tick=0.;r.next_id=900.;r.player.x=13.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    r.player.conditions.insert("hasted".into(),20.);
    r.phase=if bonus {"haste"}else{"normal"}.to_string().into();
    let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
    f.lights.fill(100.);for x in 10..=14 {f.tiles[10*w+x]="floor".into();}
    let mut mud=vec![0.;n];mud[10*w+10]=1.;if two_sticky {mud[10*w+11]=1.;}f.mud=mud.into();
    let player=r.player.id;let body=engine::actor_mut_trusted(&mut r,ID).unwrap();
    body.abilities.retain(|id|id!="enemy:beast:fire-breath" && (fast || id!="enemy:beast:fast"));
    body.conditions.insert("targetId".into(),player);body.last_seen=ActorLastSeen{x:13.,y:10.}.into();
    if hasted {body.conditions.insert("hasted".into(),20.);}
    r
}
fn dispatched_arrival(mut r:Run,hasted:bool,flight:bool)->(u32,u32) {
    // Ground comparison cannot elect flight later. The aerial counterfactual
    // uses a temporary ground-only barrier for its first action, so production
    // dispatch pays takeoff even when the original routes tie. Restore the same
    // terrain immediately after takeoff; no movement has happened yet.
    if !flight {engine::actor_mut_trusted(&mut r,ID).unwrap().energy=0.;}
    let original=r.floor().unwrap().tiles.clone();
    if flight {
        let body=a(&r).clone();let player=r.player.clone();let f=r.floor_mut().unwrap();let w=f.width();
        for (at,tile) in f.tiles.iter_mut().enumerate() {
            let x=(at%w)as f64;let y=(at/w)as f64;
            if tile=="floor" && (x!=body.x || y!=body.y) && (x-player.x).abs().max((y-player.y).abs())>1. {*tile="chasm".into();}
        }
    }
    for actions in 1..=8 {
        next_actor_action(&mut r,hasted);
        if flight && actions==1 {
            assert_eq!((a(&r).x,a(&r).y),(10.,10.));assert_eq!(a(&r).energy,42.);
            assert!(condition(a(&r),FLIGHT)>0.);r.floor_mut().unwrap().tiles=original.clone();
        }
        if crate::portals::actor_distance(&r,a(&r),&r.player)<=std::f64::consts::SQRT_2 && !engine::flying_in(&r,a(&r)) {
            let bonus=r.phase.as_ref().is_some_and(|p|p=="normal");
            return (actions,r.tick as u32+u32::from(bonus));
        }
    }
    panic!("route did not arrive: {:?}",(a(&r).x,a(&r).y));
}
#[test]
fn fast_haste_route_estimate_matches_dispatch_and_rejects_actual_turn_ties() {
    for fast in [false,true] {for hasted in [false,true] {for bonus in [false,true] {for two_sticky in [false,true] {
        let r=route_fixture(fast,hasted,bonus,two_sticky);
        let ground=dispatched_arrival(r.clone(),hasted,false);
        let air=dispatched_arrival(r.clone(),hasted,true);
        let ground_actions=if fast {if two_sticky {3}else{2}} else {if two_sticky {4}else{3}};
        let air_actions=if fast {2}else{3};
        assert_eq!(ground.0,ground_actions,"ground Fast={fast} Haste={hasted} bonus={bonus} mud={two_sticky}");
        assert_eq!(air.0,air_actions,"air Fast={fast} Haste={hasted} bonus={bonus} mud={two_sticky}");
        let used=u32::from(hasted && !bonus);let rate=if hasted {2}else{1};
        assert_eq!(ground.1,(ground_actions+used).div_ceil(rate));
        assert_eq!(air.1,(air_actions+used).div_ceil(rate));
        let mut planned=r;next_actor_action(&mut planned,hasted);
        let should_fly=air.1<ground.1;
        assert_eq!(condition(a(&planned),FLIGHT)>0.,should_fly,"Fast={fast} Haste={hasted} bonus={bonus} mud={two_sticky}: air={air:?} ground={ground:?}");
        assert_eq!(a(&planned).energy,if should_fly {42.}else{50.});
        // In a real tie the first sticky attempt stays put and earns no Fast
        // continuation; the planner must neither spend nor move twice.
        if !should_fly {assert_eq!((a(&planned).x,a(&planned).y),(10.,10.));assert_eq!(condition(a(&planned),"stuck"),1.);}
    }}}}
}
#[test]
fn offscreen_resting_fast_dragon_finishes_two_step_paid_landing() {
    let mut r=room();let player=r.player.id;turn(&mut r,ID,Some(player),false).unwrap();turn(&mut r,ID,Some(player),true).unwrap();
    let body=engine::actor_mut_trusted(&mut r,ID).unwrap();body.state="ally".into();body.last_seen=Field::Missing;
    body.conditions.insert("monsterAllyRest".into(),1.);
    // Prevent the one-step return so the already-paid landing needs both Fast
    // moves. The off-floor player proxy is not an occupancy or attack target.
    r.actors_at_mut(r.depth).push(Actor{id:802.,x:10.,y:10.,hp:100.,max_hp:100.,state:"sleeping".into(),..Default::default()});
    let depth=r.depth;let tick=r.tick;let f=r.floor_mut().unwrap();f.dirty=true;f.last_tick=tick;
    r.depth=2.;r.tick+=1.;crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    let body=r.actors_at(depth).iter().find(|a|a.id==ID).unwrap();
    assert_eq!((body.x,body.y),(13.,10.));assert!(!body.dead.value_or(false));
    assert_eq!(condition(body,FLIGHT),0.);assert_eq!(condition(body,"flying"),0.);assert_eq!(body.energy,42.);
}

fn flight_budget_fixture(moves:usize,fast:bool)->Run {
    let mut r=room();r.tick=0.;r.next_id=900.;r.phase="haste".to_string().into();
    let landing=10+moves;r.player.x=(landing+1)as f64;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("wall".into());f.lights.fill(100.);
    f.mud=Field::Missing;f.roots=Field::Missing;f.slime=Field::Missing;
    for x in 10..=landing+1 {f.tiles[10*w+x]=if x>10 && x<landing {"chasm"}else{"floor"}.into();}
    let player=r.player.id;let target_x=r.player.x;let body=engine::actor_mut_trusted(&mut r,ID).unwrap();
    body.abilities.retain(|id|id!="enemy:beast:fire-breath" && (fast || id!="enemy:beast:fast"));
    body.last_seen=ActorLastSeen{x:target_x,y:10.}.into();body.conditions.insert("targetId".into(),player);
    r
}
#[test]
fn dispatch_five_move_flight_does_not_pay_for_a_route_next_upkeep_abandons() {
    for fast in [false,true] {
        let mut r=flight_budget_fixture(5,fast);
        for tick in 1..=2 {
            engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
            assert_eq!(r.tick,tick as f64);
            assert_eq!((a(&r).x,a(&r).y),(10.,10.));
            assert_eq!(a(&r).energy,50.,"Fast={fast}: rejected route must not pay for takeoff");
            assert_eq!(condition(a(&r),FLIGHT),0.);assert!(!engine::flying_in(&r,a(&r)));
        }
    }
}
#[test]
fn dispatch_max_admitted_flight_lands_with_reserved_normal_turn() {
    for fast in [false,true] {
        let mut r=flight_budget_fixture(4,fast);
        engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
        assert_eq!((a(&r).x,a(&r).y),(10.,10.));assert_eq!(a(&r).energy,42.);
        assert_eq!(condition(a(&r),FLIGHT),6.);assert_eq!(condition(a(&r),"flying"),6.);
        let positions=if fast {vec![12.,14.]}else{vec![11.,12.,13.,14.]};
        for (index,x) in positions.into_iter().enumerate() {
            engine::dispatch(&mut r,&crate::Command::Wait).unwrap();
            assert_eq!((a(&r).x,a(&r).y),(x,10.));assert_eq!(a(&r).energy,42.);
            assert!(!a(&r).dead.value_or(false));
            if x<14. {
                assert_eq!(condition(a(&r),FLIGHT),5.-index as f64);
                assert_eq!(condition(a(&r),"flying"),5.-index as f64);
            }
        }
        assert_eq!(condition(a(&r),FLIGHT),0.);assert!(!engine::flying_in(&r,a(&r)));
        assert!(a(&r).hp>0.);assert_eq!(r.tick,if fast {3.}else{5.});
    }
}
