use super::*;
use corpse_core::{Command, Field};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("corpse-r45-protocol-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap(); Self(path)
    }
}
impl Drop for Directory { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn fixture(revision: f64) -> tracked_engine::Engine {
    let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = revision.into(); run.history.clear(); run.haste_snapshot = Field::Missing;
    tracked_engine::Engine::new(run).unwrap()
}

#[test]
fn revision45_exact_header_pairs_reject_cross_revision_and_cross_grammar() {
    for revision in [39., 40., 41., 42., 43., 44., 45.] {
        let old = log::Header::for_revision(Some(revision)).unwrap();
        let metadata = log::Header::for_metadata_revision(Some(revision)).unwrap();
        assert!(!old.metadata_operations()); assert!(metadata.metadata_operations());
        for header in [&old, &metadata] {
            assert!(!header.legacy_revert_policy().unwrap());
            for wrong in [39, 40, 41, 42, 43, 44, 45, 46] {
                if wrong != revision as u32 {
                    let mut mismatch = header.clone(); mismatch.simulation_revision = wrong;
                    assert!(mismatch.legacy_revert_policy().is_err());
                }
            }
        }
        assert!(parse_operations(r#"{"version":2,"operations":[]}"#, false).is_err());
        assert!(parse_operations(r#"{"receipts":[]}"#, true).is_err());
    }
    assert_eq!(log::Header::for_revision(Some(45.)).unwrap().replay_build, "corpse-r45-nova-energy-v1");
    assert_eq!(log::Header::for_metadata_revision(Some(45.)).unwrap().replay_build, "corpse-r45-nova-energy-metadata-v2");
    for revision in [None, Some(45.5), Some(47.), Some(f64::NAN), Some(f64::INFINITY)] {
        assert!(log::Header::for_revision(revision).is_none());
        assert!(log::Header::for_metadata_revision(revision).is_none());
    }
    let legacy = log::Header { replay_build: log::LEGACY_REPLAY_BUILD_ID.into(), simulation_revision: 39 };
    assert!(legacy.legacy_revert_policy().unwrap());
}

#[test]
fn revision45_and_old_clean_recordings_resume_and_append_exact_metadata() {
    for revision in [41., 42., 43., 44., 45.] {
        let dir = Directory::new(); let path = dir.0.join("save.json"); let engine = fixture(revision);
        Autosave::default().flush_tracked(&path, &engine, Kind::Fixture, false).unwrap();
        let before = std::fs::read(&path).unwrap();
        let (mut loaded, kind) = super::super::load_tracked(&path).unwrap();
        assert!(!loaded.pending_batch().checkpoint_required);
        let mut writer = Autosave::default();
        writer.flush_tracked(&path, &loaded, kind, true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let replay = loaded.run.replay.clone(); let rng = loaded.run.rng;
        loaded.set_inventory_display_order(Field::Null).unwrap();
        loaded.set_run_id(Field::Value("revision45-protocol".into())).unwrap();
        assert_eq!(loaded.run.replay, replay); assert_eq!(loaded.run.rng, rng);
        loaded.dispatch(&Command::Wait).unwrap();
        writer.flush_tracked(&path, &loaded, kind, false).unwrap();
        let bytes = std::fs::read(&path).unwrap(); assert!(bytes.starts_with(&before));
        assert_eq!(log::inspect_header(&bytes).unwrap().simulation_revision, revision as u32);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&decoded.run).unwrap(), serde_json::to_vec(&loaded.run).unwrap());
    }
}

#[test]
fn revision45_headers_cannot_relabel_old_checkpoints_or_use_unknown_build_ids() {
    for (checkpoint_revision, header_revision) in [(44.,45.), (45.,44.)] {
        let engine = fixture(checkpoint_revision);
        let checkpoint = super::super::encode(&engine, Kind::Fixture).unwrap();
        for header in [log::Header::for_revision(Some(header_revision)).unwrap(),
            log::Header::for_metadata_revision(Some(header_revision)).unwrap()] {
            let initial = log::encode_initial(&checkpoint, &header).unwrap();
            assert!(decode(&initial.bytes).err().unwrap().contains("original simulation revision"));
        }
    }
    for metadata in [false, true] {
        let header = if metadata { log::Header::for_metadata_revision(Some(45.)) }
            else { log::Header::for_revision(Some(45.)) }.unwrap();
        let mut unknown = header.clone(); unknown.replay_build.push_str("-unknown");
        assert!(unknown.legacy_revert_policy().is_err());
        assert!(!unknown.metadata_operations());
    }
}

#[test]
fn revision45_command_receipt_roundtrips_without_revision_upgrade() {
    let mut engine = fixture(45.);
    let checkpoint = super::super::encode(&engine, Kind::Fixture).unwrap();
    let header = log::Header::for_revision(Some(45.)).unwrap();
    let initial = log::encode_initial(&checkpoint, &header).unwrap();
    assert!(engine.acknowledge_persisted(engine.pending_batch().token));
    engine.dispatch(&Command::Wait).unwrap();
    let receipts = engine.pending_batch().receipts; assert_eq!(receipts.len(), 1);
    let payload = serde_json::to_string(&Batch { receipts }).unwrap();
    let appended = log::encode_append(initial.next_sequence, initial.last_checksum, &payload).unwrap();
    let mut bytes = initial.bytes; bytes.extend_from_slice(&appended.bytes);
    let (restored, kind) = decode(&bytes).unwrap();
    assert_eq!(kind, Kind::Fixture);
    assert_eq!(restored.run.simulation_revision.value_or(0.), 45.);
    assert_eq!(restored.save().unwrap(), engine.save().unwrap());
}
