//! Identical v18/candidate harness. --cfg incremental_candidate selects new API.
//! Continuous128-command traces include32-command checkpoints, never just warmed
//! one-command clones. Assertions/evidence/setup are outside recorded intervals.
#![allow(dead_code,unexpected_cfgs)]
#[path="../src/session_store.rs"] mod session_store;
use corpse_core::{temporal::Engine as Core,Command,Field};
use session_store::{Autosave,Kind};
use std::{time::Instant,path::PathBuf};
#[cfg(incremental_candidate)] type Session=session_store::tracked_engine::Engine;
#[cfg(not(incremental_candidate))] type Session=Core;
/// O(core ownership transfer), no world copy.
fn session(core:Core)->Session {#[cfg(incremental_candidate)]{Session::from_core(core)}#[cfg(not(incremental_candidate))]{core}}
/// Baseline fullflush or candidate trackedflush; respective transitive costs.
fn flush(auto:&mut Autosave,path:&std::path::Path,e:&Session,force:bool){
 #[cfg(incremental_candidate)]auto.flush_tracked(path,e,Kind::Fixture,force).unwrap();
 #[cfg(not(incremental_candidate))]auto.flush(path,e,Kind::Fixture,force).unwrap();
}
/// O(file bytes+decode+bounded replay); filesystem cache is not forced cold.
fn reload(path:&std::path::Path)->Core {
 #[cfg(incremental_candidate)]{session_store::decode_bytes(&session_store::read_save_bytes(path).unwrap()).unwrap().0}
 #[cfg(not(incremental_candidate))]{session_store::decode(&std::fs::read_to_string(path).unwrap()).unwrap().0}
}
/// O(n log n), outside timed code. Upper-middle median, nearest-rank p95.
fn stats(xs:&[f64])->serde_json::Value {if xs.is_empty(){return serde_json::Value::Null;}let mut s=xs.to_vec();s.sort_by(f64::total_cmp);serde_json::json!({"n":s.len(),"median_ms":s[s.len()/2],"p95_ms":s[(s.len()*95).div_ceil(100)-1],"max_ms":s[s.len()-1],"samples_ms":xs})}
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
/// O(world bytes), removes only opaque inverse-frame encoding. V18 itself can
/// reorder commuting deletion-only history patches after reload; world stays exact.
fn world_bytes(engine:&Core)->String {
 let mut value:serde_json::Value=serde_json::from_str(&engine.save().unwrap()).unwrap();
 value.as_object_mut().unwrap().shift_remove("history");serde_json::to_string(&value).unwrap()
}
/// O(snapshot handles + ordinary clone), independent export cache for assertions.
fn isolated(source:&Core)->Core {
 let mut history=corpse_core::temporal::History::default();history.frames=source.history.frames.clone();
 history.closing=source.history.closing.clone();history.pending_haste=source.history.pending_haste.clone();
 Core{run:source.run.clone(),history}
}
/// O(world export + <=5 inverse applications); outside timing. Checks actual
/// outcomes rather than ignoring the historical patch ordering difference.
fn verify_recovery(source:&Core,loaded:Core,name:&str,step:usize){
 let mut original=isolated(source);let mut resumed=loaded;
 for rewind in 0..=5 {
  let a=world_bytes(&original);let b=world_bytes(&resumed);
  if a!=b {panic!("recovery world mismatch {name}/{step}, rewind{rewind}; lengths {} vs {}",a.len(),b.len());}
  if rewind<5 {let a=corpse_core::temporal::revert(&mut original.run,&mut original.history).unwrap();let b=corpse_core::temporal::revert(&mut resumed.run,&mut resumed.history).unwrap();assert_eq!(a,b,"rewind availability mismatch");if !a{break;}}
 }
}
/// O(cases*steps*(command+save)); persistent-state assertions/evidence excluded.
fn main(){
 let steps:usize=std::env::var("SAVE_BENCH_STEPS").ok().and_then(|x|x.parse().ok()).unwrap_or(128);assert!(steps>=32);
 let selected=std::env::var("SAVE_BENCH_CASES").unwrap_or_else(|_|"10-h0-wait,10-h1-wait,10-h5-wait,10-h5-pickup,40-h5-wait,40-h5-pickup".into());
 let evidence=PathBuf::from(std::env::var("SAVE_BENCH_EVIDENCE_DIR").expect("evidence directory"));std::fs::create_dir_all(&evidence).unwrap();
 let dir=std::env::temp_dir().join(format!("corpse-incremental-bench-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();let mut rows=Vec::new();
 for name in selected.split(','){
  let large=name.starts_with("40-");let op=if name.ends_with("-wait"){Command::Wait}else{Command::Pickup};let history=name.split('-').find_map(|s|s.strip_prefix('h').and_then(|n|n.parse::<usize>().ok())).unwrap_or(5);
  let mut e=session(prepared(large,history));let path=dir.join(format!("{name}.json"));let mut auto=Autosave::default();
  let t=Instant::now();flush(&mut auto,&path,&e,true);let initial=t.elapsed().as_secs_f64()*1000.;
  let mut command=Vec::new();let mut save=Vec::new();let mut total=Vec::new();let mut checkpoint=Vec::new();let mut append=Vec::new();let mut written=Vec::new();let mut loads=Vec::new();let mut previous=std::fs::metadata(&path).unwrap().len();
  for step in 1..=steps{
   let root=Instant::now();let t=Instant::now();let outcome=e.dispatch(&op).unwrap();let cmd=t.elapsed().as_secs_f64()*1000.;
   let t=Instant::now();flush(&mut auto,&path,&e,false);let ms=t.elapsed().as_secs_f64()*1000.;let combined=root.elapsed().as_secs_f64()*1000.;
   assert_eq!(e.run.status,"playing","controlled benchmark died at {name}/{step}");if matches!(op,Command::Wait){assert!(outcome.took_turn,"Wait failed to advance");}
   let bytes=std::fs::metadata(&path).unwrap().len();
   let is_checkpoint=cfg!(not(incremental_candidate))||step%32==0;
   written.push(if is_checkpoint{bytes}else{bytes.checked_sub(previous).expect("unexpected checkpoint outside interval")});previous=bytes;
   command.push(cmd);save.push(ms);total.push(combined);if is_checkpoint{checkpoint.push(ms)}else{append.push(ms)}
   if step%32==0||step==31||step==steps{
    let t=Instant::now();let loaded=reload(&path);let load=t.elapsed().as_secs_f64()*1000.;
    let expected=e.save().unwrap();verify_recovery(&e,loaded,name,step);
    std::fs::write(evidence.join(format!("{name}-step{step}.json")),expected).unwrap();
    loads.push(serde_json::json!({"step":step,"tail_commands":step%32,"read_decode_replay_ms":load}));
   }
  }
  rows.push(serde_json::json!({"case":name,"steps":steps,"starting_history":history,"initial_checkpoint_ms":initial,"command":stats(&command),"flush_inclusive":stats(&save),"command_plus_flush":stats(&total),"periodic_checkpoint_flush":stats(&checkpoint),"routine_append_flush":stats(&append),"bytes_written_per_command":written,"load_samples":loads}));
  std::fs::remove_file(path).unwrap();eprintln!("finished {name}");
 }
 println!("{}",serde_json::to_string_pretty(&serde_json::json!({"implementation":std::env::var("SAVE_BENCH_IMPL_LABEL").unwrap_or_else(|_|if cfg!(incremental_candidate){"incremental".into()}else{"full-envelope API (build-specific)".into()}),"profile":"normal release collector-off","scope":"continuous controlled persistence traces; includes periodic checkpoint spikes and fsync; not input-to-display or Windows runtime","cases":rows})).unwrap());
}
