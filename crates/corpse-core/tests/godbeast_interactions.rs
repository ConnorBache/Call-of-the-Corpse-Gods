use corpse_core::{Floor, Item, Run, godbeast_interactions as g};
#[test]
fn exact_source_event_interactions() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/godbeast-interactions-v48.json"
    ))
    .unwrap();
    let taps = data["taps"].as_array().unwrap();
    let floods = data["floods"].as_array().unwrap();
    assert_eq!(taps.len(), 24);
    assert_eq!(floods.len(), 24);
    for (n, c) in taps.iter().enumerate() {
        let mut r: Run = serde_json::from_value(c["input"].clone()).unwrap();
        let got = g::tap_veridian(&mut r);
        let expected: Run = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(r, expected, "tap{n}");
        assert_eq!(got.success, c["result"]["success"].as_bool().unwrap());
        assert_eq!(got.message, c["result"]["message"].as_str().unwrap());
        let sap: Option<Item> = c["result"]
            .get("sap")
            .map(|i| serde_json::from_value(i.clone()).unwrap());
        assert_eq!(got.sap, sap);
    }
    for (n, c) in floods.iter().enumerate() {
        let mut f: Floor = serde_json::from_value(c["input"].clone()).unwrap();
        assert_eq!(g::activate_mercuria(&mut f), c["ok"].as_bool().unwrap());
        let expected: Floor = serde_json::from_value(c["expected"].clone()).unwrap();
        assert_eq!(f, expected, "flood{n}");
    }
}
