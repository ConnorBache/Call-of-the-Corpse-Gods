use corpse_core::{population, startup, temporal, storage};

#[test]
fn old_birth_stats_are_frozen_and_new_flavor_is_ordered() {
    let rat41=population::create_actor_revision("rat",5.,2.,2.,false,41.).unwrap();
    assert_eq!((rat41.stats.offense,rat41.stats.dodge),(11.,4.));
    let bat41=population::create_actor_revision("cave-bat",5.,2.,2.,false,41.).unwrap();
    assert_eq!((bat41.stats.offense,bat41.stats.dodge),(6.,7.));
    let rat=population::create_actor_revision("rat",5.,2.,2.,false,42.).unwrap();
    let bat=population::create_actor_revision("cave-bat",6.,2.,3.,false,42.).unwrap();
    assert_eq!((rat.stats.dodge,bat.stats.dodge),(-3.,0.));
    assert!(rat.stats.offense<=rat41.stats.offense && bat.stats.offense<=bat41.stats.offense);
    assert_eq!((rat.max_hp,bat.max_hp),(rat41.max_hp,bat41.max_hp));
}

#[test]
fn player_base_and_rng_do_not_change_for_balance_revision() {
    let a=startup::new_run_shell_revision("0-5-11:r42","Unblessed",false,41.).unwrap();
    let b=startup::new_run_shell_revision("0-5-11:r42","Unblessed",false,42.).unwrap();
    assert_eq!(a.player.stats.accuracy,b.player.stats.accuracy);
    assert_eq!(a.player.stats.offense,b.player.stats.offense);
    assert_eq!(a.player.stats.defense,b.player.stats.defense);
    assert_eq!(a.rng,b.rng);
}

#[test]
fn saved_legacy_actors_are_not_retuned_when_loaded() {
    let mut a=temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let revision=a.run.simulation_revision.clone();
    a.run.player.stats.accuracy=7.;
    let before=serde_json::to_value(&a.run.player).unwrap();
    let b=temporal::Engine::load(&a.save().unwrap()).unwrap();
    assert_eq!(b.run.simulation_revision,revision);
    assert_eq!(serde_json::to_value(&b.run.player).unwrap(),before);
    storage::validate(&b.run).unwrap();
}

fn birth(kind:&str,revision:f64)->corpse_core::Actor {
    if kind==corpse_core::finale::BOSS_KIND {return corpse_core::finale::administrator_revision(77.,2.,2.,revision);}
    if let Some(quality)=kind.strip_prefix("zillennia:") {
        for depth in 1..=40 {
            let actor=population::zillennia_revision(depth,revision).unwrap();
            if actor.zillennia_quality.as_ref().is_some_and(|q|q==quality){return actor;}
        }
        panic!("No Zillennia quality {quality}");
    }
    if let Ok(actor)=population::create_actor_revision(kind,77.,2.,2.,false,revision){return actor;}
    for depth in 1..=40 {
        let kinds=corpse_core::ichor::eligible_fish_kinds(depth,revision);
        if let Some(i)=kinds.iter().position(|k|*k==kind){return corpse_core::ichor::make_fish_revision(77.,2.,2.,depth as f64,(i as f64+0.5)/kinds.len()as f64,revision);}
    }
    panic!("No native birth for {kind}");
}

#[test]
fn every_natural_constructor_uses_exact_selected_fields_only() {
    for row in corpse_core::balance42::profiles() {
        if ["birther:","greater-birther:","witch:","queen:"].iter().any(|p|row.kind.starts_with(p)){continue;}
        let old=birth(&row.kind,41.);let new=birth(&row.kind,42.);
        assert_eq!(new.stats.offense,row.offense,"{} offense",row.kind);
        assert_eq!(new.stats.dodge,row.dodge,"{} dodge",row.kind);
        for (key,v)in [("monsterGasPassiveVolume",row.gas_passive),("monsterGasBoostVolume",row.gas_boost),("monsterGasShotVolume",row.gas_shot)] {
            if let Some(v)=v {assert_eq!(new.conditions.get(key),Some(&v),"{} {key}",row.kind);}
        }
        let mut a=serde_json::to_value(old).unwrap();let mut b=serde_json::to_value(new).unwrap();
        for v in [&mut a,&mut b] {
            let s=v["stats"].as_object_mut().unwrap();s.remove("offense");s.remove("dodge");
            let c=v["conditions"].as_object_mut().unwrap();for key in ["monsterGasPassiveVolume","monsterGasBoostVolume","monsterGasShotVolume"] {c.remove(key);}
        }
        assert_eq!(a,b,"{} changed an unapproved birth field",row.kind);
    }
}

#[test]
fn same_seed_generation_preserves_layout_rng_population_and_player() {
    let a=startup::create_run_revision("0-5-11:balance42-generation","Unblessed",false,41.).unwrap();
    let b=startup::create_run_revision("0-5-11:balance42-generation","Unblessed",false,42.).unwrap();
    assert_eq!(a.rng,b.rng); assert_eq!(a.next_id,b.next_id);
    assert_eq!(serde_json::to_value(&a.player).unwrap(),serde_json::to_value(&b.player).unwrap());
    let fa=a.floor().unwrap();let fb=b.floor().unwrap();
    assert_eq!(fa.tiles,fb.tiles);assert_eq!(fa.width(),fb.width());assert_eq!(fa.height(),fb.height());
    let actor_identity=|r:&corpse_core::Run|r.actors_at(r.depth).iter().map(|a|(a.id,a.creature_kind.clone(),a.x,a.y,a.abilities.clone())).collect::<Vec<_>>();
    assert_eq!(actor_identity(&a),actor_identity(&b));
}

#[test]
fn unsupported_revision_still_fails_before_generation() {
    for revision in [47.,41.5,42.5,f64::NAN] {
        assert!(startup::new_run_shell_revision("0-5-11:bad","Unblessed",false,revision).is_err());
    }
}

fn summon_room(kind:&str,revision:f64,seed:u32)->corpse_core::Run {
    use corpse_core::{Field,Floor};use std::sync::Arc;
    let mut r=startup::new_run_shell_revision("0-5-11:balance42-birth","Unblessed",false,revision).unwrap();
    r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    let(w,h)=(79,29);let mut tiles=vec!["wall".into();w*h];for y in 1..h-1 {for x in 1..w-1 {tiles[y*w+x]="floor".into();}}
    let mut f=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
    let mut a=population::create_actor_revision(kind,4.,5.,5.,false,revision).unwrap();a.state="hunting".into();
    r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![a]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=5.;r.tick=3.;r.rng=seed as f64;r.generated_ability_grants=Vec::new().into();r
}

#[test]
fn all_ten_real_child_contexts_match_final_profiles_without_rng_drift() {
    use std::collections::BTreeSet;
    let mut seen=BTreeSet::new();
    for (kind,context)in [("pale:Birther","birther"),("pale:Greater Birther","greater-birther"),("witch","witch"),("saidribee-queen","queen")] {
        for seed in 1..=64 {
            let mut old=summon_room(kind,41.,seed);let mut new=summon_room(kind,42.,seed);
            let target=old.player.id;
            assert!(corpse_core::ai::enemy_action(&mut old,4.,target).unwrap());
            assert!(corpse_core::ai::enemy_action(&mut new,4.,target).unwrap());
            assert_eq!(old.rng,new.rng);assert_eq!(old.next_id,new.next_id);
            assert_eq!(old.actors_at(1.).len(),new.actors_at(1.).len());
            for (a,b)in old.actors_at(1.).iter().zip(new.actors_at(1.)).skip(1) {
                assert_eq!((a.id,a.x,a.y,&a.creature_kind),(b.id,b.x,b.y,&b.creature_kind));
                let k=b.creature_kind.as_ref().cloned().unwrap_or_else(||format!("pale:{}",b.name.strip_prefix("Pale ").unwrap_or(&b.name)));
                let key=format!("{context}:{k}");let row=corpse_core::balance42::profiles().iter().find(|row|row.kind==key).unwrap_or_else(||panic!("Missing {key}"));
                seen.insert(key.clone());assert_eq!(b.stats.offense,row.offense,"{key}");assert_eq!(b.stats.dodge,row.dodge,"{key}");
                for (key,v)in [("monsterGasPassiveVolume",row.gas_passive),("monsterGasBoostVolume",row.gas_boost),("monsterGasShotVolume",row.gas_shot)] {
                    if let Some(v)=v {assert_eq!(b.conditions.get(key),Some(&v));}
                }
                assert_eq!((a.max_hp,a.stats.defense,a.stats.accuracy,a.stats.dr),(b.max_hp,b.stats.defense,b.stats.accuracy,b.stats.dr));
                assert_eq!(serde_json::to_value(corpse_core::inventory::heart_for(a,500.).unwrap()).unwrap(),serde_json::to_value(corpse_core::inventory::heart_for(b,500.).unwrap()).unwrap());
            }
        }
    }
    assert_eq!(seen.len(),10,"Not every actual child constructor was exercised: {seen:?}");
}

#[test]
fn revision42_seed_replay_restart_and_save_are_deterministic() {
    use corpse_core::{Command,replay::{ReplayRecord,ReplaySession}};
    let initial=startup::create_run_revision("0-5-11:balance42-replay","Unblessed",false,42.).unwrap();
    let mut engine=temporal::Engine::new(initial).unwrap();
    for command in [Command::Pickup,Command::Pickup,Command::Wait] {engine.dispatch(&command).unwrap();}
    let record=ReplayRecord::try_from_run(&engine.run).unwrap();
    let expected=engine.save().unwrap();let mut replay=ReplaySession::create(record,true).unwrap();
    while replay.step().unwrap(){} assert_eq!(replay.engine.save().unwrap(),expected);
    replay.restart().unwrap();while replay.step().unwrap(){}assert_eq!(replay.engine.save().unwrap(),expected);
    let loaded=temporal::Engine::load(&expected).unwrap();assert_eq!(loaded.run.simulation_revision.value_or(0.),42.);
    assert_eq!(loaded.save().unwrap(),expected);
}
