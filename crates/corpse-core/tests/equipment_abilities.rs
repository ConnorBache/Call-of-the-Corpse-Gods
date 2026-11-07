use corpse_core::{Run, equipment_abilities as e};
#[test]
fn source_equipment_projection_and_load_reconciliation() {
    let f: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/equipment-abilities-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases = f.as_array().unwrap();
    assert_eq!(cases.len(), 192);
    let base: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let mut r = base.clone();
        r.player = serde_json::from_value(c["initial"]["player"].clone()).unwrap();
        r.actors = serde_json::from_value(c["initial"]["actors"].clone()).unwrap();
        r.inventory = serde_json::from_value(c["initial"]["inventory"].clone()).unwrap();
        r.tick = c["initial"]["tick"].as_f64().unwrap();
        assert_eq!(
            serde_json::to_value(e::equipment_ability_state(&r, &r.inventory)).unwrap(),
            c["projection"],
            "projection{i}"
        );
        assert_eq!(
            serde_json::to_value(e::learned_abilities(&r)).unwrap(),
            c["learned"],
            "learned{i}"
        );
        assert!(!e::reconcile(&mut r, false).unwrap());
        corpse_core::half_here::sync(&mut r);
        let mut expected = base.clone();
        expected.player = serde_json::from_value(c["expected"]["player"].clone()).unwrap();
        expected.actors = serde_json::from_value(c["expected"]["actors"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&r.player).unwrap(),
            serde_json::to_value(&expected.player).unwrap(),
            "player{i}"
        );
        assert_eq!(
            serde_json::to_value(&r.actors).unwrap(),
            serde_json::to_value(&expected.actors).unwrap(),
            "actors{i}"
        );
    }
}
