//! Revision41 metadata/admission tests; no trail direction or forced-movement policy.
use corpse_core::{Field, Run, progression as p, rng::GenerationRng};
/// O(fixture bytes); cold setup only.
fn fixture(revision:f64)->Run {
 let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=revision.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
 r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.offers.clear();r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r
}
/// O(A*R), exact old metadata and pointer checks.
#[test]fn frozen_catalog_is_unchanged_and_only_revision41_slot_is_replaced(){
 let old=p::catalog();assert_eq!(old.len(),72);for rev in 29..=40{assert!(std::ptr::eq(p::catalog_for_revision(rev as f64),old));}
 let new=p::catalog_for_revision(41.);assert_eq!(new.len(),old.len());let mut n=0;
 for(a,b)in old.iter().zip(new){if a.id=="numo-revert"{n+=1;assert_eq!(b.id,p::WEAVE_ID);assert_eq!(b.name,"Weave of Time");assert_eq!(b.description,"You can freely travel along your time weave, which you leave behind as you move.");assert_eq!(b.flags,vec!["passive"]);assert_eq!(a.god_id,b.god_id);assert!(a.flags.is_empty());assert_eq!(a.name,"Revert");}else{assert_eq!((&a.id,&a.name,&a.description,&a.flags,a.god_id),(&b.id,&b.name,&b.description,&b.flags,b.god_id));}}
 assert_eq!(n,1);
}
/// O(reference bytes+cases*A*weight work); frozen authored oracle.
#[test]fn all_legacy_offer_weights_still_match_frozen_authored_values(){
 let cases:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/progression-v48.json")).unwrap();
 for c in cases.as_array().unwrap(){let owned:Vec<String>=serde_json::from_value(c["owned"].clone()).unwrap();for rev in [29.,39.,40.]{for(i,a)in p::catalog().iter().enumerate(){assert_eq!(p::offer_weight_for_revision(rev,a,&owned),c["weights"][i].as_f64().unwrap());}}}
 let weave=p::definition(p::WEAVE_ID).unwrap();assert_eq!(p::offer_weight_for_revision(41.,weave,&[p::WEAVE_ID.into()]),2.);
}
/// O(128*A²); identical first-offer draws with empty ownership.
#[test]fn first_numo_offers_replace_id_without_extra_rng_draws(){
 let mut seen=false;for seed in 1..=128{let mut old=Run::default();old.player.x=-1.;old.mode="Blessed".into();old.gods=vec![3.];old.level=1.;old.exp=2.;old.rng=seed as f64;old.simulation_revision=40.0.into();let mut new=old.clone();new.simulation_revision=41.0.into();p::level_up(&mut old);p::level_up(&mut new);assert_eq!(old.rng,new.rng);assert_eq!(old.offers.len(),new.offers.len());for(a,b)in old.offers.iter().zip(&new.offers){assert_eq!(if a=="numo-revert"{p::WEAVE_ID}else{a},b);seen|=b==p::WEAVE_ID;assert_ne!(b,"numo-revert");}}assert!(seen);
}
/// O(128*(catalog+item payload)); exact old/new draw count and unchanged item fields.
#[test]fn generated_rings_replace_revert_at41_and_legacy_rng_is_identical(){
 let excluded:Vec<_>=p::catalog().iter().filter(|a|a.id!="numo-revert").map(|a|a.id.clone()).collect();let mut granted=false;
 for seed in 1..=128{let mut a=GenerationRng::new(seed);let mut b=a;let(mut x,mut y)=(1.,1.);let old=corpse_core::equipment::create_equipment("ring",&mut a,&mut x,10,&excluded).unwrap();let mut new=corpse_core::equipment::create_equipment_revision("ring",&mut b,&mut y,10,&excluded,41.).unwrap();assert_eq!(a.state,b.state);assert_eq!(x,y);if old.granted_ability.as_ref().is_some(){granted=true;assert_eq!(old.granted_ability.as_ref().unwrap(),"numo-revert");assert_eq!(new.granted_ability.as_ref().unwrap(),p::WEAVE_ID);new.granted_ability="numo-revert".to_string().into();new.name=new.name.replace(p::WEAVE_NAME,"Revert");}assert_eq!(serde_json::to_value(&old).unwrap(),serde_json::to_value(&new).unwrap());}assert!(granted);
}
/// O(fixture bytes+catalog/ownership scans); passive acquisition creates no charge/toggle.
#[test]fn weave_offer_and_equipment_ownership_are_admitted_only_at41(){
 let mut r=fixture(41.);r.offers=vec![p::WEAVE_ID.into()];assert!(p::choose_offer(&mut r,p::WEAVE_ID).unwrap());assert!(r.player.abilities.iter().any(|a|a==p::WEAVE_ID));assert!(r.player.conditions.contains_key(&format!("acquired:{}",p::WEAVE_ID)));assert!(!r.player.conditions.contains_key("revertCharges"));assert!(r.player.active.is_empty());corpse_core::storage::validate(&r).unwrap();corpse_core::storage::validate_current_revision(&r).unwrap();r.simulation_revision=40.0.into();assert!(corpse_core::storage::validate(&r).is_err());
 let mut r=fixture(41.);r.gods.clear();r.generated_ability_grants=Field::Missing;r.inventory.clear();for g in r.ground.values_mut(){*g=std::sync::Arc::new(Vec::new());}let pools=corpse_core::tinker::god_pools(&r);assert!(pools.iter().flatten().any(|a|a.id==p::WEAVE_ID));assert!(!pools.iter().flatten().any(|a|a.id=="numo-revert"));let mut ring=corpse_core::equipment::make_equipment("Ring",9000.).unwrap();ring.granted_ability=p::WEAVE_ID.to_string().into();ring.equipped="ring1".to_string().into();let state=corpse_core::equipment_abilities::equipment_ability_state(&r,&[ring]);assert_eq!(state.grants,vec![p::WEAVE_ID]);
}
/// O(fixture bytes+boundary validation); no depth40 limit change.
#[test]fn revision40_and41_admit42_reject_and_legacy_trail_presence_is_rejected(){
 for rev in [39.,40.,41.]{let r=fixture(rev);corpse_core::storage::validate_current_revision(&r).unwrap();assert!(corpse_core::startup::new_run_shell_revision("weave-revision","Unblessed",false,rev).is_ok());}
 let r=fixture(47.);assert!(corpse_core::storage::validate_current_revision(&r).is_err());assert!(corpse_core::startup::new_run_shell_revision("weave-revision","Unblessed",false,47.).is_err());assert_eq!(corpse_core::startup::new_run_shell("unchanged-default","Unblessed",false).unwrap().simulation_revision.value_or(0.),39.);
 let mut old=serde_json::to_value(fixture(40.)).unwrap();old["timeweave"]=serde_json::Value::Null;assert!(corpse_core::validation::validate_value(&old).is_err());let mut new=fixture(41.);new.player.abilities.push("numo-revert".into());assert!(corpse_core::storage::validate(&new).is_err());
}

/// O(fixture bytes + environment admission scans); actual new passive is accepted by Wait preflight.
#[test]fn learned_weave_passes_player_environment_preflight(){
 let mut r=fixture(41.);r.player.abilities=vec![p::WEAVE_ID.into()];
 corpse_core::environment::validate_supported(&r).unwrap();
 r.simulation_revision=40.0.into();r.player.abilities=vec!["numo-revert".into()];
 corpse_core::environment::validate_supported(&r).unwrap();
}

/// Two real ten-floor startups with their existing bounded candidate/population
/// costs, then O(world bytes) admission. Unlike shell tests, this reaches all
/// generation-window/candidate/setup revision guards; no fixture geometry injection.
#[test]
fn actual_generated_revision40_and41_startups_reach_every_generation_gate() {
    for revision in [40.0, 41.0] {
        let run=corpse_core::startup::create_run_revision(
            "00-09-10:menu-seed-review", "Avatar", false, revision,
        ).unwrap();
        assert_eq!(run.simulation_revision.value_or(0.0),revision);
        assert_eq!(run.floors.len(),10);
        assert!(run.floors.iter().all(|floor|!floor.tiles.is_empty()));
        corpse_core::storage::validate(&run).unwrap();
        corpse_core::storage::validate_current_revision(&run).unwrap();
        let engine=corpse_core::temporal::Engine::new(run).unwrap();
        assert!(engine.history.frames.is_empty());
        if revision==41.0 {
            assert!(!engine.run.player.abilities.iter().any(|id|id=="numo-revert"));
            assert!(!engine.run.generated_ability_grants.as_ref().is_some_and(|ids|ids.iter().any(|id|id=="numo-revert")));
        }
    }
    assert!(corpse_core::startup::create_run_revision("weave-future","Avatar",false,47.0).is_err());
    assert!(corpse_core::generation::generate_floor_full_revision(1,1,None,47.0).is_err());
    // Revision41 is supported; floor41 remains outside the unchanged world bound.
    assert!(corpse_core::generation::generate_floor_full_revision(41,1,None,41.0).is_err());
}
