use corpse_core::{Actor, Run, Stats, divine_combat as d};
#[test]
fn source_stat_pool_and_prime_vectors() {
    let cases: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/divine-combat-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 192);
    let base: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let mut r = base.clone();
        r.player = serde_json::from_value(c["player"].clone()).unwrap();
        r.inventory = serde_json::from_value(c["inventory"].clone()).unwrap();
        let a: Actor = serde_json::from_value(c["actor"].clone()).unwrap();
        if c["kind"] != "player" {
            *r.actors_at_mut(1.0) = vec![a.clone()];
        }
        let mut s: Stats = serde_json::from_value(c["inputStats"].clone()).unwrap();
        d::apply_stat_modifiers(&r, &a, &mut s);
        let expected: Stats = serde_json::from_value(c["expectedStats"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::to_value(expected).unwrap(),
            "stat vector{i}"
        );
        assert_eq!(
            d::health_from_stats(&r, &a, &s),
            c["health"].as_f64().unwrap(),
            "health vector{i}"
        );
        assert_eq!(
            d::blood_weapon_hp_cost(&a),
            c["cost"].as_f64().unwrap(),
            "cost vector{i}"
        );
        assert_eq!(
            d::blood_weapon_ready(&r, &a),
            c["ready"].as_bool().unwrap(),
            "ready vector{i}"
        );
        assert_eq!(
            d::healing_suppressed(&r, &a),
            c["suppressed"].as_bool().unwrap(),
            "suppression vector{i}"
        );
    }
}
#[test]
fn narrow_hit_and_healing_boundaries() {
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.player.abilities.extend([
        d::FIGHT_LONG.into(),
        d::BEAT_TO_DEATH.into(),
        d::BLOOD_WEAPON.into(),
    ]);
    r.player.hp = 90.0;
    let a = r.player.clone();
    let id = r.actors_at(1.0)[0].id;
    d::beat_after_landed_hit(&mut r, &a, id);
    assert_eq!(r.actors_at(1.0)[0].conditions["beatToDeathHits"], 1.0);
    r.actors_at_mut(1.0)[0].hp = 0.0;
    d::beat_after_landed_hit(&mut r, &a, id);
    assert_eq!(r.actors_at(1.0)[0].conditions["beatToDeathHits"], 1.0);
    d::heal_melee_damage(&mut r, &a, 3.0, false);
    assert_eq!(r.player.hp, 90.0);
    d::heal_melee_damage(&mut r, &a, 3.0, true);
    assert_eq!(r.player.hp, 90.6);
    d::heal_melee_damage(&mut r, &a, 1000.0, true);
    assert_eq!(r.player.hp, 100.0);
    r.player.hp = 0.0;
    d::heal_melee_damage(&mut r, &a, 10.0, true);
    assert_eq!(r.player.hp, 0.0);
    d::unlock_blood_and_fury(&mut r);
    let logs = r.log.len();
    d::unlock_blood_and_fury(&mut r);
    assert_eq!(r.log.len(), logs);
    assert_eq!(
        r.achievements
            .as_ref()
            .unwrap()
            .iter()
            .filter(|s| s.as_str() == "blood-and-fury")
            .count(),
        1
    );
}
