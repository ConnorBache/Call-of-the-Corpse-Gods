use corpse_core::{
    storage,
    temporal::{History, Snapshot},
    *,
};
#[test]
fn avatar_schema_roundtrip() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let state = v.get("run").or(v.get("state")).unwrap_or(&v);
    let run: Run = serde_json::from_value(state.clone()).unwrap();
    let out = storage::canonical_value(&run).unwrap();
    assert!(
        storage::canonical_equal(&out, state),
        "canonical field mismatch"
    );
    storage::validate(&run).unwrap();
}
#[test]
fn optional_null_absence() {
    let a: Stats = serde_json::from_str(
        r#"{"offense":0,"defense":0,"accuracy":0,"dodge":0,"dr":0,"perception":0,"maxEnergy":0}"#,
    )
    .unwrap();
    assert!(a.max_hp.is_missing());
    let mut b = a.clone();
    b.max_hp = Field::Null;
    assert_ne!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}
#[test]
fn snapshot_shares_floor_blocks() {
    let mut r = Run::default();
    r.floors.push(std::sync::Arc::new(Floor::default()));
    let s = Snapshot::capture(&r);
    assert!(std::sync::Arc::ptr_eq(&r.floors[0], &s.state.floors[0]));
    std::sync::Arc::make_mut(&mut r.floors[0]).domain = "Changed".into();
    assert_ne!(r.floors[0].domain, s.state.floors[0].domain);
}
#[test]
fn history_retains_five() {
    let r = Run::default();
    let mut h = History::default();
    for _ in 0..9 {
        h.record_completed(Snapshot::capture(&r));
    }
    assert_eq!(h.frames.len(), 5);
}
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/foundation.json")).unwrap()
}
fn assert_state(a: &Run, b: &serde_json::Value) {
    let mut actual = storage::canonical_value(a).unwrap();
    let mut expected = b.clone();
    actual.as_object_mut().unwrap().shift_remove("history");
    expected.as_object_mut().unwrap().shift_remove("history");
    assert!(
        storage::canonical_equal(&actual, &expected),
        "state differs at {}",
        first_difference(&actual, &expected, "$")
    );
}
fn first_difference(a: &serde_json::Value, b: &serde_json::Value, p: &str) -> String {
    use serde_json::Value::*;
    match (a, b) {
        (Object(a), Object(b)) => {
            for (k, v) in a {
                if let Some(w) = b.get(k) {
                    if !storage::canonical_equal(v, w) {
                        return first_difference(v, w, &format!("{p}.{k}"));
                    }
                } else {
                    return format!("{p}.{k}: unexpected");
                }
            }
            for k in b.keys() {
                if !a.contains_key(k) {
                    return format!("{p}.{k}: missing");
                }
            }
        }
        (Array(a), Array(b)) => {
            if a.len() != b.len() {
                return format!("{p}: length {} vs {}", a.len(), b.len());
            }
            for (i, (v, w)) in a.iter().zip(b).enumerate() {
                if !storage::canonical_equal(v, w) {
                    return first_difference(v, w, &format!("{p}[{i}]"));
                }
            }
        }
        _ => return format!("{p}: {a} vs {b}"),
    }
    p.into()
}
#[test]
fn rng_reference_vectors() {
    for vector in fixture()["vectors"].as_array().unwrap() {
        let seed = vector["seed"].as_str().unwrap();
        let mut state = rng::seed_hash(seed);
        assert_eq!(state, vector["hash"].as_u64().unwrap() as u32);
        for expected in vector["draws"].as_array().unwrap() {
            assert_eq!(rng::next_u32(&mut state), expected.as_u64().unwrap() as u32);
        }
    }
}
#[test]
fn legacy_history_import_revert_parity() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["beforeRevert"].clone()).unwrap();
    let mut h = History::import(&run).unwrap();
    for expected in f["reverted"].as_array().unwrap() {
        assert!(temporal::revert(&mut run, &mut h).unwrap());
        assert_state(&run, expected);
    }
    assert!(!temporal::revert(&mut run, &mut h).unwrap());
}
#[test]
fn native_ring_revert_parity() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    let mut h = History::default();
    for next in f["turns"].as_array().unwrap() {
        let before = Snapshot::capture(&run);
        run = serde_json::from_value(next.clone()).unwrap();
        h.record_completed(before);
    }
    run = serde_json::from_value(f["beforeRevert"].clone()).unwrap();
    for expected in f["reverted"].as_array().unwrap() {
        assert!(temporal::revert(&mut run, &mut h).unwrap());
        assert_state(&run, expected);
    }
}
#[test]
fn native_inverse_export_continuation() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    let mut h = History::default();
    for next in f["turns"].as_array().unwrap() {
        let before = Snapshot::capture(&run);
        run = serde_json::from_value(next.clone()).unwrap();
        h.record_completed(before);
    }
    let exported = h.export(&run).unwrap();
    let mut imported = History::import(&exported).unwrap();
    for expected in f["reverted"].as_array().unwrap() {
        assert!(temporal::revert(&mut run, &mut imported).unwrap());
        assert_state(&run, expected);
    }
}
#[test]
fn future_restore_costs_parity() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["future"]["branch"].clone()).unwrap();
    let mut h = History::import(&run).unwrap();
    assert!(temporal::restore_future(&mut run, &mut h).unwrap());
    assert_state(&run, &f["future"]["restored"]);
}
#[test]
fn packed_save_roundtrip() {
    let f = fixture();
    let run = storage::deserialize(f["packed"].as_str().unwrap()).unwrap();
    assert_state(&run, &f["initial"]);
}
#[test]
fn engine_history_full_trace() {
    let trace: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1-trace.json")).unwrap();
    let initial = serde_json::from_value(trace["initial"].clone()).unwrap();
    let mut engine = temporal::Engine::new(initial).unwrap();
    // The revision29 avatar-room trace predates snapshot-overwrite replay policy.
    engine.set_legacy_revert(true);
    for step in trace["steps"].as_array().unwrap() {
        let command = serde_json::from_value(step["command"].clone()).unwrap();
        engine.dispatch(&command).unwrap();
        let out = engine.canonical_run().unwrap();
        assert_state(&out, &step["state"]);
        let expected: Run = serde_json::from_value(step["state"].clone()).unwrap();
        let expected_history = History::import(&expected).unwrap();
        let actual_history = History::import(&out).unwrap();
        assert_eq!(expected_history.frames.len(), actual_history.frames.len());
        for (a, b) in actual_history
            .frames
            .iter()
            .zip(expected_history.frames.iter())
        {
            assert!(
                storage::canonical_equal(&a.value().unwrap(), &b.value().unwrap()),
                "history baseline differs at {}",
                first_difference(&a.value().unwrap(), &b.value().unwrap(), "$")
            );
        }
    }
}
#[test]
fn save_after_revert_continues_exactly() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["beforeRevert"].clone()).unwrap();
    let mut h = History::import(&run).unwrap();
    for expected in f["reverted"].as_array().unwrap() {
        assert!(temporal::revert(&mut run, &mut h).unwrap());
        assert_state(&run, expected);
        run = h.export(&run).unwrap();
        h = History::import(&run).unwrap();
    }
}
#[test]
fn future_activation_native_anchor_parity() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["future"]["initial"].clone()).unwrap();
    let h = History::import(&run).unwrap();
    temporal::remember_future(&mut run, &h).unwrap();
    let native = storage::deserialize(run.future.as_ref().unwrap().wire_text().unwrap()).unwrap();
    let expected = storage::deserialize(f["future"]["anchor"]["future"].as_str().unwrap()).unwrap();
    assert!(storage::canonical_equal(
        &storage::canonical_value(&native).unwrap(),
        &storage::canonical_value(&expected).unwrap()
    ));
    run.future = None;
    let mut expected_live = f["future"]["anchor"].clone();
    expected_live["future"] = serde_json::Value::Null;
    assert_state(&run, &expected_live);
}
#[test]
fn malformed_geometry_rejects_without_panic() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    std::sync::Arc::make_mut(&mut run.floors[0]).tiles.pop();
    assert!(storage::validate(&run).is_err());
}
#[test]
fn snapshot_failure_does_not_drop_archive() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    let mut s = Snapshot::capture(&run);
    s.cursor.as_mut().unwrap().events += 1000;
    let before = run.clone();
    assert!(s.restore(&mut run).is_err());
    assert_eq!(run, before);
}

#[test]
fn stale_geometry_metadata_is_ignored() {
    let f = fixture();
    let mut run: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    let floor = std::sync::Arc::make_mut(&mut run.floors[0]);
    floor.width = Field::Value(0.0);
    floor.height = Field::Value(999.0);
    assert_eq!(floor.width(), 79);
    assert_eq!(floor.height(), 29);
    assert!(storage::validate(&run).is_err());
}
#[test]
fn enemy_health_migration_reference_vectors() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/enemy-health-migration.json"
    ))
    .unwrap();
    let initial: Run = serde_json::from_value(fixture()["initial"].clone()).unwrap();
    for case in v.as_array().unwrap() {
        let mut r = initial.clone();
        r.enemy_health_revision = Field::Missing;
        r.actors.insert(
            "1".into(),
            std::sync::Arc::new(vec![
                serde_json::from_value(case["before"].clone()).unwrap(),
            ]),
        );
        assert!(migrations::migrate_enemy_health_revision(&mut r));
        let actual = serde_json::to_value(&r.actors_at(1.0)[0]).unwrap();
        assert!(
            storage::canonical_equal(&actual, &case["after"]),
            "{}",
            first_difference(&actual, &case["after"], "actor")
        );
        assert!(!migrations::migrate_enemy_health_revision(&mut r));
    }
}
#[test]
fn migration_history_rebase_reference() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/migration-rebase.json")).unwrap();
    let mut r: Run = serde_json::from_value(v["before"].clone()).unwrap();
    migrations::rebase_history(&mut r, migrations::migrate_enemy_health_revision).unwrap();
    let expected: Run = serde_json::from_value(v["after"].clone()).unwrap();
    let actual_history = History::import(&r).unwrap();
    let expected_history = History::import(&expected).unwrap();
    for (a, b) in actual_history.frames.iter().zip(&expected_history.frames) {
        assert!(
            storage::canonical_equal(&a.value().unwrap(), &b.value().unwrap()),
            "{}",
            first_difference(&a.value().unwrap(), &b.value().unwrap(), "history")
        );
    }
}
#[test]
fn malformed_save_reference_validation() {
    let initial = fixture()["initial"].clone();
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/validation.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut v = initial.clone();
        let mut at = &mut v;
        for k in c["path"].as_array().unwrap() {
            at = if let Some(s) = k.as_str() {
                at.as_object_mut()
                    .unwrap()
                    .entry(s)
                    .or_insert(serde_json::Value::Null)
            } else {
                &mut at.as_array_mut().unwrap()[k.as_u64().unwrap() as usize]
            };
        }
        *at = c["value"].clone();
        let result = storage::deserialize(&serde_json::to_string(&v).unwrap());
        assert_eq!(
            result.is_ok(),
            c["accepted"].as_bool().unwrap(),
            "{}: {:?}",
            c["name"],
            result.err()
        );
    }
}
