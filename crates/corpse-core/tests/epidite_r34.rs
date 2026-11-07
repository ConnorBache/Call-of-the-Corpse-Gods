use corpse_core::{Field, GroundItem, Run, engine, epidite, generation, inventory, population, storage};
fn room() -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1);r.actors.retain(|k,_|k=="1");r.ground.retain(|k,_|k=="1");
    r.simulation_revision=34.0.into();r.actors_at_mut(1.).clear();r.ground_at_mut(1.).clear();r.inventory.clear();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=8.;r.player.y=8.;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();r.player.stats.dr=10000.;
    r.next_id=1000.;r.phase=Field::Missing;r.offers.clear();
    let f=r.floor_mut().unwrap();f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;
    f.godbeast_event=Field::Missing;f.doors=Field::Missing;f.grates=Field::Missing;
    f.tiles=vec!["floor".into();2291];f.lights=vec![0.;2291];
    for at in 0..2291 {if at%79==0 || at%79==78 || at/79==0 || at/79==28 {f.tiles[at]="wall".into();}}
    f.tiles[80]="up".into();f.tiles[27*79+77]="down".into();f.tiles[5*79+5]="wall".into();
    f.godbeast_event=corpse_core::GodbeastEvent{kind:"Epidite".into(),x:5.,y:5.,epidite_age:0.0.into(),
        room:corpse_core::GodbeastEventRoom{left:1.,right:9.,top:1.,bottom:9.},..Default::default()}.into();
    r
}
fn heart(id:f64)->corpse_core::Item {inventory::heart_for(&population::create_actor("pale:Jackal",0.,0.,0.,false).unwrap(),id).unwrap()}
fn ground(r:&mut Run,item:corpse_core::Item,x:f64,y:f64) {r.ground_at_mut(1.).push(GroundItem{x,y,item,..Default::default()});}
#[test]
fn binary_blessing_doubles_stats_exp_energy_preserves_identity_provenance_quantity_and_stacks() {
    let mut i=heart(100.);i.quantity=3.0.into();i.source_actor_id=7.0.into();i.borge_depth=5.0.into();
    let old=i.clone();let species=inventory::heart_creature_type(&i);
    assert!(epidite::bless(&mut i));assert!(i.blessed.value_or(false));assert!(i.name.starts_with("Blessed "));
    assert_eq!(i.exp.value_or(0.),old.exp.value_or(0.)*2.);assert_eq!(i.energy.value_or(0.),old.energy.value_or(0.)*2.);
    let oldb=serde_json::to_value(old.bonus.as_ref().unwrap()).unwrap();let newb=serde_json::to_value(i.bonus.as_ref().unwrap()).unwrap();
    for (key,value) in oldb.as_object().unwrap() {assert_eq!(newb[key].as_f64().unwrap(),value.as_f64().unwrap()*2.);}
    assert_eq!(inventory::heart_creature_type(&i),species);assert_eq!(i.quantity,old.quantity);
    assert_eq!(i.source_actor_id,old.source_actor_id);assert_eq!(i.borge_depth,old.borge_depth);
    let once=i.clone();assert!(!epidite::bless(&mut i));assert_eq!(i,once);
    let unblessed=heart(200.);let mut blessed=unblessed.clone();epidite::bless(&mut blessed);
    assert_ne!(inventory::stack_key(&unblessed),inventory::stack_key(&blessed));
    assert!(!epidite::bless(&mut corpse_core::phantasmii_generation::shard(500.)));
}
#[test]
fn flash_every_thirty_world_turns_uses_los_bypasses_dr_and_blesses_only_exposed_ground() {
    let mut r=room();r.player.x=7.;r.player.y=5.;
    let mut victim=population::create_actor("rat",10.,5.,7.,false).unwrap();victim.hp=500.;victim.max_hp=500.;victim.stats.dr=10000.;r.actors_at_mut(1.).push(victim);
    let mut hidden=population::create_actor("rat",11.,3.,5.,false).unwrap();hidden.hp=500.;hidden.max_hp=500.;r.actors_at_mut(1.).push(hidden);
    r.floor_mut().unwrap().tiles[5*79+4]="wall".into();
    ground(&mut r,heart(100.),5.,6.);ground(&mut r,heart(101.),3.,5.);r.inventory.push(heart(102.));
    for _ in 0..29 {epidite::world_turn(&mut r).unwrap();}
    assert_eq!(epidite::countdown(r.floor().unwrap().godbeast_event.as_ref().unwrap()),1);
    assert_eq!(r.player.hp,1000.);assert!(!r.ground["1"][0].item.blessed.value_or(false));
    epidite::world_turn(&mut r).unwrap();
    assert_eq!(epidite::countdown(r.floor().unwrap().godbeast_event.as_ref().unwrap()),30);
    assert_eq!(r.player.hp,900.);assert_eq!(engine::actor(&r,10.).unwrap().hp,400.);assert_eq!(engine::actor(&r,11.).unwrap().hp,500.);
    assert!(r.ground["1"][0].item.blessed.value_or(false));assert!(!r.ground["1"][1].item.blessed.value_or(false));assert!(!r.inventory[0].blessed.value_or(false));
    let blessed=r.ground["1"][0].item.clone();for _ in 0..30 {epidite::world_turn(&mut r).unwrap();}
    assert_eq!(r.ground["1"][0].item,blessed);assert_eq!(r.player.hp,800.);
}
#[test]
fn pulse_ignores_dream_targets_and_new_death_hearts_wait_for_next_flash() {
    let mut r=room();let mut nightmare=corpse_core::phantasmii_generation::nightmare(1,20.,6.,5.);nightmare.hp=400.;nightmare.max_hp=400.;r.actors_at_mut(1.).push(nightmare);
    let victim=population::create_actor("rat",21.,5.,6.,false).unwrap();r.actors_at_mut(1.).push(victim);
    r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age=29.0.into();
    epidite::world_turn(&mut r).unwrap();assert_eq!(engine::actor(&r,20.).unwrap().hp,400.);
    assert!(engine::actor(&r,21.).unwrap().dead.value_or(false));
    let dropped=r.ground["1"].iter().find(|g|g.item.kind=="heart").unwrap();assert!(!dropped.item.blessed.value_or(false));
    for _ in 0..30 {epidite::world_turn(&mut r).unwrap();}
    assert!(r.ground["1"].iter().any(|g|g.item.kind=="heart" && g.item.blessed.value_or(false)));
}
#[test]
fn generation_tower_and_clock_blessed_state_roundtrip() {
    let seed=(0..1000).find(|s|generation::domain_plan_revision(*s,34.)[0].godbeast_kind=="Epidite").unwrap();
    for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        let f=generation::generate_floor_revision(5,seed,domain,true,79,None,34.).unwrap();let e=f.godbeast_event.as_ref().unwrap();
        assert_eq!(e.kind,"Epidite");assert!(epidite::solid_at(&f,e.x,e.y));assert_eq!(epidite::countdown(e),30);
        let reachable=generation::reachable_without_braziers(&f,None);assert!(generation::transitions_accessible(&f,&reachable));
    }
    let mut r=room();let mut i=heart(100.);epidite::bless(&mut i);r.inventory.push(i);
    for _ in 0..11 {epidite::world_turn(&mut r).unwrap();}
    let loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();assert_eq!(loaded,r);
    assert_eq!(epidite::countdown(loaded.floor().unwrap().godbeast_event.as_ref().unwrap()),19);
}
#[test]
fn actual_dispatch_and_offscreen_ticks_advance_the_saved_clock() {
    use corpse_core::{Command,temporal::Engine};
    let mut r=room();ground(&mut r,heart(100.),5.,6.);r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age=29.0.into();
    let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Wait).unwrap();
    assert_eq!(epidite::countdown(e.run.floor().unwrap().godbeast_event.as_ref().unwrap()),30);
    assert!(e.run.ground["1"][0].item.blessed.value_or(false));
    // A visited inactive tower remains exact, rather than entering stationary catch-up.
    let mut r=room();ground(&mut r,heart(100.),5.,6.);r.floor_mut().unwrap().seen.fill(true);
    r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age=29.0.into();
    let mut other=r.floor().unwrap().clone();other.depth=2.;other.godbeast_event=Field::Missing;r.floors.push(std::sync::Arc::new(other));r.actors.insert("2".into(),std::sync::Arc::new(vec![]));r.ground.insert("2".into(),std::sync::Arc::new(vec![]));r.depth=2.;r.tick=1.;
    corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();
    assert_eq!(epidite::countdown(r.floors[0].godbeast_event.as_ref().unwrap()),30);assert!(r.ground["1"][0].item.blessed.value_or(false));
}
