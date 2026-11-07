//! Explicit R50 fixtures only. These tests do not open new-run/replay admission
//! or author ring tiers, production loot, or ability-reserve capacities.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine,
    revision50_rings as rings, temporal::Snapshot};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.hp = 50.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    r.player.energy = 37.;
    r.player.stats = Stats { accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 900.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
    r
}
fn ring(id: f64, effect: &str) -> Item {
    Item { id, name: format!("Ring of {effect}"), kind: "ring".into(), glyph: "=".into(),
        effect: effect.to_string().into(), ..Default::default() }
}
fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}"); }
fn wait(r: &mut Run) { assert!(engine::dispatch(r, &Command::Wait).unwrap().took_turn); }
fn enemy(id: f64, x: f64, y: f64) -> Actor {
    let mut a = Actor { id, x, y, name: "Ring test actor".into(), glyph: "e".into(), color: "#ffffff".into(),
        hp: 50., max_hp: 100., base_max_hp: 100.0.into(), energy: 100., state: "hunting".into(),
        stats: Stats { max_energy: 100., ..Default::default() }, ..Default::default() };
    a.conditions.insert("paralyzed".into(), 100.); a
}

#[test]
fn identity_requires_carried_authored_ring_and_revision_not_equipped_flags() {
    let mut r = room(50.);
    r.inventory.push(ring(1., rings::REGENERATION));
    for effect in [rings::STEALTH, rings::LIGHT, rings::REGENERATION] {
        r.inventory[0].effect = effect.to_string().into();
        assert!(rings::has_effect(&r, &r.player, effect));
        assert!(rings::description(&r.inventory[0]).is_some());
    }
    r.inventory[0].effect = rings::REGENERATION.to_string().into();
    for equipped in [Field::Missing, Field::Null, Field::Value(String::new()), "ring1".to_string().into(), "amulet".to_string().into()] {
        r.inventory[0].equipped = equipped;
        assert!(rings::has_effect(&r, &r.player, rings::REGENERATION));
    }
    r.inventory[0].equipped = "ring1".to_string().into();
    for kind in ["amulet", "weapon", "potion"] {
        r.inventory[0].kind = kind.into();
        assert!(!rings::has_effect(&r, &r.player, rings::REGENERATION));
    }
    r.inventory[0].kind = "ring".into();
    for effect in [Field::Missing, "Regeneration".to_string().into(), "unknown".to_string().into()] {
        r.inventory[0].effect = effect;
        assert!(!rings::has_effect(&r, &r.player, rings::REGENERATION));
    }
    r.inventory[0].effect = rings::REGENERATION.to_string().into();
    r.simulation_revision = 49.0.into();
    assert!(!rings::has_effect(&r, &r.player, rings::REGENERATION));
}

#[test]
fn duplicate_fixed_powers_do_not_stack_or_remove_separate_tinker_grants() {
    let mut r = room(50.);
    let mut first = ring(1., rings::REGENERATION);
    first.granted_ability = "numo-mind-read".to_string().into();
    first.additional_granted_abilities.push("anavani-shine".into());
    let mut second = ring(2., rings::REGENERATION);
    second.granted_ability = "grisha-boundless-hunger".to_string().into();
    // No approved strength scale exists for these unrelated legacy fields.
    second.enchantment = 99.0.into(); second.quantity = 2.0.into();
    r.inventory = vec![first, second, ring(3., rings::LIGHT), ring(4., rings::STEALTH)];
    let before = serde_json::to_value(&r.inventory).unwrap();
    for effect in [rings::REGENERATION, rings::LIGHT, rings::STEALTH] {
        assert!(rings::has_effect(&r, &r.player, effect));
    }
    close(rings::natural_hp_rate(&r, &r.player), 0.004);
    corpse_core::equipment::sync_ownership(&mut r).unwrap();
    for id in ["numo-mind-read", "anavani-shine", "grisha-boundless-hunger"] {
        assert!(r.player.abilities.iter().any(|a| a == id), "{id}");
    }
    assert_eq!(serde_json::to_value(&r.inventory).unwrap(), before);
}

#[test]
fn regeneration_is_point_four_percent_while_carried_and_keeps_legacy_rate() {
    for (revision, copies, expected) in [(49., 1, 50.2), (50., 0, 50.2), (50., 1, 50.4), (50., 3, 50.4)] {
        let mut r = room(revision);
        for n in 0..copies { r.inventory.push(ring(n as f64 + 1., rings::REGENERATION)); }
        wait(&mut r); close(r.player.hp, expected); close(r.player.energy, 37.);
        for item in &mut r.inventory { item.equipped = Field::Null; }
        wait(&mut r); close(r.player.hp, expected + if revision >= 50. && copies > 0 { 0.4 } else { 0.2 });
        let before = corpse_core::equipment::capture_before(&r);
        r.inventory.clear();
        corpse_core::equipment::settle_removed(&mut r, before).unwrap();
        let hp = r.player.hp; wait(&mut r); close(r.player.hp, hp + 0.2);
    }
}

#[test]
fn normal_combat_regenerates_but_haste_bonus_movement_does_not() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    let mut target = enemy(901., 11., 10.); target.hp = 1000.; target.max_hp = 1000.;
    // Taking damage breaks paralysis, so the hostile still retaliates in this
    // real combat turn. Force that one attack to miss to isolate the HP gain.
    target.conditions.insert("distracted".into(), 1.);
    target.base_max_hp = 1000.0.into(); r.actors_at_mut(r.depth).push(target);
    assert!(engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert!(engine::actor(&r, 901.).unwrap().hp < 1000., "the normal turn must actually attack");
    close(r.player.hp, 50.4);
    r.player.conditions.insert("hasted".into(), 5.);
    let tick = r.tick;
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 0., dy: 1. }).unwrap().took_turn);
    assert_eq!(r.tick, tick); close(r.player.hp, 50.4);
}

#[test]
fn regeneration_never_refills_player_or_blessing_energy_and_caps_hp() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    r.player.abilities.push("numo-distract".into());
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player, "numo-distract", 50., 7.).unwrap();
    let reserves = r.player.ability_reserves.clone();
    r.player.hp = 99.9; wait(&mut r);
    close(r.player.hp, 100.); close(r.player.energy, 37.);
    assert_eq!(r.player.ability_reserves, reserves);
}

#[test]
fn existing_drown_and_creature_regeneration_multipliers_still_apply() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    r.player.abilities.push("etill-drown".into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10 * w + 10] = "shallow-water".into();
    wait(&mut r); close(r.player.hp, 51.2);
    let mut a = enemy(901., 14., 10.);
    a.abilities.push("enemy:beast:regeneration".into());
    a.copied_inventory = vec![ring(2., rings::REGENERATION)].into();
    r.actors_at_mut(r.depth).push(a);
    wait(&mut r); close(engine::actor(&r, 901.).unwrap().hp, 54.);
}

#[test]
fn reverse_entropy_still_suppresses_a_hostile_ring_wearer() {
    let mut r = room(50.);
    r.player.abilities.push("dawel-reverse-entropy".into());
    r.player.active.push("dawel-reverse-entropy".into());
    let mut a = enemy(901., 14., 10.); a.copied_inventory = vec![ring(2., rings::REGENERATION)].into();
    r.actors_at_mut(r.depth).push(a);
    wait(&mut r); close(engine::actor(&r, 901.).unwrap().hp, 50.);
}

#[test]
fn half_here_uses_owner_rings_and_one_regeneration_tick_with_water_endpoint() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    r.player.abilities.extend([corpse_core::half_here::ID.into(), "etill-drown".into()]);
    let id = corpse_core::half_here::create(&mut r, 14., 10.).unwrap();
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10 * w + 14] = "shallow-water".into();
    assert!(rings::has_effect(&r, engine::actor(&r, id).unwrap(), rings::REGENERATION));
    wait(&mut r); close(r.player.hp, 51.2); close(engine::actor(&r, id).unwrap().hp, 51.2);
}

#[test]
fn actor_local_equipment_does_not_leak_to_minime_decoys_or_ordinary_actors() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    let mut a = enemy(901., 14., 10.);
    assert!(!rings::has_effect(&r, &a, rings::REGENERATION));
    a.copied_inventory = vec![ring(2., rings::REGENERATION)].into();
    assert!(rings::has_effect(&r, &a, rings::REGENERATION));
    a.conditions.insert("miniMe".into(), 1.);
    assert!(!rings::has_effect(&r, &a, rings::REGENERATION));
    a.conditions.clear(); a.conditions.insert(corpse_core::revision50_nightmares::MARKER.into(), 1.);
    assert!(corpse_core::revision50_nightmares::is_decoy(&a));
    assert!(!rings::has_effect(&r, &a, rings::REGENERATION));
}

#[test]
fn exact_inactive_catchup_uses_actor_rings_without_healing_absent_player() {
    let mut r = room(50.); r.tick = 3.;
    r.inventory.push(ring(1., rings::REGENERATION));
    let mut f = r.floor().unwrap().clone(); f.depth = 2.; f.last_tick = 0.; f.dirty = true;
    let at = r.floors.iter().position(|f| f.depth == 2.).unwrap(); r.floors[at] = std::sync::Arc::new(f);
    let mut a = enemy(901., 14., 10.); a.copied_inventory = vec![ring(2., rings::REGENERATION), ring(3., rings::REGENERATION)].into();
    r.actors_at_mut(2.).push(a);
    corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    close(r.actors_at(2.)[0].hp, 51.2); close(r.player.hp, 50.); close(r.player.energy, 37.);
    corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    close(r.actors_at(2.)[0].hp, 51.2);
}

#[test]
fn ring_identity_and_power_survive_raw_save_and_snapshot_restore() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::REGENERATION));
    let mut loaded: Run = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert!(rings::has_effect(&loaded, &loaded.player, rings::REGENERATION));
    let snapshot = Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();
    loaded.inventory.clear(); loaded.player.hp = 1.; snapshot.restore(&mut loaded).unwrap();
    wait(&mut loaded); close(loaded.player.hp, 50.4);
    // Full Engine load/replay remains separately gated and is not claimed here.
}

#[test]
fn light_adds_ten_per_carried_copy_while_regeneration_stays_boolean() {
    let flags = [Field::Missing, Field::Null, Field::Value(String::new()),
        "ring1".to_string().into(), "amulet".to_string().into()];
    for copies in 0..=5 {
        let mut r = room(50.); r.floor_mut().unwrap().lights.fill(0.);
        for (n, flag) in flags.iter().take(copies).enumerate() {
            let mut item = ring(n as f64 + 1., rings::LIGHT); item.equipped = flag.clone();
            r.inventory.push(item);
        }
        assert_eq!(rings::count_effect(&r, &r.player, rings::LIGHT), copies);
        close(corpse_core::vision::tile_light(&r, 11, 10), 10. * copies as f64);
        close(corpse_core::vision::tile_light(&r, 10, 10), 0.);
        for item in &mut r.inventory { item.effect = rings::REGENERATION.to_string().into(); }
        close(rings::natural_hp_rate(&r, &r.player), if copies == 0 { 0.002 } else { 0.004 });
        assert!(corpse_core::revision50_inventory::finite_categories_fit(&r.inventory, &r.player.abilities));
    }
}
