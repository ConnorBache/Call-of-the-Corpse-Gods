//! Capture-only on/off experiment; recorder never becomes the persistence writer.
//! Opening Snapshot and exhaustive oracle are observers outside primary timings.
//! Typed compact gas encoding is normalized to the frozen oracle only AFTER wall timing.
#![allow(dead_code)]
#[path="../src/session_store.rs"] mod session_store;
use corpse_core::{capture_pilot,temporal::{Engine as Core,Snapshot},Command,Field,Run};
use session_store::{Autosave,Kind,tracked_engine::Engine as Session};
use std::{path::{Path,PathBuf},time::Instant,sync::Arc};

/// O(serialized world+history), detached export cache; outside all sample timers.
fn world(s:&Session)->Vec<u8>{let mut copy=s.clone();copy.set_legacy_revert(false);serde_json::to_vec(&copy.canonical_run().unwrap()).unwrap()}
/// O(file bytes+load), existing actual storage route; outside primary timing.
fn reload(p:&Path)->Session{Session::from_core(session_store::decode_bytes(&session_store::read_save_bytes(p).unwrap()).unwrap().0)}
/// O(evidence bytes), exact byte equality, never a hash-only gameplay assertion.
fn proof(out:&Path,reference:Option<&Path>,name:&str,bytes:&[u8]){if let Some(r)=reference{assert_eq!(std::fs::read(r.join(name)).unwrap(),bytes,"reference proof mismatch: {name}");}std::fs::write(out.join(name),bytes).unwrap();}
/// O(fixture world), ordinary controlled no-NPC scenario with H5 setup.
fn ordinary(large:bool)->Run {
 let mut r=if large{Core::load(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap()).unwrap().run}else{corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()};
 r.simulation_revision=39.0.into();for a in r.actors.values_mut(){*a=Arc::new(vec![]);}r.player.hp=1_000_000.;r.player.max_hp=1_000_000.;r.player.base_max_hp=1_000_000.0.into();r.player.energy=1_000_000.;r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;
 if large {let starter=r.inventory.iter().find(|i|i.kind=="weapon").unwrap().clone();while r.inventory.len()<20{let mut x=starter.clone();x.id=r.next_id;r.next_id+=1.;x.equipped=Field::Missing;r.inventory.push(x);}}
 r
}
/// O(room cells+actors), authored synthetic stress; setup is never a measurement.
fn room()->Run {
 let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:capture-pilot","Unblessed",false,39.).unwrap();r.player.x=2.;r.player.y=2.;r.player.hp=1_000_000.;r.player.max_hp=1_000_000.;r.player.base_max_hp=1_000_000.0.into();r.player.energy=100_000.;r.player.stats.dr=1000.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
 let(w,h)=(79,29);let mut tiles=vec!["wall".to_string();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
 let mut f=corpse_core::Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=4.;r.generated_ability_grants=Vec::new().into();r
}
/// O(actor count), deterministic real catalog actors with stable distinct IDs.
fn add_actor(r:&mut Run,kind:&str,x:f64,y:f64){let id=r.next_id;r.next_id+=1.;let mut a=corpse_core::population::create_actor_revision(kind,id,x,y,false,39.).unwrap();a.state="hunting".into();a.last_seen=corpse_core::ActorLastSeen{x:r.player.x,y:r.player.y}.into();r.actors_at_mut(1.).push(a);}
/// O(world+warmup Waits), actor-startup keeps H0; other fixtures retain five warmups.
fn prepared(name:&str)->Core {
 let mut r=match name{"10-h5-wait"=>ordinary(false),"40-h5-wait"=>ordinary(true),"dense-actors"|"actor-startup"=>{let mut r=room();for n in 0..32{add_actor(&mut r,"rat",(8+n%8)as f64,(5+n/8)as f64);}r},"dense-gas"=>{let mut r=room();let w=r.floor().unwrap().width();for y in (3..26).step_by(3){for x in (4..75).step_by(3){corpse_core::environment::add_gas(r.floor_mut().unwrap(),y*w+x,"caustic",100.,-1.).unwrap();}}r},_=>panic!("unknown case")};
 r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;let mut e=Core::load(&corpse_core::storage::serialize(&r).unwrap()).unwrap();for _ in 0..if name=="actor-startup"{0}else{5}{assert!(e.dispatch(&Command::Wait).unwrap().took_turn);}e
}
/// O(all physical gas cells), observer count outside every primary timer.
fn gas_nonzero(r:&Run)->usize {r.floors.iter().map(|f|f.environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0,|v|v.iter().filter(|&&x|x>0.).count())).sum()}
/// O(one birth action+selected full scan), deliberately UNTIMED safety proof.
fn birth_safety()->serde_json::Value {
 let mut r=room();add_actor(&mut r,"pale:Birther",5.,5.);r.tick=2.;let opening=Snapshot::capture(&r);let before=r.actors_at(1.).len();let scope=capture_pilot::Scope::begin();let player=r.player.id;let born=corpse_core::ai::enemy_action(&mut r,4.,player).unwrap();let report=scope.finish(&opening.state,&r);let oracle=capture_pilot::selected_oracle(&opening.state,&r);
 assert!(born&&r.actors_at(1.).len()>before,"Birther safety fixture must actually create children");assert!(!report.unsupported.is_empty(),"structural spawn must be explicitly unsupported");serde_json::json!({"untimed":true,"actors_before":before,"actors_after":r.actors_at(1.).len(),"report":report,"stable_selected_oracle":oracle,"note":"Actor membership change is unsupported; selected scalar equality is not complete spawn capture."})
}
/// O(cases*commands*(game+save+oracle)), orchestration only; no rendering claim.
fn main(){
 let enabled=match std::env::var("CAPTURE_ENABLED").as_deref(){Ok("1")=>true,Ok("0")=>false,_=>panic!("CAPTURE_ENABLED must be0or1")};let steps:usize=std::env::var("CAPTURE_STEPS").unwrap_or_else(|_|"160".into()).parse().unwrap();assert!(steps>=1);
 let out=PathBuf::from(std::env::var("CAPTURE_EVIDENCE_DIR").expect("evidence dir"));assert!(!out.exists(),"use a new evidence directory");std::fs::create_dir_all(&out).unwrap();let reference=std::env::var("CAPTURE_REFERENCE_DIR").ok().map(PathBuf::from);
 let cases=std::env::var("CAPTURE_CASES").unwrap_or_else(|_|"10-h5-wait,40-h5-wait,dense-actors,dense-gas".into());let temp=std::env::temp_dir().join(format!("corpse-capture-pilot-{}",std::process::id()));std::fs::create_dir_all(&temp).unwrap();let mut results=vec![];
 let safety=birth_safety();proof(&out,reference.as_deref(),"untimed-birth-safety.json",&serde_json::to_vec(&safety).unwrap());
 for name in cases.split(','){
  assert!(name.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'));let mut s=Session::from_core(prepared(name));let initial_floors=s.run.floors.len();let initial_actors=s.run.actors.values().map(|x|x.len()).sum::<usize>();let initial_nonzero_gas_cells=gas_nonzero(&s.run);let mut a=Autosave::default();let path=temp.join(format!("{name}.save"));proof(&out,reference.as_deref(),&format!("{name}-initial-world.json"),&world(&s));let t=Instant::now();a.flush_tracked(&path,&s,Kind::Fixture,true).unwrap();let initial_save_ns=t.elapsed().as_nanos();let mut prior=std::fs::read(&path).unwrap();proof(&out,reference.as_deref(),&format!("{name}-initial-save.bin"),&prior);let mut rows=vec![];let mut loads=vec![];let mut nonplayer_actor_change_records=0usize;let mut captured_nonplayer_actor_change_records=0usize;
  for step in 1..=steps{
   let observer=Instant::now();let opening=Snapshot::capture(&s.run);let opening_observer_ns=observer.elapsed().as_nanos();
   let wall=Instant::now();let t=Instant::now();let scope=enabled.then(capture_pilot::Scope::begin);let begin_ns=t.elapsed().as_nanos();let t=Instant::now();let outcome=s.dispatch(&Command::Wait).unwrap();let dispatch_ns=t.elapsed().as_nanos();let t=Instant::now();let mut report=scope.map(|scope|scope.finish(&opening.state,&s.run));let collect_ns=t.elapsed().as_nanos();let changes=report.as_mut().map(|r|std::mem::take(&mut r.changes));let t=Instant::now();let encoded=changes.as_ref().map(|x|serde_json::to_vec(x).unwrap());let encode_ns=t.elapsed().as_nanos();let t=Instant::now();drop(changes);let collected_values_drop_ns=t.elapsed().as_nanos();let t=Instant::now();a.flush_tracked(&path,&s,Kind::Fixture,false).unwrap();let flush_ns=t.elapsed().as_nanos();let wall_ns=wall.elapsed().as_nanos();
   assert!(outcome.took_turn);assert_eq!(s.run.status,"playing");
   let outcome_proof=serde_json::to_vec(&serde_json::json!({"command":{"kind":"Wait"},"changed":outcome.changed,"took_turn":outcome.took_turn,"events":outcome.events,"rng_bits":s.run.rng.to_bits(),"tick_bits":s.run.tick.to_bits(),"depth_bits":s.run.depth.to_bits()})).unwrap();proof(&out,reference.as_deref(),&format!("{name}-step{step}-outcome.json"),&outcome_proof);
   let oracle_start=Instant::now();let oracle=capture_pilot::selected_oracle(&opening.state,&s.run);let oracle_bytes=serde_json::to_vec(&oracle).unwrap();let oracle_observer_ns=oracle_start.elapsed().as_nanos();let normalization_start=Instant::now();let normalized=encoded.as_ref().map(|x|capture_pilot::normalize_changes(&serde_json::from_slice::<Vec<capture_pilot::Change>>(x).unwrap()).unwrap());let normalized_bytes=normalized.as_ref().map(|x|serde_json::to_vec(x).unwrap());let normalization_observer_ns=normalization_start.elapsed().as_nanos();let exact_match=normalized_bytes.as_ref().map(|x|x==&oracle_bytes);
   let nonplayer_changes=oracle.iter().filter(|c|matches!(c,capture_pilot::Change::Actor{floor:Some(_),..})).count();nonplayer_actor_change_records+=nonplayer_changes;
   let captured_nonplayer=normalized.as_ref().map(|v|v.iter().filter(|c|matches!(c,capture_pilot::Change::Actor{floor:Some(_),..})).count());captured_nonplayer_actor_change_records+=captured_nonplayer.unwrap_or(0);
   let expected_set:std::collections::BTreeSet<String>=oracle.iter().map(|v|serde_json::to_string(v).unwrap()).collect();
   let actual_set:Option<std::collections::BTreeSet<String>>=normalized.as_ref().map(|v|v.iter().map(|v|serde_json::to_string(v).unwrap()).collect());
   let missing_selected=actual_set.as_ref().map(|v|expected_set.difference(v).count());let extra_selected=actual_set.as_ref().map(|v|v.difference(&expected_set).count());
   if let Some(false)=exact_match{proof(&out,None,&format!("{name}-step{step}-capture-mismatch.json"),encoded.as_ref().unwrap());proof(&out,None,&format!("{name}-step{step}-normalized-capture-mismatch.json"),normalized_bytes.as_ref().unwrap());proof(&out,None,&format!("{name}-step{step}-oracle-mismatch.json"),&oracle_bytes);assert!(!report.as_ref().unwrap().unsupported.is_empty(),"unflagged selected-capture mismatch at {name}/{step}");}
   let after=std::fs::read(&path).unwrap();let checkpoint=!after.starts_with(&prior);let written=if checkpoint{after.as_slice()}else{&after[prior.len()..]};proof(&out,reference.as_deref(),&format!("{name}-step{step}-write.bin"),written);let written_bytes=written.len();let file_bytes=after.len();prior=after;
   let stage_sum=begin_ns+dispatch_ns+collect_ns+encode_ns+collected_values_drop_ns+flush_ns;assert!(wall_ns>=stage_sum);
   rows.push(serde_json::json!({"step":step,"command":{"kind":"Wait"},"nonplayer_actor_change_records":nonplayer_changes,"captured_nonplayer_actor_change_records":captured_nonplayer,"checkpoint":checkpoint,"wall_ns":wall_ns,"dispatch_ns":dispatch_ns,"flush_ns":flush_ns,"command_plus_flush_ns":dispatch_ns+flush_ns,"capture_begin_ns":begin_ns,"collect_ns":collect_ns,"encode_ns":encode_ns,"collected_values_drop_ns":collected_values_drop_ns,"unattributed_timer_ns":wall_ns-stage_sum,"opening_observer_ns":opening_observer_ns,"oracle_observer_ns":oracle_observer_ns,"normalization_observer_ns":normalization_observer_ns,"capture_counters":report.as_ref().map(|r|&r.counters),"unsupported":report.as_ref().map(|r|&r.unsupported),"selected_oracle_records":oracle.len(),"selected_oracle_exact_match":exact_match,"missing_selected_records":missing_selected,"extra_selected_records":extra_selected,"new_value_payload_bytes":encoded.as_ref().map(Vec::len),"written_bytes":written_bytes,"file_bytes":file_bytes}));
   if checkpoint||step==steps{let w=world(&s);proof(&out,reference.as_deref(),&format!("{name}-step{step}-world.json"),&w);let t=Instant::now();let loaded=reload(&path);let load_ns=t.elapsed().as_nanos();assert_eq!(world(&loaded),w);loads.push(serde_json::json!({"step":step,"load_ns":load_ns}));}
  }
  if name=="actor-startup"{assert!(nonplayer_actor_change_records>0,"startup trace must change nonplayer actor components");if enabled{assert!(captured_nonplayer_actor_change_records>0,"startup recorder must capture nonplayer actor changes");}}
  results.push(serde_json::json!({"case":name,"warmup_turns":if name=="actor-startup"{0}else{5},"nonplayer_actor_change_records":nonplayer_actor_change_records,"captured_nonplayer_actor_change_records":if enabled{Some(captured_nonplayer_actor_change_records)}else{None},"initial_floors":initial_floors,"initial_actors":initial_actors,"initial_nonzero_gas_cells":initial_nonzero_gas_cells,"final_nonzero_gas_cells":gas_nonzero(&s.run),"initial_save_ns":initial_save_ns,"samples":rows,"loads":loads}));eprintln!("completed {name}");
 }
 println!("{}",serde_json::to_string(&serde_json::json!({"capture_enabled":enabled,"payload_representation":"typed-gas-compact-tuples-v1","oracle_comparison":"normalized verbose selected tuples; normalization outside wall","steps":steps,"same_binary_inactive_hooks_in_control":true,"opening_snapshot_observer_outside_wall":true,"scope":"Capture-only selected actor/gas pilot; actual existing command persistence+fsync unchanged. Collector encoding bytes are NOT saved. Oracle/proofs outsidetimers; filesystem cache notforcedcold; no rendering claim.","birth_safety":safety,"cases":results})).unwrap());let _=std::fs::remove_dir_all(temp);
}
