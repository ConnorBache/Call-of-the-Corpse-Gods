//! Revision-37 overlays are state, not render-only effects.
use corpse_core::{Field,Run,storage,temporal::{Snapshot,History},sub_biomes_validation};
fn room()->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=37.0.into();
 let f=r.floor_mut().unwrap();let n=f.tiles.len();
 for (layer,at) in [(&mut f.ice,800),(&mut f.puddles,801),(&mut f.hellfire,802),(&mut f.remembered_ice,800),(&mut f.remembered_puddles,801),(&mut f.remembered_hellfire,802)] {
  let mut v=vec![false;n];v[at]=true;*layer=v.into();
 }
 r
}
#[test]
fn persisted_overlays_and_knowledge_roundtrip_without_new_rng() {
 let r=room();let saved=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&saved).unwrap();
 assert_eq!(loaded.floor().unwrap(),r.floor().unwrap());assert_eq!(loaded.rng,r.rng);
 assert_eq!(storage::serialize(&loaded).unwrap(),saved);
}
#[test]
fn rejects_short_null_or_legacy_new_layers() {
 let mut r=room();assert!(sub_biomes_validation::validate_run(&r));
 r.floor_mut().unwrap().ice=vec![true].into();assert!(!sub_biomes_validation::validate_run(&r));
 let mut r=room();r.floor_mut().unwrap().hellfire=Field::Null;assert!(!sub_biomes_validation::validate_run(&r));
 let mut r=room();r.simulation_revision=36.0.into();assert!(!sub_biomes_validation::validate_run(&r));
 let mut r=room();let f=r.floor_mut().unwrap();
 for l in [&mut f.ice,&mut f.puddles,&mut f.hellfire,&mut f.remembered_ice,&mut f.remembered_puddles,&mut f.remembered_hellfire]{*l=Field::Missing;}
 r.simulation_revision=36.0.into();assert!(sub_biomes_validation::validate_run(&r));
}
#[test]
fn revert_and_future_restore_melted_snow_and_permanent_fire_flags() {
 let mut r=room();r.player.conditions.insert("revertCharges".into(),3.);
 let original=r.floor().unwrap().clone();let opening=Snapshot::capture(&r);
 let mut history=History::default();history.record_completed(opening);
 r.tick+=1.;r.floor_mut().unwrap().ice.as_mut().unwrap()[800]=false;
 r.floor_mut().unwrap().puddles.as_mut().unwrap()[803]=true;
 history.closing=Some(Snapshot::capture(&r));
 assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());assert_eq!(r.floor().unwrap().ice,original.ice);assert_eq!(r.floor().unwrap().puddles,original.puddles);assert_eq!(r.floor().unwrap().hellfire,original.hellfire);
 corpse_core::temporal::remember_future(&mut r,&history).unwrap();
 r.floor_mut().unwrap().ice.as_mut().unwrap()[800]=false;
 r.floor_mut().unwrap().puddles.as_mut().unwrap()[803]=true;
 assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());assert_eq!(r.floor().unwrap().ice,original.ice);assert_eq!(r.floor().unwrap().puddles,original.puddles);assert_eq!(r.floor().unwrap().hellfire,original.hellfire);
}
