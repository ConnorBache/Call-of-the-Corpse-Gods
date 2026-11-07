//! Explicit fixture coverage, not production loot or R50 release admission.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine, environment,
    equipment, revision50_rings as rings, temporal::Snapshot};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 50.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.energy = 37.;
    r.player.stats = Stats { accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 900.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing; f.environment = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing; f.currents = Field::Missing;
    r
}
fn ring(id: f64, effect: &str) -> Item {
    Item { id, name: format!("Ring of {effect}"), kind: "ring".into(), glyph: "=".into(),
        effect: effect.to_string().into(), ..Default::default() }
}
fn actor(id: f64, x: f64, y: f64) -> Actor {
    Actor { id, x, y, name: "Ring test actor".into(), glyph: "e".into(), color: "#ffffff".into(),
        hp: 100., max_hp: 100., base_max_hp: 100.0.into(), energy: 100., state: "hunting".into(),
        stats: Stats { accuracy: 100., perception: 300., max_energy: 100., ..Default::default() },
        heartless: true.into(), abilities: vec!["enemy:beast:no-revival".into()], ..Default::default() }
}
fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}"); }
fn hit(r: &mut Run, attacker: f64, target: f64) { engine::attack(r, attacker, target, 1.).unwrap(); }
fn packet(r: &mut Run, kind: &str, amount: f64, exact: bool) {
    let id = r.player.id; engine::damage_environment(r, id, amount, None, kind, exact).unwrap();
}
fn tile(r: &mut Run, x: usize, y: usize, kind: &str) {
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[y * w + x] = kind.into();
}

#[test]
fn new_identities_are_carried_revision_gated_and_count_distinct_items() {
    for effect in [rings::VAMPIRISM, rings::FLIGHT, rings::FIRE_RESISTANCE,
        rings::POISON_RESISTANCE, rings::CORROSION_RESISTANCE, rings::LIGHT] {
        let mut r = room(50.); r.inventory = vec![ring(1., effect), ring(2., effect)];
        r.inventory[1].quantity = 3.0.into(); r.inventory[1].enchantment = 99.0.into();
        assert_eq!(rings::count_effect(&r, &r.player, effect), 2);
        assert!(rings::description(&r.inventory[0]).is_some());
        for flag in [Field::Missing, Field::Null, Field::Value(String::new()), "ring2".to_string().into()] {
            r.inventory[0].equipped = flag;
            assert_eq!(rings::count_effect(&r, &r.player, effect), 2);
        }
        r.inventory.remove(0);
        assert_eq!(rings::count_effect(&r, &r.player, effect), 1);
        r.inventory[0].kind = "amulet".into();
        assert!(!rings::has_effect(&r, &r.player, effect));
        r.inventory[0].kind = "ring".into(); r.simulation_revision = 49.0.into();
        assert!(!rings::has_effect(&r, &r.player, effect));
    }
}

#[test]
fn vampirism_heals_actual_melee_hp_loss_adds_fight_long_and_never_stacks() {
    for (copies, fight, fraction) in [(0, false, 0.), (1, false, 0.05), (3, false, 0.05), (1, true, 0.25)] {
        let mut r = room(50.);
        for i in 0..copies { r.inventory.push(ring(10. + i as f64, rings::VAMPIRISM)); }
        if fight { r.player.abilities.push(corpse_core::divine_combat::FIGHT_LONG.into()); }
        let mut target = actor(901., 11., 10.); target.hp = 3.; r.actors_at_mut(r.depth).push(target);
        let id = r.player.id; hit(&mut r, id, 901.);
        assert!(engine::actor(&r, 901.).unwrap().dead.value_or(false));
        close(r.player.hp, 50. + 3. * fraction); close(r.player.energy, 37.);
    }
}

#[test]
fn vampirism_caps_hp_and_ignores_ranged_misses_and_zero_damage() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::VAMPIRISM));
    r.actors_at_mut(r.depth).push(actor(901., 11., 10.));
    r.player.hp = 99.99; let id = r.player.id; hit(&mut r, id, 901.); close(r.player.hp, 100.);
    for range in [0., 1.] {
        let mut r = room(50.); r.inventory.push(ring(1., rings::VAMPIRISM));
        r.player.abilities.push(corpse_core::divine_combat::FIGHT_LONG.into());
        r.actors_at_mut(r.depth).push(actor(901., 11., 10.)); let id = r.player.id;
        engine::attack_ranged(&mut r, id, 901., 20., range, 0., true).unwrap();
        assert!(engine::actor(&r, 901.).unwrap().hp < 100.);
        close(r.player.hp, 50.);
    }
    for (accuracy, dr) in [(-1e6, 0.), (100., 1e6)] {
        let mut r = room(50.); r.inventory.push(ring(1., rings::VAMPIRISM));
        r.player.stats.accuracy = accuracy;
        let mut a = actor(901., 11., 10.); a.stats.dr = dr; r.actors_at_mut(r.depth).push(a);
        let id = r.player.id; hit(&mut r, id, 901.); close(r.player.hp, 50.);
    }
}

#[test]
fn allies_never_heal_player_and_actor_local_vampirism_heals_only_attacker() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::VAMPIRISM));
    let mut ally = actor(901., 11., 10.); ally.state = "ally".into(); ally.hp = 40.;
    ally.copied_inventory = vec![ring(2., rings::VAMPIRISM)].into();
    r.actors_at_mut(r.depth).extend([ally, actor(902., 12., 10.)]);
    hit(&mut r, 901., 902.);
    let loss = 100. - engine::actor(&r, 902.).unwrap().hp;
    assert!(loss > 0.); close(engine::actor(&r, 901.).unwrap().hp, 40. + loss * 0.05);
    close(r.player.hp, 50.);
}

#[test]
fn reverse_entropy_still_blocks_hostile_vampirism() {
    let mut r = room(50.);
    r.player.abilities.push("dawel-reverse-entropy".into());
    r.player.active.push("dawel-reverse-entropy".into());
    let mut a = actor(901., 11., 10.); a.hp = 40.;
    a.copied_inventory = vec![ring(1., rings::VAMPIRISM)].into();
    r.actors_at_mut(r.depth).push(a); let id = r.player.id; hit(&mut r, 901., id);
    assert!(r.player.hp < 50.); close(engine::actor(&r, 901.).unwrap().hp, 40.);
}

#[test]
fn fire_resistance_halves_final_packets_including_fractions_and_keeps_immunity() {
    for (revision, copies, expected) in [(49., 1, 3.), (50., 0, 3.), (50., 1, 1.5), (50., 3, 1.5)] {
        let mut r = room(revision);
        for n in 0..copies { r.inventory.push(ring(n as f64 + 1., rings::FIRE_RESISTANCE)); }
        packet(&mut r, "fire", 3., true); close(r.player.hp, 50. - expected);
    }
    let mut r = room(50.); r.inventory.push(ring(1., rings::FIRE_RESISTANCE));
    packet(&mut r, "fire", 9., false); close(r.player.hp, 48.5); // ceil(9/4)/2
    packet(&mut r, "fire", 0.1, true); close(r.player.hp, 48.); // ceil(.1)/2
    r.player.abilities.push("enemy:beast:fire-immunity".into());
    packet(&mut r, "fire", 20., true); close(r.player.hp, 48.);
    r.player.abilities = vec!["anavani-star-power".into()];
    packet(&mut r, "fire", 20., true); close(r.player.hp, 48.);
}

#[test]
fn fire_and_poison_statuses_keep_duration_and_only_normal_turn_ticks() {
    let mut r = room(50.); r.inventory = vec![ring(1., rings::FIRE_RESISTANCE), ring(2., rings::POISON_RESISTANCE)];
    r.player.conditions.insert("burning".into(), 5.); r.player.conditions.insert("poisoned".into(), 5.);
    engine::dispatch(&mut r, &Command::Wait).unwrap();
    close(r.player.hp, 47.2); // +.2 natural regeneration, -2 burning, -1 poison
    close(engine::condition(&r.player, "burning"), 4.); close(engine::condition(&r.player, "poisoned"), 4.);
    r.player.conditions.insert("hasted".into(), 4.);
    let tick = r.tick;
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    close(r.tick, tick); close(r.player.hp, 47.2);
    close(engine::condition(&r.player, "poisoned"), 4.);
}

#[test]
fn fire_resistance_does_not_prevent_ignition_or_change_terrain_or_other_actors() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::FIRE_RESISTANCE));
    r.actors_at_mut(r.depth).push(actor(901., 11., 10.));
    environment::initialize(r.floor_mut().unwrap()).unwrap();
    let w = r.floor().unwrap().width();
    environment::ignite(&mut r, 10., 10., None, None).unwrap();
    let id = r.player.id; environment::refresh_burning_from_fire(&mut r, id).unwrap();
    assert!(engine::condition(&r.player, "burning") > 0.);
    assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[10 * w + 10] > 0.);
    engine::damage_environment(&mut r, 901., 3., None, "fire", true).unwrap();
    close(engine::actor(&r, 901.).unwrap().hp, 97.);
}

#[test]
fn poison_and_corrosion_resistances_are_distinct_and_do_not_mitigate_other_kinds() {
    for (effect, kinds) in [(rings::POISON_RESISTANCE, vec!["poison", "venom"]),
        (rings::CORROSION_RESISTANCE, vec!["caustic", "ichor", "breatherGas"])] {
        for kind in ["poison", "venom", "caustic", "ichor", "fire", "breatherGas", "attack", "bleeding"] {
            let mut r = room(50.); r.inventory = vec![ring(1., effect), ring(2., effect)];
            packet(&mut r, kind, 1., true);
            close(r.player.hp, if kinds.contains(&kind) {49.5} else {49.});
        }
    }
    let mut r = room(50.); r.inventory.push(ring(1., rings::CORROSION_RESISTANCE));
    packet(&mut r, "ichor", 17., false); close(r.player.hp, 47.5); // ceil(17/4)/2
    r.player.abilities.push("grisha-entropy".into());
    packet(&mut r, "caustic", 20., true); close(r.player.hp, 47.5);
}

#[test]
fn flight_crosses_chasms_water_and_currents_but_not_fire_gas_or_attacks() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::FLIGHT));
    tile(&mut r, 11, 10, "chasm");
    let id = r.player.id; assert!(engine::move_actor(&mut r, id, 1., 0., false).unwrap());
    close(r.depth, 1.); close(r.player.x, 11.);
    tile(&mut r, 11, 10, "water"); tile(&mut r, 12, 10, "water");
    let f = r.floor_mut().unwrap(); let w = f.width();
    let mut currents = vec![0.; f.tiles.len()]; currents[10 * w + 11] = 3.; f.currents = currents.into();
    corpse_core::currents::tick_currents(&mut r).unwrap(); close(r.player.x, 11.);
    assert!(!environment::is_swimming(&r, &r.player));
    for kind in ["fire", "caustic", "attack"] { packet(&mut r, kind, 3., true); }
    close(r.player.hp, 41.);
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id: 1. }).unwrap(), Some(true));
    assert!(!corpse_core::travel::flying(&r, &r.player));
    assert!(environment::is_swimming(&r, &r.player));
    corpse_core::currents::tick_currents(&mut r).unwrap(); close(r.player.x, 12.);
}

#[test]
fn removing_last_flight_ring_settles_fall_immediately_but_preserves_other_flight() {
    let mut r = room(50.); r.inventory = vec![ring(1., rings::FLIGHT), ring(2., rings::FLIGHT)];
    tile(&mut r, 10, 10, "pit"); let tick = r.tick;
    for (id, expected_status) in [(1., "playing"), (2., "won")] {
        let before = equipment::capture_before(&r);
        r.inventory.retain(|i| i.id != id);
        equipment::settle_removed(&mut r, before).unwrap();
        assert_eq!(r.status, expected_status);
    }
    close(r.tick, tick);
    let mut r = room(50.); r.inventory.push(ring(1., rings::FLIGHT)); tile(&mut r, 10, 10, "pit");
    r.player.conditions.insert("flying".into(), 5.);
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id: 1. }).unwrap(), Some(true));
    assert_eq!(r.status, "playing");
    assert!(corpse_core::travel::flying(&r, &r.player));
    let mut r = room(50.); r.inventory.push(ring(1., rings::FLIGHT)); tile(&mut r, 10, 10, "pit");
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id: 1. }).unwrap(), Some(true));
    assert_eq!(r.status, "won");
}

#[test]
fn half_here_shares_combat_ring_owner_and_minime_decoys_inherit_none() {
    let mut r = room(50.);
    r.inventory = vec![ring(1., rings::FLIGHT), ring(2., rings::FIRE_RESISTANCE), ring(3., rings::VAMPIRISM)];
    r.player.abilities.push(corpse_core::half_here::ID.into());
    let id = corpse_core::half_here::create(&mut r, 14., 10.).unwrap();
    assert!(corpse_core::travel::flying(&r, engine::actor(&r, id).unwrap()));
    engine::damage_environment(&mut r, id, 3., None, "fire", true).unwrap(); close(r.player.hp, 48.5);
    let mut target = actor(901., 15., 10.); target.hp = 3.; r.actors_at_mut(r.depth).push(target);
    // Half Here never independently attacks; its existing secondary-hit healing
    // hook must still resolve the shared wearer rather than create a new pool.
    assert_eq!(engine::attack_blocker(&r, engine::actor(&r, id).unwrap(), engine::actor(&r, 901.).unwrap(), 0., false), Some("nonattacking-clone"));
    let endpoint = engine::actor(&r, id).unwrap().clone();
    corpse_core::divine_combat::heal_melee_damage(&mut r, &endpoint, 3., true);
    close(r.player.hp, 48.65); close(engine::actor(&r, id).unwrap().hp, 48.65);
    let mut mini = actor(902., 18., 10.); mini.copied_inventory = r.inventory.clone().into();
    for marker in ["miniMe", corpse_core::revision50_nightmares::MARKER] {
        mini.conditions.clear(); mini.conditions.insert(marker.into(), 1.);
        for effect in [rings::FLIGHT, rings::FIRE_RESISTANCE, rings::VAMPIRISM] {
            assert!(!rings::has_effect(&r, &mini, effect));
        }
        assert!(!corpse_core::travel::flying(&r, &mini));
    }
}

#[test]
fn exact_offscreen_status_resistance_does_not_use_absent_player_rings() {
    let mut r = room(50.); r.tick = 2.;
    r.inventory = vec![ring(1., rings::FIRE_RESISTANCE), ring(2., rings::POISON_RESISTANCE)];
    let mut f = r.floor().unwrap().clone(); f.depth = 2.; f.last_tick = 0.; f.dirty = true;
    let i = r.floors.iter().position(|f| f.depth == 2.).unwrap(); r.floors[i] = std::sync::Arc::new(f);
    for (id, worn) in [(901., true), (902., false)] {
        let mut a = actor(id, 14. + (id - 901.), 10.); a.hp = 50.;
        a.conditions.insert("paralyzed".into(), 100.); a.conditions.insert("burning".into(), 5.);
        a.conditions.insert("poisoned".into(), 5.);
        if worn { a.copied_inventory = r.inventory.clone().into(); }
        r.actors_at_mut(2.).push(a);
    }
    corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    close(r.actors_at(2.)[0].hp, 44.4); close(r.actors_at(2.)[1].hp, 38.4);
    close(r.player.hp, 50.); close(engine::condition(&r.actors_at(2.)[0], "poisoned"), 3.);
}

#[test]
fn raw_save_and_snapshot_preserve_powers_and_independent_grants() {
    let mut r = room(50.);
    r.player.abilities.push("isheilah-tools-of-the-trade".into()); // Six rings fit the authored bonus.
    r.inventory = vec![ring(1., rings::VAMPIRISM), ring(2., rings::VAMPIRISM), ring(3., rings::FLIGHT),
        ring(4., rings::FIRE_RESISTANCE), ring(5., rings::POISON_RESISTANCE), ring(6., rings::CORROSION_RESISTANCE)];
    r.inventory[0].granted_ability = "numo-mind-read".to_string().into();
    r.inventory[1].granted_ability = "anavani-shine".to_string().into();
    equipment::sync_ownership(&mut r).unwrap();
    for id in ["numo-mind-read", "anavani-shine"] { assert!(r.player.abilities.iter().any(|a|a == id)); }
    let mut loaded: Run = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    let snapshot = Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();
    loaded.inventory.clear(); snapshot.restore(&mut loaded).unwrap();
    assert!(corpse_core::travel::flying(&loaded, &loaded.player));
    packet(&mut loaded, "fire", 3., true); close(loaded.player.hp, 48.5);
    assert_eq!(rings::count_effect(&loaded, &loaded.player, rings::VAMPIRISM), 2);
}

#[test]
fn vampirism_reads_old_draft_identity_but_displays_only_new_name() {
    let mut r = room(50.); let mut old = ring(1., "transference");
    old.name = "Ring of Transference".into(); r.inventory.push(old);
    assert_eq!(rings::effect(&r.inventory[0]), Some(rings::VAMPIRISM));
    assert_eq!(rings::display_name(&r.inventory[0]), Some("Ring of Vampirism"));
    assert!(rings::has_effect(&r, &r.player, rings::VAMPIRISM));
}

#[test]
fn carried_reaping_rejects_manual_toggles_without_stranding_normal_or_haste_commands() {
    for phase in ["normal", "haste"] {
        for slot in ["ring", "ring1", "ring2"] {
            let mut r = room(50.);
            r.phase = phase.to_string().into();
            r.player.conditions.insert("hasted".into(), 5.);
            let mut item = ring(1., rings::REAPING);
            item.equipped = Field::Missing;
            r.inventory.push(item);
            equipment::sync_ownership(&mut r).unwrap();
            assert!(corpse_core::revision50_amulets::validate_run(&r).is_ok());
            for command in [Command::Equip { id:1., slot:slot.into() }, Command::Unequip { id:1. }] {
                let blocker = equipment::change_blocker(&r, &command).unwrap();
                assert!(blocker.contains("always active"));
                let before = (r.inventory.clone(), r.player.clone(), r.tick, r.phase.clone(), r.rng);
                assert!(!engine::dispatch(&mut r, &command).unwrap().took_turn);
                assert_eq!((r.inventory.clone(), r.player.clone(), r.tick, r.phase.clone(), r.rng), before);
                assert!(r.log.iter().any(|message| message == &blocker));
                assert!(rings::has_effect(&r, &r.player, rings::REAPING));
            }
            assert!(corpse_core::revision50_amulets::validate_run(&r).is_ok());
            engine::dispatch(&mut r, &Command::Drop { id:1. }).unwrap();
            assert!(!rings::has_effect(&r, &r.player, rings::REAPING));
            engine::dispatch(&mut r, &Command::Wait).unwrap();
            assert_eq!(r.status,"playing");
        }
    }
}
