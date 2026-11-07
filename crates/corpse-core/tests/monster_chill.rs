use corpse_core::{Run, monster_chill};
#[test]
fn source_chill_boundaries() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/monster-chill-v48.json")).unwrap();
    assert_eq!(cases.len(), 24);
    for (n, c) in cases.iter().enumerate() {
        let mut r: Run = serde_json::from_value(c["input"].clone()).unwrap();
        monster_chill::apply_adjacency(&mut r);
        let expected: Run = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(r, expected, "case{n}");
    }
}
#[test]
fn source_passive_offscreen_boundaries() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/passive-monster-offscreen-v48.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 8);
    for (n, c) in cases.iter().enumerate() {
        let mut r: Run = serde_json::from_value(c["input"].clone()).unwrap();
        corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
        let expected: Run = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(r, expected, "hidden{n}");
    }
}
