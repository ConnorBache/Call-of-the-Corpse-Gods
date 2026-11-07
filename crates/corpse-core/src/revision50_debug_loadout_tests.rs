use super::*;
fn room(start:u32)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.depth=start as f64;
    let floor=(**r.floors.first().unwrap()).clone();
    r.floors=(1..=start).map(|depth|{let mut f=floor.clone();f.depth=depth as f64;Arc::new(f)}).collect();
    r.inventory=crate::startup::starting_inventory(50.).unwrap();r.actors=Default::default();r.ground.clear();r.offers.clear();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_reserves=Field::Missing;
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.playtest=crate::playtest::Profile {version:2,start_floor:start,profile:PROFILE.into(),prior_hearts:0,prior_blessings:0.,sources:vec![],candidates:vec![],
        debug:Some(State{pending:VecDeque::new(),consumed:0,stash:vec![],entered:false,loadout_revision:0})}.into();r
}
fn stash(r:&mut Run,item:Item) {
    r.playtest.as_mut().unwrap().sources.push(crate::playtest::EquipmentSource{depth:1,item_id:item.id});
    state_mut(r).unwrap().stash.push(Reward{depth:1,item});
}
#[test]
fn deeper_r50_stash_has_unlimited_trinkets_but_enforces_finite_categories() {
    for start in [6,40] {
        let mut r=room(start);
        for id in 100..220 {stash(&mut r,crate::equipment::make_equipment("Composite bow",id as f64).unwrap());assert!(take(&mut r,id as f64).unwrap());}
        stash(&mut r,crate::equipment::make_arrows(230.,500.));assert!(take(&mut r,230.).unwrap());
        assert_eq!(crate::inventory::quantity(r.inventory.iter().find(|i|i.id==3.).unwrap()),505.);
        assert_eq!(crate::revision50_inventory::limited_occupancy(&r),0.);
        for n in 0..6 {stash(&mut r,Item{id:300.+n as f64,kind:"potion".into(),name:"Potion of Healing".into(),effect:"healing".to_string().into(),..Default::default()});}
        for id in 300..305 {assert!(take(&mut r,id as f64).unwrap());}
        let before=r.clone();assert!(take(&mut r,305.).is_err());assert_eq!(r,before);
        stash(&mut r,crate::royal_honey::item(400.));assert!(take(&mut r,400.).unwrap());
        assert_eq!(crate::revision50_inventory::limited_occupancy(&r),5.);
        assert!(crate::revision50_inventory::finite_categories_fit(&r.inventory,&r.player.abilities));
    }
}
#[test]
fn stash_acquisition_materializes_once_and_preserves_spent_item_and_grant_energy() {
    let mut r=room(6);
    let item=Item{id:900.,kind:"amulet".into(),name:"Amulet of Lightning".into(),effect:"lightning".to_string().into(),granted_ability:"shrail-execute".to_string().into(),..Default::default()};
    stash(&mut r,item);assert!(take(&mut r,900.).unwrap());
    assert_eq!(r.inventory.last().unwrap().amulet_state.as_ref().unwrap().energy,50.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["shrail-execute"].energy,crate::revision50_ability_energy::BASE_ENERGY);
    assert!(r.player.abilities.iter().any(|id|id=="shrail-execute"));
    assert!(r.inventory.last().unwrap().equipped.is_missing());
    r.inventory.last_mut().unwrap().amulet_state.as_mut().unwrap().energy=9.;
    r.player.ability_reserves.as_mut().unwrap().get_mut("shrail-execute").unwrap().energy=7.;
    assert!(put(&mut r,900.).unwrap());
    assert!(!r.player.abilities.iter().any(|id|id=="shrail-execute"));
    assert!(take(&mut r,900.).unwrap());
    let carried = r.clone();assert!(equip(&mut r,900.).unwrap());assert_eq!(r,carried);
    assert_eq!(r.inventory.last().unwrap().amulet_state.as_ref().unwrap().energy,9.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["shrail-execute"].energy,7.);
    assert!(crate::revision50_inventory::worn(r.inventory.last().unwrap()));
}
#[test]
fn invalid_stash_grants_and_non_jewelry_equip_are_atomic() {
    let mut r=room(40);
    stash(&mut r,Item{id:900.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),granted_ability:"shrail-unyielding-assault".to_string().into(),..Default::default()});
    let before=r.clone();assert!(take(&mut r,900.).is_err());assert_eq!(r,before);
    stash(&mut r,crate::borge::shard(901.));let before=r.clone();assert!(equip(&mut r,901.).is_err());assert_eq!(r,before);
    assert!(take(&mut r,901.).unwrap());assert!(crate::borge::is_shard(r.inventory.last().unwrap()));
    stash(&mut r,crate::equipment::make_equipment("Shortsword",902.).unwrap());let before=r.clone();assert!(take(&mut r,902.).is_err());assert_eq!(r,before);
}
#[test]
fn legacy_stash_still_has_shared_twenty_slots() {
    let mut r=room(6);r.simulation_revision=46.0.into();r.inventory=crate::startup::starting_inventory(46.).unwrap();
    for id in 100..117 {stash(&mut r,crate::equipment::make_equipment("Composite bow",id as f64).unwrap());assert!(take(&mut r,id as f64).unwrap());}
    assert_eq!(crate::inventory::occupancy(&r),20.);
    stash(&mut r,crate::equipment::make_equipment("Bow",118.).unwrap());let before=r.clone();assert!(take(&mut r,118.).is_err());assert_eq!(r,before);
    assert!(r.player.ability_reserves.is_missing());
}

#[test]
fn r50_debug_jewelry_is_active_on_take_and_each_category_has_five_places() {
    for start in [6,40] {
        let mut r=room(start);
        for (category,kind) in ["ring","amulet","potion","heart"].iter().enumerate() {
            for n in 0..6 {
                let id=1000.+(category*10+n) as f64;
                let mut item=Item{id,kind:(*kind).into(),name:(*kind).into(),..Default::default()};
                if *kind=="ring" {item.effect="light".to_string().into();}
                // Old equipped labels must not turn a sixth item into free cargo.
                if matches!(*kind,"ring"|"amulet") && n==5 {item.equipped=(*kind).to_string().into();}
                stash(&mut r,item);
                if n<5 {assert!(take(&mut r,id).unwrap());}
                else {
                    let before=r.clone();assert!(take(&mut r,id).is_err());assert_eq!(r,before);
                    if matches!(*kind,"ring"|"amulet") {assert!(equip(&mut r,id).is_err());assert_eq!(r,before);}
                }
            }
        }
        assert_eq!(crate::revision50_inventory::limited_occupancy(&r),20.);
        assert_eq!(crate::revision50_rings::count_effect(&r,&r.player,"light"),5);
        assert!(r.inventory.iter().filter(|i|matches!(i.kind.as_str(),"ring"|"amulet")).all(|i|i.equipped.is_missing()));
        assert!(put(&mut r,1000.).unwrap());
        assert_eq!(crate::revision50_rings::count_effect(&r,&r.player,"light"),4);
        assert!(equip(&mut r,1005.).unwrap());
        assert_eq!(crate::revision50_rings::count_effect(&r,&r.player,"light"),5);
        assert!(crate::revision50_inventory::finite_categories_fit(&r.inventory,&r.player.abilities));
    }
}
#[test]
fn r50_debug_return_blocks_capacity_loss_without_discarding_or_refilling() {
    let mut r=room(6);
    stash(&mut r,Item{id:900.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),
        granted_ability:"isheilah-tools-of-the-trade".to_string().into(),..Default::default()});
    assert!(take(&mut r,900.).unwrap());
    for n in 0..6 {
        let id=1000.+n as f64;
        stash(&mut r,Item{id,kind:"potion".into(),name:"Potion of Healing".into(),effect:"healing".to_string().into(),..Default::default()});
        assert!(take(&mut r,id).unwrap());
    }
    r.player.ability_reserves.as_mut().unwrap()["isheilah-tools-of-the-trade"].energy=7.;
    let before=r.clone();assert!(put(&mut r,900.).is_err());assert_eq!(r,before);
    assert!(put(&mut r,1005.).unwrap());assert!(put(&mut r,900.).unwrap());
    assert!(!r.player.abilities.iter().any(|id|id=="isheilah-tools-of-the-trade"));
    assert!(take(&mut r,900.).unwrap());
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["isheilah-tools-of-the-trade"].energy,7.);
}
