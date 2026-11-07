use super::*;

// Exact pre-jewelry-change creator, retained as a test oracle.
fn frozen_legacy_create(
    kind: &str, rng: &mut GenerationRng, next_id: &mut f64,
    depth: u32, owned: &[String], revision: f64,
) -> Result<Item, String> {
    let names: Vec<_> = catalog()
        .iter()
        .filter(|(n, d)| n.as_str() != "Bow" && d["kind"] == kind)
        .map(|(n, _)| n.as_str())
        .collect();
    if names.is_empty() {
        return Err(format!("unknown equipment kind {kind}"));
    }
    let name = pick(&names, rng);
    let id = *next_id;
    *next_id += 1.0;
    let mut item = make_equipment(name, id).map_err(|e| e.0)?;
    if matches!(kind, "ring" | "amulet") && rng.random() < 0.5 {
        let pool: Vec<_> = crate::progression::catalog_for_revision(revision)
            .iter()
            .filter(|a| !owned.contains(&a.id))
            .collect();
        if !pool.is_empty() {
            let a = pick(&pool, rng);
            item.granted_ability = a.id.clone().into();
            item.name += &format!(" of {}", a.name);
            if depth <= 15 {
                let k = *pick(&STATS, rng);
                let penalty = scaled(k, magnitude(depth, rng));
                add(&mut item, k, -penalty);
                item.name += &format!(", Cursed -{penalty} {k}");
            }
            return Ok(item);
        }
    }
    if rng.random() < 0.5 {
        modify_equipment(&mut item, depth, rng)
    }
    Ok(item)
}

#[test]
fn property_check_requires_real_effect_not_field_presence() {
    let mut item=make_equipment("Ring",1.).unwrap();
    assert!(!jewelry_has_property(&item));
    item.bonus=PartialStats::default().into();item.granted_ability=String::new().into();
    assert!(!jewelry_has_property(&item));
    item.bonus.as_mut().unwrap().offense=0.0.into();
    assert!(!jewelry_has_property(&item));
    item.bonus.as_mut().unwrap().offense=(-1.0).into();
    assert!(jewelry_has_property(&item),"a real curse alone qualifies");
    item.bonus=Field::Missing;item.granted_ability="anavani-illuminate".to_owned().into();
    assert!(jewelry_has_property(&item));
}
#[test]
fn old42_exact_items_and_rng_match_frozen_algorithm_and43_keeps_successful_rolls() {
    let mut repaired=0;
    for seed in 0..256 {for depth in [1,5,15,16,30,40] {for kind in KINDS {
        let mut a=GenerationRng::new(seed);let mut b=a;let mut c=a;
        let(mut aid,mut bid,mut cid)=(100.,100.,100.);
        let expected=frozen_legacy_create(kind,&mut a,&mut aid,depth,&[],42.).unwrap();
        let old=create_equipment_revision(kind,&mut b,&mut bid,depth,&[],42.).unwrap();
        assert_eq!(serde_json::to_vec(&old).unwrap(),serde_json::to_vec(&expected).unwrap());
        assert_eq!((b.state,bid),(a.state,aid));
        let new=create_equipment_revision(kind,&mut c,&mut cid,depth,&[],43.).unwrap();
        if !matches!(kind,"ring"|"amulet") || jewelry_has_property(&old) {
            assert_eq!(new,old,"seed={seed} depth={depth} kind={kind}");
            assert_eq!((c.state,cid),(b.state,bid));
        } else {assert!(jewelry_has_property(&new));repaired+=1;}
    }}}
    assert!(repaired>100);
}
#[test]
fn exhausted_ability_pool_and_floor_loot_still_have_special_jewelry() {
    let owned:Vec<_>=crate::progression::catalog_for_revision(43.).iter().map(|a|a.id.clone()).collect();
    for seed in 0..256 {for depth in [1,15,16,40] {
        for kind in ["ring","amulet"] {
            let mut rng=GenerationRng::new(seed);let mut id=100.;
            let item=create_equipment_revision(kind,&mut rng,&mut id,depth,&owned,43.).unwrap();
            assert!(item.granted_ability.is_missing());assert!(jewelry_has_property(&item));
        }
        let mut rng=GenerationRng::new(seed);let mut id=100.;
        for item in generate_floor_loot_revision(depth,&mut rng,&mut id,&owned,20,43.).unwrap() {
            if matches!(item.kind.as_str(),"ring"|"amulet") {assert!(jewelry_has_property(&item));}
        }
    }}
}
#[test]
fn godbeast_jewelry_already_has_modifier_and_authored_plain_items_stay_plain() {
    let mut count=0;
    for seed in 0..512 {
        let mut rng=GenerationRng::new(seed);
        let item=make_godbeast_permanent(1,100.,&mut rng).unwrap();
        if matches!(item.kind.as_str(),"ring"|"amulet") {count+=1;assert!(jewelry_has_property(&item));}
    }
    assert!(count>0);
    for name in ["Ring","Amulet"] {
        let item=make_equipment(name,100.).unwrap();assert!(!jewelry_has_property(&item));
        let loaded:Item=serde_json::from_str(&serde_json::to_string(&item).unwrap()).unwrap();
        assert_eq!(loaded,item);assert!(!jewelry_has_property(&loaded));
    }
}

#[test]
fn every_legacy_revision_preserves_frozen_draws_with_full_or_empty_ability_pool() {
    for revision in 29..=42 {for exhausted in [false,true] {
        let owned=if exhausted {crate::progression::catalog_for_revision(revision as f64).iter().map(|a|a.id.clone()).collect()}else{Vec::new()};
        for seed in 0..32 {for kind in ["ring","amulet"] {
            let mut a=GenerationRng::new(seed);let mut b=a;let(mut aid,mut bid)=(100.,100.);
            let expected=frozen_legacy_create(kind,&mut a,&mut aid,15,&owned,revision as f64).unwrap();
            let actual=create_equipment_revision(kind,&mut b,&mut bid,15,&owned,revision as f64).unwrap();
            assert_eq!(serde_json::to_vec(&actual).unwrap(),serde_json::to_vec(&expected).unwrap());
            assert_eq!((b.state,bid),(a.state,aid));
        }}
    }}
}
