use corpse_core::movement::{self, Point};
#[test]
fn forty_reference_fields_and_stable_steps() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/pathfinding-v1.json")).unwrap();
    for (n, c) in v.as_array().unwrap().iter().enumerate() {
        let tiles: Vec<String> = serde_json::from_value(c["tiles"].clone()).unwrap();
        let harmful: Vec<bool> = serde_json::from_value(c["harmful"].clone()).unwrap();
        let allowed: Vec<bool> = serde_json::from_value(c["allowed"].clone()).unwrap();
        let target = Point {
            x: c["target"]["x"].as_i64().unwrap() as i32,
            y: c["target"]["y"].as_i64().unwrap() as i32,
        };
        let f = movement::distance_field(
            &tiles,
            9,
            target,
            c["phase"].as_bool().unwrap(),
            c["flight"].as_bool().unwrap(),
            Some(&allowed),
            Some(&harmful),
        );
        for (i, step) in f.steps.iter().enumerate() {
            assert_eq!(
                *step as i64,
                c["steps"][i].as_i64().unwrap(),
                "scene {n} cell {i}"
            );
            let expected = &c["next"][i];
            let expected = if expected.is_null() {
                None
            } else {
                Some(Point {
                    x: expected["x"].as_i64().unwrap() as i32,
                    y: expected["y"].as_i64().unwrap() as i32,
                })
            };
            assert_eq!(
                movement::next_step(
                    &f,
                    &tiles,
                    Point {
                        x: (i % 9) as i32,
                        y: (i / 9) as i32
                    }
                ),
                expected,
                "scene {n} cell {i}"
            );
        }
    }
}
