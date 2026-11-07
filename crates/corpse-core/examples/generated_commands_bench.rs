use corpse_core::{Command, Run, temporal::Engine};
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
fn stats(mut a: Vec<f64>) -> Value {
    a.sort_by(f64::total_cmp);
    json!({"samples":a.len(),"median_us":a[a.len()/2],"p95_us":a[a.len()*95/100]})
}
fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/generated-walks-v48.json");
    let cases: Value = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 6);
    assert_eq!(
        cases
            .iter()
            .map(|c| c["steps"].as_array().unwrap().len())
            .sum::<usize>(),
        472
    );
    let mut results = vec![];
    for c in cases {
        let initial: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let commands: Vec<Command> = c["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| serde_json::from_value(s["command"].clone()).unwrap())
            .collect();
        let mut expected = c["initial"].clone();
        for step in c["steps"].as_array().unwrap() {
            for p in step["patch"].as_array().unwrap() {
                apply_patch(&mut expected, p);
            }
        }
        let mut check = Engine::new(initial.clone()).unwrap();
        for command in &commands {
            check.dispatch(command).unwrap();
        }
        let mut actual = serde_json::to_value(check.canonical_run().unwrap()).unwrap();
        for k in ["history", "hasteSnapshot"] {
            actual.as_object_mut().unwrap().remove(k);
            expected.as_object_mut().unwrap().remove(k);
        }
        assert!(
            corpse_core::storage::canonical_equal(&actual, &expected),
            "generated command parity {}",
            c["name"]
        );
        let mut samples = vec![];
        for pass in 0..8 {
            let mut e = Engine::new(initial.clone()).unwrap();
            for command in &commands {
                let t = Instant::now();
                black_box(e.dispatch(command).unwrap());
                if pass >= 2 {
                    samples.push(t.elapsed().as_secs_f64() * 1e6);
                }
            }
        }
        results.push(json!({"name":c["name"],"seed":initial.seed,"floors":initial.floors.len(),"commands":commands.len(),"timing":stats(samples)}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"scope":"Six genuine generated Avatar runs; same472commands; two warmup traces, six measured repetitions per seed. Setup excluded; temporal Engine dispatch included. Rendering excluded.","results":results})).unwrap());
}

fn apply_patch(root: &mut serde_json::Value, p: &serde_json::Value) {
    let path = p["path"].as_array().unwrap();
    let mut node = root;
    for key in &path[..path.len() - 1] {
        let k = key.as_str().unwrap();
        node = if node.is_array() {
            &mut node[k.parse::<usize>().unwrap()]
        } else {
            &mut node[k]
        };
    }
    let key = path.last().unwrap().as_str().unwrap();
    if p["remove"] == true {
        node.as_object_mut().unwrap().remove(key);
    } else if node.is_array() {
        node[key.parse::<usize>().unwrap()] = p["value"].clone();
    } else {
        node[key] = p["value"].clone();
    }
}
