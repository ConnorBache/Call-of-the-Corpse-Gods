use corpse_core::{Run, Stats, divine_reactions as d, engine};
fn base() -> Run {
    serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
}
#[test]
fn source_nightmare_and_surrender_predicates() {
    let f: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/divine-reactions-v48.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 96);
    assert_eq!(f["surrender"].as_array().unwrap().len(), 72);
    for (i, c) in f["cases"].as_array().unwrap().iter().enumerate() {
        let mut r = base();
        r.player = serde_json::from_value(c["player"].clone()).unwrap();
        r.actors = serde_json::from_value(c["actors"].clone()).unwrap();
        let a = engine::actor(&r, c["victim"].as_f64().unwrap()).unwrap();
        let source = c["source"].as_f64().and_then(|id| engine::actor(&r, id));
        assert_eq!(
            d::nightmare_absorbs(&r, a, source),
            c["expected"].as_bool().unwrap(),
            "nightmare{i}"
        );
    }
    for (i, c) in f["surrender"].as_array().unwrap().iter().enumerate() {
        let mut r = base();
        r.player = serde_json::from_value(c["player"].clone()).unwrap();
        r.actors = serde_json::from_value(c["actors"].clone()).unwrap();
        r.tick = c["tick"].as_f64().unwrap();
        let s: Stats = serde_json::from_value(c["stats"].clone()).unwrap();
        let bonus = d::surrender_bonus(&r, &r.player, s.defense, true);
        assert_eq!(
            corpse_core::combat::tuned_damage(100.0 - s.dr - bonus, Some("attack")),
            c["withBonus"].as_f64().unwrap(),
            "surrender{i}"
        );
        assert_eq!(
            corpse_core::combat::tuned_damage(100.0 - s.dr, Some("attack")),
            c["withoutBonus"].as_f64().unwrap()
        );
    }
}
#[test]
fn reaction_and_packet_boundaries() {
    let mut r = base();
    r.player.active = vec![d::QUICK_HAND.into()];
    r.player.reactions = 1.0.into();
    let b = r.player.clone();
    d::quick_hand_after_hit(&mut r, &b);
    assert_eq!(r.player.reactions.value_or(-1.0), 0.0);
    assert_eq!(r.player.conditions["hastedFresh"], 1.0);
    assert_eq!(r.player.conditions["hasted"], 1.0);
    d::quick_hand_after_hit(&mut r, &b);
    assert_eq!(r.player.reactions.value_or(-1.0), 0.0);
    r.player.active = vec![d::SURRENDER.into()];
    r.player.energy = 8.0;
    r.player.conditions.clear();
    let b = r.player.clone();
    assert_eq!(d::surrender_bonus(&r, &b, 5.0, true), 5.0);
    d::pay_surrender(&mut r, &b, 1.0);
    assert_eq!(r.player.energy, 0.0);
    assert_eq!(r.player.conditions["surrenderTick"], 0.0);
    r.player.energy = 7.0;
    r.player.conditions.clear();
    let b = r.player.clone();
    d::pay_surrender(&mut r, &b, 1.0);
    assert!(r.player.active.is_empty());
    assert_eq!(r.player.energy, 7.0);
    r.player.abilities.push(d::HIT_BACK.into());
    r.actors_at_mut(1.0)[0].x = r.player.x + 1.0;
    let a = r.actors_at(1.0)[0].clone();
    let b = r.player.clone();
    assert_eq!(
        d::prepare_hit_back(&mut r, &a, &b, 10.0, 2.0, 1.0, false),
        Some(7.0)
    );
    assert_eq!(
        d::prepare_hit_back(&mut r, &a, &b, 10.0, 2.0, 1.0, false),
        None
    );
    r.player.active = vec![d::DEFLECT.into()];
    r.player.reactions = 1.0.into();
    let b = r.player.clone();
    let old_rng = r.rng;
    assert_eq!(d::select_deflection_target(&mut r, &b, false), Some(a.id));
    assert_ne!(r.rng, old_rng);
    assert_eq!(r.player.reactions.value_or(-1.0), 0.0);
    r.player.abilities.push(d::BLEED.into());
    let p = r.player.clone();
    d::apply_bleeding(&mut r, &p, a.id, 1.0, 0.0);
    assert_eq!(r.actors_at(1.0)[0].conditions["bleeding"], 0.0);
    assert_eq!(r.actors_at(1.0)[0].conditions["bleedSourceId"], p.id);
    r.player.active = vec![d::NIGHTMARE.into()];
    r.player.nightmares = 2.0.into();
    let p = r.player.clone();
    assert!(d::absorb_nightmare(&mut r, &p, Some(&a)));
    assert_eq!(r.player.nightmares.value_or(-1.0), 1.0);
    assert_eq!(r.player.hp, p.hp);
}
