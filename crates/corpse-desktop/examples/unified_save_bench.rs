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
/// O(world bytes), exact finite fixture world comparison excluding opaque history
/// patch ordering; real five-Revert and full exact graph tests are separate gates.
fn world(s:&Session)->String{let mut value=serde_json::to_value(export(s)).unwrap();value.as_object_mut().unwrap().shift_remove("history");serde_json::to_string(&value).unwrap()}
/// O(n log n), outside timed code. Upper-middle median, nearest-rank p95.
fn stats(xs:&[f64])->serde_json::Value {if xs.is_empty(){return serde_json::Value::Null;}let mut s=xs.to_vec();s.sort_by(f64::total_cmp);serde_json::json!({"n":s.len(),"median_ms":s[s.len()/2],"p95_ms":s[(s.len()*95).div_ceil(100)-1],"max_ms":s[s.len()-1],"samples_ms":xs})}
/// O(case-token bytes), opt-in metadata stress never changes default cases.
#[derive(Clone,Copy)]
struct Stress { archive:bool,replay:bool,future:bool }
impl Stress {
 /// O(case spelling bytes), exact token matching avoids accidental mode selection.
 fn parse(name:&str)->Self{let has=|needle|name.split('-').any(|token|token==needle);Self{archive:has("archive"),replay:has("replay"),future:has("future")}}
 /// O(1), explicit setup provenance, not a claim of naturally accumulated play.
 fn description(self)->serde_json::Value{serde_json::json!({"archive":if self.archive{"manufactured deterministic valid intern table/event stream"}else{"ordinary fixture archive"},"replay":if self.replay{"manufactured deterministic Wait command metadata; not a seed replay trace"}else{"ordinary setup command trace"},"future":if self.future{"created by actual morae-see-the-future dispatch; includes original activation cost"}else{"absent unless fixture supplies one"}})}
}
/// O(environment text), bounded explicit stress sizes; invalid input fails loudly.
fn setting(name:&str,default:usize,max:usize)->usize{let value=std::env::var(name).map(|v|v.parse::<usize>().expect("stress size must be unsigned integer")).unwrap_or(default);assert!(value>0&&value<=max,"{name} must be in1..={max}");value}
/// O(archive messages + replay metadata serialization), OUTSIDE all timers. Byte
/// fields name their exact representation; none is a claimed allocator/RSS size.
fn payload_metrics(run:&corpse_core::Run)->serde_json::Value {
 let archive=run.log_archive.as_ref();
 serde_json::json!({"floors":run.floors.len(),"inventory_items":run.inventory.len(),"archive_messages":archive.map_or(0,|a|a.messages.len()),"archive_message_utf8_bytes":archive.map_or(0,|a|a.messages.iter().map(String::len).sum::<usize>()),"archive_json_bytes":archive.map_or(0,|a|serde_json::to_vec(a).unwrap().len()),"archive_events":archive.map_or(0,|a|a.events.len()),"archive_event_f64_payload_bytes":archive.map_or(0,|a|a.events.len()*std::mem::size_of::<f64>()),"replay_commands":run.replay.len(),"replay_json_bytes":serde_json::to_vec(&run.replay).unwrap().len(),"future_utf8_bytes":run.future.as_ref().and_then(|a|a.text_if_encoded()).map(str::len)})
}
/// O(1), cheap row labels outside command/save timers; no archive scan/serialization.
fn payload_counts(run:&corpse_core::Run)->serde_json::Value{serde_json::json!({"archive_messages":run.log_archive.as_ref().map_or(0,|a|a.messages.len()),"archive_events":run.log_archive.as_ref().map_or(0,|a|a.events.len()),"replay_commands":run.replay.len(),"future_utf8_bytes":run.future.as_ref().and_then(|a|a.text_if_encoded()).map(str::len)})}
/// O(setup world bytes). Controlled persistence fixtures remove NPCs and boost
/// player HP/energy to keep128 Waits alive, not a combat-performance claim.
fn prepared(large:bool,history:usize,stress:Stress)->Core {
 let mut run=if large {Core::load(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap()).unwrap().run}
 else {corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()};
 run.simulation_revision=Field::Value(39.);for actors in run.actors.values_mut(){*actors=std::sync::Arc::new(Vec::new());}run.player.hp=1_000_000.;run.player.max_hp=1_000_000.;run.player.base_max_hp=Field::Value(1_000_000.);run.player.energy=1_000_000.;
 run.history.clear();run.replay.clear();run.haste_snapshot=Field::Missing;
 if stress.archive {
  let messages=setting("SAVE_BENCH_ARCHIVE_MESSAGES",2048,20000);let events=setting("SAVE_BENCH_ARCHIVE_EVENTS",50000,1000000);let padding=setting("SAVE_BENCH_ARCHIVE_PADDING",96,1024);
  let table=(0..messages).map(|i|format!("manufactured-archive-message-{i:06}:{}","x".repeat(padding))).collect();
  run.log_archive=Field::Value(corpse_core::LogArchive{version:1.,messages:table,events:(0..events).map(|i|((i%messages)*4+i%4)as f64).collect(),incomplete:false});
 }
 if stress.replay {run.replay=vec![Command::Wait;setting("SAVE_BENCH_REPLAY_COMMANDS",20000,200000)];}
 if stress.future && !run.player.abilities.iter().any(|id|id=="morae-see-the-future") {
  run.player.abilities.push("morae-see-the-future".into());corpse_core::progression::initialize_ability_state(&mut run.player,"morae-see-the-future",run.tick).unwrap();
 }

 if large {let starter=run.inventory.iter().find(|i|i.kind=="weapon").unwrap().clone();while run.inventory.len()<20{let mut item=starter.clone();item.id=run.next_id;run.next_id+=1.;item.equipped=Field::Missing;run.inventory.push(item);}}
 // Serialization reload discards stale derived indices after synthetic edits.
 let mut core=Core::load(&corpse_core::storage::serialize(&run).unwrap()).unwrap();
 for _ in 0..history{core.dispatch(&Command::Wait).unwrap();}
 if stress.future {
  assert!(core.run.future.is_none(),"Future stress requires an initially absent anchor");
  let outcome=core.dispatch(&Command::Ability{id:"morae-see-the-future".into(),target:Default::default(),x:Default::default(),y:Default::default()}).unwrap();
  assert!(outcome.changed&&core.run.future.is_some(),"actual Future activation did not create an anchor");
  // No post-activation energy refill: preserve real command cost and exact anchor.
  if large {assert!(core.run.future.as_ref().unwrap().wire_text().unwrap().len()>1024*1024,"40-floor Future must be a genuinely large serialized anchor");}
 }
 core
}

/// O(cases*steps*(command+durable save)), assertions/file classification outside timers.
fn main(){
 let setup_only=std::env::var("SAVE_BENCH_SETUP_ONLY").as_deref()==Ok("1");
 let steps:usize=std::env::var("SAVE_BENCH_STEPS").ok().and_then(|s|s.parse().ok()).unwrap_or(160);assert!(steps>=64);
 let cases=std::env::var("SAVE_BENCH_CASES").unwrap_or_else(|_|"10-h0-wait,10-h1-wait,10-h5-wait,10-h5-pickup,40-h5-wait,40-h5-pickup".into());
 let out=PathBuf::from(std::env::var("SAVE_BENCH_EVIDENCE_DIR").expect("evidence directory"));std::fs::create_dir_all(&out).unwrap();
 let dir=std::env::temp_dir().join(format!("corpse-unified-bench-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();let mut results=vec![];
 for name in cases.split(','){
  let large=name.starts_with("40-");let history=name.split('-').find_map(|s|s.strip_prefix('h').and_then(|v|v.parse::<usize>().ok())).unwrap_or(5);let op=if name.ends_with("wait"){Command::Wait}else{assert!(name.ends_with("pickup"),"case must end in wait or pickup");Command::Pickup};let stress=Stress::parse(name);
  let mut s=session(prepared(large,history,stress));let initial_payload=payload_metrics(state(&s));std::fs::write(out.join(format!("{name}-initial-world.json")),world(&s)).unwrap();if setup_only {results.push(serde_json::json!({"case":name,"starting_history":history,"stress_setup":stress.description(),"initial_payload":initial_payload,"setup_only":true}));continue;}let p=dir.join(format!("{name}.json"));let mut a=Autosave::default();let start=Instant::now();flush(&mut a,&p,&mut s,true);let initial=start.elapsed().as_nanos();let mut before=std::fs::read(&p).unwrap();let mut rows=vec![];let mut loads=vec![];
  for step in 1..=steps{
   let wall=Instant::now();let t=Instant::now();let outcome=command(&mut s,&op);let command_ns=t.elapsed().as_nanos();let t=Instant::now();flush(&mut a,&p,&mut s,false);let flush_ns=t.elapsed().as_nanos();let wall_ns=wall.elapsed().as_nanos();
   assert_eq!(state(&s).status,"playing");if matches!(op,Command::Wait){assert!(outcome.took_turn);}
   let after=std::fs::read(&p).unwrap();let checkpoint=!after.starts_with(&before);let written=if checkpoint{after.len()}else{after.len()-before.len()};before=after;
   rows.push(serde_json::json!({"step":step,"checkpoint":checkpoint,"command_ns":command_ns,"flush_ns":flush_ns,"wall_ns":wall_ns,"written_bytes":written,"file_bytes":before.len(),"payload_counts":payload_counts(state(&s))}));
   if step==31||checkpoint||step==steps{let t=Instant::now();let loaded=reload(&p);let ns=t.elapsed().as_nanos();assert_eq!(world(&s),world(&loaded),"{name}/{step}");std::fs::write(out.join(format!("{name}-step{step}.json")),serde_json::to_vec(&export(&s)).unwrap()).unwrap();loads.push(serde_json::json!({"step":step,"load_ns":ns,"checkpoint":checkpoint}));}
  }
  results.push(serde_json::json!({"case":name,"starting_history":history,"stress_setup":stress.description(),"initial_payload":initial_payload,"final_payload":payload_metrics(state(&s)),"initial_checkpoint_ns":initial,"samples":rows,"loads":loads}));eprintln!("completed {name}");
 }
 println!("{}",serde_json::to_string(&serde_json::json!({"implementation":if cfg!(unified_candidate){"unified"}else{"snapshot-overwrite-trial"},"profile":"normal release, collector disabled","setup_only":setup_only,"scope":"continuous controlled persistence, includes actual checkpoints+fsync; fixtures remove NPCs; file classification/verification outside timings; filesystem cache not forced cold; not UI latency","cases":results})).unwrap());let _=std::fs::remove_dir_all(dir);
}
