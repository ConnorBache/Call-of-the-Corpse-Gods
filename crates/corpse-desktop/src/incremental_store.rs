//! Durable command journal with explicit legacy versus snapshot-overwrite replay policy.
//! Complete validation occurs on attachment/reopen. Routine appends check bounded
//! identity/length/boundaries under a lifetime cooperative lock, not earlier bytes.
use super::{Autosave,Kind,tracked_engine::{self,Receipt,Token,Operation},incremental_log as log};
use std::{path::{Path,PathBuf},sync::atomic::Ordering,io::Write};
const MAX_BATCH_COMMANDS:usize=32;
#[derive(serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch { receipts:Vec<Receipt> }
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationBatch {
    version: u8,
    operations: Vec<Operation>,
}
/// O(new bounded operation bytes). Grammar is selected by authenticated header;
/// old and new payloads are not heuristically mixed or silently reinterpreted.
fn parse_operations(raw: &str, metadata: bool) -> Result<Vec<Operation>, String> {
    if !metadata { return parse_batch(raw).map(|b| b.receipts.into_iter().map(Operation::from).collect()); }
    if raw.len() > log::MAX_BATCH_PAYLOAD_BYTES { return Err("Save operation batch exceeds bounds".into()); }
    let batch: OperationBatch = serde_json::from_str(raw).map_err(|e| format!("Invalid save operation batch: {e}"))?;
    if batch.version != 2 || batch.operations.is_empty() || batch.operations.len() > MAX_BATCH_COMMANDS {
        return Err("Save operation batch version/count is outside bounds".into());
    }
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    for (row, operation) in value["operations"].as_array().unwrap().iter().zip(&batch.operations) {
        operation.validate()?;
        if !matches!(operation,Operation::Command{..}) {
            let encoded=serde_json::to_value(operation).map_err(|e|e.to_string())?;
            let original=row["value"].as_object().ok_or("Metadata value must be an object")?;
            let canonical=encoded["value"].as_object().ok_or("Metadata value must serialize as an object")?;
            if original.len()!=canonical.len() || original.keys().any(|key|!canonical.contains_key(key)) {
                return Err("Unknown metadata value fields in save operation".into());
            }
        }
        if let Operation::Command { command, .. } = operation {
            // Unit Command variants otherwise ignore unknown serde keys.
            let encoded = serde_json::to_value(command).map_err(|e| e.to_string())?;
            let original = row["command"].as_object().ok_or("Command must be an object")?;
            let canonical = encoded.as_object().ok_or("Command must serialize as an object")?;
            if original.len() != canonical.len() || original.keys().any(|key| !canonical.contains_key(key)) {
                return Err("Unknown command fields in save operation".into());
            }
        }
    }
    Ok(batch.operations)
}

/// O(1) metadata/path and bounded samples; no retained full recording prefix.
pub(super) struct JournalState {
    path:PathBuf,kind:Kind,token:Token,next_sequence:u64,last_checksum:u32,
    pub(super) attachment:super::recording_attachment::Attachment,
}
/// O(record bytes), bounded32 receipts; no simulation execution or world traversal.
fn parse_batch(raw:&str)->Result<Batch,String> {
    let batch:Batch=serde_json::from_str(raw).map_err(|e|format!("Invalid save command batch: {e}"))?;
    if batch.receipts.is_empty()||batch.receipts.len()>MAX_BATCH_COMMANDS {return Err("Save command batch count is outside bounds".into());}
    // serde's internally tagged unit variants otherwise ignore unknown keys.
    // Compare only command key sets; retain valid numeric representations/Field semantics.
    let value:serde_json::Value=serde_json::from_str(raw).map_err(|e|e.to_string())?;
    for (row,receipt) in value["receipts"].as_array().unwrap().iter().zip(&batch.receipts) {
        let encoded=serde_json::to_value(&receipt.command).map_err(|e|e.to_string())?;
        let original=row["command"].as_object().ok_or("Command must be an object")?;
        let canonical=encoded.as_object().ok_or("Command must serialize as an object")?;
        if original.len()!=canonical.len()||original.keys().any(|key|!canonical.contains_key(key)) {return Err("Unknown command fields in save journal".into());}
    }
    Ok(batch)
}
/// O(file bytes + legacy checkpoint import + admitted recorded command costs).
/// A partial final record is not replayed. Complete invalid records are rejected.
pub(super) fn decode(raw:&[u8])->Result<(corpse_core::temporal::Engine,Kind),String> {
    decode_with_migration(raw).map(|(engine,kind,_)|(engine,kind))
}
/// Same bounded decode; the flag revokes append authority when the admitted
/// live policy is newer than the serialized journal root. No second replay.
pub(super) fn decode_with_migration(raw:&[u8])->Result<(corpse_core::temporal::Engine,Kind,bool),String> {
    // Selection is bounded metadata only; parse below authenticates the complete
    // committed prefix before checkpoint decode or any gameplay execution.
    let header=log::inspect_header(raw)?;
    let legacy=header.legacy_revert_policy()?;
    let metadata=header.metadata_operations();
    let expected=header.clone();
    let mut commands=0usize;
    let maximum_commands=log::MAX_RECORDS.checked_mul(MAX_BATCH_COMMANDS).ok_or("Recording command bound overflow")?;
    let parsed=log::parse(raw,&expected,|payload|{
        let operations=parse_operations(payload,metadata)?;commands=commands.checked_add(operations.len()).ok_or("Recording command count overflow")?;
        if commands>maximum_commands {return Err("Recording exceeds admitted command bound".into());}Ok(())
    })?;
    let (mut engine,kind,firebolt_migrated,amulet_migrated,potion_migrated)=super::decode_checkpoint_policy(parsed.checkpoint,Some(legacy))?;
    if engine.run.simulation_revision.as_ref().copied()!=Some(header.simulation_revision as f64) {
        return Err("Save journal requires its original simulation revision".into());
    }
    engine.set_legacy_revert(legacy);
    for record in &parsed.records {
        for operation in parse_operations(record.payload,metadata)? {
            operation.apply(&mut engine).map_err(|e|format!("Save journal replay failed at record {}: {e}",record.sequence))?;
        }
    }
    // Migration changes only future commands. Old durable tails ran entirely
    // under their original semantics; fresh Autosave attachment checkpoints the
    // new build before writing any new-policy command receipts.
    engine.set_legacy_revert(false);
    if firebolt_migrated {
        engine.run.firebolt_travel_from=corpse_core::Field::Missing;
        corpse_core::firebolt_travel::admit_current(&mut engine.run)?;
    }
    if amulet_migrated {
        engine.run.amulet_rules_from=corpse_core::Field::Missing;
        corpse_core::amulet_rules::admit_current(&mut engine.run)?;
    }
    if potion_migrated {
        engine.run.potion_rules_from=corpse_core::Field::Missing;
        corpse_core::potion_rules::admit_current(&mut engine.run)?;
    }
    Ok((engine,kind,firebolt_migrated || amulet_migrated || potion_migrated))
}
#[cfg(test)]
#[derive(Clone,Copy)]
pub(super) enum AppendFault { Partial(usize), BeforeSync, AfterSync, PausePartial(usize), PauseAfterSync }
#[cfg(test)]
thread_local! { static APPEND_FAULT:std::cell::Cell<Option<AppendFault>>=const{std::cell::Cell::new(None)}; }
/// O(1) thread-local deterministic failure injection; test builds only.
#[cfg(test)]
pub(super) fn inject_append_fault(fault:AppendFault){APPEND_FAULT.with(|slot|slot.set(Some(fault)));}
/// Test-only process-kill handshake; bounded30s timeout if the supervising test dies.
/// O(marker bytes) I/O plus explicit waiting; never compiled into release builds.
#[cfg(test)]
fn pause_append_child()->std::io::Result<()> {
    let ready=std::env::var_os("CORPSE_SAVE_PROBE_READY").ok_or_else(||std::io::Error::other("missing child readiness path"))?;
    std::fs::write(ready,b"append boundary reached")?;
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(30);
    while std::time::Instant::now()<deadline {std::thread::sleep(std::time::Duration::from_millis(50));}
    Err(std::io::Error::other("process-kill test supervisor timed out"))
}
/// O(new frame bytes) plus filesystem synchronization latency. Fault injection
/// models interruption points only, not actual hardware power-loss behavior.
fn append_and_sync(file:&mut std::fs::File,bytes:&[u8])->std::io::Result<()> {
    #[cfg(test)] let fault=APPEND_FAULT.with(|slot|slot.take());
    #[cfg(test)] if let Some(AppendFault::Partial(n))=fault {file.write_all(&bytes[..n.min(bytes.len())])?;return Err(std::io::Error::other("injected partial append"));}
    #[cfg(test)] if let Some(AppendFault::PausePartial(n))=fault {file.write_all(&bytes[..n.min(bytes.len())])?;return pause_append_child();}
    {#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.append_write");file.write_all(bytes)?;}
    #[cfg(test)] if matches!(fault,Some(AppendFault::BeforeSync)){return Err(std::io::Error::other("injected sync failure"));}
    {#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.append_sync");file.sync_all()?;}
    #[cfg(test)] if matches!(fault,Some(AppendFault::PauseAfterSync)){return pause_append_child();}
    #[cfg(test)] if matches!(fault,Some(AppendFault::AfterSync)){return Err(std::io::Error::other("injected post-sync acknowledgement failure"));}
    Ok(())
}
impl Autosave {
    /// Routine O(new receipt bytes) + O(1) bounded attachment I/O and sync latency.
    /// Attachment/reopen is O(file bytes + replay); explicit unknown mutations
    /// retain full-checkpoint compatibility. No32-command/1MiB periodic checkpoint.
    pub fn flush_tracked(&mut self,path:&Path,engine:&tracked_engine::Engine,kind:Kind,force:bool)->Result<(),String>{
        let _timing=corpse_core::timing::span("desktop.autosave_incremental_flush");
        if kind==Kind::Replay||!engine.run.playtest.is_missing(){return Ok(())}
        let batch=engine.pending_batch();
        let key=(batch.token,super::SAVE_REVISION.load(Ordering::Relaxed),kind,path.to_path_buf());
        let result=(||{
            self.ensure_recording_lock(path)?;
            if let Some(certificate)=engine.take_resume_certificate() {
                match certificate.adopt(path,kind) {
                    Ok(Some(loaded)) => {
                        if let Some(cursor)=loaded.cursor {
                            self.journal=Some(JournalState{path:path.to_path_buf(),kind,token:cursor.base,next_sequence:cursor.next_sequence,last_checksum:cursor.last_checksum,attachment:loaded.attachment});
                        }else{
                            if !batch.checkpoint_required{return Err("Revoked resume authority requires checkpoint".into());}
                            self.resume_checkpoint_guard=Some(loaded.attachment);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {self.recording_stop=Some(error.clone());return Err(error);}
                }
            }
            if !force&&self.tracked_last.as_ref()==Some(&key){return Ok(())}
            // ensure_recording_lock clears attachment only for a different canonical key.
            let attached=self.journal.is_some();
            if attached{
                let state=self.journal.as_mut().unwrap();
                if state.kind!=kind{return Err("Recording belongs to a different session type; preserved".into())}
                if let Err(error)=state.attachment.check(path){self.recording_stop=Some(error.clone());self.journal=None;return Err(error)}
            }else{
                self.check_recording_attachment(path)?;
                self.journal=None;self.verified=None;
                let loaded={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.reopen_read");super::read_recording_bytes(path)};
                match loaded{
                    Ok((raw,witness))=>{
                        self.recording_recovery=Some(witness);
                        #[cfg(test)]{self.verification_loads+=1;}
                        let existing_kind={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.reopen_decode");super::decode_bytes(&raw).map_err(|e|format!("Existing recording was preserved because full reopen validation failed: {e}"))?.1};
                        if existing_kind!=kind{return Err("Existing recording belongs to a different session type; preserved".into())}
                    },
                    Err(e)if e.kind()==std::io::ErrorKind::NotFound=>{self.recording_recovery=Some(super::recording_attachment::IdentityWitness::absent());},
                    Err(e)=>return Err(e.to_string()),
                }
            }
            if engine.run.status=="dead"&&!engine.run.dev&&kind==Kind::Genuine&&!super::unknown_seed_replay(&engine.run){
                // Close the data handle before Windows deletion; retain sidecar ownership.
                self.check_recording_attachment(path)?;if let Some(state)=self.journal.take(){self.recording_recovery=Some(state.attachment.witness());}
                if let Some(guard)=self.resume_checkpoint_guard.take(){self.recording_recovery=Some(guard.witness());}
                match std::fs::remove_file(path){Ok(())=>{},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>return Err(e.to_string())}
                self.recording_recovery=None;self.verified=None;if !engine.acknowledge_persisted(batch.token){return Err("Session changed during save acknowledgement".into())}return Ok(())
            }
            let eligible=self.journal.as_ref().is_some_and(|j|Some(j.token)==batch.base_token&&!batch.checkpoint_required);
            if eligible&&batch.operations.is_empty(){return Ok(())}
            if eligible{
                let payload={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.receipt_encode");serde_json::to_string(&OperationBatch{version:2,operations:batch.operations.clone()}).map_err(|e|e.to_string())?};
                if payload.len()>log::MAX_BATCH_PAYLOAD_BYTES{let error="Recording batch exceeds256KiB trial capacity; file and pending changes preserved. Export or choose another save path.".to_string();self.recording_stop=Some(error.clone());return Err(error)}
                let replayable={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.receipt_validate");parse_operations(&payload,true).is_ok_and(|decoded|decoded==batch.operations)};
                if replayable{
                    let state=self.journal.as_ref().unwrap();
                    #[cfg(test)] let (byte_limit,record_limit)=self.recording_limits.unwrap_or((log::MAX_FILE_BYTES,log::MAX_RECORDS as u64));
                    #[cfg(not(test))] let (byte_limit,record_limit)=(log::MAX_FILE_BYTES,log::MAX_RECORDS as u64);
                    if state.next_sequence>record_limit{let error=format!("Recording reached {} records; file and pending changes preserved. Export or choose another save path.",log::MAX_RECORDS);self.recording_stop=Some(error.clone());return Err(error)}
                    let frame={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.frame_encode");log::encode_append(state.next_sequence,state.last_checksum,&payload)?};
                    let next=state.attachment.len().checked_add(frame.bytes.len()as u64).ok_or("Recording size overflow")?;
                    if next>byte_limit as u64{let error="Recording reached32MiB trial capacity; file and pending changes preserved. Export or choose another save path.".to_string();self.recording_stop=Some(error.clone());return Err(error)}
                    let written=self.journal.as_mut().unwrap().attachment.append_with(path,&frame.bytes,append_and_sync);
                    if let Err(failure)=written{let attachment=self.journal.take().map(|state|state.attachment);return Err(match failure{super::recording_attachment::AppendFailure::Write(error)=>{self.recording_recovery=attachment.map(|a|a.witness());error},super::recording_attachment::AppendFailure::Changed(error)=>{self.recording_stop=Some(error.clone());error}})}
                    let state=self.journal.as_mut().unwrap();state.token=batch.token;state.next_sequence=frame.next_sequence;state.last_checksum=frame.last_checksum;
                    if !engine.acknowledge_persisted(batch.token){self.journal=None;return Err("Session changed during save acknowledgement".into())}return Ok(())
                }
                // Unrepresentable command JSON cannot certify replay; preserve the
                // existing exact-current-state checkpoint fallback, not lossy receipts.
            }
            // Initial attachment/reopen or explicit unknown mutation only. Never
            // checkpoint merely because a routine record/tail threshold was reached.
            let checkpoint={#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.checkpoint_encode");super::encode(engine,kind)?};
            if checkpoint.len()>log::MAX_FILE_BYTES{return Err("Current save exceeds32MiB; existing file and pending changes preserved".into())}
            let header=log::Header::for_metadata_revision(engine.run.simulation_revision.as_ref().copied());
            let framed=match header.as_ref() {
                Some(header)=>checkpoint.len()<=log::checkpoint_limit(header)?,
                None=>false,
            };
            if header.is_some() && !framed {return Err("Current checkpoint cannot fit exact metadata recording within32MiB; file and pending changes preserved".into());}
            self.check_recording_attachment(path)?;
            if let Some(state)=self.journal.take(){self.recording_recovery=Some(state.attachment.witness());}
                if let Some(guard)=self.resume_checkpoint_guard.take(){self.recording_recovery=Some(guard.witness());}self.verified=None;
            if !framed{
                // Older revisions and near-limit legacy snapshots retain their
                // original full-snapshot policy, without mechanics upgrades.
                {#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.checkpoint_write_sync_replace");super::atomic_write(path,checkpoint.as_bytes())?;}self.recording_recovery=None;
            }else{
                let mut initial=log::encode_initial(&checkpoint,header.as_ref().expect("framed revision has a header"))?;
                // Checkpoint import normalizes this optional UI field. Restore the
                // exact live value before any subsequent command may observe it.
                let order=Operation::InventoryDisplayOrder{value:tracked_engine::OrderValue::from(engine.run.inventory_display_order.clone()),after:tracked_engine::Stamp::capture(engine)};
                order.validate()?;
                let payload=serde_json::to_string(&OperationBatch{version:2,operations:vec![order]}).map_err(|e|e.to_string())?;
                let restore=log::encode_append(initial.next_sequence,initial.last_checksum,&payload)?;
                if initial.bytes.len().checked_add(restore.bytes.len()).is_none_or(|n|n>log::MAX_FILE_BYTES){return Err("Current checkpoint plus exact metadata exceeds32MiB; existing file and pending changes preserved".into())}
                initial.bytes.extend_from_slice(&restore.bytes);initial.next_sequence=restore.next_sequence;initial.last_checksum=restore.last_checksum;
                {#[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.checkpoint_write_sync_replace");super::atomic_write(path,&initial.bytes)?;}self.recording_recovery=None;
                let attachment=super::recording_attachment::Attachment::open_verified(path,&initial.bytes).map_err(|error|{self.recording_stop=Some(error.clone());error})?;
                self.journal=Some(JournalState{path:path.to_path_buf(),kind,token:batch.token,next_sequence:initial.next_sequence,last_checksum:initial.last_checksum,attachment});
            }
            if !engine.acknowledge_persisted(batch.token){self.journal=None;return Err("Session changed during save acknowledgement".into())}Ok(())
        })();
        self.tracked_last=if result.is_ok(){Some(key)}else{None};
        self.error=result.as_ref().err().cloned();result
    }
}

#[cfg(test)]
#[path="incremental_store_tests.rs"]
mod tests;

#[cfg(test)]
#[path="append_recording_tests.rs"]
mod append_recording_tests;

#[cfg(test)]
mod shore_revision_tests {
    use super::*;
    /// O(fixture import); synthetic revision is explicit, not save migration.
    fn fixture(revision:f64)->corpse_core::temporal::Engine {
        let old=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut run=old.run;run.simulation_revision=revision.into();
        corpse_core::temporal::Engine::new(run).unwrap()
    }
    /// O(checkpoint bytes + one command replay), validates the new exact pair.
    #[test]
    fn shore_checkpoint_and_receipt_round_trip_without_revision_upgrade() {
        for revision in [40.0,41.0,42.0] {
        let mut engine=tracked_engine::Engine::from_core(fixture(revision));
        let checkpoint=super::super::encode(&engine,Kind::Fixture).unwrap();
        let header=log::Header::for_revision(Some(revision)).unwrap();
        let initial=log::encode_initial(&checkpoint,&header).unwrap();
        // The tracker begins checkpoint-required; acknowledge it before collecting a receipt.
        let token=engine.pending_batch().token;assert!(engine.acknowledge_persisted(token));
        engine.dispatch(&corpse_core::Command::Wait).unwrap();
        let pending=engine.pending_batch();assert_eq!(pending.receipts.len(),1);
        let payload=serde_json::to_string(&Batch{receipts:pending.receipts}).unwrap();
        let frame=log::encode_append(initial.next_sequence,initial.last_checksum,&payload).unwrap();
        let mut bytes=initial.bytes;bytes.extend_from_slice(&frame.bytes);
        let (restored,kind)=decode(&bytes).unwrap();assert_eq!(kind,Kind::Fixture);
        assert_eq!(restored.run.simulation_revision.as_ref(),Some(&revision));
        assert_eq!(restored.save().unwrap(),engine.save().unwrap());
        if revision>=41.0 {assert!(restored.history.frames.is_empty());}
        }
    }
    /// O(checkpoint bytes), fail before any tail is executed on mismatched rules.
    #[test]
    fn exact_header_pair_cannot_relabel_checkpoint_simulation_revision() {
        for (checkpoint_revision,header_revision) in [(39.0,40.0),(40.0,39.0),(39.0,41.0),(40.0,41.0),(41.0,39.0),(41.0,40.0),(39.0,42.0),(40.0,42.0),(41.0,42.0),(42.0,39.0),(42.0,40.0),(42.0,41.0)] {
            let engine=fixture(checkpoint_revision);
            let checkpoint=super::super::encode(&engine,Kind::Fixture).unwrap();
            let header=log::Header::for_revision(Some(header_revision)).unwrap();
            let initial=log::encode_initial(&checkpoint,&header).unwrap();
            assert!(decode(&initial.bytes).err().unwrap().contains("original simulation revision"));
        }
    }
}

#[cfg(test)]
#[path="metadata_operation_tests.rs"]
mod metadata_operation_tests;

#[cfg(test)]
#[path="revision43_protocol_tests.rs"]
mod revision43_protocol_tests;

#[cfg(test)]
#[path="revision44_protocol_tests.rs"]
mod revision44_protocol_tests;

#[cfg(test)]
#[path="revision45_protocol_tests.rs"]
mod revision45_protocol_tests;

#[cfg(test)]
#[path="revision46_protocol_tests.rs"]
mod revision46_protocol_tests;

#[cfg(test)]
#[path="revision50_protocol_tests.rs"]
mod revision50_protocol_tests;
