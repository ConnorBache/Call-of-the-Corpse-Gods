//! Real acquisition and owner-resource paths, with no R50 release-gate bypass.
use corpse_core::{Actor, Command, Field, GroundItem, Item, Run, abilities, equipment, inventory,
    progression, revision50_ability_energy as energy, revision50_amulets as amulets, temporal::Snapshot};
const ID: &str = "morae-all-seeing-eyes";
const GRANT: &str = "numo-distract";
fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.energy=100.;r.inventory.clear();r.actors_at_mut(r.depth).clear();r.ground.clear();r.offers.clear();
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r
}
fn ring(id:f64)->Item {Item{id,name:"Ring of Light".into(),kind:"ring".into(),effect:"light".to_string().into(),granted_ability:GRANT.to_string().into(),..Default::default()}}
fn reserve(r:&Run,id:&str)->f64 {r.player.ability_reserves.as_ref().unwrap()[id].energy}
fn pick_up(r:&mut Run,item:Item) {
    let (x,y)=(r.player.x,r.player.y);let depth=r.depth;
    r.ground_at_mut(depth).push(GroundItem{item,x,y,..Default::default()});
    assert_eq!(inventory::collect_underfoot(r,20.).unwrap(),1);
}
fn drop_item(r:&mut Run,id:f64) {
    assert_eq!(inventory::command(r,&Command::Drop{id}).unwrap(),Some(true));
}
#[test]
fn paid_activation_splits_remaining_charge_and_rejection_is_atomic() {
    let mut r=room();r.player.abilities=vec![ID.into()];energy::acquire(&mut r.player,&[ID.into()]).unwrap();
    r.player.ability_reserves.as_mut().unwrap()[ID].energy=7.;r.player.energy=23.;
    abilities::activate(&mut r,ID,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
    assert_eq!((reserve(&r,ID),r.player.energy),(0.,0.));assert_eq!(r.player.conditions.get("allSeeingReveal"),Some(&1.));
    let mut r=room();r.player.abilities=vec![ID.into()];energy::acquire(&mut r.player,&[ID.into()]).unwrap();
    r.player.ability_reserves.as_mut().unwrap()[ID].energy=7.;r.player.energy=22.;
    let before=(r.player.clone(),r.rng,r.tick,r.resource_history.clone());
    abilities::activate(&mut r,ID,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
    assert_eq!((r.player.clone(),r.rng,r.tick,r.resource_history.clone()),before);
}
#[test]
fn offers_initialize_once_and_promotion_keeps_existing_charge() {
    let mut r=room();r.offers=vec![ID.into()];progression::choose_offer(&mut r,ID).unwrap();
    assert_eq!(reserve(&r,ID),50.);assert_eq!(r.player.ability_reserves.as_ref().unwrap()[ID].capacity,50.);
    r.player.ability_reserves.as_mut().unwrap()[ID].energy=4.;
    r.offers=vec![ID.into()];progression::choose_offer(&mut r,ID).unwrap();assert_eq!(reserve(&r,ID),4.);
    pick_up(&mut r,ring(900.));
    r.player.ability_reserves.as_mut().unwrap()[GRANT].energy=6.;
    r.offers=vec![GRANT.into()];progression::choose_offer(&mut r,GRANT).unwrap();
    drop_item(&mut r,900.);assert!(r.player.abilities.contains(&GRANT.into()));assert_eq!(reserve(&r,GRANT),6.);
}
#[test]
fn pickup_activates_grants_and_initializes_amulets_only_once() {
    let mut r=room();let (x,y)=(r.player.x,r.player.y);
    let mut amulet=Item{id:901.,name:"Test amulet".into(),kind:"amulet".into(),..Default::default()};
    amulet.additional_granted_abilities=vec!["etill-drown".into()];
    r.ground_at_mut(r.depth).extend([GroundItem{item:ring(900.),x,y,..Default::default()},GroundItem{item:amulet,x,y,..Default::default()}]);
    assert_eq!(inventory::collect_underfoot(&mut r,20.).unwrap(),2);
    assert_eq!(reserve(&r,GRANT),50.);assert_eq!(reserve(&r,"etill-drown"),50.);
    assert!(r.player.abilities.contains(&GRANT.into()));
    assert!(r.player.abilities.contains(&"etill-drown".into()));
    assert_eq!(r.inventory.iter().find(|i|i.id==901.).unwrap().amulet_state.as_ref().unwrap().energy,50.);
    r.player.ability_reserves.as_mut().unwrap()[GRANT].energy=3.;
    assert!(!equipment::equip(&mut r,900.,"ring1").unwrap());
    assert!(!equipment::unequip(&mut r,900.).unwrap());
    assert!(r.player.abilities.contains(&GRANT.into()));assert_eq!(reserve(&r,GRANT),3.);
    drop_item(&mut r,900.);
    assert!(!r.player.abilities.contains(&GRANT.into()));assert_eq!(reserve(&r,GRANT),3.);
    // Drop suppresses automatic pickup until explicit collection re-entry.
    for g in r.ground_at_mut(r.depth) {g.auto_pickup_blocked=Field::Missing;}
    assert_eq!(inventory::collect_underfoot(&mut r,20.).unwrap(),1);assert_eq!(reserve(&r,GRANT),3.);
    assert!(r.player.abilities.contains(&GRANT.into()));
    equipment::sync_ownership(&mut r).unwrap();assert_eq!(reserve(&r,GRANT),3.);
}
#[test]
fn level_up_refills_dormant_blessings_never_player_or_amulet_and_roundtrips() {
    let mut r=room();r.inventory.push(ring(900.));equipment::sync_ownership(&mut r).unwrap();
    r.player.ability_reserves.as_mut().unwrap()[GRANT].energy=2.;r.player.energy=7.;
    drop_item(&mut r,900.);assert!(!r.player.abilities.contains(&GRANT.into()));
    let mut item=Item{id:901.,kind:"amulet".into(),..Default::default()};amulets::initialize_item(&mut item).unwrap();
    item.amulet_state.as_mut().unwrap().energy=11.;r.inventory.push(item);
    let snap=Snapshot::capture(&r);let raw=serde_json::to_string(&r).unwrap();
    let mut loaded:Run=serde_json::from_str(&raw).unwrap();
    for r in [&mut r,&mut loaded] {r.exp=2.0f64.powf(r.level);r.gods.clear();progression::level_up(r);}
    assert_eq!(r.player.ability_reserves,loaded.player.ability_reserves);
    assert_eq!(reserve(&r,GRANT),50.);assert_eq!(r.player.energy,7.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,11.);
    snap.restore(&mut r).unwrap();assert_eq!(reserve(&r,GRANT),2.);assert_eq!(r.player.energy,7.);
}
#[test]
fn canonical_owner_pays_for_half_here_and_dream_body_not_the_copied_pool() {
    for dream in [false,true] {
        let mut r=room();energy::acquire(&mut r.player,&[GRANT.into()]).unwrap();
        r.player.ability_reserves.as_mut().unwrap()[GRANT].energy=3.;r.player.energy=8.;
        let mut body=Actor{id:900.,name:"Shared body".into(),hp:10.,max_hp:10.,energy:100.,..Default::default()};
        body.conditions.insert(if dream {"phantasmiiBody"} else {"halfHereClone"}.into(),1.);
        body.conditions.insert("halfHereOwner".into(),r.player.id);
        energy::acquire(&mut body,&[GRANT.into()]).unwrap();
        r.actors_at_mut(r.depth).push(body);
        if dream {r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,body_id:Some(900.),..Default::default()}.into();}
        assert!(energy::spend(&mut r,900.,GRANT,10.));assert_eq!((reserve(&r,GRANT),r.player.energy),(0.,1.));
        let body=corpse_core::engine::actor(&r,900.).unwrap();assert_eq!(body.energy,1.);assert_eq!(body.ability_reserves.as_ref().unwrap()[GRANT].energy,50.);
        let before=r.clone();assert!(!energy::spend(&mut r,900.,GRANT,2.));assert_eq!(r,before);
    }
}
#[test]
fn remember_copies_reserves_once_and_reacquiring_does_not_refill() {
    let mut r=room();let remember="numo-remember";let copied="numo-mind-read";
    r.player.abilities=vec![remember.into()];energy::acquire(&mut r.player,&[remember.into()]).unwrap();
    let mut target=Actor{id:900.,name:"Test enemy".into(),hp:100.,max_hp:100.,x:r.player.x+1.,y:r.player.y,abilities:vec![copied.into()],..Default::default()};
    target.conditions.insert("bestiaryTraitsInitialized".into(),1.);r.actors_at_mut(r.depth).push(target);
    abilities::activate(&mut r,remember,&900.0.into(),&Field::Missing,&Field::Missing).unwrap();
    assert!(r.player.abilities.contains(&copied.into()));assert_eq!(reserve(&r,copied),50.);assert_eq!(reserve(&r,remember),45.);
    r.player.ability_reserves.as_mut().unwrap()[copied].energy=8.;
    // The expiry path removes source ownership, but deliberately leaves the pool.
    r.player.abilities.retain(|id|id!=copied);r.player.conditions.shift_remove(&format!("remembered:{copied}"));
    abilities::activate(&mut r,remember,&900.0.into(),&Field::Missing,&Field::Missing).unwrap();
    assert_eq!(reserve(&r,copied),8.);assert_eq!(reserve(&r,remember),40.);
}
#[test]
fn actual_revision50_starters_initialize_authored_reserves() {
    let r=corpse_core::startup::new_run_shell_revision("0-1-2:reserve-startup","Avatar",false,50.).unwrap();
    assert_eq!(r.player.abilities.len(),3);
    for id in &r.player.abilities {assert_eq!(reserve(&r,id),50.);assert_eq!(r.player.ability_reserves.as_ref().unwrap()[id].capacity,50.);}
    let old=corpse_core::startup::new_run_shell_revision("0-1-2:reserve-startup","Avatar",false,46.).unwrap();
    assert!(old.player.ability_reserves.is_missing());
}

#[test]
fn ownership_rejects_retired_or_unknown_grants_and_manual_toggles_are_blocked() {
    for id in ["shrail-unyielding-assault", "unknown:grant"] {
        for additional in [false,true] {
            let mut r=room();let mut item=ring(900.);
            if additional {item.additional_granted_abilities=vec![id.into()];}
            else {item.granted_ability=id.to_string().into();}
            r.inventory.push(item);let before=r.clone();
            assert!(equipment::validate_for_revision(&r.inventory[0],50.).is_err());assert_eq!(r,before);
            let mut blocked=r.clone();
            assert!(!equipment::equip(&mut blocked,900.,"ring1").unwrap());
            assert_eq!((blocked.inventory,blocked.player,blocked.tick,blocked.rng),
                (r.inventory.clone(),r.player.clone(),r.tick,r.rng));
            assert!(equipment::sync_ownership(&mut r).is_err());assert_eq!(r,before);
        }
    }
    // A bad carried peer is rejected by ownership preflight before valid grants
    // can mutate the actor, inventory or acquisition reserves.
    let mut r=room();r.inventory.push(ring(900.));
    let mut bad=ring(901.);bad.granted_ability="shrail-unyielding-assault".to_string().into();r.inventory.push(bad);
    let before=r.clone();assert!(equipment::sync_ownership(&mut r).is_err());assert_eq!(r,before);
    // Preserve legacy grant admission and ordinary ungranted special-item rules.
    r.simulation_revision=46.0.into();assert!(equipment::equip(&mut r,901.,"ring1").unwrap());
    let mut special=Item{id:902.,name:"Masque of the Trickster".into(),kind:"trinket".into(),..Default::default()};
    assert!(equipment::validate_for_revision(&special,50.).is_ok());
    special.kind="ring".into();r=room();r.inventory.push(special);
    assert!(!equipment::equip(&mut r,902.,"ring1").unwrap());
    equipment::sync_ownership(&mut r).unwrap();
    assert!(corpse_core::revision50_inventory::worn(&r.inventory[0]));
}

#[test]
fn carried_jewelry_grants_and_bonuses_ignore_all_historical_equipped_flags() {
    for flag in [Field::Missing,Field::Null,Field::Value(String::new()),
        "ring1".to_string().into(),"amulet".to_string().into()] {
        let mut r=room();
        let base=corpse_core::combat::effective_stats(&r,&r.player);
        let mut jewel=ring(900.);jewel.equipped=flag.clone();
        jewel.bonus=corpse_core::PartialStats{offense:3.0.into(),..Default::default()}.into();
        let amulet=Item{id:901.,kind:"amulet".into(),equipped:flag,
            granted_ability:"etill-drown".to_string().into(),
            bonus:corpse_core::PartialStats{defense:2.0.into(),..Default::default()}.into(),..Default::default()};
        pick_up(&mut r,jewel);pick_up(&mut r,amulet);
        assert!(r.player.abilities.contains(&GRANT.into()));
        assert!(r.player.abilities.contains(&"etill-drown".into()));
        let stats=corpse_core::combat::effective_stats(&r,&r.player);
        assert_eq!((stats.offense,stats.defense),(base.offense+3.,base.defense+2.));
        r.player.ability_reserves.as_mut().unwrap()[GRANT].energy=3.;
        equipment::sync_ownership(&mut r).unwrap();assert_eq!(reserve(&r,GRANT),3.);
        drop_item(&mut r,900.);drop_item(&mut r,901.);
        assert!(!r.player.abilities.contains(&GRANT.into()));
        assert!(!r.player.abilities.contains(&"etill-drown".into()));
        assert_eq!(corpse_core::combat::effective_stats(&r,&r.player),base);
    }
}
