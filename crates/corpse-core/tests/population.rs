use corpse_core::{Floor, population::*};
use serde_json::Value;
#[test]
fn exact_composition_streams_all_depths() {
    let f: Value =
        serde_json::from_str(include_str!("../../../fixtures/population-v48.json")).unwrap();
    for c in f["plans"].as_array().unwrap() {
        let p = plan_budgeted_hordes(
            c["seed"].as_str().unwrap(),
            c["depth"].as_u64().unwrap() as u32,
            c["total"].as_i64().unwrap() as i32,
            0,
            c["water"].as_bool().unwrap(),
            &[],
        )
        .unwrap();
        assert_eq!(serde_json::to_value(p).unwrap(), c["expected"], "{c}");
    }
}
#[test]
fn exact_actor_templates_and_draws() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/population-v48.json")).unwrap();
    for c in fixture["actors"].as_array().unwrap() {
        let mut f = Floor {
            depth: 17.0,
            tiles: vec!["floor".into(); 2500],
            ..Default::default()
        };
        let mut next = 10.0;
        let mut rng = population_random("actor-native");
        let a = create_population_actor(
            &mut next,
            &mut f,
            c["kind"].as_str().unwrap(),
            1275,
            c["sleeping"].as_bool().unwrap(),
            &mut rng,
        )
        .unwrap();
        assert!(
            corpse_core::storage::canonical_equal(&serde_json::to_value(a).unwrap(), &c["actor"]),
            "{}",
            c["kind"]
        );
        assert_eq!(next, c["nextId"].as_f64().unwrap());
        let mut expected = population_random("actor-native");
        for _ in 0..c["draws"].as_u64().unwrap() {
            expected.random();
        }
        assert_eq!(rng.state, expected.state);
        assert!(corpse_core::storage::canonical_equal(
            &serde_json::to_value(f.slime).unwrap(),
            &c["slime"]
        ));
    }
}
#[test]
fn complete_population_and_heatmaps() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/populated-floors-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut f: Floor = serde_json::from_value(c["initial"].clone()).unwrap();
        let map = corpse_core::spawn_heatmap::build_spawn_heatmap(&f);
        let actual = serde_json::json!({"weights":map.weights,"flags":map.flags,"land":map.land,"water":map.water});
        assert!(
            corpse_core::storage::canonical_equal(&actual, &c["heat"]),
            "heat depth{}",
            f.depth
        );
        let mut next = 4.0;
        let seed = c["seed"].as_str().unwrap();
        let mut actors = populate_hordes(seed, &mut next, &mut f, &[], None).unwrap();
        let machines = populate_machine_enemies(seed, &mut next, &mut f, &actors, &[]).unwrap();
        actors.extend(machines);
        assert!(
            corpse_core::storage::canonical_equal(
                &serde_json::to_value(&actors).unwrap(),
                &c["actors"]
            ),
            "actors depth{}",
            f.depth
        );
        assert!(
            corpse_core::storage::canonical_equal(
                &serde_json::to_value(&f).unwrap(),
                &c["expectedFloor"]
            ),
            "floor depth{}",
            f.depth
        );
        assert_eq!(next, c["nextId"].as_f64().unwrap());
    }
}
#[test]
fn complete_spawn_floor_and_sphinx() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    for c in fixture["sphinx"].as_array().unwrap() {
        let mut f: Floor = serde_json::from_value(c["initial"].clone()).unwrap();
        let mut rng = population_random(c["seed"].as_str().unwrap());
        let room = corpse_core::sphinx_generation::plan_sphinx_room(&mut f, &mut rng);
        assert!(
            corpse_core::storage::canonical_equal(&serde_json::to_value(room).unwrap(), &c["room"]),
            "sphinx room{}",
            f.depth
        );
        assert!(
            corpse_core::storage::canonical_equal(
                &serde_json::to_value(&f).unwrap(),
                &c["expectedFloor"]
            ),
            "sphinx floor{}",
            f.depth
        );
        let mut expected = population_random(c["seed"].as_str().unwrap());
        for _ in 0..c["draws"].as_u64().unwrap() {
            expected.random();
        }
        assert_eq!(rng.state, expected.state);
    }
    for c in fixture["cases"].as_array().unwrap() {
        let mut r: corpse_core::Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let depth = c["depth"].as_u64().unwrap() as u32;
        let prior: Vec<Floor> = serde_json::from_value(fixture["priorFloors"].clone()).unwrap();
        let mut floors: Vec<_> = prior
            .into_iter()
            .take(depth as usize - 1)
            .map(std::sync::Arc::new)
            .collect();
        floors.extend(r.floors.clone());
        r.floors = floors;
        corpse_core::startup::spawn_floor(&mut r, depth).unwrap();
        r.floors = r.floors.into_iter().skip(depth as usize - 1).collect();
        let actual = serde_json::to_value(r).unwrap();
        if !corpse_core::storage::canonical_equal(&actual, &c["expected"]) {
            std::fs::write(
                format!("/tmp/native-spawn-{depth}.json"),
                serde_json::to_vec(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("/tmp/oracle-spawn-{depth}.json"),
                serde_json::to_vec(&c["expected"]).unwrap(),
            )
            .unwrap();
            panic!("spawn floor{depth}");
        }
    }
}
#[test]
fn complete_new_run_three_modes_with_reference_geometry() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    let floors: Vec<Floor> = serde_json::from_value(fixture["geometry"].clone()).unwrap();
    for expected in fixture["runs"].as_array().unwrap() {
        let seed = expected["seed"].as_str().unwrap();
        let mode = expected["mode"].as_str().unwrap();
        let r = corpse_core::startup::create_run_with_revision(
            seed,
            mode,
            false,
            &|depth, _, _| Ok(floors[depth as usize - 1].clone()),
            29.0,
        )
        .unwrap();
        let actual = serde_json::to_value(r).unwrap();
        if !corpse_core::storage::canonical_equal(&actual, expected) {
            std::fs::write(
                format!("/tmp/native-newrun-{mode}.json"),
                serde_json::to_vec(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("/tmp/oracle-newrun-{mode}.json"),
                serde_json::to_vec(expected).unwrap(),
            )
            .unwrap();
            panic!("newrun {mode}")
        }
    }
}
#[test]
fn ground_placement_metrics_falls_and_normalization() {
    use corpse_core::{GroundItem, Run, item_placement::*};
    use std::{cell::RefCell, sync::Arc};
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/item-placement-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let entry: GroundItem = serde_json::from_value(c["entry"].clone()).unwrap();
        let ensured = RefCell::new(vec![]);
        let ensure = |r: &mut Run, d: f64| {
            ensured.borrow_mut().push(d);
            if !r.floors.iter().any(|f| f.depth == d) {
                let mut f = (*r.floors[0]).clone();
                f.depth = d;
                f.tiles.fill("floor".into());
                f.dirty = false;
                r.floors.push(Arc::new(f));
            }
            Ok(())
        };
        let result = if c["type"] == "piles" {
            let (a, b) = normalize_ground_items(&mut r, ensure).unwrap();
            serde_json::json!({"relocated":a,"discarded":b})
        } else {
            match place_ground_item(
                &mut r,
                if c["type"] == "deep-fall" { 40.0 } else { 1.0 },
                entry,
                ensure,
            )
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
            corpse_core::storage::canonical_equal(&result, &c["result"]),
            "result{}",
            c["type"]
        );
        assert!(
            corpse_core::storage::canonical_equal(
                &serde_json::to_value(r).unwrap(),
                &c["expected"]
            ),
            "state{}",
            c["type"]
        );
        assert!(corpse_core::storage::canonical_equal(
            &serde_json::to_value(ensured.into_inner()).unwrap(),
            &c["ensured"]
        ));
    }
}
#[test]
fn complete_native_new_run_three_modes() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    for expected in fixture["runs"].as_array().unwrap() {
        let seed = expected["seed"].as_str().unwrap();
        let mode = expected["mode"].as_str().unwrap();
        let r = corpse_core::startup::create_run_revision(seed, mode, false, 29.0).unwrap();
        let actual = serde_json::to_value(r).unwrap();
        if !corpse_core::storage::canonical_equal(&actual, expected) {
            std::fs::write(
                format!("/tmp/native-fullrun-{mode}.json"),
                serde_json::to_vec(&actual).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("/tmp/oracle-fullrun-{mode}.json"),
                serde_json::to_vec(expected).unwrap(),
            )
            .unwrap();
            panic!("native fullrun {mode}")
        }
    }
}
