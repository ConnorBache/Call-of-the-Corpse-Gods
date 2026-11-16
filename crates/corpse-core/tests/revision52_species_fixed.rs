//! Same type means the same intrinsic birth body, regardless of layer or provenance.
use corpse_core::{balance52,population,Actor};
#[path="support/balance52_birth.rs"] mod birth;
fn body(a:&Actor)->serde_json::Value {serde_json::json!({"stats":a.stats,"hp":a.max_hp,"energy":a.energy,
"coefficients":(["monsterGasPassiveVolume","monsterGasBoostVolume","monsterGasShotVolume","monsterHealAmount","monsterSpecialDamage"].map(|k|a.conditions.get(k)))})}
#[test]
fn every_fixed_species_and_context_has_identical_body_on_all_forty_layers(){
 for p in balance52::profiles(){
  let reference=population::create_actor_revision(&p.kind,900.,3.,3.,false,52.).unwrap();
  for depth in 1..=40 {
   let mut floor=birth::floor(depth);let mut id=900.;let mut rng=population::population_random("species-fixed");
   let natural=population::create_population_actor_revision(&mut id,&mut floor,&p.kind,3*79+3,false,&mut rng,52.).unwrap();
   assert_eq!(body(&reference),body(&natural),"{} at{depth}",p.kind);
   for context in ["nest","queen","witch","birther","greater-birther"]{
    let mut a=Actor::default();assert!(balance52::apply_new_actor_kind(&mut a,&format!("{context}:{}",p.kind),depth as f64,52.));
    assert_eq!(body(&reference),body(&a),"{context}:{} at{depth}",p.kind);
   }
  }
 }
}
#[test]
fn actual_summons_and_nest_use_the_same_species_body(){
 for depth in [1,6,15,28,40]{
  for parent in ["pale:Birther","pale:Greater Birther","witch","saidribee-queen"]{
   for seed in 0..12 {
    let r=birth::summon_once(parent,depth,52.,seed,false);
    for a in r.actors_at(r.depth).iter().filter(|a|a.id!=900.) {
     let k=birth::natural_kind(a);let expected=population::create_actor_revision(&k,900.,3.,3.,false,52.).unwrap();
     assert_eq!(body(&expected),body(a),"{parent} -> {k} at{depth}");
    }
   }
  }
  for a in birth::nest(depth,52.).1 {let k=birth::natural_kind(&a);let expected=population::create_actor_revision(&k,900.,3.,3.,false,52.).unwrap();assert_eq!(body(&expected),body(&a),"nest {k} at{depth}");}
 }
}
#[test]
fn fungus_and_each_named_zillennia_quality_are_fixed(){
 let f=population::fungus_revision(1,52.).unwrap();let mut qualities=std::collections::BTreeMap::new();
 for depth in 1..=40 {
  assert_eq!(body(&f),body(&population::fungus_revision(depth,52.).unwrap()));
  let a=population::zillennia_revision(depth,52.).unwrap();let k=a.zillennia_quality.as_ref().unwrap().clone();
  let first=qualities.entry(k).or_insert_with(||body(&a));assert_eq!(*first,body(&a));
 }
}
#[test]
fn jackal_weakest_body_is_preserved_and_no_layer_lookup_changes_saved_creatures(){
 let a=population::create_actor_revision("pale:Jackal",900.,3.,3.,false,52.).unwrap();
 assert_eq!((a.max_hp,a.stats.offense,a.stats.accuracy,a.stats.dodge),(40.,26.,13.,10.));
 let mut r=birth::scene(1,52.,7);r.actors_at_mut(1.).push(a.clone());r.depth=40.;
 assert_eq!(r.actors_at(1.)[0],a);
}
