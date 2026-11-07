//! Supplemental load/torn-tail recovery timing; normal release, no instrumentation.
#![allow(dead_code)]
#[path="../src/session_store.rs"] mod session_store;
use corpse_core::{temporal::Engine as Core,Command,Field};
use session_store::{Autosave,Kind,tracked_engine::{Engine,Receipt,Stamp},incremental_log as log};
use std::{path::PathBuf,time::Instant};
/// O(setup world bytes). Controlled persistence fixtures remove NPCs and boost
/// player HP/energy to keep128 Waits alive, not a combat-performance claim.
fn prepared(large:bool,history:usize)->Core {
 let mut run=if large {Core::load(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap()).unwrap().run}
 else {corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()};
 run.simulation_revision=Field::Value(39.);for actors in run.actors.values_mut(){*actors=std::sync::Arc::new(Vec::new());}run.player.hp=1_000_000.;run.player.max_hp=1_000_000.;run.player.base_max_hp=Field::Value(1_000_000.);run.player.energy=1_000_000.;
 run.history.clear();run.replay.clear();run.haste_snapshot=Field::Missing;
 if large {let starter=run.inventory.iter().find(|i|i.kind=="weapon").unwrap().clone();while run.inventory.len()<20{let mut item=starter.clone();item.id=run.next_id;run.next_id+=1.;item.equipped=Field::Missing;run.inventory.push(item);}}
 // Serialization reload discards stale derived indices after synthetic edits.
 let mut core=Core::load(&corpse_core::storage::serialize(&run).unwrap()).unwrap();
 for _ in 0..history{core.dispatch(&Command::Wait).unwrap();}core
}

/// O(n log n), outside timing; upper-middle and nearest-rank p95.
fn stats(xs:&[f64])->serde_json::Value{let mut s=xs.to_vec();s.sort_by(f64::total_cmp);serde_json::json!({"n":s.len(),"median_ms":s[s.len()/2],"p95_ms":s[(s.len()*95).div_ceil(100)-1],"samples_ms":xs})}
/// O(setup+rounds*(read+decode+<=31 simulation commands)); no output encode in timer.
fn main(){
 let rounds:usize=std::env::var("SAVE_RECOVERY_ROUNDS").ok().and_then(|s|s.parse().ok()).unwrap_or(10);
 let dir=std::env::temp_dir().join(format!("corpse-recovery-bench-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();let mut rows=Vec::new();
 for large in [false,true]{
  let name=if large{"40-floors"}else{"10-floors"};let live=dir.join(format!("{name}-live.json"));let mut e=Engine::from_core(prepared(large,5));let mut auto=Autosave::default();auto.flush_tracked(&live,&e,Kind::Fixture,true).unwrap();
  for _ in 0..31{e.dispatch(&Command::Wait).unwrap();auto.flush_tracked(&live,&e,Kind::Fixture,false).unwrap();}
  let raw=session_store::read_save_bytes(&live).unwrap();let parsed=log::parse(&raw,&log::Header::current(),|_|Ok(())).unwrap();assert_eq!(parsed.records.len(),31);
  let mut next=e.clone();next.dispatch(&Command::Wait).unwrap();let payload=serde_json::to_string(&serde_json::json!({"receipts":[Receipt{command:Command::Wait,after:Stamp::capture(&next)}]})).unwrap();
  let extra=log::encode_append(parsed.metadata.next_sequence,parsed.metadata.last_checksum,&payload).unwrap();let mut torn=raw.clone();torn.extend_from_slice(&extra.bytes[..extra.bytes.len()/2]);
  let checkpoint=session_store::encode(&e,Kind::Fixture).unwrap().into_bytes();
  let modes=[("full-checkpoint",checkpoint),("valid-31-command-tail",raw),("torn-uncommitted-32nd",torn)];let mut paths=Vec::new();
  for (mode,bytes)in &modes{let p=dir.join(format!("{name}-{mode}.json"));session_store::atomic_write(&p,bytes).unwrap();paths.push(p);}
  let expected=session_store::decode_bytes(&modes[1].1).unwrap().0.save().unwrap();assert_eq!(session_store::decode_bytes(&modes[2].1).unwrap().0.save().unwrap(),expected,"torn tail changed recovered state");
  let mut samples=vec![Vec::new();3];
  for round in 0..rounds{let indices:Vec<usize>=if round%2==0{(0..3).collect()}else{(0..3).rev().collect()};for i in indices{let t=Instant::now();let engine=session_store::decode_bytes(&session_store::read_save_bytes(&paths[i]).unwrap()).unwrap().0;let ms=t.elapsed().as_secs_f64()*1000.;samples[i].push(ms);if i>0{assert_eq!(engine.save().unwrap(),expected);}std::hint::black_box(engine);}}
  for i in 0..3{rows.push(serde_json::json!({"case":name,"mode":modes[i].0,"bytes":modes[i].1.len(),"load":stats(&samples[i])}));}
  eprintln!("finished recovery {name}");
 }
 println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"fresh Engine decode/replay incl bounded read; filesystem cache not forced cold; torn tail discards uncommitted32nd command; normal release collector-off","cases":rows})).unwrap());
}
