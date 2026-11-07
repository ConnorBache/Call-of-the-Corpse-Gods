use corpse_core::{accuracy_dodge as tuning,combat,engine,population as p,startup,Field,Floor,Run};
use std::sync::Arc;
fn room()->Run {
    let mut r=startup::new_run_shell_revision("0-5-11:accuracy33","Unblessed",false,33.).unwrap();
    r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    let (w,h)=(79,29);let mut tiles=vec!["wall".into();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
    let mut f=Floor{depth:1.,width:(w as f64).into(),height:(h as f64).into(),domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r
}

#[test]
fn specialization_is_only_accuracy_dodge_and_only_new_revision33_actors() {
    assert_eq!(tuning::natural_profiles().len(),17);
    let keys:std::collections::BTreeSet<_>=tuning::natural_profiles().iter().map(|r|r.kind.as_str()).collect();
    assert_eq!(keys.len(),17);
    for (kind,base) in &p::catalog().actors {
        let old=p::create_actor_revision(kind,9.,2.,3.,false,32.).unwrap();
        let new=p::create_actor_revision(kind,9.,2.,3.,false,33.).unwrap();
        let mut normalized=new.clone();normalized.stats.accuracy=old.stats.accuracy;normalized.stats.dodge=old.stats.dodge;
        assert_eq!(normalized,old,"unrelated state changed: {kind}");
        for rev in [29.,30.,31.,32.] {
            let mut a=base.clone();let before=a.clone();tuning::apply_new_actor_kind(&mut a,kind,rev);assert_eq!(a,before);
        }
        if let Some(row)=tuning::natural_profiles().iter().find(|r|r.kind==*kind) {
            assert_eq!((new.stats.accuracy,new.stats.dodge),(row.accuracy,row.dodge));
        }else {assert_eq!(new,old,"untargeted kind {kind}");}
        assert_eq!(corpse_core::inventory::heart_for(&new,99.).unwrap(),corpse_core::inventory::heart_for(&old,99.).unwrap());
    }
    for (kind,hp,acc,dodge) in [("night-crawler",292.,144.,74.),("skin-walker",376.,148.,92.),("saidribee-queen",430.,145.,27.)] {
        let a=p::create_actor_revision(kind,9.,2.,3.,false,33.).unwrap();
        assert_eq!((a.max_hp,a.stats.accuracy,a.stats.dodge),(hp,acc,dodge));
    }
}

#[test]
fn witch_profiles_are_fixed_separate_and_identity_not_appearance_selects_stats() {
    for (kind,acc,dodge) in [("night-crawler",140.,64.),("skin-walker",144.,78.)] {
        let mut a=p::create_actor_revision(kind,9.,2.,3.,false,33.).unwrap();
        a.stats.offense=200.;a.stats.defense=24.;a.hp=244.;a.max_hp=244.;
        tuning::apply_witch_newborn(&mut a,kind,33.);
        assert_eq!((a.hp,a.max_hp,a.stats.offense,a.stats.accuracy,a.stats.dodge),(244.,244.,200.,acc,dodge));
        let stats=a.stats.clone();
        a.conditions.insert("skinWalkerDisguise".into(),3.);
        let mut r=startup::new_run_shell_revision("0-5-11:accuracy-birth","Unblessed",false,33.).unwrap();
        r.player.x=20.;r.player.y=20.;let _=corpse_core::new_creatures::disguise_kind(&r,&a);
        assert_eq!(a.stats,stats);
    }
}

#[test]
fn saved_grown_state_and_temporal_snapshots_are_not_retuned() {
    let mut r=room();
    let mut a=p::create_actor_revision("skin-walker",4.,3.,2.,false,33.).unwrap();
    a.stats.accuracy+=11.;a.stats.dodge+=7.;a.stats.offense+=13.;a.hp-=17.;
    a.conditions.insert("witchSummoned".into(),1.);r.actors_at_mut(1.).push(a.clone());r.next_id=5.;
    let raw=corpse_core::storage::serialize(&r).unwrap();let saved=corpse_core::storage::deserialize(&raw).unwrap();
    assert_eq!(engine::actor(&saved,4.).unwrap(),&a);
    let snap=corpse_core::temporal::Snapshot::capture(&r);
    r.player.conditions.insert("revertCharges".into(),2.);
    let mut history=corpse_core::temporal::History::default();history.record_completed(snap);
    r.tick+=1.;engine::actor_mut(&mut r,4.).unwrap().stats.dodge+=2.;history.closing=Some(corpse_core::temporal::Snapshot::capture(&r));
    assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());
    assert_eq!(engine::actor(&r,4.).unwrap().stats,a.stats);
    let mut fixture=r.clone();fixture.player.ability_order=Field::Missing;
    // No load-time or rendering hook is allowed to call the birth profile again.
    assert_eq!(engine::actor(&fixture,4.).unwrap().stats.dodge,99.);
}

#[test]
fn native_witch_birth_uses_newborn_not_natural_or_disguise_profile() {
    let mut kinds=std::collections::BTreeSet::new();
    for seed in 1..=24 {
        let mut r=room();r.rng=corpse_core::rng::seed_hash(&format!("accuracy-birth:{seed}")) as f64;
        let a=p::create_actor_revision("witch",4.,5.,5.,false,33.).unwrap();r.actors_at_mut(1.).push(a);r.next_id=5.;let target=r.player.id;
        assert!(corpse_core::witch::summon(&mut r,4.,target).unwrap());
        let a=engine::actor(&r,5.).unwrap();let kind=a.creature_kind.as_ref().unwrap();kinds.insert(kind.clone());
        let expected=if kind=="night-crawler"{(140.,64.)}else{(144.,78.)};
        assert_eq!((a.stats.accuracy,a.stats.dodge),expected);assert_eq!((a.hp,a.max_hp,a.stats.offense),(244.,244.,200.));
    }
    assert_eq!(kinds.len(),2);
}

#[test]
fn meaningful_counters_keep_the_global_formula_and_no_universal_invulnerability() {
    let skin=p::create_actor_revision("skin-walker",4.,3.,2.,false,33.).unwrap();
    assert!((combat::hit_chance(90.,skin.stats.dodge,0.)-0.5265).abs()<1e-9);
    assert_eq!(combat::hit_chance(130.,skin.stats.dodge,0.),1.);
    let brute=p::create_actor_revision("minotaur",5.,4.,2.,false,33.).unwrap();
    let vampire=p::create_actor_revision("vampire",8.,3.,4.,false,33.).unwrap();
    assert_eq!((vampire.stats.accuracy,vampire.stats.dodge),(158.,92.));
    assert_eq!(combat::hit_chance(brute.stats.accuracy,skin.stats.dodge,0.),1.);
    let tiger=p::create_actor_revision("tiger",6.,3.,3.,false,33.).unwrap();
    let beetle=p::create_actor_revision("giant-beetle",7.,4.,3.,false,33.).unwrap();
    assert!(combat::hit_chance(beetle.stats.accuracy,tiger.stats.dodge,0.)<0.03);
    assert_eq!(combat::hit_chance(tiger.stats.accuracy,beetle.stats.dodge,0.),1.);
    assert_eq!(combat::hit_chance(130.,skin.stats.dodge,4.),0.9f64.powf(4.));
}
