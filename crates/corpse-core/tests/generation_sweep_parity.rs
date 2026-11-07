use corpse_core::generation::generate_floor;
use serde_json::Value;
fn hash(v: &Value) -> String {
    fn byte(h: &mut u64, b: u8) {
        *h = (*h ^ b as u64).wrapping_mul(1099511628211)
    }
    fn u32v(h: &mut u64, n: usize) {
        for b in (n as u32).to_le_bytes() {
            byte(h, b)
        }
    }
    fn visit(h: &mut u64, v: &Value) {
        match v {
            Value::Null => byte(h, 0),
            Value::Bool(b) => {
                byte(h, 1);
                byte(h, *b as u8)
            }
            Value::Number(n) => {
                byte(h, 2);
                for b in n.as_f64().unwrap().to_le_bytes() {
                    byte(h, b)
                }
            }
            Value::String(s) => {
                byte(h, 3);
                u32v(h, s.len());
                for b in s.bytes() {
                    byte(h, b)
                }
            }
            Value::Array(a) => {
                byte(h, 4);
                u32v(h, a.len());
                for v in a {
                    visit(h, v)
                }
            }
            Value::Object(o) => {
                byte(h, 5);
                u32v(h, o.len());
                let mut keys: Vec<_> = o.keys().collect();
                keys.sort();
                for k in keys {
                    visit(h, &Value::String(k.clone()));
                    visit(h, &o[k]);
                }
            }
        }
    }
    let mut h = 14695981039346656037;
    visit(&mut h, v);
    format!("{h:016x}")
}
#[test]
fn complete_generation_depth_and_seed_sweep_matches_frozen_hashes() {
    let fs: Value =
        serde_json::from_str(include_str!("fixtures/generation-sweep-v48.json")).unwrap();
    for (c, q) in fs["cases"].as_array().unwrap().iter().enumerate() {
        let result = generate_floor(
            q["depth"].as_u64().unwrap() as u32,
            q["seed"].as_u64().unwrap() as u32,
            q["domain"].as_str().unwrap(),
            q["godbeast"].as_bool().unwrap(),
            q["size"].as_u64().unwrap() as usize,
            None,
        );
        if let Some(e) = q["error"].as_str() {
            assert_eq!(result.unwrap_err(), e);
            continue;
        }
        let floor = result.unwrap_or_else(|e| panic!("case{c}: {e}"));
        let actual = hash(&serde_json::to_value(&floor).unwrap());
        assert_eq!(
            actual,
            q["hash"].as_str().unwrap(),
            "case{c} seed{} depth{} domain{}",
            q["seed"],
            q["depth"],
            q["domain"]
        );
    }
}
