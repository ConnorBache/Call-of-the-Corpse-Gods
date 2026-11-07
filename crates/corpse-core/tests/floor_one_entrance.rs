use corpse_core::{
    Field, Floor, entrance, generation, startup, storage,
    replay::{ReplayRecord, ReplaySession},
    temporal::{History, Snapshot},
};
use std::sync::Arc;

fn simple_floor() -> Floor {
    let (w, h) = (79, 29);
    let mut tiles = vec!["wall".to_string(); w * h];
    for x in 10..=39 { tiles[23 * w + x] = "floor".into(); }
    for y in 5..=23 { tiles[y * w + 10] = "floor".into(); }
    tiles[23 * w + 39] = "up".into();
    tiles[5 * w + 10] = "down".into();
    Floor {
        width: (w as f64).into(), height: (h as f64).into(), depth: 1.0,
        domain: "Dungeon".into(), tiles, lights: vec![0.0; w * h], seen: vec![false; w * h],
        ..Default::default()
    }
}

fn assert_connected(f: &Floor) {
    let at = entrance::cell(f).unwrap();
    assert_eq!(at, (f.height() - 2) * f.width() + f.width() / 2);
    assert_eq!(f.tiles.iter().filter(|t| *t == "up").count(), 1);
    let arrival = entrance::arrival(f).unwrap();
    assert_eq!((arrival.x, arrival.y), ((at % f.width()) as i32, (at / f.width()) as i32 - 1));
    let reachable = generation::reachable_without_braziers(f, None);
    assert!(generation::transitions_accessible(f, &reachable));
    assert!(reachable.len() > 5);
    for x in 0..f.width() {
        assert_eq!(f.tiles[x], "wall");
        assert_eq!(f.tiles[(f.height() - 1) * f.width() + x], "wall");
    }
}

#[test]
fn connected_bottom_center_door_is_distinct_and_idempotent() {
    let mut f = simple_floor();
    let old_down = f.tiles.iter().position(|t| t == "down").unwrap();
    entrance::install_floor_one_entrance(&mut f).unwrap();
    assert_connected(&f);
    assert_eq!(f.tiles[23 * f.width() + 39], "floor");
    assert_eq!(f.tiles[old_down], "down");
    let once = f.clone();
    entrance::install_floor_one_entrance(&mut f).unwrap();
    assert_eq!(f, once);
    let encoded = serde_json::to_value(&f).unwrap();
    assert_eq!(encoded["entrance"], serde_json::json!({"x":39.0,"y":27.0}));
    assert_eq!(serde_json::from_value::<Floor>(encoded).unwrap(), f);
}

#[test]
fn corridor_avoids_fixtures_and_invalid_install_is_atomic() {
    let mut f = simple_floor();
    f.permanent_fire = vec![(25 * f.width() + 39) as f64].into();
    entrance::install_floor_one_entrance(&mut f).unwrap();
    assert_connected(&f);
    assert_eq!(f.tiles[25 * f.width() + 39], "wall");
    assert_eq!(f.permanent_fire.as_ref().unwrap(), &vec![2014.0]);

    let mut blocked = simple_floor();
    blocked.permanent_fire = vec![(27 * blocked.width() + 39) as f64].into();
    let before = blocked.clone();
    assert!(entrance::install_floor_one_entrance(&mut blocked).is_err());
    assert_eq!(blocked, before);

    let mut lower = simple_floor();
    lower.depth = 2.0;
    let before = lower.clone();
    entrance::install_floor_one_entrance(&mut lower).unwrap();
    assert_eq!(lower, before);
}

#[test]
fn identity_validation_does_not_reinterpret_legacy_stairs() {
    let mut f = simple_floor();
    assert!(entrance::validate(&f));
    assert_eq!(entrance::cell(&f), None);
    f.entrance = Field::Null;
    assert!(!entrance::validate(&f));
    f.entrance = Field::Missing;
    entrance::install_floor_one_entrance(&mut f).unwrap();
    assert!(entrance::validate(&f));
    for e in [
        entrance::Entrance { x: 38.0, y: 27.0 },
        entrance::Entrance { x: 39.5, y: 27.0 },
        entrance::Entrance { x: f64::NAN, y: 27.0 },
        entrance::Entrance { x: 39.0, y: 1000.0 },
    ] {
        let mut invalid = f.clone();
        invalid.entrance = e.into();
        assert!(!entrance::validate(&invalid));
        assert_eq!(entrance::cell(&invalid), None);
    }
    f.depth = 2.0;
    assert!(!entrance::validate(&f));
}

#[test]
fn normal_generation_all_domains_has_safe_connected_entry() {
    for domain in ["Dungeon", "Cave", "Meadow", "Forest", "Archipelago", "Mountain"] {
        for seed in (0..20).map(|i| 42 + i * 9973) {
            let f = generation::generate_floor(1, seed, domain, false, 79, Some(entrance::generation_anchor()))
                .unwrap_or_else(|e| panic!("{domain} seed {seed}: {e}"));
            let mut a = f.clone();
            let mut b = f;
            entrance::install_floor_one_entrance(&mut a)
                .unwrap_or_else(|e| panic!("{domain} seed {seed}: {e}"));
            entrance::install_floor_one_entrance(&mut b).unwrap();
            assert_eq!(a, b, "{domain} seed {seed}");
            assert_connected(&a);
            assert!(generation::validate_floor_generation(&a));
        }
    }
}

#[test]
fn revision31_start_save_replay_and_temporal_restore_keep_identity() {
    let run = startup::create_run_revision("bottom-center-entrance", "Unblessed", false, 31.).unwrap();
    assert_eq!(run.simulation_revision.as_ref(), Some(&31.0));
    let f = &run.floors[0];
    assert_connected(f);
    let arrival = entrance::arrival(f).unwrap();
    assert_eq!((run.player.x, run.player.y), (arrival.x as f64, arrival.y as f64));
    assert!(run.actors_at(1.0).iter().all(|a| a.x != run.player.x || a.y != run.player.y));
    assert!(run.floors.iter().skip(1).all(|f| f.entrance.is_missing()));
    storage::validate_current_revision(&run).unwrap();
    let loaded = storage::deserialize(&storage::serialize(&run).unwrap()).unwrap();
    assert_eq!(loaded.floors[0].entrance, f.entrance);
    let mut replay = ReplaySession::create(ReplayRecord::from_run(&run), true).unwrap();
    assert_eq!(replay.engine.run.floors[0].entrance, f.entrance);
    assert_eq!(replay.engine.run.player, run.player);
    replay.restart().unwrap();
    assert_eq!(replay.engine.run.floors[0].entrance, f.entrance);

    let mut r = run.clone();
    let mut history = History::default();
    r.player.conditions.insert("revertCharges".into(), 2.0);
    history.record_completed(Snapshot::capture(&r));
    Arc::make_mut(&mut r.floors[0]).entrance = Field::Missing;
    r.tick += 1.0;
    history.closing = Some(Snapshot::capture(&r));
    assert!(corpse_core::temporal::revert(&mut r, &mut history).unwrap());
    assert_eq!(r.floors[0].entrance, f.entrance);
    corpse_core::temporal::remember_future(&mut r, &history).unwrap();
    Arc::make_mut(&mut r.floors[0]).entrance = Field::Missing;
    assert!(corpse_core::temporal::restore_future(&mut r, &mut history).unwrap());
    assert_eq!(r.floors[0].entrance, f.entrance);

    let mut travel_run = run.clone();
    let down = travel_run.floors[0].tiles.iter().position(|t| t == "down").unwrap();
    travel_run.player.x = (down % travel_run.floors[0].width()) as f64;
    travel_run.player.y = (down / travel_run.floors[0].width()) as f64;
    assert!(corpse_core::travel::travel(&mut travel_run, 1.0, false, false).unwrap());
    assert_eq!(travel_run.floors[0].entrance, f.entrance);
    assert!(corpse_core::travel::travel(&mut travel_run, -1.0, false, false).unwrap());
    assert_eq!(travel_run.floors[0].entrance, f.entrance);

    let mut r = run.clone();
    let at = entrance::cell(&r.floors[0]).unwrap();
    r.player.x = (at % r.floors[0].width()) as f64;
    r.player.y = (at / r.floors[0].width()) as f64;
    let before = storage::serialize(&r).unwrap();
    assert!(!corpse_core::travel::travel(&mut r, -1.0, false, false).unwrap());
    assert_eq!(storage::serialize(&r).unwrap(), before, "the entrance is not a win/escape action");
}

#[test]
fn legacy_revisions_keep_original_entrance_geometry() {
    for revision in [29.0, 30.0] {
        let mut r = startup::new_run_shell_revision("entrance-legacy", "Unblessed", false, revision).unwrap();
        let seed = corpse_core::rng::seed_hash(&r.seed);
        let original = generation::generate_floor_full(1, seed, None).unwrap();
        let old = original.tiles.iter().position(|t| t == "up").unwrap();
        startup::ensure_floors(&mut r, 1).unwrap();
        assert!(r.floors[0].entrance.is_missing());
        assert_eq!(r.floors[0].tiles.iter().position(|t| t == "up"), Some(old));
        assert!(serde_json::to_value(&r.floors[0]).unwrap().get("entrance").is_none());
    }
}

#[test]
fn ordinary_required_cell_fixtures_are_reconciled_but_machines_are_not() {
    use corpse_core::{EnvironmentalState, FloorLightSourcesEntry, FloorDoorsEntry, FloorSetupsEntry, SetupBounds};
    let mut f = simple_floor();
    let (w, h) = (f.width(), f.height());
    for y in 1..h - 1 { for x in 1..w - 1 {
        let at = y * w + x;
        if f.tiles[at] == "wall" { f.tiles[at] = "floor".into(); }
    }}
    let target = 27 * w + 39;
    let north = target - w;
    let sources = vec![
        FloorLightSourcesEntry { x:39.0, y:27.0, kind:"torch".to_string().into(), power:75.0, falloff:10.0 },
        FloorLightSourcesEntry { x:39.0, y:26.0, kind:"brazier".to_string().into(), power:65.0, falloff:10.0 },
    ];
    f.light_sources = sources.into();
    f.permanent_fire = vec![north as f64].into();
    let mut fire = vec![0.0; w * h]; fire[north] = 1.0;
    f.environment = EnvironmentalState { fire, caustic:vec![0.0;w*h], breather:vec![0.0;w*h], paralysis:vec![0.0;w*h], ..Default::default() }.into();
    f.doors = indexmap::IndexMap::from([
        (north.to_string(), FloorDoorsEntry {open:false, ..Default::default()}),
        (500.to_string(), FloorDoorsEntry {open:false, ..Default::default()}),
    ]).into();
    entrance::install_floor_one_entrance(&mut f).unwrap();
    assert_connected(&f);
    assert!(!f.doors.as_ref().unwrap().contains_key(&north.to_string()));
    assert!(f.doors.as_ref().unwrap().contains_key("500"));
    assert_eq!(f.environment.as_ref().unwrap().fire[north], 0.0);
    let relocated = f.light_sources.as_ref().unwrap();
    assert!(relocated.iter().all(|s| ![target,north].contains(&(s.y as usize*w+s.x as usize))));
    let brazier = relocated.iter().find(|s| s.kind.as_ref().is_some_and(|k| k == "brazier")).unwrap();
    let at = brazier.y as usize * w + brazier.x as usize;
    assert_eq!(f.permanent_fire.as_ref().unwrap(), &vec![at as f64]);
    assert_eq!(f.environment.as_ref().unwrap().fire[at], 1.0);
    assert_eq!(f.environment.as_ref().unwrap().fire_kind.as_ref().unwrap()[at].as_deref(), Some("brazier"));

    let mut machine = simple_floor();
    machine.setups = vec![FloorSetupsEntry {id:1.0, x:39.0, y:27.0,
        bounds:SetupBounds {left:39.0,right:39.0,top:27.0,bottom:27.0}.into(), ..Default::default()}].into();
    machine.light_sources = vec![FloorLightSourcesEntry {x:39.0,y:27.0,kind:"brazier".to_string().into(),power:65.0,falloff:10.0}].into();
    let before = machine.clone();
    assert!(entrance::install_floor_one_entrance(&mut machine).is_err());
    assert_eq!(machine,before);
}

#[test]
fn populated_floor_one_keeps_the_north_arrival_safe_and_unoccupied() {
    for sample in 0..24 {
        let mut run = startup::new_run_shell(&format!("entrance-population-{sample}"), "Unblessed", false).unwrap();
        startup::ensure_floors(&mut run, 1).unwrap();
        let arrival = entrance::arrival(&run.floors[0])
            .unwrap_or_else(|| panic!("arrival hazard for population sample {sample}"));
        assert!(run.actors_at(1.0).iter().all(|a| {
            a.dead.value_or(false) || a.x != arrival.x as f64 || a.y != arrival.y as f64
        }), "occupied arrival for population sample {sample}");
        assert_connected(&run.floors[0]);
    }
}
