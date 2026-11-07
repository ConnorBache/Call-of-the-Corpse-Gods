//! Desktop ownership boundary for command-based durable saves. Core Engine remains
//! public/unrestricted; only this private-owner wrapper can certify command receipts.
//! Mutable escapes conservatively require a full checkpoint. No state hash/diff.
use corpse_core::{Command, Run, engine::{DispatchOutcome,EngineError}, storage::StorageError};
use std::{ops::{Deref,DerefMut},sync::{Mutex,atomic::{AtomicU64,Ordering}}};
const MAX_PENDING_COMMANDS:usize=32;
#[path="tracked_metadata.rs"]
mod metadata;
pub use metadata::{Operation, OrderValue, RunIdValue};
static NEXT_ID:AtomicU64=AtomicU64::new(1);

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Token { identity:u64, epoch:u64 }
#[derive(Clone,Debug)]
pub struct PendingBatch {pub token:Token,pub base_token:Option<Token>,pub checkpoint_required:bool,pub receipts:Vec<Receipt>,pub operations:Vec<Operation>}
/// Cheap replay-divergence guard, not a canonical world checksum.
#[derive(Clone,Debug,PartialEq,serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub rng_bits:u64,pub tick_bits:u64,pub depth_bits:u64,pub replay_len:usize,
    #[serde(default,skip_serializing_if="corpse_core::Field::is_missing")]
    pub phase:corpse_core::Field<String>,pub status:String,pub history_frames:usize,
}
impl Stamp {
    /// O(phase/status bytes), fixed scalar fields; no floor/actor/world traversal.
    pub fn capture(engine:&corpse_core::temporal::Engine)->Self {Self{
        rng_bits:engine.run.rng.to_bits(),tick_bits:engine.run.tick.to_bits(),depth_bits:engine.run.depth.to_bits(),
        replay_len:engine.run.replay.len(),phase:engine.run.phase.clone(),status:engine.run.status.clone(),history_frames:engine.history.frames.len(),
    }}
    /// O(phase/status bytes), defense against divergence, not proof of state equality.
    pub fn verify(&self,engine:&corpse_core::temporal::Engine)->bool {self==&Self::capture(engine)}
}
#[derive(Clone,Debug,PartialEq,serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {pub command:Command,pub after:Stamp}
#[derive(Debug)]
struct Tracking {token:Token,base_token:Option<Token>,checkpoint_required:bool,operations:Vec<Operation>}
#[derive(Debug)]
pub struct Engine {core:corpse_core::temporal::Engine,tracking:Mutex<Tracking>,resume:Mutex<Option<super::recording_resume::Certificate>>}
impl Engine {
    /// O(1) wrapper allocation; takes ownership without duplicating canonical state.
    pub fn from_core(core:corpse_core::temporal::Engine)->Self {
        let identity=NEXT_ID.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|n|n.checked_add(1)).expect("Tracked session identity exhausted");
        Self{core,tracking:Mutex::new(Tracking{token:Token{identity,epoch:0},base_token:None,checkpoint_required:true,operations:Vec::new()}),resume:Mutex::new(None)}
    }
    /// O(1) binds an immutable fully decoded disk prefix, not the later live token.
    pub(super) fn from_certified_load(core:corpse_core::temporal::Engine,mut certificate:super::recording_resume::Certificate)->Self {
        let mut engine=Self::from_core(core);
        let tracking=engine.tracking.get_mut().unwrap_or_else(|e|e.into_inner());
        certificate.bind(tracking.token);
        if certificate.append_eligible(){
            tracking.base_token=Some(tracking.token);
            tracking.checkpoint_required=false;
        }
        *engine.resume.get_mut().unwrap_or_else(|e|e.into_inner())=Some(certificate);
        engine
    }
    /// O(1) authority transfer; unknown writes retain only the conflict guard.
    /// The unknown flag also guards caught panics before an explicit cleanup runs.
    pub(super) fn take_resume_certificate(&self)->Option<super::recording_resume::Certificate> {
        let unknown=self.tracking.lock().unwrap_or_else(|e|e.into_inner()).checkpoint_required;
        let mut certificate=self.resume.lock().unwrap_or_else(|e|e.into_inner()).take();
        if unknown {if let Some(c)=certificate.as_mut(){c.invalidate();}}
        certificate
    }
    /// Core validation/history import cost, plus O(1) wrapper setup.
    pub fn new(run:Run)->Result<Self,StorageError> {corpse_core::temporal::Engine::new(run).map(Self::from_core)}
    /// Core parsing/migration/history import cost, plus O(1) wrapper setup.
    pub fn load(raw:&str)->Result<Self,StorageError> {corpse_core::temporal::Engine::load(raw).map(Self::from_core)}
    /// O(1) epoch bump; explicit exhaustion rather than aliasing an acknowledged token.
    fn advance(tracking:&mut Tracking) {tracking.token.epoch=tracking.token.epoch.checked_add(1).expect("Tracked session epoch exhausted");}
    /// O(command transitive cost + serialized command payload clone). Queue stores
    /// at most32 admitted operations. Overflow/errors require current-state checkpoint;
    /// already queued unsaved receipts remain until successful acknowledgement.
    pub fn dispatch(&mut self,command:&Command)->Result<DispatchOutcome,EngineError> {
        // Mark unknown before execution so a caught panic cannot leave an append
        // certificate referring to partially mutated state. No lock spans core code.
        let prior_checkpoint={let tracking=self.tracking.get_mut().unwrap_or_else(|e|e.into_inner());Self::advance(tracking);let prior=tracking.checkpoint_required;tracking.checkpoint_required=true;prior};
        let result=self.core.dispatch(command);
        let tracking=self.tracking.get_mut().unwrap_or_else(|e|e.into_inner());
        if result.is_ok() && !prior_checkpoint && tracking.operations.len()<MAX_PENDING_COMMANDS {
            tracking.operations.push(Operation::Command{command:command.clone(),after:Stamp::capture(&self.core)});tracking.checkpoint_required=false;
        } else {
            if let Some(c)=self.resume.get_mut().unwrap_or_else(|e|e.into_inner()).as_mut(){c.invalidate();}
        }
        result
    }
    /// O(queued command payload), at most32 operations. Immutable snapshot for storage;
    /// a later acknowledgement must match both session identity and mutation epoch.
    pub fn pending_batch(&self)->PendingBatch {
        let tracking=self.tracking.lock().unwrap_or_else(|e|e.into_inner());
        PendingBatch{token:tracking.token,base_token:tracking.base_token,checkpoint_required:tracking.checkpoint_required,receipts:tracking.operations.iter().filter_map(|op|match op {Operation::Command{command,after}=>Some(Receipt{command:command.clone(),after:after.clone()}),_=>None}).collect(),operations:tracking.operations.clone()}
    }
    /// O(1) identity/epoch token for no-op guards without cloning receipt payloads.
    pub fn token(&self)->Token {self.tracking.lock().unwrap_or_else(|e|e.into_inner()).token}
    /// O(queued payload destruction), only after checkpoint/append is durably synced.
    /// Stale/other-session acknowledgements cannot clear current unsaved changes.
    pub fn acknowledge_persisted(&self,token:Token)->bool {
        let mut tracking=self.tracking.lock().unwrap_or_else(|e|e.into_inner());
        if tracking.token!=token{return false;}
        tracking.operations.clear();tracking.checkpoint_required=false;tracking.base_token=Some(token);self.resume.lock().unwrap_or_else(|e|e.into_inner()).take();true
    }
    /// O(1); caller can explicitly invalidate append eligibility before untracked edits.
    pub fn require_checkpoint(&mut self) {
        if let Some(c)=self.resume.get_mut().unwrap_or_else(|e|e.into_inner()).as_mut(){c.invalidate();}
        let tracking=self.tracking.get_mut().unwrap_or_else(|e|e.into_inner());Self::advance(tracking);tracking.checkpoint_required=true;
    }
}
impl Deref for Engine {
    type Target=corpse_core::temporal::Engine;
    /// O(1), immutable access cannot mutate serialized core fields.
    fn deref(&self)->&Self::Target {&self.core}
}
impl DerefMut for Engine {
    /// O(1) invalidation BEFORE exposing a mutable core reference, even for no-op edits.
    fn deref_mut(&mut self)->&mut Self::Target {self.require_checkpoint();&mut self.core}
}
impl Clone for Engine {
    /// Core clone cost plus O(1) new identity; forks never inherit append authority.
    fn clone(&self)->Self {
        let mut cloned=Self::from_core(self.core.clone());
        *cloned.resume.get_mut().unwrap_or_else(|e|e.into_inner())=self.resume.lock().unwrap_or_else(|e|e.into_inner()).as_ref().map(super::recording_resume::Certificate::clone_guard);
        cloned
    }
}
impl From<corpse_core::temporal::Engine> for Engine {
    /// O(1) ownership transfer and new identity/checkpoint requirement.
    fn from(core:corpse_core::temporal::Engine)->Self {Self::from_core(core)}
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Fixture parse/import cost, then a simulated durable initial checkpoint receipt.
    fn ready()->Engine {
        let engine=Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        assert!(engine.acknowledge_persisted(engine.token()));engine
    }
    /// O(dispatch+save) verifies common read access and inherent dispatch do not escape.
    #[test] fn ordinary_dispatch_and_reads_keep_append_eligibility() {
        let mut engine=ready();let _tick=engine.run.tick;let _saved=engine.save().unwrap();
        assert!(!engine.pending_batch().checkpoint_required);
        engine.dispatch(&Command::Wait).unwrap();let batch=engine.pending_batch();
        assert!(!batch.checkpoint_required);assert_eq!(batch.receipts.len(),1);assert_eq!(batch.receipts[0].command,Command::Wait);assert!(batch.receipts[0].after.verify(&engine));
        assert!(engine.acknowledge_persisted(batch.token));assert!(engine.pending_batch().receipts.is_empty());
    }
    /// O(1) mutation barrier, independent of whether escaped reference actually writes.
    #[test] fn mutable_escape_and_stale_or_foreign_ack_cannot_certify_state() {
        let mut engine=ready();let stale=engine.token();let _: &mut corpse_core::temporal::Engine=&mut engine;
        assert!(engine.pending_batch().checkpoint_required);assert!(!engine.acknowledge_persisted(stale));
        let fork=engine.clone();assert_ne!(engine.token(),fork.token());assert!(fork.pending_batch().checkpoint_required);
        assert!(!engine.acknowledge_persisted(fork.token()));
    }
    /// O(command validation): even a rejected command conservatively needs checkpoint.
    #[test] fn rejected_dispatch_requires_checkpoint() {
        let mut engine=ready();assert!(engine.dispatch(&Command::Dev{action:"unrecognized".into()}).is_err());
        assert!(engine.pending_batch().checkpoint_required);
    }
    /// O(33 free dispatches); overflow retains all admitted receipts and requires full state.
    #[test] fn receipt_bound_never_silently_drops_admitted_commands() {
        let mut engine=ready();for _ in 0..MAX_PENDING_COMMANDS {engine.dispatch(&Command::Pickup).unwrap();}
        assert_eq!(engine.pending_batch().receipts.len(),MAX_PENDING_COMMANDS);
        engine.dispatch(&Command::Pickup).unwrap();let batch=engine.pending_batch();
        assert!(batch.checkpoint_required);assert_eq!(batch.receipts.len(),MAX_PENDING_COMMANDS);
        assert!(engine.acknowledge_persisted(batch.token));assert!(!engine.pending_batch().checkpoint_required);
    }
    /// O(1) explicit edit through public-field syntax is intercepted by DerefMut.
    #[test] fn display_order_mutation_requires_checkpoint() {
        let mut engine=ready();engine.run.inventory_display_order=vec![100.].into();
        assert!(engine.pending_batch().checkpoint_required);
    }
    /// O(command+metadata), guard survives serialization and catches an RNG mismatch.
    #[test] fn receipt_stamp_roundtrips_and_rejects_divergent_metadata() {
        let mut engine=ready();engine.dispatch(&Command::Wait).unwrap();
        let receipt=engine.pending_batch().receipts.pop().unwrap();
        let encoded=serde_json::to_string(&receipt).unwrap();
        let restored:Receipt=serde_json::from_str(&encoded).unwrap();assert_eq!(receipt,restored);
        engine.run.rng+=1.;assert!(!restored.after.verify(&engine));
    }

    /// O(two commands), receipt base follows the last durable acknowledgement.
    #[test] fn receipt_base_identifies_the_exact_acknowledged_prefix() {
        let mut engine=ready();let first=engine.token();
        engine.dispatch(&Command::Wait).unwrap();let batch=engine.pending_batch();assert_eq!(batch.base_token,Some(first));
        assert!(engine.acknowledge_persisted(batch.token));
        engine.dispatch(&Command::Wait).unwrap();assert_eq!(engine.pending_batch().base_token,Some(batch.token));
        assert_ne!(engine.pending_batch().base_token,Some(first));
        assert_eq!(engine.clone().pending_batch().base_token,None);
    }

}

#[cfg(test)]
#[path="tracked_metadata_tests.rs"]
mod metadata_tests;
