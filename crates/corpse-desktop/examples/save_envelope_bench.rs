//! Full old-export/envelope versus candidate. Release, no runtime-timing, quiet CPU.
//! Frozen normal-run reference excludes DEBUG loadout edits (asserted absent).
#![allow(dead_code)]
#[path = "../src/session_store.rs"]
mod session_store;
#[path = "save_envelope_bench/frozen_history.rs"]
mod frozen_history;
use corpse_core::{temporal::Engine, Command, Run};
use session_store::{Autosave, Kind};
use std::time::Instant;

/// Previous desktop algorithm plus frozen original export; includes redundant JSON parse.
/// O(T_legacy_export+B) time/O(M_legacy_export+B) memory.
fn legacy_encode(engine: &Engine) -> String {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Envelope { native_save_version: u32, session: Kind, run: serde_json::Value }
    let raw=corpse_core::storage::serialize(&frozen_history::export(engine)).unwrap();
    let run = serde_json::from_str(&raw).unwrap();
    serde_json::to_string(&Envelope { native_save_version: 1, session: Kind::Fixture, run }).unwrap()
}
/// O(N log N) summary outside timed paths; nearest-rank p95, upper middle median.
fn summary(samples: &[f64]) -> serde_json::Value {
    let mut sorted = samples.to_vec(); sorted.sort_by(f64::total_cmp);
    serde_json::json!({"n":sorted.len(),"median_ms":sorted[sorted.len()/2],"p95_ms":sorted[(sorted.len()*95).div_ceil(100)-1],"samples_ms":samples})
}
/// Setup excluded from timings: O(history import + optional export + dispatch).
/// New engines avoid shared runtime cache contamination between ABBA samples.
fn prepared(canonical: &Run, mode: &str) -> Engine {
    let mut engine=Engine::new(canonical.clone()).unwrap();
    if mode!="cold" {std::hint::black_box(engine.save().unwrap());}
    if mode=="changed_turn" {engine.dispatch(&Command::Wait).unwrap();}
    engine
}
/// Optional diagnostic scopes; O(encode + labels), outside normal measurements.
fn profile(canonical: &Run, mode: &str, expected: &str) -> serde_json::Value {
    #[cfg(feature = "runtime-timing")]
    {
        let engine = prepared(canonical, mode);
        corpse_core::timing::reset();
        corpse_core::timing::set_enabled(true);
        let actual = {
            let _root = corpse_core::timing::span("sample.save_encode");
            session_store::encode(&engine, Kind::Fixture).unwrap()
        };
        corpse_core::timing::set_enabled(false);
        assert!(actual == expected, "profiled bytes differ");
        let report = corpse_core::timing::take_report();
        assert_eq!(report["sample.save_encode"].inclusive_ns,
            report.values().map(|v| v.exclusive_ns).sum::<u128>());
        return serde_json::to_value(report).unwrap();
    }
    #[cfg(not(feature = "runtime-timing"))]
    { let _ = (canonical, mode, expected); serde_json::Value::Null }
}
/// O(N*(setup+export+B)) experiment. Assertions/setup/summary outside measured intervals.
fn main() {
    let avatar=Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let generated=Engine::new(corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap()).unwrap();
    let large_raw = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap();
    let mut large = Engine::load(&large_raw).unwrap();
    let starter = large.run.inventory.iter().find(|i| i.kind == "weapon").unwrap().clone();
    while large.run.inventory.len() < 20 {
        let mut item = starter.clone();
        item.id = large.run.next_id;
        large.run.next_id += 1.;
        item.equipped = corpse_core::Field::Missing;
        large.run.inventory.push(item);
    }
    // Synthetic full-pack size stress, not a claim that these items were earned.
    large = Engine::new(large.run).unwrap();
    let dir=std::env::temp_dir().join(format!("corpse-save-envelope-bench-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap(); let path=dir.join("save.json");
    let rounds=std::env::var("SAVE_BENCH_ROUNDS").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(10).max(1);
    // Exact case selectors; unset preserves the complete existing matrix.
    let filter=std::env::var("SAVE_BENCH_FILTER").ok().map(|value|value.split(',').map(str::to_owned).collect::<std::collections::HashSet<_>>());
    let mut results=Vec::new();
    for (name,mut source) in [("avatar_fixture",avatar),("generated_10_floors",generated),("legacy_40_floors_full_pack",large)] {
        for turns in 0..=5 {
            if [0,1,5].contains(&turns) {
                let canonical=frozen_history::export(&source);
                for mode in ["cold","warm","changed_turn"] {
                    if filter.as_ref().is_some_and(|selected|!selected.contains(&format!("{name}/{turns}/{mode}"))) {continue;}
                    let oracle=prepared(&canonical,mode);
                    let expected=legacy_encode(&oracle);
                    assert_eq!(session_store::encode(&oracle,Kind::Fixture).unwrap(),expected);
                    let mut old=Vec::new();let mut new=Vec::new();
                    for _ in 0..rounds {
                        for candidate in [false,true,true,false] {
                            let engine=prepared(&canonical,mode);
                            let start=Instant::now();
                            let bytes=if candidate {session_store::encode(&engine,Kind::Fixture).unwrap()} else {legacy_encode(&engine)};
                            let ms=start.elapsed().as_secs_f64()*1000.0;
                            if candidate {new.push(ms)} else {old.push(ms)}
                            assert_eq!(bytes,expected,"{name}/{turns}/{mode}");
                        }
                    }
                    // Inclusive candidate durable flush: same cache setup as encode case.
                    // Exact old-file read/check + encoding + sync/rename; not additive to encode.
                    let mut auto=Autosave::default();
                    auto.flush(&path,&oracle,Kind::Fixture,true).unwrap();
                    let mut flush=Vec::new();
                    for _ in 0..rounds*2 {
                        let engine=prepared(&canonical,mode);
                        let start=Instant::now(); auto.flush(&path,&engine,Kind::Fixture,true).unwrap();
                        flush.push(start.elapsed().as_secs_f64()*1000.0);
                        assert_eq!(std::fs::read_to_string(&path).unwrap(),expected);
                    }
                    let scopes = profile(&canonical, mode, &expected);
                    let result=serde_json::json!({"candidate_profile":scopes,"case":name,"starting_turns":turns,"frames":oracle.history.frames.len(),"cache":mode,"bytes":expected.len(),"legacy_export_and_envelope":summary(&old),"candidate_export_and_envelope":summary(&new),"candidate_durable_flush_inclusive":summary(&flush)});
                    eprintln!("completed {name}/{turns}/{mode}");results.push(result);
                }
            }
            if turns<5 {source.dispatch(&Command::Wait).unwrap();}
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
    println!("{}",serde_json::to_string_pretty(&results).unwrap());
}
