use super::*;
use corpse_core::{Command, Field};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("corpse-r43-protocol-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
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
fn revision43_exact_header_pairs_reject_cross_revision_and_cross_grammar() {
    for revision in [39., 40., 41., 42., 43.] {
        let old = log::Header::for_revision(Some(revision)).unwrap();
        let metadata = log::Header::for_metadata_revision(Some(revision)).unwrap();
        assert!(!old.metadata_operations()); assert!(metadata.metadata_operations());
        for header in [&old, &metadata] {
            assert!(!header.legacy_revert_policy().unwrap());
            for wrong in [39, 40, 41, 42, 43, 44] {
                if wrong != revision as u32 {
                    let mut mismatch = header.clone(); mismatch.simulation_revision = wrong;
                    assert!(mismatch.legacy_revert_policy().is_err());
                }
            }
        }
        assert!(parse_operations(r#"{"version":2,"operations":[]}"#, false).is_err());
        assert!(parse_operations(r#"{"receipts":[]}"#, true).is_err());
    }
    assert_eq!(log::Header::for_revision(Some(43.)).unwrap().replay_build, "corpse-r43-item-action-v1");
    assert_eq!(log::Header::for_metadata_revision(Some(43.)).unwrap().replay_build, "corpse-r43-item-action-metadata-v2");
    let legacy = log::Header { replay_build: log::LEGACY_REPLAY_BUILD_ID.into(), simulation_revision: 39 };
    assert!(legacy.legacy_revert_policy().unwrap());
}

#[test]
fn revision43_and_old_clean_recordings_resume_and_append_exact_metadata() {
    for revision in [41., 42., 43.] {
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
        loaded.set_run_id(Field::Value("revision43-protocol".into())).unwrap();
        assert_eq!(loaded.run.replay, replay); assert_eq!(loaded.run.rng, rng);
        loaded.dispatch(&Command::Wait).unwrap();
        writer.flush_tracked(&path, &loaded, kind, false).unwrap();
        let bytes = std::fs::read(&path).unwrap(); assert!(bytes.starts_with(&before));
        assert_eq!(log::inspect_header(&bytes).unwrap().simulation_revision, revision as u32);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&decoded.run).unwrap(), serde_json::to_vec(&loaded.run).unwrap());
    }
}
