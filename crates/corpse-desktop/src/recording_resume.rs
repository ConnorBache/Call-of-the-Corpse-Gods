//! One-use authority from a fully decoded recording, never from an arbitrary Run.
//! Load O(B + replay); first attachment O(B) exact verification under writer lock.
//! Retains at most the existing 32 MiB input until attachment, explicit destination change, or owner drop.
use super::{incremental_log as log, recording_attachment::{Attachment,IdentityWitness},tracked_engine::{Engine,Token},Kind};
use std::path::{Path,PathBuf};

pub(super) struct Certificate {
    path:PathBuf, caller:PathBuf, kind:Kind, raw:std::sync::Arc<Vec<u8>>, metadata:Option<log::Metadata>,
    witness:IdentityWitness, base:Option<Token>, append_authority:bool,
}
impl std::fmt::Debug for Certificate {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.debug_struct("ResumeCertificate").field("path",&self.path).field("bytes",&self.raw.len()).field("base",&self.base).finish()
    }
}
pub(super) struct ResumeCursor {pub base:Token,pub next_sequence:u64,pub last_checksum:u32}
pub(super) struct Adopted {pub attachment:Attachment,pub cursor:Option<ResumeCursor>}
fn canonical_entry(path:&Path)->Result<PathBuf,String>{
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or_else(||Path::new("."));
    Ok(parent.canonicalize().map_err(|e|e.to_string())?.join(path.file_name().ok_or("Save path has no filename")?))
}
impl Certificate {
    pub(super) fn append_eligible(&self)->bool{self.append_authority&&self.metadata.is_some()}
    pub(super) fn bind(&mut self,base:Token){assert!(self.base.is_none());self.base=Some(base);}
    /// Unknown edits revoke append authority, not protection against overwriting
    /// a replaced source. Exact evidence survives until first locked conflict check.
    pub(super) fn invalidate(&mut self){self.append_authority=false;}
    /// O(path bytes), shared immutable evidence; cloned engines inherit only a
    /// source-conflict guard, never permission to append from its base.
    pub(super) fn clone_guard(&self)->Self{Self{path:self.path.clone(),caller:self.caller.clone(),kind:self.kind,raw:self.raw.clone(),metadata:self.metadata,witness:self.witness,base:self.base,append_authority:false}}
    /// Caller already holds the same canonical-entry lifetime lock. New destinations
    /// need a root, never reuse source authority. Same-path changes fail closed.
    pub(super) fn adopt(self,path:&Path,kind:Kind)->Result<Option<Adopted>,String>{
        if canonical_entry(path)?!=self.path{
            if path==self.caller{return Err("Loaded recording directory changed; original preserved until explicit reload".into())}
            return Ok(None)
        }
        if kind!=self.kind{return Err("Loaded recording session type changed; original preserved".into())}
        self.witness.check_identity(path)?;
        let attachment=Attachment::open_verified(path,&self.raw)?;
        // Check again after open/readback to reject replacement with identical bytes.
        self.witness.check_identity(path)?;
        let cursor=if self.append_authority {
            let metadata=self.metadata.ok_or("Missing certified resume cursor")?;
            Some(ResumeCursor{base:self.base.ok_or("Unbound resume certificate")?,next_sequence:metadata.next_sequence,last_checksum:metadata.last_checksum})
        }else{None};
        Ok(Some(Adopted{attachment,cursor}))
    }
}
/// O(identifier bytes), fixed whitelist of metadata protocols eligible only AFTER
/// full checkpoint admission/replay. Recognizing R50 here does not admit its engine.
fn resumable_header(header: &log::Header) -> bool {
    matches!(header.simulation_revision, 41 | 42 | 43 | 44 | 45 | 46 | 50 | 51 | 52)
        && header.metadata_operations()
}
/// Exact decode owns both engine and evidence. Only admitted 41–46/50 metadata
/// recordings are adopted; old-policy inputs and recovered suffixes migrate.
/// Core/storage admission remains independent and runs before header selection.
pub(super) fn load(path:&Path)->Result<(Engine,Kind),String>{
    let canonical=canonical_entry(path)?;
    let(raw,witness)=super::read_recording_bytes(path).map_err(|e|format!("Cannot load {}: {e}",path.display()))?;
    let(core,kind,firebolt_migrated)=if log::is_journal(&raw) {
        super::incremental_store::decode_with_migration(&raw)?
    } else {let(core,kind)=super::decode_bytes(&raw)?;(core,kind,false)};
    let mut cursor=None;
    if log::is_journal(&raw){
        let header=log::inspect_header(&raw)?;
        if resumable_header(&header)&&!firebolt_migrated{
            // Decode above already validated every operation and replayed it.
            // A migrated Firebolt boundary must enter a fresh checkpoint before
            // current-policy commands can append behind this disk root.
            let metadata=log::parse(&raw,&header,|_|Ok(()))?.metadata;
            if !metadata.recovered_tail&&metadata.valid_len==raw.len(){cursor=Some(metadata);}
        }
    }
    witness.check_identity(path)?;
    let certificate=Certificate{path:canonical,caller:path.to_path_buf(),kind,raw:std::sync::Arc::new(raw),metadata:cursor,witness,base:None,append_authority:cursor.is_some()};
    Ok((Engine::from_certified_load(core,certificate),kind))
}
#[cfg(test)]
#[path="recording_resume_tests.rs"]
mod tests;

#[cfg(test)]
#[path="revision52_firebolt_journal_tests.rs"]
mod firebolt_tests;
