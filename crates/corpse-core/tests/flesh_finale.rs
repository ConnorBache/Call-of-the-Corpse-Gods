use corpse_core::{Command, Field, Floor, Run, finale, storage, temporal::{Engine, Snapshot}};
use std::sync::Arc;

fn world() -> Run {
    let mut r = corpse_core::startup::new_run_shell("finale-fixture", "Unblessed", false).unwrap();
    r.depth = 40.;
    r.player.x = 1.; r.player.y = 1.;
    for depth in 1..=40 {
        let mut f = Floor { depth: depth as f64, domain: "Dungeon".into(),
            tiles: vec!["floor".into(); 2291], lights: vec![1.;2291], seen:vec![true;2291], ..Default::default() };
        for y in 0..29 { for x in 0..79 { if x==0 || y==0 || x==78 || y==28 { f.tiles[y*79+x]="wall".into(); } } }
        f.tiles[80]="up".into();
        if depth <40 { f.tiles[560]="down".into(); }
        if depth==1 { f.tiles[80]="floor".into(); f.tiles[2172]="up".into(); f.entrance=corpse_core::entrance::Entrance{x:39.,y:27.}.into(); }
        r.floors.push(Arc::new(f));
        r.actors.insert(depth.to_string(),Arc::new(vec![]));
        r.ground.insert(depth.to_string(),Arc::new(vec![]));
    }
    r.actors_at_mut(40.).push(finale::administrator(4.,4.,4.));
    r.next_id=5.; r
}
fn kill(r: &mut Run) {
    let a=r.actors_at_mut(r.depth).iter_mut().find(|a| finale::is_boss(a)).unwrap();
    let id=a.id;a.hp=0.;
    corpse_core::engine::resolve_death(r,id,Some(r.player.id),None,true).unwrap();
}
fn pulse(r: &mut Run,tick:f64) {r.tick=tick;finale::world_turn(r).unwrap();}
fn count(r: &Run,depth:u32)->usize {r.collapse.as_ref().unwrap().waves.iter().find(|w|w.depth==depth).map_or(0,|w|w.cells.iter().filter(|x|**x).count())}
fn ability(id: &str)->Command {Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
fn review(r: &Run,name:&str) {
    if let Ok(dir)=std::env::var("CORPSE_FINALE_GAME_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(name),storage::serialize(r).unwrap()).unwrap();
    }
}

#[test]
fn actual_death_seeds_real_cell_once_without_rng_corpse_or_heart() {
    let mut r=world(); let rng=r.rng; let next=r.next_id;
    let boss=r.actors_at(40.)[0].clone();
    assert!(!finale::actual_death(&mut r,&boss).unwrap());
    r.actors_at_mut(40.)[0].x=5.;r.actors_at_mut(40.)[0].y=4.;
    kill(&mut r);
    assert_eq!(r.rng,rng);assert_eq!(r.next_id,next);assert_eq!(count(&r,40),1);
    assert_eq!(r.floor().unwrap().tiles[321],"chasm");
    assert!(r.actors_at(40.).is_empty());assert!(r.ground["40"].is_empty());
    let old=r.collapse.clone();assert!(!finale::actual_death(&mut r,&boss).unwrap());assert_eq!(r.collapse,old);
    assert_eq!(r.collapse.as_ref().unwrap().next_pulse_tick,2.);
}
#[test]
fn parity_simultaneous_eight_neighbor_layers_no_rng_and_walls_erode() {
    let mut r=world();review(&r,"administrator-visible.json");kill(&mut r);let rng=r.rng;
    Arc::make_mut(&mut r.floors[39]).tiles[241]="wall".into();
    pulse(&mut r,1.);assert_eq!(count(&r,40),1);
    pulse(&mut r,2.);review(&r,"collapse-layer-one.json");assert_eq!(count(&r,40),9);assert_eq!(r.floor().unwrap().tiles[241],"chasm");
    pulse(&mut r,3.);assert_eq!(count(&r,40),9);
    pulse(&mut r,4.);assert_eq!(count(&r,40),25);assert_eq!(r.rng,rng);
}
#[test]
fn stairs_seed_only_the_reached_upper_floor_and_progress_offscreen() {
    let mut r=world();kill(&mut r);r.depth=20.;r.player.x=4.;r.player.y=4.;
    pulse(&mut r,4.);assert_eq!(count(&r,40),25);assert_eq!(count(&r,39),0);
    pulse(&mut r,6.);assert_eq!(count(&r,40),49);assert_eq!(count(&r,39),1);assert_eq!(count(&r,38),0);
    assert!(finale::collapsed(&r,39.,560));assert_eq!(r.floors[38].tiles[560],"chasm");
    pulse(&mut r,8.);assert_eq!(count(&r,39),9);assert_eq!(r.depth,20.);
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
}
#[test]
fn collapse_fall_terminal_even_on_floor40_and_preserves_flight_until_falling() {
    let mut r=world();kill(&mut r);r.player.x=4.;r.player.y=4.;
    r.player.conditions.insert("flying".into(),5.);pulse(&mut r,2.);
    assert_eq!(r.status,"playing");assert_eq!(r.player.conditions["flying"],5.);
    r.player.conditions.shift_remove("flying");
    corpse_core::travel::fall_actors_one_level(&mut r,40.,&[0.],None).unwrap();
    assert_eq!(r.status,"dead");assert_eq!(r.depth,40.);
    assert_eq!(r.run_stats.as_ref().unwrap().death.as_ref().unwrap().reason,finale::FALL_MESSAGE);
    assert_eq!(r.log.last().unwrap(),finale::FALL_MESSAGE);
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
}
#[test]
fn explicit_dive_while_flying_is_still_a_fall_and_ordinary_gap_is_unchanged() {
    let mut r=world();kill(&mut r);r.player.x=4.;r.player.y=4.;r.player.conditions.insert("flying".into(),10.);
    assert!(corpse_core::travel::travel(&mut r,1.,true,false).unwrap());assert_eq!(r.status,"dead");
    let mut ordinary=world();ordinary.depth=1.;ordinary.player.x=3.;ordinary.player.y=3.;
    Arc::make_mut(&mut ordinary.floors[0]).tiles[240]="chasm".into();
    assert!(corpse_core::travel::travel(&mut ordinary,1.,true,false).unwrap());
    assert_eq!(ordinary.depth,2.);assert_eq!(ordinary.status,"playing");
}
#[test]
fn lost_flight_npcs_are_removed_between_pulses_on_active_and_hidden_floors() {
    for active in [true,false] {
        let mut r=world();kill(&mut r);
        let mut a=corpse_core::population::create_actor("rat",6.,4.,4.,false).unwrap();
        a.conditions.insert("flying".into(),3.);r.actors_at_mut(40.).push(a);
        corpse_core::travel::fall_actors_one_level(&mut r,40.,&[6.],None).unwrap();
        assert_eq!(r.actors_at(40.).len(),1);
        r.actors_at_mut(40.)[0].conditions.shift_remove("flying");
        if !active {r.depth=20.;}
        let rng=r.rng;corpse_core::travel::fall_actors_one_level(&mut r,40.,&[6.],None).unwrap();
        assert!(r.actors_at(40.).is_empty());assert_eq!(r.rng,rng);assert!(r.ground["40"].is_empty());
    }
}
#[test]
fn collapse_removes_fixtures_objects_and_corpses_without_rewards() {
    let mut r=world();let f=Arc::make_mut(&mut r.floors[39]);
    f.doors=indexmap::IndexMap::from([("241".into(),corpse_core::FloorDoorsEntry::default())]).into();
    f.grates=vec![241.].into();f.permanent_fire=vec![241.].into();
    f.altered_chasms=indexmap::IndexMap::from([("241".into(),corpse_core::FloorAlteredChasmsEntry{tile:"floor".into(),age:0.})]).into();
    let mut dead=corpse_core::population::create_actor("rat",6.,4.,3.,false).unwrap();dead.hp=0.;dead.dead=true.into();
    r.actors_at_mut(40.).push(dead);
    r.ground_at_mut(40.).push(corpse_core::GroundItem{x:4.,y:3.,item:corpse_core::Item{id:7.,name:"test".into(),kind:"potion".into(),..Default::default()},..Default::default()});
    kill(&mut r);pulse(&mut r,2.);let f=r.floor().unwrap();
    assert!(f.doors.as_ref().unwrap().is_empty());assert!(f.grates.as_ref().unwrap().is_empty());
    assert!(f.altered_chasms.as_ref().unwrap().is_empty());assert!(f.permanent_fire.as_ref().unwrap().is_empty());
    assert!(r.ground["40"].is_empty());assert!(r.actors_at(40.).is_empty());
}
#[test]
fn only_defeated_boss_and_intact_floor_one_entrance_win() {
    let mut r=world();r.depth=1.;r.player.x=39.;r.player.y=27.;assert!(!finale::try_escape(&mut r));
    r.depth=40.;kill(&mut r);r.depth=1.;r.player.x=38.;assert!(!finale::try_escape(&mut r));
    r.player.x=39.;assert!(finale::try_escape(&mut r));assert_eq!(r.status,"won");
    assert_eq!(finale::ending(&r),Some(finale::WIN_MESSAGE));assert_eq!(r.log.last().unwrap(),finale::WIN_MESSAGE);
    let loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();assert_eq!(loaded.collapse,r.collapse);
}
#[test]
fn moving_into_entrance_wins_without_extra_world_turn() {
    let mut r=world();kill(&mut r);r.depth=1.;r.player.x=39.;r.player.y=26.;
    let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Move{dx:0.,dy:1.}).unwrap();
    assert_eq!(e.run.status,"won");assert_eq!(e.run.tick,0.);
}
#[test]
fn actual_haste_action_does_not_advance_collapse_clock() {
    let mut r=world();kill(&mut r);r.player.conditions.insert("hasted".into(),20.);
    r.player.x=1.;r.player.y=2.;let mut e=Engine::new(r).unwrap();
    e.dispatch(&Command::Move{dx:0.,dy:1.}).unwrap();assert_eq!(e.run.tick,0.);assert_eq!(count(&e.run,40),1);
    e.dispatch(&Command::Wait).unwrap();assert_eq!(e.run.tick,1.);assert_eq!(count(&e.run,40),1);
    e.dispatch(&Command::Move{dx:0.,dy:1.}).unwrap();assert_eq!(e.run.tick,1.);assert_eq!(count(&e.run,40),1);
    e.dispatch(&Command::Wait).unwrap();assert_eq!(e.run.tick,2.);assert_eq!(count(&e.run,40),9);
}
#[test]
fn save_replay_restart_and_both_time_abilities_restore_frontiers_and_boss() {
    let mut r=world();r.player.x=3.;r.player.y=4.;r.actors_at_mut(40.)[0].hp=1.;
    r.player.stats.accuracy=200.;
    for id in ["numo-revert","morae-see-the-future"] {r.player.abilities.push(id.into());corpse_core::progression::initialize_ability_state(&mut r.player,id,0.).unwrap();}
    r.player.conditions.insert("revertCharges".into(),3.);
    let initial=r.clone();let mut e=Engine::new(r).unwrap();
    e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();assert!(e.run.collapse.as_ref().is_some());
    e.dispatch(&Command::Move{dx:-1.,dy:0.}).unwrap();
    let saved=e.save().unwrap();let loaded=Engine::load(&saved).unwrap();assert_eq!(loaded.run.collapse,e.run.collapse);
    let record=corpse_core::replay::ReplayRecord::from_run(&e.run);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial.clone(),record).unwrap();
    while replay.step().unwrap() {}assert_eq!(replay.engine.save().unwrap(),saved);
    replay.restart().unwrap();assert!(replay.engine.run.collapse.is_missing());assert!(replay.engine.run.actors_at(40.).iter().any(finale::is_boss));
    e.dispatch(&ability("numo-revert")).unwrap();assert_eq!(count(&e.run,40),1);
    e.dispatch(&ability("numo-revert")).unwrap();assert!(e.run.collapse.is_missing());assert!(e.run.actors_at(40.).iter().any(finale::is_boss));
    e.dispatch(&ability("morae-see-the-future")).unwrap();
    e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();assert!(e.run.collapse.as_ref().is_some());
    let mut e=Engine::load(&e.save().unwrap()).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(e.run.collapse.is_missing());assert!(e.run.actors_at(40.).iter().any(finale::is_boss));
    let snap=Snapshot::capture(&initial);snap.restore(&mut e.run).unwrap();assert!(e.run.collapse.is_missing());
}
#[test]
fn new_floor40_has_exactly_one_reachable_boss_without_extra_ordinary_budget() {
    for seed in ["finale-spawn-a","finale-spawn-b","finale-spawn-c"] {
        let mut r=corpse_core::startup::create_run(seed,"Unblessed",false).unwrap();
        corpse_core::startup::ensure_floors(&mut r,40).unwrap();
        let bosses:Vec<_>=r.actors.values().flat_map(|v|v.iter()).filter(|a|finale::is_boss(a)).collect();
        assert_eq!(bosses.len(),1);let a=bosses[0];let f=&r.floors[39];
        assert_eq!(a.name,finale::BOSS_NAME);assert!(a.abilities.is_empty());assert!(a.heartless.value_or(false));
        assert!(!r.actors_at(40.).iter().any(|b|b.id!=a.id && b.x==a.x && b.y==a.y));
        assert!(!r.ground["40"].iter().any(|g|g.x==a.x && g.y==a.y));
        assert_eq!(a.max_hp,100.+6.*a.stats.defense);assert!(!f.tiles.iter().any(|t|t=="pit"));
        let up=f.tiles.iter().position(|t|t=="up").unwrap();let field=corpse_core::movement::distance_field(&f.tiles,f.width(),corpse_core::movement::Point{x:(up%f.width()) as i32,y:(up/f.width()) as i32},false,false,None,None);
        assert!(field.steps[a.y as usize*f.width()+a.x as usize]>=3);
        let ordinary:Vec<_>=r.actors_at(40.).iter().filter(|a|!a.heartless.value_or(false) && !a.creature_kind.as_ref().is_some_and(|k|k=="zillennia")).collect();
        let reward:f64=ordinary.iter().map(|a|corpse_core::inventory::heart_for(a,0.).unwrap().exp.value_or(0.)).sum();
        assert_eq!(reward,f.enemy_blessing_budget.value_or(0.));
        storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    }
}
#[test]
fn older_revisions_keep_pit_and_do_not_spawn_administrator() {
    for rev in [29.,30.] {
        let mut r=corpse_core::startup::create_run_revision("finale-legacy","Unblessed",false,rev).unwrap();
        corpse_core::startup::ensure_floors(&mut r,40).unwrap();
        assert!(r.floors[39].tiles.iter().any(|t|t=="pit"));
        assert!(!r.actors.values().flat_map(|v|v.iter()).any(finale::is_boss));assert!(r.collapse.is_missing());
    }
}
#[test]
fn playtest_floor40_keeps_boss_and_unranked_origin() {
    let r=corpse_core::playtest::create_run("0-5-11:finale-playtest","Avatar",40).unwrap();
    assert_eq!(r.actors_at(40.).iter().filter(|a|finale::is_boss(a)).count(),1);
    assert_eq!(corpse_core::profile::summarize_run(&r).score,None);
    Engine::new(r).unwrap();
}

#[test]
fn death_on_upstairs_seeds_above_immediately_without_extra_layer() {
    let mut r=world();r.actors_at_mut(40.)[0].x=1.;r.actors_at_mut(40.)[0].y=1.;
    r.player.x=4.;r.player.y=4.;kill(&mut r);
    assert_eq!(count(&r,40),1);assert_eq!(count(&r,39),1);assert_eq!(count(&r,38),0);
    assert!(finale::collapsed(&r,39.,560));
}
#[test]
fn hidden_boss_death_keeps_global_collapse_and_announces_it_once() {
    let mut r=world();r.depth=39.;r.player.x=2.;r.player.y=2.;
    let a=&mut r.actors_at_mut(40.)[0];a.hp=1.;a.conditions.insert("bleeding".into(),20.);
    // Dirty hidden floor follows the existing precise bleeding/environment scheduler.
    Arc::make_mut(&mut r.floors[39]).dirty=true;
    let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Wait).unwrap();
    assert!(e.run.collapse.as_ref().is_some());assert!(!e.run.actors_at(40.).iter().any(finale::is_boss));
    assert_eq!(e.run.log.iter().filter(|s|s.contains("Escape to the blue door!")).count(),1);
    e.dispatch(&Command::Wait).unwrap();e.dispatch(&Command::Wait).unwrap();assert_eq!(count(&e.run,40),9);
    assert_eq!(e.run.log.iter().filter(|s|s.contains("Escape to the blue door!")).count(),1);
}
#[test]
fn half_here_endpoint_falls_out_without_killing_airborne_owner() {
    let mut r=world();kill(&mut r);
    let mut clone=r.player.clone();clone.id=8.;clone.name="Half Here".into();clone.x=4.;clone.y=4.;
    clone.conditions.insert("halfHereClone".into(),1.);clone.conditions.insert("halfHereOwner".into(),r.player.id);
    r.actors_at_mut(40.).push(clone);
    corpse_core::travel::fall_actors_one_level(&mut r,40.,&[8.],None).unwrap();
    assert_eq!(r.status,"playing");assert!(r.actors_at(40.).is_empty());
}
#[test]
fn object_fallback_to_collapse_gap_is_discarded_not_transferred() {
    let mut r=world();let boss=r.actors_at_mut(40.).pop().unwrap();r.actors_at_mut(39.).push(boss);r.depth=39.;kill(&mut r);let f=Arc::make_mut(&mut r.floors[38]);
    for tile in &mut f.tiles {*tile="wall".into();}f.tiles[320]="chasm".into();
    let entry=corpse_core::GroundItem{x:3.,y:4.,item:corpse_core::Item{id:8.,name:"test".into(),kind:"potion".into(),..Default::default()},..Default::default()};
    let result=corpse_core::item_placement::place_ground_item(&mut r,39.,entry,|_,_|Ok(())).unwrap();
    assert!(matches!(result,corpse_core::item_placement::PlacementResult::Discarded{..}));assert!(r.ground["40"].is_empty());
}
#[test]
fn complete_physical_ascent_from_forty_reaches_the_door_before_wave() {
    let mut r=world();r.player.x=3.;r.player.y=4.;kill(&mut r);
    let mut e=Engine::new(r).unwrap();let mut steps=0;
    while e.run.status=="playing" && steps<400 {
        let f=e.run.floor().unwrap();let up=f.tiles.iter().position(|t|t=="up").expect("escape stair remains ahead");
        let sign=|v:f64|if v==0. {0.} else {v.signum()};
        let dx=sign((up%f.width()) as f64-e.run.player.x);
        let dy=sign((up/f.width()) as f64-e.run.player.y);
        e.dispatch(&Command::Move{dx,dy}).unwrap();steps+=1;
    }
    assert_eq!(e.run.status,"won", "steps={steps} depth={} tick={} pos=({}, {}) log={:?}",e.run.depth,e.run.tick,e.run.player.x,e.run.player.y,e.run.log);assert_eq!(e.run.depth,1.);
    assert_eq!(finale::ending(&e.run),Some(finale::WIN_MESSAGE));
    assert!(e.run.collapse.as_ref().unwrap().waves.len()>1);
    Engine::load(&e.save().unwrap()).unwrap();
}

#[test]
fn revision52_cardinal_first_then_diagonal_inclusive_is_solid_and_serialized() {
    let mut r=world(); r.simulation_revision=52.0.into(); kill(&mut r);
    let rng=r.rng;
    pulse(&mut r,2.);
    assert_eq!(count(&r,40),5);
    assert!(finale::collapsed(&r,40.,4*79+5));
    assert!(!finale::collapsed(&r,40.,5*79+5));
    assert!(r.collapse.as_ref().unwrap().waves[0].diagonal_next);
    let snapshot=Snapshot::capture(&r);
    let mut restored=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    pulse(&mut r,4.); pulse(&mut restored,4.);
    assert_eq!(r.collapse,restored.collapse);
    assert_eq!(count(&r,40),21);
    for y in 3..=5 {for x in 3..=5 {assert!(finale::collapsed(&r,40.,y*79+x));}}
    assert!(!finale::collapsed(&r,40.,6*79+6));
    pulse(&mut r,6.); assert_eq!(count(&r,40),37);
    assert_eq!(r.rng,rng);
    snapshot.restore(&mut r).unwrap();
    assert!(r.collapse.as_ref().unwrap().waves[0].diagonal_next);
    pulse(&mut r,4.); assert_eq!(r.collapse,restored.collapse);
}

#[test]
fn revision52_new_stair_seed_starts_cardinal_even_on_global_diagonal_pulse() {
    let mut r=world(); r.simulation_revision=52.0.into();
    Arc::make_mut(&mut r.floors[39]).tiles[80]="floor".into();
    Arc::make_mut(&mut r.floors[39]).tiles[5*79+5]="up".into();
    kill(&mut r); r.depth=20.;
    pulse(&mut r,2.); assert_eq!(count(&r,39),0);
    pulse(&mut r,4.); assert_eq!(count(&r,39),1);
    assert!(!r.collapse.as_ref().unwrap().waves.iter().find(|w|w.depth==39).unwrap().diagonal_next);
    pulse(&mut r,6.); assert_eq!(count(&r,39),5);
    pulse(&mut r,8.); assert_eq!(count(&r,39),21);
}

fn ring_flight(r: &mut Run) {
    r.inventory.clear();
    r.inventory.push(corpse_core::Item {
        id: 7., name: "Ring of Flight".into(), kind: "ring".into(),
        effect: "flight".to_owned().into(), ..Default::default()
    });
    r.next_id = 10.;
}

#[test]
fn collapse_fire_follows_at_half_speed_and_preserves_stairs_on_both_floors() {
    let mut r = world();
    r.simulation_revision = 52.0.into();
    r.collapse_rules_from = 0usize.into();
    kill(&mut r);
    let burning = |r: &Run, depth: u32, at| {
        corpse_core::terrain_runtime::hellfire_at(&r.floors[(depth - 1) as usize], at)
    };
    let mut grounded = corpse_core::population::create_actor("rat", 5., 5., 4., false).unwrap();
    let mut flying = grounded.clone();
    grounded.id = 6.;
    flying.x = 3.;
    flying.conditions.insert("flying".into(), 100.);
    r.actors_at_mut(40.).extend([grounded, flying]);

    pulse(&mut r, 2.);
    assert_eq!(count(&r, 40), 5);
    assert!(!burning(&r, 40, 320));
    assert_eq!(r.actors_at(40.).iter().map(|a| a.id).collect::<Vec<_>>(), vec![5.]);
    pulse(&mut r, 4.);
    assert_eq!(count(&r, 40), 21);
    assert!(burning(&r, 40, 320));
    assert!(!burning(&r, 40, 319));
    pulse(&mut r, 6.);
    assert_eq!(count(&r, 39), 0);
    pulse(&mut r, 8.);
    assert_eq!(count(&r, 39), 1);
    assert!(burning(&r, 40, 319));
    assert!(!burning(&r, 40, 240));
    assert!(!burning(&r, 39, 560));
    assert_eq!(r.floors[39].tiles[80], "up");
    assert_eq!(r.floors[38].tiles[560], "down");
    assert!(!finale::collapsed(&r, 40., 80));
    assert!(!finale::collapsed(&r, 39., 560));

    let saved = storage::serialize(&r).unwrap();
    let mut loaded = storage::deserialize(&saved).unwrap();
    pulse(&mut r, 20.);
    pulse(&mut loaded, 20.);
    assert_eq!(r.collapse, loaded.collapse);
    assert_eq!(r.floors[38].hellfire, loaded.floors[38].hellfire);
    assert!(burning(&r, 40, 80));
    assert!(burning(&r, 39, 560));
    assert!(!burning(&r, 39, 561));
    assert_eq!(r.floors[39].tiles[80], "up");
    assert_eq!(r.floors[38].tiles[560], "down");
}

#[test]
fn ring_flight_survives_chasm_but_takes_hellfire_damage_and_can_ascend() {
    let mut r = world();
    r.simulation_revision = 52.0.into();
    r.collapse_rules_from = 0usize.into();
    ring_flight(&mut r);
    kill(&mut r);
    r.player.x = 4.;
    r.player.y = 4.;
    pulse(&mut r, 2.);
    assert_eq!(r.status, "playing");
    assert!(corpse_core::travel::flying(&r, &r.player));
    let hp = r.player.hp;
    let mut e = Engine::new(r).unwrap();
    for _ in 0..6 { e.dispatch(&Command::Wait).unwrap(); }
    assert!(e.run.player.hp < hp, "airborne avatar must burn");
    assert!(corpse_core::terrain_runtime::hellfire_at(e.run.floor().unwrap(), 320));
    let mut wet = e.run.floor().unwrap().clone();
    wet.tiles[320] = "shallow-water".into();
    assert!(!corpse_core::revision51_fire_water::extinguish_cell(&mut wet, 320, 52.));
    assert!(corpse_core::terrain_runtime::hellfire_at(&wet, 320));

    // A delayed escape can still use the exact stair landing after both
    // stair tiles are surrounded and burning.
    pulse(&mut e.run, 25.);
    e.run.player.x = 2.;
    e.run.player.y = 1.;
    e.dispatch(&Command::Move{dx:-1.,dy:0.}).unwrap();
    assert_eq!(e.run.depth, 39.);
    assert_eq!((e.run.player.x, e.run.player.y), (7., 7.));
    assert_eq!(e.run.floor().unwrap().tiles[560], "down");
}

#[test]
fn collapse_started_under_legacy_rules_keeps_its_terrain_policy() {
    let mut r = world();
    r.simulation_revision = 52.0.into();
    r.collapse_rules_from = Field::Missing;
    kill(&mut r);
    finale::admit_current(&mut r).unwrap();
    pulse(&mut r, 20.);
    assert_eq!(r.floors[39].tiles[80], "chasm");
    assert_eq!(r.floors[38].tiles[560], "chasm");
    assert!(!corpse_core::terrain_runtime::hellfire_at(&r.floors[39], 320));
}

#[test]
fn burning_blue_exit_still_wins_by_normal_movement() {
    let mut r = world();
    r.simulation_revision = 52.0.into();
    r.collapse_rules_from = 0usize.into();
    ring_flight(&mut r);
    let b = r.actors_at_mut(40.).pop().unwrap();
    r.actors_at_mut(2.).push(b);
    r.depth = 2.;
    kill(&mut r);
    pulse(&mut r, 300.);
    r.depth = 1.;
    (r.player.x,r.player.y) = (39.,26.);
    assert_eq!(r.floor().unwrap().tiles[2172], "up");
    assert!(corpse_core::terrain_runtime::hellfire_at(r.floor().unwrap(), 2172));
    let mut e = Engine::new(r).unwrap();
    e.dispatch(&Command::Move {dx:0.,dy:1.}).unwrap();
    assert_eq!(e.run.status, "won");
}

#[test]
fn replay_policy_adopts_at_end_of_a_legacy_command_prefix() {
    use corpse_core::replay::{ReplayRecord, ReplaySession};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/gas-prechange-r52.json")).unwrap();
    let record: ReplayRecord = serde_json::from_value(fixture["record"].clone()).unwrap();
    let mut replay = ReplaySession::create(record, true).unwrap();
    while replay.step().unwrap() {}
    assert!(replay.engine.run.collapse_rules_from.is_missing());
    let live = Engine::load(&replay.engine.save().unwrap()).unwrap();
    let boundary = live.run.replay.len();
    assert_eq!(live.run.collapse_rules_from, boundary.into());
    let exported = ReplayRecord::try_from_run(&live.run).unwrap();
    let mut mixed = ReplaySession::create(exported, true).unwrap();
    assert!(mixed.engine.run.collapse_rules_from.is_missing());
    while mixed.step().unwrap() {}
    assert_eq!(mixed.engine.run.collapse_rules_from, boundary.into());
}
