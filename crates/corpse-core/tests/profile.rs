use corpse_core::{Run, profile::*, storage};
#[test]
fn profile_source_parity() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/profile-v48.json")).unwrap();
    for c in fixture["cases"].as_array().unwrap() {
        let run: Run = serde_json::from_value(c["run"].clone()).unwrap();
        let before = parse_profile(Some(&c["before"].to_string())).unwrap();
        let summary = serde_json::to_value(summarize_run(&run)).unwrap();
        assert!(storage::canonical_equal(&summary, &c["summary"]));
        let (after, added) = archive_run_record(
            &before,
            &run,
            c["options"]["replay"].as_bool().unwrap(),
            c["options"]["savedAt"].as_str(),
        )
        .unwrap();
        assert_eq!(added, c["added"].as_bool().unwrap());
        assert!(storage::canonical_equal(
            &serde_json::to_value(&after).unwrap(),
            &c["after"]
        ));
        if added {
            assert!(!archive_run_record(&after, &run, false, None).unwrap().1);
        }
    }
    let mut value = fixture["parsed"].clone();
    value["highScores"] = serde_json::json!([{"corrupt":true}]);
    assert!(storage::canonical_equal(
        &serde_json::to_value(parse_profile(Some(&value.to_string())).unwrap()).unwrap(),
        &fixture["parsed"]
    ));
}
#[test]
fn profile_identity_and_corruption() {
    let mut run: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.run_id = corpse_core::Field::Missing;
    let rng = run.rng;
    assert_eq!(
        ensure_run_id(&mut run, || "fixed-id".into()).unwrap(),
        "fixed-id"
    );
    assert_eq!(
        ensure_run_id(&mut run, || panic!("must retain identity")).unwrap(),
        "fixed-id"
    );
    assert_eq!(run.rng, rng);
    assert!(parse_profile(Some("{\"version\":1,\"runArchives\":[{}]}")).is_err());
    assert_eq!(parse_profile(None).unwrap(), Profile::default());
}
#[test]
fn profile_preserves_extension_record_fields() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/profile-v48.json")).unwrap();
    let mut value = fixture["parsed"].clone();
    value["runArchives"][0]["futureMetadata"] = serde_json::json!({"optional":null,"order":[2,1]});
    let p = parse_profile(Some(&value.to_string())).unwrap();
    let encoded = serde_json::to_value(p).unwrap();
    assert_eq!(
        encoded["runArchives"][0]["futureMetadata"],
        value["runArchives"][0]["futureMetadata"]
    );
}

#[test]
fn finale_summary_and_archive_preserve_exact_ending() {
    let mut run: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = corpse_core::Field::Value(31.0);
    run.status = "won".into();
    run.run_id = corpse_core::Field::Value("flesh-escape".into());
    run.collapse = corpse_core::Field::Value(std::sync::Arc::new(Default::default()));
    std::sync::Arc::make_mut(run.collapse.as_mut().unwrap()).escaped = true;
    let before = run.clone();

    let summary = summarize_run(&run);
    assert_eq!(summary.ending.as_deref(), Some("Collapsed the flesh!"));
    assert_eq!(summary.death_reason, None);
    let (profile, added) = archive_run_record(&Profile::default(), &run, false, None).unwrap();
    assert!(added);
    assert_eq!(run, before, "summary and archive must remain detached");
    let raw = serde_json::to_string(&profile).unwrap();
    let restored = parse_profile(Some(&raw)).unwrap();
    assert_eq!(profile, restored);
    assert_eq!(restored.run_archives[0].summary, summary);
    assert_eq!(restored.high_scores[0].summary.ending, summary.ending);
}

#[test]
fn legacy_summaries_omit_optional_ending() {
    let mut run: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.status = "won".into();
    run.run_id = corpse_core::Field::Value("legacy-victory".into());
    for revision in [29.0, 30.0] {
        run.simulation_revision = corpse_core::Field::Value(revision);
        let summary = summarize_run(&run);
        assert_eq!(summary.ending, None);
        let value = serde_json::to_value(&summary).unwrap();
        assert!(value.get("ending").is_none());
        assert_eq!(serde_json::from_value::<RunSummary>(value).unwrap(), summary);
        let (profile, added) = archive_run_record(&Profile::default(), &run, false, None).unwrap();
        assert!(added);
        let value = serde_json::to_value(profile).unwrap();
        assert!(value["runArchives"][0].get("ending").is_none());
        assert!(value["highScores"][0].get("ending").is_none());
    }
}

#[test]
fn collapse_loss_keeps_exact_recorded_reason() {
    let mut run: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.status = "dead".into();
    run.run_id = corpse_core::Field::Value("flesh-fall".into());
    run.run_stats.as_mut().unwrap().death = corpse_core::Field::Value(corpse_core::RunDeath {
        reason: "Fell while escaping the collapse of the flesh.".into(),
        depth: 40.0,
        tick: 84.0,
        unpreventable: true,
        ..Default::default()
    });
    let summary = summarize_run(&run);
    assert_eq!(summary.ending, None);
    assert_eq!(
        summary.death_reason.as_deref(),
        Some("Fell while escaping the collapse of the flesh.")
    );
    assert_eq!((summary.depth, summary.tick), (40.0, 84.0));
    let (profile, added) = archive_run_record(&Profile::default(), &run, false, None).unwrap();
    assert!(added);
    assert_eq!(profile.run_archives[0].summary, summary);
    assert_eq!(
        parse_profile(Some(&serde_json::to_string(&profile).unwrap())).unwrap(),
        profile
    );
}

#[test]
fn profile_validates_optional_ending() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/profile-v48.json")).unwrap();
    for invalid in [
        serde_json::json!(""),
        serde_json::json!("x".repeat(513)),
        serde_json::json!(42),
    ] {
        let mut value = fixture["parsed"].clone();
        value["runArchives"][0]["ending"] = invalid;
        assert!(parse_profile(Some(&value.to_string())).is_err());
    }
}
