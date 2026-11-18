//! Policy boundaries must not reinterpret historical See the Future commands.
use corpse_core::{Field, Command, replay::{ReplayRecord, ReplaySession}, temporal::Engine};
fn equal_save(a: &Engine, b: &Engine) {
    let a: serde_json::Value = serde_json::from_str(&a.save().unwrap()).unwrap();
    let b: serde_json::Value = serde_json::from_str(&b.save().unwrap()).unwrap();
    let diff: Vec<_> = a.as_object().unwrap().iter().filter(|(k,v)| b.get(*k) != Some(*v)).map(|(k,_)| k.as_str()).collect();
    assert!(diff.is_empty(), "Different save fields: {diff:?}");
}
fn record() -> ReplayRecord {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/gas-prechange-r52.json")).unwrap();
    serde_json::from_value(fixture["record"].clone()).unwrap()
}
#[test]
fn absent_policy_preserves_legacy_replay_and_restart() {
    let mut replay = ReplaySession::create(record(), true).unwrap();
    assert_eq!(replay.engine.run.future_floor_rules_from, Field::Null);
    while replay.step().unwrap() {}
    let saved = replay.engine.save().unwrap();
    replay.restart().unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), saved);
}
#[test]
fn invalid_policy_boundaries_are_rejected() {
    for policy in [Field::Null, Field::Value(usize::MAX)] {
        let mut r = record(); r.future_floor_rules_from = policy;
        assert!(ReplaySession::create(r, true).is_err());
    }
    let mut r = record(); r.future_floor_rules_from = 0usize.into(); r.simulation_revision = 51.0.into();
    assert!(ReplaySession::create(r, true).is_err());
}
#[test]
fn live_adoption_exports_exact_boundary_and_replays_it_at_eof() {
    let mut replay = ReplaySession::create(record(), true).unwrap();
    while replay.step().unwrap() {}
    let mut live = Engine::load(&replay.engine.save().unwrap()).unwrap();
    assert_eq!(live.run.future_floor_rules_from, Field::Value(live.run.replay.len()));
    let exported = ReplayRecord::try_from_run(&live.run).unwrap();
    assert_eq!(exported.future_floor_rules_from, live.run.future_floor_rules_from);
    let mut mixed = ReplaySession::create(exported, true).unwrap();
    while mixed.step().unwrap() {}
    equal_save(&mixed.engine, &live);
    live.dispatch(&Command::Wait).unwrap();
    let mut mixed = ReplaySession::create(ReplayRecord::try_from_run(&live.run).unwrap(), true).unwrap();
    while mixed.step().unwrap() {}
    equal_save(&mixed.engine, &live);
}
