//! Cache-only comparison: run identically against v18 and guarded-identity source.
//! SAVE_BENCH_IMPL=candidate for BOTH: both retain v18 direct envelope encoding.
//! Report command, immediate save and enclosing total: eager sealing is not hidden.
#![allow(dead_code)]
#[path = "../src/session_store.rs"] mod session_store;
#[path = "save_envelope_bench/frozen_history.rs"] mod frozen_history;
use corpse_core::{temporal::{Engine,History},Command};
use session_store::Kind;
use std::time::Instant;

/// O(frozen export + B) time/memory, includes old String -> Value envelope parse.
fn legacy_encode(engine:&Engine)->String {
    #[derive(serde::Serialize)] #[serde(rename_all="camelCase")]
    struct Envelope {native_save_version:u32,session:Kind,run:serde_json::Value}
    let raw=corpse_core::storage::serialize(&frozen_history::export(engine)).unwrap();
    serde_json::to_string(&Envelope{native_save_version:1,session:Kind::Fixture,run:serde_json::from_str(&raw).unwrap()}).unwrap()
}
/// O(Run shallow+COW clone + H) without history import/reconstruction. H<=5.
/// New History owns a fresh export cache; immutable snapshot/delta payloads share.
fn fresh(source:&Engine)->Engine {
    let mut history=History::default();history.frames=source.history.frames.clone();
    history.pending_haste=source.history.pending_haste.clone();history.closing=source.history.closing.clone();
    Engine{run:source.run.clone(),history}
}
/// O(N log N) outside measured paths; nearest-rank p95 and upper middle median.
fn summary(samples:&[f64])->serde_json::Value {
    let mut sorted=samples.to_vec();sorted.sort_by(f64::total_cmp);
    serde_json::json!({"n":sorted.len(),"median_ms":sorted[sorted.len()/2],"p95_ms":sorted[(sorted.len()*95).div_ceil(100)-1],"samples_ms":samples})
}
struct Case {name:String,operation:Command,expected_took_turn:bool,source:Engine,expected:String,command:Vec<f64>,save:Vec<f64>,total:Vec<f64>,profiles:Vec<serde_json::Value>}
/// O(cases*(generation + commands + export) + samples*(setup + command + save)).
/// Setup, correctness comparison, output and evidence writes are outside timing.
fn main() {
    let implementation=std::env::var("SAVE_BENCH_IMPL").unwrap_or_else(|_|"candidate".into());
    assert!(["legacy","candidate"].contains(&implementation.as_str()));
    let samples=std::env::var("SAVE_BENCH_SAMPLES").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(20).max(1);
    let evidence=std::env::var("SAVE_BENCH_EVIDENCE_DIR").ok().map(std::path::PathBuf::from);
    if let Some(dir)=&evidence {std::fs::create_dir_all(dir).unwrap();}
    let avatar=Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let generated=Engine::new(corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()).unwrap();
    // Real anchor serialization through the temporal helper, with its energy cost.
    // This stresses metadata guard size; it is not an ordinary-population claim.
    let mut future=Engine::new(generated.run.clone()).unwrap();
    corpse_core::temporal::remember_future(&mut future.run,&future.history).unwrap();
    future=Engine::new(future.run).unwrap();
    let large_raw=std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap();
    let mut large=Engine::load(&large_raw).unwrap();
    let starter=large.run.inventory.iter().find(|i|i.kind=="weapon").unwrap().clone();
    while large.run.inventory.len()<20 {let mut item=starter.clone();item.id=large.run.next_id;large.run.next_id+=1.;item.equipped=corpse_core::Field::Missing;large.run.inventory.push(item);}
    // Synthetic full inventory, not a claim these items were earned normally.
    large=Engine::new(large.run).unwrap();
    let commands=std::env::var("SAVE_BENCH_COMMANDS").unwrap_or_else(|_|"wait".into());
    let operations:Vec<(&str,Command)>=commands.split(',').map(|name|match name {"wait"=>("wait",Command::Wait),"pickup"=>("pickup",Command::Pickup),_=>panic!("unknown benchmark command")}).collect();
    let profiled=cfg!(feature="runtime-timing") && std::env::var("SAVE_BENCH_PROFILE").is_ok_and(|v|v=="1");
    let mut cases=Vec::new();
    for (name,mut source) in [("avatar_fixture",avatar),("generated_10_floors",generated),("legacy_40_floors_full_pack",large),("generated_10_floors_future_anchor",future)] {
        for turns in 0..=5 {
            if [0,1,5].contains(&turns) {
                for (operation_name,operation) in &operations {
                    let mut probe=fresh(&source);let outcome=probe.dispatch(operation).unwrap();
                    let expected=legacy_encode(&probe);
                    assert_eq!(session_store::encode(&probe,Kind::Fixture).unwrap(),expected);
                    let name=format!("{name}-history-{turns}-{operation_name}");
                    if let Some(dir)=&evidence {std::fs::write(dir.join(format!("{name}.json")),&expected).unwrap();}
                    cases.push(Case{name,operation:operation.clone(),expected_took_turn:outcome.took_turn,source:fresh(&source),expected,command:Vec::new(),save:Vec::new(),total:Vec::new(),profiles:Vec::new()});
                }
            }
            if turns<5 {source.dispatch(&Command::Wait).unwrap();}
        }
    }
    // Round-robin cases, alternating traversal direction to reduce fixed-order bias.
    for sample in 0..samples {
        let indices:Vec<usize>=if sample%2==0 {(0..cases.len()).collect()}else{(0..cases.len()).rev().collect()};
        for index in indices {
            let case=&mut cases[index];let mut engine=fresh(&case.source);
            // Warm the existing snapshot/export metadata before the new command.
            std::hint::black_box(engine.save().unwrap());
            #[cfg(feature="runtime-timing")] {corpse_core::timing::reset();corpse_core::timing::set_enabled(profiled);}
            let total_start=Instant::now();let root=corpse_core::timing::span("benchmark.command_plus_save");let command_start=Instant::now();
            let outcome=engine.dispatch(&case.operation).unwrap();
            let command_ms=command_start.elapsed().as_secs_f64()*1000.;
            let save_start=Instant::now();
            let bytes=if implementation=="legacy" {legacy_encode(&engine)}else{session_store::encode(&engine,Kind::Fixture).unwrap()};
            let save_ms=save_start.elapsed().as_secs_f64()*1000.;
            drop(root);let total_ms=total_start.elapsed().as_secs_f64()*1000.;
            #[cfg(feature="runtime-timing")] {let report=corpse_core::timing::take_report();corpse_core::timing::set_enabled(false);if profiled {case.profiles.push(serde_json::to_value(report).unwrap());}}
            assert_eq!(outcome.took_turn,case.expected_took_turn,"{} advancement changed",case.name);
            assert_eq!(bytes,case.expected,"{} sample{sample}",case.name);
            case.command.push(command_ms);case.save.push(save_ms);case.total.push(total_ms);
        }
        eprintln!("completed sample {}/{}",sample+1,samples);
    }
    let rows:Vec<_>=cases.iter().map(|c|serde_json::json!({"case":c.name,"starting_frames":c.source.history.frames.len(),"bytes":c.expected.len(),"command_inclusive":summary(&c.command),"immediate_save_inclusive":summary(&c.save),"command_plus_save_inclusive":summary(&c.total),"profile_reports":c.profiles})).collect();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"implementation":implementation,"profile":if profiled {"instrumented; not wall-time baseline"} else {"normal collector-off"},"scope":"warm metadata, one command then immediate encode; no durable disk flush","cases":rows})).unwrap());
}
