//! R52 contextual birth numbers preserve authored identity, traits and rewards.
#[path = "support/balance52_birth.rs"]
mod birth;
use corpse_core::{Actor, Run, balance52, engine, lifecycle, population, startup, temporal};
use std::{collections::BTreeSet, sync::Arc};

#[test]
fn each_profile_keeps_exact_authored_identity_traits_rewards_and_integral_health() {
    for p in balance52::profiles() {
        let (old, new) = birth::paired_birth(&p.kind, p.floor);
        assert_eq!(
            new,
            birth::expected_birth(&old, &p.kind, p.floor as f64),
            "{} floor {}",
            p.kind,
            p.floor
        );
        birth::assert_rewards(&old, &new);
        assert_eq!(new.max_hp, 100. + 6. * new.stats.defense);
        // Separately exercise the public applicator against an independent
        // oracle. Full Actor equality protects every unlisted field and trait.
        let mut applied = old.clone();
        // Heartlessness is an explicit constructor rule, outside numeric profiles.
        if birth::canonical_kind(&p.kind) == "pale:Jackal" { applied.heartless = true.into(); }
        assert!(balance52::apply_new_actor_kind(
            &mut applied,
            &p.kind,
            p.floor as f64,
            52.
        ));
        assert_eq!(applied, new);
    }
}

#[test]
fn all_original_fighters_keep_first_appearance_capacity_and_fixed_selected_potency() {
    let original: Vec<balance52::BirthProfile>=serde_json::from_str(include_str!("../src/catalog/balance-v52.json")).unwrap();
    for &(kind, first, last) in birth::ORIGINAL_FIGHTERS {
        let weakest=original.iter().filter(|r|r.kind==kind).min_by_key(|r|r.floor).unwrap();
        let row=balance52::profile(kind,first as f64).unwrap();
        assert_eq!(row.defense,weakest.defense);
        assert!(row.offense.is_finite() && row.offense.fract()==0.);
        assert_eq!(row.dr,if kind=="shellback-beetle" {9.} else {weakest.dr});
        for floor in first..=last {assert!(std::ptr::eq(row,balance52::profile(kind,floor as f64).unwrap()));}
    }
}

#[test]
fn direct_uncontextualized_constructor_and_summons_add_only_birth_baselines() {
    let kinds: BTreeSet<_> = balance52::profiles()
        .iter()
        .map(|p| birth::canonical_kind(&p.kind))
        .collect();
    for kind in kinds {
        let old = population::create_actor_revision(kind, 900., 3., 2., false, 51.).unwrap();
        let old = birth::expected_generic_birth(&old, kind);
        let new = population::create_actor_revision(kind, 900., 3., 2., false, 52.).unwrap();
        assert_eq!(
            old, new,
            "generic constructor unexpectedly applied floor tuning: {kind}"
        );
    }
    for (greater, kind) in [
        (false, "pale:Jackal"),
        (true, "pale:Biter"),
        (true, "pale:Breather"),
        (true, "pale:Spitter"),
        (true, "pale:Birther"),
        (true, "pale:Screamer"),
    ] {
        let mut old = Actor::default();
        let mut new = old.clone();
        corpse_core::physical_balance::apply_summoned_actor(&mut old, greater, kind, 51.);
        corpse_core::physical_balance::apply_summoned_actor(&mut new, greater, kind, 52.);
        let key = format!("{}:{kind}", if greater {"greater-birther"} else {"birther"});
        old = birth::expected_generic_birth(&old, &key);
        assert_eq!(old, new, "legacy summoned overlay {kind}");
    }
}

#[test]
fn every_profile_is_an_exact_noop_outside_revision52() {
    for row in balance52::profiles() {
        let old = population::create_actor_revision(
            birth::canonical_kind(&row.kind),
            900.,
            3.,
            2.,
            false,
            51.,
        )
        .unwrap();
        for revision in [
            29.,
            30.,
            31.,
            32.,
            33.,
            34.,
            35.,
            36.,
            37.,
            38.,
            39.,
            40.,
            41.,
            42.,
            43.,
            44.,
            45.,
            46.,
            50.,
            51.,
            52.5,
            53.,
            f64::NAN,
        ] {
            let mut actual = old.clone();
            assert!(!balance52::apply_new_actor_kind(
                &mut actual,
                &row.kind,
                row.floor as f64,
                revision
            ));
            assert_eq!(actual, old, "{} leaked into revision {revision}", row.kind);
        }
        let mut actual = old.clone();
        assert!(!balance52::apply_new_actor_kind(
            &mut actual,
            &format!("unknown:{}", row.kind),
            row.floor as f64,
            52.
        ));
        assert_eq!(actual, old);
    }
}

#[test]
fn native_birther_witch_and_queen_contexts_preserve_identity_heartlessness_and_clock() {
    for (parent, expected_roles) in [
        ("pale:Birther", vec!["birther:pale:Jackal"]),
        (
            "pale:Greater Birther",
            vec![
                "greater-birther:pale:Biter",
                "greater-birther:pale:Breather",
                "greater-birther:pale:Spitter",
                "greater-birther:pale:Birther",
                "greater-birther:pale:Screamer",
            ],
        ),
        ("witch", vec!["witch:night-crawler", "witch:skin-walker"]),
        (
            "saidribee-queen",
            vec!["queen:short-saidri", "queen:saidri-guardian"],
        ),
    ] {
        for heartless in [false, true] {
            let mut seen = BTreeSet::new();
            for seed in 0..64 {
                let (old, new) = birth::paired_summon(parent, 28, seed, heartless);
                assert_eq!(
                    engine::condition(&new.actors_at(28.)[0], "monsterSummonReadyAt"),
                    10.
                );
                for (a, b) in old.actors_at(28.)[1..].iter().zip(&new.actors_at(28.)[1..]) {
                    let key = birth::context_for_parent(parent, a);
                    assert!(expected_roles.contains(&key.as_str()));
                    seen.insert(key);
                    assert_eq!(b.heartless.value_or(false), heartless || birth::natural_kind(b) == "pale:Jackal");
                    assert_eq!(
                        b.born_from_birther.value_or(false),
                        parent.starts_with("pale:")
                    );
                    assert_eq!(
                        engine::condition(b, "witchSummoned"),
                        if parent == "witch" { 1. } else { 0. }
                    );
                    assert_eq!(
                        engine::condition(b, "queenSummoned"),
                        if parent == "saidribee-queen" { 1. } else { 0. }
                    );
                }
                if seen.len() == expected_roles.len() {
                    break;
                }
            }
            assert_eq!(seen, expected_roles.iter().map(|s| s.to_string()).collect());
        }
    }
}

#[test]
fn legacy_display_aliases_resolve_to_exact_canonical_keys() {
    for (kind, display) in [
        ("pale:Biter", "Pale Chomper"),
        ("pale:Greater Biter", "Greater Pale Chomper"),
        ("pale:Breather", "Pale Gasser"),
        ("pale:Greater Breather", "Greater Pale Gasser"),
    ] {
        let mut a = population::create_actor_revision(kind, 900., 3., 2., false, 52.).unwrap();
        assert_eq!(a.name, display);
        assert_eq!(birth::natural_kind(&a), kind);
        a.conditions.insert("nestResident".into(), 1.);
        assert_eq!(birth::generated_key(&a), format!("nest:{kind}"));
        assert!(balance52::profile(display, 28.).is_none());
    }
}

fn assert_saved_body(a: &Actor, expected: &Actor) {
    assert_eq!(a.stats, expected.stats);
    assert_eq!(a.max_hp, expected.max_hp);
    assert_eq!(a.hp, expected.hp);
    for key in [
        "monsterHealAmount",
        "monsterSpecialDamage",
        "monsterGasPassiveVolume",
        "monsterGasBoostVolume",
        "monsterGasShotVolume",
    ] {
        assert_eq!(
            a.conditions.get(key),
            expected.conditions.get(key),
            "lost saved body coefficient {key}"
        );
    }
}

#[test]
fn every_profile_keeps_saved_wounds_capacity_and_coefficients_without_retuning() {
    // Canonicalize the synthetic world schema once, before admitting any actor.
    // This is fixture setup, not normalization of the saved actor under test.
    let mut shell = birth::scene(1, 52., 1178);
    shell.floors.truncate(1);
    shell.actors.retain(|k, _| k == "1");
    shell.ground.retain(|k, _| k == "1");
    let shell = temporal::Engine::new(shell).unwrap();
    let shell = temporal::Engine::load(&shell.save().unwrap()).unwrap().run;
    for row in balance52::profiles() {
        let (_, mut a) = birth::paired_birth(&row.kind, row.floor);
        a.id = 900.;
        a.x = 3.;
        a.y = 3.;
        a.hp -= 7_f64.min(a.hp / 2.);
        let old = a.clone();
        assert_eq!(a.clone(), old);
        let mut r = shell.clone();
        r.actors_at_mut(r.depth).push(a);
        lifecycle::sync_enemy_health(&mut r, 900.).unwrap();
        assert_saved_body(engine::actor(&r, 900.).unwrap(), &old);
        let json = serde_json::to_string(&r).unwrap();
        let loaded: Run = serde_json::from_str(&json).unwrap();
        assert_saved_body(engine::actor(&loaded, 900.).unwrap(), &old);
        // The production storage path, not only raw serde, must retain it too.
        let e = temporal::Engine::new(r).unwrap();
        let saved = e.save().unwrap();
        let mut e = temporal::Engine::load(&saved).unwrap();
        assert!(
            e.save().unwrap() == saved,
            "save/load parity: {} floor {}",
            row.kind,
            row.floor
        );
        assert_saved_body(engine::actor(&e.run, 900.).unwrap(), &old);
        // Restore a real temporal snapshot after deliberately different depth/body.
        let snapshot = temporal::Snapshot::capture(&e.run);
        e.run.depth = if row.floor == 1 { 2. } else { 1. };
        Arc::make_mut(e.run.actors.get_mut("1").unwrap())[0]
            .stats
            .offense += 99.;
        snapshot.restore(&mut e.run).unwrap();
        assert_saved_body(engine::actor(&e.run, 900.).unwrap(), &old);
        let mut corpse = old.clone();
        corpse.dead = true.into();
        corpse.hp = 0.;
        if !corpse_core::corpses::can_revive_corpse(&corpse) {
            let before = corpse.clone();
            corpse_core::corpses::revive_corpse(&mut corpse, 500.);
            assert_eq!(corpse, before, "non-revivable authored body changed");
            continue;
        }
        corpse_core::corpses::revive_corpse(&mut corpse, 500.);
        let mut expected = old.clone();
        // Existing gas conditions were historically curable. No selected row
        // tunes them; preserve this old revival behavior until a revision-aware
        // path is deliberately introduced (the catalog admission test enforces it).
        for key in [
            "monsterGasPassiveVolume",
            "monsterGasBoostVolume",
            "monsterGasShotVolume",
        ] {
            expected.conditions.shift_remove(key);
        }
        expected.hp = if old.abilities.iter().any(|id| id == "enemy:beast:phoenix") {
            old.max_hp
        } else {
            (old.max_hp / 2.).ceil()
        };
        assert_saved_body(&corpse, &expected);
    }
}

#[test]
fn gas_tuning_is_not_admitted_until_revision_aware_revival_preserves_it() {
    // Legacy revival intentionally clears these pre-existing condition keys.
    // The selected all-role rows do not tune gas. Adding a row is a hard review
    // gate, not permission to preserve gas globally and change revision <= 51.
    for row in balance52::profiles() {
        assert!(
            row.gas_passive.is_none() && row.gas_boost.is_none() && row.gas_shot.is_none(),
            "{} floor {} needs revision-aware gas persistence before admission",
            row.kind,
            row.floor
        );
    }
    for kind in ["pale:Breather", "pale:Greater Breather"] {
        let old = population::create_actor_revision(kind, 900., 3., 2., false, 51.).unwrap();
        let new = population::create_actor_revision(kind, 900., 3., 2., false, 52.).unwrap();
        for key in [
            "monsterGasPassiveVolume",
            "monsterGasBoostVolume",
            "monsterGasShotVolume",
        ] {
            assert_eq!(old.conditions.get(key), new.conditions.get(key));
        }
    }
}

#[test]
fn health_sync_preserves_tuned_capacity_and_existing_wounds() {
    let mut r =
        startup::new_run_shell_revision("0-5-11:r52-health", "Unblessed", false, 52.).unwrap();
    let mut a = population::create_actor_revision("rat", 900., 3., 2., false, 52.).unwrap();
    balance52::apply_new_actor_kind(&mut a, "rat", 2., 52.);
    a.hp -= 7.;
    let hp = a.hp;
    let max = a.max_hp;
    r.actors.insert("1".into(), Arc::new(vec![a]));
    lifecycle::sync_enemy_health(&mut r, 900.).unwrap();
    let a = &r.actors_at(1.)[0];
    assert_eq!(a.hp, hp);
    assert_eq!(a.max_hp, max);
    let json = serde_json::to_string(&r).unwrap();
    let loaded: Run = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.actors_at(1.)[0].hp, hp);
    assert_eq!(loaded.actors_at(1.)[0].max_hp, max);
}

#[test]
fn greater_birthers_child_birther_uses_ordinary_birther_context_for_grandchildren() {
    for seed in 0..64 {
        let (old, new) = birth::paired_summon("pale:Greater Birther", 28, seed, false);
        let Some(i) = old
            .actors_at(28.)
            .iter()
            .position(|a| a.id != 900. && birth::natural_kind(a) == "pale:Birther")
        else {
            continue;
        };
        let mut children = vec![];
        for (revision, a) in [(51., &old.actors_at(28.)[i]), (52., &new.actors_at(28.)[i])] {
            let mut r = birth::scene(28, revision, 73);
            let mut parent = a.clone();
            parent.id = 900.;
            parent.x = 4.;
            parent.y = 4.;
            parent.state = "hunting".into();
            r.actors_at_mut(28.).push(parent);
            let target = r.player.id;
            assert!(corpse_core::ai::enemy_action(&mut r, 900., target).unwrap());
            assert!(r.actors_at(28.).len() >= 2);
            children.push(r.actors_at(28.)[1..].to_vec());
        }
        assert_eq!(children[0].len(), children[1].len());
        for (old, new) in children[0].iter().zip(&children[1]) {
            assert_eq!(birth::natural_kind(old), "pale:Jackal");
            assert_eq!(*new, birth::expected_birth(old, "birther:pale:Jackal", 28.));
            birth::assert_rewards(old, new);
        }
        return;
    }
    panic!("No native child Birther in deterministic seed range");
}

#[test]
fn native_floor_travel_and_future_return_preserve_saved_context_bodies() {
    for key in [
        "rat",
        "dryad",
        "moonspawn",
        "birther:pale:Jackal",
        "greater-birther:pale:Biter",
        "witch:night-crawler",
        "queen:saidri-guardian",
        "nest:saidribee-queen",
    ] {
        let (_, mut a) = birth::paired_birth(key, 28);
        a.id = 900.;
        a.x = 60.;
        a.y = 20.;
        a.state = "sleeping".into();
        // Explicit values exercise persistence even before new catalog rows land.
        a.conditions.insert("monsterHealAmount".into(), 0.);
        a.conditions.insert("monsterSpecialDamage".into(), 600.);
        a.hp -= 7.;
        let before = a.clone();
        let mut r = birth::scene(1, 52., 1178);
        r.actors_at_mut(1.).push(a);
        // Travel really executes native arrival and offscreen paths. No time is
        // advanced, so this isolates admission rather than healing/decay rules.
        assert!(corpse_core::travel::travel(&mut r, 1., false, false).unwrap());
        assert_eq!(r.depth, 2.);
        assert_saved_body(&r.actors_at(1.)[0], &before);
        assert!(corpse_core::travel::travel(&mut r, -1., false, false).unwrap());
        assert_eq!(r.depth, 1.);
        assert_saved_body(engine::actor(&r, 900.).unwrap(), &before);
        let id = corpse_core::revision50_ability_energy::FUTURE_ID;
        r.player.abilities.push(id.into());
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player, id, 50., 50.)
            .unwrap();
        let mut history = temporal::History::default();
        temporal::remember_future(&mut r, &history).unwrap();
        r.depth = 2.;
        Arc::make_mut(r.actors.get_mut("1").unwrap())[0]
            .stats
            .offense += 77.;
        let saved = corpse_core::storage::serialize(&r).unwrap();
        let mut r = corpse_core::storage::deserialize(&saved).unwrap();
        assert!(temporal::restore_future(&mut r, &mut history).unwrap());
        assert_eq!(r.depth, 1.);
        assert_saved_body(engine::actor(&r, 900.).unwrap(), &before);
    }
}

#[test]
fn saved_local_coefficients_are_rejected_by_legacy_engine_and_storage_admission() {
    for key in ["monsterHealAmount", "monsterSpecialDamage"] {
        for (revision, value, valid) in [
            (51., 0., false),
            (51., 600., false),
            (52., 0., true),
            (52., 600., true),
            (52., -1., false),
            (52., 0.5, key == "monsterSpecialDamage"),
        ] {
            let mut r = birth::scene(1, revision, 73);
            let mut a =
                population::create_actor_revision("dryad", 900., 3., 3., false, revision).unwrap();
            a.conditions.insert(key.into(), value);
            r.actors_at_mut(1.).push(a);
            let before = r.clone();
            assert_eq!(
                engine::validate_supported(&r).is_ok(),
                valid,
                "{key} {revision} {value}"
            );
            let saved = corpse_core::storage::serialize(&r).unwrap();
            assert_eq!(
                corpse_core::storage::deserialize(&saved).is_ok(),
                valid,
                "storage {key} {revision} {value}"
            );
            assert_eq!(r, before);
        }
    }
}

#[test]
fn revision52_timeweave_and_future_commands_do_not_retune_existing_body() {
    let (_, mut a) = birth::paired_birth("rat", 3);
    a.id = 900.;
    a.x = 60.;
    a.y = 20.;
    a.dead = true.into();
    a.hp = 0.;
    a.corpse_age = 0.0.into();
    // Persisted modifications need not equal any current catalog tier.
    a.stats.offense += 123.;
    a.conditions.insert("monsterHealAmount".into(), 73.);
    a.conditions.insert("monsterSpecialDamage".into(), 600.);
    let expected = a.clone();
    let mut r = birth::scene(1, 52., 73);
    r.actors_at_mut(1.).push(a);
    for id in [
        corpse_core::progression::WEAVE_ID,
        corpse_core::revision50_ability_energy::FUTURE_ID,
    ] {
        r.player.abilities.push(id.into());
    }
    corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
    let mut e = temporal::Engine::new(r).unwrap();
    let forward = corpse_core::Command::Move { dx: 1., dy: 0. };
    let back = corpse_core::Command::Move { dx: -1., dy: 0. };
    assert!(e.dispatch(&forward).unwrap().took_turn);
    let tick = e.run.tick;
    assert!(!e.dispatch(&back).unwrap().took_turn);
    assert_eq!(e.run.tick, tick);
    assert_saved_body(engine::actor(&e.run, 900.).unwrap(), &expected);
    let future = corpse_core::Command::Ability {
        id: corpse_core::revision50_ability_energy::FUTURE_ID.into(),
        target: corpse_core::Field::Missing,
        x: corpse_core::Field::Missing,
        y: corpse_core::Field::Missing,
    };
    e.dispatch(&future).unwrap();
    assert!(e.run.future.is_some());
    let before = engine::actor(&e.run, 900.).unwrap().clone();
    e.dispatch(&forward).unwrap();
    let mut e = temporal::Engine::load(&e.save().unwrap()).unwrap();
    e.dispatch(&future).unwrap();
    assert!(e.run.future.is_none());
    assert_eq!(*engine::actor(&e.run, 900.).unwrap(), before);
    assert_saved_body(engine::actor(&e.run, 900.).unwrap(), &expected);
}

#[test]
fn r52_special_and_unprofiled_births_add_only_accuracy_and_dodge_baselines() {
    fn only_birth_baselines(mut old: Actor, new: Actor) {
        old.stats.accuracy += 15.;
        old.stats.dodge = (old.stats.dodge + 10.).max(10.);
        assert_eq!(old, new);
    }
    for depth in 1..=40 {
        only_birth_baselines(population::zillennia_revision(depth, 51.).unwrap(),
            population::zillennia_revision(depth, 52.).unwrap());
        only_birth_baselines(corpse_core::phantasmii_generation::nightmare(depth, 900., 3., 2.),
            corpse_core::phantasmii_generation::nightmare_revision(depth, 900., 3., 2., 52.));
        assert_eq!(corpse_core::phantasmii_generation::nightmare(depth, 900., 3., 2.),
            corpse_core::phantasmii_generation::nightmare_revision(depth, 900., 3., 2., 51.));
        for random in [0., 0.25, 0.5, 0.75, 0.999] {
            only_birth_baselines(corpse_core::ichor::make_fish_revision(900.,3.,2.,depth as f64,random,51.),
                corpse_core::ichor::make_fish_revision(900.,3.,2.,depth as f64,random,52.));
        }
        let mut floor=corpse_core::Floor {depth:depth as f64,width:5.0.into(),height:5.0.into(),
            tiles:vec!["floor".into();25],..Default::default()};
        for kind in ["fungus", "vent-fungus", "bastidion"] {
            let make=|f:&mut corpse_core::Floor,revision| {
                let mut id=900.; let mut rng=corpse_core::rng::GenerationRng::new(1);
                population::create_population_actor_revision(&mut id,f,kind,12,false,&mut rng,revision).unwrap()
            };
            let new=make(&mut floor,52.); floor.depth=1.; let old=make(&mut floor,51.); floor.depth=depth as f64;
            only_birth_baselines(old,new);
        }
    }
    only_birth_baselines(corpse_core::finale::administrator_revision(900.,3.,2.,51.),
        corpse_core::finale::administrator_revision(900.,3.,2.,52.));
    only_birth_baselines(corpse_core::ahvamitt::actor(900.,3.,2.),
        corpse_core::ahvamitt::actor_revision(900.,3.,2.,52.));
    assert_eq!(corpse_core::ahvamitt::actor(900.,3.,2.),
        corpse_core::ahvamitt::actor_revision(900.,3.,2.,51.));
}

#[test]
fn r52_context_assignment_replaces_baselines_and_copy_load_keep_recorded_stats() {
    for p in balance52::profiles() {
        let mut a=population::create_actor_revision(birth::canonical_kind(&p.kind),900.,3.,2.,false,52.).unwrap();
        for _ in 0..2 {
            assert!(balance52::apply_new_actor_kind(&mut a,&p.kind,p.floor as f64,52.));
            assert_eq!(a.stats.accuracy,p.accuracy+15.,"{} floor{}",p.kind,p.floor);
            assert_eq!(a.stats.dodge,(birth::first_dodge_offset(&p.kind,p.dodge)+10.).max(10.),"{} floor{}",p.kind,p.floor);
        }
        // Wounds, growth and accumulated modifiers are recorded stats, not births.
        a.stats.accuracy += 9.;
        a.stats.dodge += 7.;
        let copied=a.clone();
        let loaded:Actor=serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(copied,a); assert_eq!(loaded,a);
    }
    let mut r=birth::scene(1,52.,1);
    r.player.stats.accuracy=37.; r.player.stats.dodge=27.; r.inventory.clear(); r.player.conditions.clear();
    let effective=corpse_core::combat::effective_stats(&r,&r.player);
    assert_eq!(corpse_core::mini_me::preview(&r,3.,2.).stats,effective);
    assert_eq!(corpse_core::revision50_terrain::clay_attacker(&r,
        corpse_core::revision50_terrain::Cell{x:3,y:2},900.,false).stats,effective);
}
