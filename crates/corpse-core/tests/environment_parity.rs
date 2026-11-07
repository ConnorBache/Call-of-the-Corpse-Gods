use serde_json::Value;
fn compare(a: &Value, b: &Value, path: &str) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.as_f64(), b.as_f64(), "{path}"),
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.len(),
                b.len(),
                "keys at {path}: {:?} vs {:?}",
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>()
            );
            for (k, v) in a {
                compare(
                    v,
                    b.get(k).unwrap_or_else(|| panic!("missing {path}.{k}")),
                    &format!("{path}.{k}"),
                );
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                compare(a, b, &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}
#[test]
fn static_environment_matches_complete_v48_state() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/environment-static-v48.json")).unwrap();
    let mut run: corpse_core::Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
    corpse_core::environment::tick_environment(&mut run).unwrap();
    let actual = serde_json::to_value(run).unwrap();
    compare(&actual, &fixture["expected"], "run");
}
#[test]
fn active_environment_matches_v48() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/environment-active-v48.json")).unwrap();
    for fixture in fixtures {
        let mut run: corpse_core::Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
        for _ in 0..3 {
            corpse_core::environment::tick_environment(&mut run).unwrap();
        }
        compare(
            &serde_json::to_value(run).unwrap(),
            &fixture["expected"],
            fixture["name"].as_str().unwrap(),
        );
    }
}
#[test]
fn observed_hazards_match_v48() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/environment-active-v48.json")).unwrap();
    for fixture in fixtures {
        let mut f: corpse_core::Floor =
            serde_json::from_value(fixture["expected"]["floors"][0].clone()).unwrap();
        let mask: Vec<bool> = serde_json::from_value(fixture["mask"].clone()).unwrap();
        corpse_core::hazard_memory::remember_observed_hazards(&mut f, &mask, 123.);
        compare(
            &serde_json::to_value(f.remembered_hazards).unwrap(),
            &fixture["hazardExpected"],
            fixture["name"].as_str().unwrap(),
        );
    }
}
#[test]
fn generated_currents_match_v48() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/environment-active-v48.json")).unwrap();
    for fixture in fixtures {
        let mut f: corpse_core::Floor =
            serde_json::from_value(fixture["initial"]["floors"][0].clone()).unwrap();
        corpse_core::currents::generate_currents(&mut f);
        compare(
            &serde_json::to_value(f.currents).unwrap(),
            &fixture["currentsExpected"],
            fixture["name"].as_str().unwrap(),
        );
    }
}
#[test]
fn supported_offscreen_catchup_matches_v48() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/offscreen-v48.json")).unwrap();
    for fixture in fixtures {
        let mut r: corpse_core::Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
        let current = r.floors[0].clone();
        let inactive = r.floors[1].clone();
        if fixture["name"] == "chase-direct" {
            corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
        } else {
            corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();
        }
        if fixture["initial"]["floors"][0] == fixture["expected"]["floors"][0] {
            assert!(
                std::sync::Arc::ptr_eq(&current, &r.floors[0]),
                "unchanged active floor {}",
                fixture["name"]
            );
        }
        if fixture["name"] == "untouched" {
            assert!(std::sync::Arc::ptr_eq(&inactive, &r.floors[1]));
        }
        compare(
            &serde_json::to_value(r).unwrap(),
            &fixture["expected"],
            fixture["name"].as_str().unwrap(),
        );
    }
}
#[test]
fn ordinary_explosion_and_gas_closure_match_v48() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/explosion-v48.json")).unwrap();
    for fixture in fixtures {
        let mut r: corpse_core::Run = serde_json::from_value(fixture["initial"].clone()).unwrap();
        let options = corpse_core::environment::ExplosionOptions {
            source: Some(r.player.id),
            ignite_first: fixture["igniteFirst"].as_bool().unwrap(),
            ignite_area: fixture["igniteArea"].as_bool().unwrap(),
            ..Default::default()
        };
        let (x, y) = (r.player.x, r.player.y);
        if let Some(source) = fixture.get("sourceSnapshot") {
            let source: corpse_core::Actor = serde_json::from_value(source.clone()).unwrap();
            let options = corpse_core::environment::ExplosionOptions {
                source: Some(source.id),
                ..options
            };
            corpse_core::environment::explode_with_source_snapshot(
                &mut r, x, y, options, None, &source,
            )
            .unwrap();
        } else {
            corpse_core::environment::explode(&mut r, x, y, options, None).unwrap();
        }
        compare(
            &serde_json::to_value(r).unwrap(),
            &fixture["expected"],
            fixture["name"].as_str().unwrap(),
        );
    }
}
#[test]
fn ichor_fish_catalog_matches_all_depth_bands() {
    let cases: Value = serde_json::from_str(include_str!("fixtures/ichor-v48.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let depth = case["depth"].as_f64().unwrap();
        let actor =
            corpse_core::ichor::make_fish(777., 3., 5., depth, case["random"].as_f64().unwrap());
        compare(
            &serde_json::to_value(actor).unwrap(),
            &case["expected"],
            &format!("fish depth {depth}"),
        );
    }
}
#[test]
fn ichor_dispatch_matches_reference() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/ichor-dispatch-v48.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let mut r: corpse_core::Run = serde_json::from_value(case["initial"].clone()).unwrap();
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let command = serde_json::from_value(step["command"].clone()).unwrap();
            let outcome = corpse_core::engine::dispatch(&mut r, &command).unwrap();
            let events: Vec<corpse_core::events::PresentationEvent> =
                serde_json::from_value(step["events"].clone()).unwrap();
            assert_eq!(outcome.events, events, "{} step{i}", case["name"]);
            let mut actual = serde_json::to_value(&r).unwrap();
            let mut expected = step["state"].clone();
            for key in ["history", "hasteSnapshot"] {
                actual.as_object_mut().unwrap().remove(key);
                expected.as_object_mut().unwrap().remove(key);
            }
            compare(&actual, &expected, &format!("{} step{i}", case["name"]));
        }
    }
}

#[test]
fn status_dispatch_matches_reference() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/status-dispatch-v48.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let mut r: corpse_core::Run = serde_json::from_value(case["initial"].clone()).unwrap();
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let command = serde_json::from_value(step["command"].clone()).unwrap();
            let outcome = corpse_core::engine::dispatch(&mut r, &command).unwrap();
            let events: Vec<corpse_core::events::PresentationEvent> =
                serde_json::from_value(step["events"].clone()).unwrap();
            assert_eq!(outcome.events, events, "{} step{i}", case["name"]);
            let mut actual = serde_json::to_value(&r).unwrap();
            let mut expected = step["state"].clone();
            for key in ["history", "hasteSnapshot"] {
                actual.as_object_mut().unwrap().remove(key);
                expected.as_object_mut().unwrap().remove(key);
            }
            compare(&actual, &expected, &format!("{} step{i}", case["name"]));
        }
    }
}

#[test]
fn divine_compound_dispatch_matches_reference() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/divine-compound-v48.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let mut r: corpse_core::Run = serde_json::from_value(case["initial"].clone()).unwrap();
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let command = serde_json::from_value(step["command"].clone()).unwrap();
            let outcome = corpse_core::engine::dispatch(&mut r, &command).unwrap();
            let events: Vec<corpse_core::events::PresentationEvent> =
                serde_json::from_value(step["events"].clone()).unwrap();
            assert_eq!(outcome.events, events, "{} step{i}", case["name"]);
            let mut actual = serde_json::to_value(&r).unwrap();
            let mut expected = step["state"].clone();
            for key in ["history", "hasteSnapshot"] {
                actual.as_object_mut().unwrap().remove(key);
                expected.as_object_mut().unwrap().remove(key);
            }
            compare(&actual, &expected, &format!("{} step{i}", case["name"]));
        }
    }
}
