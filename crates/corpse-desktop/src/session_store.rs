//! Presentation provenance and terminal record gate. Never consumes simulation RNG.
use corpse_core::{
    profile::{archive_run_record, Profile},
    replay::ReplayRecord,
    Run,
};
use std::path::Path;
#[path="replay_archive.rs"]
pub mod replay_archive;
#[path="tracked_engine.rs"]
pub mod tracked_engine;
#[path="incremental_log.rs"]
pub mod incremental_log;
#[path="incremental_store.rs"]
mod incremental_store;
/// Genuine-run ranking enabled for the source-tested native preview; fixtures/dev/replay stay excluded.
pub const RELEASE_RANKING_ENABLED: bool = true;
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Genuine,
    Fixture,
    Replay,
}
pub struct Archive {
    pub profile: Profile,
    pub replay: ReplayRecord,
}
/// O(1) provenance guard. Unknown experimental ancestry cannot promise a seed
/// replay; preserve its terminal checkpoint instead of newly deleting recovery data.
pub fn unknown_seed_replay(run:&Run)->bool{matches!(run.replay_snapshot_from,corpse_core::Field::Null)}
pub const UNKNOWN_REPLAY_WARNING:&str="Seed replay unavailable: older experimental Revert ancestry is unknown. Terminal save retained; F7 can export the saved session.";
pub fn prepare(
    profile: &Profile,
    run: &Run,
    kind: Kind,
    ranking_enabled: bool,
    saved_at: Option<&str>,
) -> Result<Option<Archive>, String> {
    if !ranking_enabled
        || kind != Kind::Genuine
        || run.status == "playing"
        || !run.playtest.is_missing()
        || run.dev
        || run
            .replay
            .iter()
            .any(|c| matches!(c, corpse_core::Command::Dev { .. }))
    {
        return Ok(None);
    }
    if unknown_seed_replay(run){return Err(UNKNOWN_REPLAY_WARNING.into());}
    let (profile, added) =
        archive_run_record(profile, run, false, saved_at).map_err(|e| e.to_string())?;
    if !added {
        return Ok(None);
    }
    let mut replay = ReplayRecord::try_from_run(run).map_err(|e|e.to_string())?;
    if let Some(date) = saved_at {
        replay.saved_at = corpse_core::Field::Value(date.into());
    }
    Ok(Some(Archive { profile, replay }))
}
/// Existing invalid records are never replaced. Temporary files stay beside their destination.
pub fn archive(
    profile_path: &Path,
    replay_dir: &Path,
    run: &Run,
    kind: Kind,
    saved_at: Option<&str>,
) -> Result<bool, String> {
    if !RELEASE_RANKING_ENABLED || kind != Kind::Genuine {
        return Ok(false);
    }
    let raw = match std::fs::read_to_string(profile_path) {
        Ok(raw) => Some(raw),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let profile = corpse_core::profile::parse_profile(raw.as_deref()).map_err(|e| e.to_string())?;
    let date = saved_at.map(str::to_owned).unwrap_or_else(utc_now);
    let Some(archive) = prepare(&profile, run, kind, RELEASE_RANKING_ENABLED, Some(&date))? else {
        return Ok(false);
    };
    // Publish the replay without replacing any older file, then record the run.
    replay_archive::write(replay_dir, run, &date,
        &serde_json::to_vec(&archive.replay).map_err(|e| e.to_string())?)?;
    atomic_write(
        profile_path,
        &serde_json::to_vec(&archive.profile).map_err(|e| e.to_string())?,
    )?;
    Ok(true)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let _timing = corpse_core::timing::span("desktop.save_atomic_write");
    use std::io::Write;
    let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("Cannot prepare {}: {e}", temp.display()))?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|e| {
        format!(
            "Could not replace {}: {e}. Existing destination was preserved.",
            path.display()
        )
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_archive_gate_preserves_eligible_resume_and_excludes_fixtures() {
        let mut run = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        run.status = "dead".into();
        corpse_core::profile::ensure_run_id(&mut run, || "native-test-record".into()).unwrap();
        let p = Profile::default();
        assert!(prepare(&p, &run, Kind::Fixture, true, None)
            .unwrap()
            .is_none());
        assert!(prepare(&p, &run, Kind::Replay, true, None)
            .unwrap()
            .is_none());
        assert!(prepare(&p, &run, Kind::Genuine, false, None)
            .unwrap()
            .is_none());
        let resumed =
            corpse_core::storage::deserialize(&corpse_core::storage::serialize(&run).unwrap())
                .unwrap();
        let record = prepare(&p, &resumed, Kind::Genuine, true, None)
            .unwrap()
            .unwrap();
        assert_eq!(record.profile.run_archives.len(), 1);
        assert!(prepare(&record.profile, &run, Kind::Genuine, true, None)
            .unwrap()
            .is_none());
        assert_eq!(record.replay.seed, run.seed);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveEnvelope<T = serde_json::Value> {
    native_save_version: u32,
    session: Kind,
    run: T,
}
/// Save boundary: T_export + O(B) time, M_export + O(B) transient memory,
/// where B is emitted bytes; legacy history export retains its own transitive
/// diff/serialization costs (H <= 5 frames), not claimed constant-time.
/// Export retains legacy inverse history; serialize the typed Run directly so
/// there is no intermediate JSON string, parse tree, or second Run traversal.
pub fn encode(engine: &corpse_core::temporal::Engine, kind: Kind) -> Result<String, String> {
    let _timing = corpse_core::timing::span("desktop.save_encode");
    let run = engine.checkpoint_state().map_err(|e| e.to_string())?;
    serde_json::to_string(&SaveEnvelope {
        native_save_version: 1,
        session: kind,
        run,
    })
    .map_err(|e| e.to_string())
}
/// Stable coordination handle. The empty sidecar is intentionally never unlinked:
/// deleting a live lock's pathname would let another process lock a new inode.
struct SaveLock { _file:std::fs::File }
impl SaveLock {
    /// O(path bytes) plus bounded filesystem calls, nonblocking. Coordinates only
    /// cooperating autosave writers using the same canonical parent/entry name.
    fn acquire(path:&Path)->Result<Self,String> {
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or_else(||Path::new("."));
        let parent=parent.canonicalize().map_err(|e|format!("Cannot resolve save directory: {e}"))?;
        let mut name=path.file_name().ok_or("Save path has no filename")?.to_os_string();name.push(".lock");
        let sidecar=parent.join(name);
        let file=std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&sidecar).map_err(|e|format!("Cannot open save coordination lock: {e}"))?;
        file.try_lock().map_err(|e|format!("Cannot save while another writer owns the save lock (retry): {e}"))?;
        Ok(Self{_file:file})
    }
}
/// O(min(file bytes,32MiB+1)) time/memory; stream limit remains safe if file grows.
pub fn read_save_bytes(path:&Path)->std::io::Result<Vec<u8>>{read_recording_bytes(path).map(|(bytes,_)|bytes)}
/// O(file bytes), full bounded read carries identity from the same opened handle.
fn read_recording_bytes(path:&Path)->std::io::Result<(Vec<u8>,recording_attachment::IdentityWitness)>{
    use std::io::Read;let mut file=std::fs::File::open(path)?;
    if file.metadata()?.len()>incremental_log::MAX_FILE_BYTES as u64{return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"Save exceeds32MiB; preserved"))}
    let witness=recording_attachment::IdentityWitness::from_file(&file)?;
    let mut raw=Vec::new();(&mut file).take(incremental_log::MAX_FILE_BYTES as u64+1).read_to_end(&mut raw)?;
    if raw.len()>incremental_log::MAX_FILE_BYTES{return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"Save exceeds32MiB; preserved"))}
    witness.check_identity(path).map_err(std::io::Error::other)?;Ok((raw,witness))
}
/// O(input bytes + checkpoint import + bounded replay); accepts torn UTF8 journal tails.
pub fn decode_bytes(raw: &[u8]) -> Result<(corpse_core::temporal::Engine, Kind), String> {
    if incremental_log::is_journal(raw) { return incremental_store::decode(raw); }
    decode_legacy(std::str::from_utf8(raw).map_err(|e| e.to_string())?)
}
/// Compatibility string entry point; transitive decode_bytes cost.
pub fn decode(raw: &str) -> Result<(corpse_core::temporal::Engine, Kind), String> { decode_bytes(raw.as_bytes()) }
/// Legacy JSON input path; O(bytes + validation/history import), unchanged format.
fn decode_legacy(raw: &str) -> Result<(corpse_core::temporal::Engine, Kind), String> {
    decode_checkpoint_policy(raw,None).map(|(engine,kind,_,_,_,_,_,_,_,_,_,_,_,_,_, _,_,_,_,_)|(engine,kind))
}
/// O(checkpoint bytes + import), once at loading/attachment. Recording policy is
/// supplied by its authenticated header, never inferred from turn/tick counts.
/// An old experimental snapshot checkpoint without provenance remains unknown;
/// it can load/save but cannot falsely claim a reproducible seed-only export.
/// The flags report markerless journal policies for EOF adoption.
fn decode_checkpoint_policy(raw:&str,legacy:Option<bool>)->Result<(corpse_core::temporal::Engine,Kind,bool,bool,bool,bool,bool,bool,bool,bool,bool,bool,bool,bool,bool, bool,bool,bool,bool,bool),String>{
    let v:serde_json::Value=serde_json::from_str(raw).map_err(|e|e.to_string())?;
    let(mut run,kind)=if v.get("nativeSaveVersion").is_some(){let envelope:SaveEnvelope=serde_json::from_value(v).map_err(|e|e.to_string())?;if envelope.native_save_version!=1{return Err("Unsupported native save version.".into())}(envelope.run,envelope.session)}else{(v,Kind::Genuine)};
    let mut firebolt_migrated=false;
    let mut amulet_migrated=false;
    let mut potion_migrated=false;
    let mut ring_migrated=false;
    let mut gas_migrated=false;
    let mut commandment_migrated=false;
    let mut screamer_migrated=false;
    let mut ember_migrated=false;
    let mut ragelight_migrated=false;
    let mut balance_migrated=false;
    let mut collapse_migrated=false;
    let mut blood_mist_fire_migrated=false;
    let mut half_here_lock_migrated=false;
    let mut steam_migrated=false;
    let mut execute_migrated=false;
    let mut beckoning_migrated=false;
    let mut ichor_fire_migrated=false;
    let mut future_migrated=false;
    if let Some(legacy)=legacy{
        let object=run.as_object_mut().ok_or("Save checkpoint must be an object")?;
        // A journal's durable tail predates this load. Suppress ordinary live
        // checkpoint adoption until every recorded command has been replayed.
        firebolt_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && !object.contains_key("fireboltTravelFrom");
        if firebolt_migrated {object.insert("fireboltTravelFrom".into(),serde_json::Value::Null);}
        amulet_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("amuletRulesFrom").is_none_or(serde_json::Value::is_null);
        if amulet_migrated {object.insert("amuletRulesFrom".into(),serde_json::Value::Null);}
        potion_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("potionRulesFrom").is_none_or(serde_json::Value::is_null);
        if potion_migrated {object.insert("potionRulesFrom".into(),serde_json::Value::Null);}
        ring_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("ringRulesFrom").is_none_or(serde_json::Value::is_null);
        if ring_migrated {object.insert("ringRulesFrom".into(),serde_json::Value::Null);}
        gas_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("gasRulesFrom").is_none_or(serde_json::Value::is_null);
        if gas_migrated {object.insert("gasRulesFrom".into(),serde_json::Value::Null);}
        commandment_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64).is_some_and(|r|r>=50.)
            && object.get("commandmentRulesFrom").is_none_or(serde_json::Value::is_null);
        if commandment_migrated {object.insert("commandmentRulesFrom".into(),serde_json::Value::Null);}
        screamer_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64).is_some_and(|r|r>=50.)
            && object.get("screamerRulesFrom").is_none_or(serde_json::Value::is_null);
        if screamer_migrated {object.insert("screamerRulesFrom".into(),serde_json::Value::Null);}
        ember_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64).is_some_and(|r|r>=50.)
            && object.get("emberRulesFrom").is_none_or(serde_json::Value::is_null);
        if ember_migrated {object.insert("emberRulesFrom".into(),serde_json::Value::Null);}
        ragelight_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("ragelightRulesFrom").is_none_or(serde_json::Value::is_null);
        if ragelight_migrated {object.insert("ragelightRulesFrom".into(),serde_json::Value::Null);}
        balance_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("balanceRulesFrom").is_none_or(serde_json::Value::is_null);
        if balance_migrated {object.shift_remove("balanceRulesFrom");}
        collapse_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("collapseRulesFrom").is_none_or(serde_json::Value::is_null);
        if collapse_migrated {object.shift_remove("collapseRulesFrom");}
        blood_mist_fire_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("bloodMistFireRulesFrom").is_none_or(serde_json::Value::is_null);
        if blood_mist_fire_migrated {object.shift_remove("bloodMistFireRulesFrom");}
        half_here_lock_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("halfHereLockFrom").is_none_or(serde_json::Value::is_null);
        if half_here_lock_migrated {object.shift_remove("halfHereLockFrom");}
        steam_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64).is_some_and(|r|r>=50.)
            && object.get("steamRulesFrom").is_none_or(serde_json::Value::is_null);
        if steam_migrated {object.insert("steamRulesFrom".into(),serde_json::Value::Null);}
        execute_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("executeRulesFrom").is_none_or(serde_json::Value::is_null);
        if execute_migrated {object.insert("executeRulesFrom".into(),serde_json::Value::Null);}
        beckoning_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("beckoningRulesFrom").is_none_or(serde_json::Value::is_null);
        if beckoning_migrated {object.insert("beckoningRulesFrom".into(),serde_json::Value::Null);}
        ichor_fire_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("ichorFireRulesFrom").is_none_or(serde_json::Value::is_null);
        if ichor_fire_migrated {object.insert("ichorFireRulesFrom".into(),serde_json::Value::Null);}
        future_migrated=object.get("simulationRevision").and_then(serde_json::Value::as_f64)==Some(52.)
            && object.get("futureFloorRulesFrom").is_none_or(serde_json::Value::is_null);
        if future_migrated {object.insert("futureFloorRulesFrom".into(),serde_json::Value::Null);}
        if legacy{
            if let Some(marker)=object.get("replaySnapshotFrom"){
                let count=object.get("replay").and_then(serde_json::Value::as_array).map_or(0,Vec::len);
                // A boundary exactly at EOF has no snapshot-policy command yet.
                // A real snapshot prefix or unknown ancestry cannot become legacy.
                if marker.as_u64()!=Some(count as u64){return Err("Legacy recording header contradicts replay policy provenance".into())}
            }
        }else if !object.contains_key("replaySnapshotFrom"){
            object.insert("replaySnapshotFrom".into(),serde_json::Value::Null);
        }
    }
    let raw=serde_json::to_string(&run).map_err(|e|e.to_string())?;
    let mut engine=if legacy.is_some() {corpse_core::temporal::Engine::load_replay_checkpoint(&raw)}
        else {corpse_core::temporal::Engine::load(&raw)}.map_err(|e|e.to_string())?;
    if let Some(legacy)=legacy{engine.set_legacy_revert(legacy);}
    corpse_core::engine::validate_supported(&engine.run).map_err(|e|e.to_string())?;
    Ok((engine,kind,firebolt_migrated,amulet_migrated,potion_migrated,ring_migrated,gas_migrated,future_migrated,commandment_migrated,screamer_migrated,ember_migrated,ragelight_migrated,steam_migrated,execute_migrated,beckoning_migrated,ichor_fire_migrated,balance_migrated,collapse_migrated,blood_mist_fire_migrated,half_here_lock_migrated))
}

pub fn utc_now() -> String {
    utc_millis(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    )
}
fn utc_millis(ms: u64) -> String {
    let mut days = ms / 86_400_000;
    let mut year = 1970;
    let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    loop {
        let n = if leap(year) { 366 } else { 365 };
        if days < n {
            break;
        }
        days -= n;
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= months[month] {
        days -= months[month];
        month += 1;
    }
    let seconds = ms / 1000;
    format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        month + 1,
        days + 1,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
        ms % 1000
    )
}
#[cfg(test)]
mod envelope_tests {
    use super::*;
    #[test]
    fn provenance_survives_native_save_and_plain_source_import() {
        let raw = include_str!("../../../fixtures/avatar-room-v1.json");
        let engine = corpse_core::temporal::Engine::load(raw).unwrap();
        let canonical = engine.save().unwrap();
        let (restored, kind) = decode(&encode(&engine, Kind::Fixture).unwrap()).unwrap();
        assert_eq!(kind, Kind::Fixture);
        assert_eq!(restored.save().unwrap(), canonical);
        assert_eq!(decode(raw).unwrap().1, Kind::Genuine);
        assert!(decode("{\"nativeSaveVersion\":2,\"session\":\"fixture\",\"run\":{}}").is_err());
    }
    /// Reference path retained only in tests: O(H*S + B) time and memory.
    fn legacy_encode(engine: &corpse_core::temporal::Engine, kind: Kind) -> String {
        let run: serde_json::Value = serde_json::from_str(&engine.save().unwrap()).unwrap();
        serde_json::to_string(&SaveEnvelope {
            native_save_version: 1,
            session: kind,
            run,
        }).unwrap()
    }
    /// O(H*S) per case; compares exact bytes, including embedded inverse strings.
    #[test]
    fn typed_envelope_matches_legacy_bytes_at_zero_one_and_five_history_frames() {
        let mut engine = corpse_core::temporal::Engine::load(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        )).unwrap();
        for turns in 0..=5 {
            if [0, 1, 5].contains(&turns) {
                for kind in [Kind::Genuine, Kind::Fixture, Kind::Replay] {
                    let current = encode(&engine, kind).unwrap();
                    assert_eq!(current, legacy_encode(&engine, kind), "turns={turns}, kind={kind:?}");
                    let (restored, restored_kind) = decode(&current).unwrap();
                    assert_eq!(restored_kind, kind);
                    assert_eq!(restored.save().unwrap(), engine.save().unwrap());
                }
            }
            if turns < 5 { engine.dispatch(&corpse_core::Command::Wait).unwrap(); }
        }
        assert_eq!(engine.history.frames.len(), 5);
        // Numeric edge spellings must not be changed by eliminating parse/stringify.
        for value in [-0.0, 0.1, 1.2345678901234567, 1e-20, 1e20] {
            engine.run.player.conditions.insert("saveEnvelopeNumberProbe".into(), value);
            assert_eq!(encode(&engine, Kind::Fixture).unwrap(), legacy_encode(&engine, Kind::Fixture));
        }
    }
    #[test]
    fn utc_calendar_matches_epoch_and_leap_day() {
        assert_eq!(utc_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(utc_millis(1709164800123), "2024-02-29T00:00:00.123Z");
        assert_eq!(utc_millis(1577836800000), "2020-01-01T00:00:00.000Z");
    }
}

/// Autosave bookkeeping is process-local and never part of the simulation.
static SAVE_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn changed() {
    SAVE_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
#[derive(Default)]
pub struct Autosave {
    journal: Option<incremental_store::JournalState>,
    resume_checkpoint_guard: Option<recording_attachment::Attachment>,
    recording_lock: Option<(std::path::PathBuf,SaveLock)>,
    recording_caller: Option<std::path::PathBuf>,
    recording_stop: Option<String>,
    recording_recovery: Option<recording_attachment::IdentityWitness>,
    #[cfg(test)]
    recording_limits: Option<(usize,u64)>,
    tracked_last: Option<(tracked_engine::Token, u64, Kind, std::path::PathBuf)>,
    last: Option<(u64, String, Kind, std::path::PathBuf)>,
    verified: Option<(std::path::PathBuf, String, Kind)>,
    #[cfg(test)]
    verification_loads: usize,
    pub error: Option<String>,
}
impl Autosave {
    /// O(path bytes + canonical-directory metadata), retains one exclusive
    /// cooperating-writer lock until this Autosave switches path or is dropped.
    fn ensure_recording_lock(&mut self,path:&Path)->Result<(),String>{
        #[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.lock_ownership");
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or_else(||Path::new("."));
        let key=parent.canonicalize().map_err(|e|e.to_string())?.join(path.file_name().ok_or("Save path has no filename")?);
        if self.recording_lock.as_ref().is_some_and(|(p,_)|p==&key){if let Some(error)=&self.recording_stop{return Err(error.clone())}self.recording_caller=Some(path.to_path_buf());return Ok(())}
        if self.recording_lock.is_some()&&self.recording_caller.as_deref()==Some(path){let error="Recording directory changed behind the same save path; preserved until explicit reopen or a new save path.".to_string();self.recording_stop=Some(error.clone());return Err(error)}
        let lock=SaveLock::acquire(&key)?;
        self.journal=None;self.resume_checkpoint_guard=None;self.recording_recovery=None;self.verified=None;self.tracked_last=None;self.last=None;
        self.recording_stop=None;self.recording_caller=Some(path.to_path_buf());self.recording_lock=Some((key,lock));Ok(())
    }
    /// O(1) bounded attachment I/O; detected external changes latch before any
    /// fallback writer drops its handle or overwrites an otherwise valid file.
    fn check_recording_attachment(&mut self,path:&Path)->Result<(),String>{
        if let Some(guard)=self.resume_checkpoint_guard.as_mut(){if let Err(error)=guard.check(path){self.recording_stop=Some(error.clone());return Err(error)}}
        if let Some(recovery)=self.recording_recovery.as_ref(){if let Err(error)=recovery.check_identity(path){self.recording_stop=Some(error.clone());return Err(error)}}
        if let Some(state)=self.journal.as_mut(){if let Err(error)=state.attachment.check(path){self.recording_stop=Some(error.clone());self.journal=None;return Err(error)}}Ok(())
    }
    pub fn flush(
        &mut self,
        path: &Path,
        engine: &corpse_core::temporal::Engine,
        kind: Kind,
        force: bool,
    ) -> Result<(), String> {
        let _timing = corpse_core::timing::span("desktop.autosave_flush");
        self.tracked_last = None;
        if kind == Kind::Replay || !engine.run.playtest.is_missing() {
            return Ok(());
        }
        let key = (
            SAVE_REVISION.load(std::sync::atomic::Ordering::Relaxed),
            format!(
                "{}:{}:{}:{}",
                engine.run.seed,
                engine.run.depth,
                engine.run.tick,
                engine.run.replay.len()
            ),
            kind,
            path.to_path_buf(),
        );
        // Validate before replacement or terminal deletion; malformed files remain recoverable.
        let result = (|| {
            self.ensure_recording_lock(path)?;
            self.check_recording_attachment(path)?;
            if !force && self.last.as_ref()==Some(&key){return Ok(())}
            match read_recording_bytes(path) {
                Ok((raw,witness)) => {
                    self.recording_recovery=Some(witness);
                    let existing_kind =
                        if let Some((cached_path, cached_raw, cached_kind)) = &self.verified {
                            if cached_path == path && cached_raw.as_bytes() == raw {
                                Some(*cached_kind)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    let existing_kind = match existing_kind {
                        Some(kind) => kind,
                        None => {
                            #[cfg(test)]
                            {
                                self.verification_loads += 1;
                            }
                            let (_, loaded_kind) = decode_bytes(&raw).map_err(|e| {
                                format!(
                                    "Existing save was preserved because it could not be read: {e}"
                                )
                            })?;
                            self.verified = String::from_utf8(raw).ok().map(|raw|(path.to_path_buf(),raw,loaded_kind));
                            loaded_kind
                        }
                    };
                    if existing_kind != kind {
                        return Err("Existing save belongs to a different session type. Choose a separate --save path; existing bytes were preserved.".into());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.recording_recovery=Some(recording_attachment::IdentityWitness::absent());self.verified = None;
                }
                Err(e) => return Err(e.to_string()),
            }
            if engine.run.status == "dead" && !engine.run.dev && kind == Kind::Genuine && !unknown_seed_replay(&engine.run) {
                self.check_recording_attachment(path)?;if let Some(guard)=self.resume_checkpoint_guard.take(){self.recording_recovery=Some(guard.witness());}if let Some(state)=self.journal.take(){self.recording_recovery=Some(state.attachment.witness());}
                match std::fs::remove_file(path) {
                    Ok(()) => {
                        self.recording_recovery=None;self.verified = None;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        self.recording_recovery=None;self.verified = None;
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            } else {
                let raw = encode(engine, kind)?;
                self.check_recording_attachment(path)?;if let Some(guard)=self.resume_checkpoint_guard.take(){self.recording_recovery=Some(guard.witness());}if let Some(state)=self.journal.take(){self.recording_recovery=Some(state.attachment.witness());}
                atomic_write(path, raw.as_bytes())?;
                self.recording_recovery=None;
                self.verified = Some((path.to_path_buf(), raw, kind));
                Ok(())
            }
        })();
        // Failed writes/lock contention must remain retryable without force.
        self.last = if result.is_ok() { Some(key) } else { None };
        self.error = result.as_ref().err().cloned();
        result
    }
}

#[cfg(test)]
mod autosave_tests {
    use super::*;
    /// O(H*S + B) per attempted save; all filesystem paths are test-local.
    #[test]
    fn unchanged_key_skips_write_and_failed_temp_creation_preserves_destination() {
        let dir = std::env::temp_dir().join(format!("corpse-autosave-transaction-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        let mut engine = corpse_core::temporal::Engine::load(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        )).unwrap();
        let mut auto = Autosave::default();
        auto.flush(&path, &engine, Kind::Fixture, true).unwrap();
        let original = std::fs::read(&path).unwrap();
        // A missing target remains missing on an unchanged-key no-op. This tests
        // existing bookkeeping semantics, not a new external-file policy.
        std::fs::remove_file(&path).unwrap();
        auto.flush(&path, &engine, Kind::Fixture, false).unwrap();
        assert!(!path.exists());
        std::fs::write(&path, &original).unwrap();
        engine.dispatch(&corpse_core::Command::Wait).unwrap();
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        std::fs::write(&temp, "preexisting temporary file").unwrap();
        assert!(auto.flush(&path, &engine, Kind::Fixture, true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_to_string(&temp).unwrap(), "preexisting temporary file");
        std::fs::remove_file(&temp).unwrap();
        auto.flush(&path, &engine, Kind::Fixture, true).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert_eq!(decode(&raw).unwrap().0.save().unwrap(), engine.save().unwrap());
        assert!(auto.error.is_none());
        assert!(!temp.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn accepted_state_is_saved_and_terminal_death_removes_resume() {
        let dir = std::env::temp_dir().join(format!("corpse-autosave-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        let mut engine = corpse_core::temporal::Engine::load(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let mut auto = Autosave::default();
        auto.flush(&path, &engine, Kind::Genuine, true).unwrap();
        assert!(decode(&std::fs::read_to_string(&path).unwrap()).is_ok());
        assert_eq!(
            auto.verification_loads, 0,
            "our newly encoded bytes need no Engine reload"
        );
        auto.flush(&path, &engine, Kind::Genuine, true).unwrap();
        assert_eq!(
            auto.verification_loads, 0,
            "force rechecks exact bytes without reconstructing history"
        );
        engine.dispatch(&corpse_core::Command::Wait).unwrap();
        changed();
        auto.flush(&path, &engine, Kind::Genuine, false).unwrap();
        assert_eq!(
            decode(&std::fs::read_to_string(&path).unwrap())
                .unwrap()
                .0
                .save()
                .unwrap(),
            engine.save().unwrap()
        );
        assert_eq!(auto.verification_loads, 0);
        let saved = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, "externally corrupted same path").unwrap();
        changed();
        assert!(auto.flush(&path, &engine, Kind::Genuine, false).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "externally corrupted same path"
        );
        assert_eq!(auto.verification_loads, 1);
        std::fs::write(&path, &saved).unwrap();
        auto.flush(&path, &engine, Kind::Genuine, true).unwrap();
        assert!(auto.error.is_none());
        let alternate = dir.join("alternate.json");
        std::fs::write(&alternate, &saved).unwrap();
        auto.flush(&alternate, &engine, Kind::Genuine, false)
            .unwrap();
        assert_eq!(
            auto.verification_loads, 2,
            "different path must verify even when bytes match"
        );
        engine.run.status = "dead".into();
        changed();
        auto.flush(&path, &engine, Kind::Genuine, false).unwrap();
        assert!(!path.exists());
        auto.flush(&path, &engine, Kind::Genuine, true).unwrap();
        assert!(!path.exists());
        auto.flush(&path, &engine, Kind::Fixture, true).unwrap();
        assert_eq!(
            decode(&std::fs::read_to_string(&path).unwrap()).unwrap().1,
            Kind::Fixture
        );
        std::fs::write(&path, "broken original").unwrap();
        assert!(auto.flush(&path, &engine, Kind::Genuine, true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken original");
        assert!(auto.error.is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
#[cfg(test)]
mod debug_isolation_tests {
    use super::*;
    #[test]
    fn debug_never_replaces_continue_or_highscores_even_on_force_and_death() {
        let path=std::env::temp_dir().join(format!("corpse-debug-isolation-{}.json",std::process::id()));
        let mut normal=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut autosave=Autosave::default();autosave.flush(&path,&normal,Kind::Genuine,true).unwrap();let bytes=std::fs::read(&path).unwrap();
        let mut debug=corpse_core::temporal::Engine::new(corpse_core::debug_floor::create_run("0-5-11:debug-floor-regression","Avatar",1).unwrap()).unwrap();
        for terminal in ["playing","dead","won"] {debug.run.status=terminal.into();autosave.flush(&path,&debug,Kind::Genuine,true).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);assert!(prepare(&Profile::default(),&debug.run,Kind::Genuine,true,None).unwrap().is_none());}
        normal.dispatch(&corpse_core::Command::Wait).unwrap();autosave.flush(&path,&normal,Kind::Genuine,true).unwrap();assert_ne!(std::fs::read(&path).unwrap(),bytes);std::fs::remove_file(path).unwrap();
    }
}

#[path="recording_attachment.rs"]
mod recording_attachment;

#[path="recording_resume.rs"]
mod recording_resume;
/// Fully validated load with optional one-use same-recording resume authority.
pub fn load_tracked(path:&Path)->Result<(tracked_engine::Engine,Kind),String>{recording_resume::load(path)}
