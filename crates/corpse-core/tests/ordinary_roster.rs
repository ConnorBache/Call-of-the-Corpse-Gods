use corpse_core::{ordinary_roster as roster, population as p, startup, storage};
use std::collections::BTreeSet;
#[test]
fn widened_windows_preserve_recipe_rules() {
    assert_eq!(roster::species().len(), 55);
    for depth in 1..=40 {
        for water in [false, true] {
            let old = p::budgeted_candidates(depth, water).unwrap();
            let new = roster::candidates(depth, water).unwrap();
            for recipe in old {
                assert!(
                    new.iter().any(|r| r.kinds == recipe.kinds),
                    "lost {depth} {:?}",
                    recipe.kinds
                );
            }
            for recipe in new {
                assert!(recipe.weight > 0.0 && recipe.weight <= 1.0);
                assert_eq!(recipe.blessings, p::heart_blessings(&recipe.kinds).unwrap());
                assert!(recipe.kinds.iter().filter(|k| *k == "banshee").count() <= 1);
                assert!([1, 2, 3, 5].contains(&recipe.kinds.len()));
                if recipe.kinds.len() == 2 {
                    assert_eq!(
                        p::catalog().pairs.get(&recipe.kinds[0]),
                        Some(&recipe.kinds[1])
                    );
                }
                if depth == 1 {
                    assert!(["low", "medium"].contains(&recipe.danger.as_str()));
                    assert!(recipe.kinds.len() <= 3);
                }
                for kind in &recipe.kinds {
                    let leader = p::catalog()
                        .pairs
                        .iter()
                        .find(|(_, f)| *f == kind)
                        .map_or(kind, |(l, _)| l);
                    let s = roster::species().iter().find(|s| &s.id == leader).unwrap();
                    let (first, last) = s.widened_bounds();
                    assert!((first..=last).contains(&depth));
                    if !water {
                        assert_ne!(p::catalog().aquatic.get(kind), Some(&true));
                    }
                }
            }
        }
    }
    for s in roster::species() {
        let (first, last) = s.widened_bounds();
        assert_eq!(first, s.first.saturating_sub(2).max(1));
        assert_eq!(last, (s.last + 2).min(40));
        // Floor-one danger restrictions intentionally still apply.
        for depth in first.max(2)..=last {
            assert!(
                roster::candidates(depth, true)
                    .unwrap()
                    .iter()
                    .any(|r| r.kinds.contains(&s.id)),
                "{} at {depth}",
                s.id
            );
        }
    }
    assert!(roster::candidates(0, true).is_err());
    assert!(roster::candidates(41, true).is_err());
}
#[test]
fn deterministic_distribution_sample_all_depths() {
    println!("depth,old_eligible,new_eligible,sampled_kinds,plans");
    for depth in 1..=40 {
        let mut observed = BTreeSet::new();
        for water in [false, true] {
            for sample in 0..4 {
                let seed = format!("wide-roster-{sample}");
                let budget = corpse_core::generation::ordinary_blessing_budget(
                    corpse_core::generation::BudgetSeed::Text(&seed),
                    depth,
                )
                .unwrap() as i32;
                let plan =
                    p::plan_budgeted_hordes_revision(&seed, depth, budget, 0, water, &[], 30.0)
                        .unwrap();
                let again =
                    p::plan_budgeted_hordes_revision(&seed, depth, budget, 0, water, &[], 30.0)
                        .unwrap();
                assert_eq!(
                    serde_json::to_value(&plan).unwrap(),
                    serde_json::to_value(again).unwrap()
                );
                let kinds: Vec<_> = plan.hordes.iter().flat_map(|h| h.kinds.clone()).collect();
                assert_eq!(p::heart_blessings(&kinds).unwrap(), budget);
                assert!((1.5..=2.5).contains(&(kinds.len() as f64 / plan.hordes.len() as f64)));
                assert!(kinds.iter().filter(|k| *k == "banshee").count() <= 1);
                assert!(
                    plan.hordes
                        .iter()
                        .filter(|h| p::heart_blessings(&h.kinds).unwrap() > 0)
                        .count()
                        >= plan.hordes.len().div_ceil(2).max(2)
                );
                observed.extend(kinds);
            }
        }
        let count = |c: &[p::Candidate]| {
            c.iter()
                .flat_map(|r| r.kinds.iter())
                .collect::<BTreeSet<_>>()
                .len()
        };
        println!(
            "{depth},{},{},{},8",
            count(p::budgeted_candidates(depth, true).unwrap()),
            count(roster::candidates(depth, true).unwrap()),
            observed.len()
        );
    }
}
#[test]
fn save_lazy_generation_and_replay_keep_revision() {
    use corpse_core::replay::{ReplayRecord, ReplaySession};
    for revision in [29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0] {
        let mut run =
            startup::create_run_revision("widened-revision-check", "Unblessed", false, revision)
                .unwrap();
        storage::validate_current_revision(&run).unwrap();
        let saved = serde_json::to_string(&run).unwrap();
        let mut loaded = serde_json::from_str(&saved).unwrap();
        startup::ensure_floors(&mut run, 11).unwrap();
        startup::ensure_floors(&mut loaded, 11).unwrap();
        assert!(storage::canonical_equal(
            &serde_json::to_value(&run).unwrap(),
            &serde_json::to_value(&loaded).unwrap()
        ));
        let mut replay = ReplaySession::create(ReplayRecord::from_run(&run), true).unwrap();
        let before = replay.engine.save().unwrap();
        replay.restart().unwrap();
        assert_eq!(replay.engine.save().unwrap(), before);
        assert_eq!(
            replay.engine.run.simulation_revision.as_ref(),
            Some(&revision)
        );
    }
    assert_eq!(
        startup::new_run_shell("new-default", "Unblessed", false)
            .unwrap()
            .simulation_revision
            .as_ref(),
        Some(&39.0)
    );
}
#[test]
fn ichor_bands_expand_with_same_uniform_draw_and_templates() {
    use corpse_core::ichor::{eligible_fish_kinds, make_fish_revision};
    let kinds: BTreeSet<_> = (1..=40)
        .flat_map(|d| eligible_fish_kinds(d, 29.0))
        .collect();
    for kind in kinds {
        let old: Vec<_> = (1..=40)
            .filter(|&d| eligible_fish_kinds(d, 29.0).contains(&kind))
            .collect();
        let first = old[0].saturating_sub(2).max(1);
        let last = (old.last().unwrap() + 2).min(40);
        for depth in 1..=40 {
            assert_eq!(
                eligible_fish_kinds(depth, 30.0).contains(&kind),
                (first..=last).contains(&depth)
            );
        }
    }
    for revision in [29.0, 30.0] {
        for depth in 1..=40 {
            let pool = eligible_fish_kinds(depth, revision);
            for (i, kind) in pool.iter().enumerate() {
                let actor = make_fish_revision(
                    1.0,
                    2.0,
                    3.0,
                    depth as f64,
                    (i as f64 + 0.5) / pool.len() as f64,
                    revision,
                );
                assert_eq!(
                    actor.creature_kind.as_ref().map(String::as_str),
                    Some(*kind)
                );
            }
        }
    }
}

#[test]
fn shipped_manual_matches_runtime_eligibility() {
    let manual: serde_json::Value =
        serde_json::from_str(include_str!("../../corpse-desktop/assets/manual.json")).unwrap();
    for depth in 1..=40 {
        let expected: BTreeSet<_> = roster::candidates_revision(depth, true, 33.0)
            .unwrap()
            .iter()
            .flat_map(|r| r.kinds.iter().map(String::as_str))
            .collect();
        let shown: BTreeSet<_> = manual["depths"][depth as usize - 1]["ordinary"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        assert_eq!(shown, expected, "manual floor {depth}");
        for entry in manual["entries"].as_array().unwrap() {
            let id = entry["id"].as_str().unwrap();
            let lists_depth = entry["ordinaryDepths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d.as_u64() == Some(depth as u64));
            assert_eq!(
                lists_depth,
                expected.contains(id),
                "manual {id} floor {depth}"
            );
        }
        for kind in corpse_core::ichor::eligible_fish_kinds(depth, 33.0) {
            let entry = manual["entries"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == kind)
                .unwrap();
            let listed = entry["conditionalDepths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d.as_u64() == Some(depth as u64));
            assert_eq!(listed, depth >= 4);
        }
    }
}

#[test]
fn widened_population_places_on_all_generated_depths() {
    let mut run = startup::new_run_shell("wide-full-dungeon", "Unblessed", false).unwrap();
    startup::ensure_floors(&mut run, 40).unwrap();
    assert_eq!(run.floors.len(), 40);
    for floor in &run.floors {
        let encounters = floor.encounters.as_ref().unwrap();
        let ordinary: Vec<_> = encounters
            .iter()
            .filter(|e| e.kind.as_ref().is_some_and(|k| k == "horde"))
            .collect();
        assert!(ordinary.len() >= 2, "depth {}", floor.depth);
    }
}
