//! Real journal replay/migration and same-file resume, isolated from user saves.
use super::*;
use crate::session_store::{self, Autosave, tracked_engine::{Operation, OrderValue, Stamp}};
use corpse_core::{Actor, Command, Field, Run, Stats, temporal::Engine as Core};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT:AtomicU64=AtomicU64::new(1);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new()->Self {
        let path=std::env::temp_dir().join(format!("corpse-firebolt-journal-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();Self(path)
    }
    fn path(&self)->PathBuf {self.0.join("save.json")}
}
impl Drop for Sandbox {fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn room()->Run {
    let mut r=crate::staged_firebolt::tests::room();
    r.firebolt_travel_from=Field::Null;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("floor".into());
    f.tiles[10*w+13]="tree".into();f.tiles[10*w+19]="tree".into();f.tiles[10*w+25]="wall".into();
    let mut a=Actor {id:950.,x:24.,y:10.,name:"Giant Rat".into(),glyph:"e".into(),color:"#fff".into(),
        hp:500.,max_hp:500.,base_max_hp:500.0.into(),energy:100.,state:"hunting".into(),
        stats:Stats {max_energy:100.,..Default::default()},..Default::default()};
    a.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(a);
    // Start with the same ordinary checkpoint normalization as production load.
    Core::load(&Core::new(r).unwrap().save().unwrap()).unwrap().run
}
fn shot()->Command {Command::UseAt{id:900.,x:11.,y:10.}}
fn hp(r:&Run)->f64 {corpse_core::engine::actor(r,950.).unwrap().hp}
fn tile(r:&Run,x:usize)->&str {let f=r.floor().unwrap();&f.tiles[10*f.width()+x]}
fn append(file:&mut log::Encoded,operations:Vec<Operation>,metadata:bool) {
    let payload=if metadata {serde_json::json!({"version":2,"operations":operations})} else {
        let receipts:Vec<_>=operations.into_iter().map(|operation|match operation {
            Operation::Command{command,after}=>crate::session_store::tracked_engine::Receipt{command,after},
            _=>panic!("legacy receipt grammar does not carry metadata"),
        }).collect();serde_json::json!({"receipts":receipts})
    };
    let next=log::encode_append(file.next_sequence,file.last_checksum,&payload.to_string()).unwrap();
    file.bytes.extend_from_slice(&next.bytes);file.next_sequence=next.next_sequence;file.last_checksum=next.last_checksum;
}
/// The checkpoint contains a non-turn command prefix, and metadata is not a
/// command. Adoption must use the final replay length, never tick/record counts.
fn journal(marker:Field<usize>,metadata:bool,tail:bool)->(Run,Core,Vec<u8>) {
    let initial=room();let mut old=Core::new(initial.clone()).unwrap();
    for _ in 0..2 {old.dispatch(&Command::Pickup).unwrap();}
    if !marker.is_missing() {old.run.firebolt_travel_from=marker.clone();}
    let mut checkpoint:serde_json::Value=serde_json::from_str(&session_store::encode(&old,Kind::Fixture).unwrap()).unwrap();
    if marker.is_missing() {checkpoint["run"].as_object_mut().unwrap().remove("fireboltTravelFrom");}
    let header=if metadata {log::Header::for_metadata_revision(Some(52.))} else {log::Header::for_revision(Some(52.))}.unwrap();
    let mut file=log::encode_initial(&checkpoint.to_string(),&header).unwrap();
    if metadata {
        let order=Operation::InventoryDisplayOrder{value:OrderValue::from(old.run.inventory_display_order.clone()),after:Stamp::capture(&old)};
        append(&mut file,vec![order],true);
    }
    if tail {
        old.dispatch(&shot()).unwrap();
        let after=Stamp::capture(&old);append(&mut file,vec![Operation::Command{command:shot(),after}],metadata);
    }
    (initial,old,file.bytes)
}
fn export_twice(initial:&Run,source:&Core) {
    use corpse_core::replay::{ReplayRecord,ReplaySession};
    let record=ReplayRecord::try_from_run(&source.run).unwrap();
    let record=serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
    let mut replay=ReplaySession::from_initial(initial.clone(),record).unwrap();
    for restart in [false,true] {
        if restart {replay.restart().unwrap();}
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),source.save().unwrap());
    }
}
#[test]
fn firebolt_markerless_journal_replays_old_tail_then_checkpoints_new_boundary() {
    for metadata in [false,true] {
        let (initial,mut expected,bytes)=journal(Field::Missing,metadata,true);
        assert_eq!(hp(&expected.run),500.);assert_eq!(tile(&expected.run,13),"floor");assert_eq!(tile(&expected.run,19),"tree");
        let boundary=expected.run.replay.len();assert_eq!(boundary,3);assert_ne!(boundary as f64,expected.run.tick);
        expected.run.firebolt_travel_from=boundary.into();
        let (decoded,kind)=session_store::decode_bytes(&bytes).unwrap();
        assert_eq!(kind,Kind::Fixture);assert_eq!(decoded.save().unwrap(),expected.save().unwrap());
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&bytes).unwrap();
        let (mut loaded,kind)=load(&path).unwrap();
        assert!(loaded.pending_batch().checkpoint_required);assert!(loaded.pending_batch().base_token.is_none());
        assert_eq!(std::fs::read(&path).unwrap(),bytes,"load must not rewrite the old recording");
        // A new-policy command before the first flush must not append behind the old root.
        loaded.dispatch(&shot()).unwrap();expected.dispatch(&shot()).unwrap();
        assert!(hp(&loaded.run)<500.);assert_eq!(tile(&loaded.run,19),"floor");
        assert_eq!(loaded.save().unwrap(),expected.save().unwrap());
        let mut auto=Autosave::default();auto.flush_tracked(&path,&loaded,kind,false).unwrap();
        let rewritten=std::fs::read(&path).unwrap();assert!(!rewritten.starts_with(&bytes));
        let header=log::inspect_header(&rewritten).unwrap();
        let parsed=log::parse(&rewritten,&header,|_|Ok(())).unwrap();
        let root:serde_json::Value=serde_json::from_str(parsed.checkpoint).unwrap();
        assert_eq!(root["run"]["fireboltTravelFrom"],serde_json::json!(boundary));
        let (reopened,_)=session_store::decode_bytes(&rewritten).unwrap();assert_eq!(reopened.save().unwrap(),loaded.save().unwrap());
        export_twice(&initial,&reopened);
        let (manual,_)=session_store::decode(&session_store::encode(&reopened,kind).unwrap()).unwrap();
        assert_eq!(manual.save().unwrap(),reopened.save().unwrap());
        drop(auto);
        let (mut resumed,kind)=load(&path).unwrap();assert!(!resumed.pending_batch().checkpoint_required);
        resumed.dispatch(&Command::Pickup).unwrap();
        Autosave::default().flush_tracked(&path,&resumed,kind,false).unwrap();
        let appended=std::fs::read(&path).unwrap();assert!(appended.starts_with(&rewritten));
        let (final_state,_)=session_store::decode_bytes(&appended).unwrap();assert_eq!(final_state.save().unwrap(),resumed.save().unwrap());
        export_twice(&initial,&final_state);
    }
}
#[test]
fn firebolt_explicit_journal_policies_keep_exact_semantics_and_append_authority() {
    for marker in [Field::Null,Field::Value(0),Field::Value(2)] {
        let (initial,expected,bytes)=journal(marker.clone(),true,true);
        let (decoded,_,migrated)=super::super::incremental_store::decode_with_migration(&bytes).unwrap();
        assert!(!migrated);assert_eq!(decoded.run.firebolt_travel_from,marker);
        assert_eq!(decoded.save().unwrap(),expected.save().unwrap());
        assert_eq!(tile(&decoded.run,19),if matches!(marker,Field::Null){"tree"}else{"floor"});
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&bytes).unwrap();
        let (mut loaded,kind)=load(&path).unwrap();assert!(!loaded.pending_batch().checkpoint_required);
        loaded.dispatch(&Command::Pickup).unwrap();
        Autosave::default().flush_tracked(&path,&loaded,kind,false).unwrap();
        let appended=std::fs::read(&path).unwrap();assert!(appended.starts_with(&bytes));
        let (reopened,_)=session_store::decode_bytes(&appended).unwrap();assert_eq!(reopened.save().unwrap(),loaded.save().unwrap());
    }
}
#[test]
fn firebolt_markerless_empty_or_torn_tail_adopts_only_the_durable_command_prefix() {
    for tail in [false,true] {
        let (_,mut expected,mut bytes)=journal(Field::Missing,true,tail);
        // A final uncommitted header never adds a command or changes adoption.
        bytes.extend_from_slice(b"B 000");
        let boundary=expected.run.replay.len();expected.run.firebolt_travel_from=boundary.into();
        let (decoded,_,migrated)=super::super::incremental_store::decode_with_migration(&bytes).unwrap();
        assert!(migrated);assert_eq!(decoded.save().unwrap(),expected.save().unwrap());
    }
}

#[test]
fn amulet_markerless_journal_keeps_old_roots_damage_then_adopts_current_rules() {
    for metadata in [false,true] {
        let mut initial=room();initial.firebolt_travel_from=0usize.into();initial.amulet_rules_from=Field::Null;
        initial.floor_mut().unwrap().tiles.fill("floor".into());
        let depth=initial.depth;let a=initial.actors_at_mut(depth).iter_mut().find(|a|a.id==950.).unwrap();a.x=14.;
        let item=initial.inventory.iter_mut().find(|i|i.id==900.).unwrap();
        item.effect="roots".to_string().into();item.name="Tier 1 Amulet of Roots".into();item.amulet_tier=1.0.into();
        let mut old=Core::new(initial.clone()).unwrap();
        let mut checkpoint:serde_json::Value=serde_json::from_str(&session_store::encode(&old,Kind::Fixture).unwrap()).unwrap();
        checkpoint["run"].as_object_mut().unwrap().remove("amuletRulesFrom");
        let header=if metadata {log::Header::for_metadata_revision(Some(52.))} else {log::Header::for_revision(Some(52.))}.unwrap();
        let mut file=log::encode_initial(&checkpoint.to_string(),&header).unwrap();
        let command=Command::UseAt{id:900.,x:14.,y:10.};
        old.dispatch(&command).unwrap();
        assert!(hp(&old.run)<500.,"historical roots damage must execute in the durable tail");
        let after=Stamp::capture(&old);append(&mut file,vec![Operation::Command{command,after}],metadata);
        old.run.amulet_rules_from=old.run.replay.len().into();
        let (decoded,_)=session_store::decode_bytes(&file.bytes).unwrap();
        assert_eq!(decoded.save().unwrap(),old.save().unwrap());
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&file.bytes).unwrap();
        let (loaded,_)=load(&path).unwrap();
        assert!(loaded.pending_batch().checkpoint_required);
        assert!(loaded.pending_batch().base_token.is_none());
    }
}

#[test]
fn potion_markerless_journal_keeps_retired_flooding_then_adopts_current_rules() {
    for metadata in [false,true] {
        let mut initial=room();initial.firebolt_travel_from=0usize.into();initial.potion_rules_from=Field::Null;
        initial.floor_mut().unwrap().tiles.fill("floor".into());
        initial.inventory.push(corpse_core::Item{id:901.,kind:"potion".into(),glyph:"!".into(),name:"Potion of Flooding".into(),effect:"flooding".to_string().into(),identified:true.into(),..Default::default()});
        let mut old=Core::new(initial.clone()).unwrap();
        let mut checkpoint:serde_json::Value=serde_json::from_str(&session_store::encode(&old,Kind::Fixture).unwrap()).unwrap();
        checkpoint["run"].as_object_mut().unwrap().remove("potionRulesFrom");
        let header=if metadata {log::Header::for_metadata_revision(Some(52.))} else {log::Header::for_revision(Some(52.))}.unwrap();
        let mut file=log::encode_initial(&checkpoint.to_string(),&header).unwrap();
        let command=Command::Throw{id:901.,x:14.,y:10.};
        old.dispatch(&command).unwrap();
        assert_eq!(tile(&old.run,14),"shallow-water","historical Flooding must execute in the durable tail");
        let after=Stamp::capture(&old);append(&mut file,vec![Operation::Command{command,after}],metadata);
        old.run.potion_rules_from=old.run.replay.len().into();
        let (decoded,_)=session_store::decode_bytes(&file.bytes).unwrap();
        assert_eq!(decoded.save().unwrap(),old.save().unwrap());
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&file.bytes).unwrap();
        let (loaded,_)=load(&path).unwrap();
        assert!(loaded.pending_batch().checkpoint_required);
        assert!(loaded.pending_batch().base_token.is_none());
    }
}

#[test]
fn ring_markerless_journal_keeps_old_resistance_then_adopts_current_rules() {
    for metadata in [false,true] {
        let mut initial=room();initial.firebolt_travel_from=0usize.into();initial.ring_rules_from=Field::Null;
        initial.inventory.push(corpse_core::Item{id:901.,kind:"ring".into(),glyph:"=".into(),name:"Ring of Poison Resistance".into(),effect:"poison-resistance".to_string().into(),identified:true.into(),..Default::default()});
        initial.player.conditions.insert("poisoned".into(),5.);
        initial.player.conditions.insert("r50PoisonDamage".into(),6.);
        assert_eq!(corpse_core::revision50_rings::resist_damage(&initial,&initial.player,"poison",6.),3.);
        let mut current=initial.clone();current.ring_rules_from=0usize.into();
        let mut current=Core::new(current).unwrap();current.dispatch(&Command::Wait).unwrap();
        let mut old=Core::new(initial.clone()).unwrap();
        let mut checkpoint:serde_json::Value=serde_json::from_str(&session_store::encode(&old,Kind::Fixture).unwrap()).unwrap();
        checkpoint["run"].as_object_mut().unwrap().remove("ringRulesFrom");
        let header=if metadata {log::Header::for_metadata_revision(Some(52.))} else {log::Header::for_revision(Some(52.))}.unwrap();
        let mut file=log::encode_initial(&checkpoint.to_string(),&header).unwrap();
        let command=Command::Wait;old.dispatch(&command).unwrap();
        assert!(old.run.player.hp>current.run.player.hp,"historical resistance must affect the durable tail's poison tick");
        let after=Stamp::capture(&old);append(&mut file,vec![Operation::Command{command,after}],metadata);
        old.run.ring_rules_from=old.run.replay.len().into();
        let (decoded,_)=session_store::decode_bytes(&file.bytes).unwrap();
        assert_eq!(decoded.save().unwrap(),old.save().unwrap());
        assert_eq!(corpse_core::revision50_rings::resist_damage(&decoded.run,&decoded.run.player,"poison",6.),6.);
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&file.bytes).unwrap();
        let (loaded,_)=load(&path).unwrap();
        assert!(loaded.pending_batch().checkpoint_required);
        assert!(loaded.pending_batch().base_token.is_none());
    }
}

#[test]
fn gas_markerless_journal_keeps_old_confusion_then_adopts_five_turn_refresh() {
    for metadata in [false,true] {
        let mut initial=room();initial.firebolt_travel_from=0usize.into();initial.gas_rules_from=Field::Null;
        let f=initial.floor().unwrap();let at=initial.player.y as usize*f.width()+initial.player.x as usize;
        corpse_core::environment::add_gas(initial.floor_mut().unwrap(),at,"confusion",100.,-1.).unwrap();
        let mut current=initial.clone();current.gas_rules_from=0usize.into();
        let mut current=Core::new(current).unwrap();current.dispatch(&Command::Wait).unwrap();
        let mut old=Core::new(initial.clone()).unwrap();
        let mut checkpoint:serde_json::Value=serde_json::from_str(&session_store::encode(&old,Kind::Fixture).unwrap()).unwrap();
        checkpoint["run"].as_object_mut().unwrap().remove("gasRulesFrom");
        let header=if metadata {log::Header::for_metadata_revision(Some(52.))} else {log::Header::for_revision(Some(52.))}.unwrap();
        let mut file=log::encode_initial(&checkpoint.to_string(),&header).unwrap();
        let command=Command::Wait;old.dispatch(&command).unwrap();
        assert!(corpse_core::engine::condition(&old.run.player,"confused")>corpse_core::engine::condition(&current.run.player,"confused"),"durable tail must retain historical eight-turn confusion");
        let after=Stamp::capture(&old);append(&mut file,vec![Operation::Command{command,after}],metadata);
        old.run.gas_rules_from=old.run.replay.len().into();
        let (decoded,_)=session_store::decode_bytes(&file.bytes).unwrap();
        assert_eq!(decoded.save().unwrap(),old.save().unwrap());
        assert!(corpse_core::gas_rules::current(&decoded.run));
        export_twice(&initial,&decoded);
        let dir=Sandbox::new();let path=dir.path();std::fs::write(&path,&file.bytes).unwrap();
        let (loaded,_)=load(&path).unwrap();
        assert!(loaded.pending_batch().checkpoint_required);
        assert!(loaded.pending_batch().base_token.is_none());
    }
}
