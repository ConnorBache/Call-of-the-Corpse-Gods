use super::*;
use crate::{engine, rng, Actor, Field, Run};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.hp = 1000.; r.player.max_hp = 1000.; r.player.base_max_hp = 1000.0.into();
    r.player.stats = crate::Stats { accuracy: 20., dodge: 20., offense: 80., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.inventory.clear(); r.actors_at_mut(r.depth).clear(); r.ground.clear();
    r.history.clear(); r.replay.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.offers.clear();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.environment = Field::Missing; f.doors = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    r.actors_at_mut(r.depth).push(Actor {
        id: 800., x: 11., y: 10., name: "Test enemy".into(), state: "hunting".into(),
        hp: 1000., max_hp: 1000., base_max_hp: 1000.0.into(),
        stats: crate::Stats { accuracy: 20., dodge: 20., offense: 80., ..Default::default() },
        ..Default::default()
    });
    r
}
fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}"); }
fn strike(r: &mut Run, attacker: f64, victim: f64) -> bool {
    let before = engine::actor(r, victim).unwrap().hp;
    engine::attack(r, attacker, victim, 1.).unwrap();
    engine::actor(r, victim).unwrap().hp < before
}

#[test]
fn ratio_has_fifty_percent_equal_stats_and_caps_at_double_accuracy() {
    for (accuracy, dodge, expected) in [(10., 10., 0.5), (20., 10., 1.), (40., 10., 1.),
        (5., 10., 0.25), (1., 100., 0.005), (100., 1., 1.), (0.5, 0.5, 0.25)] {
        close(ratio_hit_chance(accuracy, dodge, 0.), expected);
    }
}
#[test]
fn ratio_zero_and_negative_stats_are_explicit_and_never_nan() {
    for accuracy in [-100., -1., 0., -0.] {
        for dodge in [-100., -1., 0., 1., 100.] {
            assert_eq!(ratio_hit_chance(accuracy, dodge, 0.), 0.);
        }
    }
    for dodge in [-100., -1., 0., -0.] { assert_eq!(ratio_hit_chance(1., dodge, 0.), 0.5); }
}
#[test]
fn ratio_retains_range_loss_after_the_base_cap() {
    for range in [0., 1., 4., 10.] {
        close(ratio_hit_chance(10., 10., range), 0.5 * 0.9f64.powf(range));
        close(ratio_hit_chance(100., 1., range), 0.9f64.powf(range));
    }
    assert_eq!(ratio_hit_chance(1., 1., 100000.), 0.);
}
#[test]
fn ratio_stays_bounded_for_extreme_or_invalid_inputs() {
    assert_eq!(ratio_hit_chance(f64::MAX, f64::MAX, 0.), 0.5);
    assert_eq!(ratio_hit_chance(f64::MAX, f64::MIN_POSITIVE, 0.), 1.);
    assert_eq!(ratio_hit_chance(20., 20., -1.), 0.);
    assert_eq!(ratio_hit_chance(f64::MIN_POSITIVE, f64::MAX, -100000.), 0.);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(ratio_hit_chance(bad, 20., 0.), 0.);
        assert_eq!(ratio_hit_chance(20., bad, 0.), 0.);
        assert_eq!(ratio_hit_chance(20., 20., bad), 0.);
    }
}
#[test]
fn older_run_revisions_keep_the_frozen_formula_exactly() {
    let mut r = room();
    for revision in [29., 33., 41., 46., 50., 51.] {
        r.simulation_revision = revision.into();
        for (a, d, range) in [(-1., -3., 0.), (0., 0., 0.), (20., 20., 4.), (100., 1., 1.)] {
            assert_eq!(hit_chance_for_run(&r, a, d, range), hit_chance(a, d, range));
        }
    }
    r.simulation_revision = Field::Missing;
    assert_eq!(hit_chance_for_run(&r, 20., 20., 0.), 0.65);
    r.simulation_revision = 52.0.into();
    assert_eq!(hit_chance_for_run(&r, 20., 20., 0.), 0.5);
}
#[test]
fn actual_melee_player_enemy_and_npc_duels_use_the_ratio_roll() {
    for (attacker, victim) in [(0., 800.), (800., 0.), (800., 801.)] {
        for seed in [1., 1973., 8192., 12345., 50000., 100000., 123456789.] {
            let mut r = room(); r.rng = seed;
            if victim == 801. {
                let mut other = engine::actor(&r, 800.).unwrap().clone();
                other.id = 801.; other.x = 12.; other.state = "ally".into();
                r.actors_at_mut(r.depth).push(other);
            }
            let mut expected = r.clone(); let hit = rng::random(&mut expected) < 0.5;
            assert_eq!(strike(&mut r, attacker, victim), hit, "{attacker} -> {victim}, seed {seed}");
        }
    }
}
#[test]
fn actual_projectiles_keep_range_and_bow_miss_reduction() {
    for miss_multiplier in [1., 0.5] {
        for seed in [1., 1973., 8192., 12345., 50000., 100000., 123456789.] {
            let mut r = room(); r.rng = seed;
            engine::actor_mut_trusted(&mut r, 800.).unwrap().x = 14.;
            let chance = 1. - (1. - 0.5 * 0.9f64.powf(4.)) * miss_multiplier;
            let mut expected = r.clone(); let hit = rng::random(&mut expected) < chance;
            assert_eq!(engine::attack_ranged(&mut r, 0., 800., 80., 4., miss_multiplier, false).unwrap(), hit);
        }
    }
}
#[test]
fn actual_overrides_keep_their_existing_priority() {
    for case in ["zero", "surprise", "lucky", "unlucky", "distracted", "paralyzed", "surprise-unlucky"] {
        let mut r = room(); r.rng = 12345.; r.player.stats.accuracy = 0.;
        match case {
            "surprise" | "surprise-unlucky" => engine::actor_mut_trusted(&mut r, 800.).unwrap().state = "wandering".into(),
            "lucky" => {r.player.conditions.insert("lucky".into(), 1.);},
            "unlucky" => {r.player.stats.accuracy = 100.; r.player.conditions.insert("unlucky".into(), 1.);},
            "distracted" | "paralyzed" => {r.player.conditions.insert("lucky".into(), 1.); r.player.conditions.insert("distracted".into(), 1.);},
            _ => {}
        }
        if case == "surprise-unlucky" { r.player.conditions.insert("unlucky".into(), 1.); }
        if case == "paralyzed" { engine::actor_mut_trusted(&mut r, 800.).unwrap().conditions.insert("paralyzed".into(), 2.); }
        assert_eq!(strike(&mut r, 0., 800.), matches!(case, "surprise" | "lucky" | "paralyzed" | "surprise-unlucky"), "{case}");
    }
}
#[test]
fn actual_effective_accuracy_includes_equipment_buffs_and_shine() {
    for shine in [false, true] {
        let mut base = room();
        base.player.conditions.insert("buff:accuracy".into(), -4.);
        base.inventory.push(crate::Item {id: 900., kind: "ring".into(),
            bonus: crate::PartialStats { accuracy: 10.0.into(), ..Default::default() }.into(), ..Default::default() });
        if shine { base.player.abilities.push("anavani-shine".into()); }
        let a = effective_stats(&base, &base.player).accuracy;
        assert_eq!(a, 26.);
        let b = engine::actor(&base, 800.).unwrap();
        let accuracy = a + if shine { integer_stat(crate::vision::light(&base, b)) } else { 0. };
        let chance = ratio_hit_chance(accuracy, effective_stats(&base, b).dodge, 0.);
        for seed in [8192., 12345., 100000.] {
            let mut r = base.clone(); r.rng = seed;
            let mut expected = r.clone(); let hit = rng::random(&mut expected) < chance;
            assert_eq!(strike(&mut r, 0., 800.), hit);
        }
    }
}
#[test]
fn save_snapshot_and_future_keep_revision_and_deterministic_next_hit() {
    let original = room();
    let saved: Run = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    let snapshot = crate::temporal::Snapshot::capture(&original);
    let future = crate::future_anchor::FutureAnchor::capture41(&original).unwrap();
    let mut expected = original.clone(); let hit = strike(&mut expected, 0., 800.);
    for mut restored in [saved, (*snapshot.state).clone(), future.native_state().unwrap().clone()] {
        assert_eq!(restored.simulation_revision, 52.0.into());
        assert_eq!(strike(&mut restored, 0., 800.), hit);
        assert_eq!(restored.rng, expected.rng);
        assert_eq!(engine::actor(&restored, 800.).unwrap().hp, engine::actor(&expected, 800.).unwrap().hp);
    }
}
#[test]
fn actual_starting_and_early_creature_stats_use_approved_baselines() {
    let r = crate::startup::new_run_shell_revision("0-1-2:ratio-audit", "Unblessed", false, 52.).unwrap();
    let player = effective_stats(&r, &r.player);
    assert_eq!((player.accuracy, player.dodge), (15., 15.));
    let rat = crate::balance52::profile("rat", 1.).unwrap();
    assert_eq!((rat.accuracy, rat.dodge), (-1., 0.));
    let born = crate::population::create_actor_revision("rat", 900., 1., 1., false, 52.).unwrap();
    assert_eq!((born.stats.accuracy, born.stats.dodge), (14., 10.));
    assert_eq!(hit_chance_for_run(&r, player.accuracy, born.stats.dodge, 0.), 0.75);
    close(hit_chance_for_run(&r, born.stats.accuracy, player.dodge, 0.), 14. / 30.);
    assert_eq!(crate::balance52::profiles().iter().filter(|p| p.accuracy <= 0.).count(), 8);
    assert!(crate::balance52::profiles().iter().all(|p| creature_birth_accuracy(52., p.accuracy) > 0.));
}

#[test]
fn denominator_floor_is_local_and_handles_fractional_negative_and_zero_dodge() {
    for dodge in [-100., -1., 0., 0.5, 1.] {
        assert_eq!(ratio_hit_chance(1., dodge, 0.), 0.5);
        assert_eq!(ratio_hit_chance(0.5, dodge, 0.), 0.25);
        assert_eq!(ratio_hit_chance(2., dodge, 0.), 1.);
    }
    assert_eq!(ratio_hit_chance(15., 10., 0.), 0.75);
    let r = room(); let before = r.clone();
    assert_eq!(hit_chance_for_run(&r, 1., -50., 0.), 0.5);
    assert_eq!(r, before);
}
#[test]
fn player_birth_accuracy_and_dodge_change_only_new_r52_runs() {
    for revision in [29., 33., 46., 50., 51., 52.] {
        for mode in ["Unblessed", "Blessed", "Avatar"] {
            let r = crate::startup::new_run_shell_revision("0-1-2:accuracy-baseline", mode, false, revision).unwrap();
            assert_eq!(r.player.stats.accuracy, if revision == 52. {15.} else {0.});
            assert_eq!(r.player.stats.dodge, if revision == 52. {15.} else {0.});
        }
    }
}
#[test]
fn creature_birth_dodge_has_a_birth_only_minimum_and_preserves_higher_values() {
    for offset in [-14., -3., 0., 12., 92.] {
        assert_eq!(creature_birth_dodge(52., offset), (offset + 10.).max(10.));
        assert_eq!(creature_birth_dodge(51., offset), offset);
    }
    assert_eq!(creature_birth_dodge(52., 0.), 10.);
    assert_eq!(creature_birth_dodge(52., -14.), 10.);
}

#[test]
fn new_player_dodge_keeps_low_accuracy_attacks_probabilistic() {
    let r = crate::startup::new_run_shell_revision("0-1-2:player-dodge", "Unblessed", false, 52.).unwrap();
    let dodge = effective_stats(&r, &r.player).dodge;
    assert_eq!(dodge, 15.);
    for (accuracy, chance) in [(1., 1. / 30.), (2., 1. / 15.), (10., 1. / 3.), (15., 0.5), (20., 2. / 3.), (30., 1.)] {
        close(hit_chance_for_run(&r, accuracy, dodge, 0.), chance);
    }
}

#[test]
fn creature_birth_accuracy_preserves_offsets_and_legacy_values() {
    for offset in [-3., -1., 0., 12., 185.] {
        assert_eq!(creature_birth_accuracy(52., offset), offset + 15.);
        assert_eq!(creature_birth_accuracy(51., offset), offset);
    }
    assert_eq!(creature_birth_accuracy(52., 0.), 15.);
}
