//! One headless trace through the real desktop command/pack ownership helpers.
//! Timings are single diagnostic samples, not latency percentiles or frame timings.
use super::*;
use std::{path::{Path,PathBuf},time::Instant};
struct PlaySandbox(PathBuf);
impl Drop for PlaySandbox {
    /// O(test-owned directory entries and bytes).
    fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}
}
/// O(fixture bytes), preserves known catalog actors/items and room geometry.
fn play_engine()->temporal::Engine {
    let cases:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/combat-starters-v48.json")).unwrap();
    let mut value=cases.as_array().unwrap().iter().find(|c|c["kind"]=="number").unwrap()["initial"].clone();
    value["simulationRevision"]=39.into();value["player"]["abilities"]=serde_json::json!([]);value["player"]["abilityOrder"]=serde_json::json!([]);
    value["player"]["stats"]["accuracy"]=1000.into();
    value["actors"]["1"][0]["conditions"]["paralyzed"]=100.into();
    value["actors"]["1"][0]["stats"]["defense"]=0.into();value["actors"]["1"][0]["stats"]["dodge"]=0.into();
    let mut bow=value["inventory"].as_array_mut().unwrap().remove(1);
    bow["id"]=10000.into();value["nextId"]=10001.into();
    value["ground"]["1"]=serde_json::json!([{"x":12,"y":14,"item":bow}]);
    value.as_object_mut().unwrap().remove("inventoryDisplayOrder");
    temporal::Engine::new(serde_json::from_value(value).unwrap()).unwrap()
}
/// O(save bytes+replay/history): verifies actual disk persistence after each input.
fn persist_step(label:&str,engine:&temporal::Engine,auto:&mut session_store::Autosave,path:&Path,checkpoint:bool,command_ms:f64,rows:&mut Vec<serde_json::Value>){
    let pending=engine.pending_batch();assert_eq!(pending.checkpoint_required,checkpoint,"{label}: ownership classification");
    let before=std::fs::read(path).unwrap();
    #[cfg(unix)] let old_inode={use std::os::unix::fs::MetadataExt;std::fs::metadata(path).unwrap().ino()};
    let start=Instant::now();auto.flush_tracked(path,engine,session_store::Kind::Fixture,false).unwrap();let flush_ms=start.elapsed().as_secs_f64()*1000.;
    let after=std::fs::read(path).unwrap();
    if checkpoint {assert!(!after.starts_with(&before),"{label}: expected replacement checkpoint");}else{assert!(after.starts_with(&before)&&after.len()>before.len(),"{label}: expected append");}
    #[cfg(unix)] {use std::os::unix::fs::MetadataExt;let now=std::fs::metadata(path).unwrap().ino();assert_eq!(now==old_inode,!checkpoint,"{label}: inode");}
    assert!(engine.pending_batch().receipts.is_empty());
    let (loaded,kind)=session_store::decode_bytes(&after).unwrap();assert_eq!(kind,session_store::Kind::Fixture);
    assert_eq!(engine.save().unwrap(),loaded.save().unwrap(),"{label}: exact state and exported history after reload");
    rows.push(serde_json::json!({"action":label,"checkpoint":checkpoint,"pending_receipts":pending.receipts.len(),"command_ms":command_ms,"flush_ms":flush_ms,"file_bytes":after.len(),"tick":engine.run.tick}));
}
/// O(nine actual inputs + per-input disk replay), bounded gameplay correctness,
/// not a stress/performance percentile benchmark. No SDL window is initialized.
#[test]
fn actual_desktop_play_sequence_persists_movement_combat_pickup_equip_and_order(){
    let dir=PlaySandbox(std::env::temp_dir().join(format!("corpse-append-play-{}",std::process::id())));std::fs::create_dir_all(&dir.0).unwrap();let path=dir.0.join("play.save");
    let mut engine=play_engine();let mut effects=effects::Effects::default();let mut auto=session_store::Autosave::default();let mut rows=Vec::new();
    auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();
    assert!(!engine.run.inventory.iter().any(|i|i.id==10000.));let t=Instant::now();dispatch(&mut engine,&mut effects,0.,&Command::Pickup).unwrap();let ms=t.elapsed().as_secs_f64()*1000.;
    assert!(engine.run.inventory.iter().any(|i|i.id==10000.));assert!(!engine.run.ground["1"].iter().any(|g|g.item.id==10000.));persist_step("keyboard-pickup",&engine,&mut auto,&path,false,ms,&mut rows);
    for (label,command,x) in [("move-left",Command::Move{dx:-1.,dy:0.},11.),("move-right",Command::Move{dx:1.,dy:0.},12.)] {
        let t=Instant::now();let out=dispatch(&mut engine,&mut effects,0.,&command).unwrap();let ms=t.elapsed().as_secs_f64()*1000.;assert!(out.changed);assert_eq!(engine.run.player.x,x);
        persist_step(label,&engine,&mut auto,&path,false,ms,&mut rows);
    }
    let old_hp=engine.run.actors["1"][0].hp;let t=Instant::now();dispatch(&mut engine,&mut effects,0.,&Command::Move{dx:1.,dy:0.}).unwrap();let ms=t.elapsed().as_secs_f64()*1000.;
    assert!(engine.run.actors["1"][0].hp<old_hp,"bump must actually damage the reachable enemy");assert_eq!(engine.run.player.x,12.);
    persist_step("bump-combat",&engine,&mut auto,&path,false,ms,&mut rows);
    let mut view=pack::PackView::observe(&engine.run);let mut open=true;let mut target=None;let mut notice=String::new();
    assert_ne!(engine.run.inventory_display_order,Field::Value(view.display_order()),"first PackCommand establishes order");
    let t=Instant::now();apply_pack_action(pack::PackAction::Command(Command::Equip{id:10000.,slot:"hand2".into()}),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);let ms=t.elapsed().as_secs_f64()*1000.;
    assert!(notice.is_empty(),"{notice}");assert!(engine.run.inventory.iter().find(|i|i.id==10000.).unwrap().equipped.as_ref().is_some());
    persist_step("pack-equip-establishes-display-order",&engine,&mut auto,&path,false,ms,&mut rows);
    // Equipped groups sort first; refresh after Equip changes the visible order.
    assert_ne!(engine.run.inventory_display_order,Field::Value(view.display_order()),"equipped-first refresh changes desired order");
    let t=Instant::now();apply_pack_action(pack::PackAction::Command(Command::Unequip{id:10000.}),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);let ms=t.elapsed().as_secs_f64()*1000.;assert!(notice.is_empty(),"{notice}");
    persist_step("pack-unequip-refreshes-display-order",&engine,&mut auto,&path,false,ms,&mut rows);
    let mut ids=view.display_order();ids.reverse();assert_ne!(engine.run.inventory_display_order,Field::Value(ids.clone()),"explicit reorder must change stored order");let old_tick=engine.run.tick;let t=Instant::now();apply_pack_action(pack::PackAction::Order(ids.clone()),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);let ms=t.elapsed().as_secs_f64()*1000.;
    assert_eq!(engine.run.inventory_display_order,Field::Value(ids.clone()));assert_eq!(engine.run.tick,old_tick);persist_step("explicit-pack-reorder",&engine,&mut auto,&path,false,ms,&mut rows);
    let token=engine.token();let bytes=std::fs::read(&path).unwrap();
    apply_pack_action(pack::PackAction::Order(ids),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
    assert_eq!(engine.token(),token,"identical explicit order is a true no-op");
    auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);
    for label in ["wait-after-order","wait-continued-recording"] {let t=Instant::now();dispatch(&mut engine,&mut effects,0.,&Command::Wait).unwrap();let ms=t.elapsed().as_secs_f64()*1000.;persist_step(label,&engine,&mut auto,&path,false,ms,&mut rows);}
    println!("APPEND_PLAY_SEQUENCE {}",serde_json::to_string(&rows).unwrap());
    if let Some(out)=std::env::var_os("CORPSE_APPEND_PLAY_SEQUENCE_JSON"){std::fs::write(out,serde_json::to_vec_pretty(&rows).unwrap()).unwrap();}
}
