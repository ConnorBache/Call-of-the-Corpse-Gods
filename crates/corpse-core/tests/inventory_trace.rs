use corpse_core::{Command, Run};
#[test]
fn inventory_reference_trace() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/inventory-trace-v48.json")).unwrap();
    let mut r: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    for (i, s) in f["steps"].as_array().unwrap().iter().enumerate() {
        let c: Command = serde_json::from_value(s["command"].clone()).unwrap();
        corpse_core::engine::dispatch(&mut r, &c).unwrap();
        let e: Run = serde_json::from_value(s["state"].clone()).unwrap();
        let mut a = serde_json::to_value(&r).unwrap();
        let mut b = serde_json::to_value(e).unwrap();
        for key in ["history", "hasteSnapshot"] {
            a.as_object_mut().unwrap().remove(key);
            b.as_object_mut().unwrap().remove(key);
        }
        if a != b {
            let keys: Vec<_> = a
                .as_object()
                .unwrap()
                .keys()
                .chain(b.as_object().unwrap().keys())
                .filter(|k| a.get(*k) != b.get(*k))
                .collect();
            panic!(
                "step {i} {c:?}, differing keys {keys:?}\n{}",
                keys.iter()
                    .map(|k| format!("{k}: actual {} expected {}", a[*k], b[*k]))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

#[test]
fn consumable_reference_trace() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/consumable-trace-v48.json")).unwrap();
    let mut r: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    for (i, s) in f["steps"].as_array().unwrap().iter().enumerate() {
        let c: Command = serde_json::from_value(s["command"].clone()).unwrap();
        corpse_core::engine::dispatch(&mut r, &c).unwrap();
        let e: Run = serde_json::from_value(s["state"].clone()).unwrap();
        let mut a = serde_json::to_value(&r).unwrap();
        let mut b = serde_json::to_value(e).unwrap();
        for key in ["history", "hasteSnapshot"] {
            a.as_object_mut().unwrap().remove(key);
            b.as_object_mut().unwrap().remove(key);
        }
        if a != b {
            let keys: Vec<_> = a
                .as_object()
                .unwrap()
                .keys()
                .chain(b.as_object().unwrap().keys())
                .filter(|k| a.get(*k) != b.get(*k))
                .collect();
            panic!(
                "step {i} {c:?}, differing keys {keys:?}\n{}",
                keys.iter()
                    .map(|k| format!("{k}: actual {} expected {}", a[*k], b[*k]))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

#[test]
fn progression_reference_trace() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/progression-trace-v48.json")).unwrap();
    let mut r: Run = serde_json::from_value(f["initial"].clone()).unwrap();
    for (i, s) in f["steps"].as_array().unwrap().iter().enumerate() {
        let c: Command = serde_json::from_value(s["command"].clone()).unwrap();
        corpse_core::engine::dispatch(&mut r, &c).unwrap();
        let e: Run = serde_json::from_value(s["state"].clone()).unwrap();
        let mut a = serde_json::to_value(&r).unwrap();
        let mut b = serde_json::to_value(e).unwrap();
        for key in ["history", "hasteSnapshot"] {
            a.as_object_mut().unwrap().remove(key);
            b.as_object_mut().unwrap().remove(key);
        }
        if a != b {
            let keys: Vec<_> = a
                .as_object()
                .unwrap()
                .keys()
                .chain(b.as_object().unwrap().keys())
                .filter(|k| a.get(*k) != b.get(*k))
                .collect();
            panic!(
                "step {i} {c:?}, differing keys {keys:?}\n{}",
                keys.iter()
                    .map(|k| format!("{k}: actual {} expected {}", a[*k], b[*k]))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}
#[test]
fn unsupported_inventory_effect_is_transactional() {
    use corpse_core::{Field, Item};
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.inventory.push(Item {
        id: 500.0,
        name: "Unidentified potion".into(),
        kind: "potion".into(),
        glyph: "!".into(),
        effect: Field::Value("not-an-authored-potion".into()),
        ..Default::default()
    });
    let before = r.clone();
    assert!(corpse_core::engine::dispatch(&mut r, &Command::Use { id: 500.0 }).is_err());
    assert_eq!(r, before);
    let before = r.clone();
    assert!(
        corpse_core::engine::dispatch(
            &mut r,
            &Command::Ability {
                id: "not-an-authored-effect".into(),
                target: Field::Missing,
                x: Field::Missing,
                y: Field::Missing
            }
        )
        .is_err()
    );
    assert_eq!(r, before);
}
