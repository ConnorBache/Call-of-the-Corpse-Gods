//! Ammo allocation is independent of R52's one-floor generation horizon.
//! These are generated ammo bundles, not the separate water-current arrows.
use corpse_core::{Run, inventory, startup, temporal::Engine};
use std::collections::BTreeSet;

#[test]
fn ammo_schedule_reserves_two_or_three_distinct_bundle_floors_in_all_eight_sets() {
    let mut counts = BTreeSet::new();
    for seed in 0..50 {
        let seed = format!("0-5-11:ammo-schedule-{seed}");
        for first in (1..=36).step_by(5) {
            let chosen = startup::arrow_bundle_floors(&seed, first);
            assert!(matches!(chosen.len(), 2 | 3), "{seed}: set {first}");
            assert!(chosen.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(chosen.iter().all(|depth| (first..first + 5).contains(depth)));
            counts.insert(chosen.len());
            // Querying every floor in reverse order must recover the same
            // complete set schedule, including the final 36..=40 set.
            for depth in (first..first + 5).rev() {
                assert_eq!(startup::arrow_bundle_floors(&seed, depth), chosen);
            }
        }
    }
    assert_eq!(counts, BTreeSet::from([2, 3]));
}

fn assert_generated_set(run: &Run, first: u32) {
    let mut bundle_floors = Vec::new();
    let mut total = 0.;
    let mut ids = BTreeSet::new();
    for depth in first..first + 5 {
        for ground in &*run.ground[&depth.to_string()] {
            if inventory::is_arrow(&ground.item) {
                assert_eq!(ground.item.quantity.as_ref(), Some(&5.));
                assert!(ids.insert(ground.item.id as u64), "duplicate ammo ID");
                bundle_floors.push(depth);
                total += inventory::quantity(&ground.item);
            }
        }
    }
    assert_eq!(bundle_floors, startup::arrow_bundle_floors(&run.seed, first));
    assert!(total == 10. || total == 15., "set {first}: {total} generated arrows");
}

fn reload_without_changes(engine: Engine, horizon: u32) -> Engine {
    let saved = engine.save().unwrap();
    let loaded = Engine::load(&saved).unwrap();
    // Avoid printing megabytes of world JSON if this invariant fails.
    assert!(loaded.save().unwrap() == saved, "save/load changed horizon {horizon}");
    loaded
}

#[test]
fn streamed_ammo_sets_survive_boundary_saves_and_match_uninterrupted_generation() {
    // One real R52 seed, two paths through ten floors. Share the initial two
    // floors to bound generation cost; only floors 3..=10 are generated twice.
    let initial = Engine::new(startup::create_avatar_run("0-5-11:square-current-startup").unwrap()).unwrap();
    assert_eq!(initial.run.floors.len(), 2);
    let mut uninterrupted = initial.clone();
    let mut streamed = initial;
    let initial_rng = streamed.run.rng;
    let initial_inventory = streamed.run.inventory.clone();
    startup::ensure_floors(&mut uninterrupted.run, 10).unwrap();

    // Exercise the arrival horizon without simulation turns: otherwise firing,
    // pickup or offscreen item movement would change the generated-ground total.
    for arrival in 2..=9 {
        let target = startup::arrival_generation_target(52., arrival);
        assert_eq!(target, arrival + 1);
        let prior_ground = streamed.run.ground.clone();
        startup::ensure_floors(&mut streamed.run, target).unwrap();
        assert_eq!(streamed.run.floors.len(), target as usize);
        assert_eq!(streamed.run.rng, initial_rng);
        assert_eq!(streamed.run.inventory, initial_inventory);
        for (depth, ground) in prior_ground {
            assert_eq!(&streamed.run.ground[&depth], &ground,
                "generating floor {target} changed existing ground on floor {depth}");
        }

        // Save on both sides of set completion (4/5 and 9/10), and on both
        // sides of crossing into the next set (5/6).
        if [4, 5, 6, 9, 10].contains(&target) {
            streamed = reload_without_changes(streamed, target);
            let saved = streamed.save().unwrap();
            startup::ensure_floors(&mut streamed.run, target).unwrap();
            startup::ensure_floors(&mut streamed.run, target - 1).unwrap();
            assert!(streamed.save().unwrap() == saved,
                "repeated/shallower request changed horizon {target}");
        }
        if target >= 5 {
            assert_generated_set(&streamed.run, 1);
        }
        if target == 10 {
            assert_generated_set(&streamed.run, 6);
        }
    }
    assert_generated_set(&uninterrupted.run, 1);
    assert_generated_set(&uninterrupted.run, 6);
    assert!(streamed.save().unwrap() == uninterrupted.save().unwrap(),
        "streamed generation with boundary reloads differs from uninterrupted generation");
}
