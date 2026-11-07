use corpse_core::{Run, half_here as hh, storage::canonical_equal};
use serde_json::Value;
#[test]
fn exact_half_here_lifecycle_projection() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/half-here-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let kind = c["kind"].as_str().unwrap();
        match kind {
            "create" | "recreate" => {
                assert_eq!(hh::create(&mut r, 9.0, 7.0).unwrap(), c["result"].as_f64().unwrap());
            }
            "dismiss" => hh::dismiss(&mut r),
            "legacy-color" | "custom-color" => {
                assert_eq!(
                    hh::normalize_presentation(&mut r.actors_at_mut(1.0)[0]),
                    c["result"].as_bool().unwrap()
                );
            }
            _ => hh::sync(&mut r),
        }
        assert!(
            canonical_equal(&serde_json::to_value(&r).unwrap(), &c["expected"]),
            "{kind}"
        );
        let origins: Vec<_> = hh::origins(&r).iter().map(|a| a.id).collect();
        assert!(canonical_equal(
            &serde_json::to_value(origins).unwrap(),
            &c["origins"]
        ));
        let owners: Vec<_> = r
            .actors_at(r.depth)
            .iter()
            .map(|a| hh::owner_id(&r, a))
            .collect();
        assert!(canonical_equal(
            &serde_json::to_value(owners).unwrap(),
            &c["ownerIds"]
        ));
        let ranges: Vec<_> = [(8., 8., 0.), (8., 8., 1.), (4., 5., 1.), (20., 20., 2.)]
            .into_iter()
            .map(|(x, y, d)| hh::within_range(&r, x, y, d))
            .collect();
        assert_eq!(serde_json::to_value(ranges).unwrap(), c["ranges"]);
    }
}

#[test]
fn exact_shared_endpoint_falls() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/half-here-falls-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let kind = c["kind"].as_str().unwrap();
        if kind.starts_with("collapse") {
            corpse_core::travel::collapse_descent(
                &mut r,
                if kind == "collapse-both" { 6.0 } else { 8.0 },
                5.0,
                true,
            )
            .unwrap();
        } else {
            let ids: Vec<f64> = serde_json::from_value(c["ids"].clone()).unwrap();
            corpse_core::travel::fall_actors_one_level(&mut r, 1.0, &ids, None).unwrap();
        }
        if !canonical_equal(&serde_json::to_value(&r).unwrap(), &c["expected"]) {
            std::fs::write("/tmp/hh-native.json", serde_json::to_vec(&r).unwrap()).unwrap();
            std::fs::write(
                "/tmp/hh-source.json",
                serde_json::to_vec(&c["expected"]).unwrap(),
            )
            .unwrap();
            panic!("{kind}");
        }
    }
}

#[test]
fn exact_endpoint_stat_ownership() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/half-here-stats-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let a = r.actors_at(1.0).last().unwrap();
        let actual = corpse_core::combat::effective_stats(&r, a);
        assert!(
            canonical_equal(&serde_json::to_value(actual).unwrap(), &c["expected"]),
            "{}",
            c["kind"]
        );
    }
}
