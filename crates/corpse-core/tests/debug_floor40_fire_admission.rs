//! Regression coverage for real DEBUG floor-40 entry with generated Hellfire.
//! All reconstruction choices are test policy, not changes to player loadouts.
use corpse_core::{Command, debug_floor, storage, temporal::Engine};

fn verify_floor40_hellfire_entry(revision: f64) {
    for mode in ["Unblessed", "Avatar"] {
        let run = debug_floor::create_run_revision("floor40-fire-review-2", mode, 40, revision).unwrap();
        let mut original = Engine::new(run).unwrap();
        while let Some(id) = original.run.offers.first().cloned() {
            original.dispatch(&Command::Choose { id }).unwrap();
        }
        assert!(debug_floor::choices_complete(&original.run));
        assert!(original.run.floor().unwrap().hellfire.as_ref().unwrap().iter().any(|v| *v));
        // Prior-floor sources were collected by DEBUG reconstruction. Their
        // root cleanup and dirty-floor scheduling must not depend on a reload.
        let mut prior_links = 0;
        for floor in original.run.floors.iter().filter(|f| f.depth < 40.) {
            for link in floor.fungal_roots.as_ref().into_iter().flatten() {
                prior_links += 1;
                assert!(!link.active);
            }
        }
        assert!(prior_links > 0, "fixed seed must exercise removed fungus sources");
        let mut resumed = Engine::load(&original.save().unwrap()).unwrap();
        assert!(storage::canonical_equal(
            &serde_json::to_value(original.canonical_run().unwrap()).unwrap(),
            &serde_json::to_value(resumed.canonical_run().unwrap()).unwrap(),
        ), "{mode}, reconstructed checkpoint");
        for engine in [&mut original, &mut resumed] {
            debug_floor::enter(&mut engine.run).unwrap();
            assert!(debug_floor::state(&engine.run).unwrap().entered);
            assert_eq!(engine.run.depth, 40.);
        }
        for turn in 0..4 {
            let left = original.dispatch(&Command::Wait).unwrap_or_else(|e| panic!("{mode}, turn {turn}: {e}"));
            let right = resumed.dispatch(&Command::Wait).unwrap();
            assert!(left.took_turn, "{mode}, turn {turn}");
            assert_eq!(left.took_turn, right.took_turn);
            assert!(storage::canonical_equal(
                &serde_json::to_value(original.canonical_run().unwrap()).unwrap(),
                &serde_json::to_value(resumed.canonical_run().unwrap()).unwrap(),
            ), "{mode}, turn {turn}");
            // Check the changed burning/actor state, not only the initial checkpoint.
            resumed = Engine::load(&resumed.save().unwrap()).unwrap();
        }
    }
}

#[test]
fn debug_floor40_hellfire_entry_turns_and_save_resume_remain_deterministic() {
    verify_floor40_hellfire_entry(51.);
}

#[test]
fn revision52_debug_floor40_hellfire_entry_and_tiered_loot_round_trip() {
    verify_floor40_hellfire_entry(52.);
}

#[test]
fn legacy_debug_checkpoint_still_round_trips() {
    let run = debug_floor::create_run_revision("floor40-fire-review-2", "Unblessed", 40, 50.).unwrap();
    let original = Engine::new(run).unwrap();
    let resumed = Engine::load(&original.save().unwrap()).unwrap();
    assert!(storage::canonical_equal(
        &serde_json::to_value(original.canonical_run().unwrap()).unwrap(),
        &serde_json::to_value(resumed.canonical_run().unwrap()).unwrap(),
    ));
}
