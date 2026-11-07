use corpse_core::{Command, Run};
#[test]
fn starter_combat_reference_traces() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/combat-starters-v48.json")).unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 9);
    let names: Vec<_> = cases
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "missed",
            "counter",
            "number",
            "swift",
            "float",
            "fortune",
            "venom",
            "cross-reaction",
            "blorb"
        ]
    );
    for case in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(case["initial"].clone()).unwrap();
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let command: Command = serde_json::from_value(step["command"].clone()).unwrap();
            let outcome = corpse_core::engine::dispatch(&mut r, &command).unwrap();
            let actual_events: Vec<corpse_core::events::PresentationEvent> =
                serde_json::from_value(step["events"].clone()).unwrap();
            assert_eq!(
                outcome.events, actual_events,
                "event scene {} step{i}",
                case["kind"]
            );
            let mut actual = serde_json::to_value(&r).unwrap();
            let mut expected =
                serde_json::to_value(serde_json::from_value::<Run>(step["state"].clone()).unwrap())
                    .unwrap();
            for key in ["history", "hasteSnapshot"] {
                actual.as_object_mut().unwrap().remove(key);
                expected.as_object_mut().unwrap().remove(key);
            }
            if actual != expected {
                std::fs::write("/tmp/starter-combat-actual.json", actual.to_string()).unwrap();
                std::fs::write("/tmp/starter-combat-expected.json", expected.to_string()).unwrap();
                panic!(
                    "actor scene {} step {i}, see /tmp/starter-combat-actual.json and /tmp/starter-combat-expected.json",
                    case["kind"]
                );
            }
        }
    }
}
