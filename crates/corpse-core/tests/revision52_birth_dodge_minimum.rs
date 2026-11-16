//! Birth-only minimum: independently verify raw templates, contexts and state boundaries.
use corpse_core::{balance52, combat, engine, population, startup, temporal, Actor};
use std::collections::BTreeSet;
#[path = "support/balance52_birth.rs"] mod birth;

#[test]
fn every_catalog_species_preserves_all_fields_except_the_approved_birth_changes() {
    let kinds: BTreeSet<_> = population::catalog().actors.keys().map(String::as_str)
        .chain(corpse_core::new_creatures::templates().iter().map(|a| a.creature_kind.as_ref().unwrap().as_str()))
        .collect();
    for kind in kinds {
        let old = population::create_actor_revision(kind, 900., 3., 2., false, 51.).unwrap();
        let new = population::create_actor_revision(kind, 900., 3., 2., false, 52.).unwrap();
        assert_eq!(new, birth::expected_generic_birth(&old, kind), "{kind}");
        assert!(new.stats.dodge >= 10., "{kind}");
    }
}

#[test]
fn all_species_profiles_keep_same_dodge_across_contexts_and_depths() {
    for row in balance52::profiles() {
        let expected=(row.dodge+10.).max(10.);
        for floor in 1..=40 {
            for key in [row.kind.clone(),format!("queen:{}",row.kind),format!("witch:{}",row.kind)] {
                let mut a=Actor::default();assert!(balance52::apply_new_actor_kind(&mut a,&key,floor as f64,52.));
                assert_eq!(a.stats.dodge,expected);assert!(expected>=10.);
            }
        }
    }
    assert_eq!(balance52::birth_dodge_offset("skin-walker",52.,999.),balance52::birth_dodge_offset("witch:skin-walker",52.,999.));
}

#[test]
fn unprofiled_fallback_offsets_are_floored_only_for_new_revision52_bodies() {
    for offset in [-100., -3., -0.5, 0., 0.5, 12., 92.] {
        for revision in [29., 33., 46., 50., 51., 52.] {
            let mut a = Actor::default();
            a.stats.dodge = offset;
            a.stats.accuracy = 8.;
            let mut expected = a.clone();
            if revision == 52. {
                expected.stats.dodge = (offset + 10.).max(10.);
                expected.stats.accuracy += 15.;
            }
            corpse_core::physical_balance::apply_new_actor_kind(&mut a,"unprofiled:new-body",revision);
            assert_eq!(a, expected);
        }
    }
}

#[test]
fn special_birth_templates_and_every_eligible_ichor_fish_have_the_minimum() {
    fn check(mut old: Actor, new: Actor) {
        old.stats.accuracy += 15.;
        old.stats.dodge = (old.stats.dodge + 10.).max(10.);
        assert_eq!(new, old);
        assert!(new.stats.dodge >= 10.);
    }
    let mut fish_kinds = BTreeSet::new();
    for depth in 1..=40 {
        check(population::zillennia_revision(depth,51.).unwrap(), population::zillennia_revision(depth,52.).unwrap());
        check(corpse_core::phantasmii_generation::nightmare(depth,900.,3.,2.),
            corpse_core::phantasmii_generation::nightmare_revision(depth,900.,3.,2.,52.));
        let kinds = corpse_core::ichor::eligible_fish_kinds(depth,52.);
        for index in 0..kinds.len() {
            let random = (index as f64 + 0.5) / kinds.len() as f64;
            let new = corpse_core::ichor::make_fish_revision(900.,3.,2.,depth as f64,random,52.);
            fish_kinds.insert(new.creature_kind.as_ref().unwrap().clone());
            check(corpse_core::ichor::make_fish_revision(900.,3.,2.,depth as f64,random,51.),new);
        }
    }
    assert_eq!(fish_kinds.len(),9);
    check(corpse_core::ahvamitt::actor(900.,3.,2.),corpse_core::ahvamitt::actor_revision(900.,3.,2.,52.));
    check(corpse_core::finale::administrator_revision(900.,3.,2.,51.),
        corpse_core::finale::administrator_revision(900.,3.,2.,52.));
}

#[test]
fn saved_copied_revived_and_temporally_restored_low_dodge_is_not_reborn() {
    for dodge in [-4., 0., 7., 9.] {
        let mut r = birth::scene(1,52.,1178);
        let mut a = population::create_actor_revision("rat",900.,3.,3.,false,52.).unwrap();
        a.stats.dodge = dodge;
        let stats = a.stats.clone();
        r.actors_at_mut(1.).push(a.clone());
        let e = temporal::Engine::new(r).unwrap();
        let saved = e.save().unwrap();
        let mut e = temporal::Engine::load(&saved).unwrap();
        assert_eq!(engine::actor(&e.run,900.).unwrap().stats,stats);
        let snapshot = temporal::Snapshot::capture(&e.run);
        engine::actor_mut(&mut e.run,900.).unwrap().stats.dodge = 99.;
        snapshot.restore(&mut e.run).unwrap();
        assert_eq!(engine::actor(&e.run,900.).unwrap().stats,stats);
        let mut corpse = a.clone();
        corpse.dead = true.into(); corpse.hp = 0.;
        corpse_core::corpses::revive_corpse(&mut corpse,500.);
        assert!(!corpse.dead.value_or(false));
        assert_eq!(corpse.stats,stats);
        assert_eq!(a.clone().stats,stats);
    }
}

#[test]
fn effective_penalties_and_live_stat_copies_can_remain_below_ten() {
    let mut r = birth::scene(1,52.,1178);
    let mut a = population::create_actor_revision("rat",900.,3.,3.,false,52.).unwrap();
    assert_eq!(a.stats.dodge,10.);
    for penalty in [-1.,-6.,-15.] {
        a.conditions.insert("buff:dodge".into(),penalty);
        assert_eq!(combat::effective_stats(&r,&a).dodge,10.+penalty);
        assert_eq!(a.stats.dodge,10.);
    }
    r.player.stats.dodge = 15.;
    r.player.conditions.insert("buff:dodge".into(),-11.);
    let effective = combat::effective_stats(&r,&r.player);
    assert_eq!(effective.dodge,4.);
    assert_eq!(corpse_core::mini_me::preview(&r,3.,2.).stats,effective);
    assert_eq!(corpse_core::revision50_terrain::clay_attacker(&r,
        corpse_core::revision50_terrain::Cell{x:3,y:2},901.,false).stats,effective);
    // The denominator's minimum-one guard remains separate from the birth minimum.
    assert_eq!(combat::ratio_hit_chance(1.,-5.,0.),0.5);
    assert_eq!(combat::ratio_hit_chance(1.,4.,0.),0.125);
}

#[test]
fn statless_one_hit_hallucinations_are_not_given_creature_birth_stats() {
    let mut r = birth::scene(1,52.,1178);
    r.player.abilities.push(corpse_core::revision50_nightmares::ID.into());
    r.player.nightmares = 1.0.into();
    let id = corpse_core::revision50_nightmares::create(&mut r,3.,2.).unwrap();
    let decoy = engine::actor(&r,id).unwrap();
    assert!(corpse_core::revision50_nightmares::is_decoy(decoy));
    assert_eq!(decoy.stats,corpse_core::Stats::default());
    assert_eq!(decoy.hp,1.);
}

#[test]
fn fresh_player_stays_fifteen_and_fresh_alert_targets_roll_seventy_five_percent() {
    for debug in [false,true] {
        let r = startup::new_run_shell_revision("birth-minimum","Unblessed",debug,52.).unwrap();
        assert_eq!((r.player.stats.accuracy,r.player.stats.dodge),(15.,15.));
    }
    let mut hits = 0;
    let mut misses = 0;
    for state in ["hunting","sleeping","wandering"] {
        for seed in [1,1973,8192,12345,50000,100000,123456789] {
            let mut r = birth::scene(1,52.,seed);
            let mut a = population::create_actor_revision("rat",900.,3.,2.,false,52.).unwrap();
            a.state = state.into(); a.hp = 1000.; a.max_hp = 1000.; a.base_max_hp = 1000.0.into();
            a.abilities.clear(); a.conditions.clear(); a.stats.dr = 0.;
            assert_eq!(combat::hit_chance_for_run(&r,r.player.stats.accuracy,a.stats.dodge,0.),0.75);
            r.actors_at_mut(1.).push(a);
            let mut expected = r.clone();
            let expected_hit = state != "hunting" || corpse_core::rng::random(&mut expected) < 0.75;
            engine::attack(&mut r,0.,900.,1.).unwrap();
            let hit = engine::actor(&r,900.).unwrap().hp < 1000.;
            assert_eq!(hit,expected_hit,"{state}, seed {seed}");
            if state == "hunting" { if hit { hits += 1; } else { misses += 1; } }
        }
    }
    assert!(hits > 0 && misses > 0);
}
