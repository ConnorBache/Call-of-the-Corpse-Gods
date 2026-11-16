use super::*;
use crate::session_store::{self,Autosave};
use corpse_core::{Command,Field};
use std::sync::atomic::{AtomicU64,Ordering};
static NEXT:AtomicU64=AtomicU64::new(1);
struct Sandbox(PathBuf);
impl Sandbox{fn new()->Self{let p=std::env::temp_dir().join(format!("corpse-resume-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));std::fs::create_dir(&p).unwrap();Self(p)}fn path(&self)->PathBuf{self.0.join("save.json")}}
impl Drop for Sandbox{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn fixture()->Engine{
 let mut run=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 run.simulation_revision=41.0.into();run.history.clear();run.haste_snapshot=Field::Missing;
 Engine::new(run).unwrap()
}
fn initial(path:&Path)->Vec<u8>{let e=fixture();Autosave::default().flush_tracked(path,&e,Kind::Fixture,true).unwrap();std::fs::read(path).unwrap()}
fn equal_loaded(path:&Path,e:&Engine){let(restored,_)=session_store::decode_bytes(&std::fs::read(path).unwrap()).unwrap();assert_eq!(restored.save().unwrap(),e.save().unwrap());}
#[test]fn clean_resume_no_write_and_postload_metadata_command_append(){
 let dir=Sandbox::new();let path=dir.path();let before=initial(&path);
 let witness=IdentityWitness::from_file(&std::fs::File::open(&path).unwrap()).unwrap();
 let(mut e,k)=load(&path).unwrap();assert!(!e.pending_batch().checkpoint_required);
 let mut auto=Autosave::default();auto.flush_tracked(&path,&e,k,true).unwrap();
 assert_eq!(std::fs::read(&path).unwrap(),before);witness.check_identity(&path).unwrap();
 e.set_inventory_display_order(Field::Value(vec![])).unwrap();e.dispatch(&Command::Wait).unwrap();
 auto.flush_tracked(&path,&e,k,false).unwrap();let after=std::fs::read(&path).unwrap();
 assert!(after.starts_with(&before));assert!(after.len()>before.len());witness.check_identity(&path).unwrap();equal_loaded(&path,&e);
}
#[test]fn postload_operations_before_first_attach_are_not_acknowledged_away(){
 let dir=Sandbox::new();let path=dir.path();let before=initial(&path);let(mut e,k)=load(&path).unwrap();
 e.set_inventory_display_order(Field::Null).unwrap();e.dispatch(&Command::Wait).unwrap();
 Autosave::default().flush_tracked(&path,&e,k,false).unwrap();
 assert!(std::fs::read(&path).unwrap().starts_with(&before));assert!(e.pending_batch().operations.is_empty());equal_loaded(&path,&e);
}
#[test]fn replacement_or_truncation_after_load_latches_without_overwrite(){
 for replace in [false,true]{let dir=Sandbox::new();let path=dir.path();let before=initial(&path);let(e,k)=load(&path).unwrap();
 if replace{let other=dir.0.join("other");std::fs::write(&other,&before).unwrap();std::fs::rename(other,&path).unwrap();}else{std::fs::write(&path,&before[..before.len()-1]).unwrap();}
 let changed=std::fs::read(&path).unwrap();let mut auto=Autosave::default();
 assert!(auto.flush_tracked(&path,&e,k,false).is_err());assert!(auto.flush_tracked(&path,&e,k,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),changed);
 }}
#[test]fn clone_newpath_and_unknown_mutation_require_independent_root(){
 let dir=Sandbox::new();let path=dir.path();initial(&path);let(e,k)=load(&path).unwrap();
 assert!(e.clone().pending_batch().checkpoint_required);
 let newpath=dir.0.join("new.json");Autosave::default().flush_tracked(&newpath,&e,k,false).unwrap();equal_loaded(&newpath,&e);
 let(mut e,k)=load(&path).unwrap();e.run.metrics.insert("resumeAdminProbe".into(),1.0);assert!(e.pending_batch().checkpoint_required);
 Autosave::default().flush_tracked(&path,&e,k,false).unwrap();equal_loaded(&path,&e);
}
#[test]fn old_header_and_torn_tail_remain_explicit_checkpoint_migration(){
 for torn in [false,true]{let dir=Sandbox::new();let path=dir.path();let e=fixture();
 let raw=session_store::encode(&e,Kind::Fixture).unwrap();let header=if torn{log::Header::for_metadata_revision(Some(41.0)).unwrap()}else{log::Header::for_revision(Some(41.0)).unwrap()};
 let mut bytes=log::encode_initial(&raw,&header).unwrap().bytes;if torn{bytes.extend_from_slice(b"B 000");}std::fs::write(&path,&bytes).unwrap();
 let(e,k)=load(&path).unwrap();assert!(e.pending_batch().checkpoint_required);
 Autosave::default().flush_tracked(&path,&e,k,false).unwrap();let saved=std::fs::read(&path).unwrap();
 assert!(log::inspect_header(&saved).unwrap().metadata_operations());equal_loaded(&path,&e);
 }}
#[test]fn loaded_normalization_is_the_authority_not_predecode_object(){
 let dir=Sandbox::new();let path=dir.path();let mut e=fixture();
 // Legacy import prunes non-inventory display IDs. Resume must agree with the
 // fully decoded checkpoint, not certify the arbitrary pre-serialization Run.
 e.run.inventory_display_order=vec![999999.0].into();let raw=session_store::encode(&e,Kind::Fixture).unwrap();
 let bytes=log::encode_initial(&raw,&log::Header::for_metadata_revision(Some(41.0)).unwrap()).unwrap().bytes;std::fs::write(&path,&bytes).unwrap();
 let(mut loaded,k)=load(&path).unwrap();let(decoded,_)=session_store::decode_bytes(&bytes).unwrap();assert_eq!(loaded.save().unwrap(),decoded.save().unwrap());
 loaded.dispatch(&Command::Wait).unwrap();Autosave::default().flush_tracked(&path,&loaded,k,false).unwrap();equal_loaded(&path,&loaded);
}

#[test]fn unknown_edit_does_not_forget_loaded_source_replacement_guard(){
 let dir=Sandbox::new();let path=dir.path();let before=initial(&path);let(mut e,k)=load(&path).unwrap();
 e.run.metrics.insert("resumeAdminProbe".into(),1.0);
 let other=dir.0.join("replacement");std::fs::write(&other,&before).unwrap();std::fs::rename(other,&path).unwrap();
 let mut auto=Autosave::default();assert!(auto.flush_tracked(&path,&e,k,false).is_err());
 assert!(auto.flush_tracked(&path,&e,k,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),before);
}

#[test]fn cloned_loaded_engine_keeps_conflict_guard_without_append_authority(){
 let dir=Sandbox::new();let path=dir.path();let before=initial(&path);let(e,k)=load(&path).unwrap();let cloned=e.clone();
 assert!(cloned.pending_batch().checkpoint_required);assert!(cloned.pending_batch().base_token.is_none());
 let other=dir.0.join("replacement");std::fs::write(&other,&before).unwrap();std::fs::rename(other,&path).unwrap();
 let mut auto=Autosave::default();assert!(auto.flush_tracked(&path,&cloned,k,false).is_err());
 assert!(auto.flush_tracked(&path,&cloned,k,true).is_err());assert_eq!(std::fs::read(&path).unwrap(),before);
}

#[test]fn migrated_disk_sources_keep_conflict_guard_before_first_checkpoint(){
 for mode in ["old","torn","plain"]{
  let dir=Sandbox::new();let path=dir.path();let e=fixture();let raw=session_store::encode(&e,Kind::Fixture).unwrap();
  let bytes=match mode{
   "plain"=>raw.into_bytes(),
   "old"=>log::encode_initial(&raw,&log::Header::for_revision(Some(41.0)).unwrap()).unwrap().bytes,
   _=>{let mut b=log::encode_initial(&raw,&log::Header::for_metadata_revision(Some(41.0)).unwrap()).unwrap().bytes;b.extend_from_slice(b"B 000");b},
  };
  std::fs::write(&path,&bytes).unwrap();let(loaded,k)=load(&path).unwrap();assert!(loaded.pending_batch().checkpoint_required);assert!(loaded.pending_batch().base_token.is_none());
  let other=dir.0.join("replacement");std::fs::write(&other,&bytes).unwrap();std::fs::rename(other,&path).unwrap();
  let mut auto=Autosave::default();assert!(auto.flush_tracked(&path,&loaded,k,false).is_err(),"{mode}");
  assert!(auto.flush_tracked(&path,&loaded,k,true).is_err(),"{mode}");assert_eq!(std::fs::read(&path).unwrap(),bytes);
 }
}

include!("revision42_resume_tests.rs");

/// Protocol eligibility is separate from the Engine/storage admission gate.
#[test]
fn revision50_resume_header_admission_preserves_legacy_boundaries() {
    for revision in [39., 40., 41., 42., 43., 44., 45., 46., 50., 51., 52.] {
        let receipt = log::Header::for_revision(Some(revision)).unwrap();
        let metadata = log::Header::for_metadata_revision(Some(revision)).unwrap();
        assert!(!resumable_header(&receipt));
        assert_eq!(resumable_header(&metadata), revision >= 41.);
        for wrong in [39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52] {
            if wrong != revision as u32 {
                let mismatch = log::Header { simulation_revision: wrong, ..metadata.clone() };
                assert!(!resumable_header(&mismatch));
            }
        }
    }
    let legacy = log::Header { replay_build: log::LEGACY_REPLAY_BUILD_ID.into(), simulation_revision: 39 };
    assert!(!resumable_header(&legacy));
    let unknown = log::Header { replay_build: "corpse-r50-ability-roster-metadata-v2-unknown".into(), simulation_revision: 50 };
    assert!(!resumable_header(&unknown));
}

/// Pre-rename actor payloads enter as already-authored R50 state, not through
/// a new-run roundtrip. Original CRC bytes stay exact across admission/append.
#[test]
fn revision50_creature_identity_old_checkpoint_tail_future_and_resume() {
    let dir=Sandbox::new();let path=dir.path();
    let mut run=crate::render::creature_identity_tests::room();
    run.next_id=1000.;run.replay.clear();run.history.clear();run.haste_snapshot=Field::Missing;run.future=None;
    for (actor,kind) in run.actors_at_mut(1.).iter_mut().zip(["blorb","bear","dire-bear","bastidion","pale:Biter","pale:Breather","hemogoblin-inventor","yeti","samara"]) {
        let frozen=&corpse_core::population::catalog().actors[kind];
        actor.name=frozen.name.clone();actor.glyph=frozen.glyph.clone();actor.color=frozen.color.clone();
    }
    let from=corpse_core::timeweave::Cell{floor:1,plane:corpse_core::timeweave::Plane::Waking,tile:2};
    let to=corpse_core::timeweave::Cell{tile:3,..from};let mut trail=corpse_core::timeweave::Trail::default();
    trail.append(corpse_core::timeweave::Segment{cell:from,from,to});run.timeweave=std::sync::Arc::new(trail).into();
    let anchor=corpse_core::storage::serialize(&run).unwrap();run.future=Some(anchor.into());
    let old=Engine::new(run).unwrap();let checkpoint=session_store::encode(&old,Kind::Fixture).unwrap();
    assert!(checkpoint.contains("Pale Biter"));assert!(checkpoint.contains("Bastidion"));assert!(!checkpoint.contains("Pale Chomper"));
    let header=log::Header::for_metadata_revision(Some(50.)).unwrap();let initial=log::encode_initial(&checkpoint,&header).unwrap();
    // A historical metadata receipt is insensitive to cosmetic actor identities.
    let stamp=crate::session_store::tracked_engine::Stamp::capture(&old);
    let tail=serde_json::json!({"version":2,"operations":[{"kind":"inventoryDisplayOrder","value":{"state":"null"},"after":stamp}]}).to_string();
    let batch=log::encode_append(initial.next_sequence,initial.last_checksum,&tail).unwrap();
    let mut original=initial.bytes;original.extend(batch.bytes);std::fs::write(&path,&original).unwrap();
    let (mut loaded,kind)=load(&path).unwrap();assert_eq!(kind,Kind::Fixture);
    assert_eq!(loaded.run.actors_at(1.)[3].name,"Corpse Fungus");assert_eq!(loaded.run.actors_at(1.)[4].name,"Pale Chomper");
    assert_eq!(loaded.run.actors_at(1.)[0].glyph,"b");assert_eq!(loaded.run.actors_at(1.)[2].color,"#000000");
    assert_eq!(loaded.run.timeweave.as_ref().unwrap().len(),1);
    let future=loaded.run.future.as_ref().unwrap();assert_eq!(future.native_state().unwrap().actors_at(1.)[4].name,"Pale Chomper");
    let mut autosave=Autosave::default();autosave.flush_tracked(&path,&loaded,kind,true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(),original,"admission cannot rewrite original CRC payloads");
    loaded.dispatch(&Command::Wait).unwrap();autosave.flush_tracked(&path,&loaded,kind,false).unwrap();
    let appended=std::fs::read(&path).unwrap();assert!(appended.starts_with(&original));assert!(appended.len()>original.len());
    equal_loaded(&path,&loaded);
    let parsed=log::parse(&appended,&header,|_|Ok(())).unwrap();assert_eq!(parsed.checkpoint,checkpoint);
}

/// Captured by the independently built, verified pre-rename R50 executable.
/// The external fixture stays immutable; only a temporary copy is appended.
#[test]
#[ignore = "requires CORPSE_PRERENAME_FIXTURES with verified old-source capture"]
fn revision50_creature_identity_authentic_old_source_recording_and_future() {
    let fixtures=PathBuf::from(std::env::var_os("CORPSE_PRERENAME_FIXTURES").expect("set CORPSE_PRERENAME_FIXTURES to the verified old-source fixture directory"));
    let read=|name:&str,expected_crc:u32|{
        let bytes=std::fs::read(fixtures.join(name)).unwrap();
        assert_eq!(crc32fast::hash(&bytes),expected_crc,"immutable pre-rename fixture changed: {name}");bytes
    };
    let original=read("old-incremental-recording.json",0xa5bf4f9c);
    let final_raw=String::from_utf8(read("old-final-run.json",0x6c9c1c47)).unwrap();
    let restored_raw=String::from_utf8(read("old-restored-future-run.json",0xbf21cd57)).unwrap();
    assert!(final_raw.contains("Pale Biter"));assert!(final_raw.contains("Bastidion"));assert!(!final_raw.contains("Pale Chomper"));
    let expected=Engine::load(&final_raw).unwrap();
    let exact=|actual:&Engine,expected:&Engine,label:&str|{
        let actual:serde_json::Value=serde_json::from_str(&actual.save().unwrap()).unwrap();
        let expected:serde_json::Value=serde_json::from_str(&expected.save().unwrap()).unwrap();
        let fields:Vec<_>=actual.as_object().unwrap().keys().chain(expected.as_object().unwrap().keys()).filter(|key|actual.get(*key)!=expected.get(*key)).collect();
        assert!(actual==expected,"{label}: differing top-level fields {fields:?}");
    };
    let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&original).unwrap();
    let (mut loaded,kind)=load(&path).unwrap();assert_eq!(kind,Kind::Fixture);
    assert!(!loaded.pending_batch().checkpoint_required);assert_eq!(loaded.run.tick,2.);
    exact(&loaded,&expected,"real old two-Wait tail replay");
    for (id,name,glyph,color) in [
        (247.,"Blorb","b","#28703c"),(248.,"Bear","B","#a47c55"),(249.,"Dire Bear","B","#000000"),
        (250.,"Corpse Fungus","F","#9fc875"),(251.,"Pale Chomper","c","#b9b8b1"),
        (252.,"Greater Pale Chomper","C","#f0e8dc"),(253.,"Pale Gasser","h","#b9b8b1"),
        (254.,"Greater Pale Gasser","H","#f0e8dc"),(255.,"Hemogoblin Inventor","h","#d600ff"),
        (256.,"Yeti","Y","#D7E8F1"),(257.,"Devil's Samara Tree","T","#8a5a32"),
    ] {
        let actor=loaded.run.actors_at(1.).iter().find(|actor|actor.id==id).unwrap();
        assert_eq!((&*actor.name,&*actor.glyph,&*actor.color),(name,glyph,color));
    }
    let future=loaded.run.future.as_ref().unwrap().native_state().unwrap();
    assert_eq!(future.actors_at(1.).iter().find(|actor|actor.id==251.).unwrap().name,"Pale Chomper");
    let mut restored=loaded.clone();restored.dispatch(&Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();
    assert!(restored.run.future.is_none());
    exact(&restored,&Engine::load(&restored_raw).unwrap(),"real old Future restoration");
    let mut autosave=Autosave::default();autosave.flush_tracked(&path,&loaded,kind,true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(),original,"read-only resume must preserve original CRC bytes");
    loaded.dispatch(&Command::Wait).unwrap();autosave.flush_tracked(&path,&loaded,kind,false).unwrap();
    let appended=std::fs::read(&path).unwrap();assert!(appended.starts_with(&original));assert!(appended.len()>original.len());equal_loaded(&path,&loaded);
    assert_eq!(std::fs::read(fixtures.join("old-incremental-recording.json")).unwrap(),original);
}
