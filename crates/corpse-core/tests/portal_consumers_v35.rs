use corpse_core::{Run, Actor, portals_generation::PortalPair};

/// O(A + floor cells) synthetic setup; transfer preserves global creature uniqueness.
fn scene() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 35.0.into();
    let mut enemy = r.actors_at_mut(1.).remove(0);
    r.depth = 10.;
    r.player.x = 12.; r.player.y = 14.; r.player.conditions.clear();
    r.player.active.clear(); r.player.abilities.clear();
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.energy = 100.;
    enemy.x = 60.; enemy.y = 14.; enemy.hp = 100.; enemy.max_hp = 100.;
    enemy.energy = 100.; enemy.state = "hunting".into();
    enemy.conditions.clear(); enemy.abilities.clear();
    *r.actors_at_mut(10.) = vec![enemy];
    let f = r.floor_mut().unwrap(); let w = f.width(); let h = f.height();
    for y in 0..h { for x in 0..w {
        f.tiles[y*w+x] = if x==0 || y==0 || x+1==w || y+1==h {"wall"} else {"floor"}.into();
    }}
    f.portals = vec![PortalPair { a: 14*w+13, b: 14*w+60, color_index: 0 }].into();
    f.lights.fill(100.);
    r
}

#[test]
fn endpoint_ranges_feed_half_here_and_counterattacks() {
    let mut r = scene();
    assert!(corpse_core::half_here::within_range(&r, 60., 14., 1.));
    let a = r.actors_at(10.)[0].clone();
    let mut b = r.player.clone(); b.abilities.push("shrail-i-hit-back".into());
    assert!(corpse_core::divine_reactions::prepare_hit_back(&mut r, &a, &b, 20., 0., 0., false).is_some());
    r.simulation_revision = 34.0.into();
    assert!(!corpse_core::half_here::within_range(&r, 60., 14., 1.));
}

#[test]
fn linked_grab_survives_until_endpoint_separation() {
    let mut r = scene(); let player = r.player.id;
    r.actors_at_mut(10.)[0].abilities.push("enemy:beast:grab".into());
    let id = r.actors_at(10.)[0].id;
    assert!(corpse_core::monster_grab::hold_action(&mut r, id, player));
    corpse_core::monster_grab::release_invalid(&mut r);
    assert!(r.player.conditions.contains_key("grabbed"));
    r.player.x = 10.;
    corpse_core::monster_grab::release_invalid(&mut r);
    assert!(!r.player.conditions.contains_key("grabbed"));
}

#[test]
fn native_plane_yeti_and_silence_reach_the_linked_endpoint() {
    let mut r = scene();
    r.actors_at_mut(10.)[0].abilities = vec!["enemy:beast:yeti".into(), "enemy:beast:silence-aura".into()];
    assert!(corpse_core::abilities::energy_activation_silenced(&r, &r.player));
    corpse_core::monster_chill::apply_adjacency(&mut r);
    assert_eq!(r.player.conditions.get("slowed"), Some(&1.));
    r.player.conditions.clear();
    r.actors_at_mut(10.)[0].conditions.insert("phantasmiiPlane".into(), 1.);
    assert!(!corpse_core::abilities::energy_activation_silenced(&r, &r.player));
    corpse_core::monster_chill::apply_adjacency(&mut r);
    assert!(!r.player.conditions.contains_key("slowed"));
}

#[test]
fn dryad_heals_through_portal_without_changing_recipient_identity() {
    let mut r = scene(); r.player.hp = 40.;
    let healer = &mut r.actors_at_mut(10.)[0]; healer.state = "ally".into();
    healer.abilities = vec!["enemy:beast:dryad".into()]; let id = healer.id;
    assert!(corpse_core::monster_spells::heal_others(&mut r, id).unwrap());
    assert_eq!(r.player.hp, 65.);
    assert_eq!((r.player.x,r.player.y), (12.,14.));
}

#[test]
fn mini_me_cannot_spawn_on_an_occupied_link_alias() {
    let mut r = scene();
    let f = r.floor_mut().unwrap(); let w = f.width();
    for y in 13..=15 { for x in 11..=13 { if (x,y)!=(12,14) { f.tiles[y*w+x]="wall".into(); }}}
    f.tiles[13*w+11]="floor".into();
    f.portals = vec![PortalPair {a:13*w+11,b:14*w+60,color_index:0}].into();
    assert!(corpse_core::mini_me::destination(&r).is_none());
    r.actors_at_mut(10.).clear();
    assert_eq!(corpse_core::mini_me::destination(&r), Some((11.,13.)));
}

#[test]
fn reverse_entropy_aura_uses_linked_range_but_not_other_plane() {
    let mut r = scene(); r.player.active.push("dawel-reverse-entropy".into());
    let mut victim: Actor = r.actors_at(10.)[0].clone();
    assert!(corpse_core::divine_environment::corpse_frozen(&r, &victim));
    victim.conditions.insert("phantasmiiPlane".into(),1.);
    assert!(!corpse_core::divine_environment::corpse_frozen(&r, &victim));
}

#[test]
fn horn_bolt_uses_portal_range_and_hits_the_actual_remote_actor() {
    let mut r = scene(); let player = r.player.id;
    // Range still imposes ordinary hit falloff; choose a deterministic successful roll.
    r.rng = 1.;
    r.player.x = 64.; r.player.y = 14.; r.player.stats.dodge = 0.;
    let caster = &mut r.actors_at_mut(10.)[0]; caster.x = 12.; caster.y = 14.;
    caster.abilities = vec!["enemy:beast:horn-bolt".into()]; caster.stats.accuracy = 100.;
    let id = caster.id;
    assert!(corpse_core::monster_horn::special_action(&mut r,id,player).unwrap());
    assert!(r.player.hp < 100., "hp={} log={:?}", r.player.hp, r.log);
    assert_eq!(corpse_core::engine::actor(&r,id).unwrap().energy,92.);
}

#[test]
fn directed_fire_cone_reaches_remote_space_without_cross_plane_damage() {
    let mut r = scene(); let player = r.player.id;
    r.player.x = 63.; r.player.y = 14.;
    let caster = &mut r.actors_at_mut(10.)[0]; caster.x = 12.; caster.y = 14.;
    caster.abilities = vec!["enemy:beast:fire-breath".into()];
    let id = caster.id;
    let mut dream = r.player.clone(); dream.id = 91.;
    dream.conditions.insert("phantasmiiPlane".into(),1.);
    r.actors_at_mut(10.).push(dream);
    assert!(corpse_core::monster_inventor::breathe_fire(&mut r,id,player).unwrap());
    assert_eq!(r.player.hp,76.);
    assert_eq!(corpse_core::engine::actor(&r,91.).unwrap().hp,100.);
    assert!(!corpse_core::engine::actor(&r,91.).unwrap().conditions.contains_key("burning"));
}
