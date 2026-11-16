//! Player baseline and unchanged enemy birth profile regressions for R50.
use corpse_core::{Actor, Run, balance42, mini_me, physical_balance, revision50_player_attack};

fn attack_room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.rng = 123456.0;
    r.player.x = 10.0; r.player.y = 10.0;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.stats.offense = 20.0; r.player.stats.accuracy = 1000.0;
    r.inventory.clear(); r.actors.clear();
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.0);
    f.environment = corpse_core::Field::Missing; f.doors = corpse_core::Field::Missing;
    r.actors_at_mut(r.depth).push(Actor { id: 901.0, name: "Target".into(),
        x: 11.0, y: 10.0, hp: 1000.0, max_hp: 1000.0, state: "hunting".into(),
        stats: corpse_core::Stats { defense: 150.0, ..Default::default() }, ..Default::default() });
    r
}

#[test]
fn main_and_generic_player_strikes_share_the_revisioned_base() {
    for (revision, base) in [(46.0, 50.0), (50.0, 85.0)] {
        let mut expected = attack_room(revision); let mut main = expected.clone(); let mut generic = expected.clone();
        if revision == 50.0 {
            // Even stale hand fields cannot reintroduce weapon/offhand damage.
            for (id, slot) in [(950.0, "hand1"), (951.0, "hand2")] {
                main.inventory.push(corpse_core::Item { id, kind: "weapon".into(), damage: 999.0.into(),
                    equipped: slot.to_string().into(), ..Default::default() });
            }
        }
        let id = expected.player.id;
        assert!(corpse_core::engine::attack_ranged(&mut expected, id, 901.0, base, 0.0, 1.0, false).unwrap());
        corpse_core::engine::player_attack(&mut main, 901.0).unwrap();
        corpse_core::engine::attack(&mut generic, id, 901.0, 1.0).unwrap();
        let hp = |r: &Run| corpse_core::engine::actor(r, 901.0).unwrap().hp;
        assert!(hp(&expected) < 1000.0);
        assert_eq!(hp(&main), hp(&expected), "revision {revision} main");
        assert_eq!(hp(&generic), hp(&expected), "revision {revision} generic");
        assert_eq!(main.rng, expected.rng); assert_eq!(generic.rng, expected.rng);
    }
}

#[test]
fn explicit_ranged_damage_is_identical_across_player_baseline_revision() {
    let mut old = attack_room(46.0); let mut new = attack_room(50.0);
    let id = old.player.id;
    for r in [&mut old, &mut new] {
        r.player.conditions.insert("lucky".into(), 1.0);
        assert!(corpse_core::engine::attack_ranged(r, id, 901.0, 25.0, 1.0, 0.5, true).unwrap());
    }
    assert_eq!(corpse_core::engine::actor(&old, 901.0).unwrap().hp,
        corpse_core::engine::actor(&new, 901.0).unwrap().hp);
    assert_eq!(old.rng, new.rng);
}

#[test]
fn revision50_copies_greatsword_baseline_without_stale_offhand() {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.player.conditions.clear(); r.player.active.clear(); r.inventory.clear();
    r.player.stats.offense = 25.0;
    r.player.conditions.insert("weaponDamage".into(), 999.0);
    r.player.conditions.insert("offhandDamage".into(), 999.0);
    r.player.conditions.insert("offhandOffenseMultiplier".into(), 10.0);
    let copy = mini_me::preview(&r, 7.0, 7.0);
    assert_eq!(copy.conditions.get("weaponDamage"), Some(&revision50_player_attack::DAMAGE));
    assert_eq!(copy.conditions.get("weaponOffenseMultiplier"), Some(&revision50_player_attack::OFFENSE_MULTIPLIER));
    assert!(!copy.conditions.contains_key("offhandDamage"));
    assert!(!copy.conditions.contains_key("offhandOffenseMultiplier"));
    assert_eq!(copy.conditions["weaponDamage"] + copy.conditions["weaponOffenseMultiplier"]
        * corpse_core::combat::effective_stats(&r, &copy).offense, revision50_player_attack::base(&r));
    // Old no-equipment Mini-Me still uses its historical level-one Shortsword.
    r.simulation_revision = 46.0.into();
    let old = mini_me::preview(&r, 7.0, 7.0);
    assert_eq!(old.conditions.get("weaponDamage"), Some(&30.0));
    assert_eq!(old.conditions.get("weaponOffenseMultiplier"), Some(&1.0));
}

#[test]
fn revision50_preserves_latest_physical_profiles_without_new_retuning() {
    for row in physical_balance::tuning().iter().chain(physical_balance::tuning_revision(46.0)) {
        let mut old = Actor { creature_kind: row.kind.clone().into(), ..Default::default() };
        old.conditions.insert("unrelatedCondition".into(), 27.0);
        let mut new = old.clone();
        physical_balance::apply_new_actor(&mut old, 46.0);
        physical_balance::apply_new_actor(&mut new, 50.0);
        assert_eq!(new, old, "{} must retain the calibrated birth profile", row.kind);
    }
    // Includes natural and parent-specific context rows, including gas knobs.
    for row in balance42::profiles() {
        let mut old = Actor::default(); let mut new = old.clone();
        balance42::apply_new_actor_kind(&mut old, &row.kind, 46.0);
        balance42::apply_new_actor_kind(&mut new, &row.kind, 50.0);
        assert_eq!(new, old, "{} final profile", row.kind);
    }
}

#[test]
fn balance_admission_does_not_open_unimplemented_intermediate_revisions() {
    for revision in [47.0, 48.0, 49.0, 53.0, 50.5, 51.5, 52.5] {
        let mut a = Actor { creature_kind: "rat".to_string().into(), ..Default::default() };
        a.stats.offense = 123.0; a.stats.dodge = 456.0;
        let before = a.clone();
        physical_balance::apply_new_actor(&mut a, revision);
        balance42::apply_new_actor_kind(&mut a, "rat", revision);
        assert_eq!(a, before);
    }
}

#[test]
fn revision50_natural_birth_profiles_and_plans_preserve_revision46_except_authored_gas_and_caster_tags() {
    use corpse_core::{generation, population};
    for kind in population::catalog().actors.keys() {
        let mut expected = population::create_actor_revision(kind, 77.0, 2.0, 3.0, false, 46.0).unwrap();
        let new = population::create_actor_revision(kind, 77.0, 2.0, 3.0, false, 50.0).unwrap();
        // The later designer decision fixes Bastidion gas to hallucination;
        // this does not change any of its physical balance/profile values.
        if kind == "bastidion" { expected.conditions.insert("fungalGasType".into(), 6.); }
        // Explicit caster classification is new metadata, not an enemy stat retune.
        if matches!(kind.as_str(), "witch"|"hemogoblin-bloodmancer"|"dryad"|"horn-1"|"horn-2"|"horn-3"|"horn-4"|"horn-5"|"horn-6"|"horn-7"|"saidribee-queen"|"valta")
            && !expected.abilities.iter().any(|id| id==corpse_core::revision50_combat::MAGICKAL_CASTER) {
            expected.abilities.push(corpse_core::revision50_combat::MAGICKAL_CASTER.into());
        }
        // Designer-approved names/glyphs/colors are cosmetic; all other fields stay exact.
        corpse_core::revision50_creature_identity::apply_actor(&mut expected);
        assert_eq!(new, expected, "{kind} constructor");
    }
    for depth in 1..=40 {
        assert_eq!(population::zillennia_revision(depth, 46.0), population::zillennia_revision(depth, 50.0));
        for water in [false, true] {
            let old = population::plan_budgeted_hordes_revision("r50-baseline-parity", depth, 20, 0, water, &[], 46.0);
            let new = population::plan_budgeted_hordes_revision("r50-baseline-parity", depth, 20, 0, water, &[], 50.0);
            assert_eq!(serde_json::to_value(new).unwrap(), serde_json::to_value(old).unwrap());
        }
        for setup in generation::setup_definitions() {
            assert_eq!(generation::setup_eligible_at_depth_revision(setup, depth, 46.0),
                generation::setup_eligible_at_depth_revision(setup, depth, 50.0));
        }
    }
    assert!(population::create_actor_revision("rat", 77.0, 2.0, 3.0, false, 49.0).is_err());
}

#[test]
fn revision50_internal_floor_generation_keeps_existing_layouts() {
    for depth in [1, 10, 27, 40] {
        let old = corpse_core::generation::generate_floor_candidate_revision(depth, 7, "Dungeon", false, 79, None, 0, 46.0);
        let new = corpse_core::generation::generate_floor_candidate_revision(depth, 7, "Dungeon", false, 79, None, 0, 50.0);
        assert!(old.is_ok(), "baseline candidate must exist at depth {depth}: {old:?}");
        let (old, _, _) = old.unwrap(); let (new, _, _) = new.unwrap();
        assert_eq!(serde_json::to_value(new).unwrap(), serde_json::to_value(old).unwrap(), "depth {depth}");
    }
}

#[test]
fn revision50_early_ruin_packets_keep_the_existing_forgiving_start() {
    for quality in ["infant", "child", "lesser", "unqualified", "mature", "transcendent"] {
        let a = Actor { zillennia_quality: quality.to_string().into(), ..Default::default() };
        assert_eq!(corpse_core::monster_ruin::ruin_damage_revision(&a, 50.0),
            corpse_core::monster_ruin::ruin_damage_revision(&a, 46.0));
    }
}

#[test]
fn revision50_actual_witch_and_queen_births_keep_parent_specific_profiles() {
    fn room(kind: &str, seed: u32) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 46.0.into(); r.rng = seed as f64; r.next_id = 100.0;
        r.player.x = 2.0; r.player.y = 2.0;
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.inventory.clear(); r.actors.clear();
        let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.0);
        f.environment = corpse_core::Field::Missing; f.doors = corpse_core::Field::Missing;
        let mut parent = corpse_core::population::create_actor_revision(kind, 10.0, 5.0, 5.0, false, 46.0).unwrap();
        parent.state = "hunting".into();
        r.actors_at_mut(r.depth).push(parent); r
    }
    let mut seen = std::collections::BTreeSet::new();
    for kind in ["witch", "saidribee-queen"] {
        for seed in 1..=64 {
            let mut old = room(kind, seed); let mut new = old.clone();
            new.simulation_revision = 50.0.into();
            let target = old.player.id;
            let summon: fn(&mut Run, f64, f64) -> Result<bool, String> = if kind == "witch" { corpse_core::witch::summon } else { corpse_core::saidribee_queen::summon };
            assert!(summon(&mut old, 10.0, target).unwrap());
            assert!(summon(&mut new, 10.0, target).unwrap());
            assert_eq!(new.rng, old.rng);
            assert_eq!(new.next_id, old.next_id);
            // Birth cosmetics are revisioned independently of the frozen summon profile.
            // Keep the existing parent exact and allow only the approved child overlay.
            let mut expected = old.actors_at(old.depth).to_vec();
            for child in expected.iter_mut().filter(|a| a.id == 100.0) {
                corpse_core::revision50_creature_identity::apply_actor(child);
            }
            assert_eq!(new.actors_at(new.depth), expected.as_slice());
            let child = new.actors_at(new.depth).iter().find(|a| a.id == 100.0).unwrap();
            if kind == "saidribee-queen" {
                assert_eq!(child.color, corpse_core::revision50_creature_identity::SAIDRIBEE_YELLOW);
                assert_eq!(child.glyph, if child.creature_kind.as_ref().unwrap() == "short-saidri" { "b" } else { "B" });
            }
            seen.insert((kind, child.creature_kind.as_ref().unwrap().clone()));
        }
    }
    assert_eq!(seen.len(), 4, "Exercise both child kinds for both parent species");
}
