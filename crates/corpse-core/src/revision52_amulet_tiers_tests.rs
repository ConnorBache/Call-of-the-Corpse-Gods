//! Controlled native mechanics plus generation/serialization identity checks.
use super::*;
use crate::{Command, Stats, engine, inventory, revision50_amulet_effects as effects,
    revision50_amulets as energy, revision50_control_amulets as control, rng::GenerationRng};

fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();r.player.energy=10000.;
    r.player.stats=Stats{perception:300.,max_energy:10000.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.player.conditions.insert("allSeeingReveal".into(),1.);
    r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    r.inventory.clear();for a in r.actors.values_mut(){*a=Default::default();}
    for g in r.ground.values_mut(){*g=Default::default();}
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r.next_id=900.;r.phase="normal".to_string().into();
    r.offers.clear();r.replay.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.remembered_tiles=vec!["floor".into();f.tiles.len()].into();f.doors=Field::Missing;
    f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
    f.environment=Field::Missing;f.hellfire=Field::Missing;f.grates=Field::Missing;
    f.revision50_terrain=Field::Missing;f.revision50_temporary_terrain=Field::Missing;r
}
fn amulet(effect:&str,tier:u8,id:f64)->Item {
    let mut i=Item{id,kind:"amulet".into(),effect:effect.to_string().into(),glyph:"\"".into(),identified:true.into(),..Default::default()};
    energy::initialize_item_for_revision(&mut i,52.).unwrap();initialize_birth(&mut i,52.,if tier==1{1}else{(tier as u32-1)*5}).unwrap();i
}
fn enemy(r:&mut Run,id:f64,x:f64,y:f64) {
    let mut a=Actor{id,x,y,name:"Giant Rat".into(),glyph:"r".into(),color:"#ffffff".into(),
        hp:1000.,max_hp:1000.,base_max_hp:1000.0.into(),energy:100.,state:"hunting".into(),
        stats:Stats{max_energy:100.,dr:900.,..Default::default()},..Default::default()};
    a.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(a);
}
fn cast(r:&mut Run,id:f64,x:f64,y:f64) {effects::command(r,&Command::UseAt{id,x,y}).unwrap().unwrap();}
fn hp(r:&Run,id:f64)->f64 {engine::actor(r,id).unwrap().hp}
fn ring(effect:&str)->Item {Item{id:800.,kind:"ring".into(),effect:effect.to_string().into(),..Default::default()}}
// Assert the intentional R52 birth differences, then compare every other field
// with the frozen R51 item, including identity, subtype and untouched RNG draws.
fn normalize(i:&mut Item) {
    let state=i.amulet_state.as_mut().unwrap();assert_eq!(state.energy,10.);state.energy=50.;
    i.amulet_tier=Field::Missing;if let Some(n)=effects::display_name(i){i.name=n.into();}
}

#[test] fn floor_bands_and_all_profiles_are_strict_and_authored_at_one() {
    let r=room();let keys=["lightning","firebolt","acid","iceblast","floods","poison","roots"];
    assert_eq!(DAMAGE[0],[50.,80.,40.,30.,20.,6.,3.]);
    for depth in 1..=40 {assert_eq!(tier_for_depth(depth),if depth<=4{1}else{(depth/5+1)as u8});}
    for (c,key) in keys.iter().enumerate() {for tier in 1..=9 {
        let i=amulet(key,tier,901.);let p=profile(&r,&i).unwrap();
        assert_eq!(p.tier,tier);assert_eq!(p.direct_damage+p.periodic_damage,DAMAGE[(tier-1)as usize][c]);
        if tier>1 {assert!(DAMAGE[(tier-1)as usize][c]>DAMAGE[(tier-2)as usize][c]);}
        assert_eq!(energy::BASE_ENERGY,effects::item_spec(&i).unwrap().capacity);
        assert_eq!(i.amulet_state.as_ref().unwrap().energy,10.);
    }}
}
#[test] fn generation_changes_only_tier_name_and_reserve_without_rng_or_id_draws() {
    for depth in 1..=40 {for seed in 1..=24 {
        let mut a=GenerationRng::new(seed);let mut b=a;let(mut ai,mut bi)=(100.,100.);
        let old=crate::equipment::create_equipment_revision("amulet",&mut a,&mut ai,depth,&[],51.).unwrap();
        let mut new=crate::equipment::create_equipment_revision("amulet",&mut b,&mut bi,depth,&[],52.).unwrap();
        assert_eq!(new.amulet_tier.value_or(0.),tier_for_depth(depth)as f64);
        assert_eq!((a.state,ai),(b.state,bi));normalize(&mut new);assert_eq!(new,old);
    }}
}
#[test] fn first_four_floor_density_and_subtype_stream_stay_identical() {
    for depth in 1..=4 {for seed in 1..=8 {
        let mut old=room();old.simulation_revision=51.0.into();old.seed=format!("amulet-early-{seed}");let mut new=old.clone();new.simulation_revision=52.0.into();
        let mut a=GenerationRng::new(seed);let mut b=a;
        let legacy=crate::equipment::generate_floor_loot_for_run(&mut old,depth,&mut a,&[],5).unwrap();
        let mut tiered=crate::equipment::generate_floor_loot_for_run(&mut new,depth,&mut b,&[],5).unwrap();
        for i in &mut tiered {if i.kind=="amulet" {assert_eq!(i.amulet_tier.value_or(0.),1.);normalize(i);}}
        assert_eq!(legacy,tiered);assert_eq!((old.next_id,a.state),(new.next_id,b.state));
    }}
}
#[test] fn special_rewards_use_birth_floor_without_altering_draws() {
    let mut found=0;
    for seed in 1..=64 {let mut a=GenerationRng::new(seed);let mut b=a;
        let old=crate::equipment::make_godbeast_permanent_revision(40,901.,&mut a,51.).unwrap();
        let mut new=crate::equipment::make_godbeast_permanent_revision(40,901.,&mut b,52.).unwrap();
        if new.kind=="amulet" {found+=1;assert_eq!(new.amulet_tier.value_or(0.),9.);normalize(&mut new);}
        assert_eq!(old,new);assert_eq!(a.state,b.state);
    }assert!(found>0);
}
#[test] fn missing_tier_is_base_and_birth_never_retiers_or_recharges() {
    let mut r=room();r.depth=40.;let mut i=amulet("lightning",1,901.);i.amulet_state.as_mut().unwrap().energy=7.;
    let prior=i.clone();initialize_birth(&mut i,52.,40).unwrap();assert_eq!(i,prior);
    i.amulet_tier=Field::Missing;assert_eq!(profile(&r,&i).unwrap().direct_damage,50.);
    assert_eq!(display_name(&r,&i).unwrap(),"Tier 1 Amulet of Lightning");
    r.simulation_revision=51.0.into();assert_eq!(display_name(&r,&i).unwrap(),"Amulet of Lightning");
    let json=serde_json::to_value(&i).unwrap();assert!(json.get("amuletTier").is_none());
}
#[test] fn malformed_tiers_and_captured_rates_reject_before_dispatch_mutation() {
    for bad in [Field::Null,0.0.into(),10.0.into(),1.5.into(),f64::INFINITY.into(),f64::NAN.into()] {
        let mut r=room();let mut i=amulet("lightning",1,901.);i.amulet_tier=bad;r.inventory.push(i);
        assert!(validate_run(&r).is_err());let before=serde_json::to_value(&r).unwrap();
        assert!(engine::dispatch(&mut r,&Command::Wait).is_err());assert_eq!(serde_json::to_value(&r).unwrap(),before);
    }
    for rev in [29.,50.,51.,53.] {let mut r=room();r.simulation_revision=rev.into();r.inventory.push(amulet("lightning",1,901.));assert!(validate_run(&r).is_err());}
    for key in [effects::POISON_DAMAGE,ROOTS_DAMAGE] {for rate in [-1.,0.,1.5,10000.,f64::INFINITY] {
        let mut r=room();r.player.conditions.insert(key.into(),rate);assert!(validate_run(&r).is_err());
    }}
    let mut r=room();r.player.conditions.insert(ROOTS_DAMAGE.into(),26.);assert!(validate_run(&r).is_err());
}
#[test] fn direct_packets_use_exact_instance_tier_bypass_dr_and_keep_cost() {
    for (kind,column,x) in [("lightning",0,14.),("firebolt",1,14.),("acid",2,14.),("iceblast",3,14.),("floods",4,12.)] {
        let mut r=room();enemy(&mut r,100.,x,10.);let i=amulet(kind,9,901.);let cost=effects::item_spec(&i).unwrap().activation_cost;r.inventory.push(i);
        let main_energy=r.player.energy;
        cast(&mut r,901.,x,10.);assert_eq!(hp(&r,100.),1000.-DAMAGE[8][column],"{kind}");
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,(10.-cost).max(0.));
        assert_eq!(r.player.energy,main_energy-(cost-10.).max(0.),"{kind}");
        assert!(r.log.iter().any(|s|s.contains("Tier 9 Amulet of")));
    }
}
#[test] fn resistance_rounds_once_and_immunities_still_block() {
    for (kind,protection,expected) in [("acid","corrosion-resistance",47.),("firebolt","fire-resistance",47.),("iceblast","ice-immunity",0.)] {
        let mut r=room();enemy(&mut r,100.,14.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().copied_inventory=vec![ring(protection)].into();
        r.inventory.push(amulet(kind,9,901.));cast(&mut r,901.,14.,10.);assert_eq!(hp(&r,100.),1000.-expected, "{kind}");
    }
    let mut r=room();enemy(&mut r,100.,14.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().abilities.push("enemy:beast:fire-immunity".into());
    r.inventory.push(amulet("firebolt",9,901.));cast(&mut r,901.,14.,10.);assert_eq!(hp(&r,100.),1000.);
}
#[test] fn poison_captures_overwrite_rate_not_source_item_or_stack() {
    let mut r=room();enemy(&mut r,100.,14.,10.);r.inventory=vec![amulet("poison",9,901.),amulet("poison",1,902.)];
    cast(&mut r,901.,14.,10.);assert_eq!(hp(&r,100.),1000.);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),effects::POISON_DAMAGE),28.);
    r.inventory[0].amulet_tier=2.0.into();crate::monster_statuses::tick_poisoned(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),972.);
    cast(&mut r,902.,14.,10.);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),effects::POISON_DAMAGE),6.);
    assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),"poisoned"),20.);
    crate::monster_statuses::tick_poisoned(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),966.);
    r.inventory[0].amulet_tier=9.0.into();cast(&mut r,901.,14.,10.);
    engine::actor_mut_trusted(&mut r,100.).unwrap().copied_inventory=vec![ring("poison-resistance")].into();
    crate::monster_statuses::tick_poisoned(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),952.);
    assert!(r.player.reaping_ledger.is_missing());
}
#[test] fn roots_capture_rate_refresh_replace_and_expire_or_break() {
    let mut r=room();enemy(&mut r,100.,14.,10.);r.inventory=vec![amulet("roots",9,901.),amulet("roots",1,902.)];
    cast(&mut r,901.,14.,10.);assert_eq!(hp(&r,100.),1000.);r.inventory.remove(0);
    control::tick_actor(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),974.);
    cast(&mut r,902.,14.,10.);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),ROOTS_DAMAGE),3.);
    for _ in 0..4 {control::tick_actor(&mut r,100.).unwrap();}
    assert_eq!(hp(&r,100.),962.);assert!(!engine::actor(&r,100.).unwrap().conditions.contains_key(ROOTS_DAMAGE));
    r.inventory[0].amulet_tier=9.0.into();cast(&mut r,902.,14.,10.);
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("burning".into(),2.);
    control::tick_actor(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),962.);assert!(!engine::actor(&r,100.).unwrap().conditions.contains_key(ROOTS_DAMAGE));
}
#[test] fn roots_reject_flight_without_cost_and_legacy_missing_rate_is_three() {
    let mut r=room();enemy(&mut r,100.,14.,10.);r.inventory.push(amulet("roots",9,901.));
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("flying".into(),5.);
    let before=r.clone();assert!(effects::command(&mut r,&Command::UseAt{id:901.,x:14.,y:10.}).is_err());assert_eq!(r,before);
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.shift_remove("flying");cast(&mut r,901.,14.,10.);
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.shift_remove(ROOTS_DAMAGE);
    control::tick_actor(&mut r,100.).unwrap();assert_eq!(hp(&r,100.),997.);
}
#[test] fn utility_profile_endpoints_keep_cost_shape_and_target_count() {
    let r=room();for tier in 1..=9 {for (kind,base,is_range) in [("blinking",10.,true),("beckoning",8.,true),("teleportation",8.,true),("discord",6.,false),("obstruction",8.,false)] {
        let p=profile(&r,&amulet(kind,tier,901.)).unwrap();assert_eq!(if is_range{p.range}else{p.duration},Some(base+(tier-1)as f64));
    }}
    let mut r=room();r.inventory.push(amulet("discord",9,901.));enemy(&mut r,100.,14.,10.);cast(&mut r,901.,14.,10.);
    assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),control::DISCORD),14.);
    let mut r=room();r.inventory.push(amulet("obstruction",9,901.));cast(&mut r,901.,14.,10.);
    let terrain=r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap();assert_eq!(terrain.len(),3);assert!(terrain.iter().all(|t|t.expires_at==r.tick+16.));
    r.tick+=15.;control::expire_terrain(&mut r);assert_eq!(r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().len(),3);
    r.tick+=1.;control::expire_terrain(&mut r);assert!(r.floor().unwrap().revision50_temporary_terrain.is_missing());
}
#[test] fn extended_blink_uses_reachable_line_and_hides_unknown_occupants() {
    let mut r=room();r.inventory.push(amulet("blinking",9,901.));let p=effects::aim_preview(&r,&r.inventory[0],28.,10.);assert_eq!(p.impact,Some((28,10)));
    assert!(effects::target_blocker_ui(&r,&r.inventory[0],29.,10.).is_some());
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+23]="wall".into();
    cast(&mut r,901.,28.,10.);assert_eq!(r.player.x,22.);
    let mut r=room();r.player.conditions.clear();r.inventory.push(amulet("blinking",9,901.));let clear=effects::aim_preview(&r,&r.inventory[0],18.,10.);
    enemy(&mut r,100.,11.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("invisible".into(),20.);
    assert_eq!(effects::aim_preview(&r,&r.inventory[0],18.,10.),clear);assert!(effects::admission(&r,&Command::UseAt{id:901.,x:18.,y:10.}).unwrap().is_ok());
    let main_energy=r.player.energy;
    cast(&mut r,901.,18.,10.);assert_eq!(r.player.x,10.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    assert_eq!(r.player.energy,main_energy-10.);
}
#[test] fn extended_beckon_and_teleport_range_use_the_same_visible_preview() {
    for kind in ["beckoning","teleportation"] {
        let mut r=room();enemy(&mut r,100.,25.,10.);r.inventory=vec![amulet(kind,9,901.),amulet(kind,1,902.)];
        assert!(effects::target_blocker_ui(&r,&r.inventory[0],25.,10.).is_none());assert!(effects::target_blocker_ui(&r,&r.inventory[1],25.,10.).is_some());
        assert!(!effects::aim_preview(&r,&r.inventory[0],25.,10.).area.is_empty());
        cast(&mut r,901.,25.,10.);let a=engine::actor(&r,100.).unwrap();if kind=="beckoning"{assert_eq!((a.x,a.y),(11.,10.));}else{assert_ne!((a.x,a.y),(25.,10.));}
    }
}
#[test] fn drop_pickup_copy_and_snapshot_preserve_tiers_and_cap_existing_reserves() {
    let mut r=room();r.inventory=vec![amulet("lightning",1,901.),amulet("lightning",9,902.)];
    r.inventory[0].amulet_state.as_mut().unwrap().energy=7.;r.inventory[1].amulet_state.as_mut().unwrap().energy=23.;
    enemy(&mut r,100.,14.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().copied_inventory=r.inventory.clone().into();
    // A pre-amendment R52 item may retain 23 in the raw snapshot. Acquisition
    // and restore cap it to 10 without refilling the other item's spent charge.
    let mut expected=r.inventory.clone();expected[1].amulet_state.as_mut().unwrap().energy=10.;
    let snapshot=crate::temporal::Snapshot::capture(&r);
    assert_eq!(snapshot.state.inventory[1].amulet_state.as_ref().unwrap().energy,23.);
    inventory::command(&mut r,&Command::Drop{id:902.}).unwrap();for g in r.ground_at_mut(r.depth){g.auto_pickup_blocked=Field::Missing;}
    inventory::collect_underfoot(&mut r,20.).unwrap();assert_eq!(r.inventory,expected);
    r.inventory[0].amulet_tier=9.0.into();r.inventory[0].amulet_state.as_mut().unwrap().energy=0.;snapshot.restore(&mut r).unwrap();
    assert_eq!(r.inventory,expected);assert_eq!(engine::actor(&r,100.).unwrap().copied_inventory.as_ref().unwrap(),&expected);
    assert_eq!(snapshot.state.inventory[1].amulet_state.as_ref().unwrap().energy,23.);
}
#[test] fn reaping_and_free_cast_keep_individual_reserves_and_shared_credit() {
    let mut r=room();r.inventory=vec![amulet("lightning",9,901.),amulet("lightning",1,902.),ring("reaping")];
    r.inventory[1].amulet_state.as_mut().unwrap().energy=0.;r.player.abilities=vec!["kative-appropriate".into(),"isheilah-spellslinger".into()];enemy(&mut r,100.,14.,10.);
    assert_eq!(effects::command(&mut r,&Command::UseAt{id:901.,x:14.,y:10.}).unwrap(),Some(false));
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,4.);assert_eq!(r.inventory[1].amulet_state.as_ref().unwrap().energy,4.);
    assert_eq!(r.player.energy,9995.);
    assert_eq!(r.player.reaping_ledger.as_ref().unwrap().damage_carry,13.);
    assert_eq!(effects::command(&mut r,&Command::UseAt{id:901.,x:14.,y:10.}).unwrap(),Some(true));
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,5.);assert_eq!(r.inventory[1].amulet_state.as_ref().unwrap().energy,9.);
    assert_eq!(r.player.energy,9984.);assert_eq!(r.player.reaping_ledger.as_ref().unwrap().damage_carry,6.);
    assert_eq!(r.inventory[0].amulet_tier.value_or(0.),9.);assert_eq!(r.inventory[1].amulet_tier.value_or(0.),1.);
}
#[test] fn full_save_future_and_fixture_replay_preserve_tier_rate_and_charge() {
    let mut r=room();r.inventory=vec![amulet("poison",9,901.),amulet("roots",8,902.)];enemy(&mut r,100.,14.,10.);
    cast(&mut r,901.,14.,10.);cast(&mut r,902.,14.,10.);
    let before=r.clone();let raw=crate::storage::serialize(&r).unwrap();let loaded=crate::storage::deserialize(&raw).unwrap();assert_eq!(loaded.inventory,r.inventory);
    assert_eq!(engine::actor(&loaded,100.).unwrap().conditions,engine::actor(&r,100.).unwrap().conditions);
    r.player.abilities.push("morae-see-the-future".into());crate::revision50_ability_energy::initialize_reserve(&mut r.player,"morae-see-the-future",50.,0.).unwrap();
    crate::temporal::remember_future(&mut r,&crate::temporal::History::default()).unwrap();
    let mut restored=crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();restored.inventory[0].amulet_tier=1.0.into();restored.inventory[0].amulet_state.as_mut().unwrap().energy=0.;
    assert!(crate::temporal::restore_future(&mut restored,&mut crate::temporal::History::default()).unwrap());assert_eq!(restored.inventory,before.inventory);
    let mut recorded=crate::temporal::Engine::new(before.clone()).unwrap();recorded.dispatch(&Command::UseAt{id:901.,x:14.,y:10.}).unwrap();recorded.dispatch(&Command::Wait).unwrap();
    let record=crate::replay::ReplayRecord::from_run(&recorded.run);let mut replay=crate::replay::ReplaySession::from_initial(before,record).unwrap();while replay.step().unwrap(){}
    assert_eq!(replay.engine.run.inventory,recorded.run.inventory);assert_eq!(replay.engine.run.rng,recorded.run.rng);
    assert_eq!(engine::actor(&replay.engine.run,100.).unwrap(),engine::actor(&recorded.run,100.).unwrap());
}
#[test] fn typed_and_serialized_hidden_item_homes_reject_malformed_tiers() {
    for home in ["carried","copied","initial","ground","remembered"] {
        let mut r=room();let mut i=amulet("poison",9,901.);i.amulet_tier=10.0.into();
        match home {
            "carried"=>r.player.monster_state=crate::ActorMonsterState{carried_items:vec![i].into(),..Default::default()}.into(),
            "copied"=>r.player.copied_inventory=vec![i].into(),
            "initial"=>{let mut a=r.player.clone();a.copied_inventory=vec![i].into();r.initial_player=a.into();},
            "ground"=>r.ground_at_mut(r.depth).push(crate::GroundItem{x:10.,y:10.,item:i,..Default::default()}),
            _=>{let mut a=std::collections::BTreeMap::new();a.insert("1",vec![serde_json::json!({"x":10,"y":10,"item":i})]);let mut v=serde_json::to_value(&r).unwrap();v["rememberedItems"]=serde_json::to_value(a).unwrap();assert!(crate::storage::deserialize(&v.to_string()).is_err());continue;},
        }
        assert!(validate_run(&r).is_err(),"{home}");assert!(crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).is_err(),"{home}");
    }
}
#[test] fn debug_reconstruction_retains_each_prior_birth_floor() {
    let r=crate::debug_floor::create_run_revision("02-04-09:amulet-tier-debug","Unblessed",10,52.).unwrap();
    let state=crate::debug_floor::state(&r).unwrap();let mut count=0;
    for reward in state.stash.iter().chain(&state.pending) {if reward.item.kind=="amulet" {
        count+=1;assert_eq!(reward.item.amulet_tier.value_or(0.),tier_for_depth(reward.depth)as f64);
        assert_eq!(reward.item.amulet_state.as_ref().unwrap().energy,10.);
    }}assert!(count>0);assert!(state.stash.iter().any(|v|v.item.kind=="amulet"&&v.depth<5&&v.item.amulet_tier.value_or(0.)==1.));
}
#[test] fn malformed_snapshot_and_future_reject_before_any_world_mutation() {
    let mut r=room();r.inventory.push(amulet("lightning",9,901.));let before=r.clone();
    let mut bad=crate::temporal::Snapshot::capture(&r);std::sync::Arc::make_mut(&mut bad.state).inventory[0].amulet_tier=0.0.into();
    assert!(bad.restore(&mut r).is_err());assert_eq!(r,before);
    assert!(crate::temporal::Snapshot::from_value(bad.value().unwrap()).is_err());
    let mut bad_world=r.clone();bad_world.inventory[0].amulet_tier=10.0.into();r.future=Some(crate::storage::serialize(&bad_world).unwrap().into());let before=r.clone();
    assert!(crate::temporal::restore_future(&mut r,&mut crate::temporal::History::default()).is_err());assert_eq!(r,before);
}
#[test] fn debug_take_rejects_invalid_tier_before_moving_item() {
    let mut r=room();let mut i=amulet("lightning",9,901.);i.amulet_tier=0.0.into();
    r.playtest=crate::playtest::Profile{version:1,start_floor:1,profile:crate::debug_floor::PROFILE.into(),prior_hearts:0,prior_blessings:0.,sources:vec![],candidates:vec![],debug:Some(crate::debug_floor::State{pending:Default::default(),consumed:0,stash:vec![crate::debug_floor::Reward{depth:1,item:i}],entered:false,loadout_revision:0})}.into();
    let before=r.clone();assert!(crate::debug_floor::take(&mut r,901.).is_err());assert_eq!(r,before);
}
#[test] fn tinker_keeps_tier_identity_reserve_and_names_the_actual_item() {
    let mut r=room();r.gods=(0..12).filter(|g|*g!=8).map(|g|g as f64).collect();
    r.inventory.push(amulet("acid",9,901.));r.inventory[0].amulet_state.as_mut().unwrap().energy=7.;
    crate::revision50_ability_energy::initialize_reserve(&mut r.player,"kative-tinker",0.,0.).unwrap();
    assert!(crate::tinker::activate(&mut r,Some(901.)).unwrap());let i=&r.inventory[0];
    assert_eq!(i.amulet_tier.value_or(0.),9.);assert_eq!(i.amulet_state.as_ref().unwrap().energy,7.);assert_eq!(i.id,901.);
    assert!(i.tinkered.value_or(false));assert!(i.granted_ability.as_ref().is_some_and(|s|!s.is_empty()));
    let name=crate::revision50_item_knowledge::display_name(&r,i);
    assert!(name.starts_with("Tier 9 Amulet of Acid and "), "{name}");
    assert!(r.log.iter().any(|s|s.starts_with(&format!("{name} is blessed"))));
}
#[test] fn offscreen_periodic_packets_keep_cast_rates_after_item_removal() {
    let mut r=room();r.inventory=vec![amulet("poison",9,901.),amulet("roots",9,902.)];enemy(&mut r,100.,14.,10.);
    cast(&mut r,901.,14.,10.);cast(&mut r,902.,14.,10.);r.inventory.clear();let depth=r.depth;
    r.floor_mut().unwrap().last_tick=0.;r.floor_mut().unwrap().dirty=true;r.depth=2.;r.tick=1.;
    crate::offscreen::tick_inactive_floors(&mut r).unwrap();let a=r.actors_at(depth).iter().find(|a|a.id==100.).unwrap();
    assert_eq!(engine::condition(a,effects::POISON_DAMAGE),28.);assert_eq!(engine::condition(a,ROOTS_DAMAGE),26.);
    assert_eq!(engine::condition(a,control::ROOTS),3.);assert!(a.hp<975.);
}
#[test] fn travel_and_cloned_loadouts_never_upgrade_or_refill() {
    let mut r=room();r.inventory=vec![amulet("acid",1,901.),amulet("acid",9,902.)];r.inventory[0].amulet_state.as_mut().unwrap().energy=3.;r.inventory[1].amulet_state.as_mut().unwrap().energy=9.;
    let expected=r.inventory.clone();assert!(crate::travel::travel(&mut r,1.,false,true).unwrap());assert_eq!(r.inventory,expected);
    assert_eq!(r.depth,2.);let copy=r.clone();assert_eq!(copy.inventory,expected);
}
#[test] fn area_and_pierce_remain_independent_of_tier_and_overkill_credit_is_capped() {
    let mut r=room();r.inventory=vec![amulet("lightning",9,901.),ring("reaping")];enemy(&mut r,100.,14.,10.);enemy(&mut r,101.,20.,10.);
    for id in [100.,101.] {engine::actor_mut_trusted(&mut r,id).unwrap().hp=10.;}
    cast(&mut r,901.,12.,10.);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,1.);assert_eq!(r.player.reaping_ledger.as_ref().unwrap().damage_carry,0.);
    assert_eq!(r.player.energy,9985.);
    let mut r=room();r.inventory.push(amulet("iceblast",9,901.));enemy(&mut r,100.,14.,10.);enemy(&mut r,101.,18.,10.);
    let p=effects::aim_preview(&r,&r.inventory[0],18.,10.);assert_eq!(p.impact,Some((14,10)));assert_eq!(p.area.len(),9);
    cast(&mut r,901.,18.,10.);assert_eq!(hp(&r,100.),912.);assert_eq!(hp(&r,101.),1000.);
}
