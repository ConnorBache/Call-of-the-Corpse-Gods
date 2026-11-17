use corpse_core::{Run,Item,Actor,Field,ai,corpses,revision52_enemy_followups as follow};
fn room(copies:usize)->Run{
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=52.0.into();r.ring_rules_from=0usize.into();
 r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r.player.hp=500.;r.player.max_hp=500.;r.actors_at_mut(r.depth).clear();
 r.inventory=(0..copies).map(|i|Item{id:900.+i as f64,kind:"ring".into(),effect:"willpower".to_string().into(),..Default::default()}).collect();r
}
#[test]fn burn_scales_new_duration_not_existing_duration(){
 for (copies,want) in [(0,10.),(1,5.),(2,3.)] {
  let mut r=room(copies);let id=r.player.id;
  assert_eq!(corpses::apply_burning_run(&mut r,id,52.,0.99,None,None,None),want);
  r.player.conditions.insert("burning".into(),9.);
  corpses::apply_burning_run(&mut r,id,52.,0.,None,None,None);
  assert_eq!(r.player.conditions["burning"],9.);
 }
 let mut r=room(2);r.ring_rules_from=Field::Null;let id=r.player.id;
 assert_eq!(corpses::apply_burning_run(&mut r,id,52.,0.99,None,None,None),10.);
}
#[test]fn venom_deadline_and_refresh_use_reduced_duration(){
 let mut r=room(1);let id=r.player.id;
 let a=Actor{id:800.,hp:100.,max_hp:100.,abilities:vec!["enemy:beast:venom".into()],..Default::default()};
 r.actors_at_mut(r.depth).push(a);ai::enemy_hit_effects(&mut r,800.,id);
 assert_eq!(r.player.conditions["envenomed"],3.);
 assert!(r.player.conditions.contains_key(&format!("venom:{}:800",r.tick+3.)));
 r.tick+=1.;ai::enemy_hit_effects(&mut r,800.,id);
 assert_eq!(r.player.conditions[&format!("venom:{}:800",r.tick+3.)],4.);
}
#[test]fn killspawn_shortens_real_packets_without_compressing_damage(){
 for (copies,turns,total) in [(0,3.,15.),(1,2.,10.),(2,1.,5.)] {
  let mut r=room(copies);let id=r.player.id;
  let a=Actor{id:800.,creature_kind:"killspawn".to_string().into(),hp:100.,max_hp:100.,..Default::default()};
  follow::after_melee(&mut r,&a,id,30.,true).unwrap();
  assert_eq!(r.player.conditions[follow::BLEED],turns);
  let buckets:Vec<_>=r.player.conditions.iter().filter(|(k,_)|k.starts_with(follow::BLEED_PREFIX)).collect();
  assert_eq!(buckets.len(),turns as usize);assert_eq!(buckets.iter().map(|(_,v)|**v).sum::<f64>(),total);
 }
}
