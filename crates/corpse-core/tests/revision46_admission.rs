//! Revision46 admission and inherited behavior; Shine suppression has separate tests.
use corpse_core::{population, replay::{ReplayRecord, ReplaySession}, startup, temporal::Engine, Command, Field};

fn fixture(revision: f64) -> corpse_core::Run {
    let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = revision.into();
    run.history.clear(); run.haste_snapshot = Field::Missing;
    run
}

#[test]
fn revision46_inherits_every_fixed_profile_without_rewriting_saved_actors() {
    for row in corpse_core::balance42::profiles() {
        // Include child-context, fish, boss and Zillennia rows, not only ordinary actors.
        let mut old = corpse_core::Actor::default();
        let mut new = old.clone();
        corpse_core::balance42::apply_new_actor_kind(&mut old, &row.kind, 45.);
        corpse_core::balance42::apply_new_actor_kind(&mut new, &row.kind, 46.);
        assert_eq!(old, new, "{} profile", row.kind);
        if let Ok(old) = population::create_actor_revision(&row.kind, 7., 2., 3., false, 45.) {
            let new = population::create_actor_revision(&row.kind, 7., 2., 3., false, 46.).unwrap();
            assert_eq!(old, new, "{} constructor", row.kind);
        }
    }
    for revision in [41., 42., 43., 44., 45., 46.] {
        let mut run = fixture(revision);
        run.player.stats.offense = 123.25;
        let engine = Engine::new(run).unwrap();
        let saved = engine.save().unwrap();
        let loaded = Engine::load(&saved).unwrap();
        assert_eq!(loaded.run.simulation_revision.value_or(0.), revision);
        assert_eq!(loaded.run.player.stats.offense, 123.25);
        assert_eq!(loaded.save().unwrap(), saved);
    }
}

#[test]
fn revision46_seed_generation_replay_and_restart_preserve_revision() {
    let seed = "0-5-11:revision46-protocol";
    let old = startup::create_run_revision(seed, "Unblessed", false, 45.).unwrap();
    let initial = startup::create_run_revision(seed, "Unblessed", false, 46.).unwrap();
    let mut comparable = initial.clone(); comparable.simulation_revision = 45.0.into();
    assert_eq!(serde_json::to_value(&old).unwrap(), serde_json::to_value(&comparable).unwrap());
    let mut engine = Engine::new(initial).unwrap();
    for command in [Command::Pickup, Command::Pickup, Command::Wait] { engine.dispatch(&command).unwrap(); }
    let expected = engine.save().unwrap();
    let mut replay = ReplaySession::create(ReplayRecord::try_from_run(&engine.run).unwrap(), true).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    replay.restart().unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    assert_eq!(replay.engine.run.simulation_revision.value_or(0.), 46.);
    // Historical core convenience entry points retain their prior defaults.
    assert_eq!(startup::new_run_shell("legacy-default", "Unblessed", false).unwrap().simulation_revision.value_or(0.), 39.);
}

#[test]
fn revision46_debug_and_playtest_starts_roundtrip_without_upgrade() {
    let debug = corpse_core::debug_floor::create_run_revision("0-5-11:debug-r46", "Unblessed", 1, 46.).unwrap();
    let playtest = corpse_core::playtest::create_run_revision("0-5-11:playtest-r46", "Unblessed", false, 1,
        corpse_core::playtest::PROFILE, 46.).unwrap();
    for run in [debug, playtest] {
        assert_eq!(run.simulation_revision.value_or(0.), 46.);
        corpse_core::playtest::validate_profile(&run).unwrap();
        let engine = Engine::new(run).unwrap();
        let saved = engine.save().unwrap();
        assert_eq!(Engine::load(&saved).unwrap().save().unwrap(), saved);
    }
}

#[test]
fn revision46_admission_rejects_future_fractional_and_nonfinite_revisions() {
    for revision in [47., 45.5, 46.5, f64::NAN, f64::INFINITY] {
        assert!(startup::new_run_shell_revision("unsupported", "Unblessed", false, revision).is_err());
        assert!(population::create_actor_revision("rat", 7., 2., 3., false, revision).is_err());
        assert!(corpse_core::generation::generate_floor_full_revision(1, 1, None, revision).is_err());
        assert!(corpse_core::debug_floor::create_run_revision("unsupported", "Unblessed", 1, revision).is_err());
        assert!(corpse_core::playtest::create_run_revision("unsupported", "Unblessed", false, 1,
            corpse_core::playtest::PROFILE, revision).is_err());
        let run = fixture(revision);
        assert!(corpse_core::storage::validate_current_revision(&run).is_err());
        let record = ReplayRecord::from_run(&run);
        assert!(ReplaySession::create(record.clone(), true).is_err());
        assert!(ReplaySession::from_initial(run, record).is_err());
        let mut floor = corpse_core::Floor::default(); let before = floor.clone();
        let mut calls = 0;
        assert!(corpse_core::generation::generate_setups_revision(&mut floor, &mut || { calls += 1; 0.5 }, revision).is_err());
        assert_eq!(calls, 0); assert_eq!(floor, before);
    }
}

#[test]
fn revision46_inherits_early_ruin_pulses() {
    let mut actor = corpse_core::Actor::default();
    for (quality, old, current) in [("infant", 28., 14.), ("child", 36., 18.), ("lesser", 44., 44.)] {
        actor.zillennia_quality = quality.to_string().into();
        assert_eq!(corpse_core::monster_ruin::ruin_damage_revision(&actor, 41.), old);
        for revision in [42., 43., 44., 45., 46.] {
            assert_eq!(corpse_core::monster_ruin::ruin_damage_revision(&actor, revision), current);
        }
    }
}

fn summon_room(kind: &str, revision: f64, seed: u32) -> corpse_core::Run {
    use std::sync::Arc;
    let mut run = startup::new_run_shell_revision("0-5-11:inherited-summon", "Unblessed", false, revision).unwrap();
    run.player.x = 2.; run.player.y = 2.; run.player.abilities.clear(); run.player.active.clear();
    run.player.conditions.clear(); run.player.ability_order = Field::Missing;
    let (width, height) = (79, 29);
    let mut tiles = vec!["wall".into(); width * height];
    for y in 1..height - 1 { for x in 1..width - 1 { tiles[y * width + x] = "floor".into(); } }
    let mut floor = corpse_core::Floor { width: (width as f64).into(), height: (height as f64).into(), depth: 1.,
        domain: "Dungeon".into(), tiles, lights: vec![100.; width * height], seen: vec![true; width * height], ..Default::default() };
    corpse_core::environment::initialize(&mut floor).unwrap(); run.floors = vec![Arc::new(floor)];
    let mut actor = population::create_actor_revision(kind, 4., 5., 5., false, revision).unwrap(); actor.state = "hunting".into();
    run.actors.clear(); run.actors.insert("1".into(), Arc::new(vec![actor]));
    run.ground.clear(); run.ground.insert("1".into(), Arc::new(vec![]));
    run.next_id = 5.; run.tick = 3.; run.rng = seed as f64; run.generated_ability_grants = Vec::new().into();
    run
}

#[test]
fn revision46_witch_and_queen_children_inherit_exact_birth_stats_and_rng() {
    let mut seen = std::collections::BTreeSet::new();
    for (kind, context) in [("witch", "witch"), ("saidribee-queen", "queen")] {
        for seed in 1..=64 {
            let mut old = summon_room(kind, 45., seed); let mut new = summon_room(kind, 46., seed);
            let target = old.player.id;
            assert!(corpse_core::ai::enemy_action(&mut old, 4., target).unwrap());
            assert!(corpse_core::ai::enemy_action(&mut new, 4., target).unwrap());
            assert_eq!(old.rng, new.rng); assert_eq!(old.next_id, new.next_id);
            assert_eq!(old.actors_at(1.), new.actors_at(1.));
            for actor in new.actors_at(1.).iter().skip(1) {
                seen.insert(format!("{context}:{}", actor.creature_kind.as_ref().unwrap()));
            }
        }
    }
    assert_eq!(seen.len(), 4, "Both Witch and Queen newborn kinds must be exercised: {seen:?}");
}
