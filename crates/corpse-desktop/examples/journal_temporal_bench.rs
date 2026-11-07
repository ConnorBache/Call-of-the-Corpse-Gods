//! Identical original-source baseline versus journal temporal command benchmark.
//! Verify baseline source provenance; external legacy encode is independently frozen.
#![allow(dead_code)]
#[path="../src/session_store.rs"] mod session_store;
#[path="save_envelope_bench/frozen_history.rs"] mod frozen_history;
use corpse_core::{temporal::{Engine,History},Command};
use session_store::Kind;
use std::time::Instant;

/// O(frozen history export + output bytes), including original envelope parse.
fn legacy_encode(engine:&Engine)->String {
    #[derive(serde::Serialize)] #[serde(rename_all="camelCase")]
    struct Envelope {native_save_version:u32,session:Kind,run:serde_json::Value}
    let raw=corpse_core::storage::serialize(&frozen_history::export(engine)).unwrap();
    serde_json::to_string(&Envelope{native_save_version:1,session:Kind::Fixture,run:serde_json::from_str(&raw).unwrap()}).unwrap()
}
/// O(Run shallow/COW clone + H). Preserves pending haste and immutable snapshots.
/// Private pending-write evidence starts empty; immutable snapshot fallback is required.
fn fresh(source:&Engine)->Engine {
    let mut history=History::default();history.frames=source.history.frames.clone();
    history.pending_haste=source.history.pending_haste.clone();history.closing=source.history.closing.clone();
    Engine{run:source.run.clone(),history}
}
/// O(N log N), outside timing; nearest-rank p95 and upper middle median.
fn summary(samples:&[f64])->serde_json::Value {
    let mut sorted=samples.to_vec();sorted.sort_by(f64::total_cmp);
    serde_json::json!({"n":sorted.len(),"median_ms":sorted[sorted.len()/2],"p95_ms":sorted[(sorted.len()*95).div_ceil(100)-1],"samples_ms":samples})
}
struct Case {name:String,source:Engine,operation:Command,expected:String,expected_took_turn:bool,command:Vec<f64>,save:Vec<f64>,total:Vec<f64>,profiles:Vec<serde_json::Value>}
/// O(trace setup + samples*(clone/warm + command + save)); setup/assertions outside timing.
fn main() {
    let implementation=std::env::var("SAVE_BENCH_IMPL").unwrap_or_else(|_|"candidate".into());
    assert!(["legacy","candidate"].contains(&implementation.as_str()));
    let samples=std::env::var("SAVE_BENCH_SAMPLES").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(20).max(1);
    let evidence=std::env::var("SAVE_BENCH_EVIDENCE_DIR").ok().map(std::path::PathBuf::from);
    if let Some(dir)=&evidence {std::fs::create_dir_all(dir).unwrap();}
    let profiled=cfg!(feature="runtime-timing") && std::env::var("SAVE_BENCH_PROFILE").is_ok_and(|v|v=="1");
    let traces:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/temporal-command-traces-v48.json")).unwrap();
    let specifications=[
        ("revert_completed_free","revert completed and free actions",4usize),
        ("revert_pending_haste_free","revert pending haste and free actions",3usize),
        ("future_creation","future retained costs and resume",1usize),
        ("future_restore","future retained costs and resume",5usize),
    ];
    let mut cases=Vec::new();
    for (name,trace_name,target) in specifications {
        let trace=traces.as_array().unwrap().iter().find(|c|c["name"]==trace_name).unwrap();
        let mut source=Engine::new(serde_json::from_value(trace["initial"].clone()).unwrap()).unwrap();
        let steps=trace["steps"].as_array().unwrap();
        for step in &steps[..target] {
            let command:Command=serde_json::from_value(step["command"].clone()).unwrap();source.dispatch(&command).unwrap();
            if step["resume"]==true {source=Engine::load(&source.save().unwrap()).unwrap();}
        }
        let operation:Command=serde_json::from_value(steps[target]["command"].clone()).unwrap();
        let mut probe=fresh(&source);let outcome=probe.dispatch(&operation).unwrap();let expected=legacy_encode(&probe);
        assert_eq!(session_store::encode(&probe,Kind::Fixture).unwrap(),expected);
        if let Some(dir)=&evidence {std::fs::write(dir.join(format!("{name}.json")),&expected).unwrap();}
        cases.push(Case{name:name.into(),source,operation,expected,expected_took_turn:outcome.took_turn,command:Vec::new(),save:Vec::new(),total:Vec::new(),profiles:Vec::new()});
    }
    for sample in 0..samples {
        let indices:Vec<usize>=if sample%2==0 {(0..cases.len()).collect()}else{(0..cases.len()).rev().collect()};
        for index in indices {
            let case=&mut cases[index];let mut engine=fresh(&case.source);
            std::hint::black_box(engine.save().unwrap());
            #[cfg(feature="runtime-timing")] {corpse_core::timing::reset();corpse_core::timing::set_enabled(profiled);}
            let total_start=Instant::now();let root=corpse_core::timing::span("benchmark.temporal_command_plus_save");let command_start=Instant::now();
            let outcome=engine.dispatch(&case.operation).unwrap();let command_ms=command_start.elapsed().as_secs_f64()*1000.;
            let save_start=Instant::now();let bytes=if implementation=="legacy" {legacy_encode(&engine)}else{session_store::encode(&engine,Kind::Fixture).unwrap()};
            let save_ms=save_start.elapsed().as_secs_f64()*1000.;drop(root);let total_ms=total_start.elapsed().as_secs_f64()*1000.;
            #[cfg(feature="runtime-timing")] {let report=corpse_core::timing::take_report();corpse_core::timing::set_enabled(false);if profiled {case.profiles.push(serde_json::to_value(report).unwrap());}}
            assert_eq!(outcome.took_turn,case.expected_took_turn);
            assert_eq!(bytes,case.expected,"{} sample{sample}",case.name);
            case.command.push(command_ms);case.save.push(save_ms);case.total.push(total_ms);
        }
        eprintln!("completed temporal sample {}/{}",sample+1,samples);
    }
    let rows:Vec<_>=cases.iter().map(|c|serde_json::json!({"case":c.name,"starting_frames":c.source.history.frames.len(),"pending_haste":c.source.history.pending_haste.is_some(),"bytes":c.expected.len(),"command_inclusive":summary(&c.command),"immediate_save_inclusive":summary(&c.save),"command_plus_save_inclusive":summary(&c.total),"profile_reports":c.profiles})).collect();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"implementation":implementation,"baseline_note":std::env::var("SAVE_BENCH_BASELINE_LABEL").unwrap_or_else(|_|"verify baseline build source provenance; external legacy encode frozen".into()),"profile":if profiled {"instrumented; not wall-time baseline"}else{"normal collector-off"},"scope":"one temporal command then immediate encode; no disk flush or post-command reload","cases":rows})).unwrap());
}
