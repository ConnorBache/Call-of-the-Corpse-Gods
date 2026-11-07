//! Reproducible paid/free-command timing, independent cloned starts per sample.
//! Setup/generation and equivalence checks are outside timing. New counters never
//! enter Run/RNG/save. O(samples * (dispatch cost + state clone/verification));
//! report storage O(samples * labels + state bytes). See timing-r39 README.
use corpse_core::{Run, Command, temporal::Engine};
use serde_json::{json,Value};
use std::{time::Instant, hint::black_box};

/// O(n log n) summary sorting; raw samples retained by caller.
fn summary(v:&[u128])->Value {let mut s=v.to_vec();s.sort();let n=s.len();json!({"n":n,"median_ns":if n%2==0 {(s[n/2-1] as f64+s[n/2] as f64)/2.}else{s[n/2] as f64},"p95_ns":s[((n*95).div_ceil(100)).saturating_sub(1).min(n-1)],"min_ns":s[0],"max_ns":s[n-1]})}
/// O(argv length), intended bounded CLI diagnostic inputs.
fn arg(name:&str,default:&str)->String {let a:Vec<_>=std::env::args().collect();a.windows(2).find(|x|x[0]==name).map(|x|x[1].clone()).unwrap_or(default.into())}
/// O(L) clearing L labels with runtime-timing; O(1) feature-off. Outside measured region.
fn enable(on:bool) {#[cfg(feature="runtime-timing")] {corpse_core::timing::set_enabled(on);corpse_core::timing::reset();} #[cfg(not(feature="runtime-timing"))] let _=on;}
/// O(L) JSON projection of accumulated labels, outside measured region.
fn counters()->Value {#[cfg(feature="runtime-timing")] {serde_json::to_value(corpse_core::timing::take_report()).unwrap()} #[cfg(not(feature="runtime-timing"))] {json!({})}}
/// O(serialized state bytes); deterministic noncryptographic fingerprint for cross-build parity.
fn fingerprint(bytes:&[u8])->String {let mut h=0xcbf29ce484222325u64;for b in bytes {h^=*b as u64;h=h.wrapping_mul(0x100000001b3);}format!("{h:016x}")}
/// O(dispatch) timed; setup and complete canonical save comparison intentionally outside.
fn sample(initial:&Run, command:&Command,on:bool)->(u128,Value,String,f64,bool) {
 enable(false);let mut e=Engine::new(initial.clone()).expect("scenario validation");enable(on);
 let start=Instant::now();let result={let _scope=corpse_core::timing::span("sample.command");black_box(e.dispatch(command).expect("command dispatch"))};let elapsed=start.elapsed().as_nanos();
 let report=counters();enable(false);let ticks=e.run.tick-initial.tick;
 let state=e.save().expect("canonical save");(elapsed,report,state,ticks,result.took_turn)
}
/// O(generation + scenario cloning) setup, excluded from measurements.
fn scenarios()->Vec<(String,String,Run)> {
 let input=arg("--input",""); if !input.is_empty(){let raw=std::fs::read_to_string(input).unwrap();let r=serde_json::from_str::<Run>(&raw).expect("Run JSON input");return vec![("input".into(),"caller supplied Run JSON".into(),r)];}
 let base=corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).expect("seed generation");
 let mut empty=base.clone();for group in empty.actors.values_mut(){std::sync::Arc::make_mut(group).clear();}
 let mut crowded=base.clone();
 let template=base.actors.values().flat_map(|g|g.iter()).find(|a|a.creature_kind.as_ref().is_some_and(|k|k=="rat")).cloned().expect("generated rat template");
 let f=base.floor().unwrap(); let width=f.width();
 let cells:Vec<_>=f.tiles.iter().enumerate().filter(|(i,t)|matches!(t.as_str(),"floor"|"grass") && !base.actors_at(base.depth).iter().any(|a|a.x==(*i%width) as f64&&a.y==(*i/width) as f64) && ((*i%width) as f64-base.player.x).abs()+((*i/width) as f64-base.player.y).abs()>4.).map(|(i,_)|i).take(32).collect();
 assert_eq!(cells.len(),32,"crowded fixture must add exactly 32 rats");
 for cell in cells {let mut actor=template.clone();actor.id=crowded.next_id;crowded.next_id+=1.;actor.x=(cell%width)as f64;actor.y=(cell/width)as f64;corpse_core::ai::wake(&mut actor,true);actor.state="hunting".into();actor.last_seen=corpse_core::ActorLastSeen{x:base.player.x,y:base.player.y}.into();actor.conditions.insert("targetId".into(),base.player.id);crowded.actors_at_mut(base.depth).push(actor);}
 let mut involved=base.clone();for f in involved.floors.iter_mut().take(3){let f=std::sync::Arc::make_mut(f);f.seen.fill(true);f.last_tick=0.0.into();f.dirty=true;}
 vec![("generated-entry".into(),"Actual generated r39 seed, first floor entry, unchanged".into(),base),("synthetic-no-actors".into(),"Same generated world with all NPCs removed; terrain/hazards retained, NOT an empty-map case".into(),empty),("synthetic-crowded-plus32rats".into(),"Generated floor plus 32 cloned generated rats at distinct passable cells, explicitly awakened/hunting with last-seen player position; synthetic pursuit stress case, not normal population".into(),crowded),("synthetic-three-involved".into(),"Same generated world; first three floors marked seen/dirty to exercise inactive-floor work".into(),involved)]
}
/// O(scenarios * commands * repetitions * measured pipeline + report serialization).
fn main(){
 let samples=arg("--samples","12").parse::<usize>().unwrap().clamp(3,1000);let warmup=arg("--warmup","2").parse::<usize>().unwrap().min(100);
 let mut output=vec![];let export=arg("--export-input","");
 for(name,description,run)in scenarios(){
  if !export.is_empty()&&name=="generated-entry"{std::fs::write(&export,serde_json::to_string(&run).unwrap()).unwrap();}
  for (command_name,command) in [("wait",Command::Wait),("pickup",Command::Pickup)] {
   let(_,_,expected,_,_)=sample(&run,&command,false);let expected_hash=fingerprint(expected.as_bytes());let state_dir=arg("--state-dir","");if !state_dir.is_empty(){std::fs::create_dir_all(&state_dir).unwrap();std::fs::write(std::path::Path::new(&state_dir).join(format!("{name}-{command_name}.json")),&expected).unwrap();}
   let mut off=vec![];let mut on=vec![];let mut details=vec![];let mut ticks=0.;let mut paid=false;
   for i in 0..warmup+samples {
    // Alternate order to reduce thermal/time-order bias.
    for enabled in if i%2==0 {[false,true]} else {[true,false]} {
     let(ns,report,state,t,p)=sample(&run,&command,enabled);assert_eq!(state,expected,"profiling changed deterministic output for {name}/{command_name}");ticks=t;paid=p;
     if i>=warmup {if enabled {on.push(ns);details.push(report);}else{off.push(ns);}}
    }
   }
   output.push(json!({"scenario":name,"description":description,"seed":run.seed,"depth":run.depth,"floor_cells":run.floor().unwrap().tiles.len(),"active_actors":run.actors_at(run.depth).len(),"stored_floors":run.floors.len(),"visible_tiles":run.floor().unwrap().seen.iter().filter(|&&v|v).count(),"command":command_name,"ticks_advanced":ticks,"took_turn":paid,"deterministic_save_equal":true,"canonical_save_fnv1a64":expected_hash,"collector_disabled":summary(&off),"collector_enabled":summary(&on),"raw_disabled_ns":off,"raw_enabled_ns":on,"counter_samples":details}));
  }
 }
 let report=json!({"schema":1,"runtime_timing_feature":cfg!(feature="runtime-timing"),"sample_policy":"independent cloned starting state per command; setup/generation/verification and DispatchOutcome disposal excluded; interleaved collection on/off; inclusive labels overlap, exclusive labels do not within root; collector map reset each sample includes per-label allocations when enabled; disabled collector in feature build still has TLS/branch overhead, compare separate normal build","warmup":warmup,"samples":samples,"scenarios":output});
 let text=serde_json::to_string_pretty(&report).unwrap();let output=arg("--output","");if output.is_empty(){println!("{text}")}else{std::fs::write(output,text).unwrap();}
}
