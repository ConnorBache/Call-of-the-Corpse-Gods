use corpse_core::{Actor, enemy_traits as e};
#[test]
fn source_intrinsic_and_presentation_normalization() {
    let f: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/enemy-traits-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases = f.as_array().unwrap();
    assert_eq!(cases.len(), 602);
    for (i, c) in cases.iter().enumerate() {
        let mut a: Actor = serde_json::from_value(c["initial"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(e::enemy_traits(&a)).unwrap(),
            c["traits"],
            "traits{i}"
        );
        let costs: Vec<f64> = serde_json::from_value(c["costs"].clone()).unwrap();
        assert_eq!(e::energy_ability_costs(&a), costs, "costs{i}");
        for (id, expected) in c["has"].as_object().unwrap() {
            assert_eq!(
                e::has_enemy_ability(&a, id),
                expected.as_bool().unwrap(),
                "has{i} {id}"
            );
        }
        let mut p = a.clone();
        assert_eq!(
            e::normalize_presentation(&mut p),
            c["changed"].as_bool().unwrap(),
            "changed{i}"
        );
        let expected: Actor = serde_json::from_value(c["presentation"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            serde_json::to_value(expected).unwrap(),
            "presentation{i}"
        );
        e::initialize_enemy_traits(&mut a);
        let expected: Actor = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(expected).unwrap(),
            "initialized{i}"
        );
        let once = a.clone();
        e::initialize_enemy_traits(&mut a);
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(once).unwrap(),
            "idempotent{i}"
        );
    }
}
