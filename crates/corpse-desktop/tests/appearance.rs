//! Data-only regressions can run without a video display or SDL initialization.
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
#[path = "../src/appearance_store.rs"]
mod appearance_store;
use appearance_profile::Profile;
use appearance_store::Store;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
fn temp() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "corpse-appearance-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn source_validation_edit_reset_fixtures_match() {
    // R50 fixtures retain explicit old profiles, rebasing only omitted defaults.
    // The frozen v48 originals and enumerated R50/R51 derivation remain in source.
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../assets/appearance-parity-v50.json")).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let mut result = Profile::from_value(c["input"].clone());
        if let Some(edit) = c.get("edit") {
            result =
                result.and_then(|p| p.set(edit["path"].as_str().unwrap(), edit["value"].clone()));
        }
        if c.get("error").is_some() {
            assert!(result.is_err(), "fixture {i} should reject");
            continue;
        }
        let result = result.unwrap_or_else(|e| panic!("fixture {i}: {e}"));
        assert_eq!(result.data(), &c["output"], "fixture {i}");
        if let Some(reset) = c.get("reset") {
            assert_eq!(
                result
                    .reset(c["edit"]["path"].as_str().unwrap())
                    .unwrap()
                    .data(),
                reset,
                "reset fixture {i}"
            );
        }
    }
}
#[test]
fn defaults_and_control_catalog_are_exact_and_immutable() {
    assert_eq!(appearance_profile::controls().len(), 1665);
    let p = Profile::default();
    assert_eq!(Profile::parse(&p.json()).unwrap(), p);
    let edited = p.set("terrain.wall.color", json!("#abcdef")).unwrap();
    assert_eq!(p.text("terrain.wall.color"), "#777987");
    assert_eq!(edited.rgb("terrain.wall.color"), [171, 205, 239]);
    assert!(!appearance_profile::controls()
        .iter()
        .any(|c| c.path == "terrain.gas:caustic.glyph"));
    assert!(p.set("ui.layout.leftSidebarWidth", json!(220)).is_err());
}
#[test]
fn inherited_materials_change_only_unedited_child_fields() {
    let p = Profile::default()
        .set("terrain.wall.color", json!("#abcdef"))
        .unwrap();
    assert_eq!(
        p.material("wall:natural", Some("wall")).unwrap()["color"],
        "#abcdef"
    );
    let p = p
        .set("terrain.wall:natural.color", json!("#f1f2f3"))
        .unwrap();
    assert_eq!(
        p.material("wall:natural", Some("wall")).unwrap()["color"],
        "#f1f2f3"
    );
}
#[test]
fn entity_reset_restores_inheritance_without_losing_numeric_tuning() {
    let p = Profile::default()
        .set("entities.objects.player.color", json!("#abcdef"))
        .unwrap()
        .set("entities.objects.player.brightness", json!(2))
        .unwrap();
    assert!(!p.boolean("entities.objects.player.inherited"));
    let p = p.reset("entities.objects.player.color").unwrap();
    assert!(p.boolean("entities.objects.player.inherited"));
    assert_eq!(p.number("entities.objects.player.brightness"), 2.);
}
#[test]
fn malformed_import_and_failed_property_edits_do_not_mutate() {
    let mut s = Store::memory();
    s.set("global.exposure", json!(1.7)).unwrap();
    let before = s.export_json();
    for bad in [
        "{",
        r#"{"schemaVersion":2}"#,
        r##"{"terrain":{"wall":{"color":"#abc"}}}"##,
        r#"{"global":{"exposure":"1"}}"#,
        r#"{"constructor":{}}"#,
    ] {
        assert!(s.import_json(bad).is_err());
        assert_eq!(s.export_json(), before);
    }
    assert!(s.set("global.gamma", json!(-1)).is_err());
    assert_eq!(s.export_json(), before);
    s.undo();
    assert_eq!(s.edited().number("global.exposure"), 1.);
}
#[test]
fn import_export_preserves_unknown_fields_and_is_undoable() {
    let mut s = Store::memory();
    s.import_json(
        r#"{"presetName":"Future","future":{"settings":[true,1,"☃"]},"global":{"exposure":1.8}}"#,
    )
    .unwrap();
    let exported = s.export_json();
    assert_eq!(
        Profile::parse(&exported).unwrap().value("future.settings"),
        Some(&json!([true, 1, "☃"]))
    );
    s.undo();
    assert_eq!(s.edited(), &Profile::default());
    s.redo();
    assert_eq!(s.export_json(), exported);
}
#[test]
fn gestures_undo_redo_and_redo_invalidation() {
    let mut s = Store::memory();
    s.begin_gesture();
    for value in [1.1, 1.2, 1.3, 1.4] {
        s.set("global.exposure", json!(value)).unwrap();
    }
    s.end_gesture();
    s.undo();
    assert_eq!(s.edited().number("global.exposure"), 1.);
    assert!(!s.status().can_undo);
    s.redo();
    assert_eq!(s.edited().number("global.exposure"), 1.4);
    s.undo();
    s.set("global.exposure", json!(2.)).unwrap();
    assert!(!s.status().can_redo);
}
#[test]
fn preview_export_commit_discard_close_and_original_comparison() {
    let mut s = Store::memory();
    s.open();
    s.set("global.exposure", json!(1.5)).unwrap();
    let committed = s.export_json();
    s.begin_preview();
    s.set("global.exposure", json!(2.)).unwrap();
    assert_eq!(s.profile().number("global.exposure"), 2.);
    assert_eq!(s.export_json(), committed);
    s.compare("original").unwrap();
    assert_eq!(s.profile().number("global.exposure"), 1.);
    s.compare("edited").unwrap();
    assert_eq!(s.profile().number("global.exposure"), 2.);
    s.cancel_preview();
    assert_eq!(s.profile().number("global.exposure"), 1.5);
    s.begin_preview();
    s.set("global.exposure", json!(2.5)).unwrap();
    s.commit_preview();
    s.undo();
    assert_eq!(s.profile().number("global.exposure"), 1.5);
    s.redo();
    s.begin_preview();
    s.set("global.exposure", json!(3.)).unwrap();
    s.close();
    assert_eq!(s.profile().number("global.exposure"), 2.5);
    assert!(!s.status().previewing);
    s.open();
    s.compare("original").unwrap();
    assert_eq!(s.profile().number("global.exposure"), 2.5);
}
#[test]
fn preset_crud_startup_and_preferences_roundtrip() {
    let dir = temp();
    let path = dir.join("settings.json");
    let mut s = Store::load(&path);
    s.set("global.exposure", json!(1.8)).unwrap();
    let id = s.save_preset(" Warm ").unwrap();
    assert!(!s.status().dirty);
    s.set_default(Some(&id)).unwrap();
    s.preferences(true, true).unwrap();
    let copy = s.duplicate_preset(&id, "Evening").unwrap();
    s.rename_preset(&copy, "Dusk").unwrap();
    assert!(s.save_preset("wArM").is_err());
    s.set("global.exposure", json!(2.2)).unwrap();
    s.update_preset(&id).unwrap();
    let mut loaded = Store::load(&path);
    assert_eq!(loaded.profile().number("global.exposure"), 2.2);
    assert!(loaded.reduced_motion && loaded.fullscreen);
    assert_eq!(loaded.presets().len(), 2);
    loaded.load_preset(&copy).unwrap();
    assert_eq!(loaded.profile().number("global.exposure"), 1.8);
    loaded.compare(&id).unwrap();
    assert_eq!(loaded.profile().number("global.exposure"), 2.2);
    assert!(loaded.delete_preset(&id, false).is_err());
    loaded.delete_preset(&id, true).unwrap();
    assert_eq!(loaded.status().comparison, "edited");
    assert!(loaded.status().default_preset_id.is_none());
    let fresh = Store::load(&path);
    assert_eq!(fresh.profile(), &Profile::default());
    assert_eq!(fresh.presets().len(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn working_edits_are_not_silently_persisted_without_save() {
    let dir = temp();
    let path = dir.join("settings.json");
    let mut s = Store::load(&path);
    s.preferences(true, false).unwrap();
    s.set("global.exposure", json!(2.)).unwrap();
    s.close();
    assert_eq!(Store::load(&path).profile(), &Profile::default());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn failed_storage_rolls_back_library_and_keeps_work_exportable() {
    let dir = temp();
    let mut s = Store::load(&dir.join("missing/settings.json"));
    s.set("global.exposure", json!(2.)).unwrap();
    let before = s.export_json();
    assert!(s.save_preset("First").is_err());
    assert!(s.presets().is_empty());
    assert_eq!(s.export_json(), before);
    assert!(s.preferences(true, false).is_err());
    assert!(!s.reduced_motion);
    std::fs::create_dir(dir.join("missing")).unwrap();
    assert_eq!(s.save_preset("First").unwrap(), "preset-1");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn corrupt_storage_is_left_untouched_on_load() {
    let dir = temp();
    let path = dir.join("settings.json");
    std::fs::write(&path, "{broken json").unwrap();
    let s = Store::load(&path);
    assert!(!s.error.is_empty());
    assert_eq!(s.profile(), &Profile::default());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken json");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn atomic_export_import_and_empty_parent_path() {
    let dir = temp();
    let path = dir.join("appearance.json");
    let mut s = Store::memory();
    s.set("global.saturation", json!(1.7)).unwrap();
    s.export_file(&path).unwrap();
    s.reset_all();
    s.import_file(&path).unwrap();
    assert_eq!(s.edited().number("global.saturation"), 1.7);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn history_is_bounded_and_reset_is_reversible() {
    let mut s = Store::memory();
    for i in 0..100 {
        s.set("global.exposure", json!(1. + i as f64 / 100.))
            .unwrap();
    }
    let before = s.export_json();
    s.reset_all();
    assert_eq!(s.edited(), &Profile::default());
    s.undo();
    assert_eq!(s.export_json(), before);
    let mut count = 0;
    while s.status().can_undo {
        s.undo();
        count += 1;
    }
    assert!(count <= 80);
    assert!(count > 1);
}
#[test]
fn unsafe_depth_and_oversized_values_are_rejected() {
    let mut v = json!(0);
    for _ in 0..34 {
        v = json!({"nested":v});
    }
    assert!(Profile::from_value(json!({"future":v})).is_err());
    assert!(Profile::from_value(json!({"future":"x".repeat(100_001)})).is_err());
    assert!(Profile::from_value(json!({"future":[]})).is_ok());
}
#[test]
fn source_store_transition_fixtures_match() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../assets/appearance-store-parity-v50.json")).unwrap();
    let dir = temp();
    let mut count = 0;
    for (sequence, fixture) in fixtures.iter().enumerate() {
        let path = dir.join(format!("{sequence}.json"));
        let mut store = Store::load(&path);
        for (step, case) in fixture["steps"].as_array().unwrap().iter().enumerate() {
            let action = case["action"].as_array().unwrap();
            let arg = |i: usize| action[i].as_str().unwrap();
            match arg(0) {
                "openEditor" => store.open(),
                "closeEditor" => store.close(),
                "beginGesture" => store.begin_gesture(),
                "endGesture" => store.end_gesture(),
                "setValue" => store.set(arg(1), action[2].clone()).unwrap(),
                "resetProperty" => store.reset_property(arg(1)).unwrap(),
                "resetGroup" => store.reset_group(arg(1)).unwrap(),
                "resetAll" => store.reset_all(),
                "undo" => store.undo(),
                "redo" => store.redo(),
                "setComparison" => store.compare(arg(1)).unwrap(),
                "savePreset" => {
                    store.save_preset(arg(1)).unwrap();
                }
                "setDefaultPreset" => store.set_default(Some(arg(1))).unwrap(),
                "duplicatePreset" => {
                    store.duplicate_preset(arg(1), arg(2)).unwrap();
                }
                "renamePreset" => store.rename_preset(arg(1), arg(2)).unwrap(),
                "updatePreset" => store.update_preset(arg(1)).unwrap(),
                "loadPreset" => store.load_preset(arg(1)).unwrap(),
                "deletePreset" => store
                    .delete_preset(arg(1), action[2].as_bool().unwrap())
                    .unwrap(),
                "reload" => store = Store::load(&path),
                "previewSet" => {
                    store.preview(store.profile().set(arg(1), action[2].clone()).unwrap())
                }
                "cancelPreview" => store.cancel_preview(),
                "commitPreview" => store.commit_preview(),
                "importJSON" => store.import_json(arg(1)).unwrap(),
                other => panic!("Unknown fixture operation {other}"),
            }
            let status = store.status();
            let actual = json!({"dirty":status.dirty,"previewing":status.previewing,"canUndo":status.can_undo,"canRedo":status.can_redo,"comparison":status.comparison,"savedPresetId":status.saved_preset_id,"defaultPresetId":status.default_preset_id});
            assert_eq!(
                store.profile().data(),
                &case["appearance"],
                "sequence {sequence} step {step} appearance"
            );
            assert_eq!(
                store.edited().data(),
                &case["edited"],
                "sequence {sequence} step {step} edited"
            );
            assert_eq!(
                actual, case["status"],
                "sequence {sequence} step {step} status"
            );
            count += 1;
        }
    }
    assert_eq!(count, 41);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn malformed_native_preferences_roll_back_all_fields() {
    let dir = temp();
    let path = dir.join("settings.json");
    std::fs::write(&path,json!({"version":1,"reducedMotion":true,"fullscreen":"wrong","library":{"version":1,"nextId":1,"defaultPresetId":null,"presets":[]}}).to_string()).unwrap();
    let store = Store::load(&path);
    assert!(!store.error.is_empty());
    assert!(!store.reduced_motion);
    assert!(!store.fullscreen);
    assert!(store.presets().is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn preset_count_and_total_library_budget_are_transactional() {
    let mut store = Store::memory();
    for i in 0..50 {
        store.save_preset(&format!("Preset {i}")).unwrap();
    }
    let before = store.export_json();
    assert!(store.save_preset("Overflow").is_err());
    assert_eq!(store.presets().len(), 50);
    assert_eq!(store.export_json(), before);
    let mut store = Store::memory();
    store
        .import_json(&json!({"future":{"large":"x".repeat(90_000)}}).to_string())
        .unwrap();
    let mut saved = 0;
    for i in 0..50 {
        if store.save_preset(&format!("Large {i}")).is_err() {
            break;
        }
        saved += 1;
    }
    assert!(saved > 1 && saved < 50);
    assert_eq!(store.presets().len(), saved);
    assert_eq!(
        store
            .edited()
            .value("future.large")
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        90_000
    );
}
#[test]
fn history_character_budget_retains_nearest_changes() {
    let large: Vec<String> = (0..15).map(|_| "x".repeat(90_000)).collect();
    let mut store = Store::memory();
    store
        .import_json(&json!({"future":large}).to_string())
        .unwrap();
    for i in 0..9 {
        store
            .set("global.exposure", json!(1.1 + i as f64 / 10.))
            .unwrap();
    }
    store.undo();
    assert!((store.edited().number("global.exposure") - 1.8).abs() < 1e-12);
    let mut retained = 1;
    while store.status().can_undo {
        store.undo();
        retained += 1;
    }
    assert!(retained < 8);
    assert!(retained > 1);
}

#[test]
fn near_limit_export_uses_compact_json_and_can_be_imported_again() {
    let mut data = Profile::default().data().clone();
    data["future"] = json!(vec!["x".repeat(90_000); 21]);
    let length = appearance_profile::utf16_len(&data.to_string());
    let remaining = appearance_profile::MAX_JSON_CHARACTERS - 1 - length - 3;
    assert!(remaining < 100_000);
    data["future"]
        .as_array_mut()
        .unwrap()
        .push(json!("x".repeat(remaining)));
    let profile = Profile::from_value(data).unwrap();
    assert!(
        appearance_profile::utf16_len(&serde_json::to_string_pretty(profile.data()).unwrap())
            > appearance_profile::MAX_JSON_CHARACTERS
    );
    let exported = profile.json();
    assert!(appearance_profile::utf16_len(&exported) <= appearance_profile::MAX_JSON_CHARACTERS);
    assert!(!exported.contains('\n'));
    assert_eq!(Profile::parse(&exported).unwrap(), profile);
    assert!(Profile::default().json().contains('\n'));
}
#[test]
fn default_directory_preparation_is_explicit_and_path_is_per_user() {
    let dir = temp();
    let path = dir.join("nested/app/appearance.json");
    assert!(!path.parent().unwrap().exists());
    appearance_store::prepare_default_directory(&path).unwrap();
    assert!(path.parent().unwrap().is_dir());
    assert!(!path.exists());
    assert!(
        appearance_store::default_path().ends_with("appearance.json")
            || appearance_store::default_path().ends_with("native-appearance.json")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn native_float_input_equal_to_default_is_a_true_noop() {
    let mut store = Store::memory();
    store.set("global.exposure", Value::from(1.0)).unwrap();
    assert!(!store.status().dirty);
    assert!(!store.status().can_undo);
    assert_eq!(store.edited(), &Profile::default());
    let profile = Profile::default()
        .set("terrain.wall.brightness", Value::from(2.0))
        .unwrap()
        .set("terrain.wall:natural.brightness", Value::from(1.0))
        .unwrap();
    assert_eq!(
        profile.material("wall:natural", Some("wall")).unwrap()["brightness"].as_f64(),
        Some(2.)
    );
}

#[test]
fn historical_birther_glyph_overrides_remain_loadable() {
    for kind in ["pale:Birther", "pale:Greater Birther"] {
        let path = format!("entities.creatures.{kind}.glyph");
        let profile = Profile::default().set(&path, json!("(≬)")).unwrap();
        assert_eq!(Profile::parse(&profile.json()).unwrap(), profile);
        assert_eq!(profile.text(&path), "(≬)");
        assert!(Profile::default().set(&path, json!("(任)")).is_err());
    }
    assert!(Profile::default().set("entities.creatures.bear.glyph", json!("(≬)")).is_err());
}
