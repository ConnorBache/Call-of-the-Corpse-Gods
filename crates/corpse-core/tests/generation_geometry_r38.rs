use corpse_core::{generation::*, rng::seed_hash, startup};
use indexmap::IndexSet;

#[test]
fn distant_tap_excludes_planned_stair_footprint_in_r38() {
    let w = 79;
    let mut tiles = vec!["wall".to_string(); w * 29];
    let avoid = [Point { x: 10, y: 18 }, Point { x: 15, y: 8 }];
    // Sole distant dry cell is the future upstairs. The old choice is reproduced.
    tiles[18 * w + 10] = "floor".into();
    let mut old = tiles.clone();
    let legacy = reserve_godbeast_room_revision(&mut old, "Veridian", &mut || 0.0,
        &avoid, &[Point { x: 55, y: 14 }], &IndexSet::new(), 37.0).unwrap();
    assert_eq!((legacy.tap.as_ref().unwrap().x, legacy.tap.as_ref().unwrap().y), (10.0, 18.0));
    let revised = reserve_godbeast_room_revision(&mut tiles, "Veridian", &mut || 0.0,
        &avoid, &[Point { x: 55, y: 14 }], &IndexSet::new(), 38.0).unwrap();
    let p = revised.tap.as_ref().unwrap();
    assert!(avoid.iter().all(|a| (p.x-a.x as f64).abs().max((p.y-a.y as f64).abs()) > 1.0));
    assert_eq!(tiles[p.y as usize*w+p.x as usize], "floor");
}

#[test]
fn seed_0047_tap_survives_stair_painting_and_cleanup() {
    let shell = startup::new_run_shell_revision("r37-event-count-0047", "Unblessed", false, 37.0).unwrap();
    let seed = seed_hash(&shell.seed);
    let old = generate_floor_full_revision(20, seed, Some(Point{x:10,y:18}), 37.0).unwrap();
    let tap = old.godbeast_event.as_ref().unwrap().tap.as_ref().unwrap();
    assert_eq!(old.tiles[tap.y as usize*old.width()+tap.x as usize], "up");
    let revised = generate_floor_full_revision(20, seed, Some(Point{x:10,y:18}), 38.0).unwrap();
    let tap = revised.godbeast_event.as_ref().unwrap().tap.as_ref().unwrap();
    let at = tap.y as usize*revised.width()+tap.x as usize;
    assert!(["floor", "grass"].contains(&revised.tiles[at].as_str()));
    assert!(reachable_without_braziers(&revised, None).contains(&at));
    assert!(validate_floor_generation(&revised));
}

#[test]
fn recovery_window_keeps_stairs_domain_and_godbeast_plan() {
    let seed = 1234567;
    let first = generate_floor_full_revision(20, seed, None, 38.0).unwrap();
    let later = generate_floor_full_revision_from_attempt(20, seed, None, 38.0, 32).unwrap();
    assert_eq!(first.domain, later.domain);
    assert_eq!(first.godbeast_event.as_ref().map(|e|&e.kind), later.godbeast_event.as_ref().map(|e|&e.kind));
    for kind in ["up", "down"] {
        assert_eq!(first.tiles.iter().position(|t|t==kind), later.tiles.iter().position(|t|t==kind));
    }
    let attempt = later.generation.as_ref().unwrap().stages.iter().find(|s|s.id=="cleanup")
        .unwrap().details.as_ref().unwrap()["generationAttempt"];
    assert!((32.0..64.0).contains(&attempt));
    assert_eq!(generate_floor_full_revision_from_attempt(20, seed, None, 38.0, 32).unwrap(), later);
}

#[test]
fn invalid_recovery_inputs_are_rejected_without_attempting_geometry() {
    for start in [1, 31, 225, 256, u32::MAX] {
        assert!(generate_floor_full_revision_from_attempt(20, 0, None, 38.0, start).is_err());
    }
    assert!(generate_floor_full_revision_from_attempt(20, 0, None, 37.0, 32).is_err());
    assert!(generate_floor_candidate_revision(20, 0, "Dungeon", false, 79, None, u32::MAX, 38.0).is_err());
    assert!(generate_floor_full_revision(0, 0, None, 38.0).is_err());
    assert!(generate_floor_full_revision(41, 0, None, 38.0).is_err());
    // The existing actual-up policy clamps carried stair coordinates safely.
    let f = generate_floor_full_revision(1, 0, Some(Point{x:i32::MIN,y:i32::MAX}), 38.0).unwrap();
    assert_eq!(f.tiles.iter().position(|t|t=="up"), Some(23*79+5));
}

#[test]
fn protected_room_failure_retries_a_safe_candidate_in_r38() {
    let (depth, seed, up) = (13, 2657626501, Some(Point { x: 15, y: 18 }));
    let old = generate_floor_full_revision(depth, seed, up, 37.0).unwrap_err();
    assert_eq!(old, "Authored geometry leaves an inaccessible room");
    let f = generate_floor_full_revision(depth, seed, up, 38.0).unwrap();
    let seen = reachable_without_braziers(&f, None);
    assert!(transitions_accessible(&f, &seen));
    assert!(validate_floor_generation(&f));
    let attempt = f.generation.as_ref().unwrap().stages.iter().find(|s|s.id=="cleanup")
        .unwrap().details.as_ref().unwrap()["generationAttempt"];
    assert!((1.0..32.0).contains(&attempt));
}

#[test]
fn late_mountain_chasms_preserve_hazard_free_stair_routes() {
    // These seeds passed geometry validation but late habitat removals forced the
    // player through permanent fire. Check every final floor, not only its base map.
    for seed in ["r37-event-count-0078", "93", "生成-é-🌱-113", "0-1-2:qa-120", "11-10-9:qa-127"] {
        let mut run = startup::new_run_shell_revision(seed, "Unblessed", false, 38.0).unwrap();
        startup::ensure_floors(&mut run, 40).unwrap();
        for f in &run.floors {
            let reached = reachable_without_braziers(f, None);
            assert!(transitions_accessible(f, &reached), "{seed} floor {}", f.depth);
        }
    }
}
