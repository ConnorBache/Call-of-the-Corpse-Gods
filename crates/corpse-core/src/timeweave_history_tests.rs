//! Revision41 history-removal gates, independent of pending trail direction rules.
use super::*;
fn fixture(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();
    for a in r.actors.values_mut(){*a=Arc::new(vec![]);}
    r.player.hp=100_000.;r.player.max_hp=100_000.;r.player.base_max_hp=100_000.0.into();
    r.player.energy=100.;r.player.stats.max_energy=100.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.offers.clear();r.replay.clear();r.history.clear();r.future=None;r.haste_snapshot=Field::Missing;
    r.replay_snapshot_from=Field::Missing;r.phase=Field::Missing;r
}
fn future()->crate::Command{crate::Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
fn assert_no_history(e:&Engine){
    assert!(e.history.frames.is_empty());assert!(e.history.closing.is_none());
    assert!(e.history.pending_haste.is_none());assert!(e.run.history.is_empty());
    assert!(e.run.haste_snapshot.is_missing());
}
/// O(commands * simulation), plus actual checkpoint encode/decode; test counters are thread-local.
#[test] fn new_rules_capture_no_revert_snapshots_on_wait_free_save_or_reload(){
    SNAPSHOT_CALLS.with(|n|n.set(0));
    let mut e=Engine::new(fixture(41.)).unwrap();
    for _ in 0..8 {e.dispatch(&crate::Command::Wait).unwrap();e.dispatch(&crate::Command::Pickup).unwrap();assert_no_history(&e);}
    let raw=e.save().unwrap();let loaded=Engine::load(&raw).unwrap();assert_no_history(&loaded);
    assert_eq!(raw,loaded.save().unwrap());
    assert_eq!(SNAPSHOT_CALLS.with(|n|n.get()),0);
}
/// O(two temporal activations + simulation + checkpoint), retaining only the explicit Future anchor.
#[test] fn future_preserves_phase_and_replay_without_retained_revert_state(){
    SNAPSHOT_CALLS.with(|n|n.set(0));
    let mut r=fixture(41.);r.player.abilities.push("morae-see-the-future".into());r.phase="normal".to_string().into();
    let mut e=Engine::new(r).unwrap();let tick=e.run.tick;
    e.dispatch(&future()).unwrap();assert!(e.run.future.is_some());assert_no_history(&e);
    let anchor:Run=storage::deserialize(e.run.future.as_ref().unwrap().wire_text().unwrap()).unwrap();assert!(anchor.history.is_empty());assert!(anchor.haste_snapshot.is_missing());assert_eq!(anchor.phase,e.run.phase);
    e.dispatch(&crate::Command::Wait).unwrap();
    let mut e=Engine::load(&e.save().unwrap()).unwrap();
    e.dispatch(&future()).unwrap();assert!(e.run.future.is_none());assert_eq!(e.run.tick,tick);
    assert_eq!(e.run.phase,Field::Value("normal".into()));assert_eq!(e.run.replay.len(),3);
    assert!(!e.run.player.conditions.contains_key("revertCharges"));assert_no_history(&e);
    assert_eq!(SNAPSHOT_CALLS.with(|n|n.get()),0);
}
#[test] fn old_rules_still_capture_and_new_rules_reject_legacy_history(){
    let mut e=Engine::new(fixture(40.)).unwrap();e.dispatch(&crate::Command::Wait).unwrap();
    assert_eq!(e.history.frames.len(),1);assert!(e.history.closing.is_some());
    let mut imported=e.canonical_run().unwrap();imported.simulation_revision=41.0.into();
    assert!(Engine::new(imported).unwrap_err().to_string().contains("legacy Revert history"));
}
#[test] fn retired_activation_fails_without_mutation(){
    let mut e=Engine::new(fixture(41.)).unwrap();let before=e.save().unwrap();
    let command=crate::Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    assert!(e.dispatch(&command).is_err());assert_eq!(e.save().unwrap(),before);
}

#[test] fn administrative_retired_history_cannot_emit_unloadable_checkpoint(){
    let mut e=Engine::new(fixture(41.)).unwrap();
    e.run.history.push("{}".into());assert!(e.save().is_err());assert!(e.canonical_run().is_err());
    e.run.history.clear();e.run.haste_snapshot=Field::Value("{}".into());
    assert!(e.save().is_err());assert!(e.canonical_run().is_err());
}
/// Actual preexisting haste-free movement must keep phase, not a retired rewind opening.
#[test] fn actual_hasted_move_has_no_pending_snapshot_and_resumes_phase(){
    SNAPSHOT_CALLS.with(|n|n.set(0));
    let mut r=fixture(41.);r.player.x=5.;r.player.y=5.;
    let f=Arc::make_mut(&mut r.floors[0]);let w=f.width();
    f.tiles[5*w+5]="floor".into();
    f.tiles[5*w+6]="floor".into();
    r.player.conditions.insert("hasted".into(),5.);
    let tick=r.tick;let mut e=Engine::new(r).unwrap();
    let out=e.dispatch(&crate::Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert!(out.pending_snapshot.is_none());
    assert_eq!((e.run.player.x,e.run.player.y),(6.,5.));assert_eq!(e.run.tick,tick);
    assert_eq!(e.run.phase,Field::Value("normal".into()));assert_no_history(&e);
    let loaded=Engine::load(&e.save().unwrap()).unwrap();assert_no_history(&loaded);
    assert_eq!(loaded.run.phase,e.run.phase);assert_eq!(SNAPSHOT_CALLS.with(|n|n.get()),0);
}

#[test] fn invalid_administrative_trail_cannot_emit_unloadable_checkpoint(){
    let mut e=Engine::new(fixture(41.)).unwrap();
    let mut t=crate::timeweave::Trail::default();
    let cell=crate::timeweave::Cell{floor:99,plane:crate::timeweave::Plane::Waking,tile:0};
    t.append(crate::timeweave::Segment{cell,from:cell,to:crate::timeweave::Cell{tile:1,..cell}});
    e.run.timeweave=Field::Value(Arc::new(t));assert!(e.save().is_err());assert!(e.canonical_run().is_err());
}

#[test] fn checkpoint_borrows_new_state_and_owns_legacy_export_without_wire_change(){
    for revision in [40.,41.] {
        let mut e=Engine::new(fixture(revision)).unwrap();e.dispatch(&crate::Command::Wait).unwrap();
        let state=e.checkpoint_state().unwrap();
        assert_eq!(matches!(state,std::borrow::Cow::Borrowed(_)),revision==41.);
        assert_eq!(storage::serialize(&state).unwrap(),storage::serialize(&e.canonical_run().unwrap()).unwrap());
        if revision==41. {assert!(std::ptr::eq(state.as_ref(),&e.run));}
    }
}

#[test] fn new_future_cannot_switch_recording_mechanics_revision(){
    let mut e=Engine::new(fixture(41.)).unwrap();
    e.run.future=Some(storage::serialize(&fixture(40.)).unwrap().into());
    let before=storage::serialize(&e.run).unwrap();
    assert!(restore_future(&mut e.run,&mut e.history).unwrap_err().to_string().contains("mechanics revision"));
    assert_eq!(storage::serialize(&e.run).unwrap(),before);assert_no_history(&e);
}

#[test] fn failed_future_command_preserves_replay_metrics_and_current_state(){
    for invalid_kind in 0..3 {
        let mut r=fixture(41.);r.player.abilities.push("morae-see-the-future".into());
        let mut e=Engine::new(r).unwrap();e.dispatch(&crate::Command::Wait).unwrap();
        let mut anchor=fixture(if invalid_kind == 0 {40.} else {41.});
        if invalid_kind == 1 {anchor.history.push("{}".into());}
        if invalid_kind == 2 {
            use crate::timeweave::{Cell,Plane,Segment,Trail};
            let from=Cell{floor:999,plane:Plane::Waking,tile:0};
            let mut t=Trail::default();t.append(Segment{cell:from,from,to:Cell{tile:1,..from}});
            anchor.timeweave=Field::Value(Arc::new(t));
        }
        e.run.future=Some(storage::serialize(&anchor).unwrap().into());
        let before=storage::serialize(&e.run).unwrap();
        assert!(e.dispatch(&future()).is_err());
        assert_eq!(storage::serialize(&e.run).unwrap(),before);assert_no_history(&e);
    }
}
