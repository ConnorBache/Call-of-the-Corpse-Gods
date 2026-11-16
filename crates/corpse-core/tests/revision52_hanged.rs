//! Actual damage, acquisition and persistence regressions for R52 Hanged.
use corpse_core::{Command, Field, Run, abilities, combat, engine, hanged,
    progression, revision50_ability_energy as reserves, storage, temporal::{self, Engine, History, Snapshot}};
const ID: &str = hanged::ID;
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.abilities = vec![ID.into()]; r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_reserves = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.equipment_abilities = Field::Missing; r.player.reactions = 0.0.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 1.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.stats.defense = 0.; r.player.stats.dr = 0.;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.;
    r.inventory.clear(); r.offers.clear(); r.resource_history = Field::Missing;
    r.damage_since_last_turn = Field::Missing; r.damage_since_last_turn_max_hp = Field::Missing;
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None; r.replay.clear();
    r
}
fn packet(r: &mut Run, n: f64) { engine::damage_environment(r, r.player.id, n, None, "", true).unwrap(); }
fn enemy(r: &mut Run) -> f64 {
    let mut a = r.player.clone(); a.id = 900.; a.x = 11.; a.hp = 50.; a.max_hp = 100.;
    a.name = "Test enemy".into(); a.state = "hunting".into(); a.abilities.clear(); a.active.clear();
    a.ability_reserves = Field::Missing; a.conditions.clear(); a.conditions.insert("lucky".into(), 1.);
    a.conditions.insert("weaponDamage".into(), 20.); a.stats.offense = 0.; a.stats.accuracy = 100.;
    r.actors_at_mut(r.depth).push(a); 900.
}
#[test]
fn overflow_exact_energy_and_fractional_shortage_use_actual_resources() {
    for (hp, energy, n, expected_hp, expected_energy) in [
        (1.,8.,2.,1.,0.), (5.,8.,6.,1.,0.), (5.,4.,6.,0.,0.),
        (1.,1.,1.,0.25,0.), (1.,0.5,2.,0.,0.), (1.25,3.,1.,1.,0.),
        (0.75,4.,1.,0.75,0.), (1.25,2.,1.,0.75,0.), (5.,8.,4.,1.,8.),
        (1.,0.,1.,0.,0.), (10.,4.,2.,8.,4.),
    ] {
        let mut r = room(); r.player.hp = hp; r.player.energy = energy;
        let before_rng = r.rng; packet(&mut r,n);
        assert_eq!((r.player.hp,r.player.energy),(expected_hp,expected_energy),"{hp}/{energy}/{n}");
        assert_eq!(r.status == "dead", expected_hp == 0.); assert_eq!(r.rng,before_rng);
    }
}
#[test]
fn every_ordinary_packet_kind_uses_post_scaling_and_rounding_damage() {
    for kind in ["", "attack", "fire", "lightning", "cold", "acid", "water", "roots", "caustic",
        "ichor", "breatherGas", "bleeding", "killspawnBleed", "razorleaves", "epidite", "fall",
        "venom", "poison", "hemophilia", "banshee", "explosion"] {
        let mut r = room(); let n = combat::damage_packet(&r,&r.player,8.4,kind,false);
        let rng = r.rng;
        engine::damage_environment(&mut r,0.,8.4,None,kind,false).unwrap();
        assert_eq!((r.player.hp,r.player.energy),(1.,1000.-4.*n),"{kind}");
        assert_eq!(r.status,"playing"); assert_eq!(r.rng,rng);
    }
}
#[test]
fn direct_and_retained_source_packets_share_the_same_conversion() {
    for retained in [false,true] {
        let mut r=room(); let id=enemy(&mut r); let source=engine::actor(&r,id).unwrap().clone();
        if retained {engine::damage_direct_with_source_snapshot(&mut r,0.,3.,&source,"acid",true).unwrap();}
        else {engine::damage_direct(&mut r,0.,3.,id,"acid",true).unwrap();}
        assert_eq!((r.player.hp,r.player.energy),(1.,988.));
    }
}
#[test]
fn real_melee_has_no_retreat_or_reaction_cost_and_no_phantom_lifesteal() {
    let mut r=room(); let aid=enemy(&mut r); r.player.active.push(ID.into()); r.player.reactions=3.0.into();
    engine::actor_mut(&mut r,aid).unwrap().abilities.push("enemy:beast:lifesteal".into());
    r.player.stats.dr=6.;
    let mut control=r.clone(); control.player.abilities.clear(); control.player.active.clear(); control.player.hp=1000.;
    let rng=r.rng;
    engine::attack(&mut control,aid,0.,1.).unwrap(); let n=1000.-control.player.hp;
    engine::attack(&mut r,aid,0.,1.).unwrap();
    assert!(n>0.); assert_eq!((r.player.hp,r.player.energy),(1.,1000.-4.*n));
    assert_eq!((r.player.x,r.player.y),(10.,10.)); assert_eq!(r.player.reactions.value_or(0.),3.);
    assert_eq!(engine::actor(&r,aid).unwrap().hp,50.); assert_eq!(r.rng,control.rng); assert_ne!(r.rng,rng);
}
#[test]
fn full_absorption_has_no_hp_loss_history_or_masochism_crossing() {
    let mut r=room(); r.player.abilities.push("shrail-masochism".into()); packet(&mut r,3.);
    let meters=&r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()];
    assert_eq!(meters["hp"].loss,0.); assert_eq!(meters["energy"].loss,12.);
    assert_eq!(r.damage_since_last_turn.value_or(0.),0.); assert!(!r.player.conditions.contains_key("hasted"));
}
#[test]
fn actual_hp_loss_is_recorded_separately_from_redirected_damage() {
    let mut r=room(); r.player.hp=5.;r.player.energy=8.;packet(&mut r,6.);
    let meters=&r.resource_history.as_ref().unwrap().actors[&r.player.id.to_string()];
    assert_eq!(meters["hp"].loss,4.); assert_eq!(meters["energy"].loss,8.);
    assert_eq!(r.damage_since_last_turn.value_or(0.),4.);
}
#[test]
fn reserve_maps_are_untouched_and_cannot_rescue_an_empty_main_bar() {
    let mut r=room();reserves::initialize_reserve(&mut r.player,ID,10.,10.).unwrap();
    reserves::initialize_reserve(&mut r.player,"numo-distract",10.,10.).unwrap();
    let before=r.player.ability_reserves.clone();r.player.energy=0.;packet(&mut r,1.);
    assert_eq!(r.status,"dead");assert_eq!(r.player.ability_reserves,before);
}
#[test]
fn health_costs_and_equipment_vitality_loss_bypass_conversion() {
    for kind in ["blood-weapon","blood-letting"] {
        let mut r=room();engine::damage_environment(&mut r,0.,2.,None,kind,true).unwrap();
        assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
    }
    for ability in ["lichor-blood-weapon","lichor-blood-letting"] {
        let mut r=room();r.player.abilities.push(ability.into());
        abilities::activate(&mut r,ability,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
        assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
    }
    let mut r=room();r.player.hp=0.;engine::settle_equipment_death(&mut r).unwrap();
    assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
}
#[test]
fn execute_shard_crushing_and_collapse_are_unpreventable() {
    let mut r=room();let aid=enemy(&mut r);let a=engine::actor_mut(&mut r,aid).unwrap();
    a.abilities.push("shrail-execute".into());a.active.push("shrail-execute".into());
    reserves::initialize_reserve(a,"shrail-execute",20.,20.).unwrap();
    engine::attack(&mut r,aid,0.,1.).unwrap();assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
    let mut r=room();corpse_core::borge::kill_on_hit(&mut r,0.,900.).unwrap();
    assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+10]="wall".into();
    corpse_core::divine_environment::settle_phase_loss(&mut r,false).unwrap();
    assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
    let mut r=room();corpse_core::finale::fall_player(&mut r).unwrap();
    assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);
}
#[test]
fn failed_conversion_runs_existing_rescues_after_energy_is_drained() {
    let mut r=room();r.player.energy=2.;r.player.abilities.push(corpse_core::divine_rescues::RAGE.into());
    r.player.active.push(corpse_core::divine_rescues::RAGE.into());packet(&mut r,2.);
    assert_eq!((r.player.hp,r.player.energy,r.status.as_str()),(1.,0.,"playing"));
    assert_eq!(r.player.conditions.get("rageCost"),Some(&1.));
}
#[test]
fn packet_based_overflowing_and_bonded_keep_resolved_damage_semantics() {
    let mut r=room();let target=enemy(&mut r);engine::actor_mut(&mut r,target).unwrap().hp=100.;
    r.player.abilities.extend(["lichor-overflowing".into(),"morae-bonded-by-unseen-threads".into()]);
    r.player.active.push("morae-bonded-by-unseen-threads".into());r.player.conditions.insert("bondedTarget".into(),target);
    r.player.damage_bank=0.0.into();packet(&mut r,4.);
    assert_eq!(r.player.hp,1.);assert_eq!(r.player.damage_bank.value_or(0.),4.);
    assert_eq!(engine::actor(&r,target).unwrap().hp,98.);
}
#[test]
fn shared_bodies_use_owner_energy_once_and_mini_me_inherits_no_power() {
    for dream in [false,true] {
        let mut r=room();r.player.hp=2.;r.player.energy=8.;
        let id=if dream {
            let mut body=r.player.clone();body.id=900.;body.conditions.insert("phantasmiiBody".into(),1.);
            body.conditions.insert("halfHereOwner".into(),r.player.id);r.actors_at_mut(r.depth).push(body);
            r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,body_id:Some(900.),..Default::default()}.into();900.
        } else {r.player.abilities.push(corpse_core::half_here::ID.into());
            let id=corpse_core::half_here::create(&mut r,11.,10.).unwrap();r.player.active.push(corpse_core::half_here::ID.into());id};
        engine::actor_mut(&mut r,id).unwrap().energy=999.;
        engine::damage_environment(&mut r,id,3.,None,"",true).unwrap();
        assert_eq!((r.player.hp,r.player.energy),(1.,0.));
        let body=engine::actor(&r,id).unwrap();assert_eq!((body.hp,body.energy),(1.,0.));
    }
    let mut r=room();let a=corpse_core::mini_me::preview(&r,11.,10.);
    assert!(a.abilities.is_empty());assert!(a.active.is_empty());assert!(a.ability_reserves.is_missing());
    let id=a.id;corpse_core::mini_me::commit(&mut r,a).unwrap();
    engine::damage_environment(&mut r,id,1.,None,"",true).unwrap();
    assert!(r.actors_at(r.depth).iter().find(|a|a.id==id).is_none_or(|a|a.dead.value_or(false)));assert_eq!(r.player.energy,1000.);
}
#[test]
fn passive_metadata_acquisition_and_commands_never_grant_reactions() {
    for revision in [49.,50.,51.,52.] {
        let mut r=room();r.simulation_revision=revision.into();r.player.reactions=0.0.into();
        progression::initialize_ability_state_for_revision(revision,&mut r.player,ID,0.).unwrap();
        assert_eq!(r.player.reactions.value_or(0.),if revision>=52.{0.}else{1.});
        assert_eq!(engine::max_reactions_for_revision(revision,&r.player),if revision>=52.{0.}else{1.});
        let d=progression::definition_for_revision(revision,ID).unwrap();
        assert_eq!(d.name,if revision>=52.{"Hanged"}else{"Missed Me"});
        if revision>=52. {let before=r.clone();abilities::activate(&mut r,ID,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();assert_eq!(r,before);assert_eq!(d.flags,vec!["passive"]);}
    }
    let mut r=room();r.player.abilities.push("isheilah-quick-hand".into());
    progression::initialize_ability_state_for_revision(52.,&mut r.player,"isheilah-quick-hand",0.).unwrap();
    assert_eq!(r.player.reactions.value_or(0.),1.);assert_eq!(engine::max_reactions_for_revision(52.,&r.player),1.);
}
#[test]
fn pre_r52_damage_still_kills_without_using_energy() {
    for revision in [49.,50.,51.] {let mut r=room();r.simulation_revision=revision.into();packet(&mut r,1.);assert_eq!(r.status,"dead");assert_eq!(r.player.energy,1000.);}
}
#[test]
fn raw_and_compatible_saves_preserve_identity_and_remove_obsolete_toggle_only_on_admission() {
    for revision in [51.,52.] {
        let mut r=room();r.simulation_revision=revision.into();r.player.active.push(ID.into());r.player.reactions=1.0.into();
        r.initial_player=r.player.clone().into();r.player.hp=1.25;r.player.energy=3.5;
        let raw=storage::serialize(&r).unwrap();let structural=storage::deserialize(&raw).unwrap();
        assert_eq!(structural.player.active,r.player.active);
        let normalized=storage::deserialize_compatible(&raw).unwrap();
        assert_eq!(normalized.player.abilities,r.player.abilities);
        assert_eq!((normalized.player.hp,normalized.player.energy),(1.25,3.5));
        assert_eq!(normalized.player.active.contains(&ID.into()),revision<52.);
        assert_eq!(normalized.player.reactions.value_or(0.),if revision<52.{1.}else{0.});
        assert_eq!(normalized.initial_player.as_ref().unwrap().active.contains(&ID.into()),revision<52.);
        let direct=Engine::new(r).unwrap();assert_eq!(direct.run.player.active.contains(&ID.into()),revision<52.);
    }
}
#[test]
fn snapshots_and_future_restore_energy_without_new_temporal_costs() {
    let mut r=room();r.player.hp=2.;r.player.energy=7.;let snap=Snapshot::capture(&r);
    let costs=r.time_costs.clone();packet(&mut r,2.);assert_eq!((r.player.hp,r.player.energy),(1.,3.));assert_eq!(r.time_costs,costs);
    snap.restore(&mut r).unwrap();assert_eq!((r.player.hp,r.player.energy),(2.,7.));
    r.player.abilities.push(reserves::FUTURE_ID.into());
    reserves::initialize_reserve(&mut r.player,reserves::FUTURE_ID,10.,0.).unwrap();
    temporal::remember_future(&mut r,&History::default()).unwrap();let costs=r.time_costs.clone();
    r.player.energy=8.;packet(&mut r,3.);assert_eq!((r.player.hp,r.player.energy),(1.,0.));assert_eq!(r.time_costs,costs);
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());
    assert_eq!((r.player.hp,r.player.energy),(2.,0.));assert!(r.player.abilities.contains(&ID.into()));
}
#[test]
fn save_resume_replays_identical_wait_status_damage_in_normal_and_haste_phases() {
    for haste in [false,true] {
        let mut r=room();r.player.conditions.insert("poisoned".into(),3.);
        if haste {r.player.conditions.insert("hasted".into(),3.);}
        let mut a=Engine::new(r).unwrap();let raw=a.save().unwrap();let mut b=Engine::load(&raw).unwrap();
        for _ in 0..2 {let aa=a.dispatch(&Command::Wait).unwrap();let bb=b.dispatch(&Command::Wait).unwrap();assert_eq!((aa.changed,aa.took_turn,aa.events),(bb.changed,bb.took_turn,bb.events));assert_eq!(a.save().unwrap(),b.save().unwrap());}
        assert!(a.run.player.energy<1000.);assert!(a.run.player.hp>0.);
    }
}
#[test]
fn actual_ranged_attack_uses_energy_after_mitigation_without_extra_rng() {
    let mut r=room();let aid=enemy(&mut r);r.player.stats.dr=7.;
    let mut control=r.clone();control.player.abilities.clear();control.player.hp=1000.;
    assert!(engine::attack_ranged(&mut control,aid,0.,20.,1.,1.,true).unwrap());
    assert!(engine::attack_ranged(&mut r,aid,0.,20.,1.,1.,true).unwrap());
    let n=1000.-control.player.hp;assert_eq!((r.player.hp,r.player.energy),(1.,1000.-4.*n));assert_eq!(r.rng,control.rng);
}
#[test]
fn real_offer_and_equipment_acquisition_do_not_grant_hanged_reactions() {
    let mut r=room();r.player.abilities.clear();r.offers=vec![ID.into()];
    progression::choose_offer(&mut r,ID).unwrap();assert!(r.player.abilities.contains(&ID.into()));
    assert_eq!(r.player.reactions.value_or(0.),0.);
    let mut r=room();r.player.abilities.clear();
    r.inventory.push(corpse_core::Item{id:901.,name:"Ring of Light".into(),kind:"ring".into(),effect:"light".to_string().into(),granted_ability:ID.to_string().into(),..Default::default()});
    corpse_core::equipment::sync_ownership(&mut r).unwrap();
    assert!(r.player.abilities.contains(&ID.into()));assert_eq!(r.player.reactions.value_or(0.),0.);
    r.player.hp=1.;r.player.energy=4.;packet(&mut r,1.);assert_eq!((r.player.hp,r.player.energy),(1.,0.));
    let r=corpse_core::startup::new_run_shell_revision("2-3-4:hanged","Avatar",false,52.).unwrap();
    assert!(r.player.abilities.contains(&ID.into()));assert_eq!(engine::max_reactions_for_revision(52.,&r.player),0.);
    assert_eq!(r.player.reactions.value_or(0.),0.);
}
#[test]
fn amulet_charge_is_never_part_of_the_main_energy_shield() {
    let mut r=room();let mut item=corpse_core::Item{id:901.,name:"Amulet of Reaping".into(),kind:"amulet".into(),effect:"reaping".to_string().into(),..Default::default()};
    corpse_core::revision50_amulets::initialize_item(&mut item).unwrap();r.inventory.push(item);
    let before=r.inventory.clone();r.player.energy=0.;packet(&mut r,1.);
    assert_eq!(r.status,"dead");assert_eq!(r.inventory,before);
}
#[test]
fn a_compatible_nested_future_also_loses_stale_toggle_and_extra_reaction() {
    let mut anchor=room();anchor.player.active.push(ID.into());anchor.player.reactions=1.0.into();
    let mut r=room();r.future=Some(storage::serialize(&anchor).unwrap().into());
    let loaded=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
    let anchor:Run=serde_json::from_str(loaded.future.as_ref().unwrap().wire_text().unwrap()).unwrap();
    assert!(!anchor.player.active.contains(&ID.into()));assert_eq!(anchor.player.reactions.value_or(0.),0.);
}
