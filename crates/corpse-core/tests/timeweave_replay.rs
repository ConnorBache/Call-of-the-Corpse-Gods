//! Exact new-rule current-state, trail, Future and seed replay boundaries.
use corpse_core::{Command,Field,Run,replay::{ReplayRecord,ReplaySession},temporal::Engine};
use std::sync::Arc;
fn initial()->Run{
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=41.0.into();r.player.x=10.;r.player.y=10.;r.player.hp=10000.;r.player.max_hp=10000.;r.player.base_max_hp=10000.0.into();r.player.energy=100.;r.player.stats.max_energy=100.;
 r.player.abilities=vec![corpse_core::progression::WEAVE_ID.into(),"morae-see-the-future".into()];r.player.active.clear();r.player.conditions.clear();r.offers.clear();r.replay.clear();r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r.phase=Field::Missing;r.replay_snapshot_from=Field::Missing;
 for a in r.actors.values_mut(){*a=Arc::new(vec![]);}for g in r.ground.values_mut(){*g=Arc::new(vec![]);}
 let f=r.floor_mut().unwrap();for tile in &mut f.tiles{*tile="floor".into();}f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;r
}
fn future()->Command{Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
/// O(trace simulation + explicit checkpoint/replay), all equality outside any timing harness.
#[test] fn trail_free_moves_future_and_restarted_seed_replay_are_exact(){
 let r=initial();let mut e=Engine::new(r.clone()).unwrap();
 let commands=[Command::Move{dx:1.,dy:0.},Command::Move{dx:-1.,dy:0.},future(),Command::Move{dx:0.,dy:1.},Command::Wait,future(),Command::Move{dx:0.,dy:1.}];
 for (i,c) in commands.iter().enumerate(){let out=e.dispatch(c).unwrap();if i==1{assert!(!out.took_turn);}assert!(e.history.frames.is_empty());assert!(e.history.pending_haste.is_none());e=Engine::load(&e.save().unwrap()).unwrap();}
 assert!(e.run.timeweave.as_ref().is_some_and(|t|!t.is_empty()));assert!(e.run.future.is_none());
 let expected=e.save().unwrap();let record=ReplayRecord::try_from_run(&e.run).unwrap();
 let mut replay=ReplaySession::from_initial(r,record).unwrap();while replay.step().unwrap(){}
 assert_eq!(replay.engine.save().unwrap(),expected);replay.restart().unwrap();while replay.step().unwrap(){}
 assert_eq!(replay.engine.save().unwrap(),expected);
}
