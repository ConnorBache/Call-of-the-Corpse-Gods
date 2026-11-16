//! Narrow Firebolt gameplay/replay contract; authored damage, range and cost stay intact.
use corpse_core::{Actor, Command, Field, Run, Stats, engine, firebolt_travel as travel,
    revision50_amulet_effects as effects, replay::{ReplayRecord,ReplaySession},
    temporal::{self,Engine,History,Snapshot}};
fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();r.player.energy=1000.;
    r.player.stats=Stats{perception:300.,max_energy:1000.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.inventory.clear();for a in r.actors.values_mut(){*a=Default::default();}
    for g in r.ground.values_mut(){*g=Default::default();}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.next_id=900.;r.phase="normal".to_string().into();r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.remembered_tiles=vec!["floor".into();f.tiles.len()].into();f.doors=Field::Missing;
    f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
    f.environment=Field::Missing;f.vents=Field::Missing;f.hellfire=Field::Missing;f.grates=Field::Missing;
    f.revision50_terrain=Field::Missing;f.revision50_temporary_terrain=Field::Missing;
    f.slime=Field::Missing;f.roots=Field::Missing;f.razorleaves=Field::Missing;f.ichor=Field::Missing;
    let mut item=effects::make_amulet_for_revision(effects::Effect::Firebolt,1.,52.).unwrap();
    item.equipped="amulet".to_string().into();r.inventory.push(item);r
}
fn enemy(r:&mut Run,id:f64,x:f64,y:f64) {
    let mut a=Actor{id,x,y,name:"Giant Rat".into(),glyph:"e".into(),color:"#fff".into(),
        hp:500.,max_hp:500.,base_max_hp:500.0.into(),energy:100.,state:"hunting".into(),
        stats:Stats{max_energy:100.,..Default::default()},..Default::default()};
    a.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(a);
}
fn cast(r:&mut Run,x:f64,y:f64) {assert_eq!(effects::command(r,&Command::UseAt{id:1.,x,y}).unwrap(),Some(true));}
fn hp(r:&Run,id:f64)->f64 {engine::actor(r,id).unwrap().hp}
fn tile(r:&mut Run,x:usize,y:usize,kind:&str) {let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[y*w+x]=kind.into();}
fn flame(r:&Run,x:usize,y:usize)->f64 {let f=r.floor().unwrap();f.environment.as_ref().unwrap().fire[y*f.width()+x]}
#[test]
fn new_firebolt_burns_open_trees_before_blocking_and_preserves_damage_cost_and_range() {
    let mut r=room();enemy(&mut r,20.,32.,10.);enemy(&mut r,21.,34.,10.);
    tile(&mut r,13,10,"tree");tile(&mut r,24,10,"tree");tile(&mut r,30,10,"grass");
    cast(&mut r,11.,10.);
    assert_eq!((hp(&r,20.),hp(&r,21.)),(420.,500.));
    assert_eq!(r.player.energy,990.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    for x in [13,24,30] {assert!(flame(&r,x,10)>0.);}
    assert_eq!(flame(&r,12,10),0.);assert!(engine::condition(engine::actor(&r,20.).unwrap(),"burning")>0.);
}
#[test]
fn closed_door_burns_open_but_a_remaining_wall_still_stops_firebolt() {
    for wall in [false,true] {
        let mut r=room();enemy(&mut r,20.,16.,10.);let at=10*r.floor().unwrap().width()+13;
        r.floor_mut().unwrap().doors=serde_json::from_value(serde_json::json!({at.to_string():{"open":false}})).unwrap();
        if wall {tile(&mut r,13,10,"wall");}
        cast(&mut r,11.,10.);
        assert!(!r.floor().unwrap().doors.as_ref().unwrap().contains_key(&at.to_string()));
        assert_eq!(hp(&r,20.),if wall{500.}else{420.});
    }
}
#[test]
fn underwater_creature_is_skipped_but_first_ordinary_creature_stops_even_when_killed_or_immune() {
    for kind in ["ordinary","killed","immune"] {
        let mut r=room();enemy(&mut r,20.,12.,10.);enemy(&mut r,21.,14.,10.);enemy(&mut r,22.,16.,10.);
        tile(&mut r,12,10,"water");engine::actor_mut(&mut r,20.).unwrap().conditions.insert("underwater".into(),1.);
        let a=engine::actor_mut(&mut r,21.).unwrap();
        if kind=="killed" {a.hp=40.;}
        if kind=="immune" {a.abilities.push("enemy:beast:fire-immunity".into());}
        cast(&mut r,11.,10.);
        assert_eq!(hp(&r,20.),500.);assert_eq!(hp(&r,22.),500.);
        assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"burning"),0.);
        if kind=="killed" {assert!(engine::actor(&r,21.).is_none_or(|a|a.dead.value_or(false)||a.hp<=0.));}
        else {assert_eq!(hp(&r,21.),if kind=="immune"{500.}else{420.});}
    }
}
#[test]
fn blocked_wall_ignites_its_fuel_but_never_cells_beyond() {
    let mut r=room();enemy(&mut r,20.,16.,10.);tile(&mut r,13,10,"wall");tile(&mut r,14,10,"grass");
    let n=r.floor().unwrap().tiles.len();let at=10*r.floor().unwrap().width()+13;
    let mut slime=vec![0.;n];slime[at]=1.;r.floor_mut().unwrap().slime=slime.into();
    cast(&mut r,11.,10.);assert_eq!(hp(&r,20.),500.);assert!(flame(&r,13,10)>0.);
    assert_eq!(flame(&r,14,10),0.);assert_eq!(r.floor().unwrap().tiles[at],"wall");
}
#[test]
fn legacy_r52_and_r50_r51_still_stop_at_burned_tree_and_hit_underwater() {
    for revision in [50.,51.,52.] {
        let mut r=room();r.simulation_revision=revision.into();r.firebolt_travel_from=if revision==52.{Field::Null}else{Field::Missing};
        // Keep each revision's own reserve/profile contract.
        let mut item=effects::make_amulet_for_revision(effects::Effect::Firebolt,1.,revision).unwrap();item.equipped="amulet".to_string().into();r.inventory=vec![item];
        enemy(&mut r,20.,16.,10.);tile(&mut r,13,10,"tree");cast(&mut r,11.,10.);
        assert_eq!(hp(&r,20.),500.);assert!(flame(&r,13,10)>0.);
        let a=engine::actor_mut(&mut r,20.).unwrap();a.conditions.insert("underwater".into(),1.);
        cast(&mut r,11.,10.);assert_eq!(hp(&r,20.),420.);
    }
}
fn replay_room()->Run {let mut r=room();enemy(&mut r,20.,18.,10.);tile(&mut r,13,10,"tree");r}
fn shot()->Command {Command::UseAt{id:1.,x:11.,y:10.}}
fn compare(initial:Run,expected:&Engine,record:ReplayRecord) {
    let mut replay=ReplaySession::from_initial(initial,record).unwrap();
    for restart in [false,true] {
        if restart {replay.restart().unwrap();}
        while replay.step().unwrap(){}
        assert_eq!(replay.engine.save().unwrap(),expected.save().unwrap());
    }
}
#[test]
fn old_r52_replay_preserves_firebolt_results_rng_events_and_restart() {
    let mut initial=replay_room();initial.firebolt_travel_from=Field::Null;
    let mut expected=Engine::new(initial.clone()).unwrap();let out=expected.dispatch(&shot()).unwrap();
    let record=ReplayRecord::try_from_run(&expected.run).unwrap();assert!(record.firebolt_travel_from.is_missing());
    let mut replay=ReplaySession::from_initial(initial.clone(),record.clone()).unwrap();
    let actual=replay.step_with_outcome().unwrap().unwrap();assert_eq!(actual.events,out.events);assert_eq!(actual.took_turn,out.took_turn);
    assert_eq!(replay.engine.run.rng,expected.run.rng);assert_eq!(hp(&replay.engine.run,20.),hp(&expected.run,20.));
    compare(initial,&expected,record);
}
#[test]
fn missing_checkpoint_adopts_at_exact_prefix_boundary_and_mixed_replay_restarts() {
    for tail in [false,true] {
        let mut initial=replay_room();initial.firebolt_travel_from=Field::Null;tile(&mut initial,15,10,"tree");
        let mut old=Engine::new(initial.clone()).unwrap();old.dispatch(&shot()).unwrap();assert_eq!(hp(&old.run,20.),500.);
        let mut value:serde_json::Value=serde_json::from_str(&old.save().unwrap()).unwrap();
        value.as_object_mut().unwrap().remove("fireboltTravelFrom");
        let mut current=Engine::load(&value.to_string()).unwrap();
        assert_eq!(current.run.firebolt_travel_from,Field::Value(1));
        assert_eq!(current.run.rng,old.run.rng);
        if tail {current.dispatch(&shot()).unwrap();assert!(hp(&current.run,20.)<500.);}
        compare(initial,&current,ReplayRecord::try_from_run(&current.run).unwrap());
    }
}
#[test]
fn new_zero_boundary_and_empty_legacy_replays_remain_distinct() {
    for legacy in [false,true] {
        let mut initial=replay_room();if legacy {initial.firebolt_travel_from=Field::Null;}
        let e=Engine::new(initial.clone()).unwrap();let record=ReplayRecord::try_from_run(&e.run).unwrap();
        assert_eq!(record.firebolt_travel_from,if legacy {Field::Missing}else{Field::Value(0)});
        let mut replay=ReplaySession::from_initial(initial,record).unwrap();assert!(replay.finished());
        assert_eq!(travel::current(&replay.engine.run),!legacy);replay.restart().unwrap();assert_eq!(travel::current(&replay.engine.run),!legacy);
    }
}
#[test]
fn snapshot_future_and_import_strip_metadata_but_restore_live_policy() {
    let mut r=replay_room();r.firebolt_travel_from=Field::Null;
    let ability=corpse_core::revision50_ability_energy::FUTURE_ID;
    r.player.abilities.push(ability.into());
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,ability,10.,0.).unwrap();
    let snapshot=Snapshot::capture(&r);assert!(snapshot.state.firebolt_travel_from.is_missing());
    assert!(snapshot.value().unwrap().get("fireboltTravelFrom").is_none());
    let mut value=snapshot.value().unwrap();value["fireboltTravelFrom"]=serde_json::json!(100);
    assert!(Snapshot::from_value(value).unwrap().state.firebolt_travel_from.is_missing());
    temporal::remember_future(&mut r,&History::default()).unwrap();
    r.replay.push(Command::Wait);r.firebolt_travel_from=Field::Missing;travel::admit_current(&mut r).unwrap();
    let boundary=r.firebolt_travel_from.clone();snapshot.restore(&mut r).unwrap();assert_eq!(r.firebolt_travel_from,boundary);
    // Captured anchors omit policy, while return retains the current ledger policy.
    temporal::remember_future(&mut r,&History::default()).unwrap();
    assert!(r.future.as_ref().unwrap().native_state().unwrap().firebolt_travel_from.is_missing());
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());assert_eq!(r.firebolt_travel_from,boundary);
}
#[test]
fn invalid_boundary_rejects_before_commands_or_mutation() {
    let initial=replay_room();let e=Engine::new(initial.clone()).unwrap();
    for marker in [Field::Null,Field::Value(1)] {
        let mut record=ReplayRecord::from_run(&e.run);record.firebolt_travel_from=marker;
        assert!(ReplaySession::from_initial(initial.clone(),record).is_err());
    }
    let mut invalid=initial;invalid.firebolt_travel_from=1usize.into();assert!(Engine::new(invalid).is_err());
}
#[test]
fn firebolt_preview_uses_only_current_knowledge_and_never_mutates_or_draws_rng() {
    let mut r=room();r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    tile(&mut r,13,10,"tree");enemy(&mut r,20.,14.,10.);enemy(&mut r,21.,18.,10.);
    engine::actor_mut(&mut r,20.).unwrap().conditions.insert("underwater".into(),1.);
    let before=r.clone();let preview=effects::aim_preview(&r,&r.inventory[0],11.,10.);
    assert!(preview.delivery.contains(&(13,10)));assert_eq!(preview.impact,Some((18,10)));assert_eq!(r,before);
    r.firebolt_travel_from=Field::Null;assert_eq!(effects::aim_preview(&r,&r.inventory[0],11.,10.).impact,Some((13,10)));
    r.firebolt_travel_from=Field::Missing;r.player.conditions.clear();r.player.stats.perception=-1000.;
    r.floor_mut().unwrap().lights.fill(-100.);r.floor_mut().unwrap().light_sources=Field::Missing;
    let before=effects::aim_preview(&r,&r.inventory[0],11.,10.);assert!(!before.delivery.contains(&(13,10)));
    tile(&mut r,13,10,"floor");engine::actor_mut(&mut r,20.).unwrap().conditions.clear();
    assert_eq!(effects::aim_preview(&r,&r.inventory[0],11.,10.),before);
}
#[test]
fn portal_route_burns_actual_entry_and_exit_cells_without_losing_submerged_skip() {
    let mut r=room();let w=r.floor().unwrap().width();
    r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
    enemy(&mut r,20.,30.,10.);engine::actor_mut(&mut r,20.).unwrap().conditions.insert("underwater".into(),1.);
    enemy(&mut r,21.,34.,10.);tile(&mut r,12,10,"grass");tile(&mut r,30,10,"grass");tile(&mut r,31,10,"tree");
    cast(&mut r,11.,10.);
    assert_eq!((hp(&r,20.),hp(&r,21.)),(500.,420.));
    for x in [12,30,31] {assert!(flame(&r,x,10)>0.);}
}
#[test]
fn unentered_portal_exit_is_not_burned_and_rejected_exit_consumes_no_random_draw() {
    let mut r=room();let w=r.floor().unwrap().width();
    r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:10*w+12,b:20*w+30,color_index:0}].into();
    tile(&mut r,12,10,"grass");tile(&mut r,30,20,"tree");let rng=r.rng;
    cast(&mut r,11.,10.);
    assert!(flame(&r,12,10)>0.);assert_eq!(flame(&r,30,20),0.);assert_eq!(r.rng,rng);
}
#[test]
fn visible_snow_and_known_freezing_prevent_preview_from_promising_burn_through() {
    for frozen in [false,true] {
        let mut r=room();r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        tile(&mut r,13,10,"tree");enemy(&mut r,20.,18.,10.);
        if frozen {r.player.active.push("dawel-reverse-entropy".into());}
        else {let f=r.floor_mut().unwrap();let mut snow=vec![false;f.tiles.len()];snow[10*f.width()+13]=true;f.snow=snow.into();}
        let preview=effects::aim_preview(&r,&r.inventory[0],11.,10.);assert_eq!(preview.impact,Some((13,10)));
        cast(&mut r,11.,10.);assert_eq!(hp(&r,20.),500.);
    }
}

/// A supported Half Here endpoint inherits Phase Through. Move the player
/// through the wall and swap with that endpoint, leaving it inside the wall.
/// No ordinary NPC receives unsupported player-only active abilities.
fn phase_actor_in_wall()->Run {
    let mut r=room();
    r.player.abilities=vec!["etill-phase-through".into(),corpse_core::half_here::ID.into()];
    r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
    corpse_core::abilities::activate(&mut r,"etill-phase-through",&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
    assert!(r.player.active.iter().any(|s|s=="etill-phase-through"));
    r.next_id=20.;
    corpse_core::abilities::activate(&mut r,corpse_core::half_here::ID,&Field::Missing,&12.0.into(),&10.0.into()).unwrap();
    assert_eq!(corpse_core::half_here::clone_id(&r),Some(20.));
    r.next_id=900.;enemy(&mut r,21.,16.,10.);tile(&mut r,13,10,"wall");
    assert!(engine::actor_passable(&r,engine::actor(&r,20.).unwrap(),13.,10.));
    let player=r.player.id;
    for (dx,dy) in [(0.,-1.),(1.,0.),(1.,0.),(1.,0.),(0.,1.),(-1.,0.),(-1.,0.),(-1.,0.)] {
        assert!(engine::move_actor(&mut r,player,dx,dy,true).unwrap());
    }
    assert_eq!((r.player.x,r.player.y),(10.,10.));
    let a=engine::actor(&r,20.).unwrap();assert_eq!((a.x,a.y),(13.,10.));
    assert!(corpse_core::half_here::is_clone(a));
    engine::validate_supported(&r).unwrap();
    r
}
#[test]
fn firebolt_hits_and_ignites_phase_actor_inside_blocking_wall_without_continuing() {
    let mut r=phase_actor_in_wall();
    r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    // Use a real haste action: retain public dispatch/events without a normal
    // environmental burn tick or an NPC move changing the collision fixture.
    engine::actor_mut(&mut r,20.).unwrap().conditions.insert("paralyzed".into(),100.);
    r.phase=Field::Missing;r.player.conditions.insert("hasted".into(),5.);
    let tick=r.tick;let outcome=engine::dispatch(&mut r,&shot()).unwrap();
    assert!(!outcome.took_turn);assert_eq!(r.tick,tick);let events=outcome.events;
    assert_eq!((hp(&r,20.),hp(&r,21.)),(920.,500.));
    assert!(engine::condition(engine::actor(&r,20.).unwrap(),"burning")>0.);
    let f=r.floor().unwrap();assert_eq!(f.tiles[10*f.width()+13],"wall");
    let route=events.iter().filter_map(|e|e.animation.as_ref())
        .find(|a|a.proposal=="A02"&&a.stage==corpse_core::events::Stage::Travel).unwrap();
    let impact=corpse_core::events::EventPoint{x:13.,y:10.};
    assert_eq!(route.cells.last(),Some(&impact));assert_eq!(route.cells.iter().filter(|p|*p==&impact).count(),1);
}
#[test]
fn submerged_occupant_in_blocking_cell_is_skipped_but_wall_still_stops_firebolt() {
    let mut r=phase_actor_in_wall();engine::actor_mut(&mut r,20.).unwrap().conditions.insert("underwater".into(),1.);
    let rng=r.rng;cast(&mut r,11.,10.);
    assert_eq!((hp(&r,20.),hp(&r,21.)),(1000.,500.));
    assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"burning"),0.);assert_eq!(r.rng,rng);
}
#[test]
fn legacy_firebolt_keeps_original_wall_before_occupant_collision() {
    let mut r=phase_actor_in_wall();r.firebolt_travel_from=Field::Null;cast(&mut r,11.,10.);
    assert_eq!((hp(&r,20.),hp(&r,21.)),(1000.,500.));
    assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"burning"),0.);
}
