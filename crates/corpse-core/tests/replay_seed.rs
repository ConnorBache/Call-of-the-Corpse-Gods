use corpse_core::{
    Field,
    replay::{ReplayRecord, ReplaySession},
    storage,
};
fn record(seed: &str, mode: &str) -> ReplayRecord {
    ReplayRecord {
        playtest: Field::Missing,
        fast_mode: Field::Missing,
        compressed_depths: Field::Missing,
        version: 1.0,
        revert_policy: Field::Missing,
        snapshot_from: Field::Missing,
        firebolt_travel_from: Field::Missing,
        amulet_rules_from: Field::Missing,
        potion_rules_from: Field::Missing,
        ring_rules_from: Field::Missing,
        gas_rules_from: Field::Missing,
        future_floor_rules_from: Field::Missing,
        seed: seed.into(),
        mode: mode.into(),
        dev: false,
        commands: vec![],
        simulation_revision: Field::Value(29.0),
        depth: Field::Missing,
        tick: Field::Missing,
        status: Field::Missing,
        saved_at: Field::Missing,
    }
}
#[test]
fn seeded_replay_initial_state_matches_reference() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    for r in fixture["runs"].as_array().unwrap() {
        let mut session = ReplaySession::create(
            record(r["seed"].as_str().unwrap(), r["mode"].as_str().unwrap()),
            true,
        )
        .unwrap();
        assert!(storage::canonical_equal(
            &storage::canonical_value(&session.engine.run).unwrap(),
            r
        ));
        assert!(session.finished());
        assert!(!session.step().unwrap());
        session.engine.run.player.hp = 1.0;
        session.restart().unwrap();
        assert!(storage::canonical_equal(
            &storage::canonical_value(&session.engine.run).unwrap(),
            r
        ));
    }
}
#[test]
fn replay_revision_rejects_before_generation() {
    let mut r = record("test", "Avatar");
    r.simulation_revision = Field::Missing;
    assert!(ReplaySession::create(r, true).is_err());
    let mut r = record("test", "Avatar");
    r.simulation_revision = Field::Value(28.0);
    assert!(ReplaySession::create(r, false).is_err());
    let mut r = record("test", "Avatar");
    r.mode = "Invalid".into();
    assert!(ReplaySession::create(r, false).is_err());
}
#[test]
fn replay_steps_preserve_events_and_rejection_cursor() {
    let initial: corpse_core::Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut r = record(&initial.seed, &initial.mode);
    r.dev = initial.dev;
    r.commands = vec![
        corpse_core::Command::Wait,
        corpse_core::Command::Dev {
            action: "not-supported".into(),
        },
    ];
    let mut replay = ReplaySession::from_initial(initial.clone(), r).unwrap();
    let mut expected = corpse_core::temporal::Engine::new(initial).unwrap();
    expected.set_legacy_revert(true); // version1 fixture explicitly retains legacy policy.
    let outcome = expected.dispatch(&corpse_core::Command::Wait).unwrap();
    let actual = replay.step_with_outcome().unwrap().unwrap();
    assert_eq!(actual.events, outcome.events);
    assert_eq!(replay.cursor, 1);
    assert!(storage::canonical_equal(
        &storage::canonical_value(&replay.engine.run).unwrap(),
        &storage::canonical_value(&expected.run).unwrap()
    ));
    let before = replay.engine.save().unwrap();
    assert!(replay.step().is_err());
    assert_eq!(replay.cursor, 1);
    assert_eq!(replay.engine.save().unwrap(), before);
    let exported = ReplayRecord::from_run(&replay.engine.run);
    assert_eq!(exported.commands, replay.engine.run.replay);
    replay.restart().unwrap();
    assert_eq!(replay.cursor, 0);
}
