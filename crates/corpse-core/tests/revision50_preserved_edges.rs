//! R50 preserved blessings: target admission and action rather than world-turn expiry.
//! These are deterministic native integration tests, not reference parity claims.
use corpse_core::{
    Command, Field, Item, Run, abilities, dreamwalking, engine, environment,
    replay::{ReplayRecord, ReplaySession}, temporal::{self, Engine, History, Snapshot}, vision,
};
use std::sync::Arc;

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    r.player.energy = 100.; r.player.stats.max_energy = 100.;
    r.player.abilities = vec![abilities::SPARKLE.into(), abilities::ALL_SEEING.into()];
    r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_reserves = Field::Missing;
    if revision >= 50. {
        // Explicit test-only exhausted reserves preserve player-fallback costs;
        // these zero capacities do not define production acquisition defaults.
        for id in [abilities::SPARKLE, abilities::ALL_SEEING, abilities::SLEEP,
            "morae-see-the-future"] {
            corpse_core::revision50_ability_energy::initialize_reserve(
                &mut r.player, id, 0., 0.).unwrap();
        }
    }
    r.inventory.clear(); r.offers.clear(); r.replay.clear(); r.history.clear();
    r.haste_snapshot = Field::Missing; r.future = None; r.phase = Field::Missing;
    r.replay_snapshot_from = Field::Missing;
    for actors in r.actors.values_mut() { *actors = Arc::new(vec![]); }
    for items in r.ground.values_mut() { *items = Arc::new(vec![]); }
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(0.); f.seen.fill(false);
    f.environment = Field::Missing; f.doors = Field::Missing; f.vents = Field::Missing;
    f.currents = Field::Missing; f.mud = Field::Missing; f.slime = Field::Missing;
    f.roots = Field::Missing; f.light_sources = Field::Missing; f.permanent_fire = Field::Missing;
    r
}
fn ability(id: &str) -> Command {
    Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing }
}
fn at(id: &str, x: f64, y: f64) -> Command {
    Command::Ability { id: id.into(), target: Field::Missing, x: x.into(), y: y.into() }
}
fn reveal(r: &mut Run) {
    let tick = r.tick;
    assert!(!engine::dispatch(r, &ability(abilities::ALL_SEEING)).unwrap().took_turn);
    assert_eq!(r.tick, tick);
    assert!(vision::all_seeing(r));
}
fn enemy(r: &mut Run, x: f64, y: f64) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = corpse_core::population::create_actor_revision("rat", id, x, y, false, r.simulation_revision.value_or(46.)).unwrap();
    a.state = "sleeping".into();
    r.actors_at_mut(r.depth).push(a);
    id
}
fn assert_expired(r: &Run) {
    assert!(!vision::all_seeing(r));
    assert!(!r.player.conditions.contains_key("allSeeingReveal"));
    assert!(!r.player.conditions.contains_key("allSeeingDepth"));
}
fn assert_fire(r: &Run, x: usize, y: usize) {
    let f = r.floor().unwrap(); let i = y * f.width() + x;
    assert!(f.permanent_fire.as_ref().unwrap().contains(&(i as f64)));
    let e = f.environment.as_ref().unwrap();
    assert!(e.fire[i] > 0.);
    assert_eq!(e.fire_kind.as_ref().unwrap()[i].as_deref(), Some("brazier"));
    let provenance = &e.provenance.as_ref().unwrap()["fire"];
    assert_eq!(provenance.owner[i], -2.); assert_eq!(provenance.player[i], 1.);
    assert!(f.light_sources.as_ref().unwrap().iter().any(|l|
        l.x == x as f64 && l.y == y as f64 && l.power == 100. && l.falloff == 10.));
}

#[test]
fn sparkle_accepts_every_visible_terrain_and_ignites_immediately() {
    for tile in ["floor", "wall", "chasm", "pit", "water", "shallow-water", "grass", "tree"] {
        let mut r = room(50.); reveal(&mut r);
        let i = 10 * r.floor().unwrap().width() + 15;
        r.floor_mut().unwrap().tiles[i] = tile.into();
        let energy = r.player.energy; let rng = r.rng; let tick = r.tick;
        assert!(!engine::dispatch(&mut r, &at(abilities::SPARKLE, 15., 10.)).unwrap().took_turn);
        assert_eq!(r.player.energy, energy - 20.); assert_eq!(r.rng, rng); assert_eq!(r.tick, tick);
        assert_fire(&r, 15, 10);
        assert_eq!(r.floor().unwrap().tiles[i], if matches!(tile, "tree" | "grass") { "floor" } else { tile });
        assert!(vision::all_seeing(&r), "Sparkle itself is a no-action activation");
        // Repeated placement does not duplicate sources; permanent fire survives world upkeep.
        engine::dispatch(&mut r, &at(abilities::SPARKLE, 15., 10.)).unwrap();
        assert_eq!(r.floor().unwrap().permanent_fire.as_ref().unwrap().len(), 1);
        assert_eq!(r.floor().unwrap().light_sources.as_ref().unwrap().len(), 1);
        engine::dispatch(&mut r, &Command::Wait).unwrap();
        assert_fire(&r, 15, 10); assert_expired(&r);
    }
}

#[test]
fn legacy_sparkle_keeps_terrain_filter_and_ordinary_fire_keeps_it_in_r50() {
    for tile in ["wall", "chasm", "pit", "water", "shallow-water"] {
        let mut r = room(46.); reveal(&mut r);
        let i = 10 * r.floor().unwrap().width() + 15;
        r.floor_mut().unwrap().tiles[i] = tile.into();
        let before = serde_json::to_value(&r).unwrap();
        abilities::activate(&mut r, abilities::SPARKLE, &Field::Missing, &15.0.into(), &10.0.into()).unwrap();
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
        let mut r = room(50.); r.floor_mut().unwrap().tiles[i] = tile.into();
        let id = r.player.id;
        environment::ignite(&mut r, 15., 10., None, Some(id)).unwrap();
        assert!(r.floor().unwrap().environment.as_ref().is_none_or(|e|e.fire[i] == 0.));
        assert!(r.floor().unwrap().permanent_fire.is_missing());
    }
}

#[test]
fn sparkle_rejects_unseen_invalid_and_unfunded_targets_without_mutation() {
    for (x, y) in [(-1., 10.), (10.5, 10.), (f64::NAN, 10.), (10., f64::INFINITY), (500., 10.)] {
        let mut r = room(50.); let before = serde_json::to_value(&r).unwrap();
        abilities::activate(&mut r, abilities::SPARKLE, &Field::Missing, &x.into(), &y.into()).unwrap();
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
    }
    let mut r = room(50.); let width = r.floor().unwrap().width(); let height = r.floor().unwrap().height();
    for y in 0..height { r.floor_mut().unwrap().tiles[y * width + 15] = "wall".into(); }
    assert!(!vision::visible(&r, 20, 10));
    let before = serde_json::to_value(&r).unwrap();
    abilities::activate(&mut r, abilities::SPARKLE, &Field::Missing, &20.0.into(), &10.0.into()).unwrap();
    assert_eq!(serde_json::to_value(&r).unwrap(), before);
    reveal(&mut r); r.player.energy = 19.;
    // An unsuccessful paid activation may append its existing diagnostic, but cannot place or pay.
    let before_floor = r.floor().unwrap().clone(); let rng = r.rng;
    engine::dispatch(&mut r, &at(abilities::SPARKLE, 20., 10.)).unwrap();
    assert_eq!(r.floor().unwrap().permanent_fire, before_floor.permanent_fire);
    assert_eq!(r.floor().unwrap().environment, before_floor.environment);
    assert_eq!(r.player.energy, 19.); assert_eq!(r.rng, rng); assert!(vision::all_seeing(&r));
}

#[test]
fn dream_sparkle_uses_only_its_current_plane_on_noncombustible_terrain() {
    let mut r = room(50.); let i = 10 * r.floor().unwrap().width() + 15;
    r.floor_mut().unwrap().tiles[i] = "wall".into();
    r.dreamwalking = dreamwalking::State { unlocked: true, ..Default::default() }.into();
    assert_eq!(dreamwalking::ability(&mut r, dreamwalking::DREAMWALK_ID).unwrap(), Some(true));
    reveal(&mut r);
    let waking = r.floor().unwrap().clone();
    engine::dispatch(&mut r, &at(abilities::SPARKLE, 15., 10.)).unwrap();
    assert_eq!(r.floor().unwrap().tiles, waking.tiles);
    assert_eq!(r.floor().unwrap().environment, waking.environment);
    assert_eq!(r.floor().unwrap().light_sources, waking.light_sources);
    assert_eq!(r.floor().unwrap().permanent_fire, waking.permanent_fire);
    dreamwalking::with_plane(&mut r, true, |r| assert_fire(r, 15, 10));
}

#[test]
fn reveal_persists_through_explicitly_no_action_commands_and_failed_actions() {
    let mut r = room(50.);
    r.player.abilities.extend(["isheilah-tools-of-the-trade".into(), "numo-mind-read".into()]);
    for (id, kind, effect) in [(9001., "potion", Some("healing")), (9002., "heart", None)] {
        r.inventory.push(Item { id, name: kind.into(), kind: kind.into(), glyph: "!".into(),
            effect: effect.map(str::to_string).map(Field::Value).unwrap_or(Field::Missing), ..Default::default() });
    }
    reveal(&mut r); let tick = r.tick;
    let i = 10 * r.floor().unwrap().width() + 11;
    r.floor_mut().unwrap().tiles[i] = "wall".into();
    for command in [Command::Use { id: 9001. }, Command::Use { id: 9002. },
        ability("numo-mind-read"), Command::Pickup, Command::Rally { x: 10., y: 10. },
        Command::Move { dx: 1., dy: 0. }, Command::Use { id: 99999. },
        at(abilities::SPARKLE, -1., 10.)] {
        assert!(!engine::dispatch(&mut r, &command).unwrap().took_turn);
        assert!(vision::all_seeing(&r), "{command:?}"); assert_eq!(r.tick, tick);
    }
    assert!(r.inventory.is_empty());
    engine::dispatch(&mut r, &Command::Wait).unwrap(); assert_expired(&r);
}

#[test]
fn ordinary_consumption_and_action_abilities_expire_reveal() {
    for action in [Command::Use { id: 9001. }, ability(abilities::SLEEP)] {
        let mut r = room(50.); r.player.abilities.push(abilities::SLEEP.into());
        r.inventory.push(Item { id: 9001., name: "Heart".into(), kind: "heart".into(), glyph: "%".into(), ..Default::default() });
        reveal(&mut r);
        assert!(engine::dispatch(&mut r, &action).unwrap().took_turn);
        assert_expired(&r);
    }
}

#[test]
fn haste_and_came_and_pursuit_moves_finish_actions_without_world_turns() {
    for mode in ["haste", "came", "pursuit"] {
        let mut r = room(50.);
        match mode {
            "haste" => { r.player.conditions.insert("hasted".into(), 5.); }
            "came" => {
                let id = enemy(&mut r, 20., 10.);
                r.player.abilities.push("pelian-i-came-for-you".into());
                r.player.conditions.insert("cameTarget".into(), id);
                r.player.conditions.insert("cameTurns".into(), 1.);
            }
            "pursuit" => { r.player.conditions.insert("r50PursuitCredit".into(), 1.); }
            _ => unreachable!(),
        }
        reveal(&mut r); let tick = r.tick;
        let outcome = engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
        assert!(!outcome.took_turn, "{mode}"); assert_eq!(r.tick, tick, "{mode}");
        assert_eq!((r.player.x, r.player.y), (11., 10.)); assert_expired(&r);
    }
    let mut r = room(46.); r.player.conditions.insert("hasted".into(), 5.); reveal(&mut r);
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert!(vision::all_seeing(&r), "historical expiry follows the world-turn boundary");
}

#[test]
fn weave_movement_expires_reveal_and_snapshot_restoration_keeps_original_boundary() {
    let mut r = room(50.); r.player.abilities.push(corpse_core::progression::WEAVE_ID.into());
    engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
    reveal(&mut r); let tick = r.tick;
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: -1., dy: 0. }).unwrap().took_turn);
    assert_eq!(r.tick, tick); assert_expired(&r);
    snapshot.restore(&mut r).unwrap(); assert!(vision::all_seeing(&r)); assert_eq!(r.player.x, 11.);
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: -1., dy: 0. }).unwrap().took_turn);
    assert_expired(&r);
}

#[test]
fn future_save_load_and_seed_replay_restore_reveal_then_expire_it_once() {
    let mut initial = room(50.);
    initial.player.abilities.extend(["morae-see-the-future".into(), corpse_core::progression::WEAVE_ID.into()]);
    initial.player.conditions.insert("hasted".into(), 5.);
    let mut e = Engine::new(initial.clone()).unwrap();
    let commands = [ability(abilities::ALL_SEEING), ability("morae-see-the-future"),
        Command::Move { dx: 1., dy: 0. }, ability("morae-see-the-future"),
        Command::Move { dx: 0., dy: 1. }];
    for (index, command) in commands.iter().enumerate() {
        e.dispatch(command).unwrap();
        assert_eq!(vision::all_seeing(&e.run), matches!(index, 0 | 1 | 3));
        e = Engine::load(&e.save().unwrap()).unwrap();
        assert_eq!(vision::all_seeing(&e.run), matches!(index, 0 | 1 | 3));
        assert!(e.history.frames.is_empty()); assert!(e.history.pending_haste.is_none());
    }
    let expected = e.save().unwrap();
    let mut replay = ReplaySession::from_initial(initial, ReplayRecord::try_from_run(&e.run).unwrap()).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    replay.restart().unwrap(); while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
}

#[test]
fn preexisting_future_and_raw_save_preserve_reveal() {
    let mut r = room(50.);
    r.player.conditions.insert("hasted".into(), 5.);
    reveal(&mut r);
    let mut history = History::default();
    // This lower-level fixture complements the admitted Engine trace above.
    temporal::remember_future(&mut r, &history).unwrap();
    assert!(vision::all_seeing(&r));
    let anchor = corpse_core::storage::deserialize(r.future.as_ref().unwrap().wire_text().unwrap()).unwrap();
    assert!(vision::all_seeing(&anchor));
    // Raw canonical serialization independently exercises the persisted state.
    let raw = corpse_core::storage::serialize(&r).unwrap();
    let mut r = corpse_core::storage::deserialize(&raw).unwrap();
    assert!(vision::all_seeing(&r));
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert_expired(&r);
    assert!(temporal::restore_future(&mut r, &mut history).unwrap());
    assert!(vision::all_seeing(&r)); assert_eq!(r.player.x, 10.);
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 0., dy: 1. }).unwrap().took_turn);
    assert_expired(&r);
}

#[test]
fn sparkle_admits_occupied_squares_and_reveal_shows_hidden_creatures() {
    let mut r = room(50.);
    let id = enemy(&mut r, 20., 10.);
    let width = r.floor().unwrap().width(); let height = r.floor().unwrap().height();
    for y in 0..height { r.floor_mut().unwrap().tiles[y * width + 15] = "wall".into(); }
    assert!(!vision::visible_actor(&r, engine::actor(&r, id).unwrap()));
    reveal(&mut r);
    assert!(vision::visible_actor(&r, engine::actor(&r, id).unwrap()));
    assert!(vision::terrain_visibility(&r).iter().all(|v| *v));
    engine::dispatch(&mut r, &at(abilities::SPARKLE, 20., 10.)).unwrap();
    assert_fire(&r, 20, 10);
    assert!(vision::all_seeing(&r));
}

#[test]
fn reveal_stays_active_through_final_observation_of_a_bonus_action() {
    let mut r = room(50.);
    let width = r.floor().unwrap().width(); let height = r.floor().unwrap().height();
    for y in 0..height { r.floor_mut().unwrap().tiles[y * width + 15] = "wall".into(); }
    r.player.conditions.insert("hasted".into(), 5.);
    reveal(&mut r);
    // These changes happen after activation, so only the next action's final
    // observation can remember them despite the obstructing wall.
    r.floor_mut().unwrap().tiles[10 * width + 20] = "grass".into();
    r.ground_at_mut(1.).push(corpse_core::GroundItem { x: 20., y: 10.,
        item: Item { id: 9001., name: "Heart".into(), kind: "heart".into(), glyph: "%".into(), ..Default::default() },
        ..Default::default() });
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert_expired(&r);
    assert_eq!(r.floor().unwrap().remembered_tiles.as_ref().unwrap()[10 * width + 20], "grass");
    assert!(r.remembered_items.as_ref().unwrap()["1"].iter().any(|e|e.item.id == 9001.));
}
