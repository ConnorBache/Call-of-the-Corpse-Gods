//! Independently authored controlled integration tests, not generated-seed balance claims.
use corpse_core::{Command, Field, Run, engine, population, portals, vision, movement::Point,
    non_euclidean_generation::{NonEuclideanGroup as Group, NonEuclideanPocket as Pocket},
    temporal::Engine, replay::{ReplayRecord, ReplaySession}, dreamwalking as dream};
use std::sync::Arc;

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 36.0.into();
    r.inventory.clear(); r.offers.clear(); r.history.clear(); r.replay.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing; r.log.clear();
    r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.x = 8.; r.player.y = 8.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.;
    r.player.stats.accuracy = 10000.; r.player.stats.offense = 20.; r.next_id = 1000.; r.rng = 1.;
    let mut f = r.floors[0].as_ref().clone();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.godbeast = false; f.doors = Field::Missing;
    f.light_sources = Field::Missing; f.blood = Field::Missing; f.roots = Field::Missing;
    f.fungal_roots = Field::Missing; f.razorleaves = Field::Missing; f.grates = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    let (w, h) = (f.width(), f.height());
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into(); f.lights.fill(1.);
    r.floors.clear(); r.actors.clear(); r.ground.clear();
    for depth in 1..=20 {
        let mut floor = f.clone(); floor.depth = depth as f64; r.floors.push(Arc::new(floor));
        r.actors.insert(depth.to_string(), Arc::new(vec![])); r.ground.insert(depth.to_string(), Arc::new(vec![]));
    }
    r.depth = 20.;
    let at = |x, y| y*w+x;
    r.floor_mut().unwrap().non_euclidean = vec![Pocket {
        cells: vec![at(9,8), at(10,8), at(11,8), at(12,8), at(9,9), at(10,9), at(11,9), at(12,9)],
        groups: vec![
            Group { sources: vec![at(9,8), at(9,9)], destination: at(12,9) },
            Group { sources: vec![at(10,8), at(11,8)], destination: at(12,8) },
            Group { sources: vec![at(10,9)], destination: at(11,9) },
        ],
    }].into();
    r
}
fn rat(r: &mut Run, x: f64, y: f64, ally: bool) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = population::create_actor_revision("rat", id, x, y, false, 36.).unwrap();
    a.state = if ally { "ally" } else { "sleeping" }.into();
    a.hp = 100.; a.max_hp = 100.; a.stats.dodge = -1000.; a.stats.defense = 0.; a.stats.dr = 0.;
    r.actors_at_mut(20.).push(a); id
}
fn ability(id: &str) -> Command { Command::Ability { id:id.into(), target:Field::Missing, x:Field::Missing, y:Field::Missing } }

#[test]
fn entering_each_source_arrives_once_at_shared_destination_without_rng() {
    for y in [8., 9.] {
        let mut r = room(); r.player.y = y; let before = r.rng;
        assert!(engine::move_actor(&mut r, 0., 1., 0., false).unwrap());
        assert_eq!((r.player.x,r.player.y),(12.,9.)); assert_eq!(r.rng,before);
        assert!(engine::move_actor(&mut r, 0., 1., 0., false).unwrap());
        assert_eq!((r.player.x,r.player.y),(13.,9.)); assert_eq!(r.rng,before);
    }
}

#[test]
fn reverse_transition_consumes_exactly_one_serialized_draw_and_exercises_both_exits() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in [1u32, 8192, 16384, 123456789, 0x80000000, 0xffffffff] {
        let mut r = room(); r.player.x = 13.; r.player.y = 9.; r.rng = seed as f64;
        let mut expected = seed; let draw = corpse_core::rng::next_u32(&mut expected) as f64 / 4294967296.;
        assert!(engine::move_actor(&mut r, 0., -1., 0., false).unwrap());
        assert_eq!((r.player.x,r.player.y),(9.,if draw < 0.5 {8.} else {9.}));
        assert_eq!(r.rng,expected as f64); seen.insert(r.player.y as i32);
    }
    assert_eq!(seen,std::collections::BTreeSet::from([8,9]));
}

#[test]
fn read_only_queries_and_blocked_sibling_do_not_consume_a_lottery_draw() {
    let mut r = room(); r.player.x = 13.; r.player.y = 9.;
    let initial = r.clone();
    for _ in 0..100 {
        assert_eq!(portals::step_options(&r,&r.player,12.,9.),vec![(9.,8.),(9.,9.)]);
        let _ = portals::point_distance(&r,&r.player,8.,8.);
        let _ = vision::optical_route(&r,&r.player,Point{x:8,y:8});
    }
    assert_eq!(r,initial);
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[9*w+9] = "wall".into();
    let before = r.rng;
    assert!(!engine::move_actor(&mut r,0.,-1.,0.,false).unwrap());
    assert_eq!((r.player.x,r.player.y),(13.,9.)); assert_eq!(r.rng,before);
}

#[test]
fn linked_enemy_occupancy_attacks_without_telefrag_or_random_return() {
    let mut r = room(); r.player.x = 13.; r.player.y = 9.;
    let id = rat(&mut r,9.,9.,false); r.player.conditions.insert("lucky".into(),1.);
    assert!(engine::move_actor(&mut r,0.,-1.,0.,false).unwrap());
    assert_eq!((r.player.x,r.player.y),(13.,9.));
    let a = engine::actor(&r,id).unwrap(); assert!(a.hp<100.); assert_eq!((a.x,a.y),(9.,9.));
    assert_eq!(r.actors_at(20.).iter().filter(|a|a.id==id).count(),1);
}

#[test]
fn pockets_do_not_create_an_optical_viewpoint_or_reduce_visual_distance() {
    let mut r = room();
    let direct = vision::optical_route(&r,&r.player,Point{x:13,y:9}).unwrap();
    assert!((direct.distance-26f64.sqrt()).abs()<1e-12);
    let w = r.floor().unwrap().width();
    for y in 1..28 { r.floor_mut().unwrap().tiles[y*w+10] = "wall".into(); }
    assert!(vision::optical_route(&r,&r.player,Point{x:13,y:9}).is_none());
    // The same physical divider still permits revision35's explicitly optical portal.
    r.floor_mut().unwrap().portals = vec![corpse_core::portals_generation::PortalPair {
        a:7*w+9,b:7*w+12,color_index:0,
    }].into();
    assert!(vision::optical_route(&r,&r.player,Point{x:13,y:9}).is_some());
}

#[test]
fn shared_underfoot_items_are_transferred_once_not_cloned() {
    let mut r = room(); r.player.x = 12.; r.player.y = 9.;
    r.ground_at_mut(20.).push(corpse_core::GroundItem { x:9.,y:8.,item:corpse_core::Item {
        id:901.,kind:"ammo".into(),name:"Arrow".into(),glyph:"→".into(),
        effect:"arrows".to_string().into(),quantity:1.0.into(),..Default::default()
    },..Default::default() });
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r,20.).unwrap(),1);
    assert!(r.ground["20"].is_empty()); assert_eq!(r.inventory.iter().filter(|i|i.id==901.).count(),1);
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r,20.).unwrap(),0);
}

#[test]
fn dream_transition_does_not_move_or_attack_the_waking_body() {
    let mut r = room(); r.dreamwalking = dream::State { unlocked:true,..Default::default() }.into();
    dream::ability(&mut r,dream::DREAMWALK_ID).unwrap();
    let body = dream::body(&r).unwrap().id; let normal = rat(&mut r,12.,9.,false);
    assert!(engine::move_actor(&mut r,0.,1.,0.,false).unwrap());
    assert_eq!((r.player.x,r.player.y),(12.,9.));
    let b=engine::actor(&r,body).unwrap(); assert_eq!((b.x,b.y),(8.,8.));
    assert_eq!(engine::actor(&r,normal).unwrap().hp,100.);
    assert!(!dream::same_plane(&r,&r.player,engine::actor(&r,normal).unwrap()));
}

#[test]
fn save_replay_revert_and_future_preserve_topology_and_random_outcome() {
    let mut r = room(); r.player.x=13.; r.player.y=9.;
    r.player.abilities=vec!["numo-revert".into(),"morae-see-the-future".into()];
    r.player.conditions.insert("revertCharges".into(),5.);
    let initial=r.clone(); let topology=r.floor().unwrap().non_euclidean.clone();
    let mut e=Engine::new(r).unwrap();
    assert!(e.dispatch(&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    let first=(e.run.player.x,e.run.player.y); let after=e.run.rng;
    let saved=e.save().unwrap(); assert_eq!(Engine::load(&saved).unwrap().save().unwrap(),saved);
    let mut replay=ReplaySession::from_initial(initial,ReplayRecord::from_run(&e.run)).unwrap();
    while replay.step_with_outcome().unwrap().is_some() {}
    assert_eq!(replay.engine.save().unwrap(),e.save().unwrap());
    assert!(e.dispatch(&ability("numo-revert")).unwrap().changed);
    assert_eq!((e.run.player.x,e.run.player.y),(13.,9.)); assert_eq!(e.run.rng,1.);
    assert!(e.dispatch(&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    assert_eq!((e.run.player.x,e.run.player.y),first); assert_eq!(e.run.rng,after);
    assert_eq!(e.run.floor().unwrap().non_euclidean,topology);
    // Future is an independent speculative branch and carries the same floor metadata.
    e.run.player.x=13.; e.run.player.y=9.; let before=e.run.rng;
    assert!(e.dispatch(&ability("morae-see-the-future")).unwrap().changed);
    assert!(e.dispatch(&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    e=Engine::load(&e.save().unwrap()).unwrap();
    assert!(e.dispatch(&ability("morae-see-the-future")).unwrap().changed);
    assert_eq!((e.run.player.x,e.run.player.y),(13.,9.)); assert_eq!(e.run.rng,before);
    assert_eq!(e.run.floor().unwrap().non_euclidean,topology);
}

#[test]
fn absent_pockets_preserve_revision35_commands_and_actor_tuning() {
    let mut current=room(); current.floor_mut().unwrap().non_euclidean=Field::Missing;
    let mut old=current.clone(); old.simulation_revision=35.0.into();
    for command in [Command::Move{dx:1.,dy:1.},Command::Wait,Command::Move{dx:-1.,dy:0.}] {
        let a=engine::dispatch(&mut old,&command).unwrap(); let b=engine::dispatch(&mut current,&command).unwrap();
        assert_eq!(a.took_turn,b.took_turn); assert_eq!(old.player,current.player);
        assert_eq!(old.rng,current.rng); assert_eq!(old.floors,current.floors); assert_eq!(old.actors,current.actors);
    }
    for kind in population::catalog().actors.keys().map(String::as_str).chain(["night-crawler","skin-walker","saidribee-queen"]) {
        assert_eq!(population::create_actor_revision(kind,100.,8.,8.,false,35.).unwrap(),
            population::create_actor_revision(kind,100.,8.,8.,false,36.).unwrap(),"{kind}");
    }
}

#[test]
fn half_here_keeps_its_creation_plane_and_does_not_project_pocket_sight() {
    let mut r=room();r.player.abilities.push("etill-half-here".into());r.player.active.push("etill-half-here".into());
    let clone=corpse_core::half_here::create(&mut r,9.,8.).unwrap();
    let a=engine::actor(&r,clone).unwrap();assert!(portals::same_position(&r,a,12.,9.));
    let w=r.floor().unwrap().width();for y in 1..28{r.floor_mut().unwrap().tiles[y*w+10]="wall".into();}
    assert!(vision::optical_route(&r,engine::actor(&r,clone).unwrap(),Point{x:13,y:9}).is_none());
    r.dreamwalking=dream::State{unlocked:true,..Default::default()}.into();
    dream::ability(&mut r,dream::DREAMWALK_ID).unwrap();
    let a=engine::actor(&r,clone).unwrap();assert!(!dream::dream_actor(&r,a));
    assert!(portals::same_position(&r,a,12.,9.));
    assert!(vision::optical_route(&r,a,Point{x:13,y:9}).is_none());
}

#[test]
fn actual_empty_throw_samples_once_and_places_one_real_item_at_the_chosen_exit() {
    for seed in [1u32,8192,123456789] {
        let mut r=room();r.player.x=13.;r.player.y=9.;r.rng=seed as f64;
        r.inventory.push(corpse_core::Item{id:902.,kind:"ammo".into(),name:"Arrow".into(),glyph:"→".into(),
            effect:"arrows".to_string().into(),quantity:1.0.into(),..Default::default()});
        let mut expected=seed;let choice=corpse_core::rng::next_u32(&mut expected) as f64/4294967296.;
        assert_eq!(corpse_core::projectiles::command(&mut r,&Command::Throw{id:902.,x:12.,y:9.}).unwrap(),Some(true));
        assert!(r.inventory.is_empty());assert_eq!(r.rng,expected as f64);
        let found:Vec<_>=r.ground["20"].iter().filter(|g|g.item.id==902.).collect();
        assert_eq!(found.len(),1);assert_eq!((found[0].x,found[0].y),(9.,if choice<0.5{8.}else{9.}));
    }
}

#[test]
fn invalid_throw_and_interception_before_fork_do_not_draw_a_random_exit() {
    let mut r=room();r.player.x=13.;r.player.y=9.;let before=r.rng;
    assert_eq!(corpse_core::projectiles::command(&mut r,&Command::Throw{id:99999.,x:12.,y:9.}).unwrap(),Some(false));
    assert_eq!(r.rng,before);
    let id=rat(&mut r,9.,8.,false);let a=r.player.clone();
    let landing=corpse_core::projectiles::trace_committed_from_actor(&mut r,&a,12.,9.);
    assert_eq!(landing.actor_id,Some(id));assert_eq!(r.rng,before);
}

#[test]
fn debug_floor_twenty_and_forty_keep_exact_revision35_rewards_choices_and_costs() {
    use corpse_core::{debug_floor as debug,progression,startup};
    for floor in [20,40] {
        let seed=format!("0-5-11:r36-debug-budget-{floor}");
        let mut old=debug::create_run_revision(&seed,"Unblessed",floor,35.).unwrap();
        let mut new=debug::create_run_revision(&seed,"Unblessed",floor,36.).unwrap();
        assert_eq!(new.simulation_revision.value_or(0.),36.);
        assert_eq!(old.playtest,new.playtest);assert_eq!(old.inventory,new.inventory);
        assert_eq!(old.offers,new.offers);assert_eq!(old.rng,new.rng);assert_eq!(old.next_id,new.next_id);
        while !old.offers.is_empty() {
            assert_eq!(old.offers,new.offers);let chosen=old.offers[0].clone();
            progression::choose_offer(&mut old,&chosen).unwrap();debug::advance(&mut old).unwrap();
            progression::choose_offer(&mut new,&chosen).unwrap();debug::advance(&mut new).unwrap();
        }
        assert!(debug::choices_complete(&new));assert_eq!(old.playtest,new.playtest);
        assert_eq!(old.player,new.player);assert_eq!((old.level,old.exp),(new.level,new.exp));
        assert_eq!(old.run_stats,new.run_stats);assert_eq!(old.rng,new.rng);
        assert_eq!(old.tick,0.);assert_eq!(new.tick,0.);assert_eq!(old.actors,new.actors);assert_eq!(old.ground,new.ground);
        debug::enter(&mut new).unwrap();assert!(debug::state(&new).unwrap().entered);
        let saved=corpse_core::storage::serialize(&new).unwrap();
        assert_eq!(corpse_core::storage::deserialize(&saved).unwrap().playtest,new.playtest);
        let fresh=startup::create_run(&seed,"Unblessed",false).unwrap();
        assert_eq!(fresh.simulation_revision.value_or(0.),39.);assert_eq!(fresh.depth,1.);
        assert!(fresh.playtest.is_missing());assert!(fresh.floors.iter().all(|f|f.non_euclidean.is_missing()));
    }
}
