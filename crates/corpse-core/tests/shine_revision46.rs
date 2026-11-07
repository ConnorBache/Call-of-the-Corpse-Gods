//! Revision46 whole-source Shine suppression. Legacy source fixtures remain separate.
use corpse_core::{Actor, Command, Field, Floor, FloorLightSourcesEntry, Run, abilities,
    dreamwalking, engine, half_here, movement::Point, vision};
use std::sync::Arc;
const SHINE: &str = "anavani-shine";

/// Fixed O(V+A) controlled lighting room; no generation or gameplay RNG.
fn room(revision: f64) -> Run {
    let mut r = Run { depth: 1., status: "playing".into(), next_id: 1000.,
        simulation_revision: revision.into(), ..Default::default() };
    r.player = Actor { id: 0., x: 4., y: 4., hp: 100., max_hp: 100.,
        abilities: vec![SHINE.into()], ..Default::default() };
    r.player.stats.perception = 100.;
    r.floors.push(Arc::new(Floor { depth: 1., width: 15.0.into(), height: 15.0.into(),
        tiles: vec!["floor".into();225], lights: vec![-50.;225], seen: vec![false;225],
        ..Default::default() }));
    r.actors_at_mut(1.).push(Actor { id: 1., x: 5., y: 4., hp: 100., max_hp: 100.,
        state: "hunting".into(), ..Default::default() });
    r
}
fn suppress(r: &mut Run) { r.player.active.push(SHINE.into()); }
fn fixture_light(r: &mut Run, power: f64) {
    r.floor_mut().unwrap().light_sources = vec![FloorLightSourcesEntry {
        x: 4., y: 4., power, falloff: 10., kind: "torch".to_string().into(),
    }].into();
}
/// O(V*(scalar light + scalar/projected visibility)); independent public paths.
fn compare_projection(r: &Run) {
    let field = r.lighting_cache.field(r, dreamwalking::context_dream(r)).unwrap();
    let f = r.floor().unwrap();
    for y in 0..f.height() { for x in 0..f.width() {
        assert_eq!(field.at(Point {x:x as i32,y:y as i32}).to_bits(),
            vision::tile_light(r,x,y).to_bits(), "cell={x},{y}");
    }}
    let projection = vision::VisibilityProjection::new(r);
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        assert_eq!(projection.visible_actor(a), vision::visible_actor(r,a));
        assert_eq!(projection.directly_visible_actor(a), vision::directly_visible_actor(r,a));
    }
}

#[test]
fn missing_and_all_legacy_revisions_keep_self_only_exclusion() {
    for revision in 0..=45 {
        let mut r=room(revision as f64);
        if revision==0 { r.simulation_revision=Field::Missing; }
        suppress(&mut r);
        assert_eq!(vision::tile_light(&r,4,4),100.);
        assert_eq!(vision::light(&r,&r.player),-50.);
        assert_eq!(vision::light(&r,&r.actors_at(1.)[0]),90.);
        let field=r.lighting_cache.field(&r,false).unwrap();
        assert_eq!(field.at(Point{x:4,y:4}),100.);
        assert_eq!(field.at_excluding(Point{x:4,y:4},0.),-50.);
        assert!(vision::observe_terrain(&r).unwrap().light_sources.iter().any(|s|s.kind=="shine"));
    }
}

#[test]
fn suppression_removes_self_terrain_other_actor_and_appearance_emission() {
    for revision in [46., 51.] {
    let mut r=room(revision);
    assert_eq!(vision::light(&r,&r.player),100.);
    assert_eq!(vision::light(&r,&r.actors_at(1.)[0]),90.);
    compare_projection(&r);
    suppress(&mut r);
    assert_eq!(vision::light(&r,&r.player),-50.);
    assert_eq!(vision::light(&r,&r.actors_at(1.)[0]),-50.);
    assert_eq!(vision::tile_light(&r,5,4),-50.);
    assert!(vision::observe_terrain(&r).unwrap().light_sources.iter().all(|s|s.kind!="shine"));
    compare_projection(&r);
    r.player.active.clear();
    assert_eq!(vision::tile_light(&r,5,4),90.);
    assert!(vision::observe_terrain(&r).unwrap().light_sources.iter().any(|s|s.kind=="shine"));
    }
}

#[test]
fn overlapping_sources_and_darkness_survive_independent_suppression() {
    for revision in [46., 51.] {
    let mut r=room(revision); suppress(&mut r); fixture_light(&mut r,70.);
    assert_eq!(vision::light(&r,&r.player),70.);
    assert_eq!(vision::light(&r,&r.actors_at(1.)[0]),60.);
    let mut other=r.player.clone();other.id=2.;other.x=6.;other.active.clear();
    r.actors_at_mut(1.).push(other);
    assert_eq!(vision::tile_light(&r,5,4),90.);
    assert_eq!(vision::light(&r,&r.player),80.);
    r.actors_at_mut(1.)[1].active.push(SHINE.into());
    assert_eq!(vision::tile_light(&r,5,4),60.);
    let mut fire=corpse_core::EnvironmentalState::default(); fire.fire=vec![0.;225];fire.fire[4*15+4]=1.;
    r.floor_mut().unwrap().environment=fire.into();
    assert_eq!(vision::tile_light(&r,5,4),70.);
    r.actors_at_mut(1.).push(Actor{id:3.,x:7.,y:4.,hp:10.,
        abilities:vec!["enemy:beast:darkness-aura".into()],..Default::default()});
    assert_eq!(vision::tile_light(&r,5,4),-10.);
    compare_projection(&r);
    let mut no_shine=r.clone();no_shine.player.abilities.clear();no_shine.actors_at_mut(1.)[1].abilities.clear();
    for y in 0..15 {for x in 0..15 {assert_eq!(vision::tile_light(&r,x,y).to_bits(),vision::tile_light(&no_shine,x,y).to_bits());}}
    }
}

#[test]
fn direct_same_tick_toggles_revision_switches_and_nonemitter_moves_rebuild_exactly() {
    let mut r=room(45.);suppress(&mut r);
    let legacy=r.lighting_cache.field(&r,false).unwrap();
    r.simulation_revision=46.0.into();
    let suppressed=r.lighting_cache.field(&r,false).unwrap();
    assert!(!Arc::ptr_eq(&legacy,&suppressed));assert_eq!(suppressed.at(Point{x:4,y:4}),-50.);
    r.player.x=8.;r.player.id=50.;
    assert!(Arc::ptr_eq(&suppressed,&r.lighting_cache.field(&r,false).unwrap()),"suppressed source has no field dependencies");
    r.player.active.clear();
    let resumed=r.lighting_cache.field(&r,false).unwrap();
    assert!(!Arc::ptr_eq(&suppressed,&resumed));assert_eq!(resumed.at(Point{x:8,y:4}),100.);
    assert_eq!(resumed.at_excluding(Point{x:8,y:4},50.),100.);
    r.player.dead=true.into();
    let dead=r.lighting_cache.field(&r,false).unwrap();assert_eq!(dead.at(Point{x:8,y:4}),-50.);
    r.player.dead=false.into();r.player.conditions.insert("phantasmiiBody".into(),1.);
    assert!(Arc::ptr_eq(&dead,&r.lighting_cache.field(&r,false).unwrap()));
    r.player.conditions.clear();r.player.abilities.clear();
    assert!(Arc::ptr_eq(&dead,&r.lighting_cache.field(&r,false).unwrap()));
    r.player.abilities.push(SHINE.into());compare_projection(&r);
    let before=serde_json::to_string(&r).unwrap();
    let clone=r.clone();compare_projection(&clone);
    let loaded:Run=serde_json::from_str(&before).unwrap();compare_projection(&loaded);
    assert_eq!(serde_json::to_string(&r).unwrap(),before);
}

#[test]
fn dream_planes_and_sleeping_body_never_reintroduce_suppressed_light() {
    for revision in [46., 51.] {
    let mut r=room(revision);suppress(&mut r);dreamwalking::ensure_layer(&mut r);
    r.player.conditions.insert("phantasmiiPlane".into(),1.);
    let mut body=r.player.clone();body.id=2.;body.conditions.clear();body.conditions.insert("phantasmiiBody".into(),1.);body.active.clear();
    r.actors_at_mut(1.).push(body);
    for dream in [false,true] {dreamwalking::with_plane(&mut r,dream,|r| {
        assert_eq!(vision::tile_light(r,4,4),-50.);compare_projection(r);
    });}
    r.player.active.clear();
    dreamwalking::with_plane(&mut r,false,|r|assert_eq!(vision::tile_light(r,4,4),-50.));
    dreamwalking::with_plane(&mut r,true,|r| {assert_eq!(vision::tile_light(r,4,4),100.);compare_projection(r);});
    }
}

#[test]
fn hidden_suppressed_sources_leak_neither_origin_footprint_nor_visibility_through_portals() {
    for revision in [46., 51.] {
    let mut r=room(revision);suppress(&mut r);r.player.abilities.clear();
    r.floor_mut().unwrap().lights.fill(-100.);fixture_light(&mut r,70.);
    for y in 0..15 {r.floor_mut().unwrap().tiles[y*15+7]="wall".into();}
    r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:4*15+6,b:4*15+10,color_index:0}].into();
    let mut hidden=r.player.clone();hidden.id=2.;hidden.x=13.;hidden.y=13.;hidden.abilities=vec![SHINE.into()];
    r.actors_at_mut(1.).push(hidden);
    let before=format!("{:?}",vision::observe_terrain(&r).unwrap());
    assert!(!vision::visible_actor(&r,&r.actors_at(1.)[1]));
    r.actors_at_mut(1.)[1].x=12.;r.actors_at_mut(1.)[1].y=12.;
    assert_eq!(format!("{:?}",vision::observe_terrain(&r).unwrap()),before);
    assert!(vision::observe_terrain(&r).unwrap().light_sources.iter().all(|s|s.kind!="shine"));
    compare_projection(&r);
    }
}

/// Admitted fixed geometry for command, persistence and attack tests.
fn command_room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.depth=1.;r.tick=0.;r.status="playing".into();r.rng=123456789.;r.next_id=1000.;
    r.floors.truncate(1);r.actors.clear();r.ground.clear();r.actors_at_mut(1.).clear();r.ground_at_mut(1.).clear();r.inventory.clear();r.offers.clear();r.history.clear();r.replay.clear();r.future=None;
    r.phase=Field::Missing;r.haste_snapshot=Field::Missing;r.resource_history=Field::Missing;r.replay_snapshot_from=Field::Missing;
    r.log.clear();r.log_kinds=Field::Missing;r.log_archive=Field::Missing;
    r.player.abilities=vec![SHINE.into()];r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.x=8.;r.player.y=8.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();r.player.stats.max_hp=Field::Missing;
    r.player.energy=100.;r.player.stats.max_energy=100.;r.player.stats.perception=100.;r.player.stats.accuracy=10.;r.player.stats.offense=20.;r.player.stats.dr=10000.;
    let f=r.floor_mut().unwrap();
    f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.godbeast_event=Field::Missing;f.godbeast=false;
    f.doors=Field::Missing;f.light_sources=Field::Missing;f.blood=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;
    f.razorleaves=Field::Missing;f.grates=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;
    let(w,h)=(f.width(),f.height());for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();f.lights.fill(-50.);r
}
fn toggle()->Command{ability(SHINE)}
fn ability(id:&str)->Command{Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}

#[test]
fn zero_energy_toggle_is_free_in_normal_and_haste_and_shared_body_resumes() {
    for revision in [46., 51.] {
    for haste in [false,true] {
        let mut r=command_room(revision);r.player.energy=0.;r.player.abilities.push(half_here::ID.into());
        let clone_id=half_here::create(&mut r,11.,8.).unwrap();
        if haste {r.phase="haste".to_string().into();r.player.conditions.insert("hasted".into(),3.);}
        let(tick,rng,energy,phase)=(r.tick,r.rng,r.player.energy,r.phase.clone());
        for expected in [-50.,100.] {
            let out=engine::dispatch(&mut r,&toggle()).unwrap();assert!(!out.took_turn);
            assert_eq!((r.tick,r.rng,r.player.energy,r.phase.clone()),(tick,rng,energy,phase.clone()));
            assert_eq!(vision::light(&r,engine::actor(&r,clone_id).unwrap()),expected);
            assert_eq!(vision::tile_light(&r,11,8),expected);
            abilities::pay_upkeep(&mut r,0.).unwrap();assert_eq!(r.player.energy,0.);
        }
    }
    }
}

#[test]
fn active_shine_accuracy_still_uses_external_target_light_for_melee_and_ranged() {
    let(mut hits,mut misses)=(0,0);
    for ranged in [false,true] {for ambient in [-100.,-20.,0.,40.] {for seed in [1.,12345.,123456789.,3735928559.] {
        let mut r=command_room(46.);suppress(&mut r);r.floor_mut().unwrap().lights.fill(ambient);r.rng=seed;
        let mut a=corpse_core::population::create_actor_revision("rat",1.,if ranged {11.} else {9.},8.,false,45.).unwrap();
        a.hp=1000.;a.max_hp=1000.;a.base_max_hp=1000.0.into();a.stats.max_hp=Field::Missing;
        a.state="hunting".into();a.abilities.clear();a.active.clear();a.conditions.clear();a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
        a.stats.dodge=10.;a.stats.dr=0.;r.actors_at_mut(1.).push(a);
        assert_eq!(vision::light(&r,&r.actors_at(1.)[0]),ambient);
        let mut control=r.clone();control.player.abilities.clear();control.player.active.clear();
        control.player.stats.accuracy+=corpse_core::combat::integer_stat(ambient);
        for run in [&mut r,&mut control] {
            if ranged {engine::attack_ranged(run,0.,1.,20.,3.,1.,false).unwrap();}
            else {engine::attack(run,0.,1.,1.).unwrap();}
        }
        assert_eq!(r.rng,control.rng);assert_eq!(r.actors_at(1.),control.actors_at(1.));
        if r.actors_at(1.)[0].hp<1000. {hits+=1;} else {misses+=1;}
    }}}
    assert!(hits>0 && misses>0,"deterministic accuracy scenarios must cover both results");
}

#[test]
fn save_load_snapshot_future_timeweave_and_replay_keep_suppression_and_revision() {
    use corpse_core::{temporal::{Engine,Snapshot},replay::{ReplayRecord,ReplaySession}};
    for revision in [45.,46.] {
        let mut r=command_room(revision);r.player.abilities.extend(["morae-see-the-future".into(),corpse_core::progression::WEAVE_ID.into()]);
        let initial=r.clone();let snapshot=Snapshot::capture(&r);let mut e=Engine::new(r).unwrap();
        let future=ability("morae-see-the-future");
        let commands=[toggle(),Command::Wait,future.clone(),toggle(),Command::Move{dx:1.,dy:0.},Command::Move{dx:-1.,dy:0.},future,toggle(),toggle()];
        for c in &commands {
            e.dispatch(c).unwrap();compare_projection(&e.run);
            let saved=e.save().unwrap();let loaded=Engine::load(&saved).unwrap();assert_eq!(loaded.save().unwrap(),saved);
            assert_eq!(vision::tile_light(&loaded.run,8,8),vision::tile_light(&e.run,8,8));e=loaded;
        }
        assert!(e.run.player.active.iter().any(|s|s==SHINE));
        assert_eq!(vision::tile_light(&e.run,8,8),if revision>=46. {-50.} else {100.});
        let saved=e.save().unwrap();let record=ReplayRecord::try_from_run(&e.run).unwrap();
        let mut replay=ReplaySession::from_initial(initial.clone(),record).unwrap();while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),saved);replay.restart().unwrap();while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),saved);
        snapshot.restore(&mut e.run).unwrap();assert_eq!(vision::tile_light(&e.run,8,8),100.);
        assert_eq!(e.run.simulation_revision,initial.simulation_revision);
    }
}

#[test]
fn offscreen_independent_shine_refusal_is_unchanged_and_loaded_floor_queries_are_revision_aware() {
    for revision in [45.,46.] {
        let mut r=command_room(revision);r.player.abilities.clear();
        let mut floor=r.floor().unwrap().clone();floor.depth=2.;floor.dirty=true;floor.last_tick=0.;floor.seen.fill(true);
        r.floors.push(Arc::new(floor));r.ground_at_mut(2.).clear();
        let mut a=corpse_core::population::create_actor_revision("rat",1.,9.,8.,false,45.).unwrap();
        a.state="sleeping".into();a.abilities=vec![SHINE.into()];a.active=a.abilities.clone();a.conditions.clear();
        a.conditions.insert("bestiaryTraitsInitialized".into(),1.);a.energy=0.;
        r.actors_at_mut(2.).push(a);r.tick=2.;
        let before=serde_json::to_string(&r).unwrap();
        // The outer offscreen allowlist lists Shine, but its proxy environment
        // admission still rejects independent carriers. Do not broaden that gate.
        assert_eq!(corpse_core::offscreen::validate_supported(&r).unwrap_err(),
            "Unsupported environment integration: living burning/emitter/ability traits");
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
        assert_eq!(vision::tile_light(&r,9,8),-50.,"inactive floor contributes no light here");
        // Read-only loaded-floor lighting, not a claim of admitted offscreen execution.
        let mut loaded:Run=serde_json::from_str(&before).unwrap();loaded.depth=2.;
        assert_eq!(vision::tile_light(&loaded,9,8),if revision>=46. {-50.} else {100.});
        compare_projection(&loaded);
        loaded.actors_at_mut(2.)[0].active.clear();
        assert_eq!(vision::tile_light(&loaded,9,8),100.);compare_projection(&loaded);
    }
}

/// Opt-in bounded diagnostics, never simulation state or a flaky wall-time gate.
/// 225 cells, 33 Shine carriers; exact outputs checked outside measured regions.
#[test]
fn bounded_shine_lighting_timings() {
    if std::env::var_os("CORPSE_SHINE_TIMING").is_none() {return;}
    use std::{hint::black_box,time::Instant};
    for revision in [45.,46.] {for active in [false,true] {
        let mut r=room(revision);
        if active {suppress(&mut r);}
        for i in 2..=33 {
            let mut a=r.player.clone();a.id=i as f64;a.x=((i*3)%15) as f64;a.y=((i*7)%15) as f64;
            r.actors_at_mut(1.).push(a);
        }
        compare_projection(&r);
        let before=serde_json::to_string(&r).unwrap();
        let start=Instant::now();
        for _ in 0..16 {for y in 0..15 {for x in 0..15 {black_box(vision::tile_light(&r,x,y));}}}
        let scalar=start.elapsed();
        let start=Instant::now();for _ in 0..16 {
            let cold=r.clone();black_box(cold.lighting_cache.field(&cold,false).unwrap());
        }let cold=start.elapsed();
        let start=Instant::now();for _ in 0..128 {black_box(r.lighting_cache.field(&r,false).unwrap());}
        let warm=start.elapsed();
        let start=Instant::now();for _ in 0..8 {black_box(vision::observe_terrain(&r).unwrap());}
        let observe=start.elapsed();
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
        eprintln!("Shine r{revision} active={active}: scalar3600={scalar:?}; cold16={cold:?}; warm128={warm:?}; observe8={observe:?}");
    }}
}
