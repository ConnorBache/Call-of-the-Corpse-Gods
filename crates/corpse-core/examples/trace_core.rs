use corpse_core::{Command, Run};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut r: Run = serde_json::from_value(v["initial"].clone()).unwrap();
    for (i, s) in v["steps"].as_array().unwrap().iter().enumerate() {
        let c: Command = serde_json::from_value(s["command"].clone()).unwrap();
        match corpse_core::engine::dispatch(&mut r, &c) {
            Ok(_) => {
                std::fs::write(
                    format!("/tmp/native-step-{i}.json"),
                    serde_json::to_string(&r).unwrap(),
                )
                .unwrap();
                println!("step {i} tick {} rng {}", r.tick, r.rng)
            }
            Err(e) => {
                eprintln!("step {i}: {e}");
                break;
            }
        }
    }
}
