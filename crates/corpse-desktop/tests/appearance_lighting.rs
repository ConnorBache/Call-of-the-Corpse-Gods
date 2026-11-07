#![allow(dead_code)]
mod render {
    pub type Rgb = [u8; 3];
}
#[path = "../src/appearance_light.rs"]
mod appearance_light;
#[path = "../src/appearance_profile.rs"]
mod appearance_profile;
#[path = "../src/color.rs"]
mod color;
use corpse_core::vision::{ObservedCell, ObservedLightSource, TerrainObservation};
use serde_json::{json, Value};
fn observation(value: &Value) -> TerrainObservation {
    let width = value["width"].as_u64().unwrap() as usize;
    let height = value["height"].as_u64().unwrap() as usize;
    let cells = value["cells"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, c)| ObservedCell {
            index,
            current: c["current"].as_bool().unwrap(),
            remembered: !c["current"].as_bool().unwrap(),
            knowledge: if c["current"].as_bool().unwrap() {
                "direct"
            } else {
                "remembered"
            }
            .into(),
            direct: c["current"].as_bool().unwrap(),
            tile: c["tile"].as_str().map(str::to_owned),
            substrate: None,
            door_open: c["doorOpen"].as_bool(),
            snow: false,
            ice: false,
            puddles: false,
            hellfire: false,
            biome: None,
            flowers: false,
            grate: false,
            bridge: false,
            entrance: false,
            chance: 100.,
            direct_chance: 100.,
            illumination: c["illumination"].as_f64(),
            ambient: c["ambient"].as_f64(),
            distance: 0.,
            hazards: if c["burning"].as_bool() == Some(true) {
                Some(serde_json::from_value(json!({"fire":"ichor"})).unwrap())
            } else {
                None
            },
        })
        .collect();
    let mut origins = Vec::new();
    let light_sources = value["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let x = s["x"].as_f64().unwrap();
            let y = s["y"].as_f64().unwrap();
            let emission_group = origins
                .iter()
                .position(|p| *p == (x, y))
                .unwrap_or_else(|| {
                    origins.push((x, y));
                    origins.len() - 1
                });
            ObservedLightSource {
                origin_known: true,
                emission_group,
                x,
                y,
                kind: s["kind"].as_str().unwrap().into(),
                color_role: s["colorRole"].as_str().unwrap().into(),
                power: s["power"].as_f64().unwrap(),
                falloff: s["falloff"].as_f64().unwrap(),
                actor_id: None,
                cells: serde_json::from_value(s["cells"].clone()).unwrap(),
                strengths: serde_json::from_value(s["strengths"].clone()).unwrap(),
            }
        })
        .collect();
    TerrainObservation {
        all_seeing: false,
        width,
        height,
        cells,
        light_sources,
    }
}
#[test]
fn source_all_lighting_controls_and_conservative_rays_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("../assets/appearance-light-parity-v48.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 41);
    for case in cases {
        let profile = appearance_profile::Profile::from_value(case["appearance"].clone()).unwrap();
        let snapshot = observation(&case["scene"]);
        let before = format!("{snapshot:?}");
        let actual = appearance_light::light_field(
            &snapshot,
            &profile,
            case["scene"]["time"].as_f64().unwrap(),
        );
        assert_eq!(
            appearance_light::animated(&profile),
            case["animated"].as_bool().unwrap()
        );
        for (i, l) in actual.iter().enumerate() {
            assert_eq!(l.ambient_attenuation, 1., "legacy sources never acquire potion-only attenuation");
            for channel in 0..3 {
                assert_eq!(
                    l.energy[channel],
                    case["expected"]["energy"][i * 3 + channel]
                        .as_f64()
                        .unwrap(),
                    "{} energy {i}/{channel}",
                    case["name"]
                );
                assert_eq!(
                    l.wall_energy[channel],
                    case["expected"]["wallEnergy"][i * 3 + channel]
                        .as_f64()
                        .unwrap(),
                    "{} wall {i}/{channel}",
                    case["name"]
                );
            }
            assert_eq!(l.ambient, case["expected"]["ambient"][i].as_f64().unwrap());
            assert_eq!(
                l.illumination,
                case["expected"]["illumination"][i].as_f64().unwrap()
            );
        }
        assert_eq!(format!("{snapshot:?}"), before);
    }
}
#[test]
fn source_deterministic_flicker_matches() {
    let fixture: Value =
        serde_json::from_str(include_str!("../assets/appearance-light-parity-v48.json")).unwrap();
    let cases = fixture["flicker"].as_array().unwrap();
    assert_eq!(cases.len(), 54);
    for c in cases {
        let actual = appearance_light::flicker(
            c["x"].as_i64().unwrap() as i32,
            c["y"].as_i64().unwrap() as i32,
            c["time"].as_f64().unwrap(),
            c["frequency"].as_f64().unwrap(),
            c["smoothness"].as_f64().unwrap(),
        );
        assert!((actual - c["value"].as_f64().unwrap()).abs() < 1e-14);
    }
}
#[test]
fn unknown_origin_never_expands_and_future_source_payloads_are_inert() {
    let fixture: Value =
        serde_json::from_str(include_str!("../assets/appearance-light-parity-v48.json")).unwrap();
    let mut snapshot = observation(&fixture["cases"][0]["scene"]);
    snapshot.light_sources[0].origin_known = false;
    snapshot.light_sources[0].x = -1.;
    snapshot.light_sources[0].y = -1.;
    let profile = appearance_profile::Profile::from_value(
        json!({"lighting":{"visualRadius":3,"sources":{"future":{"flickerStrength":1}}}}),
    )
    .unwrap();
    assert!(!appearance_light::animated(&profile));
    let lights = appearance_light::light_field(&snapshot, &profile, 450.);
    assert_eq!(lights[4].energy, [0.; 3]);
    assert!(lights[0].energy[0] > 0.);
}
