//! Paired v19/unified persistence harness; no timings until correctness gates pass.
#![allow(dead_code,unexpected_cfgs)]
#[path="../src/session_store.rs"] mod session_store;
use corpse_core::{temporal::Engine as Core,Command,Field};
use session_store::Kind;use std::{time::Instant,path::{Path,PathBuf}};
#[cfg(unified_candidate)]use session_store::native_store::{NativeAutosave as Autosave,decode_or_legacy,fresh_stream,native_budget};
#[cfg(unified_candidate)]use corpse_core::change_journal::native_session::Session;
#[cfg(not(unified_candidate))]use session_store::{Autosave,tracked_engine::Engine as Session};
/// O(one explicit legacy migration), setup excluded from steady-state timings.
fn session(core:Core)->Session{#[cfg(unified_candidate)]{Session::import_legacy(core.canonical_run().unwrap(),fresh_stream().unwrap(),native_budget()).unwrap()}#[cfg(not(unified_candidate))]{Session::from_core(core)}}
/// O(1), immutable gameplay state.
fn state(s:&Session)->&corpse_core::Run{#[cfg(unified_candidate)]{s.state()}#[cfg(not(unified_candidate))]{&s.run}}
/// Transitive dispatch cost; no recovery retry or hidden simulation rerun in benchmark.
fn command(s:&mut Session,c:&Command)->corpse_core::engine::DispatchOutcome{s.dispatch(c).unwrap()}
/// Transitive actual durable save cost, including lock/read/validation/fsync.
fn flush(a:&mut Autosave,p:&Path,s:&mut Session,force:bool){#[cfg(unified_candidate)]{a.flush(p,s,Kind::Fixture,force).unwrap()}#[cfg(not(unified_candidate))]{a.flush_tracked(p,s,Kind::Fixture,force).unwrap()}}
/// O(file bytes + exact/native or original v19 replay), filesystem cache not forced cold.
fn reload(p:&Path)->Session{let raw=session_store::read_save_bytes(p).unwrap();#[cfg(unified_candidate)]{decode_or_legacy(&raw).unwrap().0}#[cfg(not(unified_candidate))]{Session::from_core(session_store::decode_bytes(&raw).unwrap().0)}}
/// O(snapshot export) outside timers; detached cache prevents proof collection
/// from prewarming the live autosave history cache.
fn export(s:&Session)->corpse_core::Run{#[cfg(unified_candidate)]{s.export_legacy().unwrap()}#[cfg(not(unified_candidate))]{let mut probe=s.clone();probe.set_legacy_revert(false);probe.canonical_run().unwrap()}}
/// O(1), actual retained completed-turn count, not serialized cache length.
fn groups(s:&Session)->usize{#[cfg(unified_candidate)]{s.history_len()}#[cfg(not(unified_candidate))]{s.history.frames.len()}}
/// O(full finite exported state/history bytes); outside all latency timers.
fn proof(s:&Session)->Vec<u8>{serde_json::to_vec(&export(s)).unwrap()}
/// O(seed setup + six actual Waits), excluded setup; no NPC/combat claim.
fn prepared()->Core{let mut run=corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap();run.simulation_revision=Field::Value(39.);for actors in run.actors.values_mut(){*actors=std::sync::Arc::new(Vec::new());}run.player.hp=1_000_000.;run.player.max_hp=1_000_000.;run.player.base_max_hp=Field::Value(1_000_000.);run.player.energy=1_000_000.;run.history.clear();run.replay.clear();run.future=None;run.haste_snapshot=Field::Missing;run.offers.clear();run.player.active.clear();run.player.conditions.clear();if !run.player.abilities.iter().any(|id|id=="numo-revert"){run.player.abilities.push("numo-revert".into());}corpse_core::progression::initialize_ability_state(&mut run.player,"numo-revert",run.tick).unwrap();run.player.conditions.insert("revertCharges".into(),20.);let mut core=Core::load(&corpse_core::storage::serialize(&run).unwrap()).unwrap();for _ in 0..6{let tick=core.run.tick;let outcome=core.dispatch(&Command::Wait).unwrap();assert!(outcome.took_turn);assert_eq!(core.run.tick,tick+1.);}assert_eq!(core.history.frames.len(),5);assert!(core.run.player.energy>=10.);assert!(core.run.player.conditions.get("revertCharges").copied().unwrap_or(0.)>=5.);core}
/// O(n log n), upper-middle median and nearest-rank p95, outside measured work.
fn stats(xs:&[f64])->serde_json::Value{let mut sorted=xs.to_vec();sorted.sort_by(f64::total_cmp);serde_json::json!({"n":sorted.len(),"median_ms":sorted[sorted.len()/2],"p95_ms":sorted[(sorted.len()*95).div_ceil(100)-1],"max_ms":sorted[sorted.len()-1]})}
/// O(trials * five real Reverts+durable flush), fixed50 samples per arm. Setup,
/// initial durability, file classification, full equality and recovery excluded.
fn main(){let out=PathBuf::from(std::env::var("REVERT_BENCH_EVIDENCE_DIR").expect("evidence directory"));std::fs::create_dir_all(&out).unwrap();let dir=std::env::temp_dir().join(format!("corpse-unified-revert-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();let op=Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};let mut samples=vec![];let mut loads=vec![];let mut initial_proof=None;
 for trial in 0..10{let mut s=session(prepared());assert_eq!(groups(&s),5);let prepared_bytes=proof(&s);if let Some(expected)=&initial_proof{assert_eq!(&prepared_bytes,expected,"trial initialization differs");}else{std::fs::write(out.join("initial-h5-full-state.json"),&prepared_bytes).unwrap();initial_proof=Some(prepared_bytes);}let p=dir.join(format!("trial-{trial}.save"));let mut a=Autosave::default();flush(&mut a,&p,&mut s,true);let mut before=std::fs::read(&p).unwrap();
  for step in 0..5{let tick=state(&s).tick;let count=groups(&s);let ledger=state(&s).time_costs.value_or_default();let wall=Instant::now();let t=Instant::now();let outcome=command(&mut s,&op);let command_ns=t.elapsed().as_nanos();let t=Instant::now();flush(&mut a,&p,&mut s,false);let flush_ns=t.elapsed().as_nanos();let wall_ns=wall.elapsed().as_nanos();
   assert!(outcome.changed,"unavailable Revert is not a sample");assert_eq!(groups(&s),count-1,"Revert must consume actual group");assert_eq!(state(&s).tick,tick-1.);assert_eq!(state(&s).time_costs.value_or_default().charges,ledger.charges+1.);assert_eq!(state(&s).time_costs.value_or_default().energy,ledger.energy+2.);
   let after=std::fs::read(&p).unwrap();let checkpoint=!after.starts_with(&before);let written=if checkpoint{after.len()}else{after.len()-before.len()};before=after;let full=proof(&s);std::fs::write(out.join(format!("trial{trial}-revert{}.json",step+1)),&full).unwrap();
   samples.push(serde_json::json!({"trial":trial,"revert":step+1,"groups_before":count,"groups_after":groups(&s),"tick_before":tick,"tick_after":state(&s).tick,"command_ns":command_ns,"flush_ns":flush_ns,"wall_ns":wall_ns,"checkpoint":checkpoint,"written_bytes":written,"file_bytes":before.len()}));
   if [0,4,9].contains(&trial)&&[1,4].contains(&step){let t=Instant::now();let mut loaded=reload(&p);let load_ns=t.elapsed().as_nanos();assert_eq!(proof(&loaded),full,"saved full state/history recovery");assert_eq!(groups(&loaded),groups(&s));let mut oracle=Core::new(export(&s)).unwrap();let remaining=groups(&loaded);for _ in 0..remaining{let prior=groups(&loaded);command(&mut loaded,&op);oracle.dispatch(&op).unwrap();assert_eq!(groups(&loaded),prior-1);assert_eq!(serde_json::to_vec(&export(&loaded)).unwrap(),serde_json::to_vec(&oracle.canonical_run().unwrap()).unwrap(),"recovered future Revert semantics");}assert_eq!(groups(&loaded),0);loads.push(serde_json::json!({"trial":trial,"revert":step+1,"load_ns":load_ns,"recovered_remaining_actual_pops":remaining}));}
  }assert_eq!(groups(&s),0);eprintln!("completed actual five-Revert trial{trial}");
 }
 let component=|key:&str|samples.iter().map(|row|row[key].as_u64().unwrap()as f64/1_000_000.).collect::<Vec<_>>();println!("{}",serde_json::to_string(&serde_json::json!({"implementation":if cfg!(unified_candidate){"unified"}else{"snapshot-overwrite-trial"},"profile":"normal release, collector disabled","scope":"10 independent trials x5 actual Reverts, six Waits prepared outside timing; NPCcontents cleared preserving floor map keys; initial checkpoint/setup/proofs/recovery excluded; command+actual durableflush includes lock/read/fsync; cached filesystem, not input-to-visible latency","command":stats(&component("command_ns")),"flush":stats(&component("flush_ns")),"wall":stats(&component("wall_ns")),"samples":samples,"loads":loads,"cross_binary_proofs":"initial-h5-full-state and all50 full post-Revert exported states/history JSON; compare paired arms outside timing without patch normalization"})).unwrap());let _=std::fs::remove_dir_all(dir);
}
