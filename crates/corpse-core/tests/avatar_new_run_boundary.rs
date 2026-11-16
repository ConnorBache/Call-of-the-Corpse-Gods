//! Avatar-only entry points must not change recorded historical modes or seeds.
use corpse_core::{
    Run, debug_floor,
    replay::{ReplayRecord, ReplaySession},
    rng, startup, storage,
    temporal::Engine,
};

fn assert_avatar(run: &Run, seed: &str) {
    assert_eq!(run.mode, "Avatar");
    assert_eq!(run.simulation_revision.as_ref(), Some(&52.0));
    assert!(!run.dev);
    assert_eq!(run.seed, seed);
    assert_eq!(run.gods, [0.0, 5.0, 11.0]);
    assert_eq!(run.player.abilities.len(), 3);
    for (god, ability) in run.gods.iter().zip(&run.player.abilities) {
        assert!(corpse_core::progression::starter_abilities_for_revision(*god as u32, 52.0)
            .unwrap().contains(ability));
    }
}

#[test]
fn new_avatar_start_preserves_the_exact_existing_revision52_run() {
    // Leading zeroes and a colon in the suffix are valid existing seed bytes.
    let seed = "00-05-11:avatar-boundary-r52:exact";
    let run = startup::create_avatar_run(seed).unwrap();
    let expected = startup::create_run_revision(seed, "Avatar", false, 52.0).unwrap();
    assert_avatar(&run, seed);
    assert!(run.playtest.is_missing());
    assert_eq!(run, expected, "The new entry point must not add RNG draws or rewrite seeds");
}

#[test]
fn new_avatar_debug_start_preserves_the_exact_existing_reconstruction() {
    let seed = "0-5-11:avatar-debug-boundary-r52";
    let run = debug_floor::create_avatar_run(seed, 1).unwrap();
    let expected = debug_floor::create_run_revision(seed, "Avatar", 1, 52.0).unwrap();
    assert_avatar(&run, seed);
    let profile = run.playtest.as_ref().unwrap();
    assert_eq!(profile.version, 2);
    assert_eq!(profile.start_floor, 1);
    assert_eq!(profile.profile, debug_floor::PROFILE);
    assert_eq!(run, expected, "DEBUG must retain its existing reward and seed semantics");
}

#[test]
fn current_avatar_boundaries_reject_duplicate_or_unknown_gods() {
    for seed in ["0-0-1:duplicate", "0-1-12:unknown", "11-11-11:duplicate"] {
        for result in [startup::create_avatar_run(seed), debug_floor::create_avatar_run(seed, 1)] {
            assert_eq!(result.unwrap_err(), "Seed prefix must contain three distinct god IDs 0–11.");
        }
    }
}

#[test]
fn historical_shells_keep_mode_specific_gods_and_rng_draw_counts() {
    for revision in [29.0, 39.0, 51.0, 52.0] {
        for (mode, count) in [("Avatar", 3), ("Blessed", 1), ("Unblessed", 0)] {
            let seed = "00-05-11:historical:exact";
            let run = startup::new_run_shell_revision(seed, mode, false, revision).unwrap();
            assert_eq!(run.mode, mode);
            assert_eq!(run.seed, seed);
            assert_eq!(run.gods, [0.0, 5.0, 11.0][..count]);
            assert_eq!(run.player.abilities.len(), count);
            let mut expected_rng = rng::seed_hash(seed);
            for _ in 0..count { rng::next_u32(&mut expected_rng); }
            assert_eq!(run.rng, expected_rng as f64);

            let bare = startup::new_run_shell_revision("historical bare seed", mode, false, revision).unwrap();
            let reparsed = startup::new_run_shell_revision(&bare.seed, mode, false, revision).unwrap();
            assert_eq!(bare, reparsed, "Canonical seed recreation must preserve {mode} R{revision}");
        }
    }
}

#[test]
fn original_non_avatar_saves_remain_loadable_without_mode_conversion() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    let mut checked = 0;
    for value in fixture["runs"].as_array().unwrap().iter()
        .filter(|run| run["mode"] != "Avatar")
    {
        let raw = value.to_string();
        let original = storage::from_canonical(&raw).unwrap();
        let loaded = Engine::load(&raw).unwrap();
        assert_eq!(loaded.run.mode, original.mode);
        assert_eq!(loaded.run.gods, original.gods);
        assert_eq!(loaded.run.seed, original.seed);
        assert_eq!(loaded.run.rng, original.rng);
        assert_eq!(loaded.run.simulation_revision, original.simulation_revision);
        checked += 1;
    }
    assert_eq!(checked, 2, "Both historical non-Avatar modes must be covered");
}

#[test]
fn revision52_non_avatar_seed_replays_restart_and_saves_keep_recorded_modes() {
    for (mode, count) in [("Blessed", 1), ("Unblessed", 0)] {
        let shell = startup::new_run_shell_revision("0-5-11:avatar-legacy-r52", mode, false, 52.0).unwrap();
        let mut replay = ReplaySession::create(ReplayRecord::from_run(&shell), true).unwrap();
        let initial = storage::canonical_value(&replay.engine.run).unwrap();
        assert_eq!(replay.engine.run.mode, mode);
        assert_eq!(replay.engine.run.gods.len(), count);
        assert!(replay.finished());

        let saved = replay.engine.save().unwrap();
        let loaded = Engine::load(&saved).unwrap();
        assert_eq!(loaded.run.mode, mode);
        assert_eq!(loaded.run.gods, replay.engine.run.gods);
        assert_eq!(loaded.run.seed, replay.engine.run.seed);
        assert_eq!(loaded.run.rng, replay.engine.run.rng);
        assert_eq!(loaded.run.simulation_revision.as_ref(), Some(&52.0));

        replay.engine.run.player.hp = 1.0;
        replay.restart().unwrap();
        assert_eq!(replay.cursor, 0);
        assert_eq!(storage::canonical_value(&replay.engine.run).unwrap(), initial);
    }
}
