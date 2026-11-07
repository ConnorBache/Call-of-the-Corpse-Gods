use super::*;
use corpse_core::{Command, Field};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(1);
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let p=std::env::temp_dir().join(format!("corpse-metadata-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&p).unwrap();Self(p)
    }
}
impl Drop for Directory {fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn fixture(revision:f64)->corpse_core::temporal::Engine {
    let mut old=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();old.run.simulation_revision=revision.into();old.run.history.clear();old.run.haste_snapshot=Field::Missing;old.run.inventory_display_order=Field::Missing;old.run.run_id=Field::Missing;
    corpse_core::temporal::Engine::new(old.run).unwrap()
}
#[test]
fn metadata_only_order_and_identity_append_under_new_exact_header(){
    for revision in [39.,40.,41.,42.]{
        let dir=Directory::new();let path=dir.0.join("session.save");let mut e=tracked_engine::Engine::from_core(fixture(revision));let mut a=Autosave::default();
        a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let before=std::fs::read(&path).unwrap();let header=log::inspect_header(&before).unwrap();assert_eq!(header,log::Header::for_metadata_revision(Some(revision)).unwrap());
        let replay=e.run.replay.clone();let rng=e.run.rng;let tick=e.run.tick;
        let ids:Vec<_>=e.run.inventory.iter().rev().map(|i|i.id).collect();e.set_inventory_display_order(ids.clone().into()).unwrap();e.ensure_run_id(||"standalone-metadata".into()).unwrap();
        a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let bytes=std::fs::read(&path).unwrap();assert!(bytes.starts_with(&before));
        let parsed=log::parse(&bytes,&header,|s|parse_operations(s,true).map(|_|())).unwrap();assert_eq!(parsed.records.len(),2);assert_eq!(parse_operations(parsed.records[1].payload,true).unwrap().len(),2);
        let (loaded,_)=decode(&bytes).unwrap();assert_eq!(loaded.run.inventory_display_order,Field::Value(ids));assert_eq!(loaded.run.run_id,Field::Value("standalone-metadata".into()));assert_eq!(loaded.run.replay,replay);assert_eq!(loaded.run.rng,rng);assert_eq!(loaded.run.tick,tick);
        assert!(e.pending_batch().operations.is_empty());let token=e.token();assert!(!e.set_run_id("standalone-metadata".to_string().into()).unwrap());assert_eq!(e.token(),token);a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);
    }
}
#[test]
fn old_receipt_grammar_is_unchanged_and_header_mixing_rejected(){
    let mut e=fixture(39.);let checkpoint=super::super::encode(&e,Kind::Fixture).unwrap();e.dispatch(&Command::Wait).unwrap();let receipt=Receipt{command:Command::Wait,after:tracked_engine::Stamp::capture(&e)};
    let old=serde_json::to_string(&Batch{receipts:vec![receipt.clone()]}).unwrap();let new=serde_json::to_string(&OperationBatch{version:2,operations:vec![receipt.into()]}).unwrap();
    for (header,payload,valid) in [(log::Header::current(),&old,true),(log::Header::for_metadata_revision(Some(39.)).unwrap(),&new,true),(log::Header::current(),&new,false),(log::Header::for_metadata_revision(Some(39.)).unwrap(),&old,false)]{
        let mut initial=log::encode_initial(&checkpoint,&header).unwrap();let next=log::encode_append(initial.next_sequence,initial.last_checksum,payload).unwrap();initial.bytes.extend_from_slice(&next.bytes);
        if valid{assert_eq!(decode(&initial.bytes).unwrap().0.save().unwrap(),e.save().unwrap());}else{assert!(decode(&initial.bytes).is_err());}
    }
    for revision in [39.,40.,41.,42.]{let new=log::Header::for_metadata_revision(Some(revision)).unwrap();assert!(new.metadata_operations());assert!(!new.legacy_revert_policy().unwrap());for wrong in [39,40,41,42]{if wrong!=new.simulation_revision{let mut h=new.clone();h.simulation_revision=wrong;assert!(h.legacy_revert_policy().is_err());}}}
}
#[test]
fn metadata_parser_rejects_unknown_fields_wrong_types_and_bounds(){
    let after=tracked_engine::Stamp::capture(&fixture(41.));let base=serde_json::json!({"version":2,"operations":[{"kind":"inventoryDisplayOrder","value":{"state":"ids","ids":[1.,2.]},"after":after}]});
    assert!(parse_operations(&base.to_string(),true).is_ok());
    let mut bad=Vec::new();let mut v=base.clone();v["extra"]=true.into();bad.push(v);let mut v=base.clone();v["version"]=1.into();bad.push(v);
    let mut v=base.clone();v["operations"][0]["extra"]=true.into();bad.push(v);let mut v=base.clone();v["operations"][0]["value"]["extra"]=true.into();bad.push(v);
    let mut v=base.clone();v["operations"][0]["value"]=serde_json::json!({"state":"missing","ids":[]});bad.push(v);
    let mut v=base.clone();v["operations"][0]["value"]["ids"]=serde_json::json!(["1"]);bad.push(v);
    let mut v=base.clone();v["operations"][0]["value"]["ids"]=serde_json::json!(vec![1.;150_000]);bad.push(v);
    let mut v=base.clone();v["operations"][0]["kind"]="runId".into();v["operations"][0]["value"]=serde_json::json!({"state":"null"});bad.push(v);
    let mut v=base.clone();v["operations"][0]=serde_json::json!({"kind":"command","command":{"type":"wait","extra":1},"after":tracked_engine::Stamp::capture(&fixture(41.))});bad.push(v);
    for v in bad{assert!(parse_operations(&v.to_string(),true).is_err(),"malformed operation accepted: {v}");}
    assert!(parse_operations(r#"{"version":2,"version":2,"operations":[]}"#,true).is_err());
}

/// Real Future capture observes exact metadata restored after the checkpoint and
/// after a clean same-file resume. No seed-ledger entry is created for metadata.
#[test]
fn exact_order_survives_initial_checkpoint_resume_and_future_capture(){
    for revision in [41.,42.] {
    for order in [Field::Missing,Field::Null,Field::Value(vec![999_999.,999_999.,-0.])]{
        for initial in [false,true]{
            let dir=Directory::new();let path=dir.0.join("future.save");
            let mut core=fixture(revision);
            core.run.player.abilities=vec!["morae-see-the-future".into()];
            core.run.player.active.clear();core.run.player.conditions.clear();
            core.run.player.energy=100.;core.run.player.stats.max_energy=100.;
            core.run.phase="normal".to_string().into();core.run.future=None;
            let mut e=tracked_engine::Engine::from_core(core);let mut save=Autosave::default();
            if initial {e.set_inventory_display_order(order.clone()).unwrap();}
            save.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();
            if !initial {e.set_inventory_display_order(order.clone()).unwrap();save.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();}
            let before=std::fs::read(&path).unwrap();let expected=e.save().unwrap();drop(save);
            let (mut resumed,kind)=super::super::recording_resume::load(&path).unwrap();
            assert_eq!(resumed.save().unwrap(),expected);assert_eq!(resumed.run.inventory_display_order,order);
            let prior_replay=resumed.run.replay.len();
            let command=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
            resumed.dispatch(&command).unwrap();assert!(resumed.run.future.is_some());assert_eq!(resumed.run.replay.len(),prior_replay+1);
            let mut save=Autosave::default();save.flush_tracked(&path,&resumed,kind,false).unwrap();
            let after=std::fs::read(&path).unwrap();assert!(after.starts_with(&before));assert!(after.len()>before.len());
            let (loaded,_)=decode(&after).unwrap();assert_eq!(loaded.save().unwrap(),resumed.save().unwrap());
            assert_eq!(loaded.run.replay,resumed.run.replay);
        }
    }
}
}

include!("mixed_metadata_fault_tests.rs");
