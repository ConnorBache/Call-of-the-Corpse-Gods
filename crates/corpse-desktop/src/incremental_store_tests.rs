//! Filesystem transaction tests: real local writes/sync calls, not a hardware power-loss claim.
use super::*;
use super::super::{decode_bytes,encode};
use corpse_core::{Command,temporal::Engine as CoreEngine};
use std::sync::atomic::{AtomicU64,Ordering as AtomicOrdering};
static CASE_ID:AtomicU64=AtomicU64::new(1);
struct Sandbox(PathBuf);
impl Sandbox {
    /// O(path bytes) setup, test-only unique directory without user save paths.
    fn new(label:&str)->Self {let path=std::env::temp_dir().join(format!("corpse-incremental-{label}-{}-{}",std::process::id(),CASE_ID.fetch_add(1,AtomicOrdering::Relaxed)));std::fs::create_dir_all(&path).unwrap();Self(path)}
    /// O(path bytes).
    fn path(&self)->PathBuf {self.0.join("save.json")}
}
impl Drop for Sandbox {
    /// O(test files), cleanup only the unique directory owned by this test.
    fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}
}
/// Synthetic current-revision fixture for journal coverage; not an upgrade policy.
/// O(fixture decode/import); production legacy saves must keep their revision.
fn core()->CoreEngine {let mut engine=CoreEngine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();engine.run.simulation_revision=39.0.into();CoreEngine::new(engine.run).unwrap()}
/// O(file bytes + checkpoint decode + command replay + canonical exports), outside timed paths.
fn assert_disk(path:&Path,engine:&tracked_engine::Engine,kind:Kind) {
    let raw=std::fs::read(path).unwrap();let (restored,loaded_kind)=decode_bytes(&raw).unwrap();
    assert_eq!(loaded_kind,kind);assert_eq!(restored.save().unwrap(),engine.save().unwrap());
}
/// O(file bytes) framing validation, no simulation replay.
// Compatibility tests count post-checkpoint batches, excluding v2 root-order restoration.
fn records(path:&Path)->usize {let bytes=std::fs::read(path).unwrap();let header=log::inspect_header(&bytes).unwrap();let n=log::parse(&bytes,&header,|_|Ok(())).unwrap().records.len();n-usize::from(header.metadata_operations())}

/// O(40 free commands + durable flushes + recovery); checks uninterrupted append bytes.
#[test]
fn initial_checkpoint_and_appends_continue_past_old_rollover_exactly() {
    let dir=Sandbox::new("rollover");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
    for n in 1..=40 {
        let before=std::fs::read(&path).unwrap();engine.dispatch(&Command::Pickup).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
        let after=std::fs::read(&path).unwrap();
        assert!(after.starts_with(&before));assert!(after.len()>before.len());assert_eq!(records(&path),n);
        assert_disk(&path,&engine,Kind::Fixture);assert!(engine.pending_batch().receipts.is_empty());
    }
}

/// O(all reference trace commands + load/save), exact original core and disk after every step.
#[test]
fn free_haste_revert_future_and_resume_preserve_exact_canonical_state() {
    let traces:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/temporal-command-traces-v48.json")).unwrap();
    for trace in traces.as_array().unwrap() {
        let dir=Sandbox::new("temporal");let path=dir.path();let mut initial:corpse_core::Run=serde_json::from_value(trace["initial"].clone()).unwrap();initial.simulation_revision=39.0.into();let mut reference=CoreEngine::new(initial).unwrap();
        let mut engine=tracked_engine::Engine::from_core(reference.clone());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
        for (i,step) in trace["steps"].as_array().unwrap().iter().enumerate() {
            let command:Command=serde_json::from_value(step["command"].clone()).unwrap();let expected=reference.dispatch(&command).unwrap();let actual=engine.dispatch(&command).unwrap();assert_eq!(actual.took_turn,expected.took_turn);
            assert_eq!(engine.save().unwrap(),reference.save().unwrap(),"{} step{i}",trace["name"]);
            auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);
            if step["resume"]==true {reference=CoreEngine::load(&reference.save().unwrap()).unwrap();engine=tracked_engine::Engine::from_core(decode_bytes(&std::fs::read(&path).unwrap()).unwrap().0);}
        }
    }
}

/// O(a few commands/checkpoints), errors and mutable escapes cannot emit incomplete command tails.
#[test]
fn error_external_order_and_cloned_sessions_force_exact_checkpoints() {
    let dir=Sandbox::new("fallback");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    engine.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),1);
    assert!(engine.dispatch(&Command::Dev{action:"unsupported-persistence-test".into()}).is_err());auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
    engine.run.inventory_display_order=corpse_core::Field::Value(engine.run.inventory.iter().rev().map(|i|i.id).collect());
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
    let mut fork=engine.clone();fork.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&fork,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&fork,Kind::Fixture);
}

/// O(valid replacement decode + checkpoint), malformed/foreign bytes are preserved exactly.
#[test]
fn external_replacement_corruption_and_provenance_are_checked() {
    let dir=Sandbox::new("external");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let mut replacement=core();replacement.run.player.hp-=0.25;let external=encode(&replacement,Kind::Fixture).unwrap();std::fs::write(&path,external.as_bytes()).unwrap();
    engine.dispatch(&Command::Wait).unwrap();let pending=engine.pending_batch();
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert_eq!(std::fs::read(&path).unwrap(),external.as_bytes());assert_eq!(engine.pending_batch().receipts,pending.receipts);
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),external.as_bytes());
    drop(auto);let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);assert_ne!(std::fs::read(&path).unwrap(),external.as_bytes());
    let malformed=b"malformed externally edited save";std::fs::write(&path,malformed).unwrap();assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),malformed);
    drop(auto);let mut auto=Autosave::default();assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),malformed);drop(auto);let mut auto=Autosave::default();let foreign=encode(&replacement,Kind::Genuine).unwrap();std::fs::write(&path,foreign.as_bytes()).unwrap();assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),foreign.as_bytes());
}

/// O(prefix decode/checkpoint), truncated final UTF8 payload is ignored then healed by checkpoint.
#[test]
fn torn_multibyte_final_record_recovers_prior_state_then_heals() {
    let dir=Sandbox::new("torn");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    engine.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let committed=std::fs::read(&path).unwrap();
    let mut after=tracked_engine::Stamp::capture(&engine);after.status.push('🫀');
    let payload=serde_json::to_string(&Batch{receipts:vec![Receipt{command:Command::Wait,after}]}).unwrap();
    let state=auto.journal.as_ref().unwrap();let frame=log::encode_append(state.next_sequence,state.last_checksum,&payload).unwrap();
    let unicode="🫀".as_bytes();let cut=frame.bytes.windows(unicode.len()).position(|v|v==unicode).unwrap()+1;
    let mut file=std::fs::OpenOptions::new().append(true).open(&path).unwrap();file.write_all(&frame.bytes[..cut]).unwrap();file.sync_all().unwrap();drop(file);
    let torn=std::fs::read(&path).unwrap();assert!(torn.starts_with(&committed));assert!(std::str::from_utf8(&torn).is_err());assert_disk(&path,&engine,Kind::Fixture);
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),torn);
    drop(auto);let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,true).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);assert!(std::str::from_utf8(&std::fs::read(&path).unwrap()).is_ok());
}

/// O(checkpoint encode/write), temp collision cannot overwrite destination or acknowledge pending state.
#[test]
fn checkpoint_temp_collision_preserves_file_and_pending_receipts_for_retry() {
    let dir=Sandbox::new("temp");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let original=std::fs::read(&path).unwrap();engine.run.player.energy-=0.25;let token=engine.token();
    let temp=path.with_extension(format!("json.tmp-{}",std::process::id()));std::fs::write(&temp,b"owned by an existing writer").unwrap();
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert_eq!(std::fs::read(&path).unwrap(),original);assert_eq!(std::fs::read(&temp).unwrap(),b"owned by an existing writer");assert!(engine.pending_batch().checkpoint_required);assert_eq!(engine.token(),token);
    std::fs::remove_file(&temp).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);assert!(!engine.pending_batch().checkpoint_required);assert!(auto.error.is_none());
}

/// O(few flushes), detected removal latches until explicit attachment close/reopen.
#[test]
fn missing_attached_file_is_preserved_as_missing_until_explicit_reopen() {
    let dir=Sandbox::new("noop");let path=dir.path();let engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    std::fs::remove_file(&path).unwrap();assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert!(!path.exists());
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert!(!path.exists());
    drop(auto);let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,true).unwrap();assert_disk(&path,&engine,Kind::Fixture);
}

/// O(checkpoints), genuine death deletes resume; replay/DEBUG never touch protected bytes.
#[test]
fn terminal_replay_and_debug_isolation_preserves_existing_contract() {
    let dir=Sandbox::new("isolation");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Genuine,false).unwrap();
    engine.run.status="dead".into();auto.flush_tracked(&path,&engine,Kind::Genuine,false).unwrap();assert!(!path.exists());
    std::fs::write(&path,b"protected unrelated bytes").unwrap();auto.flush_tracked(&path,&engine,Kind::Replay,true).unwrap();assert_eq!(std::fs::read(&path).unwrap(),b"protected unrelated bytes");
    engine.run.playtest=corpse_core::playtest::Profile{version:1,start_floor:1,profile:corpse_core::playtest::PROFILE.into(),prior_hearts:0,prior_blessings:0.,sources:vec![],candidates:vec![],debug:None}.into();auto.flush_tracked(&path,&engine,Kind::Genuine,true).unwrap();assert_eq!(std::fs::read(&path).unwrap(),b"protected unrelated bytes");
}

/// O(commands + two destination checkpoints), acknowledging another path invalidates old append base.
#[test]
fn multiple_autosaves_cannot_append_receipts_onto_a_stale_base() {
    let dir=Sandbox::new("two-paths");let a=dir.path();let b=dir.0.join("other.json");let mut engine=tracked_engine::Engine::from_core(core());let mut one=Autosave::default();let mut two=Autosave::default();
    one.flush_tracked(&a,&engine,Kind::Fixture,false).unwrap();two.flush_tracked(&b,&engine,Kind::Fixture,false).unwrap();
    engine.dispatch(&Command::Wait).unwrap();one.flush_tracked(&a,&engine,Kind::Fixture,false).unwrap();
    engine.dispatch(&Command::Wait).unwrap();two.flush_tracked(&b,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&b),0);assert_disk(&b,&engine,Kind::Fixture);
    one.flush_tracked(&a,&engine,Kind::Fixture,true).unwrap();assert_eq!(records(&a),0);assert_disk(&a,&engine,Kind::Fixture);
}

/// O(ten legacy save/load/checkpoints), admitted revisions must not become unreadable containers.
#[test]
fn legacy_revision_sessions_remain_readable_without_silent_upgrade() {
    for revision in 29..=38 {
        let dir=Sandbox::new("legacy");let path=dir.path();
        let mut raw=CoreEngine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        raw.run.simulation_revision=(revision as f64).into();let mut engine=tracked_engine::Engine::from_core(CoreEngine::new(raw.run).unwrap());
        let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);
        engine.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);
        assert_eq!(decode_bytes(&std::fs::read(&path).unwrap()).unwrap().0.run.simulation_revision.as_ref(),Some(&(revision as f64)));
    }
}

/// O(three faulted writes/recoveries), real bytes/sync calls; not simulated power loss.
#[test]
fn partial_and_sync_boundary_failures_keep_receipts_and_retry_without_double_commands() {
    for (fault,complete) in [(AppendFault::Partial(13),false),(AppendFault::BeforeSync,true),(AppendFault::AfterSync,true)] {
        let dir=Sandbox::new("append-fault");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let original=std::fs::read(&path).unwrap();let before=engine.save().unwrap();let base=engine.token();
        engine.dispatch(&Command::Wait).unwrap();let expected=engine.save().unwrap();let receipt_token=engine.token();
        inject_append_fault(fault);assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());
        let damaged=std::fs::read(&path).unwrap();assert!(damaged.starts_with(&original));assert!(damaged.len()>original.len());
        let pending=engine.pending_batch();assert_eq!(pending.token,receipt_token);assert_eq!(pending.base_token,Some(base));assert_eq!(pending.receipts.len(),1);
        let recovered=decode_bytes(&damaged).unwrap().0.save().unwrap();assert_eq!(recovered,if complete{expected.clone()}else{before});
        auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
        assert_eq!(engine.save().unwrap(),expected);assert!(engine.pending_batch().receipts.is_empty());assert!(auto.error.is_none());
    }
}

/// O(two terminal checkpoints), exact external run ID persists; genuine dead session still deletes.
#[test]
fn terminal_run_id_mutation_checkpoints_exact_id_and_genuine_death_still_deletes() {
    let dir=Sandbox::new("terminal-id");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();engine.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    engine.run.status="won".into();engine.run.run_id=corpse_core::Field::Missing;
    let exact="native-durable-terminal-id";corpse_core::profile::ensure_run_id(&mut engine.run,||exact.into()).unwrap();
    assert!(engine.pending_batch().checkpoint_required);auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
    assert_eq!(decode_bytes(&std::fs::read(&path).unwrap()).unwrap().0.run.run_id.as_ref().map(String::as_str),Some(exact));
    let deadpath=dir.0.join("dead.json");let mut dead=tracked_engine::Engine::from_core(core());let mut deadstore=Autosave::default();deadstore.flush_tracked(&deadpath,&dead,Kind::Genuine,false).unwrap();
    dead.run.status="dead".into();dead.run.run_id=corpse_core::Field::Missing;corpse_core::profile::ensure_run_id(&mut dead.run,||"native-dead-terminal-id".into()).unwrap();
    deadstore.flush_tracked(&deadpath,&dead,Kind::Genuine,false).unwrap();assert!(!deadpath.exists());assert!(!dead.pending_batch().checkpoint_required);assert!(dead.pending_batch().receipts.is_empty());
}

/// O(four bounded batch decodes/replays), production Receipt/Command admission—not fake codec types.
#[test]
fn complete_unknown_fields_divergent_stamps_and_unsupported_commands_are_preserved_and_rejected() {
    let engine=tracked_engine::Engine::from_core(core());let checkpoint=encode(&engine,Kind::Fixture).unwrap();let initial=log::encode_initial(&checkpoint,&log::Header::current()).unwrap();
    let mut next=core();next.dispatch(&Command::Wait).unwrap();
    let valid=serde_json::to_value(Batch{receipts:vec![Receipt{command:Command::Wait,after:tracked_engine::Stamp::capture(&next)}]}).unwrap();
    let mut unknown_command=valid.clone();unknown_command["receipts"][0]["command"]["ignoredExtra"]=true.into();
    let mut unknown_receipt=valid.clone();unknown_receipt["receipts"][0]["ignoredExtra"]=true.into();
    let mut divergent=valid.clone();divergent["receipts"][0]["after"]["tick_bits"]=serde_json::json!(next.run.tick.to_bits()^1);
    let mut unsupported=valid.clone();unsupported["receipts"][0]["command"]=serde_json::json!({"type":"dev","action":"unsupported-persistence-test"});
    for (index,payload) in [unknown_command,unknown_receipt,divergent,unsupported].into_iter().enumerate() {
        let raw=serde_json::to_string(&payload).unwrap();if index<2 {assert!(parse_batch(&raw).is_err());}
        let frame=log::encode_append(initial.next_sequence,initial.last_checksum,&raw).unwrap();let mut bytes=initial.bytes.clone();bytes.extend_from_slice(&frame.bytes);
        assert!(decode_bytes(&bytes).is_err());let dir=Sandbox::new("invalid-record");let path=dir.path();std::fs::write(&path,&bytes).unwrap();
        assert!(Autosave::default().flush_tracked(&path,&engine,Kind::Fixture,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),bytes);
    }
}

/// O(command + checkpoint), cooperating lock contention preserves bytes and unacked receipts.
#[test]
fn coordination_lock_contention_preserves_file_and_receipts_then_retry_succeeds() {
    let dir=Sandbox::new("lock-retry");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let original=std::fs::read(&path).unwrap();
    assert!(super::super::SaveLock::acquire(&path).is_err());drop(auto);let mut auto=Autosave::default();let held=super::super::SaveLock::acquire(&path).unwrap();engine.dispatch(&Command::Wait).unwrap();let pending=engine.pending_batch();
    assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert_eq!(std::fs::read(&path).unwrap(),original);
    assert_eq!(engine.pending_batch().base_token,pending.base_token);assert_eq!(engine.pending_batch().receipts,pending.receipts);
    drop(held);auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);assert!(engine.pending_batch().receipts.is_empty());
}

/// O(path resolution), canonical parent aliases share one persistent sidecar inode.
#[test]
fn coordination_lock_parent_aliases_and_repeated_acquisition_keep_same_file() {
    let dir=Sandbox::new("lock-alias");let path=dir.path();std::fs::create_dir(dir.0.join("nested")).unwrap();let alias=dir.0.join("nested/../save.json");let sidecar=dir.0.join("save.json.lock");
    let first=super::super::SaveLock::acquire(&path).unwrap();assert!(super::super::SaveLock::acquire(&alias).is_err());
    let before=std::fs::metadata(&sidecar).unwrap();assert_eq!(before.len(),0);drop(first);assert!(sidecar.exists());
    let second=super::super::SaveLock::acquire(&alias).unwrap();let after=std::fs::metadata(&sidecar).unwrap();assert_eq!(after.len(),0);
    #[cfg(unix)] {use std::os::unix::fs::MetadataExt;assert_eq!(before.ino(),after.ino());assert_eq!(before.dev(),after.dev());}
    drop(second);assert!(super::super::SaveLock::acquire(&path).is_ok());assert!(sidecar.exists());
}

/// O(legacy export), old full-checkpoint API cooperates with the same OS lock.
#[test]
fn legacy_full_flush_honors_incremental_coordination_lock() {
    let dir=Sandbox::new("legacy-lock");let path=dir.path();let mut engine=tracked_engine::Engine::from_core(core());let mut tracked=Autosave::default();tracked.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let bytes=std::fs::read(&path).unwrap();engine.dispatch(&Command::Wait).unwrap();let mut legacy=Autosave::default();
    assert!(legacy.flush(&path,&engine,Kind::Fixture,false).is_err());assert_eq!(std::fs::read(&path).unwrap(),bytes);
    drop(tracked);legacy.flush(&path,&engine,Kind::Fixture,false).unwrap();assert!(path.exists());assert_disk(&path,&engine,Kind::Fixture);assert_ne!(std::fs::read(&path).unwrap(),bytes);
}

struct ProbeChild { child:Option<std::process::Child> }
impl ProbeChild {
    /// O(process startup + path bytes). Environment is per-child; parent env never changes.
    fn spawn(role:&str,path:&Path,ready:&Path)->Self {
        let child=std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","session_store::incremental_store::tests::process_probe_child","--ignored","--nocapture","--test-threads=1"])
            .env("CORPSE_SAVE_PROBE_ROLE",role).env("CORPSE_SAVE_PROBE_PATH",path).env("CORPSE_SAVE_PROBE_READY",ready)
            .spawn().unwrap();Self{child:Some(child)}
    }
    /// Bounded10s readiness wait; early exit/panic fails the parent and RAII reaps child.
    fn ready(&mut self,marker:&Path) {
        let until=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !marker.exists() {
            assert!(self.child.as_mut().unwrap().try_wait().unwrap().is_none(),"probe exited before readiness");
            assert!(std::time::Instant::now()<until,"probe did not reach boundary within10s");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    /// OS process termination followed by wait/reap. This is not hardware power loss.
    fn kill_and_wait(&mut self) {
        let mut child=self.child.take().unwrap();assert!(child.try_wait().unwrap().is_none(),"probe exited before kill");
        child.kill().unwrap();assert!(!child.wait().unwrap().success());
    }
}
impl Drop for ProbeChild {
    /// Bounded cleanup of owned child on any test panic; prevents orphan lock holders.
    fn drop(&mut self){if let Some(mut child)=self.child.take(){let _=child.kill();let _=child.wait();}}
}

/// Invoked only by supervised tests; no parent global environment mutations.
/// Bounded30s readiness/kill seam. This helper is intentionally ignored by normal suites.
#[test]
#[ignore = "supervised separate-process probe; requires test-local environment"]
fn process_probe_child() {
    let role=std::env::var("CORPSE_SAVE_PROBE_ROLE").expect("supervised role missing");
    let path=PathBuf::from(std::env::var_os("CORPSE_SAVE_PROBE_PATH").expect("supervised path missing"));
    let ready=PathBuf::from(std::env::var_os("CORPSE_SAVE_PROBE_READY").expect("supervised marker missing"));
    if role=="lock" {
        let _held=super::super::SaveLock::acquire(&path).unwrap();std::fs::write(ready,b"lock held").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30));panic!("supervisor failed to terminate lock probe");
    }
    let mut engine=tracked_engine::Engine::from_core(decode_bytes(&std::fs::read(&path).unwrap()).unwrap().0);
    let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    engine.dispatch(&Command::Wait).unwrap();
    match role.as_str(){"partial"=>inject_append_fault(AppendFault::PausePartial(13)),"synced"=>inject_append_fault(AppendFault::PauseAfterSync),_=>panic!("unknown probe role")}
    auto.flush_tracked(&path,&engine,Kind::Fixture,false).expect("supervisor should kill before append returns");panic!("probe unexpectedly acknowledged");
}

/// O(process startup + command/checkpoint), proves cross-process exclusion and kernel release on kill.
#[test]
fn separate_process_lock_is_nonblocking_and_killed_owner_releases_it() {
    let dir=Sandbox::new("process-lock");let path=dir.path();let ready=dir.0.join("ready");let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let original=std::fs::read(&path).unwrap();drop(auto);let mut auto=Autosave::default();let mut child=ProbeChild::spawn("lock",&path,&ready);child.ready(&ready);
    engine.dispatch(&Command::Wait).unwrap();let start=std::time::Instant::now();assert!(auto.flush_tracked(&path,&engine,Kind::Fixture,false).is_err());assert!(start.elapsed()<std::time::Duration::from_secs(2),"coordination lock must be nonblocking");
    assert_eq!(std::fs::read(&path).unwrap(),original);assert_eq!(engine.pending_batch().receipts.len(),1);
    child.kill_and_wait();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_disk(&path,&engine,Kind::Fixture);assert!(engine.pending_batch().receipts.is_empty());
}

/// O(child setup/recovery + command), partial-record process death recovers only the committed prefix.
#[test]
fn killed_partial_append_recovers_previous_state_and_heals_without_duplicate_command() {
    let dir=Sandbox::new("process-partial");let path=dir.path();let ready=dir.0.join("ready");let mut engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();let before=engine.save().unwrap();
    drop(auto);let mut auto=Autosave::default();let mut child=ProbeChild::spawn("partial",&path,&ready);child.ready(&ready);child.kill_and_wait();
    assert_eq!(decode_bytes(&std::fs::read(&path).unwrap()).unwrap().0.save().unwrap(),before);
    engine.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&engine,Kind::Fixture);
}

/// O(child setup/recovery + one command), real sync precedes kill before acknowledgement.
/// Validates process-crash recovery, not power-cut/storage-controller durability or Windows runtime.
#[test]
fn killed_after_real_sync_before_ack_recovers_exactly_one_command() {
    let dir=Sandbox::new("process-synced");let path=dir.path();let ready=dir.0.join("ready");let engine=tracked_engine::Engine::from_core(core());let mut auto=Autosave::default();auto.flush_tracked(&path,&engine,Kind::Fixture,false).unwrap();
    let mut expected=core();expected.dispatch(&Command::Wait).unwrap();
    drop(auto);let mut child=ProbeChild::spawn("synced",&path,&ready);child.ready(&ready);child.kill_and_wait();
    let (recovered,kind)=decode_bytes(&std::fs::read(&path).unwrap()).unwrap();assert_eq!(kind,Kind::Fixture);assert_eq!(recovered.save().unwrap(),expected.save().unwrap());assert_eq!(records(&path),1);
    let recovered=tracked_engine::Engine::from_core(recovered);Autosave::default().flush_tracked(&path,&recovered,Kind::Fixture,false).unwrap();assert_eq!(records(&path),0);assert_disk(&path,&recovered,Kind::Fixture);
}

include!("snapshot_replay_compat_tests.rs");

include!("seed_replay_provenance_tests.rs");
