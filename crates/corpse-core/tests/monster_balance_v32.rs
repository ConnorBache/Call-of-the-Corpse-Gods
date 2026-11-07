use corpse_core::{Actor, physical_balance as balance, population};

fn born(kind: &str, revision: f64) -> Actor {
    if kind == corpse_core::finale::BOSS_KIND {
        return corpse_core::finale::administrator_revision(7.,2.,3.,revision);
    }
    if let Some(quality) = kind.strip_prefix("zillennia:") {
        let depth = population::catalog().zillennia.iter()
            .position(|a| a.zillennia_quality.as_ref().is_some_and(|q|q==quality)).unwrap()+1;
        return population::zillennia_revision(depth as u32,revision).unwrap();
    }
    population::create_actor_revision(kind,7.,2.,3.,false,revision).unwrap()
}

#[test]
fn revision32_overlay_is_finite_birth_only_and_preserves_rewards_and_traits() {
    let mut seen=std::collections::BTreeSet::new();
    for t in balance::tuning_revision(32.) {
        assert!(seen.insert(&t.kind),"duplicate {}",t.kind);
        assert!((1. ..=40.).contains(&t.reference_depth));
        for n in [t.offense,t.defense,t.accuracy,t.dodge,t.dr] {assert!(n.is_finite());}
        let a=born(&t.kind,32.);let old=born(&t.kind,31.);
        assert!(a.max_hp>0.);assert_eq!(a.max_hp,100.+6.*t.defense);
        assert_eq!(a.stats.offense,t.offense);assert_eq!(a.stats.accuracy,t.accuracy);
        assert_eq!(a.abilities,old.abilities);assert_eq!(a.heartless,old.heartless);
        assert_eq!(a.stats.max_energy,old.stats.max_energy);
        assert_eq!(a.stats.perception,old.stats.perception);
        if t.kind!=corpse_core::finale::BOSS_KIND {
            assert_eq!(corpse_core::inventory::heart_for(&a,88.).unwrap(),corpse_core::inventory::heart_for(&old,88.).unwrap());
        }
        let saved=serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Actor>(&saved).unwrap(),a);
        for key in ["monsterGasPassiveVolume","monsterGasBoostVolume","monsterGasShotVolume"] {
            assert!(!old.conditions.contains_key(key));
            if let Some(v)=a.conditions.get(key) {assert!(v.is_finite()&&*v>0.);}
        }
    }
    assert!(seen.len()>=70,"expected the full numeric roster, not a physical-only subset");
}

#[test]
fn fixed_brood_profiles_do_not_retune_natural_or_loaded_creatures() {
    let natural=population::create_actor_revision("pale:Jackal",8.,2.,2.,false,32.).unwrap();
    let mut child=natural.clone();
    balance::apply_summoned_actor(&mut child,false,"pale:Jackal",32.);
    assert_eq!(child.abilities,natural.abilities);
    assert_eq!(child.stats.max_energy,natural.stats.max_energy);
    assert_eq!(child.stats.perception,natural.stats.perception);
    assert_eq!(corpse_core::inventory::heart_for(&child,99.).unwrap(),corpse_core::inventory::heart_for(&natural,99.).unwrap());
    assert_ne!(child.stats,natural.stats);
    let saved=serde_json::to_string(&child).unwrap();
    assert_eq!(serde_json::from_str::<Actor>(&saved).unwrap(),child);
    let mut legacy=natural.clone();
    balance::apply_summoned_actor(&mut legacy,false,"pale:Jackal",31.);
    assert_eq!(legacy,natural);
}
