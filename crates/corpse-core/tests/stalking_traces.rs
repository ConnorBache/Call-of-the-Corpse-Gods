use corpse_core::{
    Command, Run, storage,
    temporal::{Engine, History},
};
use serde_json::Value;
fn normalize(v: &mut Value) {
    if let Some(o) = v.as_object_mut() {
        o.shift_remove("history");
        if let Some(Value::String(raw)) = o.get("future") {
            let r = storage::deserialize(raw).unwrap();
            let mut value = serde_json::to_value(r).unwrap();
            normalize(&mut value);
            o.insert("future".into(), value);
        }
        if let Some(Value::String(raw)) = o.get("hasteSnapshot") {
            let mut value: Value = serde_json::from_str(raw).unwrap();
            normalize(&mut value);
            o.insert("hasteSnapshot".into(), value);
        }
    }
}
fn diff(a: &Value, b: &Value, path: &str) -> String {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                let Some(w) = b.get(k) else {
                    return format!("{path}.{k} unexpected");
                };
                if !storage::canonical_equal(v, w) {
                    return diff(v, w, &format!("{path}.{k}"));
                }
            }
            for k in b.keys() {
                if !a.contains_key(k) {
                    return format!("{path}.{k} missing");
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return format!("{path} lengths {} != {}", a.len(), b.len());
            }
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                if !storage::canonical_equal(a, b) {
                    return diff(a, b, &format!("{path}[{i}]"));
                }
            }
        }
        _ => return format!("{path}: {a} != {b}"),
    }
    path.into()
}
fn compare(a: &mut Value, b: &mut Value, label: &str) {
    normalize(a);
    normalize(b);
    if !storage::canonical_equal(a, b) {
        std::fs::write(
            "/tmp/stalking-trace-native.json",
            serde_json::to_vec(a).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "/tmp/stalking-trace-source.json",
            serde_json::to_vec(b).unwrap(),
        )
        .unwrap();
    }
    assert!(
        storage::canonical_equal(a, b),
        "{label}: {}",
        diff(a, b, "$")
    );
}
#[test]
fn stalking_command_reference_traces() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/stalking-traces-v48.json");
    let cases: Value = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    for case in cases.as_array().unwrap() {
        if let Ok(filter) = std::env::var("STALKING_CASE") {
            if !case["name"].as_str().unwrap().contains(&filter) {
                continue;
            }
        }
        let r: Run = serde_json::from_value(case["initial"].clone()).unwrap();
        let mut e = Engine::new(r).unwrap();
        // Frozen v48 trace predates snapshot-overwrite policy and replay provenance.
        e.set_legacy_revert(true);
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let c: Command = serde_json::from_value(step["command"].clone()).unwrap();
            let label = format!("{} step{i} {c:?}", case["name"]);
            let outcome = e
                .dispatch(&c)
                .unwrap_or_else(|err| panic!("{label}: {err}"));
            if !storage::canonical_equal(
                &serde_json::to_value(&outcome.events).unwrap(),
                &step["events"],
            ) {
                std::fs::write(
                    "/tmp/stalking-event-native.json",
                    serde_json::to_vec(&e.canonical_run().unwrap()).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/stalking-event-source.json",
                    serde_json::to_vec(&step["state"]).unwrap(),
                )
                .unwrap();
            }
            assert!(
                storage::canonical_equal(
                    &serde_json::to_value(&outcome.events).unwrap(),
                    &step["events"]
                ),
                "{label} events {:?} != {}",
                outcome.events,
                step["events"]
            );
            if step["resume"] == true {
                e = Engine::load(&e.save().unwrap()).unwrap();
                e.set_legacy_revert(true);
            }
            let r = e.canonical_run().unwrap();
            compare(
                &mut serde_json::to_value(&r).unwrap(),
                &mut step["state"].clone(),
                &label,
            );
            let expected: Run = serde_json::from_value(step["state"].clone()).unwrap();
            let ah = History::import(&r).unwrap();
            let bh = History::import(&expected).unwrap();
            assert_eq!(ah.frames.len(), bh.frames.len(), "{label} history count");
            for (n, (a, b)) in ah.frames.iter().zip(&bh.frames).enumerate() {
                compare(
                    &mut a.value().unwrap(),
                    &mut b.value().unwrap(),
                    &format!("{label} history{n}"),
                );
            }
            assert_eq!(
                ah.pending_haste.is_some(),
                bh.pending_haste.is_some(),
                "{label} pending presence"
            );
            if let (Some(a), Some(b)) = (&ah.pending_haste, &bh.pending_haste) {
                compare(
                    &mut a.value().unwrap(),
                    &mut b.value().unwrap(),
                    &format!("{label} pending"),
                );
            }
        }
    }
}
