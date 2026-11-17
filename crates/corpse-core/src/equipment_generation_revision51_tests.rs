use super::*;
use std::collections::{BTreeSet, HashSet};

fn shell(seed: &str) -> Run {
    crate::startup::new_run_shell_revision(seed, "Unblessed", false, 51.).unwrap()
}
fn stream(seed: &str, depth: u32) -> GenerationRng {
    crate::population::population_random(&format!("{seed}:floor:{depth}:loot"))
}
fn assert_categories(seed: &str, depth: u32, items: &[Item]) {
    let primary = revision51::primary_kind(seed, depth).unwrap();
    assert!((5..=7).contains(&items.len()), "{seed} floor {depth}");
    for kind in revision51::KINDS {
        let count = items.iter().filter(|item| item.kind == kind).count();
        if kind == primary { assert_eq!(count, 3, "{seed} floor {depth}: {kind}"); }
        else { assert!((1..=2).contains(&count), "{seed} floor {depth}: {kind}={count}"); }
    }
    assert!(items.iter().all(|item| revision51::KINDS.contains(&item.kind.as_str())));
    assert_eq!(items.iter().map(|item| item.id as u64).collect::<HashSet<_>>().len(), items.len());
    for item in items {
        crate::equipment::validate(item).unwrap();
        if item.kind == "amulet" {
            assert_eq!(item.amulet_state.as_ref().unwrap().instance_id, item.id as u64);
            assert_eq!(item.amulet_state.as_ref().unwrap().energy, 50.);
        }
    }
}

#[test]
fn primary_schedule_has_two_potion_floors_and_all_twelve_seeded_permutations() {
    let mut permutations = BTreeSet::new();
    for seed in 0..512 {
        let seed = format!("0-5-11:primary-{seed}");
        let schedule = [1, 2, 3, 4].map(|depth| revision51::primary_kind(&seed, depth).unwrap());
        assert_eq!(schedule.iter().filter(|&&kind| kind == "potion").count(), 2);
        assert_eq!(schedule.iter().filter(|&&kind| kind == "ring").count(), 1);
        assert_eq!(schedule.iter().filter(|&&kind| kind == "amulet").count(), 1);
        for depth in [4, 2, 1, 3] {
            assert_eq!(revision51::primary_kind(&seed, depth), Some(schedule[depth as usize - 1]));
        }
        permutations.insert(schedule);
    }
    assert_eq!(permutations.len(), 12);
    for depth in [0, 5, 6, 40, u32::MAX] { assert_eq!(revision51::primary_kind("seed", depth), None); }
}

#[test]
fn primary_drops_replace_slots_have_unique_identities_and_exact_reproducible_counts() {
    let mut minor_pairs = BTreeSet::new();
    let mut repeated_subtype = false;
    for seed in 0..512 {
        let mut run = shell(&format!("0-5-11:primary-{seed}"));
        let simulation_rng = run.rng;
        let mut all_ids = HashSet::new();
        for depth in 1..=4 {
            let mut mirror = run.clone();
            let mut rng = stream(&run.seed, depth); let mut mirrored_rng = rng;
            let start = run.next_id;
            // Neither small nor large historical allocations add to/trim the new schedule.
            let items = generate_floor_loot_for_run(&mut run, depth, &mut rng, &[], 2).unwrap();
            let mirrored = generate_floor_loot_for_run(&mut mirror, depth, &mut mirrored_rng, &[], 20).unwrap();
            assert_eq!(items, mirrored); assert_eq!(rng.state, mirrored_rng.state);
            assert_eq!(run.next_id, mirror.next_id); assert_eq!(run.next_id, start + items.len() as f64);
            assert_categories(&run.seed, depth, &items);
            let primary = revision51::primary_kind(&run.seed, depth).unwrap();
            minor_pairs.insert(revision51::KINDS.into_iter().filter(|kind| *kind != primary)
                .map(|kind| items.iter().filter(|item| item.kind == kind).count()).collect::<Vec<_>>());
            let mut types = HashSet::new();
            for item in &items {
                assert!(all_ids.insert(item.id as u64));
                repeated_subtype |= !types.insert((item.kind.clone(), item.effect.as_ref().unwrap().clone()));
            }
            assert_eq!(run.composite_bow_generated, false.into());
            assert_eq!(run.rng, simulation_rng);
        }
    }
    assert_eq!(minor_pairs.len(), 4);
    assert!(repeated_subtype, "Do not silently invent a no-duplicate-subtype rule");
    let mut run = shell("invalid-primary-count"); let before = run.clone();
    let mut rng = GenerationRng::new(123); let rng_before = rng.state;
    assert!(generate_floor_loot_for_run(&mut run, 1, &mut rng, &[], 21).is_err());
    assert_eq!(run, before); assert_eq!(rng.state, rng_before);
}

#[test]
fn primary_policy_preserves_later_floor_algorithm_for_equal_incoming_bow_budget() {
    for generated in [false, true] {
        for seed in 0..64 {
            let mut run = shell(&format!("0-5-11:primary-later-{seed}"));
            for depth in 5..=40 {
                for count in [2, 7, 20] {
                    run.composite_bow_generated = generated.into();
                    let mut rng = stream(&run.seed, depth); let mut expected_rng = rng;
                    let mut expected_id = run.next_id; let mut expected_budget = generated;
                    let expected = generate_floor_loot_with_budget(depth, &mut expected_rng,
                        &mut expected_id, &[], count, 51., Some(&mut expected_budget),true).unwrap();
                    let actual = generate_floor_loot_for_run(&mut run, depth, &mut rng, &[], count).unwrap();
                    assert_eq!(actual, expected); assert_eq!(run.next_id, expected_id);
                    assert_eq!(rng.state, expected_rng.state);
                    assert_eq!(run.composite_bow_generated, expected_budget.into());
                }
            }
        }
    }
}

#[test]
fn primary_policy_does_not_change_legacy_floor_items_ids_or_rng() {
    for revision in [29., 43., 46., 50.] {
        for seed in 0..16 {
            let mut run = shell(&format!("0-5-11:primary-legacy-{seed}"));
            run.simulation_revision = revision.into(); run.composite_bow_generated = Field::Missing;
            for depth in [1, 2, 3, 4, 5, 20, 40] {
                for count in [2, 7, 20] {
                    let mut rng = stream(&run.seed, depth); let mut expected_rng = rng;
                    let mut expected_id = run.next_id;
                    let expected = generate_floor_loot_revision(depth, &mut expected_rng,
                        &mut expected_id, &[], count, revision).unwrap();
                    assert_eq!(generate_floor_loot_for_run(&mut run, depth, &mut rng, &[], count).unwrap(), expected);
                    assert_eq!(run.next_id, expected_id); assert_eq!(rng.state, expected_rng.state);
                    assert!(run.composite_bow_generated.is_missing());
                }
            }
        }
    }
}

fn ordinary(run: &Run, depth: u32) -> Vec<Item> {
    run.ground[&depth.to_string()].iter()
        .filter(|g| crate::debug_floor::ordinary_ground(run, depth, g)
            && revision51::KINDS.contains(&g.item.kind.as_str()))
        .map(|g| g.item.clone()).collect()
}

#[test]
fn primary_generated_worlds_place_exact_counts_safely_and_preserve_normal_accounting() {
    for seed in 0..8 {
        let mut run = shell(&format!("0-5-11:primary-world-{seed}"));
        let rng = run.rng;
        crate::startup::ensure_floors(&mut run, 6).unwrap();
        assert_eq!(run.rng, rng);
        let mut ids = HashSet::new();
        for floor in &run.floors {
            let depth = floor.depth as u32;
            let mut cells = HashSet::new();
            for g in &*run.ground[&depth.to_string()] {
                assert!(ids.insert(g.item.id as u64));
                let at = g.y as usize * floor.width() + g.x as usize;
                assert!(cells.insert(at));
                assert!(["floor", "grass"].contains(&floor.tiles[at].as_str()));
            }
            assert_eq!(run.ground[&depth.to_string()].iter().filter(|g| g.item.kind == "ammo").count(),
                usize::from(crate::startup::arrow_bundle_floors(&run.seed, depth).contains(&depth)));
            let details = floor.generation.as_ref().unwrap().stages.iter()
                .find(|stage| stage.id == "items").unwrap().details.as_ref().unwrap();
            let expected_budget = crate::generation::retained_loot_allocation(depth, &run.floors,
                details["retainedHeartSlots"], details["carriedItemSlots"]).unwrap();
            for (key, expected) in expected_budget { assert_eq!(details[&key], expected); }
            let items = ordinary(&run, depth);
            if depth <= 4 {
                assert_categories(&run.seed, depth, &items);
                assert_eq!(details["ordinaryLootGenerated"], items.len() as f64);
                for (kind, field) in [("potion", "ordinaryPotionDrops"), ("ring", "ordinaryRingDrops"), ("amulet", "ordinaryAmuletDrops")] {
                    assert_eq!(details[field], items.iter().filter(|item| item.kind == kind).count() as f64);
                }
            } else {
                assert!(!details.contains_key("ordinaryLootGenerated"));
                let ordinary_count = run.ground[&depth.to_string()].iter()
                    .filter(|g| crate::debug_floor::ordinary_ground(&run, depth, g) && g.item.kind != "ammo").count();
                assert_eq!(ordinary_count as f64, details["ordinaryLoot"]);
            }
        }
    }
}

#[test]
fn primary_generated_saves_seed_replays_and_debug_restarts_keep_the_same_items() {
    let seed = "0-5-11:primary-persistence";
    let run = crate::startup::create_run_revision(seed, "Unblessed", false, 51.).unwrap();
    let raw = crate::storage::serialize(&run).unwrap();
    let loaded = crate::storage::deserialize_compatible(&raw).unwrap();
    for depth in 1..=4 { assert_eq!(ordinary(&run, depth), ordinary(&loaded, depth)); }
    let snapshot = crate::temporal::Snapshot::capture(&run);
    let mut restored = loaded;
    restored.ground_at_mut(1.).clear(); snapshot.restore(&mut restored).unwrap();
    assert_eq!(restored.ground, run.ground);
    let future = crate::future_anchor::FutureAnchor::capture41(&run).unwrap().prepare_restore().unwrap();
    assert_eq!(future.ground, run.ground);
    let record = crate::replay::ReplayRecord::try_from_run(&run).unwrap();
    let mut replay = crate::replay::ReplaySession::create(record, true).unwrap();
    for _ in 0..2 {
        assert_eq!(replay.engine.run.ground, run.ground);
        assert_eq!(replay.engine.run.next_id, run.next_id);
        replay.restart().unwrap();
    }
    let debug = crate::debug_floor::create_run_revision(seed, "Unblessed", 5, 51.).unwrap();
    let restarted = crate::debug_floor::create_run_revision(seed, "Unblessed", 5, 51.).unwrap();
    assert_eq!(crate::debug_floor::state(&debug), crate::debug_floor::state(&restarted));
    let stash = &crate::debug_floor::state(&debug).unwrap().stash;
    for depth in 1..=4 {
        for item in ordinary(&run, depth) {
            let matches: Vec<_> = stash.iter().filter(|reward| reward.depth == depth && reward.item.id == item.id).collect();
            assert_eq!(matches.len(), 1); assert_eq!(matches[0].item, item);
        }
    }
}
