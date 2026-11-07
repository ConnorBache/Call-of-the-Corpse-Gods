use corpse_core::{Floor, generation::*};
use serde_json::Value;
fn equal(actual: Value, expected: &Value, label: &str) {
    if actual != *expected {
        std::fs::write(
            "/tmp/topology-actual.json",
            serde_json::to_string(&actual).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "/tmp/topology-expected.json",
            serde_json::to_string(expected).unwrap(),
        )
        .unwrap();
        panic!("topology mismatch {label}")
    }
}
#[test]
fn measured_topology_monotone_repair_and_landmark_gates_match_v48() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/topology-v48.json")).unwrap();
    for f in fixtures["cases"].as_array().unwrap() {
        let mut floor: Floor = serde_json::from_value(f["before"].clone()).unwrap();
        let label = format!("seed {} {} width {}", f["seed"], f["domain"], f["width"]);
        let input: Floor = serde_json::from_value(f["baselineInput"].clone()).unwrap();
        let baseline = capture_landmark_quality_baseline(&input);
        equal(
            serde_json::to_value(&baseline).unwrap(),
            &f["baseline"],
            &format!("baseline {label}"),
        );
        equal(
            serde_json::to_value(measure_generated_topology(&floor)).unwrap(),
            &f["measured"],
            &format!("measure {label}"),
        );
        let result = finalize_generated_topology(&mut floor);
        if let Some(error) = f["error"].as_str() {
            assert_eq!(result.unwrap_err(), error)
        } else {
            equal(
                serde_json::to_value(result.unwrap()).unwrap(),
                &f["result"],
                &format!("repair {label}"),
            )
        }
        equal(
            serde_json::to_value(evaluate_generated_topology(
                &floor,
                &measure_generated_topology(&floor),
            ))
            .unwrap(),
            &serde_json::to_value(
                serde_json::from_value::<TopologyAcceptance>(f["quality"].clone()).unwrap(),
            )
            .unwrap(),
            &format!("gate {label}"),
        );
        let actual = measure_landmark_quality(&floor, &baseline).unwrap();
        let expected: LandmarkQualityResult =
            serde_json::from_value(f["landmarks"].clone()).unwrap();
        equal(
            serde_json::to_value(actual).unwrap(),
            &serde_json::to_value(expected).unwrap(),
            &format!("landmarks {label}"),
        );
        let expected: Floor = serde_json::from_value(f["after"].clone()).unwrap();
        if floor != expected {
            equal(
                serde_json::to_value(floor).unwrap(),
                &serde_json::to_value(expected).unwrap(),
                &label,
            )
        }
    }
}
