use corpse_core::{Floor, generation::*};
use serde_json::Value;
#[test]
fn complete_native_generation_matches_v48_every_floor_field() {
    let fs: Value =
        serde_json::from_str(include_str!("fixtures/world-generation-v48.json")).unwrap();
    for (c, f) in fs["cases"].as_array().unwrap().iter().enumerate() {
        let result = generate_floor(
            f["depth"].as_u64().unwrap() as u32,
            f["seed"].as_u64().unwrap() as u32,
            f["domain"].as_str().unwrap(),
            f["godbeast"].as_bool().unwrap(),
            f["size"].as_u64().unwrap() as usize,
            f.get("actualUp")
                .map(|p| serde_json::from_value(p.clone()).unwrap()),
        );
        if let Some(e) = f["error"].as_str() {
            assert_eq!(result.unwrap_err(), e, "case{c}");
            continue;
        }
        let floor = result.unwrap_or_else(|e| panic!("case{c}: {e}"));
        let expected: Floor = serde_json::from_value(f["floor"].clone()).unwrap();
        if floor != expected {
            std::fs::write(
                "/tmp/world-actual.json",
                serde_json::to_string(&floor).unwrap(),
            )
            .unwrap();
            std::fs::write(
                "/tmp/world-expected.json",
                serde_json::to_string(&expected).unwrap(),
            )
            .unwrap();
            panic!(
                "world mismatch case{c} seed{} depth{} domain{}",
                f["seed"], f["depth"], f["domain"]
            )
        }
        assert!(
            validate_floor_generation(&floor),
            "generated diagnostics invalid case{c}"
        );
    }
}
