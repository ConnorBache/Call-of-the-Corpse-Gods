//! Explicit fixed-power fixtures; no loot tiers or R50 release admission.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, combat, engine,
    environment, monster_statuses as statuses, revision50_rings as rings,
    temporal::{self, History, Snapshot}};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.energy = 37.;
    r.player.stats = Stats { offense: 10., dr: 2., accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear();
    // Empty floor contents without removing blocks required by save validation.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 900.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing; f.environment = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing; f.currents = Field::Missing;
    f.fungal_roots = Field::Missing; f.roots = Field::Missing; f.mud = Field::Missing;
    f.slime = Field::Missing; f.ichor = Field::Missing; f.razorleaves = Field::Missing;
    f.water_blood = Field::Missing; f.kelpie_blood_sources = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    r
}
fn ring(id: f64, effect: &str) -> Item {
    Item { id, name: format!("Ring of {effect}"), kind: "ring".into(), glyph: "=".into(),
        effect: effect.to_string().into(), ..Default::default() }
}
fn enemy(id: f64, x: f64, y: f64) -> Actor {
    Actor { id, x, y, name: "Ring test actor".into(), glyph: "e".into(), color: "#ffffff".into(),
        hp: 1000., max_hp: 1000., base_max_hp: 1000.0.into(), energy: 100., state: "hunting".into(),
        stats: Stats { offense: 10., accuracy: 100., perception: 300., max_energy: 100., ..Default::default() },
        abilities: vec!["enemy:beast:no-revival".into()], ..Default::default() }
}
fn gas(r: &mut Run, x: usize, kind: &str, source: f64) {
    let at = 10 * r.floor().unwrap().width() + x;
    environment::add_gas(r.floor_mut().unwrap(), at, kind, 1000., source).unwrap();
}
fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}"); }
fn all_rings() -> Vec<Item> {
    vec![ring(1., rings::WILLPOWER), ring(2., rings::PROTECTION), ring(3., rings::SLAYING)]
}

#[test]
fn protection_and_slaying_add_four_once_without_accuracy_or_quality_scaling() {
    let mut r = room(50.);
    r.player.abilities.push("isheilah-tools-of-the-trade".into()); // Allows six ring units.
    let baseline = combat::effective_stats(&r, &r.player);
    r.inventory = all_rings();
    r.inventory.extend([ring(4., rings::PROTECTION), ring(5., rings::SLAYING)]);
    r.inventory[3].enchantment = 200.0.into(); r.inventory[4].quantity = 2.0.into();
    let before = serde_json::to_vec(&r).unwrap();
    let mut expected = baseline.clone(); expected.dr += 4.; expected.offense += 4.;
    assert_eq!(combat::effective_stats(&r, &r.player), expected);
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    assert_eq!(corpse_core::revision50_player_attack::base(&r), 45. + 2. * expected.offense);
    for item in &r.inventory { assert!(rings::description(item).is_some()); }
    for flag in [Field::Missing, Field::Null, Field::Value(String::new()), "ring1".to_string().into()] {
        for item in &mut r.inventory { item.equipped = flag.clone(); }
        assert_eq!(combat::effective_stats(&r, &r.player), expected);
    }
    assert!(corpse_core::revision50_inventory::finite_categories_fit(&r.inventory, &r.player.abilities));
    r.simulation_revision = 49.0.into();
    assert_eq!(combat::effective_stats(&r, &r.player), baseline);
}

#[test]
fn protection_uses_ordinary_dr_and_does_not_change_bypassing_packets() {
    let mut baseline = room(50.); baseline.actors_at_mut(1.).push(enemy(901., 11., 10.));
    let mut protected = baseline.clone(); protected.inventory.push(ring(1., rings::PROTECTION));
    let id = baseline.player.id;
    engine::attack(&mut baseline, 901., id, 1.).unwrap();
    engine::attack(&mut protected, 901., id, 1.).unwrap();
    close(protected.player.hp - baseline.player.hp, 2.); // +4 DR before the ordinary 0.5 damage scale.
    assert_eq!(protected.rng, baseline.rng);
    for kind in ["fire", "caustic", "poison", "venom", "hemophilia", "fall"] {
        let old = protected.player.hp;
        engine::damage_environment(&mut protected, id, 5., None, kind, true).unwrap();
        close(protected.player.hp, old - 5.);
    }
    let mut pierced = room(50.); let mut a = enemy(901., 11., 10.);
    a.abilities.push("enemy:beast:armor-piercing".into()); pierced.actors_at_mut(1.).push(a);
    let mut protected = pierced.clone(); protected.inventory.push(ring(1., rings::PROTECTION));
    engine::attack(&mut pierced, 901., id, 1.).unwrap();
    engine::attack(&mut protected, 901., id, 1.).unwrap();
    close(protected.player.hp, pierced.player.hp);
}

#[test]
fn slaying_changes_live_offense_strikes_without_changing_hit_rolls() {
    let mut baseline = room(50.); baseline.actors_at_mut(1.).push(enemy(901., 11., 10.));
    let mut slaying = baseline.clone(); slaying.inventory.push(ring(1., rings::SLAYING));
    let id = baseline.player.id;
    engine::attack(&mut baseline, id, 901., 1.).unwrap();
    engine::attack(&mut slaying, id, 901., 1.).unwrap();
    assert!(engine::actor(&slaying, 901.).unwrap().hp < engine::actor(&baseline, 901.).unwrap().hp);
    assert_eq!(slaying.rng, baseline.rng);
    assert_eq!(combat::effective_stats(&slaying, &slaying.player).accuracy,
        combat::effective_stats(&baseline, &baseline.player).accuracy);
}

#[test]
fn mental_duration_rounds_up_fractional_incoming_durations_and_never_stacks() {
    for revision in [29., 33., 43., 46., 49., 50.] {
        let mut r = room(revision); r.inventory = vec![ring(1., rings::WILLPOWER), ring(2., rings::WILLPOWER)];
        for duration in [0.1_f64, 1., 1.1, 2., 2.1, 3., 5., 20.] {
            assert_eq!(statuses::willpower_duration(&r, &r.player, duration),
                if revision >= 50. { (duration / 2.).ceil() } else { duration });
        }
        for duration in [0., -1., f64::INFINITY] {
            assert_eq!(statuses::willpower_duration(&r, &r.player, duration), duration);
        }
        assert!(statuses::willpower_duration(&r, &r.player, f64::NAN).is_nan());
    }
}

#[test]
fn gas_confusion_and_hallucination_halve_without_source_or_rng_changes() {
    for (gas_type, key, full, reduced) in [("confusion", "confused", 8., 4.), ("hallucinogenic", "hallucinating", 2., 1.)] {
        for source in [-1., -2., 901.] {
            let mut baseline = room(50.); baseline.actors_at_mut(1.).push(enemy(901., 20., 10.));
            let mut protected = baseline.clone(); protected.inventory.push(ring(1., rings::WILLPOWER));
            gas(&mut baseline, 10, gas_type, source); gas(&mut protected, 10, gas_type, source);
            environment::tick_environment(&mut baseline).unwrap(); environment::tick_environment(&mut protected).unwrap();
            assert_eq!(engine::condition(&baseline.player, key), full);
            assert_eq!(engine::condition(&protected.player, key), reduced);
            assert_eq!(baseline.rng, protected.rng);
            assert_eq!(baseline.floor().unwrap().environment, protected.floor().unwrap().environment);
            close(baseline.player.hp, protected.player.hp);
            gas(&mut protected, 10, gas_type, source); environment::tick_environment(&mut protected).unwrap();
            assert_eq!(engine::condition(&protected.player, key), reduced, "refresh never compounds halving");
        }
        let mut legacy = room(49.); legacy.inventory.push(ring(1., rings::WILLPOWER));
        gas(&mut legacy, 10, gas_type, -1.); environment::tick_environment(&mut legacy).unwrap();
        assert_eq!(engine::condition(&legacy.player, key), full);
    }
}

#[test]
fn authored_witch_and_bloodmancer_curses_are_reduced_at_the_spell_boundary() {
    for (ability, phase, key, full, reduced) in [
        ("enemy:beast:witch", 0., "slowed", 5., 3.),
        ("enemy:beast:witch", 1., "hemophilia", 20., 10.),
        ("enemy:beast:witch", 2., "poisoned", 20., 10.),
        ("enemy:beast:bloodmancy", 1., "hemophilia", 20., 10.),
    ] {
        for (revision, expected) in [(49., full), (50., reduced)] {
            let mut r = room(revision); r.inventory.push(ring(1., rings::WILLPOWER));
            let mut caster = enemy(901., 12., 10.); caster.abilities.push(ability.into());
            caster.conditions.insert("monsterSpellPhase".into(), phase); r.actors_at_mut(1.).push(caster);
            let player = r.player.id;
            assert!(corpse_core::monster_spells::special_action(&mut r, 901., player).unwrap());
            assert_eq!(engine::condition(&r.player, key), expected, "{ability}: {key}");
            assert_eq!(statuses::status_source(&r, &r.player, key), Some(901.));
            assert_eq!(engine::actor(&r, 901.).unwrap().energy, 92.);
        }
    }
}

#[test]
fn ordinary_slow_poison_physical_paralysis_and_silence_do_not_shorten() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::WILLPOWER));
    r.actors_at_mut(1.).push(enemy(901., 12., 10.)); let player = r.player.id;
    for key in ["slowed", "poisoned", "hemophilia"] {
        assert!(statuses::apply_status(&mut r, player, key, 5., Some(901.)));
        assert_eq!(engine::condition(&r.player, key), 5.);
    }
    gas(&mut r, 10, "paralysis", -1.); environment::tick_environment(&mut r).unwrap();
    assert_eq!(engine::condition(&r.player, "paralyzed"), 2.);
    r.player.conditions.insert("silenced".into(), 5.);
    assert_eq!(statuses::willpower_duration(&r, &r.player, 0.), 0.);
    corpse_core::equipment::sync_ownership(&mut r).unwrap();
    assert_eq!(engine::condition(&r.player, "silenced"), 5.);
    assert_eq!(statuses::hemophilia_episode_damage(&r.player), 2.);
}

#[test]
fn enemy_curses_round_up_without_reducing_existing_longer_timers_or_potency() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::WILLPOWER));
    r.actors_at_mut(1.).push(enemy(901., 12., 10.)); let player = r.player.id;
    for duration in [0.1_f64, 1., 1.1, 2., 2.1, 3., 5., 20.] {
        r.player.conditions.shift_remove("hemophilia");
        assert!(statuses::apply_curse(&mut r, player, "hemophilia", duration, 901.));
        assert_eq!(engine::condition(&r.player, "hemophilia"), (duration / 2.).ceil());
    }
    r.player.conditions.insert("hemophilia".into(), 17.);
    assert!(statuses::apply_curse(&mut r, player, "hemophilia", 5., 901.));
    assert_eq!(engine::condition(&r.player, "hemophilia"), 17.);
    assert_eq!(statuses::hemophilia_episode_damage(&r.player), 2.);
    let old = r.player.hp;
    statuses::tick_poisoned(&mut r, player).unwrap(); close(r.player.hp, old);
    assert!(statuses::apply_curse(&mut r, player, "poisoned", 5., 901.));
    statuses::tick_poisoned(&mut r, player).unwrap(); close(r.player.hp, old - 2.);
    for duration in [0., -1., f64::INFINITY, f64::NAN] {
        let before = r.clone(); assert!(!statuses::apply_curse(&mut r, player, "hemophilia", duration, 901.));
        assert_eq!(r, before);
    }
}

#[test]
fn self_or_allied_curses_are_not_enemy_curses_but_hostile_recipients_can_resist() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::WILLPOWER));
    let player = r.player.id;
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., player));
    assert_eq!(engine::condition(&r.player, "slowed"), 5.);
    r.player.conditions.shift_remove("slowed");
    let mut ally = enemy(901., 12., 10.); ally.state = "ally".into(); r.actors_at_mut(1.).push(ally);
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    assert_eq!(engine::condition(&r.player, "slowed"), 5.);
    let mut hostile = enemy(902., 14., 10.); hostile.copied_inventory = vec![ring(2., rings::WILLPOWER)].into();
    r.actors_at_mut(1.).push(hostile);
    assert!(statuses::apply_curse(&mut r, 902., "slowed", 5., 901.));
    assert_eq!(engine::condition(engine::actor(&r, 902.).unwrap(), "slowed"), 3.);
}

#[test]
fn pickup_and_drop_only_change_new_applications_and_live_stats() {
    let mut r = room(50.);
    r.actors_at_mut(1.).push(enemy(901., 12., 10.)); let player = r.player.id;
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    r.player.conditions.insert("confused".into(), 7.); r.player.conditions.insert("hallucinating".into(), 2.);
    let before = r.player.conditions.clone();
    let (x, y) = (r.player.x, r.player.y);
    r.ground_at_mut(1.).extend(all_rings().into_iter().map(|item| corpse_core::GroundItem { x, y, item, ..Default::default() }));
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r, 20.).unwrap(), 3);
    assert_eq!(r.player.conditions, before);
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 14.);
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    assert_eq!(engine::condition(&r.player, "slowed"), 5.);
    r.player.conditions.insert("slowed".into(), 1.);
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    assert_eq!(engine::condition(&r.player, "slowed"), 3.);
    for id in [1., 2., 3.] {
        assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id }).unwrap(), Some(true));
    }
    assert!(r.inventory.is_empty());
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 10.);
    assert_eq!(engine::condition(&r.player, "slowed"), 3.);
    assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    assert_eq!(engine::condition(&r.player, "slowed"), 5.);
}

#[test]
fn half_here_keeps_status_local_and_uses_owner_rings_once() {
    let mut r = room(50.); r.inventory = all_rings();
    r.player.abilities.push(corpse_core::half_here::ID.into());
    let clone = corpse_core::half_here::create(&mut r, 14., 10.).unwrap();
    r.actors_at_mut(1.).push(enemy(901., 16., 10.));
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, clone).unwrap()), combat::effective_stats(&r, &r.player));
    assert!(statuses::apply_curse(&mut r, clone, "hemophilia", 5., 901.));
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "hemophilia"), 3.);
    assert_eq!(engine::condition(&r.player, "hemophilia"), 0.);
    gas(&mut r, 14, "hallucinogenic", -1.); environment::tick_environment(&mut r).unwrap();
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "hallucinating"), 1.);
    assert_eq!(engine::condition(&r.player, "hallucinating"), 0.);
}

#[test]
fn actor_local_powers_exclude_decoys_and_minime_keeps_only_its_frozen_numeric_stats() {
    let mut r = room(50.); r.inventory = all_rings();
    let mut a = enemy(901., 14., 10.); a.copied_inventory = all_rings().into();
    assert_eq!(combat::effective_stats(&r, &a).offense, 14.);
    assert_eq!(combat::effective_stats(&r, &a).dr, 4.);
    assert_eq!(statuses::willpower_duration(&r, &a, 5.), 3.);
    a.conditions.insert(corpse_core::revision50_nightmares::MARKER.into(), 1.);
    assert_eq!(combat::effective_stats(&r, &a).offense, 10.);
    assert_eq!(combat::effective_stats(&r, &a).dr, 0.);
    assert_eq!(statuses::willpower_duration(&r, &a, 5.), 5.);
    let copied = corpse_core::mini_me::preview(&r, 15., 10.);
    assert_eq!(combat::effective_stats(&r, &copied), combat::effective_stats(&r, &r.player));
    assert_eq!(statuses::willpower_duration(&r, &copied, 5.), 5.);
    assert!(copied.copied_inventory.is_missing());
    r.inventory.clear();
    assert_eq!(combat::effective_stats(&r, &copied).offense, 14.);
    assert_eq!(combat::effective_stats(&r, &copied).dr, 6.);
}

#[test]
fn haste_movement_does_not_retick_or_rehalve_applied_statuses() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::WILLPOWER));
    r.phase = "haste".to_string().into();
    r.player.conditions.insert("hasted".into(), 5.);
    r.player.conditions.insert("hallucinating".into(), 1.);
    r.player.conditions.insert("hemophilia".into(), 3.);
    let tick = r.tick;
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert_eq!(r.tick, tick);
    assert_eq!(r.phase.as_ref().map(String::as_str), Some("normal"));
    assert_eq!(engine::condition(&r.player, "hasted"), 5.);
    assert_eq!(engine::condition(&r.player, "hallucinating"), 1.);
    assert_eq!(engine::condition(&r.player, "hemophilia"), 3.);
    // The following normal action still ticks each applied duration once.
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(r.tick, tick + 1.);
    assert_eq!(r.phase.as_ref().map(String::as_str), Some("haste"));
    assert_eq!(engine::condition(&r.player, "hasted"), 4.);
    assert_eq!(engine::condition(&r.player, "hallucinating"), 0.);
    assert_eq!(engine::condition(&r.player, "hemophilia"), 2.);
}

#[test]
fn raw_snapshot_and_preexisting_future_preserve_applied_durations_without_rehalving() {
    let mut r = room(50.); r.inventory = all_rings(); r.actors_at_mut(1.).push(enemy(901., 12., 10.));
    let player = r.player.id; assert!(statuses::apply_curse(&mut r, player, "slowed", 5., 901.));
    r.player.conditions.insert("hallucinating".into(), 1.);
    let expected = combat::effective_stats(&r, &r.player);
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    let mut loaded = corpse_core::storage::deserialize(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
    loaded.inventory.clear(); loaded.player.conditions.clear(); snapshot.restore(&mut loaded).unwrap();
    assert_eq!(engine::condition(&loaded.player, "slowed"), 3.);
    assert_eq!(engine::condition(&loaded.player, "hallucinating"), 1.);
    assert_eq!(combat::effective_stats(&loaded, &loaded.player), expected);
    loaded.future = Some(corpse_core::storage::serialize(&loaded).unwrap().into());
    loaded.inventory.clear(); loaded.player.conditions.clear();
    assert!(temporal::restore_future(&mut loaded, &mut History::default()).unwrap());
    assert_eq!(engine::condition(&loaded.player, "slowed"), 3.);
    assert_eq!(engine::condition(&loaded.player, "hallucinating"), 1.);
    assert_eq!(combat::effective_stats(&loaded, &loaded.player), expected);
    // This intentionally tests persisted anchors, not closed Engine50 admission.
}

#[test]
fn sustain_halves_all_eight_timed_stat_reductions_with_exact_fractions() {
    for (revision, expected) in [(49., 17.), (50., 18.5)] {
        let mut r = room(revision);
        r.player.stats = Stats { offense: 20., defense: 20., accuracy: 20., dodge: 20., dr: 20.,
            perception: 20., max_energy: 20., max_hp: 20.0.into() };
        r.inventory = vec![ring(1., rings::SUSTAIN_STRENGTH), ring(2., rings::SUSTAIN_STRENGTH)];
        r.inventory[1].enchantment = 99.0.into(); r.inventory[1].quantity = 3.0.into();
        for key in ["offense", "defense", "accuracy", "dodge", "dr", "perception", "maxEnergy", "maxHp"] {
            r.player.conditions.insert(format!("buff:{key}"), -2.5); // Normal source rounding makes this -3.
            r.player.conditions.insert(format!("buffTicks:{key}"), 7.);
        }
        let before = r.clone();
        assert_eq!(combat::effective_stats(&r, &r.player), Stats { offense: expected, defense: expected,
            accuracy: expected, dodge: expected, dr: expected, perception: expected,
            max_energy: expected, max_hp: expected.into() });
        assert_eq!(r, before, "neither magnitude storage nor duration is rewritten");
    }
}

#[test]
fn sustain_preserves_base_equipment_untimed_penalties_and_item_rounding() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::SUSTAIN_STRENGTH));
    r.inventory[0].bonus = corpse_core::PartialStats { offense: (-5.0).into(), ..Default::default() }.into();
    r.inventory[0].enhancements.push(corpse_core::ItemEnhancementsEntry {
        bonus: corpse_core::PartialStats { offense: 2.0.into(), ..Default::default() }, ..Default::default()
    });
    r.player.conditions.insert("buff:offense".into(), -3.);
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 4., "untimed penalties are not temporary");
    r.player.conditions.insert("buffTicks:offense".into(), 5.);
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 5.5, "item enhancement must not reround the half");
    r.player.conditions.insert("buff:offense".into(), 3.);
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 10., "positive buffs remain full strength");
    r.player.conditions.clear(); r.player.stats.offense = -7.;
    assert_eq!(combat::effective_stats(&r, &r.player).offense, -10., "negative permanent base stats remain full");
}

#[test]
fn sustain_halves_self_weakness_and_enemy_penalties_without_shortening_timers() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::SUSTAIN_STRENGTH));
    for (key, value) in [("weakened", 7.), ("screamWeakened", 5.), ("screamPenalty", 3.),
        ("disoriented", 5.), ("disorientedPenalty", 5.), ("gasVulnerableTicks", 5.), ("gasVulnerable", 1.)] {
        r.player.conditions.insert(key.into(), value);
    }
    let conditions = r.player.conditions.clone(); let hp = r.player.hp; let energy = r.player.energy;
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 6.); // 10 - 2.5 - 1.5.
    assert_eq!(combat::effective_stats(&r, &r.player).accuracy, 97.5);
    assert_eq!(r.player.conditions, conditions); close(r.player.hp, hp); close(r.player.energy, energy);
    environment::tick_environment(&mut r).unwrap();
    assert_eq!(engine::condition(&r.player, "disoriented"), 4.);
    assert_eq!(engine::condition(&r.player, "screamWeakened"), 4.);
    assert_eq!(engine::condition(&r.player, "gasVulnerable"), 1., "gas damage vulnerability is not a stat reduction");
    assert_eq!(combat::effective_stats(&r, &r.player).accuracy, 97.5);
    r.player.conditions.insert("disoriented".into(), 0.);
    assert_eq!(combat::effective_stats(&r, &r.player).accuracy, 100.);
    // Voluntary resource spending is not a stat penalty.
    let player = r.player.id;
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player, "numo-distract", 50., 20.).unwrap();
    assert!(corpse_core::revision50_ability_energy::spend(&mut r, player, "numo-distract", 7.));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["numo-distract"].energy, 13.);
    engine::damage_environment(&mut r, player, 10., Some(player), "blood-weapon", true).unwrap();
    close(r.player.hp, hp - 10.);
}

#[test]
fn sustain_pickup_drop_and_health_timer_changes_rederive_without_mutating_debuffs() {
    let mut r = room(50.);
    let (x, y) = (r.player.x, r.player.y);
    r.ground_at_mut(1.).push(corpse_core::GroundItem { x, y, item: ring(1., rings::SUSTAIN_STRENGTH), ..Default::default() });
    r.player.conditions.insert("buff:defense".into(), -3.);
    r.player.conditions.insert("buff:maxHp".into(), -3.);
    for key in ["defense", "maxHp"] { r.player.conditions.insert(format!("buffTicks:{key}"), 5.); }
    corpse_core::equipment::sync_health(&mut r, false);
    close(r.player.max_hp, 79.);
    let statuses = r.player.conditions.clone();
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r, 20.).unwrap(), 1);
    close(r.player.max_hp, 89.5); assert_eq!(r.player.conditions, statuses);
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id: 1. }).unwrap(), Some(true));
    close(r.player.max_hp, 79.); assert_eq!(r.player.conditions, statuses);
    for item in r.ground_at_mut(1.) { item.auto_pickup_blocked = Field::Missing; }
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r, 20.).unwrap(), 1);
    close(r.player.max_hp, 89.5);
    r.player.conditions.shift_remove("buffTicks:defense");
    assert!(corpse_core::equipment::flush_health(&mut r)); close(r.player.max_hp, 80.5);
    r.player.conditions.shift_remove("buffTicks:maxHp");
    assert!(corpse_core::equipment::flush_health(&mut r)); close(r.player.max_hp, 79.);
    assert!(!corpse_core::equipment::flush_health(&mut r));
}

#[test]
fn sustain_uses_owner_defensive_buffs_endpoint_offense_and_frozen_minime_stats() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::SUSTAIN_STRENGTH));
    r.player.conditions.insert("buff:defense".into(), -3.);
    r.player.conditions.insert("buffTicks:defense".into(), 5.);
    r.player.conditions.insert("weakened".into(), 5.);
    r.player.abilities.push(corpse_core::half_here::ID.into());
    let clone = corpse_core::half_here::create(&mut r, 14., 10.).unwrap();
    engine::actor_mut(&mut r, clone).unwrap().conditions.insert("weakened".into(), 5.);
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, clone).unwrap()).defense, -1.5);
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, clone).unwrap()).offense, 7.5);
    let copied = corpse_core::mini_me::preview(&r, 15., 10.);
    assert!(!rings::has_effect(&r, &copied, rings::SUSTAIN_STRENGTH));
    let frozen = combat::effective_stats(&r, &copied);
    assert_eq!(frozen, combat::effective_stats(&r, &r.player));
    assert_eq!(frozen.offense, 7.5); assert_eq!(frozen.defense, -1.5);
    r.inventory.clear();
    assert_eq!(combat::effective_stats(&r, &copied), frozen);
    let mut local = enemy(902., 18., 10.); local.copied_inventory = vec![ring(2., rings::SUSTAIN_STRENGTH)].into();
    local.conditions.insert("weakened".into(), 5.);
    assert_eq!(combat::effective_stats(&r, &local).offense, 7.5);
    local.conditions.insert(corpse_core::revision50_nightmares::MARKER.into(), 1.);
    assert_eq!(combat::effective_stats(&r, &local).offense, 5.);
}

#[test]
fn offscreen_status_clocks_keep_reduced_application_and_exact_sustain_magnitude() {
    let mut r = room(50.); r.tick = 1.;
    let mut f = r.floor().unwrap().clone(); f.depth = 2.; f.last_tick = 0.; f.dirty = true;
    let at = r.floors.iter().position(|f| f.depth == 2.).unwrap(); r.floors[at] = std::sync::Arc::new(f);
    let mut a = enemy(901., 14., 10.); a.copied_inventory = all_rings().into();
    a.copied_inventory.as_mut().unwrap().push(ring(4., rings::SUSTAIN_STRENGTH));
    a.conditions.insert("paralyzed".into(), 100.); a.conditions.insert("hemophilia".into(), 3.);
    a.conditions.insert("buff:offense".into(), -3.); a.conditions.insert("buffTicks:offense".into(), 3.);
    a.conditions.insert("weakened".into(), 3.); r.actors_at_mut(2.).push(a);
    corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    let a = &r.actors_at(2.)[0];
    assert_eq!(engine::condition(a, "hemophilia"), 2.);
    assert_eq!(engine::condition(a, "buffTicks:offense"), 2.);
    assert_eq!(engine::condition(a, "weakened"), 2.);
    assert_eq!(combat::effective_stats(&r, a).offense, 10.); // 10 + Slaying4 -1.5 -2.5.
    assert_eq!(engine::condition(&r.player, "hemophilia"), 0.);
}

#[test]
fn sustain_snapshot_restores_ring_debuff_magnitude_and_duration() {
    let mut r = room(50.); r.inventory.push(ring(1., rings::SUSTAIN_STRENGTH));
    r.player.conditions.insert("weakened".into(), 5.);
    r.player.conditions.insert("buff:accuracy".into(), -3.);
    r.player.conditions.insert("buffTicks:accuracy".into(), 5.);
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    r.inventory.clear(); r.player.conditions.clear(); snapshot.restore(&mut r).unwrap();
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 7.5);
    assert_eq!(combat::effective_stats(&r, &r.player).accuracy, 98.5);
    assert_eq!(engine::condition(&r.player, "weakened"), 5.);
    assert_eq!(engine::condition(&r.player, "buffTicks:accuracy"), 5.);
}
