//! R51 packets round once after mitigation/scaling/resistance; old runs replay unchanged.
use corpse_core::{Actor, Field, Item, Run, Stats, combat, engine, environment,
    monster_statuses, revision50_rings as rings, storage, temporal::Snapshot};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();
    r.player.x=10.;r.player.y=10.;r.player.hp=100.;r.player.max_hp=100.;
    r.player.base_max_hp=100.0.into();r.player.energy=100.;
    r.player.stats=Stats{accuracy:100.,perception:300.,max_energy:100.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.inventory.clear();
    for a in r.actors.values_mut(){*a=Default::default();}
    for g in r.ground.values_mut(){*g=Default::default();}
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.phase="normal".to_string().into();r.next_id=900.;
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);
    f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;
    f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;f.currents=Field::Missing;
    f.fungal_roots=Field::Missing;f.roots=Field::Missing;f.mud=Field::Missing;
    f.slime=Field::Missing;f.ichor=Field::Missing;f.razorleaves=Field::Missing;
    f.water_blood=Field::Missing;f.kelpie_blood_sources=Field::Missing;
    f.vents=Field::Missing;f.permanent_fire=Field::Missing;r
}
fn ring(effect:&str)->Item {Item{id:800.,kind:"ring".into(),name:format!("Ring of {effect}"),glyph:"=".into(),effect:effect.to_string().into(),..Default::default()}}
fn enemy(id:f64,x:f64,y:f64)->Actor {
    Actor{id,x,y,hp:100.,max_hp:100.,base_max_hp:100.0.into(),energy:100.,state:"hunting".into(),name:"Giant Rat".into(),heartless:true.into(),glyph:"r".into(),color:"#ffffff".into(),
        stats:Stats{accuracy:100.,perception:300.,max_energy:100.,..Default::default()},abilities:vec!["enemy:beast:no-revival".into()],..Default::default()}
}
fn close(actual:f64,expected:f64){assert!((actual-expected).abs()<1e-8,"{actual} != {expected}");}

#[test]
fn revision51_nearest_boundaries_and_legacy_primitives() {
    for (n,want) in [(-1.,0.),(0.,0.),(0.499999,0.),(0.5,1.),(1.49,1.),(1.5,2.),(2.5,3.),(7.9,8.)] {assert_eq!(combat::nearest_damage(n),want);}
    let current=room(51.);let legacy=room(50.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&current,30.,0.75,10.,false,false),6.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&legacy,30.,0.75,10.,false,false),7.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&current,30.,1.,100.,false,false),0.);
    assert_eq!(combat::tuned_damage_for_run(&current,4.1,Some("fire")),1.);
    assert_eq!(combat::tuned_damage(4.1,Some("fire")),2.);
    assert_eq!(combat::integer_stat(-1.5),-2.);
}
#[test]
fn revision51_all_packet_kinds_and_exact_packets_use_nearest() {
    for kind in ["attack","fire","caustic","ichor","breatherGas","fall","bleeding","lightning","cold","acid","water","roots","razorleaves","epidite","blood-weapon","blood-letting","venom","poison","hemophilia","banshee",""] {
        for exact in [false,true] {
            let mut r=room(51.);let player=r.player.id;
            let factor=if exact {1.}else{match kind {"fire"|"caustic"|"ichor"|"breatherGas"|"fall"=>0.25,"attack"|"bleeding"=>0.5,_=>1.}};
            for (packet,want) in [(2.4,2.),(2.5,3.),(0.4,0.)] {
                let hp=r.player.hp;engine::damage_environment(&mut r,player,packet/factor,None,kind,exact).unwrap();
                close(hp-r.player.hp,want);
            }
        }
    }
}
#[test]
fn revision51_resistance_precedes_the_only_round_and_keeps_legacy_fractions() {
    for (kind,effect) in [("fire",rings::FIRE_RESISTANCE),("poison",rings::POISON_RESISTANCE),("venom",rings::POISON_RESISTANCE),("acid",rings::CORROSION_RESISTANCE),("caustic",rings::CORROSION_RESISTANCE),("ichor",rings::CORROSION_RESISTANCE),("breatherGas",rings::CORROSION_RESISTANCE)] {
        for (revision,want) in [(50.,1.5),(51.,1.)] {
            let mut r=room(revision);r.inventory.push(ring(effect));let id=r.player.id;
            engine::damage_environment(&mut r,id,2.9,None,kind,true).unwrap();
            close(100.-r.player.hp,want);
        }
    }
    let mut r=room(51.);r.inventory.push(ring(rings::FIRE_RESISTANCE));let id=r.player.id;
    engine::damage_environment(&mut r,id,9.2,None,"fire",false).unwrap();assert_eq!(r.player.hp,99.);
    engine::damage_environment(&mut r,id,12.,None,"fire",false).unwrap();assert_eq!(r.player.hp,97.);
}
#[test]
fn revision51_poison_tick_hemophilia_and_blood_weapon_use_matching_values() {
    let mut r=room(51.);r.inventory.push(ring(rings::POISON_RESISTANCE));let id=r.player.id;
    r.player.conditions.insert("poisoned".into(),5.);
    r.player.conditions.insert(corpse_core::revision50_amulet_effects::POISON_DAMAGE.into(),2.9);
    monster_statuses::tick_poisoned(&mut r,id).unwrap();assert_eq!(r.player.hp,99.);
    r.player.max_hp=101.;assert_eq!(monster_statuses::hemophilia_episode_damage_for_run(&r,&r.player),2.);
    assert_eq!(monster_statuses::hemophilia_episode_damage(&r.player),3.);
    r.player.max_hp=125.;assert_eq!(monster_statuses::hemophilia_episode_damage_for_run(&r,&r.player),3.);
    r.player.max_hp=104.;assert_eq!(corpse_core::divine_combat::blood_weapon_hp_cost_for_run(&r,&r.player),10.);
    assert_eq!(corpse_core::divine_combat::blood_weapon_hp_cost(&r.player),11.);
    r.player.conditions.clear();r.player.abilities.push("lichor-blood-weapon".into());
    corpse_core::abilities::activate(&mut r,"lichor-blood-weapon",&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
    assert_eq!(r.player.hp,89.);
}
#[test]
fn revision51_gas_vulnerability_and_resistance_round_once() {
    for (revision,want) in [(50.,3.5),(51.,3.)] {
        let mut r=room(revision);r.inventory.push(ring(rings::CORROSION_RESISTANCE));
        let at=10*r.floor().unwrap().width()+10;
        environment::add_gas(r.floor_mut().unwrap(),at,"caustic",1000.,-1.).unwrap();
        environment::tick_environment(&mut r).unwrap();close(100.-r.player.hp,want);
    }
    let mut r=room(51.);r.player.max_hp=101.;r.inventory.push(ring(rings::CORROSION_RESISTANCE));
    r.player.conditions.insert("gasVulnerable".into(),1.);
    let at=10*r.floor().unwrap().width()+10;
    environment::add_gas(r.floor_mut().unwrap(),at,"breather",1000.,-1.).unwrap();
    environment::tick_environment(&mut r).unwrap();assert_eq!(r.player.hp,93.);
}
#[test]
fn revision51_explosion_aoe_rounds_each_victim_and_preserves_immunity() {
    let mut r=room(51.);r.player.max_hp=100.8;
    let mut a=enemy(901.,11.,10.);a.max_hp=101.;r.actors_at_mut(1.).push(a);
    let mut immune=enemy(902.,10.,11.);immune.abilities.push("enemy:beast:explosion-immunity".into());r.actors_at_mut(1.).push(immune);
    environment::explode(&mut r,10.,10.,environment::ExplosionOptions{radius:2.,ignite_area:false,audible:false,..Default::default()},None).unwrap();
    assert_eq!(r.player.hp,50.);assert_eq!(engine::actor(&r,901.).unwrap().hp,49.);assert_eq!(engine::actor(&r,902.).unwrap().hp,100.);
}
#[test]
fn revision51_bonded_packets_round_their_half_independently() {
    let mut r=room(51.);let id=r.player.id;r.actors_at_mut(1.).push(enemy(901.,11.,10.));
    r.player.abilities.push("morae-bonded-by-unseen-threads".into());r.player.active.push("morae-bonded-by-unseen-threads".into());
    r.player.conditions.insert("bondedTarget".into(),901.);
    engine::damage_environment(&mut r,id,5.,None,"poison",true).unwrap();
    assert_eq!(r.player.hp,95.);assert_eq!(engine::actor(&r,901.).unwrap().hp,97.);
}
#[test]
fn revision51_attack_rng_is_unchanged_and_ordinary_health_delta_is_integer() {
    let mut legacy=room(50.);legacy.actors_at_mut(1.).push(enemy(901.,11.,10.));
    let mut current=legacy.clone();current.simulation_revision=51.0.into();let id=current.player.id;
    let mut prediction=current.clone();corpse_core::rng::random(&mut prediction);let variance=0.75+corpse_core::rng::random(&mut prediction)*0.5;
    engine::attack(&mut legacy,id,901.,1.).unwrap();engine::attack(&mut current,id,901.,1.).unwrap();
    let raw=corpse_core::revision50_player_attack::base(&current)*variance*0.5;
    assert_eq!(100.-engine::actor(&current,901.).unwrap().hp,combat::nearest_damage(raw));
    assert_eq!(100.-engine::actor(&legacy,901.).unwrap().hp,
        (corpse_core::revision50_player_attack::base(&legacy)*variance*0.5).ceil());assert_eq!(current.rng,legacy.rng);
}
#[test]
fn revision51_fractional_healing_and_curse_duration_remain_unchanged() {
    let mut r=room(51.);r.player.abilities.push(corpse_core::divine_combat::FIGHT_LONG.into());r.player.hp=90.;let a=r.player.clone();
    corpse_core::divine_combat::heal_melee_damage(&mut r,&a,3.,true);assert_eq!(r.player.hp,90.6);
    r.inventory.push(ring(rings::WILLPOWER));assert_eq!(monster_statuses::willpower_duration(&r,&r.player,5.),3.);
}
#[test]
fn revision51_save_snapshot_and_fixture_replay_preserve_damage_policy() {
    let mut r=room(51.);storage::validate(&r).unwrap();let saved=storage::serialize(&r).unwrap();
    let loaded=storage::deserialize_compatible(&saved).unwrap();assert_eq!(loaded.simulation_revision,Field::Value(51.));
    let snapshot=Snapshot::capture(&r);let id=r.player.id;
    engine::damage_environment(&mut r,id,2.4,None,"poison",true).unwrap();assert_eq!(r.player.hp,98.);
    snapshot.restore(&mut r).unwrap();assert_eq!(r.player.hp,100.);assert_eq!(r.simulation_revision,Field::Value(51.));
    let mut engine=corpse_core::temporal::Engine::new(loaded.clone()).unwrap();engine.dispatch(&corpse_core::Command::Wait).unwrap();
    let record=corpse_core::replay::ReplayRecord::try_from_run(&engine.run).unwrap();
    let mut replay=corpse_core::replay::ReplaySession::from_initial(loaded,record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(storage::serialize(&replay.engine.run).unwrap(),storage::serialize(&engine.run).unwrap());
}
#[test]
fn revision51_terminal_strike_stays_lethal_below_half_hp_and_displays_integer() {
    let mut r=room(51.);let id=r.player.id;
    r.player.abilities.push(corpse_core::revision50_combat::EXECUTE.into());r.player.active.push(corpse_core::revision50_combat::EXECUTE.into());
    corpse_core::revision50_ability_energy::acquire(&mut r.player,&[corpse_core::revision50_combat::EXECUTE.into()]).unwrap();
    let mut target=enemy(901.,11.,10.);target.hp=0.4;target.stats.dr=1e6;r.actors_at_mut(1.).push(target);
    engine::attack(&mut r,id,901.,1.).unwrap();
    assert!(engine::actor(&r,901.).unwrap().dead.value_or(false));
    assert!(r.log.iter().any(|s|s.contains("for 0 damage")));assert!(!r.log.iter().any(|s|s.contains("0.4 damage")));
}
#[test]
fn revision51_shared_body_resistance_rounds_once_for_the_owner() {
    let mut r=room(51.);r.inventory.push(ring(rings::POISON_RESISTANCE));
    r.player.abilities.push(corpse_core::half_here::ID.into());
    let clone=corpse_core::half_here::create(&mut r,9.,10.).unwrap();
    engine::damage_environment(&mut r,clone,2.9,None,"poison",true).unwrap();
    assert_eq!(r.player.hp,99.);assert_eq!(engine::actor(&r,clone).unwrap().hp,99.);
}
#[test]
fn revision51_offscreen_status_packets_round_without_changing_the_clock() {
    let mut r=room(51.);r.tick=1.;let mut floor=r.floor().unwrap().clone();floor.depth=2.;floor.last_tick=0.;floor.dirty=true;
    let at=r.floors.iter().position(|f|f.depth==2.).unwrap();r.floors[at]=std::sync::Arc::new(floor);
    let mut target=enemy(901.,11.,10.);target.state="neutral".into();target.conditions.insert("poisoned".into(),5.);
    target.conditions.insert(corpse_core::revision50_amulet_effects::POISON_DAMAGE.into(),2.4);
    r.actors_at_mut(2.).push(target);let mut legacy=r.clone();legacy.simulation_revision=50.0.into();
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();corpse_core::offscreen::catch_up_floor(&mut legacy,2.).unwrap();
    let hp=r.actors_at(2.)[0].hp;assert_eq!(hp-legacy.actors_at(2.)[0].hp,1.);
    assert_eq!(r.player.hp,100.);assert_eq!(r.floors[at].last_tick,legacy.floors[at].last_tick);
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();assert_eq!(r.actors_at(2.)[0].hp,hp);
}
