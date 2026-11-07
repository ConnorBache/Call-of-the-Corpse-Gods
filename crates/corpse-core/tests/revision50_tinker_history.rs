//! History-only identity mapping; no saved-power migration or release admission.
use corpse_core::{Command, Field, Item, Run, inventory, progression, revision50_ability_energy, rng, tinker};
use serde_json::Value;
use std::collections::HashSet;

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.gods.clear(); r.inventory.clear(); r.ground.clear();
    r.generated_ability_grants = vec![].into();
    r
}
fn pools(r: &Run) -> Vec<Vec<String>> {
    tinker::god_pools(r).into_iter().map(|pool|pool.into_iter().map(|a|a.id.clone()).collect()).collect()
}
fn identities() -> Vec<Value> {
    serde_json::from_str(include_str!("../catalog/ability-identities-v50.json")).unwrap()
}

#[test]
fn every_explicit_replacement_is_excluded_by_historical_identity_without_mutation() {
    for row in identities().into_iter().filter(|row|row["legacyId"]!=row["revision50Id"]) {
        let old=row["legacyId"].as_str().unwrap();let current=row["revision50Id"].as_str().unwrap();
        let mut r=room(50.);r.generated_ability_grants=vec![old.to_string()].into();
        let before=serde_json::to_vec(&r).unwrap();let available=pools(&r);
        assert_eq!(available.iter().map(Vec::len).sum::<usize>(),71,"{old}");
        assert!(available.iter().flatten().all(|id|id!=current),"{old} must exclude {current}");
        assert_eq!(serde_json::to_vec(&r).unwrap(),before,"query must not migrate {old}");
    }
}

#[test]
fn old_new_and_duplicate_provenance_have_the_same_ordered_candidates() {
    let rows=identities();
    let mut old=room(50.);let mut current=room(50.);let mut both=room(50.);
    let selected:Vec<_>=rows.iter().filter(|row|row["legacyId"]!=row["revision50Id"]).collect();
    old.generated_ability_grants=selected.iter().map(|row|row["legacyId"].as_str().unwrap().to_string()).collect::<Vec<_>>().into();
    current.generated_ability_grants=selected.iter().map(|row|row["revision50Id"].as_str().unwrap().to_string()).collect::<Vec<_>>().into();
    both.generated_ability_grants=old.generated_ability_grants.as_ref().unwrap().iter()
        .chain(current.generated_ability_grants.as_ref().unwrap()).cloned().collect::<Vec<_>>().into();
    assert_eq!(pools(&old),pools(&current));assert_eq!(pools(&both),pools(&current));
    // Unchanged and unknown history entries retain exact-match behavior.
    old.generated_ability_grants=vec!["numo-distract".into(),"unknown:history".into()].into();
    assert_eq!(pools(&old).iter().map(Vec::len).sum::<usize>(),71);
}

#[test]
fn historical_exclusion_keeps_the_existing_two_draw_activation_and_payment() {
    let mut r=room(50.);r.gods=(1..12).map(|god|god as f64).collect();
    r.player.abilities=vec!["kative-tinker".into()];r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.player.energy=100.;
    // Explicit test-only empty reserve, not production capacity or acquisition.
    revision50_ability_energy::initialize_reserve(&mut r.player,"kative-tinker",0.,0.).unwrap();
    let remaining="shrail-rage-at-the-dying-of-the-light";
    r.generated_ability_grants=identities().into_iter().filter(|row|row["godId"]==0 && row["revision50Id"]!=remaining)
        .map(|row|row["legacyId"].as_str().unwrap().to_string()).collect::<Vec<_>>().into();
    r.inventory.push(Item{id:900.,name:"Ring of Light".into(),kind:"ring".into(),effect:"light".to_string().into(),..Default::default()});
    let history=r.generated_ability_grants.as_ref().unwrap().clone();
    assert_eq!(pools(&r),vec![vec![remaining.to_string()]]);
    let mut expected=rng::to_u32(r.rng);rng::next_u32(&mut expected);rng::next_u32(&mut expected);
    assert!(tinker::activate(&mut r,Some(900.)).unwrap());
    assert_eq!(r.rng,expected as f64);assert_eq!(r.player.energy,0.);
    assert_eq!(r.inventory[0].granted_ability.as_ref().map(String::as_str),Some(remaining));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[remaining].energy,50.);
    assert!(r.player.abilities.iter().any(|id|id==remaining),"a carried Tinker grant activates immediately");
    r.player.ability_reserves.as_mut().unwrap()[remaining].energy=9.;
    assert_eq!(inventory::command(&mut r,&Command::Drop{id:900.}).unwrap(),Some(true));
    assert!(!r.player.abilities.iter().any(|id|id==remaining));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[remaining].energy,9.);
    for item in r.ground_at_mut(r.depth) {item.auto_pickup_blocked=Field::Missing;}
    assert_eq!(inventory::collect_underfoot(&mut r,20.).unwrap(),1);
    assert!(r.player.abilities.iter().any(|id|id==remaining));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[remaining].energy,9.);
    assert_eq!(&r.generated_ability_grants.as_ref().unwrap()[..history.len()],history.as_slice());
    assert!(pools(&r).is_empty());
}

#[test]
fn every_legacy_revision_keeps_exact_id_exclusion_and_pool_order() {
    for revision in [29.,39.,41.,46.] {
        let mut r=room(revision);r.gods=vec![1.,3.,10.];
        r.generated_ability_grants=vec!["shrail-unyielding-assault".into(),"grisha-uncast".into(),"shrail-execute".into()].into();
        r.inventory.push(Item{kind:"ring".into(),granted_ability:"etill-drown".to_string().into(),..Default::default()});
        let excluded:HashSet<_>=r.generated_ability_grants.as_ref().unwrap().iter().cloned()
            .chain(r.inventory.iter().flat_map(tinker::grants).cloned()).collect();
        let expected:Vec<Vec<String>>=(0..12).filter(|god|!r.gods.contains(&(*god as f64)))
            .map(|god|progression::catalog_for_revision(revision).iter().filter(|a|a.god_id==god as f64 && !excluded.contains(&a.id))
                .map(|a|a.id.clone()).collect::<Vec<_>>()).filter(|pool|!pool.is_empty()).collect();
        let before=serde_json::to_vec(&r).unwrap();assert_eq!(pools(&r),expected);
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}
