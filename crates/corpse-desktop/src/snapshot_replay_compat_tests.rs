// Compatibility-policy tests, included inside incremental_store::tests.
/// O(fixture world + three turn simulations), no user saves touched.
fn legacy_revert_fixture()->CoreEngine {
    let mut e=core();
    for a in e.run.actors.values_mut(){*a=std::sync::Arc::new(Vec::new());}
    e.run.player.hp=100_000.;e.run.player.max_hp=100_000.;e.run.player.base_max_hp=corpse_core::Field::Value(100_000.);
    e.run.player.energy=100.;e.run.player.stats.max_energy=100.;
    e.run.player.abilities=vec!["numo-revert".into()];e.run.player.active.clear();e.run.player.conditions.clear();e.run.offers.clear();
    corpse_core::progression::initialize_ability_state(&mut e.run.player,"numo-revert",e.run.tick).unwrap();
    e.run.player.conditions.insert("revertCharges".into(),20.);
    e.set_legacy_revert(true);
    for _ in 0..2{assert!(e.dispatch(&Command::Wait).unwrap().took_turn);}
    e
}
/// O(checkpoint size + tail simulations), builds genuine old-policy receipts.
fn legacy_file_with_revert()->(Vec<u8>,CoreEngine){
    let prepared=legacy_revert_fixture();let checkpoint=encode(&prepared,Kind::Fixture).unwrap();
    let(mut oracle,_)=super::super::decode_legacy(&checkpoint).unwrap();oracle.set_legacy_revert(true);
    let header=log::Header{replay_build:log::LEGACY_REPLAY_BUILD_ID.into(),simulation_revision:log::SIMULATION_REVISION};
    let mut encoded=log::encode_initial(&checkpoint,&header).unwrap();
    for command in [Command::Wait,Command::Ability{id:"numo-revert".into(),target:corpse_core::Field::Missing,x:corpse_core::Field::Missing,y:corpse_core::Field::Missing}]{
        oracle.dispatch(&command).unwrap();
        let payload=serde_json::to_string(&Batch{receipts:vec![Receipt{command,after:tracked_engine::Stamp::capture(&oracle)}]}).unwrap();
        let next=log::encode_append(encoded.next_sequence,encoded.last_checksum,&payload).unwrap();
        encoded.bytes.extend_from_slice(&next.bytes);encoded.next_sequence=next.next_sequence;encoded.last_checksum=next.last_checksum;
    }
    (encoded.bytes,oracle)
}
#[test]
fn old_revert_tail_replays_exact_legacy_state_then_switches_policy(){
    let(raw,mut legacy)=legacy_file_with_revert();let(mut restored,kind)=decode(&raw).unwrap();assert_eq!(kind,Kind::Fixture);
    // Migration adds only the explicit policy boundary to the old ledger.
    assert_eq!(restored.run.replay_snapshot_from,corpse_core::Field::Value(legacy.run.replay.len()));
    legacy.run.replay_snapshot_from=restored.run.replay_snapshot_from.clone();
    // Full canonical current state, not the deliberately incomplete replay Stamp.
    assert_eq!(serde_json::to_value(&restored.run).unwrap(),serde_json::to_value(&legacy.run).unwrap());
    assert_eq!(restored.history.frames.len(),legacy.history.frames.len());
    for (a,b) in restored.history.frames.iter().zip(&legacy.history.frames){assert_eq!(a.value().unwrap(),b.value().unwrap());}
    assert_eq!(restored.history.closing.as_ref().unwrap().value().unwrap(),legacy.history.closing.as_ref().unwrap().value().unwrap());
    let old_hp=restored.history.frames.back().unwrap().state.player.hp;
    assert_eq!(legacy.history.closing.as_ref().unwrap().state.player.hp,old_hp);
    restored.run.player.hp=17.;legacy.run.player.hp=17.;
    assert_eq!(tracked_engine::Stamp::capture(&restored),tracked_engine::Stamp::capture(&legacy));
    assert!(corpse_core::temporal::revert(&mut restored.run,&mut restored.history).unwrap());
    assert!(corpse_core::temporal::revert(&mut legacy.run,&mut legacy.history).unwrap());
    assert_eq!(restored.run.player.hp,old_hp);assert_eq!(legacy.run.player.hp,17.);
}
#[test]
fn migrated_old_file_checkpoints_new_build_before_new_commands(){
    let(raw,_)=legacy_file_with_revert();let dir=Sandbox::new("legacy-policy-migration");let path=dir.path();std::fs::write(&path,&raw).unwrap();
    let(core,kind)=decode(&raw).unwrap();let mut e=tracked_engine::Engine::from_core(core);let mut autosave=Autosave::default();
    e.dispatch(&Command::Pickup).unwrap();autosave.flush_tracked(&path,&e,kind,false).unwrap();
    let next=std::fs::read(&path).unwrap();assert!(!next.starts_with(&raw));
    assert_eq!(log::inspect_header(&next).unwrap(),log::Header::for_metadata_revision(Some(39.)).unwrap());assert_eq!(records(&path),0);assert_disk(&path,&e,kind);
    let old=log::Header{replay_build:log::LEGACY_REPLAY_BUILD_ID.into(),simulation_revision:log::SIMULATION_REVISION};
    assert!(log::parse(&next,&old,|_|Ok(())).is_err());
    // Migration owns the cooperative lock for the entire recording attachment.
    let mut second=Autosave::default();assert!(second.flush_tracked(&path,&e,kind,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),next);
    drop(autosave);second.flush_tracked(&path,&e,kind,true).unwrap();assert_disk(&path,&e,kind);
    assert_eq!(log::inspect_header(&std::fs::read(&path).unwrap()).unwrap(),log::Header::for_metadata_revision(Some(39.)).unwrap());
}
#[test]
fn compatibility_selection_does_not_accept_unknown_build_or_corruption(){
    let(raw,_)=legacy_file_with_revert();let mut bad=raw.clone();*bad.last_mut().unwrap()^=1;assert!(decode(&bad).is_err());
    let checkpoint=encode(&legacy_revert_fixture(),Kind::Fixture).unwrap();let unknown=log::Header{replay_build:"unknown-build".into(),simulation_revision:log::SIMULATION_REVISION};
    assert!(decode(&log::encode_initial(&checkpoint,&unknown).unwrap().bytes).is_err());
    let truncated=&raw[..raw.len()-1];let(restored,_)=decode(truncated).unwrap();assert_eq!(restored.history.frames.len(),3);
    let duplicate=b"CCG-INCREMENTAL-SAVE/1\n{\"replayBuild\":\"corpse-r39-incremental-v1\",\"replayBuild\":\"corpse-r39-snapshot-overwrite-v1\",\"simulationRevision\":39}\n";
    assert!(log::inspect_header(duplicate).is_err());
}
