use corpse_core::{Run,Item,Field,rng::GenerationRng,revision50_rings as rings};
fn room()->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=52.0.into();r.ring_rules_from=0usize.into();r.inventory.clear();r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r
}
fn ring(effect:&str)->Item {Item{id:900.,kind:"ring".into(),effect:effect.to_string().into(),..Default::default()}}
#[test]fn removed_rings_keep_identity_but_no_named_effects_in_current_runs(){
 let mut r=room();
 for (effect,kind) in [(rings::FIRE_RESISTANCE,"fire"),(rings::POISON_RESISTANCE,"venom"),(rings::SUSTAIN_STRENGTH,"physical"),(rings::ICE_IMMUNITY,"cold"),(rings::CORROSION_RESISTANCE,"acid")] {
  let item=ring(effect);r.inventory=vec![item.clone()];
  assert!(rings::retired(&r,&item));assert_eq!(rings::count_effect(&r,&r.player,effect),0);
  assert_eq!(rings::resist_damage(&r,&r.player,kind,80.),80.);
  assert!(corpse_core::revision50_item_knowledge::display_name(&r,&item).starts_with("Retired Ring of "));
  assert_eq!(rings::description_for_run(&r,&item),Some("Retired ring."));
  let loaded:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();assert_eq!(loaded.inventory,r.inventory);
  r.ring_rules_from=Field::Null;assert!(!rings::retired(&r,&item));assert_eq!(rings::count_effect(&r,&r.player,effect),1);r.ring_rules_from=0usize.into();
 }
}
#[test]fn current_ordinary_ring_pool_includes_swimming_and_excludes_retired(){
 let mut seen=std::collections::BTreeSet::new();
 for seed in 0..512 {
  let mut rng=GenerationRng::new(seed);let mut id=100.;
  let item=corpse_core::equipment::create_equipment_revision("ring",&mut rng,&mut id,8,&[],52.).unwrap();
  let effect=rings::effect(&item).unwrap();assert!(!rings::removed_effect(effect));seen.insert(effect);
 }
 assert_eq!(seen.len(),10);assert!(seen.contains("swimming"));
}
#[test]fn production_early_and_deep_loot_respects_ring_policy(){
 let base=room();
 for legacy in [false,true] {let mut seen=std::collections::BTreeSet::new();
  for seed in 0..256 {for depth in [1,2,3,4,10] {
   let mut r=base.clone();r.seed=format!("ring-pool-{seed}");if legacy {r.ring_rules_from=Field::Null;}
   let items=corpse_core::equipment::generate_floor_loot_for_run(&mut r,depth,&mut GenerationRng::new(seed),&[],10).unwrap();
   for item in items {if let Some(effect)=rings::effect(&item) {seen.insert(effect);if !legacy {assert!(!rings::removed_effect(effect));}}}
  }}
  assert_eq!(seen.len(),if legacy {14}else{10});assert_eq!(seen.contains("swimming"),!legacy);
 }
}
#[test]fn willpower_copy_is_policy_aware(){let mut r=room();let item=ring(rings::WILLPOWER);assert_eq!(rings::description_for_run(&r,&item),Some("Halves the duration of all status effects."));r.ring_rules_from=Field::Null;assert!(rings::description_for_run(&r,&item).unwrap().contains("enemy curses"));}
