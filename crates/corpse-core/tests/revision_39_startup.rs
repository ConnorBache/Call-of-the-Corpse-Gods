use corpse_core::{startup, storage, debug_floor};

#[test]
fn convenience_runs_keep_revision39_and_future_revision_is_rejected() {
    let run = startup::new_run_shell("revision39-default", "Unblessed", false).unwrap();
    assert_eq!(run.simulation_revision.value_or(0.), 39.);
    assert!(startup::new_run_shell_revision("revision39-default", "Unblessed", false, 47.).is_err());
    assert!(startup::new_run_shell_revision("revision40-explicit", "Unblessed", false, 40.).is_ok());
    let run = debug_floor::create_run("revision39-debug", "Unblessed", 1).unwrap();
    assert_eq!(run.simulation_revision.value_or(0.), 39.);
    assert_eq!(storage::serialize(&storage::deserialize(&storage::serialize(&run).unwrap()).unwrap()).unwrap(), storage::serialize(&run).unwrap());
}

#[test]
fn revision39_keeps_generation_and_required_content_identical_to38() {
    for seed in ["r37-event-count-0047", "heart-audit-000071", "生成-é-🌱-37", "r37-event-count-0078"] {
        let mut old = startup::new_run_shell_revision(seed, "Unblessed", false, 38.).unwrap();
        let mut new = startup::new_run_shell_revision(seed, "Unblessed", false, 39.).unwrap();
        startup::ensure_floors(&mut old, 40).unwrap();
        startup::ensure_floors(&mut new, 40).unwrap();
        new.simulation_revision = 38.0.into();
        assert_eq!(serde_json::to_value(&old).unwrap(), serde_json::to_value(&new).unwrap(), "{seed}");
    }
}
