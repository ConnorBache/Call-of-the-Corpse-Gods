//! Revision-aware runtime preflight; unknown and retired powers remain rejected.
use corpse_core::{Actor, Command, Field, Run, abilities, engine, progression};
use serde_json::Value;

fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.inventory.clear();r.generated_ability_grants=Field::Missing;
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r.offers.clear();
    for actors in r.actors.values_mut(){*actors=std::sync::Arc::new(vec![]);}
    for ground in r.ground.values_mut(){*ground=std::sync::Arc::new(vec![]);}
    r
}
fn reject_unchanged(mut r:Run) {
    let before=serde_json::to_vec(&r).unwrap();
    assert!(engine::validate_supported(&r).is_err());
    assert!(engine::dispatch(&mut r,&Command::Wait).is_err());
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}

#[test]
fn every_retired_player_identity_rejects_before_wait_bookkeeping() {
    let rows:Vec<Value>=serde_json::from_str(include_str!("../catalog/ability-identities-v50.json")).unwrap();
    for row in rows.into_iter().filter(|row|row["legacyId"]!=row["revision50Id"]) {
        let mut r=room(50.);r.player.abilities=vec![row["legacyId"].as_str().unwrap().into()];
        reject_unchanged(r);
    }
}

#[test]
fn retired_active_ids_reject_even_when_not_present_in_owned_abilities() {
    for id in ["shrail-unyielding-assault","pelian-never-without-a-fight","grisha-uncast","ursa-waking-nightmare"] {
        let mut r=room(50.);r.player.active=vec![id.into()];reject_unchanged(r);
    }
}

#[test]
fn all_current_blessings_and_temporal_future_remain_admitted() {
    for a in progression::catalog_for_revision(50.) {
        let mut r=room(50.);r.player.abilities.push(a.id.clone());
        assert!(engine::validate_supported(&r).is_ok(),"{}",a.id);
    }
    let mut r=room(50.);r.player.abilities=vec!["unknown:power".into()];reject_unchanged(r);
    assert!(corpse_core::storage::validate_current_revision(&room(50.)).is_ok());
}

#[test]
fn borrowed_passive_traits_npcs_and_special_item_identities_are_not_catalog_migrated() {
    let mut r=room(50.);
    let id="enemy:beast:flight".to_string();
    assert!(abilities::copied_trait_supported(&id));r.player.abilities.push(id.clone());
    r.player.conditions.insert(format!("remembered:{id}"),r.tick+5.);
    let mut npc=corpse_core::population::create_actor_revision("rat",900.,13.,14.,false,50.).unwrap();
    npc.state="hunting".into();r.actors_at_mut(r.depth).push(npc);r.next_id=910.;
    r.inventory=vec![corpse_core::borge::shard(901.),corpse_core::phantasmii_generation::dagger(902.),
        corpse_core::phantasmii_generation::shard(903.),corpse_core::equipment::make_equipment("Bow",904.).unwrap()];
    let before=serde_json::to_vec(&r).unwrap();assert!(engine::validate_supported(&r).is_ok());
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}

#[test]
fn shared_endpoint_and_copy_reject_retired_powers_without_rejecting_current_ids() {
    for mini in [false,true] {
        let mut r=room(50.);
        let mut a=Actor{id:900.,name:"Test endpoint".into(),x:13.,y:14.,hp:10.,max_hp:10.,state:"ally".into(),..Default::default()};
        if mini {a.conditions.insert("miniMe".into(),1.);} else {
            a.conditions.insert("halfHereClone".into(),1.);a.conditions.insert("halfHereOwner".into(),r.player.id);
        }
        a.abilities.push("numo-distract".into());r.actors_at_mut(r.depth).push(a);r.next_id=901.;
        assert!(engine::validate_supported(&r).is_ok());
        r.actors_at_mut(r.depth)[0].abilities=vec!["shrail-unyielding-assault".into()];reject_unchanged(r);
    }
}

#[test]
fn legacy_power_admission_is_unchanged() {
    for revision in [29.,39.,46.] {
        let rows:Vec<Value>=serde_json::from_str(include_str!("../catalog/ability-identities-v50.json")).unwrap();
        for row in rows.into_iter().filter(|row|row["legacyId"]!=row["revision50Id"]) {
            let id=row["legacyId"].as_str().unwrap();let mut r=room(revision);r.player.abilities=vec![id.into()];
            assert!(engine::validate_supported(&r).is_ok(),"revision {revision}: {id}");
        }
    }
}
