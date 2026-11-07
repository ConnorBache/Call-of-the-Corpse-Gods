use corpse_core::{Floor, generation::*};
use indexmap::IndexMap;
use serde_json::Value;
#[test]
fn stage_evidence_and_generation_validator_match_v48() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/generation-validation-v48.json")).unwrap();
    for c in fixture["cases"].as_array().unwrap() {
        let mut f: Floor = serde_json::from_value(c["before"].clone()).unwrap();
        for id in GENERATION_STAGES {
            record_generation_stage(
                &mut f,
                id,
                if id == "cleanup" {
                    Some(IndexMap::from([
                        ("changed".into(), 4.0),
                        ("ratio".into(), 0.125),
                    ]))
                } else {
                    None
                },
            )
            .unwrap()
        }
        assert_eq!(
            record_generation_stage(&mut f, "hordes", None).unwrap_err(),
            c["orderError"].as_str().unwrap()
        );
        let expected: Floor = serde_json::from_value(c["after"].clone()).unwrap();
        assert_eq!(f, expected);
        assert_eq!(validate_floor_generation(&f), c["valid"].as_bool().unwrap());
        for m in c["checks"].as_array().unwrap() {
            let mut v = c["after"].clone();
            let parts: Vec<&str> = m["path"].as_str().unwrap().split('.').collect();
            let mut p = &mut v;
            for part in &parts[..parts.len() - 1] {
                p = if let Ok(i) = part.parse::<usize>() {
                    &mut p[i]
                } else {
                    &mut p[*part]
                };
            }
            let last = parts[parts.len() - 1];
            if let Ok(i) = last.parse::<usize>() {
                p[i] = m["value"].clone()
            } else {
                p[last] = m["value"].clone()
            }
            let actual = serde_json::from_value::<Floor>(v)
                .map(|f| validate_floor_generation(&f))
                .unwrap_or(false);
            assert_eq!(
                actual,
                m["expected"].as_bool().unwrap(),
                "{} {}",
                m["path"],
                m["value"]
            );
        }
    }
}
