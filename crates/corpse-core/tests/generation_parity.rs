use corpse_core::{generation::*, rng::GenerationRng};
use serde_json::Value;
fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/generation-v48.json")).unwrap()
}
#[test]
fn room_masks_match_v48_including_order_and_rng() {
    for f in fixtures()["rooms"].as_array().unwrap() {
        let seed = f["seed"].as_u64().unwrap() as u32;
        let mut r = GenerationRng::new(seed);
        let actual = make_room_shape(
            serde_json::from_value(f["kind"].clone()).unwrap(),
            serde_json::from_value(f["sizeClass"].clone()).unwrap(),
            &mut || r.random(),
        );
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            f["result"],
            "{seed} {} {}",
            f["kind"],
            f["sizeClass"]
        );
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap())
    }
}
#[test]
fn stair_spines_match_v48_current_legacy_and_clamped() {
    for f in fixtures()["stairs"].as_array().unwrap() {
        let actual = floor_stairs(
            f["depth"].as_u64().unwrap() as u32,
            f["seed"].as_u64().unwrap() as u32,
            f["size"].as_i64().unwrap() as i32,
            serde_json::from_value(f["actualUp"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(actual).unwrap(), f["result"], "{f}")
    }
}
#[test]
fn passage_cells_and_paths_match_v48() {
    for f in fixtures()["passages"].as_array().unwrap() {
        let i = &f["input"];
        let actual = make_room_passage(
            i["width"].as_u64().unwrap() as usize,
            serde_json::from_value(i["from"].clone()).unwrap(),
            serde_json::from_value(i["direction"].clone()).unwrap(),
            i["gap"].as_i64().unwrap() as i32,
            i["offset"].as_i64().unwrap() as i32,
            serde_json::from_value(i["style"].clone()).unwrap(),
            i["start"].as_bool().unwrap(),
            i["end"].as_bool().unwrap(),
            i["bend"].as_f64().unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(actual).unwrap(), f["result"], "{i}")
    }
}
#[test]
fn domain_plan_matches_v48() {
    for f in fixtures()["domains"].as_array().unwrap() {
        assert_eq!(
            serde_json::to_value(domain_plan(f["seed"].as_u64().unwrap() as u32)).unwrap(),
            f["result"]
        )
    }
}
#[test]
fn unsupported_floor_configuration_is_rejected() {
    assert!(
        generate_floor(1, 42, "Dungeon", false, 80, None)
            .unwrap_err()
            .contains("Supported floor geometries")
    );
    assert!(floor_stairs(0, 42, 79, None).is_err());
    assert!(floor_stairs(1, 42, 80, None).is_err());
}
#[test]
fn geological_fields_match_v48_float32_bits_and_rng() {
    for f in fixtures()["fields"].as_array().unwrap() {
        let mut r = GenerationRng::new(f["seed"].as_u64().unwrap() as u32);
        let values = value_field(
            f["width"].as_u64().unwrap() as usize,
            f["height"].as_u64().unwrap() as usize,
            f["wavelength"].as_u64().unwrap() as usize,
            &mut || r.random(),
        )
        .unwrap();
        let actual: Vec<u32> = values.iter().map(|v| v.to_bits()).collect();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            f["bits"],
            "seed {} width {} wavelength {}",
            f["seed"],
            f["width"],
            f["wavelength"]
        );
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap());
    }
}
#[test]
fn heart_budgets_match_v48() {
    for f in fixtures()["budgets"].as_array().unwrap() {
        let seed = if let Some(s) = f["seed"].as_str() {
            BudgetSeed::Text(s)
        } else {
            BudgetSeed::Number(f["seed"].as_u64().unwrap() as u32)
        };
        assert_eq!(
            ordinary_blessing_budget(seed, f["depth"].as_u64().unwrap() as u32).unwrap() as u64,
            f["result"].as_u64().unwrap(),
            "{f}"
        )
    }
}
#[test]
fn encounter_budgets_match_v48() {
    for f in fixtures()["encounters"].as_array().unwrap() {
        assert_eq!(
            serde_json::to_value(base_encounter_budget(
                f["depth"].as_u64().unwrap() as u32,
                f["cells"].as_u64().unwrap() as usize
            ))
            .unwrap(),
            f["result"]
        );
    }
}
#[test]
fn legacy_loot_allocations_match_v48() {
    for f in fixtures()["loot"].as_array().unwrap() {
        let actual = retained_loot_allocation(
            f["depth"].as_u64().unwrap() as u32,
            &[],
            f["hearts"].as_f64().unwrap(),
            f["carried"].as_f64().unwrap(),
        )
        .unwrap();
        let expected: indexmap::IndexMap<String, f64> =
            serde_json::from_value(f["result"].clone()).unwrap();
        assert_eq!(actual, expected, "{f}")
    }
}
#[test]
fn recorded_loot_acceptance_and_fallback_match_v48() {
    use corpse_core::{Field, Floor, GenerationMetadata, GenerationStageRecord};
    use std::sync::Arc;
    for f in fixtures()["recordedLoot"].as_array().unwrap() {
        let floors: Vec<Arc<Floor>> = f["floors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let mut floor = Floor::default();
                floor.depth = p["depth"].as_f64().unwrap();
                let mut g = GenerationMetadata::default();
                let mut stage = GenerationStageRecord::default();
                stage.id = "items".into();
                stage.details = Field::Value(
                    serde_json::from_value(p["generation"]["stages"][0]["details"].clone())
                        .unwrap(),
                );
                g.stages.push(stage);
                floor.generation = Field::Value(g);
                Arc::new(floor)
            })
            .collect();
        let actual =
            retained_loot_allocation(f["depth"].as_u64().unwrap() as u32, &floors, 1.0, 1.0)
                .unwrap();
        let expected: indexmap::IndexMap<String, f64> =
            serde_json::from_value(f["result"].clone()).unwrap();
        assert_eq!(actual, expected, "{f}")
    }
}
#[test]
fn full_room_accretion_matches_v48_stages_graph_and_rng() {
    for f in fixtures()["layouts"].as_array().unwrap() {
        let seed = f["seed"].as_u64().unwrap() as u32;
        let mut r = GenerationRng::new(seed);
        let mut stages = vec![];
        let mut cb =
            |id: &str, l: &DungeonLayout| stages.push(serde_json::json!({"id":id,"layout":l}));
        let actual = generate_dungeon_layout(
            f["size"].as_u64().unwrap() as usize,
            serde_json::from_value(f["entrance"].clone()).unwrap(),
            serde_json::from_value(f["exit"].clone()).unwrap(),
            &mut || r.random(),
            &serde_json::from_value(f["constraints"].clone()).unwrap(),
            Some(&mut cb),
        );
        if let Some(error) = f["error"].as_str() {
            assert_eq!(actual.unwrap_err(), error)
        } else {
            let actual = serde_json::to_value(actual.unwrap()).unwrap();
            if actual != f["result"] {
                std::fs::write(
                    "/tmp/native-layout-actual.json",
                    serde_json::to_string(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/native-layout-expected.json",
                    serde_json::to_string(&f["result"]).unwrap(),
                )
                .unwrap();
                panic!(
                    "layout mismatch seed {seed} size {} constraints {} rng {} vs {}",
                    f["size"], f["constraints"], r.state, f["rngState"]
                )
            }
        }
        assert_eq!(serde_json::to_value(stages).unwrap(), f["stages"]);
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap());
    }
}
#[test]
fn room_loops_match_v48_including_bent_and_rejected_candidates() {
    let fixtures = fixtures();
    for f in fixtures["loops"].as_array().unwrap() {
        let index = f["index"].as_u64().unwrap() as usize;
        let source = &fixtures["layouts"][index];
        let mut layout: DungeonLayout = serde_json::from_value(source["result"].clone()).unwrap();
        let seed = source["seed"].as_u64().unwrap() as u32;
        let mut r = GenerationRng::new(seed ^ 0x713974);
        let mode = f["mode"].as_str().unwrap();
        let options = LoopOptions {
            allow_bent: mode != "straight",
            maximum_candidate_checks: if mode == "gated" { Some(5) } else { None },
            ..Default::default()
        };
        let mut gate = |_: &DungeonLayout, c: &LayoutConnector| c.route_before.unwrap() >= 40;
        let added = add_room_loops(
            &mut layout,
            &mut || r.random(),
            &options,
            if mode == "gated" {
                Some(&mut gate)
            } else {
                None
            },
        )
        .unwrap();
        assert_eq!(added as u64, f["added"].as_u64().unwrap());
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap());
        let actual = serde_json::to_value(layout).unwrap();
        if actual != f["result"] {
            std::fs::write(
                "/tmp/native-loops-actual.json",
                serde_json::to_string(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                "/tmp/native-loops-expected.json",
                serde_json::to_string(&f["result"]).unwrap(),
            )
            .unwrap();
            panic!(
                "loop mismatch seed {seed} size {} mode {mode}",
                source["size"]
            );
        }
    }
}
