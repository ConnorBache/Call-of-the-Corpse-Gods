//! Current generation horizon and square-current integration, not frozen-R51 changes.
use corpse_core::{Field, Run, generation, rng, square_currents, startup, storage, temporal::Engine, travel};

#[test]
fn speculative_horizon_is_exactly_one_below_and_old_revisions_keep_their_batches() {
    for depth in 1..=40 {
        assert_eq!(startup::initial_generation_target(52., depth),(depth+1).min(40));
        assert_eq!(startup::arrival_generation_target(52., depth),(depth+1).min(40));
        for revision in [29.,39.,40.,51.] {
            assert_eq!(startup::initial_generation_target(revision,depth),depth.max(10));
            assert_eq!(startup::arrival_generation_target(revision,depth),
                (((depth as f64+5.)/5.).ceil()*5.).min(40.) as u32);
        }
    }
}

#[test]
fn startup_and_stair_travel_generate_only_the_needed_horizon_without_pruning_saves() {
    let seed="0-5-11:square-current-startup";
    let mut r=startup::create_avatar_run(seed).unwrap();
    assert_eq!(r.floors.len(),2);assert_eq!(r.actors.len(),2);assert_eq!(r.ground.len(),2);
    let first=r.floors[0].clone();
    assert!(travel::travel(&mut r,1.,false,false).unwrap());
    assert_eq!(r.depth,2.);assert_eq!(r.floors.len(),3);
    assert_eq!(r.floors[0].currents,first.currents);
    assert!(travel::travel(&mut r,-1.,false,false).unwrap());
    assert_eq!(r.depth,1.);assert_eq!(r.floors.len(),3);
    let saved=Engine::new(r).unwrap().save().unwrap();
    let loaded=Engine::load(&saved).unwrap();assert_eq!(loaded.save().unwrap(),saved);
    assert_eq!(loaded.run.floors.len(),3);
    let old=startup::create_run_revision(seed,"Avatar",false,51.).unwrap();
    assert_eq!(old.floors.len(),10);
}

fn fall_scene() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();
    r.floors.truncate(2);
    let keys:Vec<_>=r.actors.keys().filter(|key|key.as_str()!="1"&&key.as_str()!="2").cloned().collect();
    for key in keys {r.actors.shift_remove(&key);}
    r.ground.retain(|key,_|key=="1"||key=="2");
    for f in &mut r.floors {let f=std::sync::Arc::make_mut(f);f.tiles.fill("floor".into());f.currents=Field::Missing;
        f.tiles[5*79+5]="up".into();f.tiles[20*79+60]="down".into();f.dirty=false;}
    r.player.x=10.;r.player.y=10.;r.player.hp=10000.;r.player.max_hp=10000.;r.player.abilities.clear();
    r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
    r.floor_mut().unwrap().tiles[10*79+10]="chasm".into();
    r
}

#[test]
fn nonplayer_fall_does_not_prefetch_beyond_its_physical_destination() {
    let mut r=fall_scene();let p=r.player.id;
    let actor=r.actors_at_mut(1.).first_mut().unwrap();
    actor.x=10.;actor.y=10.;actor.hp=1000.;actor.max_hp=1000.;actor.abilities.clear();actor.conditions.clear();
    let id=actor.id;
    travel::fall_actors_one_level(&mut r,1.,&[id],Some(p)).unwrap();
    assert_eq!(r.depth,1.);assert_eq!(r.floors.len(),2);
    assert!(r.actors_at(2.).iter().any(|a|a.id==id));
}

#[test]
fn actual_player_cohort_fall_extends_one_floor_below_the_arrival() {
    let mut r=fall_scene();let id=r.player.id;
    travel::fall_actors_one_level(&mut r,1.,&[id],None).unwrap();
    assert_eq!(r.depth,2.);assert_eq!(r.floors.len(),3);
}

#[test]
fn generated_current_layer_matches_the_square_algorithm_and_preserves_geometry_and_godbeast_plan() {
    let seed=rng::seed_hash("square-current-floor-sweep");
    assert_eq!(generation::domain_plan_revision(seed,51.),generation::domain_plan_revision(seed,52.));
    for (depth,domain) in [(1,"Dungeon"),(7,"Archipelago"),(10,"Cave")] {
        let mut checked=false;
        for attempt in 0..generation::GENERATION_CANDIDATE_LIMIT {
            // Hold the attempt fixed: the separate island safeguard can now
            // reject an old candidate, but painting cannot alter its geometry.
            let Ok((old,old_quality,old_failed))=generation::generate_floor_candidate_revision(depth,seed,domain,depth==10,79,None,attempt,51.) else {continue;};
            let Ok((mut new,new_quality,new_failed))=generation::generate_floor_candidate_revision(depth,seed,domain,depth==10,79,None,attempt,52.) else {continue;};
            let mut expected=old.clone();let coverage=square_currents::generate_for_attempt(&mut expected,seed,attempt);
            assert_eq!(new.currents,expected.currents);
            assert!(coverage.squares<=coverage.covered_tiles+coverage.lakes);
            new.currents=old.currents.clone();assert_eq!(new,old,"all non-current geometry stays exact");
            assert_eq!(serde_json::to_value(new_quality).unwrap(),serde_json::to_value(old_quality).unwrap());
            assert_eq!(new_failed,old_failed);checked=true;break;
        }
        assert!(checked,"must exercise a shared fixed candidate for {domain}");
    }
}

#[test]
fn loaded_arrows_are_retained_even_when_they_differ_from_new_generation() {
    let mut r=startup::create_avatar_run("0-5-11:keep-current-save").unwrap();
    let n=r.floors[0].tiles.len();std::sync::Arc::make_mut(&mut r.floors[0]).currents=vec![0.;n].into();
    let before=r.floors[0].currents.clone();let bytes=storage::serialize(&r).unwrap();
    let loaded=Engine::load(&bytes).unwrap();assert_eq!(loaded.run.floors[0].currents,before);
}

#[test]
fn generating_a_missing_floor_keeps_the_loaded_prior_floor_bytes_and_rng() {
    let mut r=startup::create_avatar_run("0-5-11:partial-square-current-save").unwrap();
    // An old save may retain deeper geometry. There is no normalization pass
    // when loading or when requesting a new missing floor.
    let n=r.floors[0].tiles.len();std::sync::Arc::make_mut(&mut r.floors[0]).currents=vec![0.;n].into();
    let raw=Engine::new(r).unwrap().save().unwrap();let mut loaded=Engine::load(&raw).unwrap().run;
    let before:Vec<_>=loaded.floors.iter().map(|f|serde_json::to_value(f).unwrap()).collect();
    let rng_before=loaded.rng;
    startup::ensure_floors(&mut loaded,3).unwrap();
    assert_eq!(loaded.floors.len(),3);assert_eq!(loaded.rng,rng_before);
    for (i,old) in before.iter().enumerate(){assert_eq!(&serde_json::to_value(&loaded.floors[i]).unwrap(),old);}
    let bytes=storage::serialize(&loaded).unwrap();assert_eq!(storage::serialize(&storage::deserialize(&bytes).unwrap()).unwrap(),bytes);
}

#[test]
fn physical_offscreen_fall_can_generate_its_real_destination_but_no_speculative_extra_floor() {
    let mut r=fall_scene();let p=r.player.id;
    let at=10*79+10;std::sync::Arc::make_mut(&mut r.floors[1]).tiles[at]="chasm".into();
    // The saved fixture intentionally has no population on floor2. Add a
    // normally constructed actor with a fresh ID rather than assuming one.
    let id=r.next_id;r.next_id+=1.;
    let mut actor=corpse_core::population::create_actor_revision("rat",id,10.,10.,false,52.).unwrap();
    actor.hp=1000.;actor.max_hp=1000.;actor.abilities.clear();actor.conditions.clear();
    r.actors_at_mut(2.).push(actor);
    travel::fall_actors_one_level(&mut r,2.,&[id],Some(p)).unwrap();
    assert_eq!(r.depth,1.);assert_eq!(r.floors.len(),3);
    assert!(r.actors_at(3.).iter().any(|a|a.id==id));
}

#[test]
fn every_supported_geometry_has_bounded_total_seeded_lake_coverage() {
    for (width,height) in [(79,29),(50,50),(60,60)] {
        for seed in [0,u32::MAX] {
            let mut f=corpse_core::Floor{depth:40.,tiles:vec!["water".into();width*height],..Default::default()};
            let stats=square_currents::generate(&mut f,seed);
            assert_eq!((f.width(),f.height()),(width,height));
            assert_eq!(stats.lakes,1);assert_eq!(stats.covered_tiles,width*height);
            assert!(stats.squares>=2&&stats.squares<=width*height+1);
            assert!(stats.square_cell_visits<=stats.squares*width.min(height).pow(2));
            assert!(f.currents.as_ref().unwrap().iter().all(|v|[0.,1.,3.,5.,7.].contains(v)));
        }
    }
}
