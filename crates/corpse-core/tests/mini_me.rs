use corpse_core::{Run, mini_me};
#[test]
fn exact_source_clone_construction() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/mini-me-v48.json")).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 48);
    for (n, c) in cases.iter().enumerate() {
        let mut r: Run = serde_json::from_value(c["input"].clone()).unwrap();
        let cell = mini_me::destination(&r);
        if c["expected"].is_null() {
            assert!(cell.is_none(), "cell{n}");
            continue;
        }
        let (x, y) = cell.unwrap();
        let a = mini_me::preview(&r, x, y);
        let expected: corpse_core::Actor = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(a, expected, "clone{n}");
        mini_me::commit(&mut r, a).unwrap();
        assert_eq!(r.next_id, c["nextId"].as_f64().unwrap());
        mini_me::dismiss(&mut r);
        assert!(r.actors_at(r.depth).is_empty());
        assert!(!r.player.conditions.contains_key("miniMeId"));
    }
}
#[test]
fn dismissal_preserves_unaffected_shared_blocks() {
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let untouched = r.actors.get("2").unwrap().clone();
    let (x, y) = mini_me::destination(&r).unwrap();
    let a = mini_me::preview(&r, x, y);
    mini_me::commit(&mut r, a).unwrap();
    mini_me::dismiss(&mut r);
    assert!(std::sync::Arc::ptr_eq(
        &untouched,
        r.actors.get("2").unwrap()
    ));
}
