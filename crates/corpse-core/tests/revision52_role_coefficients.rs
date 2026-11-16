//! Saved birth coefficients are local numbers, never runtime floor scaling.
use corpse_core::{balance52, engine, lifecycle, population, startup, Actor, Run};
use std::sync::Arc;

#[test]
fn coefficient_defaults_are_exact_and_only_revision52_reads_saved_values() {
    let mut a=Actor::default();
    for revision in [29.,33.,42.,50.,51.,52.] {
        let mut r=startup::new_run_shell_revision("0-5-11:r52-coeff", "Unblessed", false, revision).unwrap();
        assert_eq!(balance52::local_amount(&r,&a,"monsterHealAmount",25.),25.);
        a.conditions.insert("monsterHealAmount".into(),69.);
        a.conditions.insert("monsterSpecialDamage".into(),600.);
        assert_eq!(balance52::local_amount(&r,&a,"monsterHealAmount",25.),if revision==52.{69.}else{25.});
        for depth in [1.,12.,40.] {r.depth=depth;assert_eq!(balance52::local_amount(&r,&a,"monsterSpecialDamage",120.),if revision==52.{600.}else{120.});}
        a.conditions.clear();
    }
}
#[test]
fn malformed_local_coefficients_are_rejected_not_silently_admitted() {
    for key in ["monsterHealAmount","monsterSpecialDamage"] {
        assert!(balance52::condition_key(key));
        for n in [-1.,0.25,f64::NAN,f64::INFINITY] {assert!(!balance52::valid_condition(key,n));}
        assert_eq!(balance52::valid_condition(key,0.5),key=="monsterSpecialDamage");
        for n in [0.,25.,600.] {assert!(balance52::valid_condition(key,n));}
    }
    assert!(!balance52::condition_key("monsterArbitraryDamage"));
}
#[test]
fn coefficient_clone_save_and_revive_do_not_retune() {
    let mut r=startup::new_run_shell_revision("0-5-11:r52-saved-coeff", "Unblessed", false,52.).unwrap();
    let mut a=population::create_actor_revision("dryad",900.,3.,2.,false,52.).unwrap();
    a.conditions.insert("monsterHealAmount".into(),69.);
    a.conditions.insert("monsterSpecialDamage".into(),600.);
    let before=a.conditions.clone();
    assert_eq!(a.clone().conditions,before);
    a.dead=true.into();a.hp=0.;corpse_core::corpses::revive_corpse(&mut a,99.);
    assert_eq!(a.conditions.get("monsterHealAmount"),Some(&69.));
    assert_eq!(a.conditions.get("monsterSpecialDamage"),Some(&600.));
    r.actors.insert("1".into(),Arc::new(vec![a]));
    lifecycle::sync_enemy_health(&mut r,900.).unwrap();
    let json=serde_json::to_string(&r).unwrap();let loaded:Run=serde_json::from_str(&json).unwrap();
    let a=engine::actor(&loaded,900.).unwrap();
    assert_eq!(a.conditions.get("monsterHealAmount"),Some(&69.));
    assert_eq!(a.conditions.get("monsterSpecialDamage"),Some(&600.));
}
