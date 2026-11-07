use super::*;
fn room(start: u32) -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.seed="0-5-11:r50-deeper-loadout".into();
    let floor=(**r.floors.first().unwrap()).clone();
    r.floors=(1..=start).map(|depth| {let mut f=floor.clone();f.depth=depth as f64;std::sync::Arc::new(f)}).collect();
    r.depth=start as f64;r.inventory=crate::startup::starting_inventory(50.).unwrap();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;
    r.actors=Default::default();r.ground.clear();r.offers.clear();
    r.playtest=Profile{version:1,start_floor:start,profile:PROFILE.into(),prior_hearts:0,prior_blessings:0.,sources:vec![],candidates:vec![],debug:None}.into();r
}
#[test]
fn deeper_r50_collection_and_selection_preserve_starters_named_jewelry_and_trinkets() {
    for start in [6,40] {
        let mut r=room(start);let mut rng=crate::rng::GenerationRng::new(start);let mut id=4.;
        // The first post-starter item must participate, not be left behind by drain(3..).
        let mut first=crate::equipment::create_equipment_revision("ring",&mut rng,&mut id,1,&[],50.).unwrap();
        first.granted_ability="shrail-execute".to_string().into();
        collect_reference_floor(&mut r,1,&[],&[first]).unwrap();
        // A previously spent reserve stays spent if its grant is selected later.
        crate::revision50_ability_energy::initialize_reserve(&mut r.player,"shrail-execute",50.,7.).unwrap();
        for depth in 1..start {
            let loot=crate::equipment::generate_floor_loot_revision(depth,&mut rng,&mut id,&[],20,50.).unwrap();
            collect_reference_floor(&mut r,depth,&[],&loot).unwrap();
        }
        let trinkets:Vec<_>=r.playtest.as_ref().unwrap().candidates.iter().filter(|i|crate::revision50_inventory::category(i)==Some(crate::revision50_inventory::Category::Trinket)).map(|i|i.id).collect();
        let rng_before=r.rng;let mut repeated=r.clone();
        select_loadout(&mut r).unwrap();select_loadout(&mut repeated).unwrap();
        assert_eq!(r,repeated);assert_eq!(r.rng,rng_before);
        assert!(r.playtest.as_ref().unwrap().candidates.is_empty());
        assert_eq!(r.inventory.iter().filter(|i|i.kind=="ring").count(),2);
        assert_eq!(r.inventory.iter().filter(|i|i.kind=="amulet").count(),1);
        assert_eq!(r.inventory.iter().filter(|i|i.kind=="potion").count(),4);
        assert!(r.inventory.iter().filter(|i|matches!(i.kind.as_str(),"ring"|"amulet")).all(crate::revision50_inventory::worn));
        assert!(r.inventory.iter().filter(|i|matches!(i.kind.as_str(),"ring"|"amulet")).all(|i|i.equipped.is_missing()));
        assert!(r.inventory.iter().filter(|i|i.kind=="weapon").all(|i|crate::projectiles::is_bow(i)&&i.equipped.is_missing()));
        assert!(r.inventory.iter().any(|i|i.id==2.)&&r.inventory.iter().any(|i|i.id==3.));
        assert!(!r.inventory.iter().any(|i|i.id==1.||i.kind=="armor"));
        for id in trinkets {assert!(r.inventory.iter().any(|i|i.id==id));}
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()["shrail-execute"].energy,7.);
        assert!(r.inventory.iter().filter(|i|i.kind=="amulet").all(|i|i.amulet_state.as_ref().is_some_and(|s|s.energy==50.)));
        assert!(crate::revision50_inventory::finite_categories_fit(&r.inventory,&r.player.abilities));
        assert!(r.playtest.as_ref().unwrap().sources.iter().all(|s|s.depth<start&&r.inventory.iter().any(|i|i.id==s.item_id)));
    }
}
#[test]
fn deeper_r50_with_only_two_starters_does_not_panic_or_equip_a_weapon() {
    for start in [2,6,40] {let mut r=room(start);let before=r.inventory.clone();select_loadout(&mut r).unwrap();assert_eq!(r.inventory,before);}
}

#[test]
fn r50_selected_jewelry_grants_are_active_without_slot_assignment() {
    let mut r=room(6);
    let ring=Item{id:900.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),
        granted_ability:"numo-distract".to_string().into(),..Default::default()};
    let amulet=Item{id:901.,kind:"amulet".into(),name:"Amulet of Lightning".into(),effect:"lightning".to_string().into(),
        granted_ability:"shrail-execute".to_string().into(),..Default::default()};
    collect_reference_floor(&mut r,1,&[],&[ring,amulet]).unwrap();
    crate::revision50_ability_energy::initialize_reserve(&mut r.player,"numo-distract",50.,3.).unwrap();
    select_loadout(&mut r).unwrap();
    for id in ["numo-distract","shrail-execute"] {assert!(r.player.abilities.iter().any(|owned|owned==id));}
    assert!(r.inventory.iter().filter(|i|i.id>=900.).all(|i|i.equipped.is_missing()));
    assert!(crate::revision50_rings::has_effect(&r,&r.player,"light"));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["numo-distract"].energy,3.);
    assert_eq!(r.inventory.iter().find(|i|i.id==901.).unwrap().amulet_state.as_ref().unwrap().energy,50.);
}

#[test]
fn r50_unselected_candidates_never_affect_prior_heart_progression_or_save_schema() {
    let mut r=room(6);let mut baseline=r.clone();
    let mut ring=Item{id:900.,kind:"ring".into(),name:"Ring of Protection".into(),effect:"protection".to_string().into(),
        granted_ability:"isheilah-tools-of-the-trade".to_string().into(),..Default::default()};
    ring.bonus=crate::PartialStats{defense:20.0.into(),max_hp:40.0.into(),..Default::default()}.into();
    collect_reference_floor(&mut r,1,&[],&[ring]).unwrap();
    assert_eq!(r.inventory,baseline.inventory);assert_eq!(r.player,baseline.player);
    assert_eq!(crate::combat::effective_stats(&r,&r.player),crate::combat::effective_stats(&baseline,&baseline.player));
    assert!(!crate::revision50_rings::has_effect(&r,&r.player,"protection"));
    let profile=serde_json::to_value(r.playtest.as_ref().unwrap()).unwrap();
    assert!(profile.get("candidates").is_none());
    let actor=crate::population::create_actor_revision("rat",800.,10.,10.,false,50.).unwrap();
    collect_reference_floor(&mut r,2,&[actor.clone()],&[]).unwrap();
    collect_reference_floor(&mut baseline,2,&[actor],&[]).unwrap();
    assert_eq!(r.player,baseline.player);assert_eq!(r.rng,baseline.rng);assert_eq!(r.level,baseline.level);
    select_loadout(&mut r).unwrap();
    assert!(r.playtest.as_ref().unwrap().candidates.is_empty());
    assert!(crate::revision50_rings::has_effect(&r,&r.player,"protection"));
    assert!(r.player.abilities.iter().any(|id|id=="isheilah-tools-of-the-trade"));
}
