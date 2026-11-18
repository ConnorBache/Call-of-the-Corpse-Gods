use super::*;
use crate::{Command, Stats, engine, temporal};
const ID: &str = "morae-see-the-future";
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into(); r.player.x=10.; r.player.y=10.;
    r.player.hp=202.;r.player.max_hp=202.;r.player.base_max_hp=202.0.into();
    r.player.stats=Stats { max_energy: 100., perception: 30., ..Default::default() };
    r.player.energy=100.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.inventory.clear();for a in r.actors.values_mut(){*a=Default::default();}
    for g in r.ground.values_mut(){*g=Default::default();}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.resource_history=Field::Missing;r.phase="haste".to_string().into();r.fast_action_phase=Field::Missing;
    r.next_id=1000.;r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.environment=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.speed_up=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.godbeast_event=Field::Missing;
    r
}
fn delta(r:&Run,key:&str,value:f64,max:f64)->f64 { player_change_percent(r,key,value,max).unwrap() }
fn close(a:f64,b:f64){assert!((a-b).abs()<1e-8,"{a} != {b}");}
fn ability(id:&str)->Command{Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
fn add_reserve(r:&mut Run,id:&str,energy:f64) {
    r.player.abilities.push(id.into());
    crate::revision50_ability_energy::reconcile_acquired(r).unwrap();
    r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=energy;
}
#[test]
fn resource_turn_capped_heal_net_loss_zero_death_and_current_maximum() {
    let mut r=room();r.player.hp=191.9;begin_resource_turn(&mut r);
    r.player.hp=(r.player.hp+50.).min(r.player.max_hp);close(delta(&r,"hp",r.player.hp,r.player.max_hp),5.);
    r.player.hp-=20.2;close(delta(&r,"hp",r.player.hp,r.player.max_hp),-5.);
    r.player.hp=191.9;close(delta(&r,"hp",r.player.hp,r.player.max_hp),0.);
    r.player.hp=0.;close(delta(&r,"hp",r.player.hp,r.player.max_hp),-95.);
    close(delta(&r,"hp",212.1,404.),5.); // Current capacity is the denominator.
    assert_eq!(player_change_percent(&r,"hp",1.,0.),None);
    assert_eq!(player_change_percent(&r,"hp",f64::NAN,202.),None);
}
#[test]
fn resource_turn_cost_refill_and_independent_amulet_instances() {
    let mut r=room();add_reserve(&mut r,ID,10.);
    for id in [900.,901.] {
        let mut i=Item{id,kind:"amulet".into(),effect:"lightning".to_string().into(),..Default::default()};
        crate::revision50_amulets::initialize_item_for_revision(&mut i,52.).unwrap();r.inventory.push(i);
    }
    begin_resource_turn(&mut r);let player=r.player.id;
    assert!(crate::revision50_ability_energy::spend(&mut r,player,ID,15.));
    close(delta(&r,"energy",r.player.energy,100.),-5.);
    close(delta(&r,&ability_key(ID),0.,10.),-100.);
    r.inventory[0].amulet_state.as_mut().unwrap().energy=0.;
    close(delta(&r,&amulet_key(900.),0.,10.),-100.);
    close(delta(&r,&amulet_key(901.),10.,10.),0.);
    begin_resource_turn(&mut r);
    crate::revision50_ability_energy::refill_on_level_up_for_revision(&mut r.player,52.).unwrap();
    close(delta(&r,&ability_key(ID),10.,10.),100.);
    r.inventory[0].amulet_state.as_mut().unwrap().energy=10.;
    close(delta(&r,&amulet_key(900.),10.,10.),100.);
}
#[test]
fn resource_turn_free_command_haste_and_rejected_move_preserve_baseline() {
    let mut r=room();r.player.conditions.insert("hasted".into(),5.);
    begin_resource_turn(&mut r);r.player.energy=80.;
    let history=r.resource_history.clone();let tick=r.tick;
    assert!(!engine::dispatch(&mut r,&Command::Pickup).unwrap().took_turn);
    assert_eq!(r.resource_history,history);assert_eq!(r.tick,tick);
    assert!(!engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!(r.phase.as_ref().map(String::as_str),Some("normal"));
    let start=r.resource_history.clone();r.player.energy=70.;
    assert!(engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!(r.resource_history.as_ref().unwrap().tick,start.as_ref().unwrap().tick);
    close(delta(&r,"energy",70.,100.),-10.);
    let before=r.resource_history.clone();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]="wall".into();
    assert!(!engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    assert_eq!(r.resource_history,before);
}
#[test]
fn resource_turn_save_resume_and_future_restore_exact_baselines() {
    let mut r=room();r.future_floor_rules_from=Field::Null;add_reserve(&mut r,ID,10.);begin_resource_turn(&mut r);
    r.player.hp=181.8;r.player.energy=80.;
    let history=r.resource_history.clone();let raw=crate::storage::serialize(&r).unwrap();
    let loaded=crate::storage::deserialize(&raw).unwrap();assert_eq!(loaded.resource_history,history);
    close(delta(&loaded,"hp",loaded.player.hp,202.),-10.);
    let mut e=temporal::Engine::new(r).unwrap();e.dispatch(&ability(ID)).unwrap();
    assert!(e.run.future.is_some());let paid_history=e.run.resource_history.clone();
    close(delta(&e.run,"energy",e.run.player.energy,100.),-100.);
    close(delta(&e.run,&ability_key(ID),0.,10.),-100.);
    e.dispatch(&Command::Wait).unwrap();let mut resumed=temporal::Engine::load_replay_checkpoint(&e.save().unwrap()).unwrap();
    resumed.dispatch(&ability(ID)).unwrap();assert!(resumed.run.future.is_none());
    assert_eq!(resumed.run.resource_history,paid_history);
    close(delta(&resumed.run,"energy",resumed.run.player.energy,100.),-100.);
}
#[test]
fn resource_turn_legacy_missing_entries_and_boundless_reference() {
    let mut r=room();r.player.abilities.push("grisha-boundless-hunger".into());r.player.energy=300.;
    begin_resource_turn(&mut r);close(delta(&r,"energy",325.,f64::INFINITY),25.);
    let old=r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()]["hp"].clone();
    add_reserve(&mut r,ID,4.);observe_resource_history(&mut r);
    close(delta(&r,&ability_key(ID),4.,10.),0.);
    assert_eq!(r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()]["hp"],old);
    r.simulation_revision=51.0.into();begin_resource_turn(&mut r);
    assert_eq!(r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()].len(),3);
    assert_eq!(player_change_percent(&r,"energy",325.,100.),None);
}

#[test]
fn resource_turn_same_item_drop_equip_and_reacquisition_keep_turn_identity() {
    let mut r=room();let mut item=Item{id:900.,kind:"amulet".into(),effect:"lightning".to_string().into(),..Default::default()};
    crate::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();r.inventory.push(item);
    begin_resource_turn(&mut r);r.inventory[0].amulet_state.as_mut().unwrap().energy=4.;
    observe_resource_history(&mut r);close(delta(&r,&amulet_key(900.),4.,10.),-60.);
    // Ownership and observation are exercised without inventing another turn.
    let mut dropped=r.inventory.pop().unwrap();observe_resource_history(&mut r);
    dropped.equipped="amulet".to_string().into();r.inventory.push(dropped);observe_resource_history(&mut r);
    close(delta(&r,&amulet_key(900.),4.,10.),-60.);
    r.inventory[0].equipped=Field::Missing;observe_resource_history(&mut r);
    close(delta(&r,&amulet_key(900.),4.,10.),-60.);
    let dropped=r.inventory.pop().unwrap();begin_resource_turn(&mut r);r.inventory.push(dropped);observe_resource_history(&mut r);
    close(delta(&r,&amulet_key(900.),4.,10.),0.); // New turn: first observation, not phantom loss.
    // A newly gained reserve that starts full is also zero change.
    add_reserve(&mut r,ID,10.);observe_resource_history(&mut r);close(delta(&r,&ability_key(ID),10.,10.),0.);
}

#[test]
fn resource_turn_blessing_level_cost_uses_new_goal_and_refills_owned_reserves() {
    let mut r=room();r.level=5.;r.exp=64.;add_reserve(&mut r,ID,0.);begin_resource_turn(&mut r);
    crate::progression::level_up(&mut r);
    assert_eq!(r.level,6.);assert_eq!(r.exp,32.);
    close(delta(&r,"blessings",r.exp,2f64.powf(r.level)),-50.);
    close(delta(&r,&ability_key(ID),10.,10.),100.);
}
#[test]
fn resource_turn_missing_history_initializes_without_inventing_a_change() {
    let mut r=room();add_reserve(&mut r,ID,3.);observe_resource_history(&mut r);
    close(delta(&r,"hp",202.,202.),0.);close(delta(&r,&ability_key(ID),3.,10.),0.);
    let before=r.resource_history.clone();r.player.hp=190.;observe_resource_history(&mut r);
    assert_eq!(r.resource_history,before);close(delta(&r,"hp",190.,202.),-12./202.*100.);
}

#[test]
fn resource_turn_acquiring_and_losing_boundless_uses_the_current_meter_reference() {
    let mut r=room();begin_resource_turn(&mut r);r.player.abilities.push("grisha-boundless-hunger".into());
    r.player.energy=125.;close(delta(&r,"energy",125.,f64::INFINITY),25.);
    begin_resource_turn(&mut r);r.player.abilities.clear();r.player.energy=100.;
    close(delta(&r,"energy",100.,100.),-25.);
}

#[test]
fn resource_turn_validation_keeps_legacy_and_rejects_forged_or_malformed_keys() {
    let mut r=room();begin_resource_turn(&mut r);
    let player=r.player.id.to_string();
    let base=serde_json::to_value(&r).unwrap();
    let sample=serde_json::json!({"start":10.,"max":10.,"loss":0.});
    let loads=|value:&serde_json::Value|crate::storage::deserialize(&serde_json::to_string(value).unwrap());
    assert!(loads(&base).is_ok()); // Old optional/missing reserve entries still load.
    for revision in [29.,50.,51.] {
        let mut old=base.clone();old["simulationRevision"]=revision.into();
        assert!(loads(&old).is_ok(),"legacy base failed at revision {revision}");
    }
    let copied=crate::enemy_traits::catalog().abilities.iter().find(|(_,ability)|ability.passive).unwrap().0;
    let mut known_copied=base.clone();known_copied["resourceHistory"]["actors"][&player][ability_key(copied)]=sample.clone();
    assert!(loads(&known_copied).is_ok());
    for key in ["ability:","ability:not-authored","ability:__proto__","ability:constructor",
        "ability:morae-see-the-future\n","amulet:","amulet:-1","amulet:+1","amulet:01",
        "amulet:1.0","amulet:1e3","amulet:9007199254740992","amulet:999999999999999999999",
        "amulet:constructor","user-controlled-label"] {
        let mut v=base.clone();v["resourceHistory"]["actors"][&player][key]=sample.clone();
        assert!(loads(&v).is_err(),"accepted malformed key {key}");
    }
    let mut too_long=base.clone();too_long["resourceHistory"]["actors"][&player][format!("ability:{}","a".repeat(129))]=sample.clone();
    assert!(loads(&too_long).is_err());
    for key in [ability_key(ID),amulet_key(900.)] {
        let mut v=base.clone();v["resourceHistory"]["actors"][&player][&key]=sample.clone();
        assert!(loads(&v).is_ok(),"valid player meter {key}");
        for revision in [29.,50.,51.] {
            let mut legacy=v.clone();legacy["simulationRevision"]=revision.into();
            assert!(loads(&legacy).is_err(),"accepted new meter under revision {revision}");
        }
        for actor in ["1","00"] {
            let mut forged=base.clone();let mut meters=serde_json::Map::new();meters.insert(key.clone(),sample.clone());
            forged["resourceHistory"]["actors"][actor]=serde_json::Value::Object(meters);
            assert!(loads(&forged).is_err(),"accepted non-player meter {actor}: {key}");
        }
    }
    for id in ["0","9007199254740991"] {
        let mut v=base.clone();v["resourceHistory"]["actors"][&player][format!("amulet:{id}")]=sample.clone();
        assert!(loads(&v).is_ok(),"canonical safe physical ID {id}");
    }
}
#[test]
fn resource_turn_validation_live_engine_roundtrip_keeps_ability_and_amulet_baselines() {
    let mut r=room();add_reserve(&mut r,ID,10.);
    let mut item=Item{id:900.,kind:"amulet".into(),effect:"lightning".to_string().into(),name:"Amulet of Lightning".into(),..Default::default()};
    crate::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();r.inventory.push(item);
    begin_resource_turn(&mut r);r.inventory[0].amulet_state.as_mut().unwrap().energy=4.;
    let mut engine=temporal::Engine::new(r).unwrap();engine.dispatch(&Command::Pickup).unwrap();
    let saved=engine.save().unwrap();let mut restored=temporal::Engine::load(&saved).unwrap();
    assert_eq!(restored.run.resource_history,engine.run.resource_history);
    close(delta(&restored.run,&amulet_key(900.),4.,10.),-60.);
    restored.dispatch(&Command::Wait).unwrap();
    temporal::Engine::load(&restored.save().unwrap()).unwrap();
}
