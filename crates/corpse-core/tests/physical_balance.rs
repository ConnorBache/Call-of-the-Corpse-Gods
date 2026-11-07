use corpse_core::{Command, Field, Floor, Run, combat, engine, physical_balance as b, population as p, startup};
use std::sync::Arc;

#[test]
fn reference_depth_is_independent_of_spawn_window() {
    for kind in ["blorb", "rat", "cave-bat", "shellback-beetle", "hemogoblin", "needleling", "pale:Jackal"] {
        assert_eq!(b::reference_depth(kind), Some(1.0));
    }
    assert_eq!(b::reference_depth("bastidion"), Some(1.0));
    assert_eq!(b::reference_depth("glim"), Some(11.5));
    assert_eq!(b::reference_depth("minotaur"), Some(39.5));
    assert_eq!(b::reference_depth("ursa-major"), Some(39.0));
    assert_eq!(b::reference_depth("not-a-creature"), None);
}

#[test]
fn revision_overlay_preserves_abilities_rewards_and_frozen_catalog() {
    assert_eq!(b::tuning().len(), 48);
    let kinds: std::collections::BTreeSet<_> = b::tuning().iter().map(|t| t.kind.as_str()).collect();
    assert_eq!(kinds.len(), b::tuning().len(), "duplicate numeric overlay rows");
    let before = serde_json::to_value(&p::catalog().actors).unwrap();
    for tuning in b::tuning() {
        let old = p::create_actor_revision(&tuning.kind, 7., 2., 3., true, 30.).unwrap();
        let new = p::create_actor_revision(&tuning.kind, 7., 2., 3., true, 31.).unwrap();
        assert_eq!(old, p::create_actor(&tuning.kind, 7., 2., 3., true).unwrap());
        assert!((1.0..=40.0).contains(&tuning.reference_depth));
        assert!(new.max_hp > 0.0);
        assert_eq!(new.stats.defense, tuning.defense);
        assert_eq!(new.stats.offense, tuning.offense);
        assert_eq!(new.stats.accuracy, tuning.accuracy);
        assert_eq!(new.stats.dodge, tuning.dodge);
        assert_eq!(new.stats.dr, tuning.dr);
        assert_eq!(new.max_hp, 100. + 6. * tuning.defense);
        assert_eq!(new.hp, new.max_hp);
        assert_eq!(new.abilities, old.abilities);
        assert_eq!(new.conditions, old.conditions);
        assert_eq!(new.state, old.state);
        assert_eq!(corpse_core::inventory::heart_for(&new, 9.).unwrap(), corpse_core::inventory::heart_for(&old, 9.).unwrap());
        assert_eq!(b::reference_depth(&tuning.kind), Some(tuning.reference_depth));
    }
    assert_eq!(serde_json::to_value(&p::catalog().actors).unwrap(), before);
    assert!(p::create_actor_revision("rat", 0., 0., 0., false, 47.).is_err());
    assert!(p::create_actor_revision("rat", 0., 0., 0., false, 37.).is_ok());
    assert!(p::create_actor_revision("rat", 0., 0., 0., false, 38.).is_ok());
    assert!(p::create_actor_revision("rat", 0., 0., 0., false, 39.).is_ok());
    assert!(p::create_actor_revision("rat", 0., 0., 0., false, 40.).is_ok());
    let rat = p::create_actor_revision("rat", 4., 0., 0., false, 31.).unwrap();
    assert_eq!((rat.stats.offense, rat.max_hp, rat.stats.dodge), (11., 22., 3.));
    let shell = p::create_actor_revision("shellback-beetle", 4., 0., 0., false, 31.).unwrap();
    assert!(combat::ordinary_strike_damage(30., 0.75, shell.stats.dr, false, false) > 0.);
    assert_eq!((shell.max_hp, shell.stats.dr), (46., 14.));
}

#[test]
fn revised_stat_scale_does_not_rebudget_population() {
    for depth in 1..=40 {
        for water in [false, true] {
            let budget = corpse_core::generation::ordinary_blessing_budget(corpse_core::generation::BudgetSeed::Text("physical-policy"), depth).unwrap() as i32;
            let old = p::plan_budgeted_hordes_revision("physical-policy", depth, budget, 0, water, &[], 30.).unwrap();
            let new = p::plan_budgeted_hordes_revision("physical-policy", depth, budget, 0, water, &[], 31.).unwrap();
            assert_eq!(serde_json::to_value(old).unwrap(), serde_json::to_value(new).unwrap());
        }
    }
}

fn duel_scene(kind: &str, seed: u32) -> Run {
    let mut r = startup::new_run_shell_revision("0-5-11:physical-duel", "Unblessed", false, 31.).unwrap();
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.player.ability_order = Field::Missing;
    r.player.x = 2.; r.player.y = 2.;
    r.rng = seed as f64;
    let w = 7;
    let mut tiles = vec!["wall".to_string(); w*w];
    for y in 1..w-1 { for x in 1..w-1 { tiles[y*w+x] = "floor".into(); } }
    r.floors = vec![Arc::new(Floor {
        width: (w as f64).into(), height: (w as f64).into(), depth: 1., domain: "Dungeon".into(),
        tiles, lights: vec![100.; w*w], seen: vec![true; w*w], ..Default::default()
    })];
    let mut a = p::create_actor_revision(kind, 4., 3., 2., false, 31.).unwrap();
    a.state = "hunting".into();
    a.last_seen = corpse_core::ActorLastSeen { x: 2., y: 2. }.into();
    r.actors.clear(); r.actors.insert("1".into(), Arc::new(vec![a]));
    r.ground.clear(); r.ground.insert("1".into(), Arc::new(vec![]));
    r.next_id = 5.;
    r
}

#[test]
fn native_dispatch_rat_whole_encounter_and_fast_cadence() {
    // Executed native commands, not the Python fit. Fixed seeds and plain starter build.
    // Small room still runs validation, normal turns, automatic regen and death handling.
    let mut total = 0.;
    let trials = 1000;
    for n in 0..trials {
        let seed = corpse_core::rng::seed_hash(&format!("rat-native31:{n}"));
        let mut r = duel_scene("rat", seed);
        for _ in 0..100 {
            if r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false) { break; }
            engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
        }
        assert!(r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false));
        total += 1. - r.player.hp.max(0.) / 100.;
    }
    let mean = total / trials as f64;
    eprintln!("native1000 Rat mean whole-fight cost={mean}");
    assert!((0.18..0.23).contains(&mean), "whole-fight mean={mean}");

    for (kind, low, high) in [("shellback-beetle", 0.18, 0.22), ("cave-bat", 0.14, 0.20)] {
        let mut sum = 0.;
        for n in 0..1000 {
            let seed = corpse_core::rng::seed_hash(&format!("{kind}-native31:{n}"));
            let mut r = duel_scene(kind, seed);
            for _ in 0..100 {
                if r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false) { break; }
                engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
            }
            assert!(r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false));
            sum += 1. - r.player.hp.max(0.) / 100.;
        }
        let mean = sum / 1000.;
        eprintln!("native1000 {kind} mean whole-fight cost={mean}");
        assert!((low..high).contains(&mean), "{kind} mean={mean}");
    }

    // Only the Fast flag differs: already-adjacent action sequences must match exactly.
    let mut fast = duel_scene("rat", 1178);
    fast.actors_at_mut(1.)[0].abilities.push("enemy:beast:fast".into());
    let mut normal = duel_scene("rat", 1178);
    for _ in 0..4 {
        if normal.player.hp <= 0. || normal.actors_at(1.)[0].dead.value_or(false) { break; }
        engine::dispatch(&mut fast, &Command::Move { dx: 1., dy: 0. }).unwrap();
        engine::dispatch(&mut normal, &Command::Move { dx: 1., dy: 0. }).unwrap();
        assert_eq!(fast.player.hp, normal.player.hp);
        assert_eq!(fast.actors_at(1.)[0].hp, normal.actors_at(1.)[0].hp);
        assert_eq!(fast.rng, normal.rng);
    }
}

#[test]
fn native_late_profile_trades_and_cancer_ignore_armor() {
    // Three recorded joint profiles, not averages manufactured by mixing independent stats.
    // These are controlled adjacent native command fights, not roaming/full-floor playtests.
    let rows: serde_json::Value = serde_json::from_str(include_str!("../../../docs/artifacts/physical-balance-v31-native-smoke-profiles.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let kind = row["kind"].as_str().unwrap();
        let stats = row["stats"].as_array().unwrap();
        let value = |i: usize| stats[i].as_f64().unwrap();
        let mut total = 0.;
        for n in 0..256 {
            let seed = corpse_core::rng::seed_hash(&format!("{kind}-late-native31:{n}"));
            let mut r = duel_scene(kind, seed);
            r.player.stats.offense = value(0);
            r.player.stats.defense = value(1);
            r.player.stats.accuracy = value(2);
            r.player.stats.dodge = value(3);
            r.player.stats.dr = value(4);
            r.player.hp = value(5); r.player.max_hp = value(5);
            let mut armor_control = r.clone();
            armor_control.player.stats.dr = 1000.;
            for _ in 0..100 {
                if r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false) { break; }
                engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
                if kind == "cancer" {
                    engine::dispatch(&mut armor_control, &Command::Move { dx: 1., dy: 0. }).unwrap();
                    assert_eq!(r.player.hp, armor_control.player.hp, "Cancer must bypass DR");
                    assert_eq!(r.rng, armor_control.rng);
                }
            }
            assert!(r.player.hp <= 0. || r.actors_at(1.)[0].dead.value_or(false));
            total += 1. - r.player.hp.max(0.) / value(5);
        }
        let mean = total / 256.;
        let expected = row["modeled_mean"].as_f64().unwrap();
        eprintln!("native256 {kind} recorded-profile cost={mean}, Python reference={expected}");
        assert!((mean - expected).abs() < 0.04, "{kind} native={mean}, reference={expected}");
    }
}

#[test]
fn pale_identity_and_ichor_event_construction_receive_tuning() {
    let pale = p::create_actor_revision("pale:Biter", 8., 3., 3., false, 31.).unwrap();
    assert!(pale.creature_kind.is_missing(), "Do not rewrite legacy identity to apply tuning");
    assert_eq!(pale.stats.offense, b::tuning().iter().find(|t| t.kind=="pale:Biter").unwrap().offense);
    for row in b::tuning().iter().filter(|t| t.kind.contains("ichor") && t.kind!="ichor-fish") {
        let (depth,index,total) = (1..=40).find_map(|depth| {
            let kinds=corpse_core::ichor::eligible_fish_kinds(depth,31.);
            kinds.iter().position(|k| *k==row.kind).map(|i|(depth,i,kinds.len()))
        }).unwrap();
        let fish=corpse_core::ichor::make_fish_revision(9.,3.,3.,depth as f64,(index as f64+0.5)/total as f64,31.);
        assert_eq!(fish.creature_kind.as_ref().unwrap(), &row.kind);
        assert_eq!(fish.stats.offense,row.offense);
        assert_eq!(fish.stats.accuracy,row.accuracy);
        assert_eq!(fish.stats.dr,row.dr);
        assert_eq!(fish.max_hp,100.+6.*row.defense);
        assert_eq!(b::reference_depth(&row.kind),Some(row.reference_depth));
        let old = p::create_actor(&row.kind,9.,3.,3.,false).unwrap();
        assert_eq!(corpse_core::inventory::heart_for(&fish,10.).unwrap(),corpse_core::inventory::heart_for(&old,10.).unwrap());
    }
    for revision in [29.,30.,31.] {
        let mut r=duel_scene("rat",corpse_core::rng::seed_hash("native-ichor-event-overlay"));
        r.simulation_revision=revision.into(); r.depth=40.;
        r.actors.clear();r.actors.insert("40".into(),Arc::new(vec![]));
        r.ground.clear();r.ground.insert("40".into(),Arc::new(vec![]));
        let f=Arc::make_mut(&mut r.floors[0]); f.depth=40.;
        f.ichor=vec![100.;49].into();
        f.godbeast_event=corpse_core::GodbeastEvent {
            kind:"Mercuria".into(),x:3.,y:3.,activated:true.into(),flood_age:8.0.into(),
            next_fish_age:8.0.into(),fish_spawned:0.0.into(),..Default::default()
        }.into();
        let mut expected=r.clone();let random=corpse_core::rng::random(&mut expected);
        let fish=corpse_core::ichor::make_fish_revision(r.next_id,3.,3.,40.,random,revision);
        corpse_core::ichor::tick_fish(&mut r).unwrap();
        assert_eq!(r.actors_at(40.),&[fish]);
        assert_eq!(r.rng,expected.rng);
        assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().fish_spawned.as_ref(),Some(&1.));
    }
}
