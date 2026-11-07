use corpse_core::{Actor, Field, Run, migrations, storage};
#[test]
fn bastidion_source_vectors() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/identity-migrations.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut a: Actor = serde_json::from_value(c["before"].clone()).unwrap();
        migrations::migrate_bastidion(&mut a);
        assert!(storage::canonical_equal(
            &serde_json::to_value(&a).unwrap(),
            &c["after"]
        ));
        assert!(!migrations::migrate_bastidion(&mut a));
    }
}
#[test]
fn generated_grant_order_is_stable() {
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.generated_ability_grants = Field::Missing;
    r.ground.clear();
    r.inventory[0].id = 10.0;
    r.inventory[0].granted_ability = Field::Value("anavani-illuminate".into());
    r.inventory[1].id = 2.0;
    r.inventory[1].granted_ability = Field::Value("anavani-sparkle".into());
    r.inventory[2].id = 10.0;
    r.inventory[2].granted_ability = Field::Value("anavani-illuminate".into());
    assert!(migrations::migrate_generated_ability_grants(&mut r));
    assert_eq!(
        r.generated_ability_grants.as_ref().unwrap(),
        &vec![
            "anavani-sparkle".to_owned(),
            "anavani-illuminate".to_owned()
        ]
    );
    assert!(!migrations::migrate_generated_ability_grants(&mut r));
}
#[test]
fn display_order_normalization_matches_source() {
    let initial: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/display-order-v48.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let mut input = initial.clone();
        input["inventoryDisplayOrder"] = c["order"].clone();
        let loaded = storage::deserialize(&input.to_string()).unwrap();
        assert!(storage::canonical_equal(
            &serde_json::to_value(&loaded.inventory_display_order).unwrap(),
            &c["expected"]
        ));
    }
}
#[test]
fn divine_source_vectors() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/divine-migrations.json")).unwrap();
    for (i, c) in cases.as_array().unwrap().iter().enumerate() {
        let mut run: Run = serde_json::from_value(expand_health_case(&c["before"])).unwrap();
        assert!(migrations::migrate_divine_revision(&mut run));
        assert!(
            storage::canonical_equal(
                &storage::canonical_value(&run).unwrap(),
                &expand_health_case(&c["after"])
            ),
            "divine case {i}"
        );
        assert!(!migrations::migrate_divine_revision(&mut run));
    }
}
#[test]
fn integer_health_source_vectors() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/integer-health-migrations.json"
    ))
    .unwrap();
    for (i, c) in cases.as_array().unwrap().iter().enumerate() {
        let mut run: Run = serde_json::from_value(expand_health_case(&c["before"])).unwrap();
        assert_eq!(
            migrations::normalize_integer_stat_health(&mut run),
            c["changed"].as_bool().unwrap()
        );
        assert!(
            storage::canonical_equal(
                &storage::canonical_value(&run).unwrap(),
                &expand_health_case(&c["after"])
            ),
            "integer-health case {i}"
        );
        assert!(!migrations::normalize_integer_stat_health(&mut run));
    }
}

fn expand_health_case(patch: &serde_json::Value) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .shift_remove("divineRevision");
    for (k, v) in patch.as_object().unwrap() {
        value[k] = v.clone();
    }
    value
}
