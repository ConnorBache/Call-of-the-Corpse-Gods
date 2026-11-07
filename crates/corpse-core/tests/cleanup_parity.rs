use corpse_core::{Floor, generation::*, rng::GenerationRng};
use serde_json::Value;
fn assert_json(actual: Value, expected: &Value, label: &str) {
    if actual != *expected {
        std::fs::write(
            "/tmp/cleanup-actual.json",
            serde_json::to_string(&actual).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "/tmp/cleanup-expected.json",
            serde_json::to_string(expected).unwrap(),
        )
        .unwrap();
        panic!("cleanup mismatch {label}")
    }
}
#[test]
fn protected_stairs_and_full_cleanup_match_v48() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/cleanup-v48.json")).unwrap();
    for f in fixtures["cases"].as_array().unwrap() {
        let mut floor: Floor = serde_json::from_value(f["before"].clone()).unwrap();
        let seed = f["seed"].as_u64().unwrap() as u32;
        let label = format!("seed {seed} {} width {}", f["domain"], f["width"]);
        let mut r = GenerationRng::new(seed ^ 0x8572);
        let recess = generate_stair_recesses(&mut floor, &mut || r.random());
        if let Some(error) = f["recessError"].as_str() {
            assert_eq!(recess.unwrap_err(), error)
        } else {
            let expected: Vec<corpse_core::StairRecess> =
                serde_json::from_value(f["recesses"].clone()).unwrap();
            assert_eq!(recess.unwrap(), expected)
        }
        let expected: Floor = serde_json::from_value(f["framed"].clone()).unwrap();
        if floor != expected {
            assert_json(
                serde_json::to_value(&floor).unwrap(),
                &serde_json::to_value(expected).unwrap(),
                &format!("recess {label}"),
            )
        }
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap());
        assert_eq!(
            serde_json::to_value(protected_generation_cells(&floor)).unwrap(),
            f["protection"]
        );
        assert_eq!(
            serde_json::to_value(reachable_without_braziers(&floor, None)).unwrap(),
            f["reachable"]
        );
        let result = cleanup_generated_floor(&mut floor);
        if let Some(error) = f["error"].as_str() {
            assert_eq!(result.unwrap_err(), error, "{label}")
        } else {
            assert_json(
                serde_json::to_value(result.unwrap()).unwrap(),
                &f["result"],
                &format!("stats {label}"),
            )
        }
        let expected: Floor = serde_json::from_value(f["after"].clone()).unwrap();
        if floor != expected {
            assert_json(
                serde_json::to_value(floor).unwrap(),
                &serde_json::to_value(expected).unwrap(),
                &label,
            )
        }
    }
}
