//! R51 reduces fixed player attack components, never the numeric Damage bonus.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, combat, engine, mini_me,
    projectiles, revision50_player_attack as player_attack, rng, storage, temporal::Snapshot};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.rng = 123456.; r.next_id = 1000.;
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.base_max_hp = 1000.0.into(); r.player.energy = 100.;
    r.player.stats = Stats { offense: 20., accuracy: 1000., perception: 100., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.inventory.clear();
    // Keep the required empty actor blocks for every generated fixture floor.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into();
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.environment = Field::Missing; f.doors = Field::Missing; f.non_euclidean = Field::Missing;
    f.portals = Field::Missing; f.currents = Field::Missing;
    r.actors_at_mut(r.depth).push(Actor { id: 901., name: "Target".into(),
        x: 11., y: 10., hp: 1000., max_hp: 1000., base_max_hp: 1000.0.into(),
        state: "hunting".into(), heartless: true.into(),
        stats: Stats { accuracy: 1000., ..Default::default() }, ..Default::default() });
    r
}
fn bow() -> Item {
    Item { id: 950., name: "Bow".into(), kind: "weapon".into(), damage: 15.0.into(),
        enchantment: 3.0.into(), offense_multiplier: 0.5.into(),
        ranged_weapon: "bow".to_string().into(), hands: 2.0.into(),
        equipped: "hand1".to_string().into(), ..Default::default() }
}
fn hatchet(thrown: bool) -> Item {
    Item { id: 952., name: "Hatchet".into(), kind: "weapon".into(), damage: 30.0.into(),
        enchantment: 3.0.into(), offense_multiplier: 1.5.into(),
        hands: 1.0.into(), thrown: thrown.into(), ..Default::default() }
}
fn variance(r: &Run) -> (f64, f64) {
    let mut predicted = r.clone(); rng::random(&mut predicted);
    let factor = 0.75 + rng::random(&mut predicted) * 0.5;
    (factor, predicted.rng)
}
fn hp(r: &Run) -> f64 { engine::actor(r, 901.).unwrap().hp }

#[test]
fn revision51_scales_only_fixed_components_and_preserves_bonus_coefficients() {
    let r = room(51.); let old = room(50.);
    assert_eq!(player_attack::with_offense_for_run(&r, 0.), 31.5);
    assert_eq!(player_attack::with_offense_for_run(&old, 0.), 45.);
    assert_eq!(player_attack::base(&r), 71.5);
    assert_eq!(player_attack::base(&old), 85.);
    // A +1 internal offense still means +1 nominal melee Damage, not +0.7.
    for offense in [-5., 0., 1.5, 20., 100.] {
        assert_eq!(player_attack::with_offense_for_run(&r, offense + 1.)
            - player_attack::with_offense_for_run(&r, offense), 2.);
        assert_eq!(engine::player_weapon_strike_base_for_revision(&bow(), offense + 1., 51.)
            - engine::player_weapon_strike_base_for_revision(&bow(), offense, 51.), 0.5);
        assert_eq!(projectiles::player_throw_base_for_revision(&hatchet(true), offense + 1., 51.)
            - projectiles::player_throw_base_for_revision(&hatchet(true), offense, 51.), 1.5);
    }
    assert_eq!(engine::player_weapon_strike_base(&r, &bow()), 23.5); // 10.5 + 3 + 10
    assert_eq!(engine::player_weapon_strike_base(&old, &bow()), 28.);
    assert_eq!(projectiles::player_throw_base(&r, &hatchet(true)), 54.); // 21 + 3 + 30
    assert_eq!(projectiles::player_throw_base(&old, &hatchet(true)), 63.);
    assert_eq!(projectiles::player_throw_base(&r, &hatchet(false)), 27.);
    assert_eq!(projectiles::player_throw_base(&old, &hatchet(false)), 31.5);
    let mut enchanted = bow(); enchanted.enchantment = 4.0.into();
    assert_eq!(engine::player_weapon_strike_base(&r, &enchanted)
        - engine::player_weapon_strike_base(&r, &bow()), 1.);
    for item in [Item { kind: "tool".into(), damage: 500.0.into(), ..Default::default() }, bow()] {
        assert_eq!(projectiles::player_throw_base(&r, &item), 20.);
        assert_eq!(projectiles::player_throw_base(&old, &item), 20.);
    }
    let before = r.clone(); player_attack::base(&r); engine::player_weapon_strike_base(&r, &bow());
    projectiles::player_throw_base(&r, &hatchet(true)); assert_eq!(r, before);
}

#[test]
fn main_generic_and_haste_player_attacks_use_revisioned_base_without_extra_rng() {
    for revision in [46., 50., 51.] { for offense in [0., 20.] { for dr in [0., 10., 80.] {
        for phase in ["normal", "haste"] {
            let mut r = room(revision); r.player.stats.offense = offense; r.phase = phase.to_string().into();
            engine::actor_mut(&mut r, 901.).unwrap().stats.dr = dr;
            let mut generic = r.clone(); let (v, next_rng) = variance(&r);
            let base = if revision < 50. { 30. + offense }
                else if revision < 51. { 45. + 2. * offense } else { 31.5 + 2. * offense };
            let expected = combat::ordinary_strike_damage_for_run(&r, base, v, dr, false, false);
            let id = r.player.id;
            engine::player_attack(&mut r, 901.).unwrap();
            engine::attack(&mut generic, id, 901., 1.).unwrap();
            assert_eq!(1000. - hp(&r), expected, "main rev {revision}, offense {offense}, DR {dr}, {phase}");
            assert_eq!(hp(&generic), hp(&r));
            // Positive hits on this bleeding target also use the unchanged
            // cosmetic blood-splatter stream: one count roll and 2..5 cell rolls.
            let mut expected_rng = generic.clone(); expected_rng.rng = next_rng;
            if expected > 0. {
                let droplets = 2 + (rng::random(&mut expected_rng) * 4.).floor() as usize;
                for _ in 0..droplets { rng::random(&mut expected_rng); }
            }
            assert_eq!(r.rng, expected_rng.rng, "main rev {revision}, offense {offense}, DR {dr}, {phase}");
            assert_eq!(generic.rng, expected_rng.rng);
        }
    }}}
}

#[test]
fn committed_bows_and_throw_types_apply_fixed_reduction_once() {
    for revision in [46., 50., 51.] {
        for item in [bow(), hatchet(true), hatchet(false), Item {
            id: 953., name: "Rock".into(), kind: "tool".into(), ..Default::default()
        }] {
            let mut r = room(revision); r.player.conditions.insert("lucky".into(), 1.);
            let shoot = projectiles::is_bow(&item);
            let fixed = if revision >= 51. { 0.7 } else { 1. };
            let base = if shoot { 15. * fixed + 3. + 10. }
                else if item.kind == "weapon" { (30. * fixed + 3. + 30.) / if item.thrown.value_or(false) { 1. } else { 2. } }
                else { 20. };
            r.inventory.push(item.clone());
            if shoot { r.inventory.push(Item { id: 951., name: "Arrow".into(), kind: "ammo".into(),
                quantity: 2.0.into(), effect: "arrows".to_string().into(), ..Default::default() }); }
            let (v, _) = variance(&r);
            let expected = combat::ordinary_strike_damage_for_run(&r, base, v, 0., false, false);
            let command = if shoot { Command::Shoot { id: item.id, x: 11., y: 10. } }
                else { Command::Throw { id: item.id, x: 11., y: 10. } };
            assert_eq!(projectiles::command(&mut r, &command).unwrap(), Some(true));
            assert_eq!(1000. - hp(&r), expected, "revision {revision}, {} thrown={}", item.name, item.thrown.value_or(false));
        }
    }
}

#[test]
fn reduction_precedes_variance_mitigation_and_only_final_damage_rounding() {
    let r = room(51.); let base = player_attack::with_offense_for_run(&r, 0.);
    assert_eq!(base, 31.5);
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 1., 10., false, false), 11.);
    // Early rounding 31.5 to 32 would incorrectly turn 11.4 into 12 here.
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 1., 8.7, false, false), 11.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 0.75, 10., false, false), 7.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 1.25, 10., false, false), 15.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 1., 30.5, false, false), 1.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&r, base, 1., 30.7, false, false), 0.);
}

#[test]
fn enemy_attacks_and_explicit_ability_packets_are_not_scaled() {
    for revision in [50., 51.] {
        let mut r = room(revision); let id = r.player.id;
        let a = engine::actor_mut(&mut r, 901.).unwrap();
        a.stats.offense = 20.; a.conditions.insert("weaponDamage".into(), 45.);
        a.conditions.insert("weaponOffenseMultiplier".into(), 2.);
        let (v, next_rng) = variance(&r);
        let expected = combat::ordinary_strike_damage_for_run(&r, 85., v, 0., false, false);
        engine::attack(&mut r, 901., id, 1.).unwrap();
        assert_eq!(1000. - r.player.hp, expected); assert_eq!(r.rng, next_rng);
        // Explicit attack/spell bases remain authored values, even when the source is the player.
        let (v, _) = variance(&r); r.player.conditions.insert("lucky".into(), 1.);
        let expected = combat::ordinary_strike_damage_for_run(&r, 80., v, 0., false, false);
        assert!(engine::attack_ranged(&mut r, id, 901., 80., 1., 1., true).unwrap());
        assert_eq!(1000. - hp(&r), expected);
        let before = hp(&r); engine::damage_direct(&mut r, 901., 20., id, "lightning", true).unwrap();
        assert_eq!(before - hp(&r), 20.);
    }
}

#[test]
fn copied_attack_bases_follow_frozen_mini_me_and_live_clay_semantics() {
    for revision in [50., 51.] {
        let mut r = room(revision);
        let mut copy = mini_me::preview(&r, 10., 11.);
        assert_eq!(copy.conditions["weaponDamage"], if revision < 51. {45.} else {31.5});
        assert_eq!(copy.conditions["weaponOffenseMultiplier"], 2.);
        assert_eq!(copy.stats.offense, 20.);
        // Copies stay frozen even after their creator gains more Damage.
        r.player.stats.offense = 40.;
        copy.conditions.insert("lucky".into(), 1.); let id = copy.id;
        r.actors_at_mut(r.depth).push(copy.clone());
        let (v, _) = variance(&r);
        let expected = combat::ordinary_strike_damage_for_run(&r,
            copy.conditions["weaponDamage"] + 2. * 20., v, 0., false, false);
        engine::attack(&mut r, id, 901., 1.).unwrap(); assert_eq!(1000. - hp(&r), expected);
        let clay = corpse_core::revision50_terrain::clay_attacker(&r,
            corpse_core::revision50_terrain::Cell { x: 10, y: 11 }, 1001., false);
        assert_eq!(clay.conditions["weaponDamage"] + clay.stats.offense, player_attack::base(&r));
        assert!(clay.abilities.is_empty()); assert!(copy.abilities.is_empty());
    }
}

#[test]
fn existing_copy_snapshots_and_enemy_weapon_conditions_survive_save_and_rewind() {
    let mut r = room(51.);
    let mut copy = mini_me::preview(&r, 10., 11.);
    // A pre-tuning R51 copy preserves its serialized independent baseline.
    copy.conditions.insert("weaponDamage".into(), 45.); let copy_id = copy.id;
    r.next_id += 1.; r.player.conditions.insert("miniMeId".into(), copy_id);
    r.actors_at_mut(r.depth).push(copy);
    engine::actor_mut(&mut r, 901.).unwrap().conditions.insert("weaponDamage".into(), 123.);
    let saved = storage::serialize(&r).unwrap();
    let loaded = storage::deserialize_compatible(&saved).unwrap();
    for state in [&r, &loaded] {
        assert_eq!(engine::actor(state, copy_id).unwrap().conditions["weaponDamage"], 45.);
        assert_eq!(engine::actor(state, 901.).unwrap().conditions["weaponDamage"], 123.);
        assert_eq!(player_attack::base(state), 71.5);
    }
    let snapshot = Snapshot::capture(&r);
    engine::actor_mut(&mut r, copy_id).unwrap().conditions.insert("weaponDamage".into(), 999.);
    snapshot.restore(&mut r).unwrap();
    assert_eq!(engine::actor(&r, copy_id).unwrap().conditions["weaponDamage"], 45.);
}

#[test]
fn synthetic_shared_avatar_secondary_attack_uses_live_owner_baseline() {
    let mut r = room(51.);
    let counter = "pelian-never-without-a-fight";
    r.player.abilities = vec![corpse_core::half_here::ID.into(), counter.into()];
    // This retired blessing cannot be acquired in R50+. Exercise only the
    // retained low-level secondary-attack/owner branch with explicit test state.
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player, counter, 50., 50.).unwrap();
    r.player.active.push(counter.into());
    let clone_id = corpse_core::half_here::create(&mut r, 11., 11.).unwrap();
    r.player.stats.offense = 40.; corpse_core::half_here::sync(&mut r);
    // Oracle: ordinary incoming hit without a counter, followed by the explicit
    // expected attack base. Both include their own existing blood-splatter RNG.
    let mut expected = r.clone();
    expected.player.active.retain(|id| id != counter); corpse_core::half_here::sync(&mut expected);
    engine::attack(&mut expected, 901., clone_id, 1.).unwrap();
    let player = expected.player.id;
    assert!(engine::attack_ranged(&mut expected, player, 901., 31.5 + 2. * 40., 0., 1., false).unwrap());
    engine::attack(&mut r, 901., clone_id, 1.).unwrap();
    assert_eq!(hp(&r), hp(&expected)); assert!(hp(&r) < 1000.);
    assert_eq!(r.rng, expected.rng);
}
