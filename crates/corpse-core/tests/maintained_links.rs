use corpse_core::{Run, maintained_links as m};
#[test]
fn source_relationship_cleanup() {
    let cases: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/maintained-links-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 120);
    let base: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let mut r = base.clone();
        r.player = serde_json::from_value(c["initial"]["player"].clone()).unwrap();
        r.actors = serde_json::from_value(c["initial"]["actors"].clone()).unwrap();
        let id = r.player.id;
        let changed = match c["op"].as_str().unwrap() {
            "clear" => Some(m::clear_maintained_link(
                &mut r,
                id,
                c["ability"].as_str().unwrap(),
            )),
            "end" => {
                m::end_maintained_links_to_dead_target(&mut r, 4.0);
                None
            }
            _ => Some(m::normalize_dead_maintained_links(&mut r)),
        };
        assert_eq!(changed, c["changed"].as_bool(), "result{i}");
        let mut expected = base.clone();
        expected.player = serde_json::from_value(c["expected"]["player"].clone()).unwrap();
        expected.actors = serde_json::from_value(c["expected"]["actors"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&r.player).unwrap(),
            serde_json::to_value(&expected.player).unwrap(),
            "player{i} {} {}",
            c["op"],
            c["kind"]
        );
        assert_eq!(
            serde_json::to_value(&r.actors).unwrap(),
            serde_json::to_value(&expected.actors).unwrap(),
            "actors{i} {} {}",
            c["op"],
            c["kind"]
        );
    }
}
