use corpse_core::{Run, storage, temporal::History};
use serde_json::Value;
fn compare(a: &Run, b: &Run, label: &str) {
    let mut a_value = storage::canonical_value(a).unwrap();
    let mut b_value = storage::canonical_value(b).unwrap();
    for key in ["history", "future", "hasteSnapshot"] {
        a_value.as_object_mut().unwrap().shift_remove(key);
        b_value.as_object_mut().unwrap().shift_remove(key);
    }
    if !storage::canonical_equal(&a_value, &b_value) {
        std::fs::write("/tmp/load-actual.json", a_value.to_string()).unwrap();
        std::fs::write("/tmp/load-expected.json", b_value.to_string()).unwrap();
        panic!("{label}: world differs; /tmp/load-{{actual,expected}}.json");
    }
    let ah = History::import(a).unwrap();
    let bh = History::import(b).unwrap();
    assert_eq!(ah.frames.len(), bh.frames.len(), "{label}: frame count");
    for (i, (a, b)) in ah.frames.iter().zip(&bh.frames).enumerate() {
        compare(&a.state, &b.state, &format!("{label}/frame{i}"));
        assert_eq!(a.cursor, b.cursor, "{label}/frame{i} cursor");
    }
    match (ah.pending_haste, bh.pending_haste) {
        (Some(a), Some(b)) => {
            compare(&a.state, &b.state, &format!("{label}/haste"));
            assert_eq!(a.cursor, b.cursor);
        }
        (None, None) => {}
        _ => panic!("{label}: pending presence"),
    };
    match (&a.future, &b.future) {
        (Some(a), Some(b)) => compare(
            &storage::deserialize(a.wire_text().unwrap()).unwrap(),
            &storage::deserialize(b.wire_text().unwrap()).unwrap(),
            &format!("{label}/future"),
        ),
        (None, None) => {}
        _ => panic!("{label}: future presence"),
    }
}
#[test]
fn source_ordered_load_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/load-migrations-v48.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let actual = storage::deserialize_compatible(case["raw"].as_str().unwrap())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Run = serde_json::from_value(case["expected"].clone()).unwrap();
        compare(&actual, &expected, name);
        let round = storage::deserialize_compatible(&storage::serialize(&actual).unwrap()).unwrap();
        compare(&round, &expected, &format!("{name}/resume"));
    }
}
#[test]
fn malformed_nested_state_is_rejected_and_old_simulation_stays_gated() {
    let base: Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut v = base.clone();
    v["future"] = Value::String("{\"version\":1}".into());
    assert!(storage::deserialize_compatible(&v.to_string()).is_err());
    let mut v = base.clone();
    let frame = serde_json::json!({"groups":[[{"path":["player","x"],"value":-1}]],"future":null});
    v["history"] = serde_json::json!([frame.to_string()]);
    assert!(storage::deserialize_compatible(&v.to_string()).is_err());
    let mut v = base.clone();
    v["hasteSnapshot"] =
        Value::String("{\"logArchiveCursor\":{\"messages\":999999,\"events\":999999}}".into());
    assert!(storage::deserialize_compatible(&v.to_string()).is_err());
    let mut v = base.clone();
    for k in [
        "gameplayRevision",
        "divineRevision",
        "enemyHealthRevision",
        "simulationRevision",
    ] {
        v.as_object_mut().unwrap().shift_remove(k);
    }
    let loaded = storage::deserialize_compatible(&v.to_string()).unwrap();
    assert!(loaded.simulation_revision.is_missing());
    assert!(corpse_core::temporal::Engine::new(loaded).is_err());
}
#[test]
fn every_divine_equipment_loss_load_matches_source() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/load-grants-v48.json")).unwrap();
    let expand = |patch: &Value| {
        let mut base = fixture["base"].clone();
        for key in patch["remove"].as_array().unwrap() {
            base.as_object_mut()
                .unwrap()
                .shift_remove(key.as_str().unwrap());
        }
        for (k, v) in patch["set"].as_object().unwrap() {
            base[k] = v.clone();
        }
        base
    };
    for c in fixture["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let raw = expand(&c["before"]).to_string();
        let actual =
            storage::deserialize_compatible(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Run = serde_json::from_value(expand(&c["after"])).unwrap();
        compare(&actual, &expected, name);
    }
}
#[test]
fn empty_haste_marker_survives_load_and_free_command() {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    value["hasteSnapshot"] = Value::String(String::new());
    let mut engine = corpse_core::temporal::Engine::load(&value.to_string()).unwrap();
    assert_eq!(
        engine.run.haste_snapshot.as_ref().map(String::as_str),
        Some("")
    );
    engine.dispatch(&corpse_core::Command::Pickup).unwrap();
    assert_eq!(engine.run.tick, 0.0);
    assert_eq!(
        engine.run.haste_snapshot.as_ref().map(String::as_str),
        Some("")
    );
}
