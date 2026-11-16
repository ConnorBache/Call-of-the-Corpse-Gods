use super::*;
use crate::{Command, Field, GroundItem, Item, Stats, engine, environment};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into(); r.depth = 1.; r.tick = 0.; r.next_id = 2000.;
    r.actors = Default::default(); r.ground.clear(); r.inventory.clear(); r.floors.truncate(3);
    r.history.clear(); r.replay.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.fast_action_phase = Field::Missing;
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 10000.; r.player.max_hp = 10000.;
    r.player.base_max_hp = 10000.0.into(); r.player.stats = Stats { perception: 300., max_energy: 100., ..Default::default() };
    r.player.energy = 100.; r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.player.amulet_ledger = Field::Missing;
    for floor in &mut r.floors {
        let f = crate::mutation_journal::make_mut(floor);
        f.tiles.fill("floor".into()); f.lights.fill(100.); f.seen.fill(true); f.dirty = false; f.last_tick = 0.;
        f.doors = Field::Missing; f.portals = Field::Missing; f.non_euclidean = Field::Missing;
        f.environment = Field::Missing; f.vents = Field::Missing; f.currents = Field::Missing;
        f.roots = Field::Missing; f.fungal_roots = Field::Missing; f.mud = Field::Missing;
        f.slime = Field::Missing; f.ichor = Field::Missing; f.grates = Field::Missing;
        f.permanent_fire = Field::Missing; f.razorleaves = Field::Missing; f.godbeast_event = Field::Missing;
        f.ice = Field::Missing; f.hellfire = Field::Missing; f.light_sources = Field::Missing;
        f.revision50_terrain = Field::Missing; f.revision50_temporary_terrain = Field::Missing;
    }
    for d in 1..=3 { r.actors_at_mut(d as f64); r.ground_at_mut(d as f64); }
    r
}
fn corpse(id: f64, x: f64, y: f64) -> Actor {
    let mut a = Actor { id, x, y, name: "Cave Bat".into(), glyph: "b".into(),
        hp: 0., max_hp: 100., energy: 100., dead: true.into(), heartless: true.into(),
        corpse_age: 3.0.into(), state: "sleeping".into(),
        stats: Stats { max_energy: 100., ..Default::default() },
        abilities: vec!["enemy:beast:flight".into()], ..Default::default() };
    a.conditions.insert("bestiaryTraitsInitialized".into(), 1.);
    a.conditions.insert("paleTraitsInitialized".into(), 1.);
    a
}
fn item(id: f64, x: f64, y: f64) -> GroundItem {
    GroundItem { x, y, item: Item { id, kind: "heart".into(), name: "Rat heart".into(), glyph: ";".into(),
        ..Default::default() }, ..Default::default() }
}
fn at(r: &Run, x: usize, y: usize) -> usize { y * r.floor().unwrap().width() + x }
fn arrow(r: &mut Run, x: usize, y: usize, direction: f64) {
    let i = at(r, x, y); let f = r.floor_mut().unwrap();
    f.tiles[i] = "water".into(); let n = f.tiles.len();
    f.currents.get_or_insert_with(|| vec![0.; n])[i] = direction;
}
fn pos(r: &Run, id: f64) -> (f64, f64) {
    let a = engine::actor(r, id).unwrap(); (a.x, a.y)
}

#[test]
fn dead_flight_sources_never_hold_corpses_above_currents_and_revival_keeps_innate_flight() {
    for mode in ["innate", "timed", "vampire", "ascend", "phase"] {
        let mut r = room(); arrow(&mut r, 15, 10, 3.);
        let mut a = corpse(800., 15., 10.); a.abilities.clear();
        match mode {
            "innate" => a.abilities.push("enemy:beast:flight".into()),
            "timed" => { a.conditions.insert("flying".into(), 6.); },
            "vampire" => { a.abilities.push("enemy:beast:vampire".into()); a.conditions.insert("monsterVampireBat".into(), 1.); },
            "ascend" => a.abilities.push("anavani-ascend".into()),
            _ => a.active.push("etill-phase-through".into()),
        }
        let before = a.clone(); r.actors_at_mut(1.).push(a);
        assert!(!crate::travel::flying(&r, engine::actor(&r, 800.).unwrap()));
        assert!(!engine::flying_in(&r, engine::actor(&r, 800.).unwrap()));
        let rng = r.rng; drift(&mut r); assert_eq!(pos(&r, 800.), (16., 10.)); assert_eq!(r.rng, rng);
        let a = engine::actor(&r, 800.).unwrap();
        assert_eq!(a.abilities, before.abilities); assert_eq!(a.conditions, before.conditions);
        assert_eq!(a.corpse_age, before.corpse_age); assert_eq!(a.hp, 0.);
        if mode == "innate" {
            crate::corpses::revive_corpse(engine::actor_mut_trusted(&mut r, 800.).unwrap(), 1.);
            assert!(crate::travel::flying(&r, engine::actor(&r, 800.).unwrap()));
        }
    }
}

#[test]
fn corpse_item_and_living_layers_are_independent_and_living_flyers_remain_immune() {
    let mut r = room(); arrow(&mut r, 15, 10, 3.);
    r.actors_at_mut(1.).push(corpse(800., 15., 10.));
    let mut flyer = corpse(801., 15., 10.); flyer.dead = false.into(); flyer.hp = 100.;
    r.actors_at_mut(1.).push(flyer); r.ground_at_mut(1.).push(item(900., 15., 10.));
    crate::currents::tick_currents(&mut r).unwrap(); environment::tick_environment(&mut r).unwrap();
    assert_eq!(pos(&r, 800.), (16., 10.)); assert_eq!(pos(&r, 801.), (15., 10.));
    assert_eq!((r.ground["1"][0].x, r.ground["1"][0].y), (16., 10.));
    assert_eq!(r.ground["1"][0].item.id, 900.); assert_eq!(r.actors_at(1.).len(), 2);
}

#[test]
fn collision_uses_starting_corpses_and_stable_ids_not_storage_order() {
    for reverse in [false, true] {
        let mut r = room(); arrow(&mut r, 15, 10, 3.); arrow(&mut r, 17, 10, 7.);
        let mut actors = vec![corpse(801., 17., 10.), corpse(800., 15., 10.)];
        if reverse { actors.reverse(); } r.actors_at_mut(1.).extend(actors);
        drift(&mut r); assert_eq!(pos(&r, 800.), (16., 10.)); assert_ne!(pos(&r, 801.), (16., 10.));
        let mut r = room(); arrow(&mut r, 15, 10, 3.); arrow(&mut r, 16, 10, 3.);
        r.actors_at_mut(1.).extend([corpse(800., 15., 10.), corpse(801., 16., 10.)]);
        drift(&mut r); assert_eq!(pos(&r, 800.), (15., 10.)); assert_eq!(pos(&r, 801.), (17., 10.));
        let mut r = room(); arrow(&mut r, 15, 10, 3.); arrow(&mut r, 16, 10, 7.);
        r.actors_at_mut(1.).extend([corpse(800., 15., 10.), corpse(801., 16., 10.)]);
        drift(&mut r); assert_eq!(pos(&r, 800.), (15., 10.)); assert_eq!(pos(&r, 801.), (16., 10.));
    }
}

#[test]
fn illegal_destinations_do_not_discard_or_relocate_corpses_and_safe_bank_alternates_work() {
    for tile in ["wall", "tree", "chasm", "pit", "up", "down"] {
        let mut r = room(); arrow(&mut r, 15, 10, 3.); let to = at(&r, 16, 10);
        r.floor_mut().unwrap().tiles[to] = tile.into(); r.actors_at_mut(1.).push(corpse(800., 15., 10.));
        drift(&mut r); assert_eq!(pos(&r, 800.), (15., 10.), "{tile}");
        assert_eq!(r.actors_at(1.).len(), 1);
    }
    let mut r = room(); arrow(&mut r, 15, 10, 3.);
    r.actors_at_mut(1.).extend([corpse(800., 15., 10.), corpse(801., 16., 10.)]);
    drift(&mut r); assert_eq!(pos(&r, 800.), (15., 9.)); assert_eq!(pos(&r, 801.), (16., 10.));
}

#[test]
fn rooted_corpses_hold_but_partial_conduits_sever_when_the_corpse_drifts() {
    for complete in [false, true] {
        let mut r = room(); arrow(&mut r, 15, 10, 3.); let i = at(&r, 15, 10);
        r.actors_at_mut(1.).push(corpse(800., 15., 10.));
        let mut fungus = corpse(801., 14., 10.); fungus.dead = false.into(); fungus.hp = 100.;
        fungus.abilities = vec!["enemy:beast:fungal-gas".into()]; r.actors_at_mut(1.).push(fungus);
        let f = r.floor_mut().unwrap(); let mut roots = vec![0.; f.tiles.len()]; roots[i-1] = 1.;
        if complete { roots[i] = 1.; } f.roots = roots.into();
        f.fungal_roots = vec![crate::FungalRoot { fungus_id: 801., corpse_id: 800.,
            path: vec![(i-1) as f64, i as f64], active: true, color: "#8b9675".into(),
            growth: crate::FungalRootGrowth { grown: if complete { 2 } else { 1 }, turns: 0 }.into() }].into();
        assert_eq!(has_activity(&r, r.floor().unwrap()), !complete); drift(&mut r);
        assert_eq!(pos(&r, 800.), if complete { (15., 10.) } else { (16., 10.) });
        let root = &r.floor().unwrap().fungal_roots.as_ref().unwrap()[0]; assert_eq!(root.active, complete);
        if !complete { assert!(r.floor().unwrap().roots.as_ref().unwrap().iter().all(|v| *v == 0.)); }
    }
}

#[test]
fn inactive_corpse_only_current_floor_advances_exactly_like_visible_normal_ticks() {
    let mut r = room(); for x in 15..=19 { arrow(&mut r, x, 10, 3.); }
    r.actors_at_mut(1.).push(corpse(800., 15., 10.)); r.floor_mut().unwrap().dirty = true;
    let mut active = r.clone(); for _ in 0..4 { engine::dispatch(&mut active, &Command::Wait).unwrap(); }
    r.depth = 2.; r.tick = 4.; crate::offscreen::catch_up_floor(&mut r, 1.).unwrap();
    assert_eq!(r.actors_at(1.)[0].x, 19.); assert_eq!(r.actors_at(1.), active.actors_at(1.));
    assert_eq!(r.rng, active.rng); assert_eq!(r.depth, 2.); assert_eq!(r.ground, active.ground);
}

#[test]
fn normal_haste_save_snapshot_and_future_replay_keep_exact_positions_and_ids() {
    let mut r = room(); for x in 15..=19 { arrow(&mut r, x, 10, 3.); }
    r.actors_at_mut(1.).push(corpse(800., 15., 10.)); r.ground_at_mut(1.).push(item(900., 15., 10.));
    r.player.conditions.insert("hasted".into(), 10.); r.phase = "haste".to_string().into();
    assert!(!engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn); assert_eq!(pos(&r, 800.), (15., 10.));
    assert_eq!(r.ground["1"][0].x, 15.);
    let snap = crate::temporal::Snapshot::capture(&r);
    let anchor = crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    let mut future = anchor.prepare_restore().unwrap();
    let mut loaded = crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();
    drift(&mut r); snap.restore(&mut r).unwrap();
    for state in [&mut r, &mut loaded, &mut future] {
        assert!(engine::dispatch(state, &Command::Wait).unwrap().took_turn);
        assert_eq!(pos(state, 800.), (16., 10.)); assert_eq!(state.ground["1"][0].x, 16.);
        assert_eq!(state.actors_at(1.).len(), 1); assert_eq!(state.ground["1"].len(), 1);
    }
    assert_eq!(r.actors, loaded.actors); assert_eq!(r.actors, future.actors); assert_eq!(r.ground, loaded.ground);
    assert_eq!(r.rng, loaded.rng); assert_eq!(r.rng, future.rng);
}

#[test]
fn flyer_death_immediately_falls_through_two_chasms_and_preserves_revival_identity() {
    let mut r = room(); let i = at(&r, 15, 10);
    for f in &mut r.floors[..2] { crate::mutation_journal::make_mut(f).tiles[i] = "chasm".into(); }
    let mut a = corpse(800., 15., 10.); a.dead = false.into(); a.hp = 1.; a.heartless = false.into();
    r.actors_at_mut(1.).push(a); let count = r.floors.len();
    engine::damage_environment(&mut r, 800., 100., None, "fire", true).unwrap();
    assert!(r.actors_at(1.).is_empty()); assert!(r.actors_at(2.).is_empty());
    let a = &r.actors_at(3.)[0]; assert_eq!((a.id, a.x, a.y, a.hp), (800., 15., 10., 0.));
    assert!(a.dead.value_or(false)); assert_eq!(a.corpse_age, 0.0.into());
    assert!(a.abilities.iter().any(|s| s == "enemy:beast:flight")); assert!(!crate::travel::flying(&r, a));
    assert_eq!(r.ground["3"].len(), 1); assert_eq!(r.floors.len(), count);
    assert_eq!(r.actors.values().map(|v| v.len()).sum::<usize>(), 1);
    let raw = crate::storage::serialize(&r).unwrap(); assert_eq!(crate::storage::deserialize(&raw).unwrap().actors, r.actors);
}

#[test]
fn collapsing_floor_uses_existing_corpse_and_item_falls_without_living_flight_loss() {
    let mut r = room(); r.actors_at_mut(1.).push(corpse(800., 15., 10.));
    let mut a = corpse(801., 15., 10.); a.dead = false.into(); a.hp = 100.; r.actors_at_mut(1.).push(a);
    r.ground_at_mut(1.).push(item(900., 15., 10.));
    assert!(crate::travel::collapse_descent(&mut r, 15., 10., true).unwrap());
    assert!(r.actors_at(1.).iter().all(|a| a.id != 800.)); assert_eq!(pos(&r, 801.), (15., 10.));
    assert!(crate::travel::flying(&r, engine::actor(&r, 801.).unwrap()));
    assert_eq!(r.actors_at(2.).iter().filter(|a| a.id == 800.).count(), 1);
    assert_eq!(r.ground["2"].iter().filter(|e| e.item.id == 900.).count(), 1);
    assert!(r.ground["1"].is_empty());
}

#[test]
fn legacy_corpses_and_dream_plane_are_unchanged() {
    let mut r = room(); arrow(&mut r, 15, 10, 3.); r.actors_at_mut(1.).push(corpse(800., 15., 10.));
    r.simulation_revision = 51.0.into(); let before = r.clone(); drift(&mut r);
    assert_eq!(r, before); assert!(!has_activity(&r, r.floor().unwrap()));
    assert!(crate::travel::flying(&r, engine::actor(&r, 800.).unwrap()));
    r.simulation_revision = 52.0.into(); crate::dreamwalking::ensure_layer(&mut r);
    crate::dreamwalking::with_plane(&mut r, true, drift); assert_eq!(pos(&r, 800.), (15., 10.));
}

#[test]
fn existing_loose_item_types_move_once_and_keep_payloads_and_established_exemptions() {
    for kind in ["heart", "ring", "potion", "tool", "weapon", "armor"] {
        let mut r = room(); arrow(&mut r, 15, 10, 3.); arrow(&mut r, 16, 10, 3.);
        let mut e = item(900., 15., 10.); e.item.kind = kind.into();
        let before = e.item.clone(); r.ground_at_mut(1.).push(e);
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.ground["1"][0].x, 16., "{kind}"); assert_eq!(r.ground["1"][0].item, before);
        assert_eq!(r.ground.values().map(|g| g.len()).sum::<usize>(), 1);
    }
    for protected in ["drown", "shard", "dream shard"] {
        let mut r = room(); arrow(&mut r, 11, 10, 3.); let mut e = item(900., 11., 10.);
        if protected == "drown" { r.player.abilities.push("etill-drown".into()); }
        else {
            e.item = crate::phantasmii_generation::shard(900.);
            if protected == "dream shard" { e.item = crate::borge::shard(900.); e.item.dream_ground = true.into(); }
        }
        r.ground_at_mut(1.).push(e); environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.ground["1"][0].x, 11., "{protected}"); assert_eq!(r.ground["1"].len(), 1);
    }
}

#[test]
fn active_waking_corpses_drift_only_once_during_two_plane_environment_processing() {
    let mut r = room(); arrow(&mut r, 15, 10, 3.); arrow(&mut r, 16, 10, 3.);
    r.actors_at_mut(1.).push(corpse(800., 15., 10.)); crate::dreamwalking::ensure_layer(&mut r);
    crate::dreamwalking::tick_environments(&mut r, &[]).unwrap(); assert_eq!(pos(&r, 800.), (16., 10.));
}

#[test]
fn inactive_flying_death_falls_once_and_does_not_leave_a_source_floor_copy() {
    let mut r = room(); let i = at(&r, 15, 10); r.floor_mut().unwrap().tiles[i] = "chasm".into();
    r.floor_mut().unwrap().dirty = true;
    let mut a = corpse(800., 15., 10.); a.dead = false.into(); a.hp = 0.; r.actors_at_mut(1.).push(a);
    r.depth = 3.; r.tick = 1.; crate::offscreen::catch_up_floor(&mut r, 1.).unwrap();
    assert!(r.actors_at(1.).is_empty()); assert_eq!(r.actors_at(2.).len(), 1);
    assert_eq!(r.actors_at(2.)[0].id, 800.); assert!(r.actors_at(2.)[0].dead.value_or(false));
    assert_eq!(r.actors.values().flat_map(|v| v.iter()).filter(|a| a.id == 800.).count(), 1);
    assert_eq!(r.depth, 3.);
}

#[test]
fn occupied_fall_landings_separate_corpses_and_items_without_losing_ids() {
    let mut r = room(); let i = at(&r, 15, 10); r.floor_mut().unwrap().tiles[i] = "chasm".into();
    r.actors_at_mut(2.).push(corpse(801., 15., 10.)); r.ground_at_mut(2.).push(item(901., 15., 10.));
    let mut a = corpse(800., 15., 10.); a.dead = false.into(); a.hp = 0.; a.heartless = false.into();
    r.actors_at_mut(1.).push(a); engine::death_if_nonpositive(&mut r, 800.).unwrap();
    assert!(r.actors_at(1.).is_empty()); assert_eq!(r.actors_at(2.).len(), 2);
    let actors = r.actors_at(2.); assert_ne!((actors[0].x, actors[0].y), (actors[1].x, actors[1].y));
    let items = &r.ground["2"]; assert_eq!(items.len(), 2);
    assert_ne!((items[0].x, items[0].y), (items[1].x, items[1].y));
    assert_eq!(actors.iter().filter(|a| a.id == 800.).count(), 1);
    assert_eq!(items.iter().filter(|g| g.item.id == 901.).count(), 1);
}

#[test]
fn saved_mature_gap_corpse_falls_at_command_boundary_without_time_or_revival() {
    let mut r = room(); let gap = at(&r, 15, 10); let wall = at(&r, 11, 10);
    r.floor_mut().unwrap().tiles[gap] = "chasm".into(); r.floor_mut().unwrap().tiles[wall] = "wall".into();
    let mut a = corpse(800., 15., 10.); a.corpse_age = 1000.0.into(); r.actors_at_mut(1.).push(a);
    let mut r = crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();
    let rng = r.rng; let tick = r.tick;
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert!(r.actors_at(1.).is_empty()); let a = &r.actors_at(2.)[0];
    assert_eq!((a.id, a.x, a.y, a.corpse_age.value_or(0.)), (800., 15., 10., 1000.));
    assert!(a.dead.value_or(false)); assert_eq!(r.tick, tick); assert_eq!(r.rng, rng);
}

#[test]
fn black_pit_under_corpse_falls_by_normal_or_haste_command_completion() {
    for haste in [false, true] {
        let mut r = room(); r.actors_at_mut(1.).push(corpse(800., 10., 10.));
        let power = "grisha-black-pit"; r.player.abilities.push(power.into()); r.player.active.push(power.into());
        crate::revision50_ability_energy::initialize_reserve(&mut r.player, power, 10., 10.).unwrap();
        if haste { r.player.conditions.insert("hasted".into(), 10.); r.phase = "haste".to_string().into(); }
        let rng = r.rng; let outcome = engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
        assert_eq!(outcome.took_turn, !haste); assert_eq!(r.tick, if haste { 0. } else { 1. });
        let i = at(&r, 10, 10); assert_eq!(r.floor().unwrap().tiles[i], "chasm");
        assert!(r.actors_at(1.).is_empty()); let a = &r.actors_at(2.)[0];
        assert_eq!((a.id, a.x, a.y), (800., 10., 10.)); assert!(a.dead.value_or(false));
        if haste { assert_eq!(a.corpse_age.value_or(0.), 3.); assert_eq!(r.rng, rng); }
    }
}

#[test]
fn unsupported_corpse_falls_before_temporary_chasm_restoration() {
    let mut r = room(); let i = at(&r, 15, 10); r.floor_mut().unwrap().tiles[i] = "chasm".into();
    r.floor_mut().unwrap().altered_chasms.get_or_insert_with(Default::default)
        .insert(i.to_string(), crate::FloorAlteredChasmsEntry { tile: "floor".into(), age: 2. });
    r.actors_at_mut(1.).push(corpse(800., 15., 10.)); environment::tick_environment(&mut r).unwrap();
    assert_eq!(r.floor().unwrap().tiles[i], "floor"); assert!(r.actors_at(1.).is_empty());
    assert_eq!((r.actors_at(2.)[0].id, r.actors_at(2.)[0].x), (800., 15.));
}

#[test]
fn inactive_gap_only_floor_is_precise_and_gravity_precedes_guaranteed_revival() {
    let mut r = room(); let i = at(&r, 15, 10); r.floor_mut().unwrap().tiles[i] = "chasm".into();
    r.floor_mut().unwrap().dirty = true; let mut a = corpse(800., 15., 10.); a.corpse_age = 1000.0.into();
    r.actors_at_mut(1.).push(a); assert!(has_activity(&r, r.floor().unwrap()));
    r.depth = 3.; r.tick = 1.; let rng = r.rng; crate::offscreen::catch_up_floor(&mut r, 1.).unwrap();
    assert!(r.actors_at(1.).is_empty()); let a = &r.actors_at(2.)[0]; assert!(a.dead.value_or(false));
    assert_eq!(a.corpse_age.value_or(0.), 1000.); assert_eq!(r.rng, rng);
}

#[test]
fn dream_only_gap_never_transfers_waking_corpses_and_legacy_gaps_remain_unchanged() {
    let mut r = room(); r.actors_at_mut(1.).push(corpse(800., 15., 10.));
    crate::dreamwalking::ensure_layer(&mut r); let i = at(&r, 15, 10);
    crate::dreamwalking::with_plane(&mut r, true, |r| {
        r.floor_mut().unwrap().tiles[i] = "chasm".into(); settle_falls(r).unwrap();
    });
    assert_eq!(pos(&r, 800.), (15., 10.)); assert!(r.actors_at(2.).is_empty());
    settle_falls(&mut r).unwrap(); assert_eq!(pos(&r, 800.), (15., 10.));
    r.simulation_revision = 51.0.into(); r.floor_mut().unwrap().tiles[i] = "chasm".into();
    let before = r.clone(); settle_falls(&mut r).unwrap(); assert_eq!(r, before);
}

#[test]
fn stationary_illegal_or_absent_currents_keep_corpse_only_catchup_on_bulk_path() {
    for mode in ["absent", "zero", "wall", "chasm", "stair", "ichor", "grate"] {
        let mut r = room();
        if mode != "absent" {
            arrow(&mut r, 15, 10, if mode == "zero" { 0. } else { 3. });
            let to = at(&r, 16, 10); let from = at(&r, 15, 10); let f = r.floor_mut().unwrap();
            match mode {
                "wall" | "chasm" => f.tiles[to] = mode.into(),
                "stair" => f.tiles[to] = "down".into(),
                "ichor" => { let mut layer = vec![0.; f.tiles.len()]; layer[from] = 1.; f.ichor = layer.into(); },
                "grate" => f.grates = vec![to as f64].into(),
                _ => (),
            }
        }
        r.actors_at_mut(1.).push(corpse(800., 15., 10.)); r.floor_mut().unwrap().dirty = true;
        assert!(!has_activity(&r, r.floor().unwrap()), "{mode}");
        let env_cells = r.metrics.get("environmentCells").copied();
        r.depth = 3.; r.tick = 30.; crate::offscreen::catch_up_floor(&mut r, 1.).unwrap();
        assert_eq!(r.metrics.get("environmentCells").copied(), env_cells, "{mode}");
        let a = &r.actors_at(1.)[0]; assert_eq!((a.x, a.y, a.corpse_age.value_or(0.)), (15., 10., 33.));
        let f = r.floors.iter().find(|f| f.depth == 1.).unwrap(); assert!(!f.dirty); assert_eq!(f.last_tick, 30.);
    }
}
