//! Native fixture runner: no shell, Node, JS VM, or browser dependency.
use corpse_core::{
    Command, Run, storage,
    temporal::{Engine, History},
};
use serde_json::{Value, json};
fn difference(a: &Value, b: &Value, p: &str) -> Option<String> {
    if storage::canonical_equal(a, b) {
        return None;
    }
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                let Some(w) = b.get(k) else {
                    return Some(format!("{p}.{k}: unexpected field"));
                };
                if let Some(d) = difference(v, w, &format!("{p}.{k}")) {
                    return Some(d);
                }
            }
            for k in b.keys() {
                if !a.contains_key(k) {
                    return Some(format!("{p}.{k}: missing field"));
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{p}: length {} != {}", a.len(), b.len()));
            }
            for (i, (v, w)) in a.iter().zip(b).enumerate() {
                if let Some(d) = difference(v, w, &format!("{p}[{i}]")) {
                    return Some(d);
                }
            }
        }
        _ => return Some(format!("{p}: {a} != {b}")),
    }
    None
}
fn compare_runs(a: &Run, b: &Run) -> Result<(), String> {
    let mut av = storage::canonical_value(a).map_err(|e| e.to_string())?;
    let mut bv = storage::canonical_value(b).map_err(|e| e.to_string())?;
    for key in ["history", "future", "hasteSnapshot"] {
        av.as_object_mut().unwrap().shift_remove(key);
        bv.as_object_mut().unwrap().shift_remove(key);
    }
    if let Some(d) = difference(&av, &bv, "$") {
        return Err(d);
    }
    let ah = History::import(a).map_err(|e| e.to_string())?;
    let bh = History::import(b).map_err(|e| e.to_string())?;
    if ah.frames.len() != bh.frames.len() {
        return Err("$.history frame count differs".into());
    }
    for (i, (a, b)) in ah.frames.iter().zip(bh.frames.iter()).enumerate() {
        if let Some(d) = difference(
            &a.value().map_err(|e| e.to_string())?,
            &b.value().map_err(|e| e.to_string())?,
            &format!("$.history[{i}]"),
        ) {
            return Err(d);
        }
    }
    match (&a.future, &b.future) {
        (None, None) => {}
        (Some(a), Some(b)) => compare_runs(
            &storage::deserialize(a.wire_text().map_err(|e|e.to_string())?).map_err(|e| e.to_string())?,
            &storage::deserialize(b.wire_text().map_err(|e|e.to_string())?).map_err(|e| e.to_string())?,
        )?,
        _ => return Err("$.future presence differs".into()),
    };
    match (ah.pending_haste, bh.pending_haste) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            if let Some(d) = difference(
                &a.value().map_err(|e| e.to_string())?,
                &b.value().map_err(|e| e.to_string())?,
                "$.hasteSnapshot",
            ) {
                return Err(d);
            }
        }
        _ => return Err("$.hasteSnapshot presence differs".into()),
    };
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        return Err("usage: corpse-fixtures check RUN.json | trace TRACE.json | run INITIAL.json COMMANDS.json | replay REPLAY.json".into());
    }
    let raw = std::fs::read_to_string(&a[2])?;
    match a[1].as_str() {
        "replay" => {
            let record = serde_json::from_str(&raw)?;
            let mut replay = corpse_core::replay::ReplaySession::create(record, true)?;
            while replay
                .step()
                .map_err(|e| format!("replay command {}: {e}", replay.cursor))?
            {}
            println!("{}", replay.engine.save()?);
        }
        "check" => {
            let r = storage::deserialize(&raw)?;
            let save = storage::serialize(&r)?;
            let round = storage::deserialize(&save)?;
            compare_runs(&r, &round).map_err(|e| format!("roundtrip: {e}"))?;
            println!(
                "{}",
                json!({"checked":true,"floors":r.floors.len(),"tick":r.tick,"history":r.history.len()})
            );
        }
        "trace" => {
            let fixture: Value = serde_json::from_str(&raw)?;
            let initial = serde_json::from_value(fixture["initial"].clone())?;
            let mut engine = Engine::new(initial)?;
            let steps = fixture["steps"].as_array().ok_or("missing steps")?;
            for (i, s) in steps.iter().enumerate() {
                let command = serde_json::from_value(s["command"].clone())?;
                engine
                    .dispatch(&command)
                    .map_err(|e| format!("step {i}: {e}"))?;
                let expected = serde_json::from_value(s["state"].clone())?;
                compare_runs(&engine.canonical_run()?, &expected)
                    .map_err(|e| format!("step {i}: {e}"))?;
            }
            println!(
                "{}",
                json!({"parity":true,"commands":steps.len(),"state":"all fields including reconstructed history and anchors"})
            );
        }
        "run" => {
            let commands: Vec<Command> = serde_json::from_str(&std::fs::read_to_string(
                a.get(3).ok_or("missing commands file")?,
            )?)?;
            let mut engine = Engine::load(&raw)?;
            for c in commands {
                engine.dispatch(&c)?;
            }
            println!("{}", engine.save()?);
        }
        _ => return Err("unknown operation".into()),
    }
    Ok(())
}
