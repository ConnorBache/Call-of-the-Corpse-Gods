use corpse_core::{Run, storage::canonical_equal, travel};
use serde_json::Value;
fn compare(r: &Run, expected: &Value, label: &str) {
    let actual = serde_json::to_value(r).unwrap();
    if !canonical_equal(&actual, expected) {
        std::fs::write(
            "/tmp/native-travel.json",
            serde_json::to_vec(&actual).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "/tmp/oracle-travel.json",
            serde_json::to_vec(expected).unwrap(),
        )
        .unwrap();
        panic!("{label}")
    }
}
#[test]
fn exact_travel_departure_arrival_and_ally_stairs() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/travel-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let kind = c["kind"].as_str().unwrap();
        let result = travel::travel(
            &mut r,
            c["delta"].as_f64().unwrap(),
            c["falling"].as_bool().unwrap(),
            c["vertical"].as_bool().unwrap(),
        )
        .unwrap();
        if !c["result"].is_null() {
            assert_eq!(result, c["result"].as_bool().unwrap(), "{kind}")
        }
        compare(&r, &c["afterTravel"], kind);
        for expected in c["ticks"].as_array().unwrap() {
            r.tick = expected["tick"].as_f64().unwrap();
            travel::tick_ally_stair_followers(&mut r).unwrap();
            compare(&r, expected, &format!("{kind}:tick{}", r.tick));
        }
        if kind == "stair-return" {
            r.tick = 2.0;
            travel::tick_ally_stair_followers(&mut r).unwrap();
            r.depth = 1.0;
            r.player.x = 5.0;
            r.player.y = 5.0;
            travel::restore_returning_ally_stair_followers(&mut r).unwrap();
        }
        compare(&r, &c["expected"], kind);
    }
}
#[test]
fn exact_forced_fall_cohorts() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/fall-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let ids: Vec<f64> = serde_json::from_value(c["ids"].clone()).unwrap();
        travel::fall_actors_one_level(
            &mut r,
            c["source"].as_f64().unwrap(),
            &ids,
            c["instigator"].as_f64(),
        )
        .unwrap();
        compare(&r, &c["expected"], c["kind"].as_str().unwrap());
    }
}
#[test]
fn exact_collapse_and_corpse_placement() {
    use corpse_core::item_placement::PlacementResult;
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/collapse-v48.json")).unwrap();
    for c in cases["collapses"].as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let result = travel::collapse_descent(
            &mut r,
            c["x"].as_f64().unwrap(),
            c["y"].as_f64().unwrap(),
            c["diluted"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(result, c["result"].as_bool().unwrap());
        compare(&r, &c["expected"], c["kind"].as_str().unwrap());
    }
    for c in cases["corpses"].as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let result = if c["kind"] == "piles" {
            let (a, b) =
                corpse_core::corpse_placement::normalize_corpses(&mut r, |_, _| Ok(())).unwrap();
            serde_json::json!({"relocated":a,"discarded":b})
        } else {
            let depth = r.depth;
            let a = r
                .actors_at(depth)
                .iter()
                .find(|a| a.id == 10.0)
                .unwrap()
                .clone();
            match corpse_core::corpse_placement::place_corpse(&mut r, depth, a, |_, _| Ok(()))
                .unwrap()
            {
                PlacementResult::Placed { depth, x, y } => {
                    serde_json::json!({"status":"placed","depth":depth,"x":x,"y":y})
                }
                PlacementResult::Discarded { depth } => {
                    serde_json::json!({"status":"discarded","depth":depth})
                }
            }
        };
        assert!(
            canonical_equal(&result, &c["result"]),
            "corpse result{}",
            c["kind"]
        );
        compare(&r, &c["expected"], c["kind"].as_str().unwrap());
    }
}

#[test]
fn exact_movement_terrain_hooks() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/movement-hooks-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        travel::enemy_moved(&mut r, 10.0).unwrap();
        compare(&r, &c["expected"], c["kind"].as_str().unwrap());
    }
}
