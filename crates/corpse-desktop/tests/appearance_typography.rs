#![allow(dead_code)]
mod render {
    pub type Rgb = [u8; 3];
}
mod color {
    pub fn hex(s: &str) -> [u8; 3] {
        let n = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
        [(n >> 16) as u8, (n >> 8) as u8, n as u8]
    }
}
#[path = "../src/appearance_profile.rs"]
mod appearance_profile;
#[path = "../src/typography_ui.rs"]
mod typography_ui;
use appearance_profile::Profile;
use serde_json::{json, Value};
use typography_ui::{Face, Role, RoleStyle};
#[test]
fn frozen_source_role_override_rules_match_for_every_role() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../assets/appearance-typography-parity-v48.json"
    ))
    .unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 53);
    for case in cases {
        let p = Profile::from_value(case["appearance"].clone()).unwrap();
        for role in Role::ALL {
            let mut baseline = RoleStyle::baseline(21.);
            baseline.font_family = "native baseline face".into();
            baseline.font_weight = 500.;
            baseline.letter_spacing = 0.7;
            baseline.line_height = 1.3;
            let mut expected = baseline.clone();
            for (key, value) in case["expected"][role.key()].as_object().unwrap() {
                let value = value.as_str().unwrap();
                match key.as_str() {
                    "font-family" => expected.font_family = value.into(),
                    "font-size" => {
                        expected.font_size = value.trim_end_matches("px").parse().unwrap()
                    }
                    "font-weight" => expected.font_weight = value.parse().unwrap(),
                    "letter-spacing" => {
                        expected.letter_spacing = value.trim_end_matches("px").parse().unwrap()
                    }
                    "line-height" => expected.line_height = value.parse().unwrap(),
                    _ => panic!("unexpected source declaration"),
                }
            }
            assert_eq!(
                typography_ui::resolve(&p, role, baseline),
                expected,
                "{} {}",
                case["name"],
                role.key()
            );
            assert_eq!(
                typography_ui::overrides(&p, role).any(),
                !case["expected"][role.key()].as_object().unwrap().is_empty()
            );
        }
        for (base_line, base_height) in [(24., 108.), (22., 99.)] {
            let variables = &case["variables"];
            let expected_line = variables["--chronicle-line"].as_f64().unwrap_or(base_line);
            let expected_height = variables["--chronicle-height"]
                .as_f64()
                .unwrap_or(base_height);
            assert_eq!(
                typography_ui::chronicle_metrics(&p, base_line, base_height),
                (expected_line, expected_height),
                "{}",
                case["name"]
            );
        }
    }
}
#[test]
fn default_role_reset_preserves_selector_specific_weight_and_size() {
    let base = RoleStyle {
        font_size: 21.,
        font_weight: 500.,
        line_height: 1.3,
        ..RoleStyle::baseline(21.)
    };
    let d = Profile::default();
    assert_eq!(
        typography_ui::resolve(&d, Role::Headings, base.clone()),
        base
    );
    let p = d.set("typography.headings.fontWeight", json!(700)).unwrap();
    assert_eq!(
        typography_ui::resolve(&p, Role::Headings, base.clone()).font_weight,
        700.
    );
    let p = p.reset("typography.headings.fontWeight").unwrap();
    assert_eq!(
        typography_ui::resolve(&p, Role::Headings, base.clone()),
        base
    );
}
#[test]
fn bundled_family_order_and_unavailable_faces_are_explicit() {
    let p = Profile::default();
    let ui = typography_ui::font_candidates(p.text("typography.primary.fontFamily"), false);
    assert_eq!(ui.faces[0], Face::IbmPlexMono);
    assert_eq!(ui.unavailable, ["Cascadia Mono"]);
    let map = typography_ui::font_candidates(p.text("typography.map.fontFamily"), true);
    assert_eq!(
        map.faces,
        [
            Face::BrogueAscii,
            Face::Inconsolata,
            Face::CorpseGlyphMono,
            Face::CorpseGlyphSymbols,
            Face::CorpseGlyphCjk,
            Face::IbmPlexMono
        ]
    );
    assert!(map.unavailable.is_empty());
    let selected = typography_ui::font_candidates(
        "'Inconsolata', \"IBM Plex Mono\", Inconsolata, monospace",
        false,
    );
    assert_eq!(selected.faces[0], Face::Inconsolata);
    assert_eq!(selected.faces[1], Face::IbmPlexMono);
    assert_eq!(selected.faces.len(), 5);
    let unavailable =
        typography_ui::font_candidates("'Brogue CE ASCII', 'A platform font', serif", false);
    assert!(!unavailable.faces.contains(&Face::BrogueAscii));
    assert_eq!(
        unavailable.unavailable,
        ["Brogue CE ASCII", "A platform font", "serif"]
    );
    assert_eq!(Role::parse("chronicle"), Some(Role::Chronicle));
    assert_eq!(Role::parse("map"), None);
}
#[test]
fn explicit_height_overrides_derived_track_and_compact_baseline_survives() {
    let p=Profile::from_value(json!({"typography":{"chronicle":{"fontSize":24,"lineHeight":1.5}},"ui":{"layout":{"chronicleHeight":180}}})).unwrap();
    assert_eq!(typography_ui::chronicle_metrics(&p, 22., 99.), (36., 180.));
    let p = p.reset("ui.layout.chronicleHeight").unwrap();
    assert_eq!(typography_ui::chronicle_metrics(&p, 22., 99.), (36., 162.));
    assert_eq!(
        typography_ui::chronicle_metrics(&Profile::default(), 22., 99.),
        (22., 99.)
    );
}
