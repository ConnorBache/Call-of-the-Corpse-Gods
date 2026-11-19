//! Real scheduler checks for the detached movement graph and hazard timing.
use super::*;
use super::revision52_shallow_water_pathing_tests::{room,set_tile,corridor,W,N};
use corpse_core::{model::FastActionPhase, revision52_swimming};

fn phase(r: &mut Run, state: usize) {
    r.fast_action_phase = Field::Missing;
    r.phase = match state {0 => Field::Missing, 1 => "haste".to_string().into(), _ => "normal".to_string().into()};
    if state >= 3 { r.fast_action_phase = if state == 3 {FastActionPhase::Haste} else {FastActionPhase::Normal}.into(); }
}
fn powers(r: &mut Run, fast: bool, haste: bool, swimming: bool) {
    if fast {r.player.abilities.push("enemy:beast:fast".into());}
    if haste {r.player.conditions.insert("hasted".into(),20.);}
    if swimming {r.inventory.push(revision52_swimming::make_ring(900.));}
}
fn state(r: &Run) -> usize {
    ActionStage::observe(r) as usize + if c(&r.player,"cameFree") != 0. {4} else {0}
}
fn coords(r: &Run) -> usize {r.player.y as usize*W+r.player.x as usize}
fn knowledge(r: &Run) -> Knowledge {Knowledge::observe(r,&[true;N], |_|false).unwrap()}

#[test]
fn r52_action_stages_match_real_move_and_wait_from_every_saved_phase() {
    for input in 0..5 {for fast in [false,true] {for haste in [false,true] {for moving in [false,true] {
        let mut r=room(); phase(&mut r,input); powers(&mut r,fast,haste,false);
        let before=r.clone(); let initial=ActionStage::observe(&r);
        let (next,ticks)=initial.advance(haste,fast&&moving);
        assert_eq!(action_skips_world(&r,moving),!ticks); assert_eq!(r,before);
        let command=if moving {Command::Move{dx:1.,dy:0.}} else {Command::Wait};
        let outcome=engine::dispatch(&mut r,&command).unwrap();
        assert_eq!(outcome.took_turn,ticks,"input={input} fast={fast} haste={haste} move={moving}");
        assert_eq!(r.tick,before.tick+f64::from(u8::from(ticks)));
        assert_eq!(ActionStage::observe(&r),next);
        assert_eq!((r.player.x,r.player.y),(if moving {2.} else {1.},4.));
    }}}}
}

#[test]
fn r52_route_graph_samples_swimming_on_every_departure_including_fast_substages() {
    for input in 0..5 {for fast in [false,true] {for starts_wet in [false,true] {
        let mut r=room(); corridor(&mut r); phase(&mut r,input); powers(&mut r,fast,false,true);
        for x in 1..=7 {set_tile(&mut r,x,4,if if starts_wet {x%2==1} else {x==2||x==3||x==5} {"shallow-water"} else {"floor"});}
        let before=r.clone(); let k=knowledge(&r); assert_eq!(k.phases(),8);
        let path=k.path(7,4,false); assert_eq!(path.len(),6); assert_eq!(r,before);
        let mut projected=k.phase; let mut from=k.start;
        for step in path {
            let entry=step.y*W+step.x;
            let (arrival,next)=k.transition(from,entry,projected,4*W+7).unwrap();
            let (_,ticks)=ActionStage::from_index(projected&3).advance(k.hasted_at(from),k.fast);
            let tick=r.tick;
            let command=Command::Move{dx:step.x as f64-r.player.x,dy:step.y as f64-r.player.y};
            let outcome=engine::dispatch(&mut r,&command).unwrap();
            assert_eq!(coords(&r),arrival);
            assert_eq!(state(&r),next,"input={input} fast={fast} starts_wet={starts_wet} entry={entry}");
            assert_eq!(outcome.took_turn,ticks); assert_eq!(r.tick,tick+f64::from(u8::from(ticks)));
            projected=next; from=arrival;
        }
    }}}
}

#[test]
fn r52_swimming_phase_projection_uses_memory_not_hidden_water_or_ichor() {
    let mut r=room(); corridor(&mut r); powers(&mut r,true,false,true); phase(&mut r,1);
    for x in [2,3,5] {set_tile(&mut r,x,4,"shallow-water");}
    let remembered=Knowledge::observe(&r,&[false;N], |_|false).unwrap();
    let expected:Vec<_>=(1..7).flat_map(|x|(0..8).map(move |s|(x,s)))
        .map(|(x,s)|remembered.transition(4*W+x,4*W+x+1,s,4*W+7)).collect();
    for hidden in ["water","floor","shallow-water"] {
        let f=r.floor_mut().unwrap(); for x in 1..=7 {f.tiles[4*W+x]=hidden.into();}
        f.ichor=vec![1.;N].into(); f.currents=vec![3.;N].into();
        let before=r.clone(); let k=Knowledge::observe(&r,&[false;N], |_|false).unwrap();
        let actual:Vec<_>=(1..7).flat_map(|x|(0..8).map(move |s|(x,s)))
            .map(|(x,s)|k.transition(4*W+x,4*W+x+1,s,4*W+7)).collect();
        assert_eq!(actual,expected); assert_eq!(r,before);
    }
    // No ring bonus is inferred from an unknown live shallow square.
    r.floor_mut().unwrap().seen[4*W+3]=false;
    let k=Knowledge::observe(&r,&[false;N], |_|false).unwrap();
    assert!(!k.hasted_at(4*W+3)); assert!(k.path(7,4,false).is_empty());
}

#[test]
fn r52_auto_travel_and_autoexplore_replan_across_fast_and_swimming_stages() {
    for exploring in [false,true] {
        let mut r=room(); corridor(&mut r); phase(&mut r,1); powers(&mut r,true,false,true);
        for x in [2,3,5] {set_tile(&mut r,x,4,"shallow-water");}
        let target=if exploring {5} else {7};
        let mut walk=if exploring {
            r.player.conditions.clear();r.player.stats.perception=0.;r.floor_mut().unwrap().lights.fill(-100.);
            for x in [6,7] {r.floor_mut().unwrap().seen[4*W+x]=false;}
            let mut e=AutoExplore::default();assert!(e.press(&r).is_none());e.press(&r).unwrap()
        } else {let Start::Walk(walk)=start(&r,target,4) else {panic!("known route")};walk};
        let mut moves=0;
        loop {
            let Some(command)=walk.next_command(&r).unwrap() else {break;};
            let mut direct=r.clone();engine::dispatch(&mut direct,&command).unwrap();
            engine::dispatch(&mut r,&command).unwrap(); assert_eq!(r,direct);
            moves+=1;assert!(moves<=6);if !walk.after_step(&r,false).unwrap(){break;}
        }
        assert_eq!(r.player.x,target as f64);assert_eq!(moves,target-1);assert!(walk.path().is_empty());
    }
}

fn current_scene(input:usize,fast:bool,haste:bool,swimming:bool,moving:bool,portal:bool)->Run {
    let mut r=room();r.player.x=2.;phase(&mut r,input);powers(&mut r,fast,haste,swimming);
    for x in 2..=7 {set_tile(&mut r,x,4,"water");}
    let f=r.floor_mut().unwrap();f.currents=vec![0.;N].into();
    let crossing=if portal {5} else if moving {3} else {2};
    f.currents.as_mut().unwrap()[4*W+crossing]=3.;
    f.permanent_fire=vec![(4*W+crossing+1) as f64].into();
    if portal {f.portals=vec![corpse_core::portals_generation::PortalPair{a:4*W+3,b:4*W+5,color_index:0}].into();}
    r
}

#[test]
fn r52_manual_move_wait_and_portal_current_warnings_match_real_action_timing() {
    for input in 0..5 {for fast in [false,true] {for haste in [false,true] {for swimming in [false,true] {
        for (moving,portal) in [(true,false),(false,false),(true,true)] {
            let mut r=current_scene(input,fast,haste,swimming,moving,portal);
            let command=if moving {Command::Move{dx:1.,dy:0.}} else {Command::Wait};
            let before=r.clone();let skips=action_skips_world(&r,moving);
            let warning=if moving {crate::hazards_ui::intentional(&r,&command)} else {crate::hazards_ui::waiting(&r)};
            assert_eq!(warning.is_some(),!skips,"input={input} fast={fast} haste={haste} swimming={swimming} move={moving} portal={portal}: {warning:?}");
            assert_eq!(r,before);
            let outcome=engine::dispatch(&mut r,&command).unwrap();
            let crossed=if portal {5.} else if moving {3.} else {2.};
            assert_eq!(r.player.x,crossed+if skips {0.} else {1.});
            assert_eq!(outcome.took_turn,!skips);assert_eq!(r.tick,before.tick+if skips {0.} else {1.});
            // Manual input can enter deep water; no automatic route is invented.
            assert!(path_to(&before,3,4,true).is_empty());
        }
    }}}}
}

#[test]
fn r52_saved_fast_stage_survives_lost_fast_haste_and_slowing() {
    for pending in [3,4] {for slowed in [false,true] {for moving in [false,true] {
        let mut r=room();phase(&mut r,pending);
        if slowed {r.player.conditions.insert("slowed".into(),5.);}
        let before=r.clone();let k=knowledge(&r);
        let command=if moving {Command::Move{dx:1.,dy:0.}} else {Command::Wait};
        let skips=action_skips_world(&r,moving);assert_eq!(skips,pending==3);
        let projected=if moving {Some(k.transition(k.start,k.start+1,k.phase,4*W+7).unwrap())} else {None};
        let outcome=engine::dispatch(&mut r,&command).unwrap();
        assert_eq!(outcome.took_turn,!skips);
        assert_eq!(r.tick,before.tick+if skips {0.} else if slowed {2.} else {1.});
        if let Some((arrival,next))=projected {assert_eq!(coords(&r),arrival);assert_eq!(state(&r),next);}
    }}}
}

#[test]
fn r52_fast_and_haste_keep_the_independent_came_free_bit() {
    for fast in [false,true] {for haste in [false,true] {for input in 0..5 {
        let mut r=room();phase(&mut r,input);powers(&mut r,fast,haste,false);
        r.player.conditions.insert("cameTurns".into(),20.);r.player.conditions.insert("cameTarget".into(),999.);
        let mut quarry=r.player.clone();quarry.id=999.;quarry.x=7.;quarry.y=4.;quarry.state="sleeping".into();
        quarry.conditions.clear();quarry.abilities=vec!["enemy:beast:rooted".into()];quarry.active.clear();
        r.actors_at_mut(r.depth).push(quarry);
        let k=Knowledge::observe(&r,&[true;N], |_|true).unwrap();let mut from=k.start;let mut projected=k.phase;
        for _ in 0..4 {
            let (arrival,next)=k.transition(from,from+1,projected,4*W+5).unwrap();
            engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
            assert_eq!(coords(&r),arrival);assert_eq!(state(&r),next,"fast={fast} haste={haste} input={input}");
            from=arrival;projected=next;
        }
    }}}
}

#[test]
fn r52_bonus_wait_suppresses_tick_exposure_without_suppressing_move_contact_or_legacy_wait() {
    let mut r=room();phase(&mut r,1);powers(&mut r,false,true,false);
    r.floor_mut().unwrap().permanent_fire=vec![(4*W+1) as f64,(4*W+2) as f64].into();
    let before=r.clone();assert!(crate::hazards_ui::waiting(&r).is_none());
    assert!(crate::hazards_ui::intentional(&r,&Command::Move{dx:1.,dy:0.}).unwrap().contains("fire"));assert_eq!(r,before);
    r.simulation_revision=51.0.into();assert!(crate::hazards_ui::waiting(&r).unwrap().contains("fire"));
    let k=knowledge(&r);assert_eq!(k.phases(),4);
}

fn yeti(r:&mut Run,x:f64,y:f64)->f64 {
    let mut a=r.player.clone();a.id=999.;a.x=x;a.y=y;a.state="sleeping".into();
    a.abilities=vec!["enemy:beast:yeti".into(),"enemy:beast:rooted".into()];
    a.active.clear();a.conditions.clear();a.stats.offense=0.;a.stats.accuracy=0.;
    r.actors_at_mut(r.depth).push(a);999.
}
fn observed_knowledge(r:&Run)->Knowledge {
    Knowledge::observe(r,&vision::terrain_visibility(r),|a|vision::visible_actor(r,a)).unwrap()
}

#[test]
fn r52_known_pre_action_chill_matches_move_and_wait_without_cancelling_pending_actions() {
    for input in 0..5 {for moving in [false,true] {
        let mut r=room();phase(&mut r,input);powers(&mut r,true,true,false);yeti(&mut r,1.,3.);
        r.floor_mut().unwrap().permanent_fire=vec![(4*W+1)as f64].into();
        let k=observed_knowledge(&r);assert!(k.chill[k.start]);
        let ticks=action_world_ticks(&r,moving);
        let expected=if input<=1 {2} else if input==3 || (input==2&&moving) {0} else {1};
        assert_eq!(ticks,expected,"input={input} moving={moving}");
        if !moving {assert_eq!(crate::hazards_ui::waiting(&r).is_some(),ticks>0);}
        let projected=if moving {Some(k.transition(k.start,k.start+1,k.phase,4*W+7).unwrap())} else {None};
        let command=if moving {Command::Move{dx:1.,dy:0.}} else {Command::Wait};let tick=r.tick;
        engine::dispatch(&mut r,&command).unwrap();assert_eq!(r.tick,tick+ticks as f64);
        if let Some((arrival,next))=projected {assert_eq!(coords(&r),arrival);assert_eq!(state(&r),next);}
    }}
}

#[test]
fn r52_known_chill_warns_for_the_second_current_step_only_when_it_is_observed() {
    for moving in [false,true] {for speed in [false,true] {
        let mut r=current_scene(1,speed,speed,false,moving,false);yeti(&mut r,2.,3.);
        let crossed=if moving {3} else {2};let final_x=crossed+2;
        let f=r.floor_mut().unwrap();f.currents.as_mut().unwrap()[4*W+crossed+1]=3.;
        f.permanent_fire=vec![(4*W+final_x)as f64].into();
        let command=if moving {Command::Move{dx:1.,dy:0.}} else {Command::Wait};
        let warning=if moving {crate::hazards_ui::intentional(&r,&command)} else {crate::hazards_ui::waiting(&r)};
        assert!(warning.unwrap().contains("Be swept by the current"));
        assert_eq!(action_world_ticks(&r,moving),2);
        let tick=r.tick;engine::dispatch(&mut r,&command).unwrap();
        assert_eq!(r.tick,tick+2.);assert_eq!(r.player.x,final_x as f64);
    }}
}

#[test]
fn r52_chill_projection_respects_hidden_sources_ice_immunity_planes_and_owned_endpoints() {
    for variant in ["hidden","oblivious-hidden","ice","other-plane","half-here-other-plane"] {
        let mut r=room();phase(&mut r,1);powers(&mut r,true,true,false);
        let id=yeti(&mut r,1.,3.);
        match variant {
            "hidden"|"oblivious-hidden"=>{
                r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
                engine::actor_mut(&mut r,id).unwrap().conditions.insert("invisible".into(),5.);
                if variant=="oblivious-hidden" {r.player.abilities.push(corpse_core::revision50_combat::OBLIVIOUS.into());}
                assert!(!vision::visible_actor(&r,engine::actor(&r,id).unwrap()));
            }
            "ice"=>{let mut ring=revision52_swimming::make_ring(900.);ring.effect=corpse_core::revision50_rings::ICE_IMMUNITY.to_string().into();r.inventory.push(ring);}
            "other-plane"|"half-here-other-plane"=>{
                engine::actor_mut(&mut r,id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
                r.dreamwalking=corpse_core::dreamwalking::State{depth:r.depth,floors:r.floors.clone(),..Default::default()}.into();
                if variant=="half-here-other-plane" {
                    r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());
                    r.player.conditions.insert("halfHereId".into(),998.);
                    let mut clone=r.player.clone();clone.id=998.;clone.x=6.;clone.y=4.;clone.conditions.clear();
                    clone.conditions.insert("halfHereClone".into(),1.);clone.conditions.insert("halfHereOwner".into(),r.player.id);
                    clone.conditions.insert("phantasmiiPlane".into(),1.);r.actors_at_mut(r.depth).push(clone);
                    let a=engine::actor_mut(&mut r,id).unwrap();a.x=7.;a.y=4.;
                }
            }
            _=>unreachable!(),
        }
        let before=r.clone();let k=observed_knowledge(&r);let expected=variant=="half-here-other-plane";
        assert_eq!(k.chill[k.start],expected,"{variant}");assert_eq!(action_world_ticks(&r,false),if expected {2}else{0},"{variant}");
        assert_eq!(r,before);
        if variant=="hidden" {
            // Private contact effects may differ in reality; previews must not expose them.
            r.actors_at_mut(r.depth).clear();assert_eq!(action_world_ticks(&r,false),0);
        } else {
            let tick=r.tick;engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.tick,tick+if expected {2.}else{0.},"{variant}");
        }
    }
}

#[test]
fn r52_chill_distance_uses_known_transport_links_but_not_a_hidden_mate() {
    let mut r=room();phase(&mut r,1);powers(&mut r,false,true,false);yeti(&mut r,7.,4.);
    r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:4*W+1,b:4*W+6,color_index:0}].into();
    let k=observed_knowledge(&r);assert!(k.chill[k.start]);assert_eq!(action_world_ticks(&r,false),2);
    let mut actual=r.clone();engine::dispatch(&mut actual,&Command::Wait).unwrap();assert_eq!(actual.tick,r.tick+2.);
    r.floor_mut().unwrap().seen[4*W+6]=false;
    let k=Knowledge::observe(&r,&[false;N], |_|true).unwrap();assert!(!k.chill[k.start]);
}

#[test]
fn r52_stranded_wait_warns_and_ticks_before_any_haste_or_fast_continuation() {
    for input in 0..5 {
        let mut r=room();phase(&mut r,input);powers(&mut r,true,true,false);
        r.player.abilities.push("enemy:beast:ichor-bound".into());
        r.floor_mut().unwrap().permanent_fire=vec![(4*W+1)as f64].into();
        let before=r.clone();assert!(corpse_core::ichor::stranded(&r,&r.player));
        assert_eq!(action_world_ticks(&r,false),1);assert!(crate::hazards_ui::waiting(&r).unwrap().contains("fire"));
        assert!(path_to(&r,7,4,false).is_empty());assert_eq!(r,before);
        engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.tick,before.tick+1.);
    }
}

#[test]
fn r52_projected_identity_hides_raw_chill_from_routes_and_warnings_in_both_planes() {
    for half_here in [false,true] {for source_dream in [false,true] {
        let mut r=room();phase(&mut r,1);powers(&mut r,true,true,false);let id=yeti(&mut r,1.,3.);
        if source_dream {engine::actor_mut(&mut r,id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);}
        if half_here {
            r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());
            r.player.conditions.insert("halfHereId".into(),998.);
            let mut clone=r.player.clone();clone.id=998.;clone.x=2.;clone.y=4.;clone.conditions.clear();
            for (key,value) in [("halfHereClone",1.),("halfHereOwner",r.player.id),("phantasmiiPlane",1.),("hallucinating",5.)] {clone.conditions.insert(key.into(),value);}
            r.actors_at_mut(r.depth).push(clone);
            r.dreamwalking=corpse_core::dreamwalking::State{depth:r.depth,floors:r.floors.clone(),..Default::default()}.into();
        } else {r.player.conditions.insert("hallucinating".into(),5.);}
        r.floor_mut().unwrap().permanent_fire=vec![(4*W+1)as f64].into();
        let shown=crate::creature_presentation::project(&r,engine::actor(&r,id).unwrap()).expect("identity must be projected");
        let before=r.clone();let k=observed_knowledge(&r);assert!(!k.chill[k.start]);
        let warning=crate::hazards_ui::waiting(&r);assert_eq!(action_world_ticks(&r,false),0);assert!(warning.is_none());
        let route=k.path(7,4,false);assert_eq!(r,before);
        engine::actor_mut(&mut r,id).unwrap().abilities.retain(|s|s!="enemy:beast:yeti");
        let apparent=crate::creature_presentation::project(&r,engine::actor(&r,id).unwrap()).unwrap();
        assert_eq!((shown.name,shown.abilities),(apparent.name,apparent.abilities));
        assert_eq!(observed_knowledge(&r).path(7,4,false),route);
        assert_eq!(crate::hazards_ui::waiting(&r),warning);assert_eq!(action_world_ticks(&r,false),0);
    }}
}

#[test]
fn steam_warns_when_visible_and_deliberate_not_for_forced_moves_or_embers() {
    let mut r=room();
    let from=4*W+1; let to=4*W+2;
    let command=Command::Move{dx:1.,dy:0.};
    corpse_core::environment::add_gas(r.floor_mut().unwrap(),to,"steam",30.,-1.).unwrap();
    assert!(crate::hazards_ui::intentional(&r,&command).unwrap().contains("steam"));
    corpse_core::environment::add_gas(r.floor_mut().unwrap(),from,"caustic",30.,-1.).unwrap();
    let before=r.clone();
    let prompt=crate::hazards_ui::intentional(&r,&command).unwrap();
    assert!(prompt.contains("Step in steam?") && prompt.contains("scald"));
    let Start::Direct(clicked) = start(&r,2,4) else { panic!("adjacent steam click was not a direct move"); };
    assert_eq!(clicked,command);
    assert_eq!(crate::hazards_ui::intentional(&r,&clicked).as_deref(),Some(prompt.as_str()));
    assert!(observed_knowledge(&r).hazards[to]);
    assert_eq!(r,before);

    corpse_core::environment::add_gas(r.floor_mut().unwrap(),from,"steam",30.,-1.).unwrap();
    assert!(crate::hazards_ui::intentional(&r,&command).is_none());
    assert!(crate::hazards_ui::waiting(&r).unwrap().contains("steam"));
    r.player.x=3.;
    assert!(crate::hazards_ui::intentional(&r,&Command::Move {dx:-1.,dy:0.}).unwrap().contains("steam"));
    r.player.x=1.;
    corpse_core::environment::add_gas(r.floor_mut().unwrap(),from,"caustic",30.,-1.).unwrap();

    r.player.conditions.insert("forcedMove".into(),1.);
    assert!(crate::hazards_ui::intentional(&r,&command).is_none());
    r.player.conditions.shift_remove("forcedMove");
    r.steam_rules_from=Field::Null;
    assert!(crate::hazards_ui::intentional(&r,&command).is_none());
    r.steam_rules_from=0usize.into();
    r.player.abilities.push("enemy:beast:fire-immunity".into());
    assert!(crate::hazards_ui::intentional(&r,&command).is_none());
    r.player.abilities.clear();

    let mut dark=r.clone();
    dark.player.conditions.shift_remove("allSeeingReveal");dark.player.conditions.shift_remove("allSeeingDepth");
    dark.player.stats.perception=0.;
    dark.floor_mut().unwrap().lights[to]=-100.;
    assert!(!vision::visible(&dark,2,4));
    assert!(crate::hazards_ui::intentional(&dark,&command).is_none());
    assert!(!observed_knowledge(&dark).hazards[to]);

    let e=corpse_core::environment::initialize(r.floor_mut().unwrap()).unwrap();
    e.gas_type.as_mut().unwrap()[to]=None;
    e.gas_volume.as_mut().unwrap()[to]=0.;
    e.fire_kind.as_mut().unwrap()[to]=Some("embers".into());
    assert!(crate::hazards_ui::intentional(&r,&command).is_none());
    assert!(!observed_knowledge(&r).hazards[to]);
}
