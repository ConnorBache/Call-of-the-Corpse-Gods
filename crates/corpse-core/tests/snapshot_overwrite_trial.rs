//! Full-snapshot trial policy tests. These intentionally do not demand selective
//! preservation of unrelated later edits or byte-identical legacy patch arrays.
use corpse_core::{Command, Field, Run, RunTimeCosts, engine, resources, storage,
    temporal::{self, Engine, Snapshot}};
use std::sync::Arc;

/// O(generated world size), deterministic fixture construction outside assertions.
fn fixture() -> Run {
    let mut r = corpse_core::startup::create_run("02-04-09:timing-r39-1", "Blessed", false).unwrap();
    r.simulation_revision = Field::Value(39.);
    for a in r.actors.values_mut() { *a = Arc::new(Vec::new()); }
    r.player.hp = 100_000.; r.player.max_hp = 100_000.;
    r.player.base_max_hp = Field::Value(100_000.);
    r.player.energy = 100.; r.player.stats.max_energy = 100.;
    r.player.abilities = vec!["numo-revert".into(), "morae-see-the-future".into()];
    r.player.active.clear(); r.player.conditions.clear();
    corpse_core::progression::initialize_ability_state(&mut r.player, "numo-revert", r.tick).unwrap();
    r.player.conditions.insert("revertCharges".into(), 20.);
    r.history.clear(); r.replay.clear(); r.offers.clear(); r.future = None;
    r.haste_snapshot = Field::Missing; r.phase = Field::Missing;
    r.time_costs = Field::Missing;
    resources::begin_resource_turn(&mut r);
    r
}
/// O(1), fixed command construction.
fn ability(id: &str) -> Command { Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing } }
/// O(snapshot header), independent synthetic completed boundary for policy isolation.
fn push_frame(e: &mut Engine) -> Snapshot {
    let s = Snapshot::capture(&e.run);
    e.history.record_completed(s.clone());
    e.run.tick += 1.;
    e.history.closing = Some(Snapshot::capture(&e.run));
    s
}
/// Expected O(1), named player's energy presentation meter lookup.
fn energy_loss(r: &Run) -> f64 {
    r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()]["energy"].loss
}
/// O(serialized state size), comparison excludes no gameplay or resource fields.
fn state_value(r: &Run) -> serde_json::Value { serde_json::to_value(r).unwrap() }

#[test]
fn full_restore_overwrites_unrelated_later_edits_but_preserves_identity_and_replay() {
    let mut r = fixture();
    engine::log(&mut r, "old snapshot message", "player");
    let future = storage::serialize(&r).unwrap();
    r.future = Some(future.clone().into());
    r.run_id = Field::Value("opening-id".into());
    let mut e = Engine::new(r).unwrap();
    let old = push_frame(&mut e);
    e.run.player.hp = 17.; e.run.rng = 123.; e.run.next_id += 100.;
    e.run.metrics.insert("commands".into(), 999.);
    e.run.future = None;
    e.run.run_id = Field::Value("live-id".into());
    e.run.replay.push(Command::Wait);
    engine::log(&mut e.run, "discarded future message", "player");
    let replay = e.run.replay.clone();
    assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
    assert_eq!(e.run.player.hp, old.state.player.hp);
    assert_eq!(e.run.rng, old.state.rng);
    assert_eq!(e.run.next_id, old.state.next_id);
    assert_eq!(e.run.metrics, old.state.metrics);
    assert_eq!(e.run.future, Some(future.into()));
    assert_eq!(e.run.run_id, Field::Value("live-id".into()));
    assert_eq!(e.run.replay, replay);
    assert_eq!(e.run.log, old.state.log);
    assert_eq!(e.run.log_kinds, old.state.log_kinds);
    let cursor = old.cursor.unwrap();
    assert_eq!(e.run.log_archive.as_ref().unwrap().messages.len(), cursor.messages);
    assert_eq!(e.run.log_archive.as_ref().unwrap().events.len(), cursor.events);
    assert!(matches!(e.run.haste_snapshot, Field::Missing));
}

#[test]
fn five_original_snapshots_reapply_cumulative_cost_once_even_after_each_reload() {
    let mut e = Engine::new(fixture()).unwrap();
    for _ in 0..5 { push_frame(&mut e); }
    for n in 1..=5 {
        assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
        assert_eq!(e.run.player.energy, 100. - 2. * n as f64);
        assert_eq!(e.run.player.conditions["revertCharges"], 20. - n as f64);
        assert_eq!(energy_loss(&e.run), 2. * n as f64, "earlier temporal costs must not be added twice");
        assert_eq!(e.run.time_costs.as_ref().unwrap(), &RunTimeCosts { energy: 2. * n as f64, charges: n as f64 });
        assert_eq!(e.history.frames.len(), 5 - n);
        let saved = e.save().unwrap(); e = Engine::load(&saved).unwrap();
        assert_eq!(e.run.player.energy, 100. - 2. * n as f64);
        assert_eq!(energy_loss(&e.run), 2. * n as f64);
    }
}

#[test]
fn snapshot_ledger_and_existing_meter_loss_are_not_charged_again() {
    let mut r = fixture(); r.player.energy = 50.;
    r.time_costs = Field::Value(RunTimeCosts { energy: 8., charges: 3. });
    r.player.conditions.insert("revertCharges".into(), 10.);
    resources::begin_resource_turn(&mut r);
    resources::record_player_energy_loss(&mut r, 3.);
    let mut e = Engine::new(r).unwrap(); push_frame(&mut e);
    e.run.time_costs = Field::Value(RunTimeCosts { energy: 12., charges: 5. });
    assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
    assert_eq!(e.run.player.energy, 44.); // (12 + 2) - 8, not the whole14.
    assert_eq!(e.run.player.conditions["revertCharges"], 7.);
    assert_eq!(energy_loss(&e.run), 9.); // snapshot3 plus actual new cumulative6.
}

#[test]
fn restored_resources_clamp_and_meter_records_only_actual_energy_removed() {
    let mut r = fixture(); r.player.energy = 1.;
    r.player.conditions.insert("revertCharges".into(), 0.);
    resources::begin_resource_turn(&mut r);
    let mut e = Engine::new(r).unwrap(); push_frame(&mut e);
    e.run.player.energy = 100.; e.run.player.conditions.insert("revertCharges".into(), 20.);
    e.run.time_costs = Field::Value(RunTimeCosts { energy: 10., charges: 7. });
    assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
    assert_eq!(e.run.player.energy, 0.);
    assert_eq!(e.run.player.conditions["revertCharges"], 0.);
    assert_eq!(energy_loss(&e.run), 1.);
    assert_eq!(e.run.time_costs.as_ref().unwrap().energy, 12.);
}

#[test]
fn pending_haste_cannot_erase_live_cost_ledger_before_completed_restore() {
    for serialized_pending in [false, true] {
        let mut r = fixture();
        r.time_costs = Field::Value(RunTimeCosts { energy: 4., charges: 1. });
        let mut e = Engine::new(r).unwrap(); let opening = push_frame(&mut e);
        let pending = Snapshot::capture(&e.run);
        e.history.pending_haste = Some(pending.clone());
        if serialized_pending { e.run.haste_snapshot = Field::Value(temporal::js_stringify(&pending.value().unwrap())); }
        e.run.time_costs = Field::Value(RunTimeCosts { energy: 10., charges: 3. });
        e.run.player.hp = 7.;
        assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
        assert_eq!(e.run.tick, opening.state.tick);
        assert_eq!(e.run.player.hp, opening.state.player.hp);
        assert_eq!(e.run.player.energy, 92.); // live12 minus opening4.
        assert_eq!(e.run.player.conditions["revertCharges"], 17.);
        assert_eq!(energy_loss(&e.run), 8.);
        assert!(e.history.pending_haste.is_none());
        assert!(matches!(e.run.haste_snapshot, Field::Missing));
    }
}

#[test]
fn missing_archive_at_opening_removes_later_archive() {
    let mut r = fixture(); r.log_archive = Field::Missing;
    let mut e = Engine::new(r).unwrap(); push_frame(&mut e);
    engine::log(&mut e.run, "later", "player");
    assert!(e.run.log_archive.as_ref().is_some());
    assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
    assert!(matches!(e.run.log_archive, Field::Missing));
}

#[test]
fn actual_wait_revert_commands_match_across_save_load_at_every_boundary() {
    let mut live = Engine::new(fixture()).unwrap();
    for _ in 0..6 { assert!(live.dispatch(&Command::Wait).unwrap().took_turn); }
    assert_eq!(live.history.frames.len(), 5);
    let mut resumed = Engine::load(&live.save().unwrap()).unwrap();
    for n in 0..5 {
        let count = live.history.frames.len();
        let before_replay = live.run.replay.len();
        let command = ability("numo-revert");
        assert!(live.dispatch(&command).unwrap().changed);
        assert!(resumed.dispatch(&command).unwrap().changed);
        assert_eq!(live.history.frames.len(), count - 1);
        assert_eq!(live.run.replay.len(), before_replay + 1);
        assert_eq!(state_value(&live.canonical_run().unwrap()), state_value(&resumed.canonical_run().unwrap()), "Revert{n}");
        resumed = Engine::load(&resumed.save().unwrap()).unwrap();
    }
}

#[test]
fn future_restore_retains_costs_and_replay_then_snapshot_revert_restores_old_future() {
    let mut e = Engine::new(fixture()).unwrap();
    let initial_position = (e.run.player.x, e.run.player.y);
    let remember = ability("morae-see-the-future");
    e.dispatch(&remember).unwrap();
    assert!(e.run.future.is_some());
    assert_eq!(e.run.time_costs.as_ref().unwrap().energy, 100.);
    e.dispatch(&Command::Wait).unwrap();
    e = Engine::load(&e.save().unwrap()).unwrap();
    let replay_len = e.run.replay.len();
    e.dispatch(&remember).unwrap();
    assert!(e.run.future.is_none());
    assert_eq!((e.run.player.x, e.run.player.y), initial_position);
    assert_eq!(e.run.time_costs.as_ref().unwrap().energy, 100.);
    assert_eq!(e.run.replay.len(), replay_len + 1);
    // A later live anchor is not exempt from rewinding to an older snapshot.
    e.run.player.energy = 100.; e.run.player.conditions.insert("revertCharges".into(), 20.);
    push_frame(&mut e);
    temporal::remember_future(&mut e.run, &e.history).unwrap();
    e.run.player.energy = 100.;
    assert!(e.run.future.is_some());
    assert!(temporal::revert(&mut e.run, &mut e.history).unwrap());
    assert!(e.run.future.is_none());
    assert_eq!(e.run.time_costs.as_ref().unwrap().energy, 202.);
}

#[test]
fn optional_roots_added_after_closing_cannot_leak_through_checkpoint_or_cache() {
    for warm in [false,true] {
        let mut r=fixture();r.veridian_sap_harvested=Field::Missing;
        let mut e=Engine::new(r).unwrap();push_frame(&mut e);
        if warm {e.save().unwrap();}
        e.run.veridian_sap_harvested=Field::Value(true);
        let mut memory=e.clone();let mut loaded=Engine::load(&e.save().unwrap()).unwrap();
        assert!(temporal::revert(&mut memory.run,&mut memory.history).unwrap());
        assert!(temporal::revert(&mut loaded.run,&mut loaded.history).unwrap());
        assert!(matches!(memory.run.veridian_sap_harvested,Field::Missing));
        assert!(matches!(loaded.run.veridian_sap_harvested,Field::Missing));
        assert_eq!(state_value(&memory.canonical_run().unwrap()),state_value(&loaded.canonical_run().unwrap()));
    }
}

#[test]
fn full_restore_dirties_health_cache_and_settles_like_deserialized_snapshot() {
    let mut r=fixture();r.player.conditions.insert("buff:defense".into(),3.);
    corpse_core::equipment::sync_health(&mut r,false);
    assert!(!corpse_core::health_dirty::needs_sync(&r));
    let mut e=Engine::new(r).unwrap();push_frame(&mut e);
    e.run.player.conditions.insert("buff:defense".into(),9.);
    corpse_core::equipment::sync_health(&mut e.run,false);
    assert!(!corpse_core::health_dirty::needs_sync(&e.run));
    assert!(temporal::revert(&mut e.run,&mut e.history).unwrap());
    assert!(corpse_core::health_dirty::needs_sync(&e.run));
    let mut oracle:Run=serde_json::from_value(state_value(&e.run)).unwrap();
    assert!(corpse_core::equipment::flush_health(&mut e.run));
    assert!(corpse_core::equipment::flush_health(&mut oracle));
    assert_eq!(state_value(&e.run),state_value(&oracle));
}
