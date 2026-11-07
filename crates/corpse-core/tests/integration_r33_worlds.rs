//! Independent integration QA: actual seeded 40-floor startup, not isolated scaffolds.
use corpse_core::{generation, pocket_biomes, royal_honey, startup, water_blood};
use std::collections::HashSet;

fn audit_world(seed: &str) {
    let mut r = startup::new_run_shell_revision(seed, "Unblessed", false, 33.0).unwrap();
    let hash = corpse_core::rng::seed_hash(&r.seed);
    let expected_nest = pocket_biomes::nest_depth(hash);
    let mut ids = HashSet::from([r.player.id as u64]);
    let mut godbeasts = Vec::new();
    let mut nest_count = 0;
    let mut boss_count = 0;
    for depth in 1..=40 {
        startup::ensure_floors(&mut r, depth).unwrap_or_else(|e| panic!("{seed}, depth {depth}: {e}"));
        let f = r.floors.iter().find(|f| f.depth == depth as f64).unwrap();
        let reachable = generation::reachable_without_braziers(f, None);
        assert!(generation::transitions_accessible(f, &reachable), "{seed}, disconnected stairs at {depth}");
        assert_eq!(f.godbeast, depth < 40 && depth % 5 == 0, "{seed}, schedule {depth}");
        if f.godbeast {
            let event = f.godbeast_event.as_ref().expect("scheduled Godbeast has event");
            assert!(["Borge", "Mercuria", "Veridian", "Zillennia"].contains(&event.kind.as_str()));
            godbeasts.push(depth);
            if event.kind == "Borge" {
                assert_eq!(f.tiles.iter().enumerate().filter(|(i, _)| corpse_core::borge::solid_at(f, (i % f.width()) as f64, (i / f.width()) as f64)).count(), 25);
                let altar = event.altar.as_ref().unwrap();
                assert!(reachable.contains(&(altar.y as usize * f.width() + altar.x as usize)), "{seed}, unreachable Borge altar {depth}");
            }
        }
        assert!(pocket_biomes::validate(f), "{seed}, biome schema {depth}");
        if let Some(nest) = pocket_biomes::nest(f) {
            nest_count += 1;
            assert_eq!(depth, expected_nest);
            assert!(reachable.contains(&nest.honey_cell.unwrap()));
            assert!(reachable.contains(&nest.queen_cell.unwrap()));
            let population = r.actors_at(depth as f64);
            assert_eq!(population.iter().filter(|a| a.conditions.contains_key("nestResident")).count(), 7);
            assert_eq!(population.iter().filter(|a| a.creature_kind.as_ref().is_some_and(|k| k == "saidribee-queen")).count(), 1);
            assert!(population.iter().filter(|a| !a.conditions.contains_key("nestResident")).all(|a| !nest.cells.contains(&(a.y as usize * f.width() + a.x as usize))), "{seed}, ordinary population in reserved Nest");
            let honey: Vec<_> = r.ground[&depth.to_string()].iter().filter(|g| royal_honey::is_honey(&g.item)).collect();
            assert_eq!(honey.len(), 1);
            assert_eq!(honey[0].y as usize * f.width() + honey[0].x as usize, nest.honey_cell.unwrap());
        }
        assert_eq!(f.water_blood.as_ref().unwrap().len(), f.tiles.len());
        for source in [&f.kelpie_blood_sources, &f.blood_river_sources] {
            let values = source.as_ref().map(Vec::as_slice).unwrap_or(&[]);
            assert_eq!(values.len(), values.iter().map(|x| *x as usize).collect::<HashSet<_>>().len());
            for &at in values {
                let at = at as usize;
                assert!(water_blood::water_cell(f, at));
                assert!(f.water_blood.as_ref().unwrap()[at] >= 3.0);
            }
        }
        for a in r.actors_at(depth as f64) {
            assert!(ids.insert(a.id as u64), "{seed}, duplicate actor ID {} at {depth}", a.id);
            if corpse_core::finale::is_boss(a) { boss_count += 1; assert_eq!(depth, 40); }
            if a.creature_kind.as_ref().is_some_and(|k| k == "saidribee-queen") { assert_eq!(depth, expected_nest); }
        }
        eprintln!("QA_WORLD seed={seed} floor={depth} domain={} actors={} ground={} godbeast={} nest={}", f.domain, r.actors_at(depth as f64).len(), r.ground[&depth.to_string()].len(), f.godbeast, pocket_biomes::nest(f).is_some());
    }
    assert_eq!(godbeasts, vec![5, 10, 15, 20, 25, 30, 35]);
    assert_eq!(nest_count, 1);
    assert_eq!(boss_count, 1);
    assert_eq!(r.floors.len(), 40);
    let encoded = corpse_core::storage::serialize(&r).unwrap();
    let decoded = corpse_core::storage::deserialize(&encoded).unwrap();
    assert_eq!(decoded.floors, r.floors);
    assert_eq!(decoded.actors, r.actors);
    assert_eq!(decoded.ground, r.ground);
    r.depth = expected_nest as f64;
    let at = pocket_biomes::nest(r.floor().unwrap()).unwrap().honey_cell.unwrap();
    let width = r.floor().unwrap().width();
    r.player.x = (at % width) as f64; r.player.y = (at / width) as f64;
    export_preview(&r, &format!("nest-{hash}"));
}
#[test] fn actual_world_40_floors_seed_one() { audit_world("0-5-11:integration-r33-1"); }
#[test] fn actual_world_40_floors_seed_forty_two() { audit_world("0-5-11:integration-r33-42"); }
#[test] fn actual_world_40_floors_seed_legacy_regression() { audit_world("0-5-11:nest-preview"); }

fn export_preview(r: &corpse_core::Run, name: &str) {
    let Ok(dir) = std::env::var("R33_UI_FIXTURE_DIR") else { return; };
    let mut preview = r.clone();
    preview.player.conditions.insert("allSeeingReveal".into(), 1.0);
    preview.player.conditions.insert("allSeeingDepth".into(), preview.depth);
    let f = preview.floor_mut().unwrap();
    f.seen.fill(true);
    f.lights.fill(100.0);
    f.remembered_tiles = f.tiles.clone().into();
    corpse_core::engine::remember(&mut preview);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(std::path::Path::new(&dir).join(format!("{name}.json")), corpse_core::storage::serialize(&preview).unwrap()).unwrap();
}

#[test]
fn borge_exact_skin_heart_and_honey_keep_independent_provenance_and_optional_previews() {
    use corpse_core::{Field, Floor, Command, borge, engine, population, inventory};
    use std::sync::Arc;
    let mut r = startup::new_run_shell_revision("0-5-11:integration-borge-ui", "Unblessed", false, 33.0).unwrap();
    let (w, h) = (79, 29);
    let mut f = Floor { width: (w as f64).into(), height: (h as f64).into(), depth: 1.0,
        domain: "Dungeon".into(), godbeast: true, tiles: vec!["wall".into(); w*h],
        lights: vec![100.0; w*h], seen: vec![true; w*h], ..Default::default() };
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into();
    let event = borge::reserve_room(&mut f.tiles, &[]).unwrap();
    let (cx, cy) = (event.x, event.y);
    f.godbeast_event = event.into(); corpse_core::environment::initialize(&mut f).unwrap();
    r.floors = vec![Arc::new(f)]; r.depth = 1.0; r.player.x = cx-3.0; r.player.y = cy;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear(); r.player.ability_order = Field::Missing;
    r.actors.clear(); r.ground.clear(); r.ground.insert("1".into(), Arc::new(vec![])); r.inventory.clear(); r.next_id = 100.0;
    let mut skin = population::create_actor_revision("skin-walker", 4.0, cx-4.0, cy-3.0, false, 33.0).unwrap();
    skin.state = "wandering".into(); r.actors_at_mut(1.0).push(skin);
    r.inventory.push(royal_honey::item(90.0));
    let rng = r.rng;
    assert!(borge::interact(&mut r, cx-2.0, cy).unwrap());
    assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().sacrifice_actor_id.as_ref(), Some(&4.0));
    assert_eq!(r.rng, rng);
    export_preview(&r, "borge-mark");
    r.player.hp = 1.0; r.player.energy = 1.0;
    assert_eq!(inventory::command(&mut r, &Command::Use { id: 90.0 }).unwrap(), Some(true));
    assert!(!r.inventory.iter().any(royal_honey::is_honey));
    assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().sacrifice_actor_id.as_ref(), Some(&4.0));
    engine::actor_mut(&mut r, 4.0).unwrap().hp = 0.0;
    let player_id = r.player.id;
    engine::resolve_death(&mut r, 4.0, Some(player_id), None, true).unwrap();
    let i = r.ground["1"].iter().position(|g| g.item.source_actor_id.as_ref() == Some(&4.0)).unwrap();
    let heart = r.ground_at_mut(1.0).remove(i).item; r.inventory.push(heart);
    let altar = r.floor().unwrap().godbeast_event.as_ref().unwrap().altar.as_ref().unwrap().clone();
    r.player.x = altar.x; r.player.y = altar.y+1.0;
    export_preview(&r, "borge-heart");
    let stats = r.player.stats.clone(); let exp = r.exp;
    assert!(borge::interact(&mut r, altar.x, altar.y).unwrap());
    assert_eq!(r.player.stats, stats); assert_eq!(r.exp, exp);
    assert_eq!(r.inventory.len(), 1); assert!(borge::is_shard(&r.inventory[0]));
    export_preview(&r, "borge-shard");
}

#[test]
fn actual_world_with_all_water_nest_window_preserves_connected_floors() {
    let seed = (0..10_000).map(|i| format!("0-5-11:integration-r33-water-{i}")).find(|seed| {
        let plan = generation::domain_plan_revision(corpse_core::rng::seed_hash(seed), 33.0);
        [28usize,29,31,32,33,34].iter().all(|depth| plan[(depth-1)/5].domain == "Archipelago")
    }).expect("deterministic search must exercise an all-Archipelago Nest window");
    audit_world(&seed);
}
