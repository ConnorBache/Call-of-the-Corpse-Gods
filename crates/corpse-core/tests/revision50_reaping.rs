//! Authored direct-impact accounting, not a Revision 50 release certification.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine, environment,
    revision50_amulets as amulets, revision50_rings as rings, temporal::Snapshot};

fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=100.;r.player.max_hp=100.;r.player.base_max_hp=100.0.into();r.player.energy=37.;
    r.player.stats=Stats{accuracy:100.,perception:300.,max_energy:100.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;r.player.reaping_ledger=Field::Missing;
    r.inventory=vec![ring(1.),amulet(2.,10.,true),amulet(3.,20.,false)];
    r.actors=Default::default();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;
    r.future=None;r.phase="normal".to_string().into();r.next_id=900.;r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);
    f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;
    f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    r
}
fn ring(id:f64)->Item {Item{id,kind:"ring".into(),name:"Ring of Reaping".into(),effect:rings::REAPING.to_string().into(),..Default::default()}}
fn amulet(id:f64,energy:f64,legacy_flag:bool)->Item {
    let mut i=corpse_core::revision50_amulet_effects::make_amulet(corpse_core::revision50_amulet_effects::Effect::Lightning,id).unwrap();
    i.amulet_state.as_mut().unwrap().energy=energy;if legacy_flag {i.equipped="amulet".to_string().into();}i
}
fn target(id:f64,hp:f64)->Actor {Actor{id,x:11.,y:10.,hp,max_hp:hp.max(100.),name:"Target".into(),state:"hunting".into(),
    stats:Stats{accuracy:100.,max_energy:100.,..Default::default()},heartless:true.into(),
    abilities:vec!["enemy:beast:no-revival".into()],..Default::default()}}
fn energy(r:&Run,id:f64)->f64 {r.inventory.iter().find(|i|i.id==id).unwrap().amulet_state.as_ref().unwrap().energy}
fn carry(r:&Run)->f64 {r.player.reaping_ledger.as_ref().map_or(0.,|l|l.damage_carry)}
fn direct(r:&mut Run,id:f64,amount:f64,kind:&str){let player=r.player.id;engine::damage_direct(r,id,amount,player,kind,true).unwrap();}
fn close(a:f64,b:f64){assert!((a-b).abs()<1e-8,"{a} != {b}");}

#[test]
fn nineteen_then_one_across_turns_rewards_every_owned_amulet_once() {
    let mut r=room();r.inventory.push(ring(4.));r.actors_at_mut(r.depth).push(target(901.,1000.));
    let rng=r.rng;let reserves=r.player.ability_reserves.clone();let activation=r.player.amulet_ledger.clone();
    direct(&mut r,901.,19.,"lightning");assert_eq!((carry(&r),energy(&r,2.),energy(&r,3.)),(19.,10.,20.));
    r.tick+=7.;direct(&mut r,901.,1.,"lightning");
    assert_eq!((carry(&r),energy(&r,2.),energy(&r,3.)),(0.,11.,21.));
    assert_eq!(r.player.energy,37.);assert_eq!(r.player.ability_reserves,reserves);assert_eq!(r.player.amulet_ledger,activation);assert_eq!(r.rng,rng);
}

#[test]
fn fractional_damage_overkill_and_multiple_whole_awards_are_exact() {
    let mut r=room();let mut small=target(901.,19.5);small.max_hp=100.;
    r.actors_at_mut(r.depth).extend([small,target(902.,100.)]);
    direct(&mut r,901.,10000.,"lightning");assert_eq!(carry(&r),19.5);assert_eq!(energy(&r,2.),10.);
    let resistant=rings_item(5.,rings::FIRE_RESISTANCE);
    engine::actor_mut(&mut r,902.).unwrap().copied_inventory=vec![resistant].into();
    direct(&mut r,902.,1.,"fire");assert_eq!(carry(&r),0.);assert_eq!(energy(&r,2.),11.);
    direct(&mut r,902.,85.,"lightning");assert_eq!(carry(&r),5.);assert_eq!(energy(&r,2.),15.);
}
fn rings_item(id:f64,effect:&str)->Item {Item{effect:effect.to_string().into(),..ring(id)}}

#[test]
fn caps_waste_whole_awards_but_retain_subtwenty_remainder() {
    let mut r=room();r.inventory[1].amulet_state.as_mut().unwrap().energy=49.75;
    r.inventory[2].amulet_state.as_mut().unwrap().energy=50.;
    let mut lazy=amulet(4.,50.,false);lazy.amulet_state=Field::Missing;r.inventory.push(lazy);
    r.actors_at_mut(r.depth).push(target(901.,1000.));direct(&mut r,901.,99.,"lightning");
    assert_eq!((energy(&r,2.),energy(&r,3.),carry(&r)),(50.,50.,19.));assert!(r.inventory[3].amulet_state.is_missing());
    r.inventory[1].amulet_state.as_mut().unwrap().energy=0.;direct(&mut r,901.,1.,"lightning");
    assert_eq!((energy(&r,2.),carry(&r)),(1.,0.));
}

#[test]
fn only_dropping_reaping_stops_accrual_and_recollecting_preserves_carry_without_refill() {
    for flag in [Field::Missing,Field::Null,Field::Value(String::new()),"ring".to_string().into()] {
        let mut r=room();r.inventory[0].equipped=flag;
        r.actors_at_mut(r.depth).push(target(901.,1000.));direct(&mut r,901.,19.,"lightning");
        assert_eq!((energy(&r,2.),carry(&r)),(10.,19.));
        assert_eq!(corpse_core::inventory::command(&mut r,&Command::Drop{id:1.}).unwrap(),Some(true));
        direct(&mut r,901.,60.,"lightning");
        assert_eq!((energy(&r,2.),carry(&r)),(10.,19.));
        for item in r.ground_at_mut(r.depth) {item.auto_pickup_blocked=Field::Missing;}
        assert_eq!(corpse_core::inventory::collect_underfoot(&mut r,20.).unwrap(),1);
        assert_eq!((energy(&r,2.),carry(&r)),(10.,19.));
        direct(&mut r,901.,1.,"lightning");assert_eq!((energy(&r,2.),carry(&r)),(11.,0.));
    }
}

#[test]
fn allies_neutral_self_and_allied_sources_do_not_generate_player_credit() {
    let mut r=room();let mut ally=target(901.,500.);ally.state="ally".into();ally.copied_inventory=vec![ring(5.),amulet(6.,0.,true)].into();
    let mut neutral=target(902.,500.);neutral.state="neutral".into();r.actors_at_mut(r.depth).extend([ally,neutral,target(903.,500.)]);
    direct(&mut r,901.,20.,"lightning");direct(&mut r,902.,20.,"lightning");let id=r.player.id;direct(&mut r,id,20.,"lightning");
    engine::damage_direct(&mut r,903.,40.,901.,"lightning",true).unwrap();
    assert!(r.player.reaping_ledger.is_missing());assert_eq!(energy(&r,2.),10.);
    assert_eq!(engine::actor(&r,901.).unwrap().copied_inventory.as_ref().unwrap()[1].amulet_state.as_ref().unwrap().energy,0.);
    // Discord is an explicit temporary hostility override, even for an ally.
    engine::actor_mut(&mut r,901.).unwrap().conditions.insert(corpse_core::revision50_control_amulets::DISCORD.into(),1.);
    direct(&mut r,901.,20.,"lightning");assert_eq!(energy(&r,2.),11.);
}

#[test]
fn damage_over_time_hazards_and_bonded_secondary_packets_never_count() {
    let mut r=room();r.actors_at_mut(r.depth).extend([target(901.,1000.),target(902.,1000.)]);let source=r.player.clone();
    for kind in ["fire","poison","venom","bleeding","roots","fall","explosion"] {
        engine::damage_environment_with_source_snapshot(&mut r,901.,20.,&source,kind,true).unwrap();
    }
    assert_eq!((energy(&r,2.),carry(&r)),(10.,0.));
    let a=engine::actor_mut(&mut r,902.).unwrap();a.abilities.push("morae-bonded-by-unseen-threads".into());
    a.active.push("morae-bonded-by-unseen-threads".into());a.conditions.insert("bondedTarget".into(),901.);
    let hp_before=engine::actor(&r,901.).unwrap().hp;
    direct(&mut r,902.,20.,"lightning");assert_eq!((energy(&r,2.),carry(&r)),(11.,0.));
    assert!(engine::actor(&r,901.).unwrap().hp<hp_before);
}

#[test]
fn immunity_fated_duel_and_damage_reduction_do_not_create_phantom_credit() {
    let mut r=room();let mut a=target(901.,500.);a.stats.dr=1e6;a.abilities.push("enemy:beast:fire-immunity".into());
    r.actors_at_mut(r.depth).push(a);let id=r.player.id;engine::attack(&mut r,id,901.,1.).unwrap();direct(&mut r,901.,80.,"fire");
    let a=engine::actor_mut(&mut r,901.).unwrap();a.abilities.push(corpse_core::revision50::DUEL.into());a.active.push(corpse_core::revision50::DUEL.into());
    for (k,v) in [("r50DuelTarget",999.),("r50DuelX",11.),("r50DuelY",10.),("r50DuelDepth",1.)] {a.conditions.insert(k.into(),v);}
    direct(&mut r,901.,50.,"lightning");assert_eq!(engine::actor(&r,901.).unwrap().hp,500.);
    assert_eq!((energy(&r,2.),carry(&r)),(10.,0.));
}

#[test]
fn melee_ranged_and_execute_credit_only_actual_positive_hp_removed() {
    for ranged in [false,true] {
        let mut r=room();r.actors_at_mut(r.depth).push(target(901.,19.5));let id=r.player.id;
        if ranged {engine::attack_ranged(&mut r,id,901.,1000.,1.,0.,true).unwrap();}
        else {r.player.stats.offense=1000.;engine::attack(&mut r,id,901.,1.).unwrap();}
        assert!(engine::actor(&r,901.).unwrap().dead.value_or(false));assert_eq!((energy(&r,2.),carry(&r)),(10.,19.5));
    }
    let mut r=room();r.player.abilities.push(corpse_core::revision50_combat::EXECUTE.into());r.player.active.push(corpse_core::revision50_combat::EXECUTE.into());
    // This lower-level attack fixture bypasses dispatch acquisition reconciliation.
    // Supply the approved initial reserve so Execute can pay its 20-energy cost.
    corpse_core::revision50_ability_energy::acquire(&mut r.player,&[corpse_core::revision50_combat::EXECUTE.into()]).unwrap();
    let mut a=target(901.,21.);a.stats.dr=1e6;r.actors_at_mut(r.depth).push(a);let id=r.player.id;
    engine::attack(&mut r,id,901.,1.).unwrap();assert!(engine::actor(&r,901.).unwrap().dead.value_or(false));
    assert_eq!((energy(&r,2.),carry(&r)),(11.,1.));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[corpse_core::revision50_combat::EXECUTE].energy,30.);
    assert_eq!(r.player.energy,37.);
}

#[test]
fn rescued_target_awards_gross_removed_hp_not_overkill_or_net_postrescue_loss() {
    let mut r=room();let mut a=target(901.,21.);a.abilities=vec![corpse_core::divine_rescues::PELIAN.into()];a.active=a.abilities.clone();
    r.actors_at_mut(r.depth).push(a);direct(&mut r,901.,1000.,"lightning");
    assert_eq!(engine::actor(&r,901.).unwrap().hp,100.);assert!(!engine::actor(&r,901.).unwrap().dead.value_or(false));
    assert_eq!((energy(&r,2.),carry(&r)),(11.,1.));
}

#[test]
fn half_here_uses_owner_items_and_single_owner_carry_but_mini_me_and_clay_do_not() {
    let mut r=room();r.player.abilities.push(corpse_core::half_here::ID.into());
    let clone=corpse_core::half_here::create(&mut r,9.,10.).unwrap();r.actors_at_mut(r.depth).push(target(901.,500.));
    engine::damage_direct(&mut r,901.,19.,clone,"lightning",true).unwrap();direct(&mut r,901.,1.,"lightning");
    assert_eq!((energy(&r,2.),carry(&r)),(11.,0.));
    for marker in ["miniMe",corpse_core::revision50_terrain::SOLDIER_KEY] {
        let mut source=engine::actor(&r,clone).unwrap().clone();source.conditions.insert(marker.into(),1.);
        engine::damage_direct_with_source_snapshot(&mut r,901.,20.,&source,"lightning",true).unwrap();
    }
    assert_eq!((energy(&r,2.),carry(&r)),(11.,0.));
}

#[test]
fn simultaneous_multitarget_blast_keeps_original_source_even_after_self_death() {
    for reverse in [false,true] {
        let mut r=room();r.player.hp=1.;let mut a=target(901.,19.5);a.x=11.;let mut b=target(902.,20.5);b.x=9.;
        let targets=if reverse {vec![b,a]}else{vec![a,b]};r.actors_at_mut(r.depth).extend(targets);
        let source=r.player.clone();let rng=r.rng;
        environment::explode_with_source_snapshot(&mut r,10.,10.,environment::ExplosionOptions{
            direct_impact:true,source:Some(source.id),ignite_area:false,audible:false,..Default::default()},None,&source).unwrap();
        assert_eq!(r.status,"dead");assert_eq!((energy(&r,2.),energy(&r,3.),carry(&r)),(12.,22.,0.));assert_eq!(r.rng,rng);
    }
}

#[test]
fn immediate_ruin_cast_counts_but_an_allied_caster_does_not() {
    let mut r=room();r.player.abilities.push("enemy:beast:ruin-pulse".into());r.player.energy=100.;
    r.actors_at_mut(r.depth).push(target(901.,500.));let id=r.player.id;
    assert!(corpse_core::monster_ruin::special_action(&mut r,id,901.).unwrap());
    assert_eq!(carry(&r),11.);assert_eq!(energy(&r,2.),10.);
    let mut ally=target(902.,100.);ally.state="ally".into();ally.abilities=vec!["enemy:beast:ruin-pulse".into()];ally.energy=100.;
    r.actors_at_mut(r.depth).push(ally);assert!(corpse_core::monster_ruin::special_action(&mut r,902.,901.).unwrap());
    assert_eq!(carry(&r),11.);assert_eq!(energy(&r,2.),10.);
}

#[test]
fn raw_save_snapshot_and_timeweave_keep_carry_without_recharge_or_replay() {
    let mut r=room();r.actors_at_mut(r.depth).push(target(901.,500.));direct(&mut r,901.,19.,"lightning");
    let saved=serde_json::to_string(&r).unwrap();let mut loaded:Run=serde_json::from_str(&saved).unwrap();assert_eq!(carry(&loaded),19.);
    let snapshot=Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();direct(&mut loaded,901.,1.,"lightning");
    snapshot.restore(&mut loaded).unwrap();assert_eq!((energy(&loaded,2.),carry(&loaded)),(10.,19.));
    direct(&mut loaded,901.,1.,"lightning");assert_eq!((energy(&loaded,2.),carry(&loaded)),(11.,0.));
    loaded.actors=Default::default();loaded.player.abilities.push(corpse_core::progression::WEAVE_ID.into());
    engine::dispatch(&mut loaded,&Command::Move{dx:1.,dy:0.}).unwrap();let before=(loaded.tick,loaded.rng,loaded.player.reaping_ledger.clone(),energy(&loaded,2.));
    assert!(!engine::dispatch(&mut loaded,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    assert_eq!((loaded.tick,loaded.rng,loaded.player.reaping_ledger.clone(),energy(&loaded,2.)),before);
}

#[test]
fn invalid_ledgers_and_old_revisions_are_rejected_and_legacy_damage_does_not_accrue() {
    let mut r=room();for damage_carry in [-1.,20.,f64::INFINITY,f64::NAN] {
        r.player.reaping_ledger=amulets::ReapingLedger{damage_carry}.into();assert!(amulets::validate_run(&r).is_err());
    }
    r.player.reaping_ledger=Field::Null;assert!(amulets::validate_run(&r).is_err());
    r.player.reaping_ledger=amulets::ReapingLedger{damage_carry:19.}.into();assert!(amulets::validate_run(&r).is_ok());
    r.simulation_revision=49.0.into();assert!(amulets::validate_run(&r).is_err());
    r.player.reaping_ledger=Field::Missing;r.inventory.iter_mut().for_each(|i|i.amulet_state=Field::Missing);
    r.actors_at_mut(r.depth).push(target(901.,500.));direct(&mut r,901.,20.,"lightning");assert!(r.player.reaping_ledger.is_missing());
}

#[test]
fn retained_damage_math_handles_finite_extremes_without_per_energy_work() {
    let mut r=room();let attacker=r.player.clone();let target=target(901.,f64::MAX);
    amulets::reap_direct_hit(&mut r,&attacker,&target,f64::MAX);
    assert_eq!(energy(&r,2.),50.);assert!(carry(&r).is_finite() && (0.0..20.0).contains(&carry(&r)));
    close(energy(&r,3.),50.);
}

#[test]
fn one_lightning_activation_credits_each_hostile_hit_and_the_spent_source_amulet() {
    let mut r=room();r.inventory[1].amulet_state.as_mut().unwrap().energy=50.;
    let mut second=target(902.,500.);second.x=12.;r.actors_at_mut(r.depth).extend([target(901.,500.),second]);
    let rng=r.rng;
    assert_eq!(corpse_core::revision50_amulet_effects::command(&mut r,&Command::UseAt{id:2.,x:12.,y:10.}).unwrap(),Some(true));
    assert_eq!((energy(&r,2.),energy(&r,3.),carry(&r)),(30.,25.,0.));assert_eq!(r.rng,rng);
}
