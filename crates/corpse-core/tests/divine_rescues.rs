use corpse_core::{Actor, Run, divine_rescues as d};
#[test]
fn source_ordered_rescues_and_damage_bank() {
    let fixture: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/divine-rescues-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let base: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 144);
    for (i, c) in cases.iter().enumerate() {
        let mut r = base.clone();
        r.player = serde_json::from_value(c["initial"].clone()).unwrap();
        let id = r.player.id;
        let rescued = d::try_rescue(&mut r, id, c["bypass"].as_bool().unwrap());
        assert_eq!(rescued, c["rescued"].as_bool().unwrap(), "decision{i}");
        let expected: Actor = serde_json::from_value(c["expected"].clone()).unwrap();
        let mut a = serde_json::to_value(&r.player).unwrap();
        let mut b = serde_json::to_value(expected).unwrap();
        if !rescued {
            a.as_object_mut().unwrap().remove("hp");
            b.as_object_mut().unwrap().remove("hp");
            a["conditions"]
                .as_object_mut()
                .unwrap()
                .remove("beatToDeathHits");
            b["conditions"]
                .as_object_mut()
                .unwrap()
                .remove("beatToDeathHits");
        }
        assert_eq!(a, b, "owner state{i}");
    }
    let banks = fixture["banks"].as_array().unwrap();
    assert_eq!(banks.len(), 64);
    for (i, c) in banks.iter().enumerate() {
        let mut r = base.clone();
        r.player.abilities = vec![d::OVERFLOWING.into()];
        r.player.blood = c["blood"].as_f64().unwrap().into();
        r.player.damage_bank = c["bank"].as_f64().unwrap().into();
        let id = r.player.id;
        d::accrue_damage(&mut r, id, c["amount"].as_f64().unwrap());
        assert_eq!(
            r.player.blood.value_or(0.0),
            c["expectedBlood"].as_f64().unwrap(),
            "blood{i}"
        );
        assert_eq!(
            r.player.damage_bank.value_or(0.0),
            c["expectedBank"].as_f64().unwrap(),
            "bank{i}"
        );
    }
}
