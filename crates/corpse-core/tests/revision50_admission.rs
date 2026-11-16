//! Coordinated R50 release gates. Authored acceptance checks are not Windows QA.
use corpse_core::{
    Command, Field, Item, Run, abilities, equipment, progression,
    replay::{ReplayRecord, ReplaySession}, revision50_ability_energy as energy,
    revision50_amulet_effects::{self as amulet_effects, Effect}, startup, storage,
    temporal::Engine,
};
use std::{collections::BTreeSet, sync::Arc};

fn fixture(revision: f64) -> Run {
    let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = revision.into();
    run.history.clear(); run.haste_snapshot = Field::Missing;
    run
}
fn ability(id: &str) -> Command {
    Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing }
}
fn checkpoint(engine: &mut Engine) {
    let saved = engine.save().unwrap();
    *engine = Engine::load(&saved).unwrap();
    assert_eq!(engine.save().unwrap(), saved);
    assert_eq!(engine.run.simulation_revision, Field::Value(50.));
    assert!(engine.history.frames.is_empty());
    assert!(engine.history.pending_haste.is_none());
}
fn replay_exact(initial: Option<Run>, engine: &Engine) {
    let expected = engine.save().unwrap();
    let record = ReplayRecord::try_from_run(&engine.run).unwrap();
    let mut replay = match initial {
        Some(initial) => ReplaySession::from_initial(initial, record).unwrap(),
        None => ReplaySession::create(record, true).unwrap(),
    };
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    replay.restart().unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
}

#[test]
fn revision50_all_twelve_starter_pools_keep_draws_and_initialize_every_selected_power() {
    for god in 0..12 {
        let pool = progression::starter_abilities_for_revision(god, 50.).unwrap();
        let expected: BTreeSet<_> = pool.iter().cloned().collect();
        let mut observed = BTreeSet::new();
        // A bounded deterministic seed corpus exercises both entries where a god
        // has two. No frequency/balance or exhaustive RNG claim is made.
        for suffix in 0..256 {
            let seed = format!("{god}-{}-{}:r50-starter-{suffix}", (god + 1) % 12, (god + 2) % 12);
            let new = startup::new_run_shell_revision(&seed, "Blessed", false, 50.).unwrap();
            let old = startup::new_run_shell_revision(&seed, "Blessed", false, 46.).unwrap();
            assert_eq!(new.rng, old.rng, "god {god}: acquisition must not add random draws");
            assert_eq!(new.gods, vec![god as f64]);
            assert_eq!(new.player.abilities.len(), 1);
            let id = &new.player.abilities[0];
            assert!(expected.contains(id));
            let reserve = &new.player.ability_reserves.as_ref().unwrap()[id];
            assert_eq!((reserve.capacity, reserve.energy), (50., 50.));
            assert!(old.player.ability_reserves.is_missing());
            observed.insert(id.clone());
            if observed == expected { break; }
        }
        assert_eq!(observed, expected, "god {god}: fixed seed corpus missed a starter");
    }
}

#[test]
fn revision50_generated_avatar_blessed_unblessed_save_load_seed_replay_and_restart() {
    for (mode, starters) in [("Avatar", 3), ("Blessed", 1), ("Unblessed", 0)] {
        let initial = startup::create_run_revision("0-5-11:revision50-native-start", mode, false, 50.).unwrap();
        assert_eq!(initial.simulation_revision, Field::Value(50.));
        assert_eq!(initial.player.abilities.len(), starters);
        assert_eq!(initial.floors.len(), 10);
        assert_eq!(initial.status, "playing");
        for id in &initial.player.abilities {
            let reserve = &initial.player.ability_reserves.as_ref().unwrap()[id];
            assert_eq!((reserve.capacity, reserve.energy), (50., 50.));
        }
        let mut engine = Engine::new(initial).unwrap();
        checkpoint(&mut engine);
        engine.dispatch(&Command::Pickup).unwrap();
        let tick = engine.run.tick;
        assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);
        assert_eq!(engine.run.tick, tick + 1.);
        checkpoint(&mut engine);
        replay_exact(None, &engine);
    }
}

#[test]
fn revision50_debug_and_playtest_roundtrip_keep_explicit_replay_boundaries() {
    let debug = corpse_core::debug_floor::create_run_revision("0-5-11:debug-r50", "Unblessed", 1, 50.).unwrap();
    corpse_core::playtest::validate_profile(&debug).unwrap();
    let mut debug = Engine::new(debug).unwrap();
    checkpoint(&mut debug);
    // Debug loadout edits are still not a seed command ledger. Do not silently
    // relax that provenance boundary just because its revision is supported.
    assert!(ReplaySession::create(ReplayRecord::try_from_run(&debug.run).unwrap(), true).is_err());
    corpse_core::debug_floor::enter(&mut debug.run).unwrap();
    assert!(debug.dispatch(&Command::Wait).unwrap().took_turn);
    checkpoint(&mut debug);

    let playtest = corpse_core::playtest::create_run_revision(
        "0-5-11:playtest-r50", "Unblessed", false, 1, corpse_core::playtest::PROFILE, 50.,
    ).unwrap();
    corpse_core::playtest::validate_profile(&playtest).unwrap();
    let mut playtest = Engine::new(playtest).unwrap();
    assert!(playtest.dispatch(&Command::Wait).unwrap().took_turn);
    checkpoint(&mut playtest);
    replay_exact(None, &playtest);
}

#[test]
fn revision50_default_admission_never_upgrades_a_revision46_saved_run() {
    let mut old = fixture(46.);
    old.player.abilities = vec![abilities::ALL_SEEING.into()];
    old.player.active.clear(); old.player.conditions.clear();
    old.player.ability_order = Field::Missing; old.player.ability_reserves = Field::Missing;
    old.player.stats.offense = 123.25;
    let old = Engine::new(old).unwrap();
    let saved = old.save().unwrap();
    let mut loaded = Engine::load(&saved).unwrap();
    assert_eq!(loaded.save().unwrap(), saved);
    assert_eq!(loaded.run.simulation_revision, Field::Value(46.));
    assert_eq!(loaded.run.player.stats.offense, 123.25);
    let shared = loaded.run.player.energy;
    assert!(!loaded.dispatch(&ability(abilities::ALL_SEEING)).unwrap().took_turn);
    assert_eq!(loaded.run.player.energy, shared - 30.);
    assert!(loaded.run.player.ability_reserves.is_missing());
    assert_eq!(Engine::load(&loaded.save().unwrap()).unwrap().run.simulation_revision, Field::Value(46.));
    // Historical core convenience functions remain historical, not UI defaults.
    assert_eq!(startup::new_run_shell("legacy-default", "Unblessed", false).unwrap().simulation_revision, Field::Value(39.));
}

#[test]
fn revision50_admission_rejects_skipped_future_fractional_and_nonfinite_revisions() {
    // R52 is now implemented explicitly; the next revision must still fail.
    for revision in [47., 48., 49., 53., 52.5, 49.5, 50.5, 51.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(startup::new_run_shell_revision("unsupported", "Unblessed", false, revision).is_err());
        assert!(corpse_core::debug_floor::create_run_revision("unsupported", "Unblessed", 1, revision).is_err());
        assert!(corpse_core::playtest::create_run_revision("unsupported", "Unblessed", false, 1,
            corpse_core::playtest::PROFILE, revision).is_err());
        let initial = fixture(revision);
        assert!(storage::validate_current_revision(&initial).is_err());
        let record = ReplayRecord::from_run(&initial);
        assert!(ReplaySession::create(record.clone(), true).is_err());
        assert!(ReplaySession::from_initial(initial, record).is_err());
    }
    assert_eq!(startup::new_run_shell_revision("supported51", "Unblessed", false, 51.).unwrap().simulation_revision, Field::Value(51.));
    assert_eq!(startup::new_run_shell_revision("supported52", "Unblessed", false, 52.).unwrap().simulation_revision, Field::Value(52.));
    let mut initial = fixture(50.);
    let record = ReplayRecord::from_run(&initial);
    initial.simulation_revision = 46.0.into();
    assert!(ReplaySession::from_initial(initial, record).is_err());
}

fn temporal_room() -> Run {
    let mut run = fixture(50.);
    run.player.x = 10.; run.player.y = 10.;
    run.player.abilities = vec![progression::WEAVE_ID.into(), energy::FUTURE_ID.into(),
        abilities::ALL_SEEING.into(), abilities::SPARKLE.into()];
    run.player.active.clear(); run.player.conditions.clear();
    run.player.ability_order = Field::Missing; run.player.equipment_abilities = Field::Missing;
    run.player.ability_reserves = Field::Missing;
    run.player.energy = 100.; run.player.stats.max_energy = 100.;
    run.inventory.clear(); run.offers.clear(); run.replay.clear();
    run.future = None; run.phase = Field::Missing; run.replay_snapshot_from = Field::Missing;
    run.generated_ability_grants = Vec::new().into(); run.timeweave = Field::Missing;
    for actors in run.actors.values_mut() { *actors = Arc::new(vec![]); }
    for ground in run.ground.values_mut() { *ground = Arc::new(vec![]); }
    let floor = run.floor_mut().unwrap();
    floor.tiles.fill("floor".into()); floor.lights.fill(100.); floor.seen.fill(true);
    floor.environment = Field::Missing; floor.doors = Field::Missing; floor.vents = Field::Missing;
    floor.currents = Field::Missing; floor.mud = Field::Missing; floor.slime = Field::Missing;
    floor.roots = Field::Missing; floor.light_sources = Field::Missing; floor.permanent_fire = Field::Missing;
    let amulet = amulet_effects::make_amulet(Effect::Iceblast, 9001.).unwrap();
    run.inventory.push(amulet);
    for (id, effect) in [(9002., "flooding"), (9003., "fire immunity")] {
        run.inventory.push(Item { id, name: format!("Potion of {effect}"), kind: "potion".into(),
            glyph: "!".into(), effect: effect.to_string().into(), identified: true.into(), ..Default::default() });
    }
    run.next_id = 9004.;
    energy::reconcile_acquired(&mut run).unwrap();
    equipment::sync_ownership(&mut run).unwrap();
    assert!(run.inventory.iter().find(|item|item.id==9001.).unwrap().equipped.is_missing());
    let spec=amulet_effects::item_spec(run.inventory.iter().find(|item|item.id==9001.).unwrap()).unwrap();
    assert!(corpse_core::revision50_amulets::plan_item_activation(&run,9001.,Some(&spec)).is_ok());
    run.initial_player = run.player.clone().into();
    run
}
fn amulet_charge(run: &Run) -> f64 {
    run.inventory.iter().find(|item| item.id == 9001.).unwrap().amulet_state.as_ref().unwrap().energy
}

#[test]
fn revision50_engine_commands_restore_pools_terrain_status_and_weave_through_future() {
    let initial = temporal_room();
    let mut engine = Engine::new(initial.clone()).unwrap();
    let commands = [
        Command::Move { dx: 1., dy: 0. }, Command::Move { dx: -1., dy: 0. },
        Command::Move { dx: 1., dy: 0. }, Command::Use { id: 9002. }, Command::Use { id: 9003. },
        ability(abilities::ALL_SEEING), ability(energy::FUTURE_ID),
    ];
    for (index, command) in commands.iter().enumerate() {
        let before_tick = engine.run.tick;
        let outcome = engine.dispatch(command).unwrap();
        if index == 1 { assert!(!outcome.took_turn); assert_eq!(engine.run.tick, before_tick); }
        if index == 5 { assert_eq!(engine.run.player.energy, 100., "All Seeing spends its own reserve first"); }
        checkpoint(&mut engine);
    }
    assert!(engine.run.future.is_some());
    assert_eq!(engine.run.player.energy, 0.);
    let pools = engine.run.player.ability_reserves.as_ref().unwrap();
    assert_eq!(pools[energy::FUTURE_ID].energy, 0.);
    assert_eq!(pools[abilities::ALL_SEEING].energy, 20.);
    assert_eq!(pools[abilities::SPARKLE].energy, 50.);
    assert_eq!(amulet_charge(&engine.run), 50.);
    assert!(engine.run.floor().unwrap().tiles.iter().any(|tile| tile == "shallow-water"));
    assert!(corpse_core::revision50_potions::fire_immune(&engine.run.player));
    let anchor = engine.run.clone();

    let sparkle = Command::Ability { id: abilities::SPARKLE.into(), target: Field::Missing,
        x: 18.0.into(), y: 10.0.into() };
    for command in [sparkle, Command::UseAt { id: 9001., x: 15., y: 10. }, Command::Move { dx: 0., dy: 1. }] {
        engine.dispatch(&command).unwrap(); checkpoint(&mut engine);
    }
    assert_eq!(engine.run.player.ability_reserves.as_ref().unwrap()[abilities::SPARKLE].energy, 30.);
    assert_eq!(amulet_charge(&engine.run), 20.);
    assert_ne!(engine.run.floor().unwrap().permanent_fire, anchor.floor().unwrap().permanent_fire);
    assert_ne!(engine.run.player.conditions, anchor.player.conditions);
    engine.dispatch(&ability(energy::FUTURE_ID)).unwrap();
    checkpoint(&mut engine);
    assert!(engine.run.future.is_none());
    assert_eq!(engine.run.player.energy, anchor.player.energy);
    assert_eq!(engine.run.player.ability_reserves, anchor.player.ability_reserves);
    assert_eq!(engine.run.player.conditions, anchor.player.conditions);
    assert_eq!(engine.run.player.amulet_ledger, anchor.player.amulet_ledger);
    assert_eq!(engine.run.inventory, anchor.inventory);
    assert_eq!(engine.run.floor().unwrap().tiles, anchor.floor().unwrap().tiles);
    assert_eq!(engine.run.floor().unwrap().permanent_fire, anchor.floor().unwrap().permanent_fire);
    assert_eq!(engine.run.floor().unwrap().ice, anchor.floor().unwrap().ice);
    assert_eq!(engine.run.floor().unwrap().environment, anchor.floor().unwrap().environment);
    assert_eq!(engine.run.floor().unwrap().light_sources, anchor.floor().unwrap().light_sources);
    assert_eq!(engine.run.floor().unwrap().revision50_temporary_terrain,
        anchor.floor().unwrap().revision50_temporary_terrain);
    assert_eq!(engine.run.timeweave, anchor.timeweave);
    assert_eq!((engine.run.player.x, engine.run.player.y, engine.run.tick),
        (anchor.player.x, anchor.player.y, anchor.tick));
    // The restored reverse edge is still a free move; action completion expires
    // restored reveal without advancing the world or charging another resource.
    let tick = engine.run.tick;
    assert!(!engine.dispatch(&Command::Move { dx: -1., dy: 0. }).unwrap().took_turn);
    assert_eq!(engine.run.tick, tick);
    assert!(!corpse_core::vision::all_seeing(&engine.run));
    checkpoint(&mut engine);
    replay_exact(Some(initial), &engine);
}

fn assert_r50_loadout(run: &Run) {
    assert!(run.inventory.iter().any(|item| item.id == 2.));
    assert!(run.inventory.iter().any(|item| item.id == 3.));
    assert!(!run.inventory.iter().any(|item| item.id == 1. || item.kind == "armor"));
    assert!(run.inventory.iter().filter(|item| item.kind == "weapon")
        .all(|item| corpse_core::projectiles::is_bow(item) && item.equipped.is_missing()));
    assert!(corpse_core::revision50_inventory::finite_categories_fit(&run.inventory, &run.player.abilities));
    for item in run.inventory.iter().filter(|item| item.kind == "amulet") {
        assert!(item.amulet_state.as_ref().is_some_and(|state|
            state.instance_id == item.id as u64 && state.energy == 50.));
    }
    let profile = run.playtest.as_ref().unwrap();
    assert!(profile.prior_hearts > 0);
    assert!(profile.sources.iter().all(|source| source.depth < profile.start_floor));
}

#[test]
fn revision50_deep_public_debug_creation_choices_stash_and_save_admit_real_generated_worlds() {
    for floor in [6, 40] {
        let run = corpse_core::debug_floor::create_run_revision(
            "0-5-11:r50-deep-public", "Unblessed", floor, 50.,
        ).unwrap();
        assert_eq!(run.depth, floor as f64);
        assert_eq!(run.floors.len(), floor.max(10) as usize);
        let mut engine = Engine::new(run).unwrap();
        // Reload while actual generated prior-floor rewards are still pending.
        checkpoint(&mut engine);
        let tick = engine.run.tick;
        for _ in 0..128 {
            if corpse_core::debug_floor::choices_complete(&engine.run) { break; }
            let id = engine.run.offers.first().expect("Pending debug rewards must expose a choice").clone();
            engine.dispatch(&Command::Choose { id }).unwrap();
        }
        assert!(corpse_core::debug_floor::choices_complete(&engine.run));
        assert_eq!(engine.run.tick, tick, "Synthetic collection does not simulate survival turns");
        let profile = engine.run.playtest.as_ref().unwrap();
        assert_eq!(corpse_core::debug_floor::state(&engine.run).unwrap().consumed, profile.prior_hearts);
        assert_r50_loadout(&engine.run);
        checkpoint(&mut engine);

        // Exercise the public stash path on real generated jewelry, not an
        // injected item or helper-only deeper-floor fixture.
        let item = corpse_core::debug_floor::state(&engine.run).unwrap().stash.iter()
            .find(|reward| matches!(reward.item.kind.as_str(), "ring" | "amulet"))
            .expect("Fixed deep seed must supply a real jewelry acquisition case").item.clone();
        assert!(corpse_core::debug_floor::take(&mut engine.run, item.id).unwrap());
        assert!(corpse_core::debug_floor::equip(&mut engine.run, item.id).unwrap());
        let worn = engine.run.inventory.iter().find(|worn| worn.id == item.id).unwrap();
        assert!(corpse_core::revision50_inventory::worn(worn));
        assert_eq!(worn.amulet_state, item.amulet_state);
        checkpoint(&mut engine);
        corpse_core::debug_floor::enter(&mut engine.run).unwrap();
        assert!(!corpse_core::debug_floor::preparing(&engine.run));
        engine.dispatch(&Command::Wait).unwrap();
        checkpoint(&mut engine);
    }
}

#[test]
fn revision50_deep_public_playtest_creation_loadout_and_save_admit_real_generated_worlds() {
    for floor in [6, 40] {
        let run = corpse_core::playtest::create_run_revision(
            "0-5-11:r50-deep-public", "Unblessed", false, floor, corpse_core::playtest::PROFILE, 50.,
        ).unwrap();
        assert_eq!(run.depth, floor as f64);
        assert_eq!(run.floors.len(), floor.max(10) as usize);
        assert_r50_loadout(&run);
        assert!(run.inventory.iter().filter(|item| matches!(item.kind.as_str(), "ring" | "amulet"))
            .all(corpse_core::revision50_inventory::worn));
        assert!(run.inventory.iter().filter(|item| item.kind == "ring").count() <= 2);
        assert!(run.inventory.iter().filter(|item| item.kind == "amulet").count() <= 1);
        assert!(run.inventory.iter().filter(|item| item.kind == "potion").count() <= 4);
        assert!(run.playtest.as_ref().unwrap().sources.iter()
            .all(|source| run.inventory.iter().any(|item| item.id == source.item_id)));
        let mut engine = Engine::new(run).unwrap();
        checkpoint(&mut engine);
        engine.dispatch(&Command::Wait).unwrap();
        checkpoint(&mut engine);
    }
}
