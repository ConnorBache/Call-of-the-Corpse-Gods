//! Protocol checks preserve legacy framing and R50 exact-pair identity.
//! The integration gate also uses actual R50 generation and admitted engine resume.
use super::*;
use corpse_core::{Command, Field};

// Deliberately a framing fixture, not a valid playable Run or saved-game claim.
const CHECKPOINT: &str = r#"{"nativeSaveVersion":1,"session":"fixture","run":{"simulationRevision":50.0,"text":"quote\" slash\\ snowman ☃"}}"#;
fn stamp() -> tracked_engine::Stamp {
    tracked_engine::Stamp {
        rng_bits: 1_f64.to_bits(), tick_bits: 0_f64.to_bits(), depth_bits: 1_f64.to_bits(),
        replay_len: 0, phase: Field::Value("normal".into()), status: "playing".into(), history_frames: 0,
    }
}
fn metadata_payload() -> String {
    use tracked_engine::{OrderValue, RunIdValue};
    serde_json::to_string(&OperationBatch { version: 2, operations: vec![
        Operation::InventoryDisplayOrder { value: OrderValue::Missing, after: stamp() },
        Operation::InventoryDisplayOrder { value: OrderValue::Null, after: stamp() },
        Operation::InventoryDisplayOrder { value: OrderValue::Ids(vec![999_999., 999_999., -0.]), after: stamp() },
        Operation::RunId { value: RunIdValue::Missing, after: stamp() },
        Operation::RunId { value: RunIdValue::Text("r50 ☃ quote\" slash\\".into()), after: stamp() },
        Operation::Command { command: Command::Wait, after: stamp() },
    ] }).unwrap()
}
fn receipt_payload() -> String {
    serde_json::to_string(&Batch { receipts: vec![Receipt { command: Command::Wait, after: stamp() }] }).unwrap()
}
fn parse_metadata<'a>(bytes: &'a [u8], header: &log::Header) -> Result<log::Parsed<'a>, String> {
    log::parse(bytes, header, |raw| parse_operations(raw, header.metadata_operations()).map(|_| ()))
}

#[test]
fn revision50_protocol_exact_pairs_preserve_every_historical_identity() {
    let expected = [
        (39., "corpse-r39-snapshot-overwrite-v1", "corpse-r39-snapshot-metadata-v2"),
        (40., "corpse-r40-shore-snapshot-overwrite-v1", "corpse-r40-shore-metadata-v2"),
        (41., "corpse-r41-timeweave-v1", "corpse-r41-timeweave-metadata-v2"),
        (42., "corpse-r42-forgiving-start-v1", "corpse-r42-forgiving-start-metadata-v2"),
        (43., "corpse-r43-item-action-v1", "corpse-r43-item-action-metadata-v2"),
        (44., "corpse-r44-bleed-v1", "corpse-r44-bleed-metadata-v2"),
        (45., "corpse-r45-nova-energy-v1", "corpse-r45-nova-energy-metadata-v2"),
        (46., "corpse-r46-shine-suppression-v1", "corpse-r46-shine-suppression-metadata-v2"),
        (50., "corpse-r50-ability-roster-v1", "corpse-r50-ability-roster-metadata-v2"),
        (51., "corpse-r51-nearest-damage-v1", "corpse-r51-nearest-damage-metadata-v2"),
    ];
    for (revision, receipt_id, metadata_id) in expected {
        let receipt = log::Header::for_revision(Some(revision)).unwrap();
        let metadata = log::Header::for_metadata_revision(Some(revision)).unwrap();
        assert_eq!(receipt.replay_build, receipt_id);
        assert_eq!(metadata.replay_build, metadata_id);
        assert_eq!(receipt.simulation_revision, revision as u32);
        assert_eq!(metadata.simulation_revision, revision as u32);
        assert!(!receipt.metadata_operations());
        assert!(metadata.metadata_operations());
        for header in [&receipt, &metadata] {
            assert!(!header.legacy_revert_policy().unwrap());
            let initial = log::encode_initial(CHECKPOINT, header).unwrap();
            assert_eq!(log::inspect_header(&initial.bytes).unwrap(), *header);
            for wrong in [0, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, u32::MAX] {
                if wrong == revision as u32 { continue; }
                let mismatch = log::Header { simulation_revision: wrong, ..header.clone() };
                assert!(mismatch.legacy_revert_policy().is_err());
                assert!(!mismatch.metadata_operations());
                assert!(parse_metadata(&initial.bytes, &mismatch).is_err());
                // Pair selection rejects before trying to decode this dummy checkpoint.
                let mismatched = log::encode_initial(CHECKPOINT, &mismatch).unwrap();
                assert!(decode(&mismatched.bytes).err().unwrap().contains("unsupported replay build/simulation revision pair"));
            }
            let unknown = log::Header { replay_build: format!("{}-unknown", header.replay_build), ..header.clone() };
            assert!(unknown.legacy_revert_policy().is_err());
            assert!(!unknown.metadata_operations());
        }
    }
    assert_eq!(log::Header::current().simulation_revision, 39);
    assert_eq!(log::Header::current().replay_build, "corpse-r39-snapshot-overwrite-v1");
    let legacy = log::Header { replay_build: "corpse-r39-incremental-v1".into(), simulation_revision: 39 };
    assert!(legacy.legacy_revert_policy().unwrap());
    assert!(!legacy.metadata_operations());
    for revision in [None, Some(38.), Some(47.), Some(48.), Some(49.), Some(52.), Some(49.5), Some(50.5), Some(f64::NAN), Some(f64::INFINITY), Some(f64::NEG_INFINITY)] {
        assert!(log::Header::for_revision(revision).is_none());
        assert!(log::Header::for_metadata_revision(revision).is_none());
    }
}

#[test]
fn revision50_protocol_framing_preserves_metadata_and_record_bytes() {
    let header = log::Header::for_metadata_revision(Some(50.)).unwrap();
    let payload = metadata_payload();
    let first = log::encode_initial(CHECKPOINT, &header).unwrap();
    let second = log::encode_append(first.next_sequence, first.last_checksum, &payload).unwrap();
    let third = log::encode_append(second.next_sequence, second.last_checksum, &payload).unwrap();
    let mut bytes = first.bytes.clone();
    bytes.extend_from_slice(&second.bytes); bytes.extend_from_slice(&third.bytes);
    let parsed = parse_metadata(&bytes, &header).unwrap();
    assert_eq!(parsed.header, header);
    assert_eq!(parsed.checkpoint.as_bytes(), CHECKPOINT.as_bytes());
    assert_eq!(parsed.records.len(), 2);
    for (i, record) in parsed.records.iter().enumerate() {
        assert_eq!(record.sequence, i as u64 + 1);
        assert_eq!(record.payload.as_bytes(), payload.as_bytes());
        let operations = parse_operations(record.payload, true).unwrap();
        let roundtrip = serde_json::to_string(&OperationBatch { version: 2, operations: operations.clone() }).unwrap();
        assert_eq!(roundtrip, payload);
        let Operation::InventoryDisplayOrder { value: tracked_engine::OrderValue::Ids(ids), .. } = &operations[2] else { panic!("ordered metadata changed") };
        assert_eq!(ids[2].to_bits(), (-0_f64).to_bits());
        let start = record.payload.as_ptr() as usize;
        assert!(start >= bytes.as_ptr() as usize && start + record.payload.len() <= bytes.as_ptr() as usize + bytes.len());
    }
    assert_eq!(parsed.metadata.valid_len, bytes.len());
    assert_eq!(parsed.metadata.next_sequence, 3);
    assert_eq!(parsed.metadata.last_checksum, third.last_checksum);
    assert_eq!(parsed.metadata.record_count, 2);
    assert!(!parsed.metadata.recovered_tail);
}

#[test]
fn revision50_protocol_receipts_and_metadata_never_mix_grammars() {
    for (header, accepted, rejected) in [
        (log::Header::for_revision(Some(50.)).unwrap(), receipt_payload(), metadata_payload()),
        (log::Header::for_metadata_revision(Some(50.)).unwrap(), metadata_payload(), receipt_payload()),
    ] {
        let initial = log::encode_initial(CHECKPOINT, &header).unwrap();
        for (payload, valid) in [(&accepted, true), (&rejected, false)] {
            let appended = log::encode_append(initial.next_sequence, initial.last_checksum, payload).unwrap();
            let mut bytes = initial.bytes.clone(); bytes.extend_from_slice(&appended.bytes);
            assert_eq!(parse_metadata(&bytes, &header).is_ok(), valid);
        }
    }
    let header = log::Header::for_metadata_revision(Some(50.)).unwrap();
    let initial = log::encode_initial(CHECKPOINT, &header).unwrap();
    for payload in [r#"{"version":2,"operations":[]}"#, r#"{"version":2,"operations":[{"kind":"unknown"}]}"#, "{"] {
        let appended = log::encode_append(initial.next_sequence, initial.last_checksum, payload).unwrap();
        let mut bytes = initial.bytes.clone(); bytes.extend_from_slice(&appended.bytes);
        assert!(parse_metadata(&bytes, &header).unwrap_err().contains("command batch"));
    }
}

/// O(P²) time over every cut of a small fixed-size payload P; production parsing
/// remains linear in bytes plus operation validation. No simulation is performed.
#[test]
fn revision50_protocol_every_torn_tail_keeps_last_committed_metadata_record() {
    let header = log::Header::for_metadata_revision(Some(50.)).unwrap();
    let payload = metadata_payload();
    let first = log::encode_initial(CHECKPOINT, &header).unwrap();
    let second = log::encode_append(first.next_sequence, first.last_checksum, &payload).unwrap();
    let third = log::encode_append(second.next_sequence, second.last_checksum, &payload).unwrap();
    let mut bytes = first.bytes;
    bytes.extend_from_slice(&second.bytes);
    let committed_len = bytes.len(); bytes.extend_from_slice(&third.bytes);
    for cut in committed_len..bytes.len() {
        let mut calls = 0;
        let parsed = log::parse(&bytes[..cut], &header, |raw| {
            calls += 1; parse_operations(raw, true).map(|_| ())
        }).unwrap_or_else(|e| panic!("cut {cut}: {e}"));
        assert_eq!(calls, 1);
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].payload, payload);
        assert_eq!(parsed.metadata.valid_len, committed_len);
        assert_eq!(parsed.metadata.next_sequence, 2);
        assert_eq!(parsed.metadata.last_checksum, second.last_checksum);
        assert_eq!(parsed.metadata.recovered_tail, cut != committed_len);
    }
    assert_eq!(parse_metadata(&bytes, &header).unwrap().records.len(), 2);
    for index in committed_len..bytes.len() {
        let mut corrupt = bytes.clone(); corrupt[index] ^= 1;
        assert!(parse_metadata(&corrupt, &header).is_err(), "corrupt byte {index}");
    }
}

#[test]
fn revision50_protocol_header_cannot_relabel_an_admitted_legacy_checkpoint() {
    let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = 46.0.into(); run.history.clear(); run.haste_snapshot = Field::Missing;
    let engine = tracked_engine::Engine::new(run).unwrap();
    let checkpoint = super::super::encode(&engine, Kind::Fixture).unwrap();
    for header in [log::Header::for_revision(Some(50.)).unwrap(), log::Header::for_metadata_revision(Some(50.)).unwrap()] {
        let initial = log::encode_initial(&checkpoint, &header).unwrap();
        assert!(decode(&initial.bytes).err().unwrap().contains("original simulation revision"));
    }
}

/// Native release gate: actual R50 startup, metadata recording and clean resume.
#[test]
fn revision50_protocol_integration_clean_resume_appends_without_replacing_recording() {
    let run = corpse_core::startup::create_run_revision(
        "0-5-11:revision50-protocol-resume", "Unblessed", false, 50.,
    ).unwrap();
    let engine = tracked_engine::Engine::new(run).unwrap();
    let directory = std::env::temp_dir().join(format!("corpse-r50-protocol-resume-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    let _cleanup = Cleanup(directory.clone());
    let path = directory.join("save.json");
    Autosave::default().flush_tracked(&path, &engine, Kind::Fixture, false).unwrap();
    let before = std::fs::read(&path).unwrap();
    let witness = super::super::recording_attachment::IdentityWitness::from_file(&std::fs::File::open(&path).unwrap()).unwrap();
    assert_eq!(log::inspect_header(&before).unwrap(), log::Header::for_metadata_revision(Some(50.)).unwrap());
    let (mut loaded, kind) = super::super::load_tracked(&path).unwrap();
    assert!(!loaded.pending_batch().checkpoint_required);
    let mut writer = Autosave::default();
    writer.flush_tracked(&path, &loaded, kind, true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    witness.check_identity(&path).unwrap();
    let replay = loaded.run.replay.clone(); let rng = loaded.run.rng;
    loaded.set_inventory_display_order(Field::Null).unwrap();
    loaded.set_run_id(Field::Value("revision50-protocol".into())).unwrap();
    assert_eq!(loaded.run.replay, replay); assert_eq!(loaded.run.rng, rng);
    loaded.dispatch(&Command::Wait).unwrap();
    writer.flush_tracked(&path, &loaded, kind, false).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(&before)); assert!(bytes.len() > before.len());
    witness.check_identity(&path).unwrap();
    let (decoded, _) = decode(&bytes).unwrap();
    assert_eq!(decoded.run.simulation_revision, Field::Value(50.));
    assert_eq!(decoded.save().unwrap(), loaded.save().unwrap());
}
