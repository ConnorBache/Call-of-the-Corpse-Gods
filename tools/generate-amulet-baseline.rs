use corpse_core::{replay::ReplayRecord, temporal::Engine};
fn digest(v: &serde_json::Value)->String {format!("{:016x}",serde_json::to_vec(v).unwrap().iter().fold(0xcbf29ce484222325u64,|h,b|(h ^ *b as u64).wrapping_mul(0x100000001b3)))}
fn main() {
    let run=corpse_core::startup::create_avatar_run("amulet-compat-prechange-f8b1952").unwrap();
    let mut engine=Engine::new(run).unwrap();
    let initial=corpse_core::storage::canonical_value(&engine.run).unwrap();
    let command=serde_json::from_str(r#"{"type":"wait"}"#).unwrap();
    for _ in 0..3 {engine.dispatch(&command).unwrap();}
    let record=ReplayRecord::from_run(&engine.run);
    let final_state=corpse_core::storage::canonical_value(&engine.run).unwrap();
    println!("{}",serde_json::json!({"sourceCommit":"f8b1952ea66a158800943eb6bdf6e8bdcf9f41f8","initialFnv64":digest(&initial),"record":record,"finalFnv64":digest(&final_state)}));
}
