//! Own Speed-up terrain, preserving authored r37 links and legacy overlays.
use corpse_core::{Field, Run, engine, portals, non_euclidean_generation as ne,
    non_euclidean_generation::{NonEuclideanPocket as Pocket, NonEuclideanGroup as Group},
    sub_biomes_generation, storage, temporal::{History, Snapshot}};
use std::sync::Arc;
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.inventory.clear();r.offers.clear();r.history.clear();r.replay.clear();r.future=None;
    r.phase=Field::Missing;r.haste_snapshot=Field::Missing;r.log.clear();r.log_kinds=Field::Missing;r.log_archive=Field::Missing;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.next_id=1000.;r.rng=1.;
    r.player.x=10.;r.player.y=10.;
    let mut f=r.floors[0].as_ref().clone();
    f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;
    f.godbeast_event=Field::Missing;f.godbeast=false;f.doors=Field::Missing;f.light_sources=Field::Missing;
    f.blood=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;f.razorleaves=Field::Missing;
    f.grates=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
    let(w,h)=(f.width(),f.height());
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();f.lights.fill(100.);
    f.seen.fill(true);f.remembered_tiles=Field::Missing;
    r.floors.clear();r.actors.clear();r.ground.clear();
    for depth in 1..=20 {let mut f=f.clone();f.depth=depth as f64;r.floors.push(Arc::new(f));
        r.actors.insert(depth.to_string(),Arc::new(vec![]));r.ground.insert(depth.to_string(),Arc::new(vec![]));}
    r.depth=20.;let f=r.floor_mut().unwrap();
    f.speed_up=vec![Pocket{cells:(11..=14).map(|x|10*w+x).collect(),groups:vec![
        Group{sources:vec![10*w+11],destination:10*w+12},Group{sources:vec![10*w+13],destination:10*w+14}]}].into();
    if revision>=51. {ne::install_speedup_terrain(f);}
    r
}
#[test]
fn revision51_speed_up_generation_replaces_ground_without_changing_assignments() {
    let mut old=room(50.).floor().unwrap().clone();old.speed_up=Field::Missing;
    let mut new=old.clone();sub_biomes_generation::install_links("shortcuts",&mut old,&[],&[],50.);
    sub_biomes_generation::install_links("shortcuts",&mut new,&[],&[],51.);
    assert_eq!(old.speed_up,new.speed_up);assert!(new.speed_up.as_ref().unwrap().len()>0);
    assert!(old.speed_up_terrain.is_missing());assert!(new.speed_up_terrain.value_or(false));
    let cells:Vec<_>=new.speed_up.as_ref().unwrap().iter().flat_map(|p|&p.cells).copied().collect();
    for at in 0..new.tiles.len() {assert_eq!(new.tiles[at],if cells.contains(&at){"speed-up"}else{&old.tiles[at]});}
    assert!(ne::traversable(&new));assert!(ne::validate_floor(&new));
    let once=new.clone();sub_biomes_generation::install_links("different",&mut new,&[],&[],51.);assert_eq!(new,once);
}
#[test]
fn revision51_speed_up_preserves_movement_projectiles_and_local_reversal() {
    for revision in [50.,51.] {for phase in ["normal","haste"] {
        let mut r=room(revision);r.phase=phase.to_string().into();let id=r.player.id;let rng=r.rng;
        assert!(engine::actor_passable(&r,&r.player,11.,10.));
        assert_eq!(portals::step_options(&r,&r.player,11.,10.),vec![(12.,10.)]);
        assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(r.player.x,12.);
        assert!(engine::move_actor(&mut r,id,-1.,0.,true).unwrap());assert_eq!(r.player.x,11.);
        assert!(portals::same_cell(r.floor().unwrap(),11.,10.,12.,10.));
        let trace=portals::directed_ray(r.floor().unwrap(),corpse_core::movement::Point{x:10,y:10},
            corpse_core::movement::Point{x:11,y:10},|_|true);
        assert_eq!(trace.last().unwrap(),&corpse_core::movement::Point{x:12,y:10});assert_eq!(r.rng,rng);
    }}
}
#[test]
fn revision51_replacement_deactivates_tile_links_and_exact_restoration_reactivates() {
    for revision in [50.,51.] {
        let mut r=room(revision);let w=r.floor().unwrap().width();let at=10*w+12;
        let original=r.floor().unwrap().tiles[at].clone();let id=ne::group_number(r.floor().unwrap(),at);
        r.floor_mut().unwrap().tiles[at]="wall".into();
        let f=r.floor().unwrap();
        assert_eq!(ne::exits(f,at).is_empty(),revision>=51.);
        assert_eq!(portals::links(f).len(),if revision>=51.{1}else{2});
        assert_eq!(portals::ordered_pairs(f).len(),if revision>=51.{1}else{2});
        assert_eq!(portals::aliases(f,at).len(),if revision>=51.{1}else{2});
        assert_eq!(ne::group_number(f,at),id);
        r.floor_mut().unwrap().tiles[at]=original;
        assert_eq!(ne::exits(r.floor().unwrap(),at),vec![at-1]);
    }
}
#[test]
fn revision51_speed_up_saves_memory_replay_and_temporal_state_exactly() {
    let mut r=room(51.);engine::remember(&mut r);let w=r.floor().unwrap().width();let at=10*w+11;
    assert_eq!(r.floor().unwrap().remembered_tiles.as_ref().unwrap()[at],"speed-up");
    let saved=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&saved).unwrap();
    assert_eq!(storage::serialize(&loaded).unwrap(),saved);
    let snapshot=Snapshot::capture(&r);r.floor_mut().unwrap().tiles[at]="wall".into();snapshot.restore(&mut r).unwrap();
    assert_eq!(r.floor().unwrap().tiles[at],"speed-up");
    // Revert was retired in R41; own terrain must not accidentally re-enable it.
    let mut h=History::default();let unchanged=storage::serialize(&r).unwrap();
    assert!(corpse_core::temporal::revert(&mut r,&mut h).is_err());
    assert_eq!(storage::serialize(&r).unwrap(),unchanged);
    r.player.ability_reserves=Field::Missing;
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,"morae-see-the-future",50.,50.).unwrap();
    corpse_core::temporal::remember_future(&mut r,&h).unwrap();r.floor_mut().unwrap().tiles[at]="shallow-water".into();
    assert!(corpse_core::temporal::restore_future(&mut r,&mut h).unwrap());assert_eq!(r.floor().unwrap().tiles[at],"speed-up");
    let mut first=corpse_core::temporal::Engine::new(r.clone()).unwrap();let mut second=corpse_core::temporal::Engine::new(r).unwrap();
    let command=corpse_core::Command::Move{dx:1.,dy:0.};first.dispatch(&command).unwrap();second.dispatch(&command).unwrap();
    assert_eq!(first.save().unwrap(),second.save().unwrap());
    let resumed=corpse_core::temporal::Engine::load(&first.save().unwrap()).unwrap();assert_eq!(resumed.save().unwrap(),first.save().unwrap());
}
#[test]
fn revision51_speed_up_admission_and_ids_are_revision_safe() {
    let mut r=room(51.);assert!(ne::validate_run(&r));let w=r.floor().unwrap().width();
    r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![12*w+11,12*w+12],groups:vec![Group{sources:vec![12*w+11],destination:12*w+12}]}].into();
    let f=r.floor().unwrap();assert_eq!(ne::group_number(f,12*w+11),Some(1));assert_eq!(ne::group_number(f,10*w+11),Some(2));assert_eq!(ne::group_number(f,10*w+12),Some(2));assert_eq!(ne::group_number(f,10*w+14),Some(3));
    let mut legacy=r.clone();legacy.simulation_revision=50.0.into();assert!(!ne::validate_run(&legacy));
    r.floor_mut().unwrap().speed_up_terrain=Field::Null;assert!(!ne::validate_run(&r));
    let mut r=room(51.);r.floor_mut().unwrap().tiles[100]="speed-up".into();assert!(!ne::validate_run(&r));
    assert!(ne::validate_run(&room(50.)));
}
#[test]
fn revision51_speed_up_lighting_and_visibility_are_local_and_not_opaque() {
    let mut r=room(51.);let w=r.floor().unwrap().width();let at=10*w+11;
    let mut beyond=r.player.clone();beyond.x=13.;
    assert!(corpse_core::vision::line_of_sight(&r,&r.player,&beyond));
    let before=corpse_core::vision::observe_terrain(&r).unwrap();
    assert_eq!(before.cells[at].tile.as_deref(),Some("speed-up"));assert!(before.cells[at].current);
    // Same-tick terrain changes must invalidate geometry; no temporal delay.
    r.floor_mut().unwrap().tiles[at]="wall".into();
    assert!(!corpse_core::vision::line_of_sight(&r,&r.player,&beyond));
    assert_eq!(corpse_core::vision::observe_terrain(&r).unwrap().cells[at].tile.as_deref(),Some("wall"));
    r.floor_mut().unwrap().tiles[at]="speed-up".into();
    assert_eq!(corpse_core::vision::observe_terrain(&r).unwrap().cells[at].tile.as_deref(),Some("speed-up"));
}

#[test]
fn revision51_black_pit_restores_speed_up_on_the_authored_three_updates() {
    let mut r=room(51.);let at=10*r.floor().unwrap().width()+11;let id=r.player.id;
    r.player.active.push("grisha-black-pit".into());
    corpse_core::divine_environment::leave_black_pit(&mut r,id,at);
    assert_eq!(r.floor().unwrap().tiles[at],"chasm");assert!(ne::exits(r.floor().unwrap(),at).is_empty());
    let saved=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&saved).unwrap();assert_eq!(storage::serialize(&loaded).unwrap(),saved);
    for i in 1..=3 {corpse_core::environment::restore_altered_chasms_revision(r.floor_mut().unwrap(),false,51.);
        assert_eq!(r.floor().unwrap().tiles[at],if i<3{"chasm"}else{"speed-up"});}
    assert_eq!(ne::exits(r.floor().unwrap(),at),vec![at+1]);
    assert!(corpse_core::travel::descent_cells(r.floor().unwrap(),11.,10.,true).contains(&at));
}

/// The R52 clarification preserves the existing ordered pairs and numbering,
/// rather than relabeling a whole area or inventing a second portal mapping.
#[test]
fn revision52_speed_up_generated_numbers_identify_adjacent_destinations() {
    for seed in ["adjacent-pairs-a", "adjacent-pairs-b", "adjacent-pairs-c"] {
        let mut old=room(51.).floor().unwrap().clone();
        old.speed_up=Field::Missing;old.speed_up_terrain=Field::Missing;
        for tile in &mut old.tiles {if tile=="speed-up" {*tile="floor".into();}}
        let mut current=old.clone();
        sub_biomes_generation::install_links(seed,&mut old,&[],&[],51.);
        sub_biomes_generation::install_links(seed,&mut current,&[],&[],52.);
        assert_eq!(current,old,"the clarification must not change seeded R51 mapping");
        assert!(ne::traversable(&current));
        let w=current.width();let mut endpoints=std::collections::HashSet::new();
        let pockets=current.speed_up.as_ref().unwrap();assert!(!pockets.is_empty());
        for pocket in pockets {
            assert_eq!(pocket.cells.len(),2*pocket.groups.len());
            let mut prior_number=None;
            for group in &pocket.groups {
                assert_eq!(group.sources.len(),1);
                let (a,b)=(group.sources[0],group.destination);
                assert_eq!((a%w).abs_diff(b%w)+(a/w).abs_diff(b/w),1);
                assert!(endpoints.insert(a)&&endpoints.insert(b),"overlapping pair");
                assert_eq!(ne::exits(&current,a),vec![b]);
                assert_eq!(ne::exits(&current,b),vec![a]);
                let number=ne::group_number(&current,a).unwrap();
                assert_eq!(ne::group_number(&current,b),Some(number));
                if let Some(prior)=prior_number {assert_eq!(number,prior+1);}
                prior_number=Some(number);
                assert_eq!(current.tiles[a],"speed-up");assert_eq!(current.tiles[b],"speed-up");
            }
        }
        let once=current.clone();
        sub_biomes_generation::install_links("repeated-install",&mut current,&[],&[],52.);
        assert_eq!(current,once,"repeated installation must not overlap or renumber");
    }
}

#[test]
fn revision52_speed_up_odd_eligible_line_leaves_unpaired_ground_unchanged() {
    let mut f=corpse_core::Floor {depth:20.,domain:"Dungeon".into(),
        tiles:vec!["wall".into();2291],lights:vec![100.;2291],seen:vec![true;2291],
        ..Default::default()};
    let w=f.width();
    // Stairs protect three cells on each end: x=5..13 is nine eligible cells.
    for x in 1..=17 {f.tiles[10*w+x]="floor".into();}
    f.tiles[10*w+1]="up".into();f.tiles[10*w+17]="down".into();
    sub_biomes_generation::install_links("odd-adjacent-line",&mut f,&[],&[],52.);
    let speed=f.speed_up.as_ref().unwrap();assert_eq!(speed.len(),1);
    assert_eq!(speed[0].cells.len(),8);assert_eq!(speed[0].groups.len(),4);
    let unpaired:Vec<_>=(5..=13).map(|x|10*w+x).filter(|&at|ne::speedup_at(&f,at).is_none()).collect();
    assert_eq!(unpaired.len(),1);assert_eq!(f.tiles[unpaired[0]],"floor");
    assert_eq!(ne::group_number(&f,unpaired[0]),None);
    assert!(ne::validate_floor(&f));assert!(ne::traversable(&f));
}

#[test]
fn revision52_speed_up_mapping_survives_repeated_replacements_save_and_replay() {
    let initial=room(52.);let w=initial.floor().unwrap().width();let at=10*w+11;
    let topology=initial.floor().unwrap().speed_up.clone();
    let number=ne::group_number(initial.floor().unwrap(),at);
    let mut restored=initial.clone();
    for replacement in ["wall","shallow-water","chasm","floor"] {
        restored.floor_mut().unwrap().tiles[at]=replacement.into();
        restored.floor_mut().unwrap().tiles[at+1]=replacement.into();
        assert!(ne::exits(restored.floor().unwrap(),at).is_empty());
        restored=storage::deserialize(&storage::serialize(&restored).unwrap()).unwrap();
        restored.floor_mut().unwrap().tiles[at]="speed-up".into();
        assert!(ne::exits(restored.floor().unwrap(),at).is_empty());
        restored.floor_mut().unwrap().tiles[at+1]="speed-up".into();
        assert_eq!(ne::exits(restored.floor().unwrap(),at),vec![at+1]);
        assert_eq!(ne::group_number(restored.floor().unwrap(),at),number);
        assert_eq!(restored.floor().unwrap().speed_up,topology);
    }
    let mut engine=corpse_core::temporal::Engine::new(initial.clone()).unwrap();
    for (dx,x) in [(1.,12.),(1.,14.),(-1.,13.),(-1.,11.)] {
        engine.dispatch(&corpse_core::Command::Move{dx,dy:0.}).unwrap();
        assert_eq!(engine.run.player.x,x);
    }
    let saved=engine.save().unwrap();
    assert_eq!(corpse_core::temporal::Engine::load(&saved).unwrap().save().unwrap(),saved);
    let record=corpse_core::replay::ReplayRecord::from_run(&engine.run);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(),saved);
    replay.restart().unwrap();while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(),saved);
    assert_eq!(replay.engine.run.floor().unwrap().speed_up,topology);
    assert_eq!(ne::group_number(replay.engine.run.floor().unwrap(),at),number);
}
