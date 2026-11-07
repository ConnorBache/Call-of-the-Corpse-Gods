use corpse_core::{Command, Run};
#[test]
fn actual_generated_avatar_first_actions() {
    let case: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/generated-avatar-play-v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(case["steps"].as_array().unwrap().len(), 20);
    let mut r: Run = serde_json::from_value(case["initial"].clone()).unwrap();
    for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        let c: Command = serde_json::from_value(step["command"].clone()).unwrap();
        let out = corpse_core::engine::dispatch(&mut r, &c)
            .unwrap_or_else(|e| panic!("generated Avatar step{i}: {e}"));
        let mut a = serde_json::to_value(&r).unwrap();
        let mut b =
            serde_json::to_value(serde_json::from_value::<Run>(step["state"].clone()).unwrap())
                .unwrap();
        for k in ["history", "hasteSnapshot"] {
            a.as_object_mut().unwrap().remove(k);
            b.as_object_mut().unwrap().remove(k);
        }
        if a != b {
            std::fs::write("/tmp/generated-native.json", a.to_string()).unwrap();
            std::fs::write("/tmp/generated-source.json", b.to_string()).unwrap();
            panic!(
                "actual generated Avatar step{i} differs; /tmp/generated-native.json and /tmp/generated-source.json"
            );
        }
        let expected: Vec<corpse_core::events::PresentationEvent> =
            serde_json::from_value(step["events"].clone()).unwrap();
        assert_eq!(out.events, expected, "generated events step{i}");
    }
}
