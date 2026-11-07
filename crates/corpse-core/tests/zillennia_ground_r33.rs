//! Ground contact is not an erasing attack or an area effect.
use corpse_core::{
    ActorLastSeen, ActorMonsterState, Command, Field, Floor, GroundItem, Item, Run,
    engine, environment, population, startup, storage, temporal, zillennia_ground as ground,
};
use std::sync::Arc;
const W: usize = 79;
const H: usize = 29;
const ID: f64 = 4.0;
fn room(quality_depth: u32, revision: f64) -> Run {
    let mut r = startup::new_run_shell_revision("0-5-11:zillennia-ground", "Unblessed", false, revision).unwrap();
    r.player.x = 2.0;
    r.player.y = 5.0;
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.player.ability_order = Field::Missing;
    let mut tiles = vec!["wall".into(); W * H];
    for y in 1..H - 1 { for x in 1..W - 1 { tiles[y * W + x] = "floor".into(); } }
    let mut f = Floor { width: (W as f64).into(), height: (H as f64).into(), depth: 1.0,
        domain: "Dungeon".into(), tiles, lights: vec![100.0; W * H], seen: vec![true; W * H], ..Default::default() };
    environment::initialize(&mut f).unwrap();
    r.floors = vec![Arc::new(f)];
    let mut a = population::zillennia_revision(quality_depth, revision).unwrap();
    a.id = ID; a.x = 8.0; a.y = 5.0; a.state = "hunting".into();
    a.abilities.clear(); a.active.clear(); a.conditions.clear();
    a.conditions.insert("bestiaryTraitsInitialized".into(), 1.0);
    a.last_seen = ActorLastSeen { x: 2.0, y: 5.0 }.into();
    r.actors.clear(); r.actors.insert("1".into(), Arc::new(vec![a]));
    r.ground.clear(); r.ground.insert("1".into(), Arc::new(vec![]));
    r.inventory.clear(); r.next_id = 100.0; r.generated_ability_grants = Vec::new().into();
    r
}
fn item(id: f64, kind: &str) -> Item {
    Item { id, name: format!("test {kind}"), kind: kind.into(), glyph: ";".into(), ..Default::default() }
}
fn put(r: &mut Run, x: usize, y: usize, id: f64, kind: &str) {
    r.ground_at_mut(r.depth).push(GroundItem { x: x as f64, y: y as f64, item: item(id, kind), ..Default::default() });
}
fn cover(r: &mut Run, x: usize, y: usize) {
    let f = r.floor_mut().unwrap(); let at = y * W + x; let n = f.tiles.len();
    f.tiles[at] = "grass".into();
    f.snow.get_or_insert_with(|| vec![false; n])[at] = true;
    f.slime.get_or_insert_with(|| vec![0.0; n])[at] = 1.0;
    f.blood.get_or_insert_with(|| vec![false; n])[at] = true;
}
fn covered(r: &Run, at: usize) -> bool {
    let f = r.floor().unwrap();
    f.tiles[at] == "grass" && f.snow.as_ref().unwrap()[at]
        && f.slime.as_ref().unwrap()[at] == 1.0 && f.blood.as_ref().unwrap()[at]
}
fn cleared(r: &Run, at: usize) -> bool {
    let f = r.floor().unwrap();
    f.tiles[at] == "floor" && !f.snow.as_ref().unwrap()[at]
        && f.slime.as_ref().unwrap()[at] == 0.0 && !f.blood.as_ref().unwrap()[at]
}
#[test]
fn quality_cutoff_is_greater_not_current_depth_and_legacy_is_unchanged() {
    for revision in [29.0, 30.0, 31.0, 32.0, 33.0] {
        for quality in 1..=40 {
            let mut r = room(quality, revision); cover(&mut r, 8, 5); put(&mut r, 8, 5, 50.0, "heart");
            let before = r.clone(); let expected = revision >= 33.0 && quality >= 21;
            assert_eq!(ground::clears_ground(&r, engine::actor(&r, ID).unwrap()), expected);
            assert_eq!(ground::on_arrival(&mut r, ID), expected, "revision {revision}, quality depth {quality}");
            if expected { assert!(r.ground["1"].is_empty()); assert!(cleared(&r, 5 * W + 8)); }
            else { assert_eq!(r, before); }
        }
    }
}
#[test]
fn ordinary_arrival_clears_all_loose_items_but_not_corpses_loot_inventory_or_adjacent_tiles() {
    let mut r = room(21, 33.0); cover(&mut r, 7, 5); cover(&mut r, 7, 6);
    for (id, kind) in [(50.0, "heart"), (51.0, "potion"), (52.0, "weapon")] { put(&mut r, 7, 5, id, kind); }
    put(&mut r, 7, 6, 53.0, "heart");
    r.inventory.push(item(54.0, "heart"));
    let mut corpse = population::create_actor_revision("rat", 55.0, 7.0, 5.0, false, 33.0).unwrap();
    corpse.dead = true.into(); corpse.hp = 0.0;
    corpse.monster_state = ActorMonsterState { carried_items: vec![item(56.0, "heart")].into(), ..Default::default() }.into();
    r.actors_at_mut(1.0).push(corpse.clone());
    let mut carried = ActorMonsterState::default(); carried.carried_items = vec![item(57.0, "potion")].into();
    engine::actor_mut(&mut r, ID).unwrap().monster_state = carried.clone().into();
    let rng = r.rng;
    assert!(engine::move_actor(&mut r, ID, -1.0, 0.0, false).unwrap());
    assert_eq!(r.ground["1"].iter().map(|g| g.item.id).collect::<Vec<_>>(), vec![53.0]);
    assert!(cleared(&r, 5 * W + 7)); assert!(covered(&r, 6 * W + 7));
    assert_eq!(engine::actor(&r, 55.0).unwrap(), &corpse);
    assert_eq!(engine::actor(&r, ID).unwrap().monster_state.as_ref(), Some(&carried));
    assert_eq!(r.inventory, vec![item(54.0, "heart")]); assert_eq!(r.rng, rng);
    assert!(r.floor().unwrap().dirty);
}
#[test]
fn living_occupancy_blocks_contact_instead_of_erasing_or_killing_anybody() {
    let mut r = room(40, 33.0); cover(&mut r, 7, 5); put(&mut r, 7, 5, 50.0, "heart");
    let rat = population::create_actor_revision("rat", 55.0, 7.0, 5.0, false, 33.0).unwrap();
    r.actors_at_mut(1.0).push(rat.clone());
    assert!(!engine::move_actor(&mut r, ID, -1.0, 0.0, true).unwrap());
    assert_eq!(engine::actor(&r, 55.0).unwrap(), &rat); assert!(covered(&r, 5 * W + 7));
    assert_eq!(r.ground["1"].len(), 1);
    // Even a deliberately overlapping fixture does not mutate a living body.
    engine::actor_mut(&mut r, ID).unwrap().x = 7.0;
    assert!(ground::on_arrival(&mut r, ID)); assert_eq!(engine::actor(&r, 55.0).unwrap(), &rat);
}
#[test]
fn water_ichor_topology_structures_and_environment_are_untouched() {
    for tile in ["water", "shallow-water", "wall", "tree", "up", "down", "chasm", "pit", "floor"] {
        let mut r = room(21, 33.0); cover(&mut r, 8, 5); put(&mut r, 8, 5, 50.0, "heart");
        let f = r.floor_mut().unwrap(); let at = 5 * W + 8;
        f.tiles[at] = tile.into(); f.ichor = vec![2.0; W * H].into();
        f.water_blood = vec![3.0; W * H].into(); f.kelpie_blood_sources = vec![4.0; W * H].into();
        f.bloody_walls = vec![true; W * H].into();
        f.doors = indexmap::IndexMap::from([(at.to_string(), corpse_core::FloorDoorsEntry { open: false, ..Default::default() })]).into();
        let mut expected = f.clone(); expected.dirty = true;
        assert!(ground::on_arrival(&mut r, ID)); assert!(r.ground["1"].is_empty());
        assert_eq!(r.floor().unwrap(), &expected, "{tile}");
    }
    // Snow/slime/blood on un-Ichor'd stairs are also outside the conservative land-cover list.
    let mut r = room(21, 33.0); cover(&mut r, 8, 5); r.floor_mut().unwrap().tiles[5 * W + 8] = "down".into();
    let before = r.clone(); assert!(!ground::on_arrival(&mut r, ID)); assert_eq!(r, before);
}
#[test]
fn forced_current_and_haste_paths_clear_only_arrival_tiles() {
    let mut current = room(21, 33.0);
    for x in [8, 9] { current.floor_mut().unwrap().tiles[5 * W + x] = "water".into(); }
    current.floor_mut().unwrap().currents = vec![0.0; W * H].into();
    current.floor_mut().unwrap().currents.as_mut().unwrap()[5 * W + 8] = 3.0;
    put(&mut current, 9, 5, 50.0, "heart");
    corpse_core::currents::tick_currents(&mut current).unwrap();
    assert_eq!(engine::actor(&current, ID).unwrap().x, 9.0); assert!(current.ground["1"].is_empty());
    assert_eq!(current.floor().unwrap().tiles[5 * W + 9], "water");
    let mut r = room(21, 33.0);
    for x in [6, 7] { cover(&mut r, x, 5); put(&mut r, x, 5, (50 + x) as f64, "heart"); }
    engine::actor_mut(&mut r, ID).unwrap().conditions.insert("hasted".into(), 10.0);
    engine::dispatch(&mut r, &Command::Wait).unwrap();
    assert_eq!(engine::actor(&r, ID).unwrap().x, 6.0);
    assert!(r.ground["1"].is_empty()); assert!(cleared(&r, 5 * W + 6)); assert!(cleared(&r, 5 * W + 7));
}
#[test]
fn initial_current_tile_clears_without_motion_and_dead_zillennia_do_not_clear() {
    let mut r = room(21, 33.0); cover(&mut r, 8, 5); put(&mut r, 8, 5, 50.0, "heart");
    let mut f = (*r.floors[0]).clone(); f.depth = 2.0; r.floors.push(Arc::new(f));
    r.depth = 2.0;
    assert!(ground::initialize_floor(&mut r, 1.0)); assert!(r.ground["1"].is_empty());
    assert_eq!(r.floors[0].tiles[5 * W + 8], "floor");
    assert_eq!(r.floors[1].tiles[5 * W + 8], "grass");
    r.depth = 1.0; cover(&mut r, 8, 5); put(&mut r, 8, 5, 51.0, "heart");
    engine::actor_mut(&mut r, ID).unwrap().dead = true.into();
    let before = r.clone(); assert!(!ground::initialize_floor(&mut r, 1.0)); assert_eq!(r, before);
}
#[test]
fn offscreen_catchup_preserves_knowledge_until_the_cell_is_observed_again() {
    let mut r = room(21, 33.0); cover(&mut r, 7, 5); put(&mut r, 7, 5, 50.0, "heart");
    engine::remember(&mut r);
    let knowledge = r.remembered_items.clone(); let remembered_tiles = r.floor().unwrap().remembered_tiles.clone();
    let mut other = (*r.floors[0]).clone(); other.depth = 2.0;
    r.floors.push(Arc::new(other)); r.actors.insert("2".into(), Arc::new(vec![]));
    r.ground.insert("2".into(), Arc::new(vec![])); r.depth = 2.0; r.tick = 1.0;
    corpse_core::offscreen::catch_up_floor(&mut r, 1.0).unwrap();
    assert!(r.ground["1"].is_empty()); assert_eq!(r.floors[0].tiles[5 * W + 7], "floor");
    assert_eq!(r.remembered_items, knowledge); assert_eq!(r.floors[0].remembered_tiles, remembered_tiles);
    r.depth = 1.0; engine::remember(&mut r);
    assert!(r.remembered_items.as_ref().unwrap()["1"].iter().all(|g| g.item.id != 50.0));
    assert_eq!(r.floor().unwrap().remembered_tiles.as_ref().unwrap()[5 * W + 7], "floor");
}
#[test]
fn save_revert_future_and_replay_restore_the_same_erasure_state() {
    let mut initial = room(21, 33.0); cover(&mut initial, 7, 5); put(&mut initial, 7, 5, 50.0, "heart");
    initial.player.conditions.insert("revertCharges".into(), 3.0);
    let mut e = temporal::Engine::new(initial.clone()).unwrap(); e.dispatch(&Command::Wait).unwrap();
    assert!(e.run.ground["1"].is_empty()); assert!(cleared(&e.run, 5 * W + 7));
    let reloaded = temporal::Engine::load(&e.save().unwrap()).unwrap();
    assert_eq!(reloaded.run.ground, e.run.ground); assert_eq!(reloaded.run.floors, e.run.floors);
    let record = corpse_core::replay::ReplayRecord::from_run(&e.run);
    let mut replay = corpse_core::replay::ReplaySession::from_initial(initial.clone(), record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.run.ground, e.run.ground); assert_eq!(replay.engine.run.floors, e.run.floors);
    let mut r = storage::deserialize(&storage::serialize(&e.run).unwrap()).unwrap();
    let mut history = temporal::History::default(); history.record_completed(temporal::Snapshot::capture(&initial));
    history.closing = Some(temporal::Snapshot::capture(&r));
    assert!(temporal::revert(&mut r, &mut history).unwrap());
    assert_eq!(r.ground, initial.ground); assert!(covered(&r, 5 * W + 7));
    temporal::remember_future(&mut r, &history).unwrap();
    assert!(engine::move_actor(&mut r, ID, -1.0, 0.0, true).unwrap()); assert!(r.ground["1"].is_empty());
    assert!(temporal::restore_future(&mut r, &mut history).unwrap());
    assert_eq!(r.ground, initial.ground); assert!(covered(&r, 5 * W + 7));
}
#[test]
fn float_away_displacement_also_resolves_ground_contact() {
    let mut r = room(21, 33.0); cover(&mut r, 9, 5); put(&mut r, 9, 5, 50.0, "heart");
    engine::actor_mut(&mut r, ID).unwrap().active.push("etill-float-away".into());
    let player = r.player.id;
    engine::damage_environment(&mut r, ID, 1.0, Some(player), "attack", true).unwrap();
    assert_eq!(engine::actor(&r, ID).unwrap().x, 9.0);
    assert!(r.ground["1"].is_empty()); assert!(cleared(&r, 5 * W + 9));
}
#[test]
fn real_startup_population_clears_a_deep_zillennia_spawn_tile() {
    let mut r = room(21, 33.0);
    let mut f = (*r.floors[0]).clone();
    f.depth = 21.0;
    f.seen.fill(false);
    let at = f.tiles.iter().position(|tile| tile == "floor").unwrap();
    let (x, y) = ((at % W) as f64, (at / W) as f64);
    f.tiles[at] = "grass".into();
    f.godbeast_event = corpse_core::GodbeastEvent { kind: "Zillennia".into(), x, y, ..Default::default() }.into();
    r.floors = vec![Arc::new(f)]; r.actors.clear(); r.ground.clear();
    startup::spawn_floor(&mut r, 21).unwrap();
    assert!(r.actors_at(21.0).iter().any(|a| a.creature_kind.as_ref().is_some_and(|k| k == "zillennia") && a.x == x && a.y == y));
    assert_eq!(r.floors[0].tiles[at], "floor");
    assert!(!r.floors[0].dirty, "generation must not activate an unvisited floor");
    r.tick = 50.0;
    let actors = r.actors_at(21.0).to_vec();
    corpse_core::offscreen::catch_up_floor(&mut r, 21.0).unwrap();
    assert_eq!(r.actors_at(21.0), actors);
}
#[test]
fn cleared_cover_spares_existing_gas_and_empty_contact_is_a_true_noop() {
    let mut r = room(21, 33.0); cover(&mut r, 8, 5); let at = 5 * W + 8;
    environment::add_gas(r.floor_mut().unwrap(), at, "caustic", 25.0, -2.0).unwrap();
    let gas = r.floor().unwrap().environment.clone();
    assert!(ground::on_arrival(&mut r, ID)); assert!(cleared(&r, at));
    assert_eq!(r.floor().unwrap().environment, gas);
    let before = r.clone(); assert!(!ground::on_arrival(&mut r, ID)); assert_eq!(r, before);
    let mut empty = room(21, 33.0); empty.ground.clear();
    let before = empty.clone(); assert!(!ground::on_arrival(&mut empty, ID)); assert_eq!(empty, before);
}
