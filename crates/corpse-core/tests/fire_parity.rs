use corpse_core::propagation::*;
use serde_json::{Value, json};
struct Fixture {
    input: Value,
    layers: Vec<Vec<FireLayer>>,
    exposure: Vec<u32>,
    rng: u32,
    events: Vec<Value>,
}
impl RandomInt for Fixture {
    fn random_int(&mut self, min: u32, max: u32) -> u32 {
        min + ((corpse_core::rng::next_u32(&mut self.rng) as f64 / 4294967296.)
            * ((max - min + 1) as f64))
            .floor() as u32
    }
}
impl FireExposure for Fixture {
    fn size(&self) -> GridSize {
        GridSize {
            width: 5,
            height: 4,
        }
    }
    fn layers(&self, i: usize) -> Vec<FireLayer> {
        self.layers[i].clone()
    }
    fn exposure(&self, i: usize) -> u32 {
        self.exposure[i]
    }
    fn increment_exposure(&mut self, i: usize) {
        self.exposure[i] += 1
    }
    fn qualifies_for_confinement(&self, i: usize) -> bool {
        self.input["qualifies"][i].as_bool().unwrap()
    }
    fn promote(&mut self, i: usize, layer: FireLayer, explosive: bool) {
        self.events
            .push(json!({"i":i,"id":layer.id,"explosive":explosive}));
        self.layers[i][layer.id.parse::<usize>().unwrap()].flammable = false;
        if layer.id == "0" {
            self.layers[i][2].flammable = false
        }
    }
}
#[test]
fn fire_exposure_matches_64_v48_cases() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/fire-v48.json")).unwrap();
    for case in cases {
        let input = &case["initial"];
        let layers = input["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|l| FireLayer {
                        id: l["id"].as_str().unwrap().into(),
                        flammable: l["flammable"].as_bool().unwrap(),
                        ignition_chance: l["ignitionChance"].as_f64().unwrap(),
                        draw_priority: l["drawPriority"].as_f64().unwrap(),
                        extinguishes: l["extinguishes"].as_bool().unwrap(),
                        is_gas: l["isGas"].as_bool().unwrap(),
                        explosive: l["explosive"].as_bool().unwrap(),
                    })
                    .collect()
            })
            .collect();
        let mut f = Fixture {
            input: input.clone(),
            layers,
            exposure: serde_json::from_value(input["exposure"].clone()).unwrap(),
            rng: input["rng"].as_u64().unwrap() as u32,
            events: vec![],
        };
        let mut results = vec![];
        for x in 0..5 {
            for y in 0..4 {
                let i = y * 5 + x;
                results.push(expose_fire(&mut f, i, i % 7 == 0));
            }
        }
        let actual_layers:Vec<Vec<Value>>=f.layers.iter().map(|ls|ls.iter().map(|l|json!({"id":l.id,"flammable":l.flammable,"ignitionChance":l.ignition_chance as u32,"drawPriority":l.draw_priority as u32,"extinguishes":l.extinguishes,"isGas":l.is_gas,"explosive":l.explosive})).collect()).collect();
        assert_eq!(
            json!({"layers":actual_layers,"exposure":f.exposure,"rng":f.rng,"events":f.events,"results":results}),
            case["expected"]
        );
    }
}
