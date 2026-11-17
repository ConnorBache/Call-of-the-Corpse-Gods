use corpse_core::{Field, Item, Run, Stats, engine, environment, revision50_rings as rings};
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.energy = 37.;
    r.player.stats = Stats { offense: 10., dr: 2., accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear();
    // Empty floor contents without removing blocks required by save validation.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 900.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing; f.environment = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing; f.currents = Field::Missing;
    f.fungal_roots = Field::Missing; f.roots = Field::Missing; f.mud = Field::Missing;
    f.slime = Field::Missing; f.ichor = Field::Missing; f.razorleaves = Field::Missing;
    f.water_blood = Field::Missing; f.kelpie_blood_sources = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    r
}
fn ring(id: f64, effect: &str) -> Item {
    Item { id, name: format!("Ring of {effect}"), kind: "ring".into(), glyph: "=".into(),
        effect: effect.to_string().into(), ..Default::default() }
}

fn current(copies:usize)->Run {let mut r=room(52.);r.ring_rules_from=0usize.into();for i in 0..copies {r.inventory.push(ring(100.+i as f64,rings::WILLPOWER));}r}
const GASES: [(&str, &str, f64); 3] = [("paralysis", "paralyzed", 2.), ("confusion", "confused", 8.), ("hallucinogenic", "hallucinating", 2.)];
fn gas_run(copies: usize, gas: &str) -> Run {
 let mut r=current(copies); r.gas_rules_from=0usize.into();
 let at=10*r.floor().unwrap().width()+10;
 environment::add_gas(r.floor_mut().unwrap(),at,gas,1000.,-1.).unwrap(); r
}
#[test] fn every_status_gas_refreshes_five_turns_scaled_once() {
 for (gas,key,_) in GASES { for copies in 0..=3 {
  let mut r=gas_run(copies,gas);let expected=(5. * 0.5f64.powi(copies as i32)).ceil();
  for _ in 0..3 { environment::tick_environment(&mut r).unwrap(); assert_eq!(engine::condition(&r.player,key),expected,"{gas}, {copies}"); r.player.conditions.insert(key.into(),1.); }
 }}
}
#[test] fn contact_does_not_shorten_longer_effect_and_exit_expires() {
 for (gas,key,_) in GASES {
  let mut r=gas_run(0,gas);r.player.conditions.insert(key.into(),12.);
  environment::tick_environment(&mut r).unwrap();
  assert_eq!(engine::condition(&r.player,key),if key=="hallucinating" {11.} else {12.});
  r.player.conditions.insert(key.into(),1.);environment::tick_environment(&mut r).unwrap();
  assert_eq!(engine::condition(&r.player,key),5.);
  r.floor_mut().unwrap().environment=Field::Missing;
  for remaining in (0..5).rev() {
   // The two actor-clock statuses use the engine countdown; hallucination
   // is deliberately decremented by the environment/world clock.
   if key!="hallucinating" {engine::tick_condition(&mut r.player,key);}
   environment::tick_environment(&mut r).unwrap();
   assert_eq!(engine::condition(&r.player,key),remaining as f64,"{gas}");
  }
 }
}
#[test] fn enemies_receive_five_not_players_ring_reduction_and_hallucination_scope_is_unchanged() {
 for (gas,key,_) in GASES {
  let mut r=gas_run(3,gas);let mut a=r.player.clone();a.id=777.;a.state="sleeping".into();a.copied_inventory=Field::Missing;
  r.actors_at_mut(r.depth).push(a);environment::tick_environment(&mut r).unwrap();
  assert_eq!(engine::condition(engine::actor(&r,777.).unwrap(),key),if key=="hallucinating" {0.}else{5.});
 }
}
#[test] fn gas_immunity_still_blocks_all_status_gases() {
 for (gas,key,_) in GASES {let mut r=gas_run(0,gas);r.player.abilities.push("enemy:beast:gas-immunity".into());environment::tick_environment(&mut r).unwrap();assert_eq!(engine::condition(&r.player,key),0.);}
 let mut r=gas_run(0,"hallucinogenic");r.player.abilities.push("enemy:pale:gas-immunity".into());environment::tick_environment(&mut r).unwrap();assert_eq!(engine::condition(&r.player,"hallucinating"),0.);
}
#[test] fn historical_gas_durations_are_preserved() {
 for (gas,key,old) in GASES {let mut r=gas_run(0,gas);r.gas_rules_from=Field::Null;environment::tick_environment(&mut r).unwrap();assert_eq!(engine::condition(&r.player,key),old);}
}
#[test] fn non_status_gases_do_not_gain_statuses() {
 for gas in environment::GAS_TYPES {if GASES.iter().any(|(g,_,_)|*g==gas){continue;}let mut r=gas_run(0,gas);environment::tick_environment(&mut r).unwrap();for (_,key,_) in GASES {assert_eq!(engine::condition(&r.player,key),0.,"{gas}/{key}");}}
}
