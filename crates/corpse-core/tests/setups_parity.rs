use corpse_core::{Floor, GodbeastEvent, generation::*, rng::GenerationRng};
use serde_json::Value;
fn equal<T: serde::Serialize + PartialEq>(a: &T, b: &T, label: &str) {
    if a != b {
        std::fs::write("/tmp/setups-actual.json", serde_json::to_string(a).unwrap()).unwrap();
        std::fs::write(
            "/tmp/setups-expected.json",
            serde_json::to_string(b).unwrap(),
        )
        .unwrap();
        panic!("setup mismatch {label}")
    }
}
#[test]
fn authored_stamps_packing_and_godbeast_rooms_match_v48() {
    let fs: Value = serde_json::from_str(include_str!("fixtures/setups-v48.json")).unwrap();
    for (c, f) in fs["stamps"].as_array().unwrap().iter().enumerate() {
        let mut floor: Floor = serde_json::from_value(f["before"].clone()).unwrap();
        stamp_setup(
            &mut floor,
            f["id"].as_u64().unwrap() as u32,
            6,
            6,
            &serde_json::from_value::<Vec<String>>(f["kinds"].clone()).unwrap(),
        )
        .unwrap();
        let expected: Floor = serde_json::from_value(f["after"].clone()).unwrap();
        equal(&floor, &expected, &format!("stamp{c}"));
    }
    for (c, f) in fs["cases"].as_array().unwrap().iter().enumerate() {
        let mut floor: Floor = serde_json::from_value(f["before"].clone()).unwrap();
        let mut rng = GenerationRng::new(f["seed"].as_u64().unwrap() as u32);
        generate_room_doors(&mut floor, &mut || rng.random());
        equal(
            &floor,
            &serde_json::from_value::<Floor>(f["doors"].clone()).unwrap(),
            &format!("doors{c}"),
        );
        let result = generate_setups(&mut floor, &mut || rng.random());
        if let Some(e) = f["error"].as_str() {
            assert_eq!(result.unwrap_err(), e, "case{c}")
        } else {
            result.unwrap();
        }
        assert_eq!(rng.state, f["state"].as_u64().unwrap() as u32, "rng{c}");
        equal(
            &floor,
            &serde_json::from_value::<Floor>(f["after"].clone()).unwrap(),
            &format!("packing{c}"),
        );
    }
    for (c, f) in fs["beasts"].as_array().unwrap().iter().enumerate() {
        let mut tiles: Vec<String> = serde_json::from_value(f["before"].clone()).unwrap();
        let mut rng = GenerationRng::new(f["seed"].as_u64().unwrap() as u32);
        let event = reserve_godbeast_room(
            &mut tiles,
            f["kind"].as_str().unwrap(),
            &mut || rng.random(),
            &serde_json::from_value::<Vec<Point>>(f["avoid"].clone()).unwrap(),
            &serde_json::from_value::<Vec<Point>>(f["preferred"].clone()).unwrap(),
            &serde_json::from_value(f["core"].clone()).unwrap(),
        )
        .unwrap();
        equal(
            &event,
            &serde_json::from_value::<GodbeastEvent>(f["event"].clone()).unwrap(),
            &format!("beast{c}"),
        );
        equal(
            &tiles,
            &serde_json::from_value(f["tiles"].clone()).unwrap(),
            &format!("beast tiles{c}"),
        );
        assert_eq!(rng.state, f["state"].as_u64().unwrap() as u32);
    }
}
