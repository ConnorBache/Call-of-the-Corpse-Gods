//! R52 Never Surrender waits for damage whose mitigation includes DR.
use corpse_core::{Actor, Field, Run, Stats, divine_reactions as d, engine, revision50_ability_energy};

fn room(revision: f64, piercing: bool) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.inventory.clear(); r.actors.clear();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.energy = 100.; r.player.stats.defense = 12.; r.player.stats.dr = 0.; r.player.stats.dodge = 15.;
    r.player.abilities = vec![d::SURRENDER.into()]; r.player.active = r.player.abilities.clone();
    r.player.conditions.clear(); r.player.ability_reserves = Field::Missing;
    revision50_ability_energy::initialize_reserve(&mut r.player, d::SURRENDER, 10., 10.).unwrap();
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.environment = Field::Missing; f.doors = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing;
    let mut a = Actor { id: 900., x: 11., y: 10., name: "Surrender audit".into(), state: "hunting".into(),
        hp: 100., max_hp: 100., stats: Stats { accuracy: 100., ..Default::default() }, ..Default::default() };
    a.conditions.insert("weaponDamage".into(), 40.);
    if piercing { a.abilities.push("enemy:beast:armor-piercing".into()); }
    r.actors_at_mut(r.depth).push(a);
    r
}
fn reserve(r: &Run) -> f64 { r.player.ability_reserves.as_ref().unwrap()[d::SURRENDER].energy }
fn hit(r: &mut Run, ranged: bool, arrow: bool) {
    let a = engine::actor_mut(r, 900.).unwrap();
    a.conditions.insert("lucky".into(), 1.); a.x = if ranged { 13. } else { 11. };
    if ranged { assert!(engine::attack_ranged_resolved(r, 900., 0., 40., 3., 1., arrow).unwrap()); }
    else { engine::attack(r, 900., 0., 1.).unwrap(); }
}
#[test]
fn ranged_arrows_and_other_projectiles_gain_dr_but_piercing_neither_benefits_nor_spends() {
    for arrow in [false, true] { for piercing in [false, true] {
        let mut r = room(52., piercing); let mut control = r.clone(); control.player.active.clear();
        hit(&mut control, true, arrow); hit(&mut r, true, arrow);
        assert_eq!(r.player.hp - control.player.hp, if piercing { 0. } else { 12. });
        assert_eq!(reserve(&r), if piercing { 10. } else { 2. });
        assert_eq!(r.player.energy, 100.);
        assert_eq!(r.player.conditions.contains_key("surrenderTick"), !piercing);
        assert_eq!(r.rng, control.rng);
    }}
}
#[test]
fn piercing_then_normal_same_turn_preserves_then_uses_exactly_one_proc() {
    for ranged in [false, true] {
        let mut r = room(52., true); hit(&mut r, ranged, false);
        assert_eq!(reserve(&r), 10.); assert!(!r.player.conditions.contains_key("surrenderTick"));
        engine::actor_mut(&mut r, 900.).unwrap().abilities.clear();
        let mut control = r.clone(); control.player.active.clear();
        hit(&mut control, ranged, false); hit(&mut r, ranged, false);
        assert_eq!(r.player.hp - control.player.hp, 12.); assert_eq!(reserve(&r), 2.);
        assert_eq!(r.player.conditions["surrenderTick"], r.tick);
        let mut control = r.clone(); control.player.active.clear();
        hit(&mut control, ranged, false); hit(&mut r, ranged, false);
        assert_eq!(r.player.hp, control.player.hp); assert_eq!(reserve(&r), 2.);
    }
}
#[test]
fn direct_spell_status_and_environment_ignore_dr_and_leave_normal_hit_available() {
    for kind in ["acid", "lightning", "fire", "poison", "bleeding", ""] {
        let mut r = room(52., false);
        if matches!(kind, "acid" | "lightning") { engine::damage_direct(&mut r, 0., 8., 900., kind, true).unwrap(); }
        else { engine::damage_environment(&mut r, 0., 8., None, kind, true).unwrap(); }
        assert_eq!(r.player.hp, 92., "{kind}"); assert_eq!(reserve(&r), 10.);
        assert!(!r.player.conditions.contains_key("surrenderTick"), "{kind}");
        let mut control = r.clone(); control.player.active.clear();
        hit(&mut control, false, false); hit(&mut r, false, false);
        assert_eq!(r.player.hp - control.player.hp, 12., "{kind}"); assert_eq!(reserve(&r), 2.);
    }
}
#[test]
fn missed_or_naturally_zero_hits_are_free_but_full_surrender_block_pays() {
    let mut r = room(52., false);
    engine::actor_mut(&mut r, 900.).unwrap().conditions.insert("distracted".into(), 1.);
    engine::attack(&mut r, 900., 0., 1.).unwrap();
    assert_eq!(r.player.hp, 100.); assert_eq!(reserve(&r), 10.);
    r.player.stats.dr = 100.; hit(&mut r, false, false);
    assert_eq!(reserve(&r), 10.); assert!(!r.player.conditions.contains_key("surrenderTick"));
    r.player.stats.dr = 0.; r.player.stats.defense = 100.; hit(&mut r, false, false);
    assert_eq!(r.player.hp, 100.); assert_eq!(reserve(&r), 2.);
    assert_eq!(r.player.conditions["surrenderTick"], r.tick);
}
#[test]
fn bypass_cannot_deactivate_an_unfunded_toggle_and_legacy_behavior_is_frozen() {
    let mut r = room(52., true); r.player.energy = 0.;
    r.player.ability_reserves.as_mut().unwrap().get_mut(d::SURRENDER).unwrap().energy = 7.;
    hit(&mut r, false, false);
    assert_eq!(reserve(&r), 7.); assert!(r.player.active.contains(&d::SURRENDER.into()));
    assert!(!r.player.conditions.contains_key("surrenderTick"));
    engine::actor_mut(&mut r, 900.).unwrap().abilities.clear(); hit(&mut r, false, false);
    assert_eq!(reserve(&r), 7.); assert!(!r.player.active.contains(&d::SURRENDER.into()));
    for revision in [49., 50., 51.] {
        let mut r = room(revision, true); hit(&mut r, false, false);
        assert_eq!(r.player.conditions["surrenderTick"], r.tick);
        assert_eq!(r.player.energy, if revision < 50. { 92. } else { 100. });
        assert_eq!(reserve(&r), if revision < 50. { 10. } else { 2. });
    }
}
