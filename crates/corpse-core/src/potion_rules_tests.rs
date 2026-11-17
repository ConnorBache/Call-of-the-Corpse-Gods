use super::*;
use crate::{Command, replay::{ReplayRecord,ReplaySession}, temporal::Engine};
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/potion-prechange-r52.json")).unwrap()}
fn fingerprint(r:&Run)->String {
    let mut v=crate::storage::canonical_value(r).unwrap();
    v.as_object_mut().unwrap().shift_remove("potionRulesFrom");
    v.as_object_mut().unwrap().shift_remove("ringRulesFrom");
    v.as_object_mut().unwrap().shift_remove("gasRulesFrom");
    format!("{:016x}",serde_json::to_vec(&v).unwrap().iter().fold(0xcbf29ce484222325u64,|h,b|(h ^ *b as u64).wrapping_mul(0x100000001b3)))
}
#[test]
fn genuine_prechange_seed_replay_matches_initial_world_and_final_state() {
    let f=fixture();let record:ReplayRecord=serde_json::from_value(f["record"].clone()).unwrap();
    let mut replay=ReplaySession::create(record,true).unwrap();
    assert!(!current(&replay.engine.run));
    assert_eq!(fingerprint(&replay.engine.run),f["initialFnv64"].as_str().unwrap());
    while replay.step().unwrap(){}
    assert_eq!(fingerprint(&replay.engine.run),f["finalFnv64"].as_str().unwrap());
    replay.restart().unwrap();
    assert_eq!(fingerprint(&replay.engine.run),f["initialFnv64"].as_str().unwrap());
}
#[test]
fn old_checkpoint_upgrades_at_command_boundary_and_mixed_replay_restarts_exactly() {
    let f=fixture();let record:ReplayRecord=serde_json::from_value(f["record"].clone()).unwrap();
    let mut old=ReplaySession::create(record,true).unwrap();while old.step().unwrap(){}
    let mut saved:serde_json::Value=serde_json::from_str(&old.engine.save().unwrap()).unwrap();
    saved.as_object_mut().unwrap().shift_remove("potionRulesFrom");
    let mut live=Engine::load(&saved.to_string()).unwrap();
    assert_eq!(live.run.potion_rules_from,Field::Value(3));assert!(current(&live.run));
    // The end-of-ledger boundary must apply even with no subsequent command.
    let mut mixed=ReplaySession::create(ReplayRecord::try_from_run(&live.run).unwrap(),true).unwrap();
    while mixed.step().unwrap(){};assert_eq!(mixed.engine.save().unwrap(),live.save().unwrap());
    live.dispatch(&Command::Wait).unwrap();
    let mut mixed=ReplaySession::create(ReplayRecord::try_from_run(&live.run).unwrap(),true).unwrap();
    while mixed.step().unwrap(){};assert_eq!(mixed.engine.save().unwrap(),live.save().unwrap());
    mixed.restart().unwrap();while mixed.step().unwrap(){};assert_eq!(mixed.engine.save().unwrap(),live.save().unwrap());
    // Explicit old playback checkpoints are also upgraded when loaded for live play.
    assert!(current(&Engine::load(&old.engine.save().unwrap()).unwrap().run));
}
#[test]
fn potion_policy_survives_snapshot_restore_and_rejects_invalid_replay_boundary() {
    let f=fixture();let record:ReplayRecord=serde_json::from_value(f["record"].clone()).unwrap();
    let mut old=ReplaySession::create(record.clone(),true).unwrap();
    let snapshot=crate::temporal::Snapshot::capture(&old.engine.run);
    old.engine.run.potion_rules_from=0usize.into();snapshot.restore(&mut old.engine.run).unwrap();
    assert_eq!(old.engine.run.potion_rules_from,Field::Value(0));
    assert!(snapshot.value().unwrap().get("potionRulesFrom").is_none());
    for boundary in [Field::Null,Field::Value(4)] {
        let mut invalid=record.clone();invalid.potion_rules_from=boundary;
        assert!(ReplaySession::create(invalid,true).is_err());
    }
}
#[test]
fn legacy_playback_checkpoint_resume_keeps_policy_until_boundary() {
    let f=fixture();let record:ReplayRecord=serde_json::from_value(f["record"].clone()).unwrap();
    let mut replay=ReplaySession::create(record.clone(),true).unwrap();replay.step().unwrap();
    let saved=replay.engine.save().unwrap();
    let mut resumed=Engine::load_replay_checkpoint(&saved).unwrap();assert!(!current(&resumed.run));
    for c in &record.commands[1..] {resumed.dispatch(c).unwrap();}
    assert_eq!(fingerprint(&resumed.run),f["finalFnv64"].as_str().unwrap());
    replay.restart().unwrap();while replay.step().unwrap(){}
    assert_eq!(replay.engine.save().unwrap(),resumed.save().unwrap());
}
#[test]
fn future_restore_keeps_both_live_amendment_boundaries() {
    let f=fixture();let record:ReplayRecord=serde_json::from_value(f["record"].clone()).unwrap();
    let mut e=ReplaySession::create(record,true).unwrap().engine;
    e.run.amulet_rules_from=Field::Null;e.run.potion_rules_from=Field::Null;
    e.run.player.abilities.push("morae-see-the-future".into());
    crate::revision50_ability_energy::initialize_reserve(&mut e.run.player,"morae-see-the-future",10.,0.).unwrap();
    crate::temporal::remember_future(&mut e.run,&e.history).unwrap();
    e.run.amulet_rules_from=0usize.into();e.run.potion_rules_from=0usize.into();
    e.dispatch(&Command::Wait).unwrap();
    let raw=e.save().unwrap();let mut loaded=Engine::load(&raw).unwrap();
    assert!(crate::temporal::restore_future(&mut loaded.run,&mut loaded.history).unwrap());
    assert_eq!(loaded.run.amulet_rules_from,Field::Value(0));
    assert_eq!(loaded.run.potion_rules_from,Field::Value(0));
    assert!(crate::amulet_rules::current(&loaded.run));assert!(current(&loaded.run));
}
