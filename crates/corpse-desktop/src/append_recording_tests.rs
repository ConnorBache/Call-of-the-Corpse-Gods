//! Extended append-recording contract. Real temporary-file writes/syncs; not a
//! hardware power-loss test. Hidden-prefix mutation acceptance is intentional and
//! tested explicitly, never described as full-file verification on each append.
use super::*;
use super::super::{decode_bytes, SaveLock};
use corpse_core::{Command, Field, temporal::Engine as Core};
use std::{io::{Seek, SeekFrom},sync::{Arc,atomic::{AtomicU64,Ordering as AtomicOrdering}}};
static RECORDING_CASE: AtomicU64 = AtomicU64::new(1);
struct RecordingSandbox(PathBuf);
impl RecordingSandbox {
    /// O(path bytes), creates only a unique test-owned directory.
    fn new(label:&str)->Self {
        let p=std::env::temp_dir().join(format!("corpse-recording-{label}-{}-{}",std::process::id(),RECORDING_CASE.fetch_add(1,AtomicOrdering::Relaxed)));
        std::fs::create_dir_all(&p).unwrap();Self(p)
    }
    /// O(path bytes).
    fn path(&self)->PathBuf {self.0.join("recording.save")}
}
impl Drop for RecordingSandbox {
    /// O(test file bytes/entries), deletes no path outside this owned sandbox.
    fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}
}
/// O(fixture decode+validation), empty-NPC fixture isolates persistence semantics.
fn recording_core()->Core {
    let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=39.0.into();
    for a in r.actors.values_mut(){*a=Arc::new(Vec::new());}
    r.player.hp=100_000.;r.player.max_hp=100_000.;r.player.base_max_hp=100_000.0.into();
    r.player.energy=1000.;r.player.stats.max_energy=1000.;
    r.player.abilities=vec!["numo-revert".into()];r.player.active.clear();r.player.conditions.clear();
    corpse_core::progression::initialize_ability_state(&mut r.player,"numo-revert",r.tick).unwrap();
    r.player.conditions.insert("revertCharges".into(),20.);
    r.offers.clear();r.history.clear();r.replay.clear();r.future=None;r.haste_snapshot=Field::Missing;r.phase=Field::Missing;
    Core::new(r).unwrap()
}
/// O(file bytes), validates framing and reports actual append records, no world replay.
// Count ordinary post-checkpoint records, not the v2 root-order restoration record.
fn recording_count(raw:&[u8])->usize {let header=log::inspect_header(raw).unwrap();let n=log::parse(raw,&header,|_|Ok(())).unwrap().records.len();n-usize::from(header.metadata_operations())}
/// O(state/history export), explicit complete canonical comparison outside timing.
fn same_core(a:&Core,b:&Core){assert_eq!(a.save().unwrap(),b.save().unwrap());}
/// O(1), original Revert command shape.
fn recording_revert()->Command{Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}

/// O(40 real commands+durable appends+one recovery+five Reverts); proves no32-turn rollover.
#[test]
fn recording_more_than_32_waits_remain_appends_and_recover_five_reverts(){
    let dir=RecordingSandbox::new("forty-waits");let path=dir.path();
    let mut engine=tracked_engine::Engine::from_core(recording_core());let mut auto=Autosave::default();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let initial=std::fs::read(&path).unwrap();let mut previous=initial.clone();
    for n in 1..=40 {
        let tick=engine.run.tick;assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);assert_eq!(engine.run.tick,tick+1.);
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
        let raw=std::fs::read(&path).unwrap();assert!(raw.starts_with(&previous),"unexpected checkpoint at command{n}");
        assert!(raw.len()>previous.len());assert_eq!(recording_count(&raw),n);previous=raw;
        assert!(engine.pending_batch().receipts.is_empty());
    }
    assert!(previous.starts_with(&initial));assert_eq!(engine.history.frames.len(),5);
    let (mut loaded,kind)=decode_bytes(&previous).unwrap();assert_eq!(kind,Kind::Fixture);same_core(&engine,&loaded);
    for remaining in (0..5).rev(){
        let command=recording_revert();let tick=engine.run.tick;
        assert!(engine.dispatch(&command).unwrap().changed);assert!(loaded.dispatch(&command).unwrap().changed);
        assert_eq!(engine.history.frames.len(),remaining);assert_eq!(loaded.history.frames.len(),remaining);assert_eq!(engine.run.tick,tick-1.);
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();same_core(&engine,&loaded);
    }
    assert_eq!(recording_count(&std::fs::read(&path).unwrap()),45);
    let (again,kind)=decode_bytes(&std::fs::read(&path).unwrap()).unwrap();assert_eq!(kind,Kind::Fixture);same_core(&engine,&again);
}

/// O(checkpoint bytes+one pending command); same-byte rename exercises identity,
/// not merely sampled contents. Truncation exercises actual length checks.
#[test]
fn recording_external_replacement_and_truncation_fail_without_acknowledging(){
    for replacement in [true,false]{
        let dir=RecordingSandbox::new(if replacement{"replace"}else{"truncate"});let path=dir.path();
        let mut engine=tracked_engine::Engine::from_core(recording_core());let mut auto=Autosave::default();
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let original=std::fs::read(&path).unwrap();
        engine.dispatch(&Command::Wait).unwrap();let pending=engine.pending_batch();
        if replacement{
            let temp=dir.0.join("external-replacement");let mut f=std::fs::File::create(&temp).unwrap();
            f.write_all(&original).unwrap();f.sync_all().unwrap();drop(f);std::fs::rename(temp,&path).unwrap();
        }else{let f=std::fs::OpenOptions::new().write(true).open(&path).unwrap();f.set_len(original.len()as u64-1).unwrap();f.sync_all().unwrap();}
        let edited=std::fs::read(&path).unwrap();
        assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());
        assert_eq!(std::fs::read(&path).unwrap(),edited);assert_eq!(engine.token(),pending.token);
        assert_eq!(engine.pending_batch().receipts,pending.receipts);assert_eq!(engine.pending_batch().base_token,pending.base_token);
        assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err(),"external mismatch must remain latched");
        assert!(auto.flush(&path,&engine,Kind::Fixture,true).is_err(),"legacy full-flush API must not bypass the latch");
        assert_eq!(std::fs::read(&path).unwrap(),edited);
    }
}

/// O(checkpoint+one append+reopen). Explicit approved tradeoff: middle-prefix
/// corruption outside both64-byte samples is NOT detected by a live attachment.
#[test]
fn recording_hidden_prefix_edit_can_append_but_full_reopen_rejects_and_preserves(){
    let dir=RecordingSandbox::new("hidden-prefix");let path=dir.path();
    let mut engine=tracked_engine::Engine::from_core(recording_core());let mut auto=Autosave::default();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let original=std::fs::read(&path).unwrap();assert!(original.len()>512);
    let at=original.len()/2;assert!(at>64&&at<original.len()-64);let mut edited=original.clone();edited[at]^=1;
    {let mut file=std::fs::OpenOptions::new().write(true).open(&path).unwrap();file.seek(SeekFrom::Start(at as u64)).unwrap();file.write_all(&edited[at..at+1]).unwrap();file.sync_all().unwrap();}
    assert_eq!(&edited[..64],&original[..64]);assert_eq!(&edited[edited.len()-64..],&original[original.len()-64..]);
    engine.dispatch(&Command::Wait).unwrap();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).expect("approved bounded checks do not inspect the hidden prefix");
    assert!(engine.pending_batch().receipts.is_empty());let appended=std::fs::read(&path).unwrap();assert!(appended.starts_with(&edited));assert!(appended.len()>edited.len());
    assert!(decode_bytes(&appended).is_err(),"full checksum validation must reject the corrupted committed prefix");
    drop(auto);let mut reopened=Autosave::default();
    assert!(reopened.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),appended);
}

/// O(two checkpoints): lock ownership outlives individual flush calls; dropping
/// its owner releases the lock, and a prior contention failure is retryable.
#[test]
fn recording_lifetime_lock_blocks_second_writer_until_drop(){
    let dir=RecordingSandbox::new("lifetime-lock");let path=dir.path();
    let engine=tracked_engine::Engine::from_core(recording_core());let mut first=Autosave::default();let mut second=Autosave::default();
    first.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let before=std::fs::read(&path).unwrap();
    assert!(SaveLock::acquire(&path).is_err());
    assert!(second.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert_eq!(std::fs::read(&path).unwrap(),before);
    drop(first);{let released=SaveLock::acquire(&path).expect("drop must release cooperative OS lock");drop(released);}
    second.flush_tracked(&path,&engine,Kind::Fixture,false).expect("same pending save must retry after contention clears");
    let (loaded,_)=decode_bytes(&std::fs::read(&path).unwrap()).unwrap();same_core(&engine,&loaded);
}

/// O(two checkpoints+34 commands per case). Per-instance test limits exercise both
/// record and byte capacity without huge worlds or65,536 gameplay turns.
#[test]
fn recording_capacity_failure_latches_through_pending_overflow_and_allows_new_path(){
    for byte_limit in [false,true] {
        let dir=RecordingSandbox::new(if byte_limit {"byte-cap"} else {"record-cap"});let path=dir.path();
        let mut engine=tracked_engine::Engine::from_core(recording_core());let mut auto=Autosave::default();
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let before=std::fs::read(&path).unwrap();
        engine.dispatch(&Command::Wait).unwrap();let pending=engine.pending_batch();
        auto.recording_limits=Some(if byte_limit {(before.len(),log::MAX_RECORDS as u64)} else {(log::MAX_FILE_BYTES,0)});
        let error=auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap_err();
        assert!(error.contains("capacity")||error.contains("records"),"{error}");
        assert_eq!(std::fs::read(&path).unwrap(),before);assert_eq!(engine.token(),pending.token);assert_eq!(engine.pending_batch().receipts,pending.receipts);
        assert_eq!(engine.pending_batch().base_token,pending.base_token);assert!(!engine.pending_batch().checkpoint_required);
        auto.recording_limits=None;
        assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err(),"capacity stop requires explicit destination change/reopen");
        // Pending queue overflow must not turn a stopped append into a destructive checkpoint.
        for _ in 0..33 {engine.dispatch(&Command::Pickup).unwrap();}
        assert!(engine.pending_batch().checkpoint_required);
        let after_more=engine.pending_batch();
        assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());
        assert!(auto.flush(&path,&engine,Kind::Fixture,true).is_err());
        assert_eq!(engine.pending_batch().receipts,after_more.receipts);assert_eq!(engine.token(),after_more.token);
        assert_eq!(std::fs::read(&path).unwrap(),before);
        let other=dir.0.join("explicit-new-recording.save");
        auto.flush_tracked(&other,&engine,Kind::Fixture,false).unwrap();assert!(engine.pending_batch().receipts.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(),before);
        let raw=std::fs::read(&other).unwrap();assert_eq!(recording_count(&raw),0);
        let(loaded,_)=decode_bytes(&raw).unwrap();same_core(&engine,&loaded);
    }
}

/// O(checkpoint bytes+one command per API): a failed create_new checkpoint keeps
/// its exact old-file identity witness; retry cannot overwrite a replacement.
#[test]
fn recording_checkpoint_collision_then_replacement_latches_before_retry(){
    for bare_retry in [false,true] {
        let dir=RecordingSandbox::new("checkpoint-collision");let path=dir.path();
        let mut engine=tracked_engine::Engine::from_core(recording_core());
        {let mut initial=Autosave::default();initial.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();}
        let original=std::fs::read(&path).unwrap();engine.dispatch(&Command::Wait).unwrap();
        let pending=engine.pending_batch();let mut auto=Autosave::default();
        let collision=path.with_extension(format!("json.tmp-{}",std::process::id()));
        std::fs::write(&collision,b"test-owned collision").unwrap();
        let error=auto.flush_tracked(&path,&engine,Kind::Fixture,true).unwrap_err();
        assert!(error.contains("Cannot prepare"),"{error}");
        assert_eq!(std::fs::read(&path).unwrap(),original);
        assert_eq!(engine.pending_batch().receipts,pending.receipts);
        std::fs::remove_file(&collision).unwrap();
        let replacement=dir.0.join("same-bytes-replacement");
        {let mut f=std::fs::File::create(&replacement).unwrap();f.write_all(&original).unwrap();f.sync_all().unwrap();}
        std::fs::rename(&replacement,&path).unwrap();
        for _ in 0..2 {
            let result=if bare_retry {auto.flush(&path,&engine,Kind::Fixture,true)} else {auto.flush_tracked(&path,&engine,Kind::Fixture,true)};
            assert!(result.is_err(),"failed checkpoint must retain identity across retries");
            assert_eq!(std::fs::read(&path).unwrap(),original);
            assert_eq!(engine.token(),pending.token);assert_eq!(engine.pending_batch().receipts,pending.receipts);
            assert_eq!(engine.pending_batch().base_token,pending.base_token);
        }
        assert!(SaveLock::acquire(&path).is_err(),"original cooperative lock stays held");
    }
}

/// O(two checkpoints+one command), Unix filesystem identity regression. A parent
/// symlink retarget is not an explicit new destination when caller spelling stays.
#[cfg(unix)]
#[test]
fn recording_same_caller_parent_symlink_retarget_latches_and_retains_old_lock(){
    use std::os::unix::fs::symlink;
    let dir=RecordingSandbox::new("parent-symlink");
    let a=dir.0.join("a");let b=dir.0.join("b");
    std::fs::create_dir_all(&a).unwrap();std::fs::create_dir_all(&b).unwrap();
    let alias=dir.0.join("alias");symlink(&a,&alias).unwrap();
    let caller=alias.join("recording.save");let old_path=a.join("recording.save");let new_path=b.join("recording.save");
    let mut engine=tracked_engine::Engine::from_core(recording_core());let mut auto=Autosave::default();
    auto.flush_tracked(&caller,&engine,Kind::Fixture,false).unwrap();
    let old_bytes=std::fs::read(&old_path).unwrap();
    // B is a valid identical save, so content validation alone cannot detect intent.
    {let mut f=std::fs::File::create(&new_path).unwrap();f.write_all(&old_bytes).unwrap();f.sync_all().unwrap();}
    let new_bytes=std::fs::read(&new_path).unwrap();
    engine.dispatch(&Command::Wait).unwrap();let pending=engine.pending_batch();
    std::fs::remove_file(&alias).unwrap();symlink(&b,&alias).unwrap();
    for _ in 0..2 {
        assert!(auto.flush_tracked(&caller,&engine,Kind::Fixture,true).is_err());
        assert_eq!(std::fs::read(&old_path).unwrap(),old_bytes);assert_eq!(std::fs::read(&new_path).unwrap(),new_bytes);
        assert_eq!(engine.token(),pending.token);assert_eq!(engine.pending_batch().receipts,pending.receipts);
        assert_eq!(engine.pending_batch().base_token,pending.base_token);
        assert!(SaveLock::acquire(&old_path).is_err(),"retarget must not release original lock");
    }
    assert!(auto.flush(&caller,&engine,Kind::Fixture,true).is_err());
    assert_eq!(std::fs::read(&new_path).unwrap(),new_bytes);
    drop(auto);drop(SaveLock::acquire(&old_path).expect("drop releases old lock"));
}
