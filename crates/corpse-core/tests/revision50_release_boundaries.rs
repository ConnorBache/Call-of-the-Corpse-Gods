//! Persisted identity boundaries; production admission is covered by revision50_admission.rs.
use corpse_core::{Item, progression, storage};
use serde_json::{Value, json};

fn room(revision: f64) -> Value {
    let mut v: Value = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    v["simulationRevision"] = json!(revision);
    v["generatedAbilityGrants"] = json!([]);
    v["inventory"] = json!([]);
    for floor in v["actors"].as_object_mut().unwrap().values_mut() { *floor = json!([]); }
    for floor in v["ground"].as_object_mut().unwrap().values_mut() { *floor = json!([]); }
    v
}
fn ring(id: f64) -> Value {
    serde_json::to_value(Item {id, name:"Ring of Light".into(), kind:"ring".into(),
        effect:"light".to_string().into(), ..Default::default()}).unwrap()
}
fn valid(v: &Value) -> bool { storage::deserialize(&serde_json::to_string(v).unwrap()).is_ok() }

#[test]
fn starter_replacements_preserve_every_position_and_weight() {
    let identities: Vec<Value> = serde_json::from_str(include_str!("../catalog/ability-identities-v50.json")).unwrap();
    for god in 0..12 {
        let old = progression::starter_abilities(god).unwrap();
        assert_eq!(progression::starter_abilities_for_revision(god,46.).unwrap(),old);
        let new = progression::starter_abilities_for_revision(god,50.).unwrap();
        assert_eq!(old.len(),new.len());
        for (old,new) in old.iter().zip(new) {
            let identity = identities.iter().find(|row| row["legacyId"] == old.as_str()).unwrap();
            assert_eq!(identity["revision50Id"],new);
            assert!(progression::definition_for_revision(50.,&new).is_some());
        }
    }
    assert!(corpse_core::startup::new_run_shell_revision("0-1-10:admitted", "Avatar", false, 50.).is_ok());
}

#[test]
fn all_72_current_ids_validate_as_generated_primary_and_equipment_grants() {
    for ability in progression::catalog_for_revision(50.) {
        let mut v = room(50.);
        v["generatedAbilityGrants"] = json!([ability.id]);
        v["player"]["equipmentAbilities"] = json!([ability.id]);
        let mut item = ring(900.); item["grantedAbility"] = json!(ability.id);
        v["inventory"] = json!([item]);
        assert!(valid(&v),"{}",ability.id);
    }
}

#[test]
fn additional_grants_validate_without_losing_primary_or_accepting_duplicates() {
    let mut v = room(50.); let mut item = ring(900.);
    item["grantedAbility"] = json!("numo-distract");
    item["additionalGrantedAbilities"] = json!(["shrail-execute","kative-cogworks"]);
    item["tinkered"] = json!(true); v["inventory"] = json!([item]);
    assert!(valid(&v));
    let loaded = storage::deserialize(&v.to_string()).unwrap();
    assert_eq!(loaded.inventory[0].additional_granted_abilities,vec!["shrail-execute","kative-cogworks"]);
    for bad in [json!(["unknown:power"]),json!(["numo-distract"]),json!(["shrail-execute","shrail-execute"]),Value::Null] {
        let mut bad_run=v.clone();bad_run["inventory"][0]["additionalGrantedAbilities"]=bad;assert!(!valid(&bad_run));
    }
    v["simulationRevision"]=json!(46.);assert!(!valid(&v));
}

#[test]
fn current_grants_are_revision_scoped_unknown_ids_fail_and_old_provenance_survives() {
    for field in ["generated", "equipment", "primary"] {
        let mut v=room(50.);v["inventory"]=json!([ring(900.)]);
        let put=|v:&mut Value,id:&str|match field {
            "generated"=>v["generatedAbilityGrants"]=json!([id]),
            "equipment"=>v["player"]["equipmentAbilities"]=json!([id]),
            _=>v["inventory"][0]["grantedAbility"]=json!(id),
        };
        put(&mut v,"shrail-execute");assert!(valid(&v));
        v["simulationRevision"]=json!(46.);assert!(!valid(&v));
        v["simulationRevision"]=json!(50.);put(&mut v,"unknown:power");assert!(!valid(&v));
        if field != "generated" {put(&mut v,"shrail-unyielding-assault");assert!(!valid(&v));}
    }
    let mut v=room(50.);v["generatedAbilityGrants"]=json!(["grisha-uncast"]);
    assert!(valid(&v),"historical generation is not an equipped grant");
}

#[test]
fn reserve_ids_and_numbers_validate_without_inventing_missing_capacities() {
    let mut v=room(50.);assert!(valid(&v));
    v["player"]["abilities"]=json!([]); // A dormant unequipped reserve is valid.
    v["player"]["abilityReserves"]=json!({"shrail-execute":{"capacity":37.,"energy":12.}});
    assert!(valid(&v));
    for reserves in [json!({"unknown:power":{"capacity":37.,"energy":12.}}),
        json!({"shrail-execute":{"capacity":37.,"energy":38.}}),
        json!({"shrail-execute":{"capacity":-1.,"energy":0.}}),
        json!({"shrail-execute":{"capacity":37.,"energy":-1.}}),Value::Null] {
        let mut bad=v.clone();bad["player"]["abilityReserves"]=reserves;assert!(!valid(&bad));
    }
    v["simulationRevision"]=json!(46.);assert!(!valid(&v));
}

#[test]
fn copied_trait_reserves_and_nested_grants_use_their_own_context() {
    let mut v=room(50.);
    let trait_id=corpse_core::enemy_traits::catalog().abilities.iter()
        .find(|(_,a)|a.passive).unwrap().0.clone();
    v["player"]["abilities"]=json!([trait_id]);
    v["player"]["abilityReserves"]=json!({(trait_id.clone()):{"capacity":10.,"energy":3.}});
    assert!(valid(&v));
    let mut item=ring(900.);item["additionalGrantedAbilities"]=json!(["kative-cogworks"]);
    v["player"]["copiedInventory"]=json!([item]);assert!(valid(&v));
    v["player"]["copiedInventory"][0]["additionalGrantedAbilities"]=json!([trait_id]);assert!(!valid(&v));
}

#[test]
fn r50_worn_flags_never_bypass_independent_jewelry_capacity() {
    for kind in ["ring","amulet"] {
        for count in [6,101] {
            let mut v=room(50.);
            v["player"]["abilities"]=json!([]);
            let items:Vec<_>=(0..count).map(|n| {
                let mut i=ring(900.+n as f64);i["kind"]=json!(kind);
                i["equipped"]=json!(if kind=="ring" {"ring1"} else {"amulet"});i
            }).collect();
            v["inventory"]=json!(items);
            let raw=v.to_string();
            assert!(storage::deserialize(&raw).is_err());
            assert!(storage::deserialize_compatible(&raw).is_err());
            assert!(storage::from_canonical(&raw).is_err());
            assert_eq!(v.to_string(),raw,"invalid inventory is rejected, never repaired");
        }
        let mut v=room(50.);v["player"]["abilities"]=json!([]);
        let mut stack=ring(900.);stack["kind"]=json!(kind);stack["quantity"]=json!(6);
        stack["equipped"]=json!(if kind=="ring" {"ring1"} else {"amulet"});
        v["inventory"]=json!([stack]);assert!(!valid(&v),"stack quantity counts");
        v["inventory"][0]["quantity"]=json!(1);assert!(valid(&v));
    }
}

#[test]
fn unlimited_special_inventory_roundtrips_without_loosening_finite_categories() {
    let mut v=room(50.);
    let mut items:Vec<Item>=(0..120).map(|n|corpse_core::equipment::make_equipment("Bow",1000.+n as f64).unwrap()).collect();
    items.extend([corpse_core::equipment::make_arrows(3000.,10000.),corpse_core::borge::shard(3001.),
        corpse_core::phantasmii_generation::dagger(3002.),corpse_core::phantasmii_generation::shard(3003.),
        corpse_core::ahvamitt::mask_for_revision(3004.,50.),corpse_core::royal_honey::item(3005.)]);
    v["inventory"]=json!(items);assert!(valid(&v));
    let loaded=storage::deserialize(&serde_json::to_string(&v).unwrap()).unwrap();
    assert_eq!(loaded.inventory,items);
    assert!(corpse_core::inventory::is_arrow(&loaded.inventory[120]));
    assert!(corpse_core::borge::is_shard(&loaded.inventory[121]));
    assert!(corpse_core::phantasmii_generation::is_dagger(&loaded.inventory[122]));
    assert!(corpse_core::phantasmii_generation::is_shard(&loaded.inventory[123]));
    assert!(corpse_core::ahvamitt::has_mask(&loaded));
    for kind in ["ring","amulet","potion","heart"] {
        let mut full=v.clone();
        for n in 0..5 {
            let item=Item{id:4000.+n as f64,name:kind.into(),kind:kind.into(),..Default::default()};
            full["inventory"].as_array_mut().unwrap().push(json!(item));
        }
        assert!(valid(&full),"five {kind}");
        full["inventory"].as_array_mut().unwrap().push(json!(Item{id:4005.,name:kind.into(),kind:kind.into(),..Default::default()}));
        assert!(!valid(&full),"six {kind}");
    }
    v["simulationRevision"]=json!(46.);assert!(!valid(&v));
}

#[test]
fn r50_all_five_sections_roundtrip_together_without_equipped_normalization() {
    let mut v=room(50.);v["player"]["abilities"]=json!([]);
    let mut items=Vec::new();
    for (category,kind) in ["ring","amulet","potion","heart"].iter().enumerate() {
        for n in 0..5 {
            let mut item=Item{id:1000.+(category*10+n) as f64,name:(*kind).into(),kind:(*kind).into(),..Default::default()};
            if *kind=="ring" {item.effect="light".to_string().into();}
            if matches!(*kind,"ring"|"amulet") && n%2==0 {item.equipped=(*kind).to_string().into();}
            if *kind=="amulet" {
                corpse_core::revision50_amulets::initialize_item(&mut item).unwrap();
                item.amulet_state.as_mut().unwrap().energy=n as f64;
            }
            items.push(item);
        }
    }
    items.extend((0..120).map(|n|corpse_core::equipment::make_equipment("Bow",2000.+n as f64).unwrap()));
    v["inventory"]=json!(items);
    let raw=v.to_string();let loaded=storage::deserialize(&raw).unwrap();
    assert_eq!(loaded.inventory,items);
    assert_eq!(corpse_core::revision50_inventory::limited_occupancy(&loaded),20.);
    assert_eq!(corpse_core::revision50_rings::count_effect(&loaded,&loaded.player,"light"),5);
    assert_eq!(storage::deserialize(&storage::serialize(&loaded).unwrap()).unwrap().inventory,items);
    for kind in ["ring","amulet","potion","heart"] {
        let mut bad=v.clone();bad["inventory"].as_array_mut().unwrap().push(json!(Item{id:3000.,name:kind.into(),kind:kind.into(),..Default::default()}));
        assert!(!valid(&bad),"overflow in {kind} cannot borrow another category's slots");
    }
}

#[test]
fn legacy_worn_jewelry_keeps_original_entry_bound_and_slots() {
    let mut v=room(46.);
    v["inventory"]=json!((0..6).map(|n| {
        let mut item=ring(900.+n as f64);item["equipped"]=json!(if n%2==0 {"ring1"} else {"ring2"});item
    }).collect::<Vec<_>>());
    let original=v["inventory"].clone();let loaded=storage::deserialize(&v.to_string()).unwrap();
    assert_eq!(serde_json::to_value(&loaded.inventory).unwrap(),original);
    v["inventory"]=json!((0..101).map(|n|ring(900.+n as f64)).collect::<Vec<_>>());
    assert!(!valid(&v));
}
