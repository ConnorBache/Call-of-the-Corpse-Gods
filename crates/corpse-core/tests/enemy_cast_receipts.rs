//! Successful casts and paid misses through the production enemy dispatcher.
use corpse_core::{
    engine, non_euclidean_generation::{NonEuclideanGroup, NonEuclideanPocket},
    population, projectiles, Field, Run,
};

fn pocket_room() -> Run {
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 8.;
    r.player.y = 10.;
    r.player.hp = 50.;
    r.player.max_hp = 100.;
    r.player.stats.perception = 300.;
    r.player.conditions.clear();
    r.player.abilities.clear();
    r.player.active.clear();
    r.log.clear();
    r.log_kinds = Field::Missing;
    r.log_archive = Field::Missing;
    r.actors_at_mut(r.depth).clear();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into());
    f.lights.fill(100.);
    f.doors = Field::Missing;
    f.portals = Field::Missing;
    f.environment = Field::Missing;
    let w = f.width();
    f.non_euclidean = vec![NonEuclideanPocket {
        cells: vec![10 * w + 10, 10 * w + 11, 11 * w + 10],
        groups: vec![NonEuclideanGroup {
            sources: vec![10 * w + 10, 11 * w + 10],
            destination: 10 * w + 11,
        }],
    }].into();
    r
}

#[test]
fn paid_dryad_heal_reports_miss_without_claiming_or_duplicating_a_heal() {
    let id = 900.;
    for (seed, lands) in [(1, true), (8192, false)] {
        let mut r = pocket_room();
        let mut a = population::create_actor_revision("dryad", id, 11., 10., false, 52.).unwrap();
        a.state = "ally".into();
        a.energy = 25.;
        r.actors_at_mut(r.depth).push(a);
        assert_eq!(projectiles::possible_shot_range(&r, engine::actor(&r, id).unwrap(), &r.player, 6.), Some(2.));
        r.rng = seed as f64;
        assert!(engine::monster_ai_turn(&mut r, id, false).unwrap());
        let caster = engine::actor(&r, id).unwrap();
        assert_eq!(caster.energy, 20.);
        assert_eq!(caster.conditions["monsterHealReadyAt"], r.tick + 2.);
        assert_eq!(r.player.hp > 50., lands);
        let message = if lands { "Dryad healed you." } else { "Dryad uses Healing Grove." };
        assert_eq!(r.log.iter().filter(|s| s.contains("Dryad")).collect::<Vec<_>>(), [message]);
        assert!(r.log_archive.as_ref().unwrap().messages.iter().any(|s| s == message));
    }
}

fn horn_room(current: bool) -> (Run, f64) {
    let mut r = pocket_room();
    r.player.x = 10.;
    r.tick = 1.;
    if current { corpse_core::stone_seal_rules::admit_current(&mut r).unwrap(); }
    let origin = r.depth;
    let dest = corpse_core::fast_mode::below(&r, origin);
    let f = r.floor_mut().unwrap();
    f.non_euclidean = Field::Missing;
    f.godbeast_event = Field::Missing;
    f.last_tick = 1.;
    let below = f.clone();
    r.floors.retain(|f| f.depth != dest && f.depth != dest + 1.);
    for depth in [dest, dest + 1.] {
        let mut floor = below.clone();
        floor.depth = depth;
        r.floors.push(std::sync::Arc::new(floor));
        r.ground_at_mut(depth).clear();
    }
    r.floors.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    r.actors_at_mut(dest).clear();
    r.actors_at_mut(dest + 1.).clear();
    let id = 900.;
    let mut horn = population::create_actor_revision("horn-5", id, 13., 10., false, 52.).unwrap();
    horn.state = "hunting".into();
    horn.energy = 100.;
    r.next_id = id + 1.;
    r.actors_at_mut(origin).push(horn);
    (r, id)
}

#[test]
fn stone_seal_expires_at_its_origin_when_caster_falls_not_on_the_destination() {
    let (mut r, id) = horn_room(true);
    let source = r.depth;
    let dest = corpse_core::fast_mode::below(&r, source);
    let player = r.player.id;
    assert!(corpse_core::monster_horn::special_action(&mut r, id, player).unwrap());
    let caster = engine::actor(&r, id).unwrap();
    assert_eq!(caster.energy, 85.);
    assert_eq!(caster.conditions["raisedWallExpires"], 6.);
    assert!(!caster.conditions.contains_key("raisedWallIndex"));
    let floor = r.floor().unwrap();
    let seal = &floor.stone_seals.as_ref().unwrap()[0];
    let cell = seal.cell;
    let cast_tick = seal.expires_at;
    assert_eq!(floor.tiles[cell], "wall");
    let json = serde_json::to_string(&r).unwrap();
    let mut r: Run = serde_json::from_str(&json).unwrap();
    assert!(corpse_core::stone_seal_rules::current(&r));
    let f = r.floor_mut().unwrap();
    let w = f.width();
    f.tiles[10 * w + 13] = "chasm".into();
    corpse_core::travel::fall_actors_one_level(&mut r, source, &[id], None).unwrap();
    assert!(r.actors_at(dest).iter().any(|a| a.id == id && a.hp > 0.));
    r.tick = cast_tick - 1.;
    corpse_core::monster_horn::world_turn(&mut r);
    assert_eq!(r.floor().unwrap().tiles[cell], "wall");

    r.depth = dest;
    r.floor_mut().unwrap().tiles[cell] = "wall".into(); // independent ordinary wall
    r.tick = cast_tick;
    corpse_core::monster_horn::world_turn(&mut r);
    assert_eq!(r.floor().unwrap().tiles[cell], "wall");
    assert!(!engine::actor(&r, id).unwrap().conditions.contains_key("raisedWallExpires"));

    r.depth = dest + 1.;
    r.local_floor_ticks_from = 0usize.into();
    corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();
    assert_eq!(r.floors.iter().find(|f| f.depth == source).unwrap().tiles[cell], "wall",
        "a floor two away is not simulated");
    r.depth = dest;
    corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();
    let origin = r.floors.iter().find(|f| f.depth == source).unwrap();
    assert_eq!(origin.tiles[cell], "floor", "an adjacent simulated floor expires its own seal");
    assert!(origin.stone_seals.is_missing());
    assert_eq!(r.floor().unwrap().tiles[cell], "wall");
}

#[test]
fn stone_seal_legacy_cast_keeps_old_storage_and_expires_on_its_original_plane() {
    let (mut r, id) = horn_room(false);
    let player = r.player.id;
    assert!(corpse_core::monster_horn::special_action(&mut r, id, player).unwrap());
    let caster = engine::actor(&r, id).unwrap();
    let cell = caster.conditions["raisedWallIndex"] as usize;
    assert_eq!(caster.conditions["raisedWallExpires"], 6.);
    assert!(!corpse_core::stone_seal_rules::current(&r));
    assert!(r.floor().unwrap().stone_seals.is_missing());
    let mut historical = r.clone();
    historical.tick = 6.;
    corpse_core::monster_horn::world_turn(&mut historical);
    assert_eq!(historical.floor().unwrap().tiles[cell], "floor");

    // Migration cannot infer where the cast originated if its caster fell.
    let origin = r.depth;
    let dest = corpse_core::fast_mode::below(&r, origin);
    let f = r.floor_mut().unwrap();
    let w = f.width();
    f.tiles[10 * w + 13] = "chasm".into();
    corpse_core::travel::fall_actors_one_level(&mut r, origin, &[id], None).unwrap();
    r.depth = dest;
    r.floor_mut().unwrap().tiles[cell] = "wall".into();
    corpse_core::stone_seal_rules::admit_current(&mut r).unwrap();
    r.tick = 6.;
    corpse_core::monster_horn::world_turn(&mut r);
    assert_eq!(r.floor().unwrap().tiles[cell], "wall", "unrelated destination wall remains");
    assert!(engine::actor(&r, id).unwrap().conditions.get("raisedWallExpires").is_none());
}

#[test]
fn stone_seal_replay_boundary_applies_at_command_index_and_survives_restart() {
    use corpse_core::{Command, replay::{ReplayRecord, ReplaySession}};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/gas-prechange-r52.json")).unwrap();
    let mut record: ReplayRecord = serde_json::from_value(fixture["record"].clone()).unwrap();
    record.commands = vec![Command::Wait, Command::Wait];
    record.stone_seal_from = 1usize.into();
    let mut session = ReplaySession::create(record, true).unwrap();
    for pass in 0..2 {
        if pass > 0 { session.restart().unwrap(); }
        assert_eq!(session.engine.run.stone_seal_from, Field::Null);
        assert!(!corpse_core::stone_seal_rules::current(&session.engine.run));
        assert!(session.step().unwrap());
        assert_eq!(session.engine.run.stone_seal_from, Field::Value(1));
        assert!(session.step().unwrap());
        assert_eq!(session.engine.run.stone_seal_from, Field::Value(1));
        assert!(!session.step().unwrap());
    }
}

#[test]
fn stone_seal_is_not_copied_into_a_new_dream_or_allowed_to_erase_a_shattered_replacement() {
    let (mut r, id) = horn_room(true);
    let player = r.player.id;
    assert!(corpse_core::monster_horn::special_action(&mut r, id, player).unwrap());
    let room = r.floor().unwrap();
    let seal = room.stone_seals.as_ref().unwrap()[0].clone();
    let w = room.width();
    corpse_core::dreamwalking::with_plane(&mut r, true, |dream| {
        assert_eq!(dream.floor().unwrap().tiles[seal.cell], "floor");
        assert!(dream.floor().unwrap().stone_seals.is_missing());
    });
    assert_eq!(r.floor().unwrap().tiles[seal.cell], "wall");
    assert!(corpse_core::revision50_potions::apply(
        &mut r, "shattering", None, (seal.cell % w) as f64, (seal.cell / w) as f64, 1., false,
    ).unwrap());
    assert_eq!(r.floor().unwrap().tiles[seal.cell], "floor");
    assert!(r.floor().unwrap().stone_seals.is_missing());
    corpse_core::revision50_control_amulets::add_temporary_terrain(&mut r, &[seal.cell], "wall", 10.);
    r.tick = seal.expires_at;
    corpse_core::monster_horn::world_turn(&mut r);
    assert_eq!(r.floor().unwrap().tiles[seal.cell], "wall");
}

#[test]
fn stone_seal_rewind_and_save_restore_keep_the_due_terrain_on_the_casting_floor() {
    use corpse_core::temporal::{Engine, Snapshot};
    let (mut r, id) = horn_room(true);
    let player = r.player.id;
    assert!(corpse_core::monster_horn::special_action(&mut r, id, player).unwrap());
    let seal = r.floor().unwrap().stone_seals.as_ref().unwrap()[0].clone();
    let save = serde_json::to_string(&r).unwrap();
    let mut r = Engine::load(&save).unwrap().run;
    assert_eq!(r.floor().unwrap().stone_seals.as_ref().unwrap(), &vec![seal.clone()]);
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    r.tick = seal.expires_at;
    corpse_core::monster_horn::world_turn(&mut r);
    assert!(r.floor().unwrap().stone_seals.is_missing());
    snapshot.restore(&mut r).unwrap();
    assert!(corpse_core::stone_seal_rules::current(&r));
    assert_eq!(r.floor().unwrap().stone_seals.as_ref().unwrap(), &vec![seal.clone()]);
    assert_eq!(r.floor().unwrap().tiles[seal.cell], "wall");
}
