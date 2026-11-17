use corpse_core::{Field, Item, Run, Stats, engine, environment, monster_statuses as statuses, revision50_rings as rings};
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
fn dose(r:&mut Run,effect:&str){let i=r.inventory.len();r.inventory.push(Item{id:999.,kind:"potion".into(),effect:effect.to_string().into(),..Default::default()});let (x,y)=(r.player.x,r.player.y);assert!(corpse_core::potions::apply(r,i,x,y,false).unwrap());}
#[test]fn beneficial_potions_and_stat_buff_timers_scale_but_not_magnitude(){
 for (effect,key,duration) in [("telepathy","telepathic",30.),("levitation","flying",20.),("speed","hasted",10.),("invisibility","invisible",20.),("detect magic","magicSensitive",20.),("fire immunity","r50FireImmunity",20.)] {
  for copies in 0..=2 {let mut r=current(copies);dose(&mut r,effect);assert_eq!(engine::condition(&r.player,key),(duration*0.5f64.powi(copies as i32)).ceil(),"{effect}/{copies}");}
 }
 let mut r=current(2);dose(&mut r,"accuracy");assert_eq!(engine::condition(&r.player,"buffTicks:accuracy"),5.);assert_eq!(engine::condition(&r.player,"buff:accuracy"),5.);
}
#[test]fn reapplication_scales_incoming_not_remaining_and_legacy_is_unchanged(){
 let mut r=current(1);r.player.conditions.insert("telepathic".into(),24.);dose(&mut r,"telepathy");assert_eq!(engine::condition(&r.player,"telepathic"),24.);
 r.ring_rules_from=Field::Null;r.player.conditions.clear();dose(&mut r,"telepathy");assert_eq!(engine::condition(&r.player,"telepathic"),30.);
}
#[test]fn helper_rounding_permanent_sentinels_and_half_here_owner(){
 let r=current(2);assert_eq!(statuses::status_duration(&r,&r.player,5.),2.);assert_eq!(statuses::status_duration(&r,&r.player,1.),1.);
 for n in [0.,-1.,f64::INFINITY] {assert_eq!(statuses::status_duration(&r,&r.player,n),n);}
 let mut endpoint=r.player.clone();endpoint.id=777.;endpoint.conditions.insert("halfHereClone".into(),1.);endpoint.conditions.insert("halfHereOwner".into(),r.player.id);
 assert_eq!(statuses::status_duration(&r,&endpoint,8.),2.);
 endpoint.conditions.insert("miniMe".into(),1.);assert_eq!(statuses::status_duration(&r,&endpoint,8.),8.);
}
#[test]fn current_curse_is_scaled_once_and_neutral_status_is_also_scaled(){
 let mut r=current(1);let mut enemy=r.player.clone();enemy.id=777.;enemy.state="hunting".into();enemy.x+=1.;r.actors_at_mut(r.depth).push(enemy);let id=r.player.id;
 assert!(statuses::apply_curse(&mut r,id,"poisoned",12.,777.));assert_eq!(engine::condition(&r.player,"poisoned"),6.);
 assert!(statuses::apply_status(&mut r,id,"hemophilia",5.,None));assert_eq!(engine::condition(&r.player,"hemophilia"),3.);
}
#[test]fn gas_contact_refresh_does_not_repeatedly_halve_countdowns(){
 for (kind,key,normal) in [("paralysis","paralyzed",5.),("confusion","confused",5.),("hallucinogenic","hallucinating",5.)] {
  let mut r=current(1);r.gas_rules_from=0usize.into();let at=10*r.floor().unwrap().width()+10;
  environment::add_gas(r.floor_mut().unwrap(),at,kind,1000.,-1.).unwrap();
  environment::tick_environment(&mut r).unwrap();
  let first=engine::condition(&r.player,key);assert!(first>0. && first<=(normal/2.0_f64).ceil(),"{kind}: {first}");
  environment::tick_environment(&mut r).unwrap();assert_eq!(engine::condition(&r.player,key),first,"{kind}");
 }
}

#[test]fn poison_amulet_duration_uses_recipient_rings_not_caster_and_legacy_is_unchanged(){
 use corpse_core::revision50_amulet_effects as effects;
 use corpse_core::revision50_amulet_effects::Effect;
 for (caster_copies,target_copies,legacy,expected) in [(2,0,false,20.),(0,1,false,10.),(2,2,false,5.),(0,2,true,20.)] {
  let mut r=current(caster_copies);if legacy {r.ring_rules_from=Field::Null;}
  r.player.energy=100.;r.floor_mut().unwrap().seen.fill(true);
  let mut a=r.player.clone();a.id=777.;a.x=12.;a.state="hunting".into();a.hp=1000.;a.max_hp=1000.;a.conditions.clear();
  a.copied_inventory=(0..target_copies).map(|i|ring(300.+i as f64,rings::WILLPOWER)).collect::<Vec<_>>().into();
  r.actors_at_mut(r.depth).push(a);
  r.inventory.push(effects::make_amulet_for_revision(Effect::Poison,999.,52.).unwrap());
  assert_eq!(effects::command(&mut r,&corpse_core::Command::UseAt{id:999.,x:12.,y:10.}).unwrap(),Some(true));
  assert_eq!(engine::condition(engine::actor(&r,777.).unwrap(),"poisoned"),expected,"caster {caster_copies}, recipient {target_copies}, legacy {legacy}");
 }
}

#[test]fn gas_and_ring_policies_are_independent_in_all_four_combinations(){
 for current_gas in [false,true] {for current_ring in [false,true] {
  for (gas,key,old) in [("paralysis","paralyzed",2.),("confusion","confused",8.),("hallucinogenic","hallucinating",2.)] {
   let mut r=current(1);r.gas_rules_from=if current_gas {0usize.into()}else{Field::Null};r.ring_rules_from=if current_ring {0usize.into()}else{Field::Null};
   let at=10*r.floor().unwrap().width()+10;environment::add_gas(r.floor_mut().unwrap(),at,gas,1000.,-1.).unwrap();
   environment::tick_environment(&mut r).unwrap();
   let duration: f64=if current_gas {5.}else{old};
   let expected=if current_ring || key!="paralyzed" {(duration/2.).ceil()}else{duration};
   assert_eq!(engine::condition(&r.player,key),expected,"gas {gas}, current_gas {current_gas}, current_ring {current_ring}");
  }
 }}
}
