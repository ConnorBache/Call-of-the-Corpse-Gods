use corpse_core::{Run, vision};
#[test]
fn perception_matches_twenty_reference_scenes() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/vision-v1.json")).unwrap();
    for (n, c) in cases.as_array().unwrap().iter().enumerate() {
        let r: Run = serde_json::from_value(c["run"].clone()).unwrap();
        for (i, (a, b)) in vision::terrain_visibility_chance(&r)
            .iter()
            .zip(c["chances"].as_array().unwrap())
            .enumerate()
        {
            let b = b.as_f64().unwrap();
            assert!(
                (a - b).abs() < 1e-10,
                "chance scene {n} cell {i}: {a} vs {b}"
            );
        }
        for (i, b) in c["lights"].as_array().unwrap().iter().enumerate() {
            let a = vision::tile_light(&r, i % 79, i / 79);
            let b = b.as_f64().unwrap();
            assert!(
                (a - b).abs() < 1e-10,
                "light scene {n} cell {i}: {a} vs {b}"
            );
        }
        for (a, b) in r
            .actors_at(r.depth)
            .iter()
            .zip(c["actors"].as_array().unwrap())
        {
            assert_eq!(
                vision::visible_actor(&r, a),
                b["visible"].as_bool().unwrap(),
                "scene {n}"
            );
            assert_eq!(
                vision::directly_visible_actor(&r, a),
                b["direct"].as_bool().unwrap(),
                "scene {n}"
            );
            assert_eq!(
                vision::telepathic(&r, a),
                b["telepathic"].as_bool().unwrap(),
                "scene {n}"
            );
            assert!(
                (vision::detection_chance(&r, a, &r.player) - b["detection"].as_f64().unwrap())
                    .abs()
                    < 1e-10,
                "detection scene {n}"
            );
        }
    }
}
