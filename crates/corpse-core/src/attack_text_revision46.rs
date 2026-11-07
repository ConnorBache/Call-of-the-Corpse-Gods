//! Deterministic attacks compare all gameplay state and presentation events with
//! revision45; only the Chronicle text and its existing archive may differ.
use crate::{Field, Run, engine, equipment, events};

fn room(kind: &str) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 46.0.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.hp = 100_000.; r.player.max_hp = 100_000.;
    r.inventory.clear(); r.offers.clear(); r.log.clear();
    r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    for actors in r.actors.values_mut() { *actors = std::sync::Arc::new(Vec::new()); }
    let mut a = crate::population::create_actor_revision(kind, 10., 11., 10., false, 46.).unwrap();
    a.hp = 100_000.; a.max_hp = 100_000.; a.state = "hunting".into();
    r.actors_at_mut(1.).push(a);
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    if kind == "river-drake" { let at = 10 * f.width() + 11; f.tiles[at] = "water".into(); }
    f.environment = Field::Missing; f.doors = Field::Missing;
    r
}

fn same_gameplay(mut current: Run, operation: impl Fn(&mut Run)) -> (Run, Run) {
    let mut old = current.clone(); old.simulation_revision = 45.0.into();
    events::begin(); operation(&mut old); let old_events = events::finish();
    events::begin(); operation(&mut current); let current_events = events::finish();
    assert_eq!(current_events, old_events, "attack geometry, damage labels and events");
    let mut a = old.clone(); let mut b = current.clone();
    a.simulation_revision = 46.0.into();
    for r in [&mut a, &mut b] {
        r.log.clear(); r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    }
    assert_eq!(a, b, "all non-Chronicle state, including RNG, damage and conditions");
    (old, current)
}

#[test]
fn weapon_identity_tracks_each_hand_and_implicit_counter_path() {
    let mut r = room("rat");
    r.player.conditions.insert("lucky".into(), 1.);
    let mut main = equipment::make_equipment("Shortsword", 90.).unwrap();
    main.equipped = "hand1".to_string().into();
    let mut off = main.clone(); off.id = 91.; off.name = "Unspecified weapon".into();
    off.equipped = "hand2".to_string().into(); r.inventory = vec![main, off];
    let (old, current) = same_gameplay(r.clone(), |r| engine::player_attack(r, 10.).unwrap());
    assert!(old.log[0].starts_with("You strike Giant Rat for "));
    assert!(current.log[0].starts_with("You slash Giant Rat for "));
    assert!(current.log[1].starts_with("You strike Giant Rat for ")
        || current.log[1] == "You try to strike Giant Rat, but miss.");
    let (_, current) = same_gameplay(r, |r| engine::attack(r, r.player.id, 10., 1.).unwrap());
    assert!(current.log[0].starts_with("You slash Giant Rat for "));
}

#[test]
fn hit_and_miss_use_creature_identity_and_correct_person() {
    for hit in [false, true] {
        let mut r = room("night-crawler");
        engine::actor_mut_trusted(&mut r, 10.).unwrap().conditions.insert(
            if hit { "lucky" } else { "distracted" }.into(), 1.);
        let (old, current) = same_gameplay(r, |r| engine::attack(r, 10., r.player.id, 1.).unwrap());
        assert!(old.log[0].starts_with("Night Crawler strikes you"));
        if hit { assert!(current.log[0].starts_with("Night Crawler skewers you for ")); }
        else { assert_eq!(current.log[0], "Night Crawler tries to skewer you, but misses."); }
    }
    let mut r = room("rat");
    r.player.name = "Tiger".into(); // Never infer anatomy from the player's name.
    r.player.conditions.insert("distracted".into(), 1.);
    let (_, current) = same_gameplay(r, |r| engine::player_attack(r, 10.).unwrap());
    assert_eq!(current.log[0], "You try to strike Giant Rat, but miss.");
}

#[test]
fn ranged_routes_preserve_hit_rolls_and_projectile_identity() {
    for (kind, phrase) in [("river-drake", "jets water at"), ("valta", "fires a moonbeam at"), ("horn-3", "fires a magical bolt at")] {
        for hit in [false, true] {
            let mut r = room(kind);
            engine::actor_mut_trusted(&mut r, 10.).unwrap().conditions.insert(
                if hit { "lucky" } else { "distracted" }.into(), 1.);
            let name = engine::actor(&r, 10.).unwrap().name.clone();
            assert_eq!(engine::attack_blocker(&r, engine::actor(&r, 10.).unwrap(), &r.player, 1., false), None,
                "{kind}: {:?}", crate::projectiles::trace_from_actor(&r, engine::actor(&r, 10.).unwrap(), r.player.x, r.player.y));
            let (_, current) = same_gameplay(r, |r| {
                engine::attack_ranged_resolved(r, 10., r.player.id, 22., 1., 1., false).unwrap();
            });
            if hit { assert!(current.log[0].starts_with(&format!("{name} {phrase} you for "))); }
            else { assert!(current.log[0].contains("tries to ") && current.log[0].ends_with(", but misses.")); }
        }
    }
    for arrow in [false, true] {
        let mut r = room("rat"); r.player.conditions.insert("lucky".into(), 1.);
        let (_, current) = same_gameplay(r, |r| {
            engine::attack_ranged_resolved(r, r.player.id, 10., 20., 1., 1., arrow).unwrap();
        });
        assert!(current.log[0].starts_with(if arrow { "You shoot at Giant Rat for " } else { "You throw at Giant Rat for " }));
    }
}

#[test]
fn nonattacking_creatures_stay_nonattacking_and_zero_damage_is_not_a_miss() {
    for kind in ["maulchild", "bastidion", "samara"] {
        let r = room(kind);
        let (_, current) = same_gameplay(r, |r| engine::attack(r, 10., r.player.id, 1.).unwrap());
        assert!(current.log.is_empty(), "{kind}");
    }
    let mut r = room("sphinx"); r.player.conditions.insert("lucky".into(), 1.);
    let (_, current) = same_gameplay(r, |r| engine::player_attack(r, 10.).unwrap());
    assert!(current.log.iter().any(|s| s == "You strike Sphinx for 0 damage."));
}

#[test]
fn deflection_preserves_the_original_projectile_verb() {
    let mut r = room("river-drake");
    r.player.abilities.push(crate::divine_reactions::DEFLECT.into());
    r.player.active.push(crate::divine_reactions::DEFLECT.into());
    r.player.reactions = 1.0.into();
    engine::actor_mut_trusted(&mut r, 10.).unwrap().conditions.insert("distracted".into(), 1.);
    let (_, current) = same_gameplay(r, |r| {
        engine::attack_ranged_resolved(r, 10., r.player.id, 22., 1., 1., false).unwrap();
    });
    assert_eq!(current.log.len(), 2);
    assert_eq!(current.log[0], "River Drake tries to jet water at you, but misses.");
    assert!(current.log[1].contains("water at River Drake"), "{}", current.log[1]);
    assert!(!current.log[1].contains("strike"));
}
