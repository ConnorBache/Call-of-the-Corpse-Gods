//! Accelerated descent uses effective-depth identities at every physical boundary.
use crate::{Actor, Command, Field, Floor, Run, fast_mode as fast, storage, temporal};
use std::sync::Arc;

fn room(depths: &[u32], fast_mode: bool) -> Run {
    let mut r = crate::startup::new_run_shell_revision("0-1-2:fast-mode-regressions", "Avatar", false, 52.).unwrap();
    r.fast_mode = if fast_mode { true.into() } else { Field::Missing };
    r.player.abilities.clear(); r.player.ability_reserves=Field::Missing;
    r.player.ability_order=Field::Missing; r.player.equipment_abilities=Field::Missing;
    r.player.active.clear(); r.player.conditions.clear();
    r.player.x=10.; r.player.y=10.; r.next_id=1000.;
    r.depth=depths[0] as f64;
    for &depth in depths {
        let mut f = Floor { depth:depth as f64,domain:"Dungeon".into(),width:79.0.into(),height:29.0.into(),
            tiles:vec!["floor".into();79*29],lights:vec![100.;79*29],seen:vec![false;79*29],..Default::default() };
        f.tiles[10*79+10]="up".into(); f.tiles[10*79+12]="down".into();
        r.floors.push(Arc::new(f)); r.actors_at_mut(depth as f64); r.ground_at_mut(depth as f64);
    }
    r
}

#[test]
fn local_floor_ticks_use_physical_neighbors_without_catch_up_debt() {
    let mut r=room(&[5,7,10,11,13,15],true);
    r.depth=10.;
    r.compressed_depths=true.into();
    r.local_floor_ticks_from=0usize.into();
    r.tick=100.;
    for (i,d) in [5.,7.,11.,13.,15.].into_iter().enumerate() {
        let f=r.floors.iter_mut().find(|f|f.depth==d).unwrap();
        let f=Arc::make_mut(f);
        f.last_tick=80.;
        f.dirty=true;
        let mut a=crate::population::create_actor_revision("rat",400.+i as f64,17.,10.,false,52.).unwrap();
        a.conditions.insert("paralyzed".into(),80.);
        r.actors_at_mut(d).push(a);
    }
    let state=|r:&Run,d| (r.floors.iter().find(|f|f.depth==d).unwrap().last_tick,
        crate::engine::condition(&r.actors_at(d)[0],"paralyzed"));
    let distant=r.floors.iter().find(|f|f.depth==13.).unwrap().clone();
    crate::offscreen::tick_inactive_floors(&mut r).unwrap();
    for d in [7.,11.] { assert_eq!(state(&r,d),(100.,79.)); }
    for d in [5.,13.,15.] { assert_eq!(state(&r,d),(80.,80.)); }
    assert!(Arc::ptr_eq(&distant,r.floors.iter().find(|f|f.depth==13.).unwrap()));
    r.depth=11.;
    r.tick=101.;
    crate::offscreen::tick_inactive_floors(&mut r).unwrap();
    assert_eq!(state(&r,13.),(101.,79.));
    assert_eq!(state(&r,7.),(100.,79.));
    // Direct arrival also drops debt; the active scheduler owns its next tick.
    crate::offscreen::catch_up_floor(&mut r,15.).unwrap();
    assert_eq!(state(&r,15.),(101.,80.));
}

#[test]
fn local_floor_ticks_allow_real_fall_into_frozen_destination_without_simulating_it() {
    let mut r=room(&[7,10,11,13,15],true);
    r.depth=10.;r.compressed_depths=true.into();r.local_floor_ticks_from=0usize.into();
    r.tick=50.;
    let source=Arc::make_mut(r.floors.iter_mut().find(|f|f.depth==11.).unwrap());
    source.tiles[10*79+16]="chasm".into();
    let dest=Arc::make_mut(r.floors.iter_mut().find(|f|f.depth==13.).unwrap());
    dest.dirty=true;dest.last_tick=3.;
    let mut sentinel=crate::population::create_actor_revision("rat",401.,17.,12.,false,52.).unwrap();
    sentinel.conditions.insert("paralyzed".into(),100.);
    r.actors_at_mut(13.).push(sentinel);
    let mut falling=crate::population::create_actor_revision("rat",400.,16.,10.,false,52.).unwrap();
    falling.state="sleeping".into();r.actors_at_mut(11.).push(falling);
    crate::travel::fall_actors_one_level(&mut r,11.,&[400.],None).unwrap();
    assert!(r.actors_at(11.).is_empty());
    assert!(r.actors_at(13.).iter().any(|a|a.id==400.));
    assert_eq!(r.floors.iter().find(|f|f.depth==13.).unwrap().last_tick,3.);
    assert_eq!(crate::engine::condition(&r.actors_at(13.)[0],"paralyzed"),100.);
    r.depth=11.;r.tick+=1.;
    crate::offscreen::tick_inactive_floors(&mut r).unwrap();
    assert_eq!(crate::engine::condition(&r.actors_at(13.)[0],"paralyzed"),99.);
    assert_eq!(r.floors.iter().find(|f|f.depth==13.).unwrap().last_tick,51.);
}

#[test]
fn local_floor_ticks_do_not_expand_a_distant_collapse() {
    let mut r=room(&[7,10,11,13],true);
    r.depth=10.;r.compressed_depths=true.into();r.local_floor_ticks_from=0usize.into();
    let at=10*79+40;
    let f=Arc::make_mut(r.floors.iter_mut().find(|f|f.depth==13.).unwrap());
    f.tiles[at]="chasm".into();
    let mut cells=vec![false;f.tiles.len()];cells[at]=true;
    r.collapse=Arc::new(crate::finale::Collapse {
        started_tick:0.,next_pulse_tick:2.,waves:vec![crate::finale::CollapseFloor {
            depth:13,up:Some(10*79+10),cells,frontier:vec![at],..Default::default()
        }],..Default::default()
    }).into();
    r.tick=10.;
    crate::finale::world_turn(&mut r).unwrap();
    assert_eq!(r.collapse.as_ref().unwrap().waves[0].frontier,vec![at]);
    r.depth=11.;r.tick=12.;
    crate::finale::world_turn(&mut r).unwrap();
    assert_eq!(r.collapse.as_ref().unwrap().waves[0].frontier.len(),4);
}

#[test]
fn fast_depth_helpers_cover_fixed_route_and_every_scheduled_godbeast() {
    let depths:Vec<_>=(1..40).step_by(2).chain([40]).collect();
    let r=room(&depths,true);
    for pair in depths.windows(2) {
        assert_eq!(fast::below(&r,pair[0] as f64),pair[1] as f64);
        assert_eq!(fast::above(&r,pair[1] as f64),pair[0] as f64);
    }
    for d in 1..=40 { assert_eq!(fast::playable(&r,d as f64),d%2==1 || d==40); }
    assert_eq!(fast::destination(&r,4.,1.),6.); // Physical interval arithmetic remains literal.
    for boss in (5..40).step_by(5) {
        assert_eq!(fast::boss_depth(boss-1,boss+1),Some(boss));
        assert_eq!(fast::boss_depth(boss-2,boss),Some(boss));
        assert_eq!(fast::boss_depth(boss,boss+2),None);
    }
    assert_eq!(fast::boss_depth(39,40),None);
    let n=room(&[1,2,3],false);assert_eq!(fast::below(&n,1.),2.);assert_eq!(fast::above(&n,3.),2.);
}
#[test]
fn fast_stairs_and_explicit_teleports_use_real_destination_once() {
    let mut r=room(&[1,3,5,7],true);r.depth=3.;r.player.x=12.;
    let rng=r.rng;
    assert!(crate::travel::travel(&mut r,1.,false,false).unwrap());assert_eq!(r.depth,5.);
    assert_eq!(r.log.last().unwrap(), "You descend.");
    assert!(crate::travel::travel(&mut r,-1.,false,false).unwrap());assert_eq!(r.depth,3.);
    assert_eq!(r.log.last().unwrap(), "You ascend.");
    assert!(crate::travel::travel_to(&mut r,5.,false,true).unwrap());assert_eq!(r.depth,5.);
    assert_eq!(r.log.last().unwrap(), "You descend.");
    assert_eq!(r.floors.len(),4);assert_eq!(r.rng,rng);
}
#[test]
fn fast_actor_corpse_item_and_collapsed_floor_falls_skip_the_same_depth() {
    for method in ["actor","corpse","item","collapse"] {
        let mut r=room(&[1,3,5,7],true);r.depth=3.;r.player.x=20.;r.player.y=20.;
        r.floor_mut().unwrap().tiles[10*79+11]="chasm".into();
        let a=Actor { id:400.,x:11.,y:10.,hp:100.,max_hp:100.,energy:100.,state:"sleeping".into(),
            stats:crate::Stats {max_energy:100.,..Default::default()},..Default::default() };
        match method {
            "actor"=>{r.actors_at_mut(3.).push(a);crate::travel::fall_actors_one_level(&mut r,3.,&[400.],None).unwrap();assert!(r.actors_at(5.).iter().any(|a|a.id==400.));},
            "corpse"=>{let mut dead=a;dead.dead=true.into();dead.hp=0.;r.actors_at_mut(3.).push(dead.clone());
                let placed=crate::corpse_placement::place_corpse(&mut r,3.,dead,|_,_|Ok(())).unwrap();
                assert!(matches!(placed,crate::item_placement::PlacementResult::Placed {depth:5.,..}));},
            "item"=>{let item=crate::GroundItem{x:11.,y:10.,item:crate::Item{id:400.,kind:"heart".into(),name:"Rat heart".into(),..Default::default()},..Default::default()};
                let placed=crate::item_placement::place_ground_item(&mut r,3.,item,|_,_|Ok(())).unwrap();
                assert!(matches!(placed,crate::item_placement::PlacementResult::Placed{depth:5.,..}));},
            _=>{r.floor_mut().unwrap().tiles[10*79+11]="floor".into();r.actors_at_mut(3.).push(a);
                assert!(crate::travel::collapse_descent(&mut r,11.,10.,true).unwrap());assert!(r.actors_at(5.).iter().any(|a|a.id==400.));},
        }
        assert!(!r.actors.contains_key("4"));assert_eq!(r.floors.len(),4);
    }
}
#[test]
fn fast_player_fall_and_flying_chase_record_effective_destination() {
    let mut r=room(&[1,3,5,7],true);r.depth=3.;r.player.x=11.;
    r.floor_mut().unwrap().tiles[10*79+11]="chasm".into();
    let mut a=Actor {id:400.,x:9.,y:10.,hp:100.,max_hp:100.,energy:100.,state:"hunting".into(),
        abilities:vec!["enemy:beast:flight".into()],stats:crate::Stats{perception:200.,max_energy:100.,..Default::default()},
        last_seen:crate::ActorLastSeen{x:11.,y:10.}.into(),..Default::default()};
    a.conditions.insert("targetId".into(),r.player.id);r.actors_at_mut(3.).push(a);
    assert!(crate::travel::jump_chasm(&mut r).unwrap());assert_eq!(r.depth,5.);
    let hunter=&r.actors_at(3.)[0];assert_eq!(hunter.conditions.get("chaseDepth"),Some(&5.));
    let mut source=r.clone();source.depth=3.;assert!(crate::travel::flying_chasm_chase(&source,hunter));
    assert!(!r.actors.contains_key("4"));
}
#[test]
fn player_falls_record_descent_but_enemy_falls_do_not_announce_it() {
    let mut r = room(&[1,3,5,7], true);
    r.depth = 3.;
    let at = r.player.y as usize * 79 + r.player.x as usize;
    r.floor_mut().unwrap().tiles[at] = "chasm".into();
    let player = r.player.id;
    crate::travel::fall_actors_one_level(&mut r, 3., &[player], None).unwrap();
    assert_eq!(r.depth, 5.);
    assert_eq!(r.log.iter().filter(|line| *line == "You descend.").count(), 1);
    let a = Actor { id: 400., x: 11., y: 10., hp: 100., max_hp: 100., ..Default::default() };
    r.floor_mut().unwrap().tiles[10 * 79 + 11] = "chasm".into();
    r.actors_at_mut(5.).push(a);
    crate::travel::fall_actors_one_level(&mut r, 5., &[400.], None).unwrap();
    assert!(r.actors_at(7.).iter().any(|actor| actor.id == 400.));
    assert_eq!(r.depth, 5.);
    assert_eq!(r.log.iter().filter(|line| *line == "You descend.").count(), 1);
}
#[test]
fn fast_snapshot_future_and_wire_round_trip_keep_mode_and_sparse_identity() {
    let mut r=room(&[1,3,5],true);r.depth=3.;
    storage::validate(&r).unwrap();
    let raw=storage::serialize(&r).unwrap();assert!(raw.contains("\"fastMode\":true"));
    let saved=storage::deserialize(&raw).unwrap();assert!(fast::enabled(&saved));assert_eq!(saved.depth,3.);
    let snapshot=temporal::Snapshot::capture(&r);
    let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    r.fast_mode=Field::Missing;r.depth=1.;snapshot.restore(&mut r).unwrap();assert!(fast::enabled(&r));assert_eq!(r.depth,3.);
    let restored=anchor.prepare_restore().unwrap();assert!(fast::enabled(&restored));assert_eq!(restored.depth,3.);
    let mut invalid=saved.clone();invalid.fast_mode=Field::Missing;assert!(storage::validate(&invalid).is_err());
    invalid=saved.clone();invalid.fast_mode=Field::Null;assert!(storage::validate(&invalid).is_err());
    invalid=saved;invalid.simulation_revision=51.0.into();assert!(storage::validate(&invalid).is_err());
}
#[test]
fn fast_gas_pilot_reads_sparse_floor_identity_not_array_offset() {
    let r=room(&[1,3,5],true);
    assert!(crate::environment::capture_pilot_gas_cell(&r,3f64.to_bits(),1).is_ok());
    assert!(crate::environment::capture_pilot_gas_borrowed(&r,3f64.to_bits(),1).is_ok());
    assert!(crate::environment::capture_pilot_gas_cell(&r,2f64.to_bits(),1).is_err());
}
#[test]
fn fast_normal_save_and_replay_wire_remain_absent_and_unchanged() {
    let r=room(&[1,2,3],false);storage::validate(&r).unwrap();
    let raw=storage::serialize(&r).unwrap();assert!(!raw.contains("fastMode"));
    let replay=crate::replay::ReplayRecord::from_run(&r);assert!(!serde_json::to_string(&replay).unwrap().contains("fastMode"));
    let mut fast_record=replay;fast_record.fast_mode=true.into();
    assert!(crate::replay::ReplaySession::from_initial(r,fast_record).is_err());
}
#[test]
fn fast_generation_relocates_all_skipped_bosses_without_changing_their_schedule() {
    let seed=crate::rng::seed_hash("0-1-2:fast-boss-regression");let plan=crate::generation::domain_plan_revision(seed,52.);
    for boss in (5..40).step_by(5) {
        let destination=boss+1;
        let mut generated=None;
        for attempt in 0..crate::startup::COMPLETE_FLOOR_ATTEMPT_LIMIT {
            if let Ok(f)=crate::generation::generate_fast_floor_from_attempt(destination,seed,None,52.,
                attempt*crate::generation::GENERATION_CANDIDATE_LIMIT,Some(boss)) {generated=Some(f);break;}
        }
        let f=generated.unwrap_or_else(||panic!("boss {boss} -> {destination}"));
        assert_eq!(f.depth,destination as f64);assert!(f.godbeast);
        assert_eq!(f.domain,plan[(destination as usize-1)/5].domain);
        assert_eq!(f.godbeast_event.as_ref().unwrap().kind,plan[(boss as usize-1)/5].godbeast_kind);
    }
}
#[test]
fn fast_seed_replay_restores_mode_and_first_floor_lookahead() {
    let initial=crate::startup::create_avatar_run_fast("0-1-2:fast-start-regression").unwrap();
    assert!(fast::enabled(&initial));assert_eq!(initial.floors.len(),2);
    assert_eq!(initial.floors[1].depth,initial.depth+2.);
    let mut expected=temporal::Engine::new(initial.clone()).unwrap();
    let opening=expected.save().unwrap();expected.dispatch(&Command::Wait).unwrap();
    let record=crate::replay::ReplayRecord::from_run(&expected.run);
    let wire=serde_json::to_string(&record).unwrap();assert!(wire.contains("\"fastMode\":true"));
    let mut replay=crate::replay::ReplaySession::create(serde_json::from_str(&wire).unwrap(),true).unwrap();
    let raw=storage::canonical_value(&initial).unwrap();
    let admitted=storage::canonical_value(&replay.engine.run).unwrap();
    let keys:std::collections::BTreeSet<_>=raw.as_object().unwrap().keys().chain(admitted.as_object().unwrap().keys()).collect();
    let differences:Vec<_>=keys.into_iter().filter(|key|raw.get(*key)!=admitted.get(*key)).map(String::as_str).collect();
    eprintln!("FAST_REPLAY_RAW_STARTUP_DIFFERENCES={differences:?}");
    assert_eq!(differences,vec!["futureFloor", "replaySnapshotFrom"],"Engine admission initializes the floor-entry checkpoint and replay policy");
    assert!(replay.engine.save().unwrap()==opening,"seed replay opening differs from admitted initial engine");
    replay.step().unwrap();assert!(replay.engine.save().unwrap()==expected.save().unwrap(),"seed replay command result differs");
    if let Some(root)=std::env::var_os("COTCG_FAST_MODE_EXPORT_DIR") {
        let dir=std::path::Path::new(&root).join("fast-seed-replay");std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("initial-run.json"),&opening).unwrap();
        std::fs::write(dir.join("after-wait-run.json"),expected.save().unwrap()).unwrap();
        std::fs::write(dir.join("replay.json"),&wire).unwrap();
        std::fs::write(dir.join("provenance.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "seed":initial.seed,"constructor":"startup::create_avatar_run_fast","commands":["Wait"],
            "terrainActorInventoryEdits":false,"strictReplayEquality":true,"depth":expected.run.depth,"tick":expected.run.tick,
            "purpose":"Authentic generated Fast startup and one accepted production command, not a broad playthrough"
        })).unwrap()).unwrap();
    }
    replay.restart().unwrap();assert!(fast::enabled(&replay.engine.run));assert_eq!(replay.engine.run.depth,initial.depth);
}
#[test]
fn fast_arrow_schedule_intersects_visited_depths_without_topups() {
    for index in 0..256 {
        let seed=format!("0-1-2:arrow-schedule-{index}");
        let mut r=room(&[1,2,3],false);r.seed=seed.clone();
        let saved:Run=serde_json::from_str(&storage::serialize(&r).unwrap()).unwrap();
        for first in (1..40).step_by(5) {
            let scheduled=crate::startup::arrow_bundle_floors(&seed,first);
            assert!([2,3].contains(&scheduled.len()));assert!(scheduled.windows(2).all(|w|w[0]<w[1]));
            assert!(scheduled.iter().all(|&d|(first..first+5).contains(&d)));
            for d in first..first+5 {assert_eq!(crate::startup::arrow_bundle_floors(&saved.seed,d),scheduled);}
            for parity in [0,1] {
                let visited=scheduled.iter().filter(|&&d|d%2==parity).count();
                assert!(visited<=if first%2==parity{3}else{2});
                let actual_arrows=visited*5;assert!([0,5,10,15].contains(&actual_arrows));
            }
        }
    }
    let scheduled=crate::startup::arrow_bundle_floors("0-1-2:fast-arrow-3",1);
    assert_eq!(scheduled,vec![1,3]);assert_eq!(scheduled.iter().filter(|&&d|d%2==0).count(),0);
}
#[test]
fn fast_unsupported_legacy_flag_rejects_before_command_mutation() {
    let mut r=room(&[1,2,3],false);r.fast_mode=true.into();r.simulation_revision=51.0.into();
    let before=storage::serialize(&r).unwrap();assert!(crate::engine::dispatch(&mut r,&Command::Wait).is_err());
    assert_eq!(storage::serialize(&r).unwrap(),before);
}
#[test]
fn fast_skipped_and_out_of_bounds_targets_reject_before_mutation() {
    let mut r=room(&[1,3,5,7],true);r.depth=3.;
    for target in [0.,2.,4.,6.,38.,41.,f64::NAN] {
        let before=storage::serialize(&r).unwrap();
        assert!(!crate::travel::travel_to(&mut r,target,false,true).unwrap());
        assert_eq!(storage::serialize(&r).unwrap(),before,"target {target}");
    }
}
#[test]
fn fast_final_physical_floor_does_not_generate_unreachable_fall_targets() {
    // Only floor40 has no physical successor on the finalized route.
    for last in [40.] {
        let mut r=room(&[last as u32],true);r.depth=last;r.player.x=11.;
        r.floor_mut().unwrap().tiles[10*79+11]="chasm".into();
        let before=storage::serialize(&r).unwrap();let player=r.player.id;
        crate::travel::fall_actors_one_level(&mut r,last,&[player],None).unwrap();
        assert_eq!(storage::serialize(&r).unwrap(),before);
        let tiles=r.floor().unwrap().tiles.clone();assert!(!crate::travel::collapse_descent(&mut r,20.,20.,true).unwrap());
        assert_eq!(r.depth,last);assert_eq!(r.floors.len(),1);assert_eq!(r.floor().unwrap().tiles,tiles);
    }
}

#[test]
fn fast_finale_one_step_and_ally_return_use_physical_neighbors() {
    let depths:Vec<_>=(1..40).step_by(2).chain([40]).collect();
    for (source,destination) in [(3.,5.),(39.,40.),(40.,39.)] {
        let mut r=room(&depths,true);r.depth=source;
        r.player.x=if destination>source {12.} else {10.};
        let a=Actor {id:400.,name:"Ally".into(),x:r.player.x-1.,y:10.,hp:100.,max_hp:100.,
            energy:100.,state:"ally".into(),stats:crate::Stats{max_energy:100.,..Default::default()},..Default::default()};
        r.actors_at_mut(source).push(a);
        assert!(crate::travel::travel(&mut r,if destination>source {1.} else {-1.},false,false).unwrap());
        assert_eq!(r.depth,destination);
        assert_eq!(r.actors_at(source)[0].conditions.get("allyStairDepth"),Some(&destination));
        for _ in 0..3 {r.tick+=1.;crate::travel::tick_ally_stair_followers(&mut r).unwrap();}
        assert!(r.actors_at(source).is_empty());assert_eq!(r.actors_at(destination)[0].id,400.);
        assert!(!crate::travel::has_ally_stair_follow(&r.actors_at(destination)[0]));
    }
}
#[test]
fn fast_ascend_uses_previous_odd_floor_and_finale_return() {
    for (source,destination) in [(5.,3.),(40.,39.)] {
        let mut r=room(&(1..40).step_by(2).chain([40]).collect::<Vec<_>>(),true);r.depth=source;
        r.player.abilities.push("anavani-ascend".into());
        crate::progression::initialize_ability_state_for_revision(52.,&mut r.player,"anavani-ascend",0.).unwrap();
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        assert!(crate::revision50_ability_energy::can_pay(&r,r.player.id,"anavani-ascend",50.));
        crate::mutation_journal::make_mut(r.floors.iter_mut().find(|f|f.depth==destination).unwrap()).tiles[10*79+10]="chasm".into();
        let energy_before=r.player.energy+r.player.ability_reserves.as_ref().unwrap()["anavani-ascend"].energy;
        // The ability API returns whether it consumes a turn, not whether it succeeds.
        assert!(!crate::abilities::activate(&mut r,"anavani-ascend",&Field::Missing,&Field::Missing,&Field::Missing).unwrap());
        assert_eq!(r.depth,destination);
        let energy_after=r.player.energy+r.player.ability_reserves.as_ref().unwrap()["anavani-ascend"].energy;
        assert_eq!(energy_before-energy_after,50.);
    }
}
#[test]
fn fast_finale_collapse_propagates_40_to_39_to_37_and_escapes_at_one() {
    let depths:Vec<_>=(1..40).step_by(2).chain([40]).collect();
    let mut r=room(&depths,true);r.depth=40.;r.player.x=20.;r.player.y=20.;
    r.local_floor_ticks_from=Field::Missing; // Historical replay: collapse runs at all distances.
    let mut boss=crate::finale::administrator_revision(400.,10.,10.,52.);boss.hp=0.;boss.dead=true.into();
    r.actors_at_mut(40.).push(boss.clone());
    assert!(crate::finale::actual_death(&mut r,&boss).unwrap());
    assert_eq!(r.collapse.as_ref().unwrap().waves.iter().map(|w|w.depth).collect::<Vec<_>>(),vec![40,39]);
    r.tick=4.;crate::finale::world_turn(&mut r).unwrap();
    assert!(r.collapse.as_ref().unwrap().waves.iter().any(|w|w.depth==37));
    assert!(!r.collapse.as_ref().unwrap().waves.iter().any(|w|w.depth==38));
    r.depth=1.;crate::entrance::install_floor_one_entrance(r.floor_mut().unwrap()).unwrap();
    let f=r.floor().unwrap();let at=crate::entrance::cell(f).unwrap();
    (r.player.x,r.player.y)=((at%f.width())as f64,(at/f.width())as f64);
    assert!(crate::finale::try_escape(&mut r));assert_eq!(r.status,"won");
    let raw=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&raw).unwrap();
    assert_eq!(storage::serialize(&loaded).unwrap(),raw);
}
#[test]
fn fast_final_step_actor_and_item_falls_arrive_at_40() {
    let mut r=room(&[37,39,40],true);r.depth=39.;r.player.x=20.;
    r.floor_mut().unwrap().tiles[10*79+11]="chasm".into();
    r.actors_at_mut(39.).push(Actor{id:400.,x:11.,y:10.,hp:100.,max_hp:100.,state:"sleeping".into(),..Default::default()});
    crate::travel::fall_actors_one_level(&mut r,39.,&[400.],None).unwrap();
    assert_eq!(r.actors_at(40.)[0].id,400.);
    let item=crate::GroundItem{x:11.,y:10.,item:crate::Item{id:401.,kind:"heart".into(),name:"Rat heart".into(),..Default::default()},..Default::default()};
    assert!(matches!(crate::item_placement::place_ground_item(&mut r,39.,item,|_,_|Ok(())).unwrap(),crate::item_placement::PlacementResult::Placed{depth:40.,..}));
}
#[test]
fn fast_debug_reconstructs_only_visited_rewards_and_preserves_save_choices() {
    use crate::{debug_floor as debug,inventory,progression};
    const SEED:&str="0-1-2:fast-debug-regression";
    for floor in (2..40).step_by(2) {assert!(debug::create_avatar_run_fast(SEED,floor).unwrap_err().contains("odd floor"));}
    for target in [1,5,40] {
        let mut reference=crate::startup::new_run_shell_revision(SEED,"Avatar",false,52.).unwrap();
        reference.fast_mode=true.into();let to=fast::arrival_target(&reference,target);
        crate::startup::ensure_floors(&mut reference,to).unwrap();
        let mut count=0;let mut blessings=0.;
        for f in reference.floors.iter().filter(|f|f.depth<(target as f64)) {
            for a in reference.actors_at(f.depth) {
                if debug::natural_heart(a) {let heart=inventory::heart_for_revision(a,99999.,52.).unwrap();count+=1;blessings+=2.*heart.exp.value_or(0.);}
                for item in a.monster_state.as_ref().and_then(|s|s.carried_items.as_ref()).into_iter().flatten().filter(|i|i.kind=="heart") {count+=1;blessings+=2.*item.exp.value_or(0.);}
            }
            for g in reference.ground.get(&(f.depth as u32).to_string()).into_iter().flat_map(|v|v.iter()).filter(|g|g.item.kind=="heart"&&debug::ordinary_ground(&reference,f.depth as u32,g)) {count+=1;blessings+=2.*g.item.exp.value_or(0.);}
        }
        let mut r=debug::create_avatar_run_fast(SEED,target).unwrap();
        assert_eq!(r.floors.last().unwrap().depth,to as f64);assert_eq!(r.depth,target as f64);
        assert_eq!(r.playtest.as_ref().unwrap().prior_hearts,count);assert_eq!(r.playtest.as_ref().unwrap().prior_blessings,blessings);
        assert_eq!(r.actors_at(target as f64),reference.actors_at(target as f64));
        assert!(r.floors.iter().all(|f|fast::playable(&r,f.depth)));
        while !r.offers.is_empty() {let id=r.offers[0].clone();progression::choose_offer(&mut r,&id).unwrap();debug::advance(&mut r).unwrap();}
        assert!(debug::choices_complete(&r));assert_eq!(r.run_stats.as_ref().unwrap().blessings_received,blessings);
        assert!(debug::state(&r).unwrap().stash.iter().all(|v|v.depth==0||fast::playable(&r,v.depth as f64)));
        let raw=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&raw).unwrap();assert_eq!(storage::serialize(&loaded).unwrap(),raw);
        debug::enter(&mut r).unwrap();temporal::Engine::new(r).unwrap();
    }
}

#[test]
fn fast_configuration_rejects_non_avatar_dev_and_unknown_revision_transactionally() {
    for mode in ["Blessed","Unblessed","Avatar"] {
        for (revision,dev) in [(51.,false),(52.,false),(53.,false),(52.,true)] {
            let mut r=room(&[1,3,5],true);r.mode=mode.into();r.simulation_revision=revision.into();r.dev=dev;
            if mode=="Avatar" && revision==52. && !dev {storage::validate(&r).unwrap();continue;}
            assert!(storage::validate(&r).is_err());assert!(crate::validation::validate(&r).is_err());
            let before=storage::serialize(&r).unwrap();assert!(crate::engine::dispatch(&mut r,&Command::Wait).is_err());
            assert_eq!(storage::serialize(&r).unwrap(),before);
            assert!(crate::replay::ReplaySession::create(crate::replay::ReplayRecord::from_run(&r),true).is_err());
        }
    }
}

#[test]
fn compressed_route_has_twenty_floors_and_all_eight_milestones() {
    let depths:Vec<_>=(1..=20).map(fast::content_depth).collect();
    let mut r=room(&depths,true);r.compressed_depths=true.into();
    assert_eq!(depths,vec![1,3,5,7,10,11,13,15,17,20,21,23,25,27,30,31,33,35,37,40]);
    for (i,&d) in depths.iter().enumerate() {
        assert_eq!(fast::display_depth(&r,d as f64),i as f64+1.);
        assert!(fast::playable(&r,d as f64));
        if i>0 { assert_eq!(fast::above(&r,d as f64),depths[i-1] as f64); }
        if i<19 { assert_eq!(fast::below(&r,d as f64),depths[i+1] as f64); }
    }
    assert!(!fast::playable(&r,39.)); assert!(!fast::playable(&r,41.));
    assert_eq!(fast::maximum_display_depth(&r),20);
    assert_eq!((5..=40).step_by(5).map(|d|fast::display_depth(&r,d as f64) as u32).collect::<Vec<_>>(),vec![3,5,8,10,13,15,18,20]);
    storage::validate(&r).unwrap();
    let raw=storage::serialize(&r).unwrap();let saved=storage::deserialize(&raw).unwrap();
    assert!(fast::compressed(&saved));assert_eq!(saved.floors.len(),20);
    let snap=temporal::Snapshot::capture(&r);
    let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    r.compressed_depths=Field::Missing;snap.restore(&mut r).unwrap();assert!(fast::compressed(&r));
    assert!(fast::compressed(&anchor.prepare_restore().unwrap()));
}
#[test]
fn compressed_finale_travel_uses_twentieth_floor_without_extra_floor() {
    let mut r=room(&[1,3,5,7,10,11,13,15,17,20,21,23,25,27,30,31,33,35,37,40],true);
    r.compressed_depths=true.into();r.depth=37.;r.player.x=12.;
    assert!(crate::travel::travel(&mut r,1.,false,false).unwrap());assert_eq!(r.depth,40.);
    assert_eq!(fast::display_depth(&r,r.depth),20.);
    assert!(crate::travel::travel(&mut r,-1.,false,false).unwrap());assert_eq!(r.depth,37.);
    assert_eq!(r.floors.len(),20);
    assert!(!r.floors.iter().any(|f|f.depth==39.));
}
#[test]
fn compressed_new_game_replays_seed_with_same_route_and_rewards() {
    let initial=crate::startup::create_avatar_run("0-1-2:compressed-start-regression").unwrap();
    assert!(fast::compressed(&initial));assert!(fast::enabled(&initial));
    assert_eq!(initial.floors.len(),2);assert_eq!(initial.floors[1].depth,3.);
    let mut expected=temporal::Engine::new(initial).unwrap();expected.dispatch(&Command::Wait).unwrap();
    let record=crate::replay::ReplayRecord::from_run(&expected.run);
    let mut replay=crate::replay::ReplaySession::create(record,true).unwrap();
    replay.step().unwrap();assert_eq!(replay.engine.save().unwrap(),expected.save().unwrap());
}
#[test]
fn compressed_generation_reaches_all_bosses_and_depth40_finale() {
    let mut r=crate::startup::new_run_shell_revision("0-1-2:compressed-generation-regression","Avatar",false,52.).unwrap();
    r.fast_mode=true.into();r.compressed_depths=true.into();
    crate::startup::ensure_floors(&mut r,40).unwrap();
    assert_eq!(r.floors.len(),20);
    assert_eq!(r.floors.last().unwrap().depth,40.);
    for boss in (5..40).step_by(5) {
        let f=r.floors.iter().find(|f|f.depth==boss as f64).unwrap();
        assert!(f.godbeast,"boss {boss}");
    }
    assert!(!r.floors.iter().any(|f|f.depth==39.));
}
