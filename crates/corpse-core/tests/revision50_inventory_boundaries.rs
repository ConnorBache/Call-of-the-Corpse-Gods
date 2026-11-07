//! Carried jewelry remains active and category limits remain strict at persistence boundaries.
use corpse_core::{Field, Item, PartialStats, Run, equipment, migrations, revision50_ability_energy as energy,
    revision50_amulets as amulets, revision50_inventory as inventory, revision50_rings as rings, storage,
    temporal::{self, Engine, History, Snapshot}};

fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.inventory.clear();r.offers.clear();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;
    r.player.amulet_ledger=Field::Missing;r.player.reaping_ledger=Field::Missing;
    for actors in r.actors.values_mut() {*actors=Default::default();}
    for ground in r.ground.values_mut() {*ground=Default::default();}
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r.replay.clear();r
}
fn ring(id:f64)->Item {Item{id,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),..Default::default()}}
fn filled() -> Run {
    let mut r=room();
    for n in 0..5 {
        let mut item=ring(900.+n as f64);
        if n==0 {item.granted_ability="numo-distract".to_string().into();}
        if n%2==0 {item.equipped="ring1".to_string().into();}
        energy::acquire_item(&mut r.player,&mut item).unwrap();r.inventory.push(item);
        let mut item=Item{id:910.+n as f64,kind:"amulet".into(),name:"Amulet of Lightning".into(),effect:"lightning".to_string().into(),..Default::default()};
        energy::acquire_item(&mut r.player,&mut item).unwrap();
        item.amulet_state.as_mut().unwrap().energy=(n+1) as f64;r.inventory.push(item);
        r.inventory.push(Item{id:920.+n as f64,kind:"potion".into(),name:"Potion of Healing".into(),effect:"healing".to_string().into(),..Default::default()});
        r.inventory.push(Item{id:930.+n as f64,kind:"heart".into(),name:"Heart".into(),glyph:";".into(),..Default::default()});
    }
    r.inventory.extend((0..120).map(|n|equipment::make_equipment("Bow",1000.+n as f64).unwrap()));
    equipment::sync_ownership(&mut r).unwrap();
    r.player.ability_reserves.as_mut().unwrap()["numo-distract"].energy=7.;
    r
}
fn assert_carried(r:&Run) {
    assert!(inventory::finite_categories_fit(&r.inventory,&r.player.abilities));
    assert_eq!(inventory::limited_occupancy(r),20.);
    assert_eq!(rings::count_effect(r,&r.player,"light"),5);
    assert!(r.player.abilities.iter().any(|id|id=="numo-distract"));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["numo-distract"].energy,7.);
    let spec=amulets::AmuletSpec{capacity:50.,activation_cost:0.};
    assert!(amulets::plan_item_activation(r,910.,Some(&spec)).is_ok());
}
#[test]
fn r50_save_and_snapshot_preserve_carried_activity_flags_and_spent_charge() {
    let r=filled();storage::validate(&r).unwrap();assert_carried(&r);
    let mut loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(loaded.inventory,r.inventory);assert_carried(&loaded);
    let snapshot=Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();
    loaded.inventory.clear();equipment::sync_ownership(&mut loaded).unwrap();
    assert!(!rings::has_effect(&loaded,&loaded.player,"light"));
    snapshot.restore(&mut loaded).unwrap();
    assert_eq!(loaded.inventory,r.inventory);assert_carried(&loaded);
}
#[test]
fn r50_native_and_reloaded_future_restore_all_category_counts_and_item_charges() {
    for reload in [false,true] {
        let mut r=filled();r.player.abilities.push(energy::FUTURE_ID.into());
        energy::acquire(&mut r.player,&[energy::FUTURE_ID.into()]).unwrap();
        temporal::remember_future(&mut r,&History::default()).unwrap();
        let expected=r.inventory.clone();
        if reload {r=Engine::load(&storage::serialize(&r).unwrap()).unwrap().run;}
        r.inventory.retain(|i|i.id!=900. && i.id!=910.);
        equipment::sync_ownership(&mut r).unwrap();
        assert!(!r.player.abilities.iter().any(|id|id=="numo-distract"));
        for item in &mut r.inventory {if let Some(state)=item.amulet_state.as_mut(){state.energy=0.;}}
        assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());
        assert_eq!(r.inventory,expected);assert_carried(&r);
    }
}
#[test]
fn r50_invalid_future_jewelry_overflow_rejects_atomically_in_native_and_text_anchors() {
    for text in [false,true] {
        let mut invalid=filled();invalid.inventory.push(ring(2000.));
        invalid.inventory.last_mut().unwrap().equipped="ring1".to_string().into();
        invalid.player.abilities.push(energy::FUTURE_ID.into());
        energy::acquire(&mut invalid.player,&[energy::FUTURE_ID.into()]).unwrap();
        temporal::remember_future(&mut invalid,&History::default()).unwrap();
        let mut current=filled();current.future=invalid.future;
        if text {current=storage::deserialize(&storage::serialize(&current).unwrap()).unwrap();}
        let before=storage::serialize(&current).unwrap();
        assert!(temporal::restore_future(&mut current,&mut History::default()).is_err());
        assert_eq!(storage::serialize(&current).unwrap(),before);
        assert_carried(&current);
    }
}
#[test]
fn r50_health_migrations_count_carried_jewelry_without_rewriting_flags() {
    for (revision,equipped,expected) in [(50.,false,106.),(50.,true,106.),(46.,false,100.),(46.,true,106.)] {
        let mut r=room();r.simulation_revision=revision.into();
        r.player.stats.defense=0.;r.player.stats.max_hp=Field::Missing;r.player.base_max_hp=100.0.into();
        r.player.max_hp=100.;r.player.hp=80.;
        let mut item=ring(900.);item.bonus=PartialStats{defense:1.4.into(),..Default::default()}.into();
        if equipped {item.equipped="ring1".to_string().into();}
        r.inventory.push(item);let flags=r.inventory[0].equipped.clone();
        migrations::normalize_integer_stat_health(&mut r);
        assert_eq!(r.player.max_hp,expected);assert_eq!(r.player.hp,80.);assert_eq!(r.inventory[0].equipped,flags);
        r.player.max_hp=100.;r.divine_revision=Field::Missing;
        migrations::migrate_divine_revision(&mut r);
        assert_eq!(r.player.max_hp,expected);assert_eq!(r.inventory[0].equipped,flags);
    }
}
