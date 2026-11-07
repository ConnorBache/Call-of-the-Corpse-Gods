use corpse_core::{Command, Run};
#[test]
fn remember_exact_trace() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/remember-v48.json");
    let cases: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    for case in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(case["initial"].clone()).unwrap();
        let mut expected = case["initial"].clone();
        expected.as_object_mut().unwrap().remove("history");
        expected.as_object_mut().unwrap().remove("hasteSnapshot");
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let c: Command = serde_json::from_value(step["command"].clone()).unwrap();
            let outcome = corpse_core::engine::dispatch(&mut r, &c)
                .unwrap_or_else(|e| panic!("{} step{i}: {e}", case["name"]));
            assert!(
                corpse_core::storage::canonical_equal(
                    &serde_json::to_value(&outcome.events).unwrap(),
                    &step["events"]
                ),
                "{} step{i} events: actual {:?}, expected {}",
                case["name"],
                outcome.events,
                step["events"]
            );
            for patch in step["patch"].as_array().unwrap() {
                apply_patch(&mut expected, patch);
            }
            let mut parsed = expected.clone();
            parsed["history"] = serde_json::json!([]);
            let expected_run: Run = serde_json::from_value(parsed).unwrap();
            let mut a = serde_json::to_value(&r).unwrap();
            let mut b = serde_json::to_value(expected_run).unwrap();
            for key in ["history", "hasteSnapshot"] {
                a.as_object_mut().unwrap().remove(key);
                b.as_object_mut().unwrap().remove(key);
            }
            if a != b {
                let keys: Vec<_> = a
                    .as_object()
                    .unwrap()
                    .keys()
                    .chain(b.as_object().unwrap().keys())
                    .filter(|k| a.get(*k) != b.get(*k))
                    .collect();
                panic!(
                    "{} step{i} differing {keys:?}\n{}",
                    case["name"],
                    keys.iter()
                        .filter(|k| k.as_str() != "floors")
                        .map(|k| format!("{k}: actual {} expected {}", a[*k], b[*k]))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    }
}

fn apply_patch(root: &mut serde_json::Value, p: &serde_json::Value) {
    let path = p["path"].as_array().unwrap();
    let mut node = root;
    for key in &path[..path.len() - 1] {
        let k = key.as_str().unwrap();
        node = if node.is_array() {
            &mut node[k.parse::<usize>().unwrap()]
        } else {
            &mut node[k]
        };
    }
    let key = path.last().unwrap().as_str().unwrap();
    if p["remove"] == true {
        node.as_object_mut().unwrap().remove(key);
    } else if node.is_array() {
        node[key.parse::<usize>().unwrap()] = p["value"].clone();
    } else {
        node[key] = p["value"].clone();
    }
}

#[test]
fn unknown_remember_target_trait_is_atomic() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/remember-v48.json");
    let cases: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    let mut r: Run = serde_json::from_value(cases[0]["initial"].clone()).unwrap();
    r.player.energy = 100.0;
    std::sync::Arc::make_mut(r.actors.get_mut("1").unwrap())[0].abilities =
        vec!["enemy:beast:unknown-native-trait".into()].into();
    let before = serde_json::to_value(&r).unwrap();
    let c: Command = serde_json::from_value(
        serde_json::json!({"type":"ability","id":"numo-remember","target":4}),
    )
    .unwrap();
    assert!(corpse_core::engine::dispatch(&mut r, &c).is_err());
    assert_eq!(before, serde_json::to_value(r).unwrap());
}
