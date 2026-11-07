//! Admission regression only; item/Fury behavior has separate deterministic tests.
use corpse_core::{population, replay::{ReplayRecord, ReplaySession}, startup, temporal::Engine, Command};

#[test]
fn revision44_inherits_fixed_birth_profiles_without_rewriting_old_actors() {
    for row in corpse_core::balance42::profiles() {
        if row.kind.starts_with("zillennia:") { continue; }
        let Ok(old) = population::create_actor_revision(&row.kind, 7., 2., 3., false, 42.) else { continue; };
        let new = population::create_actor_revision(&row.kind, 7., 2., 3., false, 44.).unwrap();
        assert_eq!(serde_json::to_value(old).unwrap(), serde_json::to_value(new).unwrap(), "{}", row.kind);
    }
    for revision in [41., 42., 43., 44.] {
        let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = revision.into();
        run.history.clear(); run.haste_snapshot = corpse_core::Field::Missing;
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
fn revision44_seed_replay_restart_and_admission_are_explicit() {
    let initial = startup::create_run_revision("0-5-11:revision44-protocol", "Unblessed", false, 44.).unwrap();
    let mut engine = Engine::new(initial).unwrap();
    for command in [Command::Pickup, Command::Pickup, Command::Wait] { engine.dispatch(&command).unwrap(); }
    let expected = engine.save().unwrap();
    let mut replay = ReplaySession::create(ReplayRecord::try_from_run(&engine.run).unwrap(), true).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    replay.restart().unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    for revision in [47., 43.5, 44.5, f64::NAN] {
        assert!(startup::new_run_shell_revision("unsupported", "Unblessed", false, revision).is_err());
    }
    // Historical core convenience entry point remains unchanged; desktop opts in.
    assert_eq!(startup::new_run_shell("legacy-default", "Unblessed", false).unwrap().simulation_revision.value_or(0.), 39.);
}

#[test]
fn revision44_inherits_early_ruin_pulses() {
    let mut actor = corpse_core::Actor::default();
    for (quality, old, current) in [("infant", 28., 14.), ("child", 36., 18.), ("lesser", 44., 44.)] {
        actor.zillennia_quality = quality.to_string().into();
        assert_eq!(corpse_core::monster_ruin::ruin_damage_revision(&actor, 41.), old);
        for revision in [42., 43., 44.] { assert_eq!(corpse_core::monster_ruin::ruin_damage_revision(&actor, revision), current); }
    }
}
