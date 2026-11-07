//! Accounting fixtures, not generated amulet defaults or release admission.
use corpse_core::{Command, Field, Item, Run, Stats, inventory, revision50_amulets as a, temporal::Snapshot};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.player.stats=Stats{max_energy:100.0,..Default::default()};r.player.energy=30.0;
    r.inventory.clear();r.actors=Default::default();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;
    r.future=None;r.next_id=900.0;r
}
fn amulet(id:f64)->Item{Item{id,name:"Authored test amulet".into(),kind:"amulet".into(),glyph:"☷".into(),
    ..Default::default()}}
fn spec()->a::AmuletSpec{a::AmuletSpec{capacity:a::BASE_ENERGY,activation_cost:20.0}}
fn use_item(r:&mut Run,id:f64){let p=a::plan_item_activation(r,id,Some(&spec())).unwrap();a::commit_item_activation(r,id,&p,Some(&spec())).unwrap();}
#[test]
fn initial_fifty_is_once_only_never_drop_recollect_or_passive_refill(){
    let mut r=room();r.inventory.push(amulet(1.0));
    let before=serde_json::to_value(&r).unwrap();let p=a::plan_item_activation(&r,1.0,Some(&spec())).unwrap();
    assert_eq!(serde_json::to_value(&r).unwrap(),before);assert_eq!(p.amulet_energy_spent(),20.0);
    a::commit_item_activation(&mut r,1.0,&p,Some(&spec())).unwrap();
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,30.0);assert_eq!(r.player.energy,30.0);
    assert_eq!(inventory::command(&mut r,&Command::Drop{id:1.}).unwrap(),Some(true));
    assert_eq!(a::plan_item_activation(&r,1.,Some(&spec())),Err(a::AdmissionError::MissingItem));
    assert_eq!(r.ground.get(&r.depth.to_string()).unwrap()[0].item.amulet_state.as_ref().unwrap().energy,30.);
    for ground in r.ground_at_mut(r.depth) {ground.auto_pickup_blocked=Field::Missing;}
    assert_eq!(inventory::collect_underfoot(&mut r,20.).unwrap(),1);
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,30.0);
    let tick=r.tick;r.tick+=100.0;assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,30.0);r.tick=tick;
}
#[test]
fn free_credit_is_shared_and_invalid_stale_commit_is_unchanged(){
    let mut r=room();r.player.abilities.push("isheilah-spellslinger".into());r.inventory=vec![amulet(1.0),amulet(2.0)];
    let p=a::plan_item_activation(&r,1.0,Some(&spec())).unwrap();assert!(!p.consumes_action());
    let other=a::plan_item_activation(&r,2.0,Some(&spec())).unwrap();
    a::commit_item_activation(&mut r,1.0,&p,Some(&spec())).unwrap();
    assert!(a::plan_item_activation(&r,2.0,Some(&spec())).unwrap().consumes_action());
    let before=serde_json::to_value(&r).unwrap();assert!(a::commit_item_activation(&mut r,2.0,&other,Some(&spec())).is_err());
    assert_eq!(serde_json::to_value(&r).unwrap(),before);
    r.tick+=1.0;assert!(!a::plan_item_activation(&r,2.0,Some(&spec())).unwrap().consumes_action());
}
#[test]
fn exhausted_fallback_appropriate_and_legacy_deadline_never_blocks(){
    let mut r=room();r.inventory.push(amulet(1.0));a::initialize_item(&mut r.inventory[0]).unwrap();
    let state=r.inventory[0].amulet_state.as_mut().unwrap();state.energy=0.0;state.ready_at_half_tick=u64::MAX;
    r.player.abilities.push("kative-appropriate".into());use_item(&mut r,1.0);
    assert_eq!(r.player.energy,20.0);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.0);
}
#[test]
fn raw_save_and_snapshot_restore_energy_and_shared_credit_together(){
    let mut r=room();r.player.abilities.push("isheilah-spellslinger".into());r.inventory.push(amulet(1.0));
    use_item(&mut r,1.0);
    let saved=serde_json::to_string(&r).unwrap();let mut loaded:Run=serde_json::from_str(&saved).unwrap();
    let snapshot=Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();
    loaded.inventory[0].amulet_state.as_mut().unwrap().energy=0.0;loaded.player.amulet_ledger=Field::Missing;
    snapshot.restore(&mut loaded).unwrap();
    assert_eq!(loaded.inventory[0].amulet_state.as_ref().unwrap().energy,30.0);
    assert!(a::plan_item_activation(&loaded,1.0,Some(&spec())).unwrap().consumes_action());
    assert!(a::validate_run(&loaded).is_ok());
}
#[test]
fn invalid_item_state_and_old_revision_are_rejected_without_mutation(){
    let mut r=room();r.inventory.push(amulet(1.0));a::initialize_item(&mut r.inventory[0]).unwrap();
    for energy in [-1.0,51.0,f64::INFINITY] {
        r.inventory[0].amulet_state.as_mut().unwrap().energy=energy;assert!(a::validate_run(&r).is_err());
    }
    r.inventory[0].amulet_state.as_mut().unwrap().energy=50.0;
    r.inventory[0].amulet_state.as_mut().unwrap().instance_id=2;assert!(a::validate_run(&r).is_err());
    r.inventory[0].amulet_state.as_mut().unwrap().instance_id=1;r.simulation_revision=49.0.into();assert!(a::validate_run(&r).is_err());
}
#[test]
fn partial_item_energy_splits_and_combined_insufficiency_rolls_back() {
    let mut r=room();r.inventory.push(amulet(1.));a::initialize_item(&mut r.inventory[0]).unwrap();
    r.inventory[0].amulet_state.as_mut().unwrap().energy=7.;r.player.energy=13.;
    let p=a::plan_item_activation(&r,1.,Some(&spec())).unwrap();
    assert_eq!((p.amulet_energy_spent(),p.player_energy_spent()),(7.,13.));
    a::commit_item_activation(&mut r,1.,&p,Some(&spec())).unwrap();
    assert_eq!((r.inventory[0].amulet_state.as_ref().unwrap().energy,r.player.energy),(0.,0.));
    r.inventory[0].amulet_state.as_mut().unwrap().energy=7.;r.player.energy=12.;
    let before=r.clone();assert_eq!(a::plan_item_activation(&r,1.,Some(&spec())),Err(a::AdmissionError::InsufficientPlayerEnergy));assert_eq!(r,before);
    r.player.energy=30.;
    for flag in [Field::Missing,Field::Null,Field::Value(String::new()),"amulet".to_string().into(),"ring1".to_string().into()] {
        r.inventory[0].equipped=flag;
        let before=r.clone();let plan=a::plan_item_activation(&r,1.,Some(&spec())).unwrap();
        assert_eq!((plan.amulet_energy_spent(),plan.player_energy_spent()),(7.,13.));
        assert_eq!(r,before,"planning ignores historical flags without rewriting them");
    }
    r.inventory[0].kind="ring".into();
    let before=r.clone();assert_eq!(a::plan_item_activation(&r,1.,Some(&spec())),Err(a::AdmissionError::NotAmulet));assert_eq!(r,before);
}

#[test]
fn every_historical_equipped_flag_allows_carried_amulet_commit() {
    for flag in [Field::Missing,Field::Null,Field::Value(String::new()),
        "amulet".to_string().into(),"ring2".to_string().into()] {
        let mut r=room();let mut item=amulet(1.);item.equipped=flag.clone();r.inventory.push(item);
        use_item(&mut r,1.);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,30.);
        assert_eq!(r.inventory[0].equipped,flag);assert_eq!(r.player.energy,30.);
    }
}
