#![allow(dead_code)]
mod render {
    pub type Rgb = [u8; 3];
}
#[path = "../src/appearance_material.rs"]
mod appearance_material;
#[path = "../src/appearance_profile.rs"]
mod appearance_profile;
#[path = "../src/color.rs"]
mod color;
#[path = "../src/color_profile.rs"]
mod color_profile;
#[path = "../src/entity_profile.rs"]
mod entity_profile;
use serde_json::{json, Value};
fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../assets/appearance-entities-parity-v48.json"
    ))
    .unwrap()
}
#[test]
fn source_entity_inheritance_parent_and_tuning_match() {
    let fixture = fixtures();
    let cases = fixture["entities"].as_array().unwrap();
    assert_eq!(cases.len(), 59);
    for c in cases {
        let profile = appearance_profile::Profile::from_value(c["appearance"].clone()).unwrap();
        let color = serde_json::from_value(c["base"]["color"].clone()).unwrap();
        let result = entity_profile::resolve(
            &profile,
            c["group"].as_str().unwrap(),
            c["key"].as_str().unwrap(),
            c["parent"].as_str(),
            c["base"]["glyph"].as_str().unwrap(),
            color,
        );
        let expected = &c["expected"];
        assert_eq!(
            result.glyph,
            expected["glyph"].as_str().unwrap(),
            "{}",
            c["name"]
        );
        assert_eq!(json!(result.color), expected["color"], "{}", c["name"]);
        assert_eq!(
            json!(result.background),
            expected["background"],
            "{}",
            c["name"]
        );
        assert_eq!(
            result.emphasis,
            expected["emphasis"].as_f64().unwrap(),
            "{}",
            c["name"]
        );
        assert_eq!(
            result.speed,
            expected["speed"].as_f64().unwrap(),
            "{}",
            c["name"]
        );
    }
}
#[test]
fn source_material_inheritance_matches() {
    let fixture = fixtures();
    let cases = fixture["materials"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for c in cases {
        let profile = appearance_profile::Profile::from_value(c["appearance"].clone()).unwrap();
        assert_eq!(
            profile
                .material(c["key"].as_str().unwrap(), c["parent"].as_str())
                .unwrap(),
            c["expected"]
        );
    }
}
#[test]
fn source_feature_colors_fallbacks_and_grading_match() {
    let fixture = fixtures();
    let cases = fixture["features"].as_array().unwrap();
    assert_eq!(cases.len(), 26);
    for c in cases {
        let profile = appearance_profile::Profile::from_value(c["appearance"].clone()).unwrap();
        let fallback = if c["fallback"].is_null() {
            None
        } else {
            Some(serde_json::from_value::<([u8; 3], [u8; 3])>(c["fallback"].clone()).unwrap())
        };
        let key = c["key"].as_str().unwrap();
        let (fg, bg) = appearance_material::feature_colors(
            &profile,
            key,
            c["x"].as_u64().unwrap() as usize,
            c["y"].as_u64().unwrap() as usize,
            c["seed"].as_u64().unwrap() as u32,
            fallback,
        );
        assert_eq!(json!([fg, bg]), c["expected"], "{key}");
        assert_eq!(
            json!(appearance_material::feature_color(
                &profile,
                key,
                [152, 118, 84]
            )),
            c["single"],
            "single {key}"
        );
    }
}
#[test]
fn source_stationary_and_animated_pulses_match() {
    let fixture = fixtures();
    let cases = fixture["pulses"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
    for c in cases {
        let speed = c["speed"].as_f64().unwrap();
        let now = c["now"].as_f64().unwrap();
        let x = c["x"].as_i64().unwrap() as i32;
        let y = c["y"].as_i64().unwrap() as i32;
        assert!(
            (appearance_material::material_pulse(speed, now, x, y)
                - c["material"].as_f64().unwrap())
            .abs()
                < 1e-14
        );
        assert!(
            (appearance_material::entity_pulse(speed, now, x, y) - c["entity"].as_f64().unwrap())
                .abs()
                < 1e-14
        );
    }
    let cases = fixture["animated"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for c in cases {
        let profile = appearance_profile::Profile::from_value(c["appearance"].clone()).unwrap();
        assert_eq!(
            appearance_material::animated(&profile),
            c["expected"].as_bool().unwrap()
        );
    }
}

#[test]
fn source_observed_fog_transition_and_all_seeing_match() {
    let fixture = fixtures();
    let cases = fixture["fog"].as_array().unwrap();
    assert_eq!(cases.len(), 48);
    for c in cases {
        let p = appearance_profile::Profile::from_value(c["appearance"].clone()).unwrap();
        let knowledge = match c["knowledge"].as_str().unwrap() {
            "direct" => color::Knowledge::Direct,
            "telepathic" => color::Knowledge::Telepathic,
            "remembered" => color::Knowledge::Remembered,
            _ => color::Knowledge::Layout,
        };
        let ctx = color::Context {
            tile: "floor",
            knowledge,
            distance: 8.,
            light: color::Light {
                display: None,
                ambient: 0.,
                illumination: 0.,
                ambient_attenuation: 1.,
                energy: [0.1f32 as f64, 0.2f32 as f64, 0.3f32 as f64],
                wall_energy: [0.3f32 as f64, 0.2f32 as f64, 0.1f32 as f64],
            },
            important: false,
        };
        let (fg, bg) = color_profile::map_colors_observed(
            &p,
            [81, 91, 107],
            [21, 25, 35],
            ctx,
            false,
            1.,
            c["chance"].as_f64().unwrap(),
            c["allSeeing"].as_bool().unwrap(),
        );
        assert_eq!(
            json!([fg, bg]),
            c["expected"],
            "{} {} chance {}",
            c["knowledge"],
            c["allSeeing"],
            c["chance"]
        );
    }
}
