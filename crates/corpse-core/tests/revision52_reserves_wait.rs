//! Focused recovered-source tests, not complete-source or platform certification.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine, equipment, storage, temporal,
    revision50_ability_energy as abilities, revision50_amulets as amulets,
    revision50_amulet_effects as effects, revision52_reserves as reserves};
const ABILITY: &str = "anavani-illuminate";
const REACTIONS: [&str; 2] = ["isheilah-quick-hand", "isheilah-are-you-sure-about-that"];
fn room(revision:f64) -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();
    r.player.x=10.;r.player.y=10.;r.player.hp=100.;r.player.max_hp=100.;
    r.player.stats=Stats {accuracy:100.,perception:300.,max_energy:100.,..Default::default()};
    r.player.energy=40.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.amulet_ledger=Field::Missing;r.player.reaping_ledger=Field::Missing;
    r.inventory.clear();r.actors=Default::default();r.ground.clear();
    for depth in 1..=r.floors.len() {r.actors_at_mut(depth as f64);r.ground_at_mut(depth as f64);}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.phase="normal".to_string().into();r.next_id=1000.;r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;f.vents=Field::Missing;
    f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    r
}
fn reserve(r:&Run,id:&str)->abilities::AbilityReserve {r.player.ability_reserves.as_ref().unwrap()[id]}
fn amulet(energy:f64)->Item {
    let mut item=effects::make_amulet(effects::Effect::Firebolt,900.).unwrap();
    item.amulet_state.as_mut().unwrap().energy=energy;item
}
#[test]
fn revision_specific_acquisition_refill_and_reacquisition_preserve_other_pools() {
    for (revision,cap) in [(50.,50.),(51.,50.),(52.,10.)] {
        let mut r=room(revision);r.player.abilities=vec![ABILITY.into()];
        abilities::reconcile_acquired(&mut r).unwrap();
        assert_eq!(reserve(&r,ABILITY),abilities::AbilityReserve{capacity:cap,energy:cap});
        r.player.ability_reserves.as_mut().unwrap()[ABILITY].energy=2.;
        abilities::reconcile_acquired(&mut r).unwrap();assert_eq!(reserve(&r,ABILITY).energy,2.);
        let mut item=Item{id:900.,kind:"amulet".into(),..Default::default()};
        abilities::acquire_item_for_revision(&mut r.player,&mut item,revision).unwrap();
        assert_eq!(item.amulet_state.as_ref().unwrap().energy,cap);
        item.amulet_state.as_mut().unwrap().energy=3.;
        abilities::acquire_item_for_revision(&mut r.player,&mut item,revision).unwrap();
        assert_eq!(item.amulet_state.as_ref().unwrap().energy,3.);
        r.inventory.push(item);r.exp=2.0f64.powf(r.level);r.gods.clear();
        corpse_core::progression::level_up(&mut r);
        assert_eq!(reserve(&r,ABILITY),abilities::AbilityReserve{capacity:cap,energy:cap});
        assert_eq!(r.player.energy,40.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,3.);
    }
}
#[test]
fn early_and_late_generation_and_special_rewards_use_ten_without_rng_changes() {
    for depth in [1,2,4,5,20,40] {
        let mut a=room(51.);let mut b=room(52.);
        let mut ar=corpse_core::population::population_random(&format!("reserve:{depth}"));let mut br=ar.clone();
        let aa=equipment::generate_floor_loot_for_run(&mut a,depth,&mut ar,&[],6).unwrap();
        let bb=equipment::generate_floor_loot_for_run(&mut b,depth,&mut br,&[],6).unwrap();
        assert_eq!(aa.len(),bb.len());
        for (old,new) in aa.iter().zip(&bb) {
            assert_eq!((&old.kind,&old.effect,old.id),(&new.kind,&new.effect,new.id));
            if new.kind=="amulet" {assert_eq!(old.amulet_state.as_ref().unwrap().energy,50.);assert_eq!(new.amulet_state.as_ref().unwrap().energy,10.);}
        }
        assert_eq!(ar.random(),br.random());
    }
    for seed in 0..32 {
        let mut rng=corpse_core::population::population_random(&format!("special:{seed}"));
        let item=equipment::make_godbeast_permanent_revision(10,900.,&mut rng,52.).unwrap();
        if item.kind=="amulet" {assert_eq!(item.amulet_state.as_ref().unwrap().energy,10.);}
    }
}
#[test]
fn raw_stale_ability_reserves_are_capped_for_admission_spend_and_future() {
    let mut r=room(52.);abilities::acquire(&mut r.player,&[ABILITY.into(),abilities::FUTURE_ID.into()]).unwrap();
    let player=r.player.id;r.player.energy=4.;
    assert!(!abilities::can_pay(&r,player,ABILITY,15.));
    let before=r.clone();assert!(!abilities::spend(&mut r,player,ABILITY,15.));assert_eq!(r,before);
    assert!(abilities::spend(&mut r,player,ABILITY,14.));
    assert_eq!((reserve(&r,ABILITY).capacity,reserve(&r,ABILITY).energy,r.player.energy),(10.,0.,0.));
    let p=abilities::plan_future_payment_for_revision(&r.player,52.).unwrap();
    assert_eq!((p.reserve_energy_spent(),p.actor_energy_spent()),(10.,0.));
    abilities::commit_payment(&p,r.player.ability_reserves.as_mut().unwrap(),&mut r.player.energy).unwrap();
    assert_eq!(reserve(&r,abilities::FUTURE_ID),abilities::AbilityReserve{capacity:10.,energy:0.});
}
#[test]
fn amulet_activation_uses_ten_then_player_and_preserves_cost_discounts_and_action_rules() {
    for initial in [0.,5.,10.,50.] {
        let mut r=room(52.);r.inventory.push(amulet(initial));
        let spec=effects::item_spec(&r.inventory[0]);
        let p=amulets::plan_item_activation(&r,900.,spec.as_ref()).unwrap();
        assert_eq!(p.amulet_energy_spent(),initial.min(10.));
        assert_eq!(p.player_energy_spent(),20.-initial.min(10.));assert!(p.consumes_action());
        amulets::commit_item_activation(&mut r,900.,&p,spec.as_ref()).unwrap();
        assert_eq!(r.player.energy,40.-20.+initial.min(10.));
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    }
    let mut r=room(52.);r.inventory.push(amulet(50.));r.player.energy=9.;
    let spec=effects::item_spec(&r.inventory[0]);let before=r.clone();
    assert!(amulets::plan_item_activation(&r,900.,spec.as_ref()).is_err());assert_eq!(r,before);
    r.player.abilities=vec!["kative-appropriate".into(),"isheilah-spellslinger".into()];
    let p=amulets::plan_item_activation(&r,900.,spec.as_ref()).unwrap();
    assert_eq!((p.amulet_energy_spent(),p.player_energy_spent()),(10.,0.));assert!(!p.consumes_action());
    amulets::commit_item_activation(&mut r,900.,&p,spec.as_ref()).unwrap();
    r.player.energy=10.;let p=amulets::plan_item_activation(&r,900.,spec.as_ref()).unwrap();assert!(p.consumes_action());
}
#[test]
fn actual_amulet_command_and_ability_activation_keep_authored_costs() {
    let mut r=room(52.);r.inventory.push(amulet(50.));
    assert_eq!(effects::command(&mut r,&Command::UseAt{id:900.,x:20.,y:10.}).unwrap(),Some(true));
    assert_eq!(r.player.energy,30.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    // Sparkle keeps its 20-energy activation and visible target requirement.
    let mut r=room(52.);r.player.abilities=vec![ABILITY.into()];abilities::reconcile_acquired(&mut r).unwrap();
    assert!(!corpse_core::abilities::activate(&mut r,ABILITY,&Field::Missing,&15.0.into(),&10.0.into()).unwrap(), "Sparkle stays a free activation");
    assert_eq!(reserve(&r,ABILITY).energy,0.);assert_eq!(r.player.energy,30.);
}
#[test]
fn storage_engine_and_snapshot_boundaries_cap_old_r52_without_refill() {
    for revision in [51.,52.] {
        let mut r=room(revision);abilities::acquire(&mut r.player,&[ABILITY.into()]).unwrap();r.inventory.push(amulet(45.));
        let raw=storage::serialize(&r).unwrap();let canonical=storage::from_canonical(&raw).unwrap();
        assert_eq!(reserve(&canonical,ABILITY).energy,50.,"canonical fixture reader stays exact");
        for out in [storage::deserialize(&raw).unwrap(),storage::deserialize_compatible(&raw).unwrap(),temporal::Engine::new(r.clone()).unwrap().run] {
            assert_eq!(reserve(&out,ABILITY).energy,if revision==52.{10.}else{50.});
            assert_eq!(out.inventory[0].amulet_state.as_ref().unwrap().energy,if revision==52.{10.}else{45.});
            assert_eq!(out.player.energy,r.player.energy);assert_eq!(out.rng,r.rng);
        }
        let snapshot=temporal::Snapshot::capture(&r);r.player.energy=7.;snapshot.restore(&mut r).unwrap();
        assert_eq!(reserve(&r,ABILITY).energy,if revision==52.{10.}else{50.});
        r.player.ability_reserves.as_mut().unwrap()[ABILITY].energy=2.;r.inventory[0].amulet_state.as_mut().unwrap().energy=3.;
        reserves::normalize_run(&mut r).unwrap();assert_eq!(reserve(&r,ABILITY).energy,2.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,3.);
        let before=r.clone();reserves::normalize_run(&mut r).unwrap();assert_eq!(before,r);
    }
}
#[test]
fn future_text_and_native_restores_cannot_reintroduce_fifty() {
    for native in [false,true] {
        let mut r=room(52.);r.player.abilities=vec![abilities::FUTURE_ID.into(),ABILITY.into()];
        let ids=r.player.abilities.clone();abilities::acquire(&mut r.player,&ids).unwrap();r.inventory.push(amulet(50.));
        let mut history=temporal::History::import(&r).unwrap();
        if native {temporal::remember_future(&mut r,&history).unwrap();}
        else {r.future=Some(storage::serialize(&r).unwrap().into());}
        r.player.energy=0.;r.inventory[0].amulet_state.as_mut().unwrap().energy=0.;
        assert!(temporal::restore_future(&mut r,&mut history).unwrap());
        assert_eq!(reserve(&r,ABILITY).energy,10.);assert_eq!(reserve(&r,ABILITY).capacity,10.);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,10.);
    }
}
#[test]
fn malformed_late_reserve_rejects_normalization_atomically() {
    let mut r=room(52.);abilities::acquire(&mut r.player,&[ABILITY.into()]).unwrap();r.inventory.push(amulet(50.));
    let mut bad=r.player.clone();bad.id=901.;bad.ability_reserves.as_mut().unwrap()[ABILITY].energy=-1.;
    r.actors_at_mut(r.depth).push(bad);let before=r.clone();
    assert!(reserves::normalize_run(&mut r).is_err());assert_eq!(r,before);
}
#[test]
fn reaping_caps_at_ten_and_waits_do_not_refill_reserves() {
    let mut r=room(52.);r.inventory.push(amulet(9.5));
    r.inventory.push(Item{id:901.,kind:"ring".into(),effect:"reaping".to_string().into(),..Default::default()});
    let source=r.player.clone();let target=Actor{id:902.,state:"hunting".into(),hp:1000.,max_hp:1000.,..Default::default()};
    amulets::reap_direct_hit(&mut r,&source,&target,100.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,10.);
    amulets::reap_direct_hit(&mut r,&source,&target,100.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,10.);
    r.inventory[0].amulet_state.as_mut().unwrap().energy=3.;
    r.player.abilities=vec![ABILITY.into()];abilities::reconcile_acquired(&mut r).unwrap();r.player.ability_reserves.as_mut().unwrap()[ABILITY].energy=2.;
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,3.);assert_eq!(reserve(&r,ABILITY).energy,2.);assert_eq!(r.player.energy,40.);
}
#[test]
fn wait_refreshes_exactly_one_with_missing_empty_partial_full_and_zero_cap() {
    for phase in ["normal","haste"] {for current in [Field::Missing,0.0.into(),1.0.into(),2.0.into(),3.0.into()] {
        let mut r=room(52.);r.phase=phase.to_string().into();r.player.abilities=REACTIONS.map(str::to_string).to_vec();
        abilities::reconcile_acquired(&mut r).unwrap();r.player.reactions=current;
        let expected=(r.player.reactions.value_or(0.)+1.).min(2.);let tick=r.tick;
        engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.player.reactions.value_or(-1.),expected);assert_eq!(r.tick,tick+1.);
    }}
    let mut r=room(52.);r.player.reactions=Field::Missing;engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.player.reactions.value_or(-1.),0.);
    let mut r=room(51.);r.player.abilities=REACTIONS.map(str::to_string).to_vec();abilities::reconcile_acquired(&mut r).unwrap();
    r.player.reactions=0.0.into();engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.player.reactions.value_or(-1.),2.);
}
#[test]
fn movement_haste_and_offscreen_ticks_do_not_create_reactions() {
    let mut r=room(52.);r.player.abilities=REACTIONS.map(str::to_string).to_vec();abilities::reconcile_acquired(&mut r).unwrap();
    r.player.reactions=0.0.into();r.player.conditions.insert("hasted".into(),5.);r.phase="haste".to_string().into();
    engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();assert_eq!(r.player.reactions.value_or(-1.),0.);
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.player.reactions.value_or(-1.),1.);
    let mut floor=(**r.floors.first().unwrap()).clone();floor.depth=2.;floor.last_tick=r.tick-1.;floor.dirty=true;
    r.floors.push(floor.into());r.ground_at_mut(2.);
    let mut actor=Actor{id:901.,x:10.,y:10.,hp:100.,max_hp:100.,state:"sleeping".into(),reactions:0.0.into(),..Default::default()};
    actor.abilities=REACTIONS.map(str::to_string).to_vec();r.actors_at_mut(2.).push(actor);
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(r.actors_at(2.)[0].reactions.value_or(-1.),0.);assert_eq!(r.player.reactions.value_or(-1.),1.);
}

#[test]
fn shared_bodies_cannot_spend_uncapped_copied_reserves() {
    for dream in [false,true] {
        let mut r=room(52.);abilities::acquire(&mut r.player,&[ABILITY.into()]).unwrap();r.player.energy=8.;
        let mut body=Actor{id:901.,hp:10.,max_hp:10.,energy:100.,..Default::default()};
        body.conditions.insert(if dream {"phantasmiiBody"}else{"halfHereClone"}.into(),1.);
        body.conditions.insert("halfHereOwner".into(),r.player.id);
        abilities::acquire(&mut body,&[ABILITY.into()]).unwrap();r.actors_at_mut(r.depth).push(body);
        if dream {r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,body_id:Some(901.),..Default::default()}.into();}
        assert!(!abilities::can_pay(&r,901.,ABILITY,19.));
        assert!(abilities::spend(&mut r,901.,ABILITY,18.));
        assert_eq!((reserve(&r,ABILITY).energy,r.player.energy),(0.,0.));
        assert_eq!(engine::actor(&r,901.).unwrap().energy,0.);
        assert!(!abilities::spend(&mut r,901.,ABILITY,1.));
    }
}
#[test]
fn normalization_covers_offscreen_carried_pilots_ground_and_item_memory() {
    use indexmap::IndexMap;
    let mut r=room(52.);abilities::acquire(&mut r.player,&[ABILITY.into()]).unwrap();
    let mut actor=Actor{id:901.,hp:100.,max_hp:100.,state:"sleeping".into(),..Default::default()};
    abilities::acquire(&mut actor,&[ABILITY.into()]).unwrap();
    actor.copied_inventory=vec![amulet(50.)].into();
    let mut pilot=actor.clone();pilot.id=902.;
    actor.monster_state.get_or_insert_with(Default::default).pilot=Box::new(pilot).into();
    actor.monster_state.as_mut().unwrap().carried_items=vec![amulet(35.)].into();
    r.actors_at_mut(2.).push(actor);
    r.ground_at_mut(2.).push(corpse_core::GroundItem{x:10.,y:10.,item:amulet(40.),..Default::default()});
    r.remembered_items=IndexMap::from([("2".into(),vec![corpse_core::RunRememberedItemsEntryEntry{x:10.,y:10.,item:amulet(45.)}])]).into();
    r.detected_items=IndexMap::from([("2".into(),vec![corpse_core::RunDetectedItemsEntryEntry{x:10.,y:10.,item:amulet(46.)}])]).into();
    let rng=r.rng;let energy=r.player.energy;
    reserves::normalize_run(&mut r).unwrap();let actor=&r.actors_at(2.)[0];
    assert_eq!(actor.ability_reserves.as_ref().unwrap()[ABILITY].capacity,10.);
    assert_eq!(actor.copied_inventory.as_ref().unwrap()[0].amulet_state.as_ref().unwrap().energy,10.);
    let state=actor.monster_state.as_ref().unwrap();
    assert_eq!(state.carried_items.as_ref().unwrap()[0].amulet_state.as_ref().unwrap().energy,10.);
    assert_eq!(state.pilot.as_ref().unwrap().ability_reserves.as_ref().unwrap()[ABILITY].energy,10.);
    assert_eq!(r.ground["2"][0].item.amulet_state.as_ref().unwrap().energy,10.);
    assert_eq!(r.remembered_items.as_ref().unwrap()["2"][0].item.amulet_state.as_ref().unwrap().energy,10.);
    assert_eq!(r.detected_items.as_ref().unwrap()["2"][0].item.amulet_state.as_ref().unwrap().energy,10.);
    assert_eq!((r.rng,r.player.energy),(rng,energy));
}
#[test]
fn missing_amulet_state_projects_ten_without_mutating_admission() {
    let mut r=room(52.);let mut item=amulet(50.);item.amulet_state=Field::Missing;r.inventory.push(item);
    let before=r.clone();let spec=effects::item_spec(&r.inventory[0]);
    let p=amulets::plan_item_activation(&r,900.,spec.as_ref()).unwrap();assert_eq!(r,before);
    assert_eq!((p.amulet_energy_spent(),p.player_energy_spent()),(10.,10.));
    amulets::commit_item_activation(&mut r,900.,&p,spec.as_ref()).unwrap();
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);assert_eq!(r.player.energy,30.);
}

#[test]
fn real_starter_shell_uses_revision_specific_reserves() {
    for (revision,capacity) in [(50.,50.),(51.,50.),(52.,10.)] {
        let r=corpse_core::startup::new_run_shell_revision("0-5-11:reserve-starters","Blessed",false,revision).unwrap();
        assert!(!r.player.abilities.is_empty());
        for id in &r.player.abilities {assert_eq!(reserve(&r,id),abilities::AbilityReserve{capacity,energy:capacity});}
    }
}
#[test]
fn slowed_wait_still_recovers_only_one_despite_extra_world_upkeep() {
    let mut r=room(52.);r.player.abilities=REACTIONS.map(str::to_string).to_vec();
    abilities::reconcile_acquired(&mut r).unwrap();r.player.reactions=0.0.into();r.player.conditions.insert("slowed".into(),5.);
    let tick=r.tick;engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert_eq!(r.tick,tick+2.);assert_eq!(r.player.reactions.value_or(-1.),1.);
}
