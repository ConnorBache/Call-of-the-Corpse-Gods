use corpse_core::{generation::*, rng::GenerationRng};
use serde_json::Value;
#[test]
fn geological_plans_match_v48_all_domains() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/geology-v48.json")).unwrap();
    for f in fixtures["plans"].as_array().unwrap() {
        let seed = f["seed"].as_u64().unwrap() as u32;
        let mut r = GenerationRng::new(seed);
        let p = plan_geology(
            f["width"].as_u64().unwrap() as usize,
            f["height"].as_u64().unwrap() as usize,
            f["domain"].as_str().unwrap(),
            &mut || r.random(),
            &serde_json::from_value(f["protection"].clone()).unwrap(),
        )
        .unwrap();
        let mut actual = serde_json::to_value(&p).unwrap();
        actual["field"] =
            serde_json::to_value(p.field.iter().map(|v| v.to_bits()).collect::<Vec<_>>()).unwrap();
        actual["forestDensity"] = serde_json::to_value(
            p.forest_density
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        if actual != f["result"] {
            std::fs::write(
                "/tmp/geology-actual.json",
                serde_json::to_string(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                "/tmp/geology-expected.json",
                serde_json::to_string(&f["result"]).unwrap(),
            )
            .unwrap();
            panic!(
                "geology mismatch seed {seed} domain {} width {} rng {} vs {}",
                f["domain"], f["width"], r.state, f["rngState"]
            )
        }
        assert_eq!(r.state as u64, f["rngState"].as_u64().unwrap());
    }
}
#[test]
fn geology_constrained_accretion_loops_and_materialization_match_v48() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/geology-v48.json")).unwrap();
    for f in fixtures["pipelines"].as_array().unwrap() {
        let src = &fixtures["plans"][f["index"].as_u64().unwrap() as usize];
        let seed = src["seed"].as_u64().unwrap() as u32;
        let width = src["width"].as_u64().unwrap() as usize;
        let height = src["height"].as_u64().unwrap() as usize;
        let domain = src["domain"].as_str().unwrap();
        let protection = serde_json::from_value(src["protection"].clone()).unwrap();
        let mut pr = GenerationRng::new(seed);
        let plan = plan_geology(width, height, domain, &mut || pr.random(), &protection).unwrap();
        let mut lr = GenerationRng::new(seed ^ 0x9184);
        let mut loopr = GenerationRng::new(seed ^ 0x4819);
        let mut gr = GenerationRng::new(seed ^ 0x1347);
        let result = generate_dungeon_layout(
            width,
            serde_json::from_value(src["stairs"]["up"].clone()).unwrap(),
            serde_json::from_value(src["stairs"]["down"].clone()).unwrap(),
            &mut || lr.random(),
            &plan.constraints(),
            None,
        );
        if let Some(error) = f["layoutError"].as_str() {
            assert_eq!(result.unwrap_err(), error, "seed {seed} {domain} {width}");
            continue;
        }
        let mut layout = result.unwrap();
        let added = add_room_loops(
            &mut layout,
            &mut || loopr.random(),
            &LoopOptions {
                constraints: plan.constraints(),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        assert_eq!(added as u64, f["added"].as_u64().unwrap());
        let actual = serde_json::to_value(&layout).unwrap();
        if actual != f["layout"] {
            std::fs::write(
                "/tmp/constrained-actual.json",
                serde_json::to_string(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                "/tmp/constrained-expected.json",
                serde_json::to_string(&f["layout"]).unwrap(),
            )
            .unwrap();
            panic!("constrained layout mismatch seed {seed} {domain} {width}")
        }
        let mut tiles = layout.tiles.clone();
        let terrain = generate_geology(
            &mut tiles,
            domain,
            &mut || gr.random(),
            &layout,
            &protection,
            &serde_json::from_value::<Vec<GeologyTransition>>(f["transitions"].clone()).unwrap(),
            Some(&plan),
        )
        .unwrap();
        let actual = serde_json::json!({"tiles":tiles,"terrain":terrain});
        let expected = serde_json::json!({"tiles":f["tiles"],"terrain":f["terrain"]});
        if actual != expected {
            std::fs::write(
                "/tmp/terrain-actual.json",
                serde_json::to_string(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                "/tmp/terrain-expected.json",
                serde_json::to_string(&expected).unwrap(),
            )
            .unwrap();
            panic!("terrain mismatch seed {seed} {domain} {width}")
        }
        assert_eq!(lr.state as u64, f["layoutRng"].as_u64().unwrap());
        assert_eq!(loopr.state as u64, f["loopRng"].as_u64().unwrap());
        assert_eq!(gr.state as u64, f["geologyRng"].as_u64().unwrap());
    }
}
