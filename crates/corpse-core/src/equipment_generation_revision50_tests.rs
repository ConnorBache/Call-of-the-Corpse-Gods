use super::*;
use crate::{Command, Field, GroundItem, Run};
use std::collections::BTreeSet;

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.x = 10.; r.player.y = 10.; r.player.energy = 100.; r.next_id = 9000.;
    r.floor_mut().unwrap().lights.fill(100.);
    r.floor_mut().unwrap().seen.fill(true);
    r
}
fn named(item: &Item) {
    assert!(crate::revision50_inventory::classified(item));
    crate::equipment::validate(item).unwrap();
    if item.kind == "ring" {
        assert!(crate::revision50_rings::effect(item).is_some());
    } else if item.kind == "amulet" {
        assert!(crate::revision50_amulet_effects::item_spec(item).is_some());
        assert!(crate::revision50_amulets::valid_item_state(item, 50.));
        assert_eq!(item.amulet_state.as_ref().unwrap().energy, 50.);
        assert_eq!(item.amulet_state.as_ref().unwrap().instance_id, item.id as u64);
    } else { assert!(crate::projectiles::is_bow(item)); }
    if matches!(item.kind.as_str(), "ring" | "amulet") {
        assert!(item.identified.value_or(false));
        assert!(item.bonus.is_missing()); assert!(item.base_bonus.is_missing());
        assert!(item.granted_ability.is_missing()); assert!(item.additional_granted_abilities.is_empty());
        assert!(item.enhancements.is_empty());
    }
}
#[test]
fn uniform_named_groups_have_one_draw_fixed_effects_and_all_names_reachable() {
    for (kind, roster) in [("ring", &revision50::RINGS[..]), ("amulet", &revision50::AMULETS[..])] {
        let mut seen = BTreeSet::new();
        for seed in 0..512 {
            let mut rng = GenerationRng::new(seed);
            let mut expected = rng;
            let effect = roster[(expected.random() * roster.len() as f64).floor() as usize];
            let mut next_id = 100.;
            let item = create_equipment_revision(kind, &mut rng, &mut next_id, 1, &[], 50.).unwrap();
            named(&item);
            assert_eq!(item.effect.as_ref().unwrap(), effect);
            assert_eq!(rng.state, expected.state); assert_eq!(next_id, 101.);
            seen.insert(item.effect.as_ref().unwrap().clone());
            let mut deeper_rng = GenerationRng::new(seed); let mut deeper_id = 100.;
            assert_eq!(item, create_equipment_revision(kind, &mut deeper_rng, &mut deeper_id, 40, &["shrail-execute".into()], 50.).unwrap());
        }
        assert_eq!(seen, roster.iter().map(|s| s.to_string()).collect());
    }
}
#[test]
fn floor_loot_generates_named_items_with_unique_ids_and_deterministic_rng() {
    let mut rings = BTreeSet::new(); let mut amulets = BTreeSet::new();
    for seed in 0..128 {
        let mut rng = GenerationRng::new(seed); let mut mirrored = rng;
        let mut next_id = 100.; let mut next_mirror = next_id;
        let items = generate_floor_loot_revision(12, &mut rng, &mut next_id, &[], 20, 50.).unwrap();
        assert_eq!(items, generate_floor_loot_revision(12, &mut mirrored, &mut next_mirror, &[], 20, 50.).unwrap());
        assert_eq!(rng.state, mirrored.state); assert_eq!(next_id, next_mirror);
        assert_eq!(items.len(), 20); assert_eq!(next_id, 120.);
        assert_eq!(items.iter().map(|i| i.id as u64).collect::<BTreeSet<_>>().len(), 20);
        assert_ne!(items[0].kind, items[1].kind);
        assert!((2..=4).contains(&items.iter().filter(|i| i.kind != "potion").count()));
        for item in items.iter().filter(|i| i.kind != "potion") {
            named(item);
            if item.kind == "ring" { rings.insert(item.effect.as_ref().unwrap().clone()); }
            if item.kind == "amulet" { amulets.insert(item.effect.as_ref().unwrap().clone()); }
        }
    }
    assert_eq!(rings.len(), 14); assert_eq!(amulets.len(), 12);
}
#[test]
fn generated_jewelry_is_active_on_pickup_and_generated_amulet_applies() {
    let mut rng = GenerationRng::new(75); let mut id = 100.;
    let mut seen = BTreeSet::new(); let mut applied_lightning = false;
    for _ in 0..512 {
        for kind in ["ring", "amulet"] {
            let item = create_equipment_revision(kind, &mut rng, &mut id, 8, &[], 50.).unwrap();
            if !seen.insert(format!("{kind}:{}", item.effect.as_ref().unwrap())) { continue; }
            // Each generated identity gets a real legal pickup, rather than
            // accumulating every item behind a historical worn-capacity bypass.
            let mut r = room(); let item_id = item.id;
            let depth = r.depth; let (x,y) = (r.player.x,r.player.y);
            assert!(item.equipped.is_missing());
            r.ground_at_mut(depth).push(GroundItem { x,y,item,..Default::default() });
            assert_eq!(crate::inventory::collect_underfoot(&mut r,20.).unwrap(),1);
            assert!(crate::revision50_inventory::worn(&r.inventory[0]));
            assert!(r.inventory[0].equipped.is_missing());
            assert_eq!(crate::revision50_inventory::limited_occupancy(&r),1.);
            assert!(crate::revision50_inventory::finite_categories_fit(&r.inventory,&r.player.abilities));
            if kind == "ring" {
                assert!(crate::revision50_rings::has_effect(&r,&r.player,
                    crate::revision50_rings::effect(&r.inventory[0]).unwrap()));
            } else if r.inventory[0].effect.as_ref().is_some_and(|effect|effect=="lightning") {
                let energy = r.player.energy;
                assert_eq!(crate::inventory::command(&mut r,&Command::UseAt{id:item_id,x:11.,y:10.}).unwrap(),Some(true));
                assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,25.);
                assert_eq!(r.player.energy,energy); applied_lightning = true;
            }
        }
    }
    assert_eq!(seen.len(), 26); assert!(applied_lightning);
}
#[test]
fn starting_inventory_keeps_bow_and_ammo_without_sword_and_apply_needs_no_equipping() {
    let old = crate::startup::starting_inventory(46.).unwrap();
    assert_eq!(old.len(),3); assert_eq!(old[0].name,"Shortsword");
    assert_eq!(old[0].equipped.as_ref().unwrap(),"hand1");
    let mut r = room(); r.inventory = crate::startup::starting_inventory(50.).unwrap();
    assert_eq!(r.inventory.len(),2); assert_eq!(r.inventory[0],old[1]); assert_eq!(r.inventory[1],old[2]);
    assert!(r.inventory.iter().all(|i|i.equipped.is_missing()));
    assert_eq!(crate::inventory::carrying_capacity(&r).unwrap(),20.);
    assert_eq!(crate::inventory::command(&mut r,&Command::UseAt{id:2.,x:11.,y:10.}).unwrap(),Some(true));
    assert_eq!(crate::inventory::quantity(&r.inventory[1]),4.);
}
#[test]
fn revisioned_event_rewards_are_valid_and_preserve_every_legacy_reward_draw() {
    let mut rings=BTreeSet::new();let mut amulets=BTreeSet::new();
    for seed in 0..512 {
        let mut rng=GenerationRng::new(seed);
        let item=make_godbeast_permanent_revision(10,100.,&mut rng,50.).unwrap();named(&item);
        if item.kind=="ring" {rings.insert(item.effect.as_ref().unwrap().clone());}
        if item.kind=="amulet" {amulets.insert(item.effect.as_ref().unwrap().clone());}
        let mut old=GenerationRng::new(seed);let mut versioned=old;
        assert_eq!(make_godbeast_permanent(10,100.,&mut old).unwrap(),
            make_godbeast_permanent_revision(10,100.,&mut versioned,46.).unwrap());
        assert_eq!(old.state,versioned.state);
    }
    assert_eq!(rings.len(),14);assert_eq!(amulets.len(),12);
}
#[test]
fn actual_mercuria_reward_uses_revisioned_constructor_and_preserves_simulation_rng() {
    let mut r=room();r.next_id=200.;let old_rng=r.rng;
    r.floor_mut().unwrap().godbeast_event=crate::GodbeastEvent {
        kind:"Mercuria".into(),x:15.,y:14.,
        lever:crate::GodbeastEventLever{x:14.,y:14.}.into(),..Default::default()
    }.into();
    let mut expected_rng=crate::population::population_random(&format!("{}:floor:{}:Mercuria",r.seed,r.depth));
    let expected=make_godbeast_permanent_revision(r.depth as u32,200.,&mut expected_rng,50.).unwrap();
    assert!(crate::godbeast_interactions::pull_lever(&mut r,14.,14.).unwrap());
    let item=&r.ground.get(&r.depth.to_string()).unwrap()[0].item;
    assert_eq!(*item,expected);named(item);assert_eq!(r.next_id,201.);assert_eq!(r.rng,old_rng);
    assert!(!crate::godbeast_interactions::pull_lever(&mut r,14.,14.).unwrap());
    assert_eq!(r.ground.get(&r.depth.to_string()).unwrap().len(),1);
}
#[test]
fn actual_mercuria_lever_guarantees_one_max_tier_amulet_at_any_birth_depth() {
    let mut effects = std::collections::HashSet::new();
    let mut old_kinds = std::collections::HashSet::new();
    for depth in [5., 21.] {
        for seed in 0..20 {
            let mut run = room();
            run.simulation_revision = 52.0.into();
            run.amulet_rules_from = 0usize.into();
            run.seed = format!("mercuria-max-{seed}");
            run.depth = depth;
            run.floors.truncate(1);
            let floor = std::sync::Arc::make_mut(&mut run.floors[0]);
            floor.depth = depth;
            floor.tiles.fill("floor".into());
            floor.godbeast_event = crate::GodbeastEvent {
                kind: "Mercuria".into(), x: 15., y: 14.,
                lever: crate::GodbeastEventLever { x: 14., y: 14. }.into(), ..Default::default()
            }.into();
            run.next_id = 200.;
            run.composite_bow_generated = (seed % 2 == 0).into();
            let mut historical = run.clone();
            run.mercuria_amulet_from = 0usize.into();
            let before = (run.rng, run.composite_bow_generated.clone());
            assert!(crate::godbeast_interactions::pull_lever(&mut run, 14., 14.).unwrap());
            assert!(crate::godbeast_interactions::pull_lever(&mut historical, 14., 14.).unwrap());
            let prize = &run.ground[&depth.to_string()][0];
            let old = &historical.ground[&depth.to_string()][0].item;
            old_kinds.insert(old.kind.clone());
            if old.kind == "amulet" { assert_eq!(prize.item.effect, old.effect); }
            assert!(prize.item.name.starts_with("Tier 9 Amulet of "));
            assert_eq!((prize.item.id, prize.item.kind.as_str()), (200., "amulet"));
            assert_eq!((prize.x, prize.y), (15., 14.));
            assert_eq!(crate::revision52_amulet_tiers::profile(&run, &prize.item).unwrap().tier, crate::revision52_amulet_tiers::MAX_TIER);
            assert!(prize.item.amulet_state.as_ref().is_some());
            assert!(!crate::revision52_amulet_tiers::retired(&run, &prize.item));
            effects.insert(prize.item.effect.as_ref().unwrap().clone());
            assert_eq!((run.rng, run.composite_bow_generated.clone()), before);
            assert_eq!(run.next_id, 201.);
            assert!(!crate::godbeast_interactions::pull_lever(&mut run, 14., 14.).unwrap());
            assert_eq!(run.ground[&depth.to_string()].len(), 1);
        }
    }
    assert!(effects.len() > 1);
    assert!(old_kinds.contains("ring") && old_kinds.contains("weapon"));
}
#[test]
fn actual_sphinx_reward_branch_uses_named_permanents() {
    let fixture: serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/startup-v48.json")).unwrap();
    let case=fixture["cases"].as_array().unwrap().iter().find(|c|c["depth"]==35).unwrap();
    let mut r:Run=serde_json::from_value(case["initial"].clone()).unwrap();
    let prior:Vec<crate::Floor>=serde_json::from_value(fixture["priorFloors"].clone()).unwrap();
    let mut floors:Vec<_>=prior.into_iter().take(34).map(std::sync::Arc::new).collect();
    floors.extend(r.floors.clone());r.floors=floors;
    r.simulation_revision=50.0.into();r.inventory=crate::startup::starting_inventory(50.).unwrap();
    r.player.abilities.clear();r.player.active.clear();
    crate::startup::spawn_floor(&mut r,35).unwrap();
    let f=r.floors.iter().find(|f|f.depth==35.).unwrap();
    let room=f.sphinx_room.as_ref().expect("frozen geometry has a guarded room");
    let ground=r.ground.get("35").unwrap();
    let rewards:Vec<_>=room.reward_cells.iter().map(|at| {
        &ground.iter().find(|g|g.y*f.width() as f64+g.x==*at).unwrap().item
    }).collect();
    for i in &rewards[..2] {named(i);}
    assert_eq!(rewards[2].effect.as_ref().unwrap(),"healing");
    assert_eq!(rewards[3].effect.as_ref().unwrap(),"energy");
    assert_eq!(ground.iter().map(|g|g.item.id as u64).collect::<BTreeSet<_>>().len(),ground.len());
}
