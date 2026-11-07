use corpse_core::{Command, Field, Run, playtest, startup, storage, replay::{ReplayRecord, ReplaySession}, temporal::{Engine, Snapshot}};
fn run(floor: u32) -> Run { playtest::create_run("0-5-11:floor-playtest-regression", "Avatar", floor).unwrap() }
#[test]
fn input_validation_has_no_silent_clamping() {
    for bad in ["", "0", "41", "-1", "1.5", "x", "999999999999999999999"] { assert!(playtest::parse_floor(bad).is_err(), "{bad}"); }
    assert_eq!(playtest::parse_floor(" 40 ").unwrap(), 40);
    assert!(playtest::create_run("", "Avatar", 1).is_err());
    assert!(playtest::create_run("a\nb", "Avatar", 1).is_err());
    assert!(playtest::create_run("valid", "Avatar", 0).is_err());
    assert!(playtest::create_run("valid", "Avatar", 41).is_err());
}
#[test]
fn floor_one_has_zero_prior_budget_and_exact_opening() {
    let r = run(1);
    let p = r.playtest.as_ref().unwrap();
    assert_eq!((p.prior_hearts, p.prior_blessings), (0, 0.));
    assert_eq!(r.level, 1.);
    assert_eq!(r.inventory.len(), 3);
    assert!(p.sources.is_empty());
    assert!(r.log[0].ends_with("You begin to adventure down to end the corruption of the Flesh."));
    assert_eq!(r.tick, 0.);
    assert!(r.floor().unwrap().entrance.as_ref().is_some());
    Engine::new(r).unwrap();
}
#[test]
fn identical_seed_floor_and_mode_reproduce_complete_state() {
    let a = run(6); let b = run(6);
    assert_eq!(storage::serialize(&a).unwrap(), storage::serialize(&b).unwrap());
    assert_eq!(a.depth, 6.);
    assert!(a.offers.is_empty());
    assert_eq!(a.player.hp, a.player.max_hp);
    assert_eq!(a.player.energy, corpse_core::combat::effective_stats(&a, &a.player).max_energy);
    assert!(corpse_core::inventory::occupancy(&a) <= corpse_core::inventory::carrying_capacity(&a).unwrap());
    assert!(a.playtest.as_ref().unwrap().sources.iter().all(|s| s.depth < 6));
    let c = playtest::create_run("0-5-11:another-playtest-seed", "Avatar", 6).unwrap();
    assert_ne!(a.inventory, c.inventory);
}
#[test]
fn full_collection_matches_actual_prior_generated_ordinary_hearts() {
    let seed = "0-5-11:floor-playtest-regression";
    let normal = startup::create_run(seed, "Avatar", false).unwrap();
    let r = run(4);
    let mut hearts = 0; let mut blessings = 0.;
    for depth in 1..4 {
        for a in normal.actors.get(&depth.to_string()).unwrap().iter() {
            if a.heartless.value_or(false) || a.dead.value_or(false) || a.creature_kind.as_ref().is_some_and(|k| k == "zillennia") || a.conditions.contains_key("godbeastDepth") { continue; }
            let heart = corpse_core::inventory::heart_for(a, 0.).unwrap();
            hearts += 1; blessings += heart.exp.value_or(0.);
        }
    }
    let p = r.playtest.as_ref().unwrap();
    assert_eq!(p.prior_hearts, hearts);
    assert_eq!(p.prior_blessings, blessings);
    assert_eq!(r.run_stats.as_ref().unwrap().blessings_received, blessings);
    assert!(r.actors["4"].iter().any(|a| !a.heartless.value_or(false)));
    for depth in 1..4 {
        assert!(r.actors[&depth.to_string()].iter().all(|a| a.heartless.value_or(false) || a.creature_kind.as_ref().is_some_and(|k| k == "zillennia") || a.conditions.contains_key("godbeastDepth")));
    }
    assert!(normal.playtest.is_missing());
}
#[test]
fn floor_forty_is_valid_without_target_rewards() {
    let r = run(40);
    assert_eq!(r.depth, 40.);
    assert_eq!(r.floors.len(), 40);
    assert!(r.playtest.as_ref().unwrap().sources.iter().all(|s| s.depth < 40));
    assert!(!r.actors["40"].is_empty());
    assert!(r.status == "playing");
    assert!(r.player.hp > 0.);
    Engine::new(r).unwrap();
}
#[test]
fn save_replay_restart_and_snapshot_preserve_profile() {
    let r = run(4);
    let profile = r.playtest.clone();
    let mut engine = Engine::new(r).unwrap();
    engine.dispatch(&Command::Wait).unwrap();
    let saved = engine.save().unwrap();
    let mut loaded = Engine::load(&saved).unwrap();
    assert_eq!(loaded.run.playtest, profile);
    let record = ReplayRecord::from_run(&loaded.run);
    let mut replay = ReplaySession::create(record, true).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), saved);
    replay.restart().unwrap();
    assert_eq!(replay.engine.run.depth, 4.);
    assert_eq!(replay.engine.run.playtest, profile);
    let snapshot = Snapshot::capture(&loaded.run);
    loaded.run.playtest = Field::Missing;
    snapshot.restore(&mut loaded.run).unwrap();
    assert_eq!(loaded.run.playtest, profile);
}
#[test]
fn playtest_cannot_enter_normal_high_scores_even_if_imported_as_genuine() {
    let mut r = run(3);
    r.status = "won".into();
    corpse_core::profile::ensure_run_id(&mut r, || "test-unranked".into()).unwrap();
    let profile = corpse_core::profile::Profile::default();
    let (after, archived) = corpse_core::profile::archive_run_record(&profile, &r, false, None).unwrap();
    assert!(!archived); assert_eq!(after, profile);
    assert_eq!(corpse_core::profile::summarize_run(&r).score, None);
}
#[test]
fn all_vessels_and_both_temporal_abilities_retain_synthetic_origin() {
    for mode in ["Unblessed", "Blessed", "Avatar"] {
        let r = playtest::create_run("3-5-11:temporal-playtest", mode, 2).unwrap();
        assert!(r.offers.is_empty());
        Engine::new(r).unwrap();
    }
    let mut r = run(4);
    for id in ["numo-revert", "morae-see-the-future"] {
        if !r.player.abilities.iter().any(|s| s == id) { r.player.abilities.push(id.into()); }
        corpse_core::progression::initialize_ability_state(&mut r.player, id, r.tick).unwrap();
    }
    r.player.conditions.insert("revertCharges".into(), 2.);
    let origin = r.playtest.clone();
    let mut engine = Engine::new(r).unwrap();
    engine.dispatch(&Command::Wait).unwrap();
    let ability = |id: &str| Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing };
    engine.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(engine.run.tick, 0.);
    assert_eq!(engine.run.playtest, origin);
    engine.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(engine.run.future.is_some());
    engine.dispatch(&Command::Wait).unwrap();
    let mut loaded = Engine::load(&engine.save().unwrap()).unwrap();
    loaded.dispatch(&ability("morae-see-the-future")).unwrap();
    assert_eq!(loaded.run.tick, 0.);
    assert!(loaded.run.future.is_none());
    assert_eq!(loaded.run.playtest, origin);
}
