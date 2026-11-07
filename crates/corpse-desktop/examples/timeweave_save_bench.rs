//! Revision40/41 matched persistence control, no input/render claim.
//! Actual command + durable append/checkpoint trial. Capture pilot remains disabled.
//! Exact canonical/history proofs and metadata observers run outside measured intervals.
#![allow(dead_code)]
#[path="../src/session_store.rs"] mod session_store;
use corpse_core::{temporal::Engine as Core,Command,Field,Run};
use session_store::{Autosave,Kind,tracked_engine::Engine as Session};
use std::{path::{Path,PathBuf},time::Instant,sync::Arc};
/// O(world/history export), detached cache, observer-only.
fn world(s:&Session)->Vec<u8>{let mut copy=s.clone();copy.set_legacy_revert(false);serde_json::to_vec(&copy.canonical_run().unwrap()).unwrap()}
/// O(save bytes+replay), real cold-load API; OS cache is NOT forcibly cold.
fn reload(p:&Path)->Session{Session::from_core(session_store::decode_bytes(&session_store::read_save_bytes(p).unwrap()).unwrap().0)}
/// O(proof bytes), exact comparison across arms despite intentionally different save bytes.
fn proof(out:&Path,reference:Option<&Path>,name:&str,bytes:&[u8]){if let Some(r)=reference{assert_eq!(std::fs::read(r.join(name)).unwrap(),bytes,"proof mismatch {name}");}std::fs::write(out.join(name),bytes).unwrap();}
/// O(fixture world), ordinary controlled no-NPC scenario with H5 setup.
fn revision()->f64 { let r:f64=std::env::var("WEAVE_REVISION").expect("40 or41").parse().unwrap();assert!(r==40.||r==41.);r }
fn ordinary(large:bool)->Run {
 let mut r=if large{Core::load(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap()).unwrap().run}else{corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()};
 r.simulation_revision=revision().into();r.inventory_display_order=Field::Missing;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();for a in r.actors.values_mut(){*a=Arc::new(vec![]);}r.player.hp=1_000_000.;r.player.max_hp=1_000_000.;r.player.base_max_hp=1_000_000.0.into();r.player.energy=1_000_000.;r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;
 if large {let starter=r.inventory.iter().find(|i|i.kind=="weapon").unwrap().clone();while r.inventory.len()<20{let mut x=starter.clone();x.id=r.next_id;r.next_id+=1.;x.equipped=Field::Missing;r.inventory.push(x);}}
 r
}
/// O(room cells+actors), authored synthetic stress; setup is never a measurement.
fn room()->Run {
 let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:capture-pilot","Unblessed",false,39.).unwrap();r.player.x=2.;r.player.y=2.;r.player.hp=1_000_000.;r.player.max_hp=1_000_000.;r.player.base_max_hp=1_000_000.0.into();r.player.energy=100_000.;r.player.stats.dr=1000.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
 let(w,h)=(79,29);let mut tiles=vec!["wall".to_string();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
 let mut f=corpse_core::Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=4.;r.generated_ability_grants=Vec::new().into();r
}
/// O(fixture+five real turns), setup excluded; only long case grants Revert explicitly.
fn prepared(name:&str)->Core{
 let mut r=match name{"10-h5-wait"=>ordinary(false),"40-h5-wait"=>ordinary(true),"long-one-floor"=>room(),_=>panic!("unknown case {name}")};
 r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;
 if name=="long-one-floor"{r.player.abilities.push("numo-revert".into());corpse_core::progression::initialize_ability_state(&mut r.player,"numo-revert",r.tick).unwrap();r.player.conditions.insert("revertCharges".into(),20.);}
 let mut e=Core::load(&corpse_core::storage::serialize(&r).unwrap()).unwrap();for _ in 0..5{assert!(e.dispatch(&Command::Wait).unwrap().took_turn);}assert_eq!(e.history.frames.len(),if revision()==41.{0}else{5});e
}
/// O(1) metadata observer on Unix: rename checkpoints change inode; append does not.
#[cfg(unix)]fn stamp(p:&Path)->(u64,u64,u64){use std::os::unix::fs::MetadataExt;let m=std::fs::metadata(p).unwrap();(m.dev(),m.ino(),m.len())}
/// Unsupported host fails explicitly rather than guessing checkpoint classification.
#[cfg(not(unix))]fn stamp(_: &Path)->(u64,u64,u64){panic!("This measurement harness requires Unix inode metadata")}
/// O(n log n) sorting, nearest-rank tails and upper-middle median, outside timings.
fn stats(values:&[u128])->serde_json::Value{let mut v=values.to_vec();v.sort_unstable();let at=|pct:usize|v[(v.len()*pct).div_ceil(100)-1]as f64/1e6;serde_json::json!({"n":v.len(),"median_ms":v[v.len()/2]as f64/1e6,"p95_ms":at(95),"p99_ms":at(99),"max_ms":v[v.len()-1]as f64/1e6})}
/// O(L) diagnostic reset outside wall; normal builds perform no collector work.
fn diag_reset(enabled:bool){#[cfg(feature="runtime-timing")]if enabled{corpse_core::timing::set_enabled(false);corpse_core::timing::reset();}#[cfg(not(feature="runtime-timing"))]assert!(!enabled,"APPEND_DIAGNOSTICS requires runtime-timing feature");}
/// O(1) diagnostic mode switch; recorder remains disabled independently.
fn diag_enable(enabled:bool){#[cfg(feature="runtime-timing")]corpse_core::timing::set_enabled(enabled);#[cfg(not(feature="runtime-timing"))]let _=enabled;}
/// O(L) serialization after wall; root inclusive = root exclusive + direct child inclusives.
fn diag_take(enabled:bool)->serde_json::Value{#[cfg(feature="runtime-timing")]if enabled{corpse_core::timing::set_enabled(false);return serde_json::to_value(corpse_core::timing::take_report()).unwrap();}let _=enabled;serde_json::Value::Null}
/// Transitive actual flush, timing root spans initial checkpoints and append samples alike.
fn flush(a:&mut Autosave,path:&Path,s:&Session,force:bool){let _root=corpse_core::timing::span("bench.append.flush.root");a.flush_tracked(path,s,Kind::Fixture,force).unwrap();}
/// O(commands*(dispatch+durability)+proof exports), bounded observer cadence; no rendering claim.
fn main(){
 assert!(!corpse_core::capture_pilot::enabled());
 let steps:usize=std::env::var("APPEND_STEPS").unwrap_or_else(|_|"160".into()).parse().unwrap();assert!(steps>0);
 let cases=std::env::var("APPEND_CASES").unwrap_or_else(|_|"10-h5-wait,40-h5-wait".into());
 let diagnostics=std::env::var("APPEND_DIAGNOSTICS").as_deref()==Ok("1");let long_required=std::env::var("APPEND_REQUIRE_LONG_TAIL").as_deref()==Ok("1");
 let out=PathBuf::from(std::env::var("APPEND_EVIDENCE_DIR").expect("new evidence directory"));assert!(!out.exists());std::fs::create_dir_all(&out).unwrap();let reference=std::env::var("APPEND_REFERENCE_DIR").ok().map(PathBuf::from);
 let temp=std::env::temp_dir().join(format!("corpse-append-recording-{}",std::process::id()));assert!(!temp.exists());std::fs::create_dir(&temp).unwrap();let mut results=vec![];
 for name in cases.split(','){
  assert!(matches!(name,"10-h5-wait"|"40-h5-wait"),"new-rule benchmark has no retired Revert case");
  let mut s=Session::from_core(prepared(name));let floors=s.run.floors.len();let mut a=Autosave::default();let path=temp.join(format!("{name}.save"));proof(&out,reference.as_deref(),&format!("{name}-initial-world.json"),&world(&s));
  diag_reset(diagnostics);diag_enable(diagnostics);let t=Instant::now();flush(&mut a,&path,&s,true);let initial_save_ns=t.elapsed().as_nanos();let initial_diagnostics=diag_take(diagnostics);let mut prior=stamp(&path);let initial_bytes=prior.2;
  let mut rows=vec![];let mut outcomes=vec![];let mut checkpoint_count=0;let mut wall_samples=vec![];let mut dispatch_samples=vec![];let mut flush_samples=vec![];
  for step in 1..=steps{
   diag_reset(diagnostics);let wall=Instant::now();let t=Instant::now();let outcome=s.dispatch(&Command::Wait).unwrap();let dispatch_ns=t.elapsed().as_nanos();diag_enable(diagnostics);let t=Instant::now();flush(&mut a,&path,&s,false);let flush_ns=t.elapsed().as_nanos();let wall_ns=wall.elapsed().as_nanos();let diagnostic=diag_take(diagnostics);
   assert!(outcome.took_turn);assert_eq!(s.run.status,"playing");assert!(!corpse_core::capture_pilot::enabled());
   let after=stamp(&path);let checkpoint=(after.0,after.1)!=(prior.0,prior.1)||after.2<prior.2;let written_bytes=if checkpoint{after.2}else{after.2-prior.2};checkpoint_count+=usize::from(checkpoint);prior=after;
   outcomes.push(serde_json::json!({"step":step,"changed":outcome.changed,"took_turn":outcome.took_turn,"events":outcome.events,"rng_bits":s.run.rng.to_bits(),"tick_bits":s.run.tick.to_bits(),"depth_bits":s.run.depth.to_bits()}));
   if step%32==0||step==steps{proof(&out,reference.as_deref(),&format!("{name}-step{step}-world.json"),&world(&s));}
   assert!(wall_ns>=dispatch_ns+flush_ns);wall_samples.push(wall_ns);dispatch_samples.push(dispatch_ns);flush_samples.push(flush_ns);
   rows.push(serde_json::json!({"step":step,"dispatch_ns":dispatch_ns,"flush_ns":flush_ns,"wall_ns":wall_ns,"command_plus_flush_ns":dispatch_ns+flush_ns,"unattributed_timer_ns":wall_ns-dispatch_ns-flush_ns,"written_bytes":written_bytes,"checkpoint":checkpoint,"file_bytes":after.2,"diagnostic":diagnostic}));
  }
  proof(&out,reference.as_deref(),&format!("{name}-outcomes.json"),&serde_json::to_vec(&outcomes).unwrap());
  // Explicit administrative order mutation retains current fallback semantics; measured separately.
  let order=s.run.inventory.iter().map(|i|i.id).collect::<Vec<_>>();
  assert_ne!(s.run.inventory_display_order,Field::Value(order.clone()));s.run.inventory_display_order=Field::Value(order);assert!(s.pending_batch().checkpoint_required);let t=Instant::now();flush(&mut a,&path,&s,false);let administrative_checkpoint_ns=t.elapsed().as_nanos();
  let administrative_bytes=std::fs::metadata(&path).unwrap().len();
  let final_world=world(&s);let t=Instant::now();let mut loaded=reload(&path);let cold_load_ns=t.elapsed().as_nanos();assert_eq!(world(&loaded),final_world,"final exact canonical/history recovery");
  let mut actual_reverts=0;
  if name=="long-one-floor"{let mut expected=s.clone();let op=Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};for n in 1..=5{let tick=loaded.run.tick;let groups=loaded.history.frames.len();let charges=loaded.run.time_costs.value_or_default().charges;let a=loaded.dispatch(&op).unwrap();let b=expected.dispatch(&op).unwrap();assert!(a.changed&&b.changed);assert_eq!(loaded.run.tick,tick-1.);assert_eq!(loaded.history.frames.len(),groups-1);assert_eq!(loaded.run.time_costs.value_or_default().charges,charges+1.);assert_eq!(world(&loaded),world(&expected));proof(&out,reference.as_deref(),&format!("{name}-revert{n}-world.json"),&world(&loaded));actual_reverts+=1;}if long_required{assert!(steps>=4096);assert_eq!(checkpoint_count,0,"long candidate must retain same epoch");assert!(prior.2-initial_bytes>1024*1024,"must prove >1MiB append tail");}}
  results.push(serde_json::json!({"case":name,"revision":revision(),"retained_frames":s.history.frames.len(),"administrative_checkpoint_ns":administrative_checkpoint_ns,"administrative_checkpoint_bytes":administrative_bytes,"floors":floors,"steps":steps,"initial_save_ns":initial_save_ns,"initial_bytes":initial_bytes,"initial_diagnostic":initial_diagnostics,"checkpoint_count":checkpoint_count,"append_end_bytes":prior.2,"tail_growth_bytes":prior.2.saturating_sub(initial_bytes),"cold_load_ns":cold_load_ns,"actual_postload_reverts":actual_reverts,"dispatch":stats(&dispatch_samples),"flush":stats(&flush_samples),"wall":stats(&wall_samples),"samples":rows}));drop(a);eprintln!("completed {name}, {steps} commands");
 }
 println!("{}",serde_json::to_string(&serde_json::json!({"revision":revision(),"capture_enabled":false,"diagnostics":diagnostics,"steps":steps,"scope":"Same-current-source revision40/41 persistence control. Five preparatory Waits retain H5 only under40. Actual dispatch+durable flush; setup, initial checkpoint, explicit administrative-order fallback checkpoint, metadata/proofs and reload are separate. No input/render/Windows latency claim. Filesystem cache not forcibly cleared.","proof":"identical outcomes and full canonical/history bytes across supplied reference; save bytes may differ by checkpoint epoch/tail layout","cases":results})).unwrap());std::fs::remove_dir_all(temp).unwrap();
}
