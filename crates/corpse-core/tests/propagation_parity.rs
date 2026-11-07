use corpse_core::propagation::*;
use serde_json::{Value, json};
struct Fixture {
    input: Value,
    volume: Vec<u16>,
    gas: Vec<Option<String>>,
    rng: u32,
    events: Vec<Value>,
}
impl RandomInt for Fixture {
    fn random_int(&mut self, min: u32, max: u32) -> u32 {
        if min == max {
            min
        } else {
            min + ((corpse_core::rng::next_u32(&mut self.rng) as f64 / 4294967296.)
                * ((max - min + 1) as f64))
                .floor() as u32
        }
    }
}
impl GasGrid for Fixture {
    type Gas = String;
    fn size(&self) -> GridSize {
        GridSize {
            width: self.input["width"].as_u64().unwrap() as usize,
            height: self.input["height"].as_u64().unwrap() as usize,
        }
    }
    fn volume(&self, i: usize) -> u16 {
        self.volume[i]
    }
    fn set_volume(&mut self, i: usize, v: u16) {
        self.volume[i] = v
    }
    fn gas(&self, i: usize) -> Option<String> {
        self.gas[i].clone()
    }
    fn set_gas(&mut self, i: usize, g: Option<String>) {
        self.gas[i] = g
    }
    fn blocks_gas(&self, i: usize) -> bool {
        self.input["blocked"][i].as_bool().unwrap()
    }
    fn frozen(&self, i: usize) -> bool {
        self.input["frozen"][i].as_bool().unwrap()
    }
    fn auto_descent(&self, i: usize) -> bool {
        self.input["descent"][i].as_bool().unwrap()
    }
    fn dissipation(&self, g: Option<String>) -> Dissipation {
        match g.as_deref() {
            None | Some("explosive") => Dissipation::None,
            Some("confusion") => Dissipation::Quick,
            _ => Dissipation::Normal,
        }
    }
    fn on_type_change(
        &mut self,
        i: usize,
        previous: Option<String>,
        next: Option<String>,
        source: usize,
    ) {
        self.events
            .push(json!({"i":i,"previous":previous,"next":next,"source":source}))
    }
    fn on_cell_resolved(&mut self, i: usize, source: usize, contributors: &[usize], output: u16) {
        self.events.push(
            json!({"i":i,"sourceIndex":source,"contributors":contributors,"outputVolume":output}),
        )
    }
}
#[test]
fn all_64_frozen_v48_diffusion_fixtures() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/propagation-v48.json")).unwrap();
    for (i, case) in cases.into_iter().enumerate() {
        let input = &case["initial"];
        let mut f = Fixture {
            input: input.clone(),
            volume: serde_json::from_value(input["volume"].clone()).unwrap(),
            gas: serde_json::from_value(input["gas"].clone()).unwrap(),
            rng: input["rng"].as_u64().unwrap() as u32,
            events: vec![],
        };
        diffuse_gas_pass(&mut f);
        assert_eq!(
            json!({"volume":f.volume,"gas":f.gas,"rng":f.rng,"events":f.events}),
            case["expected"],
            "seed case {}",
            i + 1
        );
    }
}
