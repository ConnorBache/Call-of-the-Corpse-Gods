use corpse_core::{Floor, generation::*, rng::GenerationRng};
use serde_json::Value;
fn equal(actual: Value, expected: &Value, label: &str) {
    if actual != *expected {
        std::fs::write(
            "/tmp/genenv-actual.json",
            serde_json::to_string(&actual).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "/tmp/genenv-expected.json",
            serde_json::to_string(expected).unwrap(),
        )
        .unwrap();
        panic!("generation environment mismatch {label}")
    }
}
fn equal_floor(actual: &Floor, expected: &Value, label: &str) {
    let expected: Floor = serde_json::from_value(expected.clone()).unwrap();
    if *actual != expected {
        equal(
            serde_json::to_value(actual).unwrap(),
            &serde_json::to_value(expected).unwrap(),
            label,
        )
    }
}
#[test]
fn native_vents_lighting_fire_safety_match_v48() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/generation-environment-v48.json")).unwrap();
    for f in fixtures["cases"].as_array().unwrap() {
        let mut floor: Floor = serde_json::from_value(f["before"].clone()).unwrap();
        let seed = f["seed"].as_u64().unwrap() as u32;
        let label = format!("seed {seed} {} width {}", f["domain"], f["width"]);
        let mut vr = GenerationRng::new(seed ^ 0x9562);
        let mut lr = GenerationRng::new(seed ^ 0x4187);
        let vents = generate_basic_vents(&mut floor, &mut || vr.random());
        assert_eq!(vents as u64, f["vents"].as_u64().unwrap());
        equal_floor(&floor, &f["afterVents"], &format!("vents {label}"));
        let lights = generate_habitation_lights(&mut floor, &mut || lr.random());
        let expected: Vec<corpse_core::FloorLightSourcesEntry> =
            serde_json::from_value(f["lights"].clone()).unwrap();
        assert_eq!(lights, expected, "{label}");
        equal_floor(&floor, &f["afterLights"], &format!("lights {label}"));
        equal(
            serde_json::to_value(build_generation_gas_components(&floor)).unwrap(),
            &f["gasGraph"],
            &label,
        );
        for e in f["exclusions"].as_array().unwrap() {
            equal(
                serde_json::to_value(initial_fuel_spawn_exclusions(
                    &floor,
                    e["kind"].as_str().unwrap(),
                ))
                .unwrap(),
                &e["cells"],
                &label,
            )
        }
        for w in f["weights"].as_array().unwrap() {
            assert_eq!(
                fire_setup_weight(&floor, w["id"].as_i64().unwrap() as i32),
                w["weight"].as_f64().unwrap()
            )
        }
        let safety = finalize_generated_fire_safety(&mut floor);
        if let Some(error) = f["error"].as_str() {
            assert_eq!(safety.unwrap_err(), error)
        } else {
            equal(
                serde_json::to_value(safety.unwrap()).unwrap(),
                &f["safety"],
                &label,
            )
        }
        equal(
            serde_json::to_value(reconcile_generated_basic_vents(&mut floor)).unwrap(),
            &f["reconciled"],
            &label,
        );
        equal_floor(&floor, &f["after"], &label);
        assert_eq!(vr.state as u64, f["ventRng"].as_u64().unwrap());
        assert_eq!(lr.state as u64, f["lightRng"].as_u64().unwrap());
    }
}
