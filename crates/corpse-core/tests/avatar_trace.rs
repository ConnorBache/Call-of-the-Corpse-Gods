use corpse_core::{Command, Run};
#[test]
fn avatar_reference_trace_non_temporal_state() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1-trace.json")).unwrap();
    let mut run: Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
    for (i, step) in fixture["steps"].as_array().unwrap().iter().enumerate() {
        let command: Command = serde_json::from_value(step["command"].clone()).unwrap();
        corpse_core::engine::dispatch(&mut run, &command).unwrap();
        let mut actual = serde_json::to_value(&run).unwrap();
        let mut expected =
            serde_json::to_value(serde_json::from_value::<Run>(step["state"].clone()).unwrap())
                .unwrap();
        for key in ["history", "hasteSnapshot"] {
            actual.as_object_mut().unwrap().remove(key);
            expected.as_object_mut().unwrap().remove(key);
        }
        if actual != expected {
            std::fs::write("/tmp/avatar-actual.json", actual.to_string()).unwrap();
            std::fs::write("/tmp/avatar-expected.json", expected.to_string()).unwrap();
            panic!(
                "full non-temporal state mismatch at command {i}; inspect /tmp/avatar-actual.json and /tmp/avatar-expected.json"
            );
        }
    }
}
#[test]
fn unsupported_command_is_transactional() {
    let mut run: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before = run.clone();
    assert!(
        corpse_core::engine::dispatch(
            &mut run,
            &Command::Ability {
                id: "unsupported-ability-test".into(),
                target: Default::default(),
                x: Default::default(),
                y: Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(run, before);
}
#[test]
fn avatar_player_terminal_death_matches_reference() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-death-v1-trace.json")).unwrap();
    let mut run: Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
    for (i, step) in fixture["steps"].as_array().unwrap().iter().enumerate() {
        let command: Command = serde_json::from_value(step["command"].clone()).unwrap();
        corpse_core::engine::dispatch(&mut run, &command).unwrap();
        let mut actual = serde_json::to_value(&run).unwrap();
        let mut expected =
            serde_json::to_value(serde_json::from_value::<Run>(step["state"].clone()).unwrap())
                .unwrap();
        for key in ["history", "hasteSnapshot"] {
            actual.as_object_mut().unwrap().remove(key);
            expected.as_object_mut().unwrap().remove(key);
        }
        assert!(actual == expected, "death state mismatch at command {i}");
    }
}
