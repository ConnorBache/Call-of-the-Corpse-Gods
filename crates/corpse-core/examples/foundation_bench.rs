use corpse_core::{
    Command, Run,
    temporal::{Engine, Snapshot},
};
use std::{hint::black_box, time::Instant};
fn stats(mut samples: Vec<f64>) -> serde_json::Value {
    samples.sort_by(f64::total_cmp);
    serde_json::json!({"median_us":samples[samples.len()/2],"p95_us":samples[samples.len()*95/100],"samples":samples.len()})
}
fn main() {
    let initial: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut output = Vec::new();
    for floors in [10, 20, 40] {
        for long in [false, true] {
            let mut run = initial.clone();
            while run.floors.len() < floors {
                let mut floor = (*run.floors[0]).clone();
                floor.depth = (run.floors.len() + 1) as f64;
                run.floors.push(std::sync::Arc::new(floor));
            }
            if long {
                run.replay = vec![Command::Wait; 100000];
                let a = run.log_archive.as_mut().unwrap();
                a.events = vec![2.0; 100000];
            }
            let mut times = Vec::new();
            for i in 0..1100 {
                let t = Instant::now();
                black_box(Snapshot::capture(black_box(&run)));
                let us = t.elapsed().as_secs_f64() * 1e6;
                if i >= 100 {
                    times.push(us);
                }
            }
            output.push(serde_json::json!({"operation":"snapshot","floors":floors,"ledger_entries":if long{100000}else{0},"timing":stats(times)}));
        }
    }
    let trace: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1-trace.json")).unwrap();
    let commands: Vec<Command> = trace["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| serde_json::from_value(s["command"].clone()).unwrap())
        .collect();
    for long in [false, true] {
        let mut samples = Vec::new();
        for i in 0..110 {
            let mut run = initial.clone();
            if long {
                run.replay = vec![Command::Wait; 100000];
                run.log_archive.as_mut().unwrap().events = vec![2.0; 100000];
            }
            let mut engine = Engine::new(run).unwrap();
            for c in &commands {
                let t = Instant::now();
                engine.dispatch(c).unwrap();
                if i >= 10 {
                    samples.push(t.elapsed().as_secs_f64() * 1e6);
                }
            }
        }
        output.push(serde_json::json!({"operation":"covered_avatar_command","ledger_entries":if long{100000}else{0},"timing":stats(samples)}));
    }
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
