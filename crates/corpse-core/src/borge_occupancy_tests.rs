use super::*;
use crate::Stats;
const ENEMY:f64=101.;
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();for a in r.actors.values_mut(){*a=Default::default();}r.next_id=200.;r.offers.clear();
    r.player.x=14.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.godbeast_event=GodbeastEvent{kind:"Borge".into(),x:11.,y:7.,altar:GodbeastEventTap{x:11.,y:10.}.into(),..Default::default()}.into();
    r.actors_at_mut(1.).push(Actor{id:ENEMY,name:"Test enemy".into(),x:10.,y:10.,hp:100.,max_hp:100.,energy:100.,stats:Stats{max_energy:100.,..Default::default()},state:"hunting".into(),..Default::default()});r
}
#[test] fn borge_occupancy_blocks_movement_flight_phase_and_forced_entry() {
    for mode in ["ordinary","flight","phase","forced"] {
        let mut r=room();let a=engine::actor_mut_trusted(&mut r,ENEMY).unwrap();
        match mode {"flight"=>a.abilities.push("enemy:beast:flight".into()),"phase"=>a.active.push("etill-phase-through".into()),"forced"=>{a.conditions.insert("forcedMove".into(),1.);},_=>{}}
        let a=engine::actor(&r,ENEMY).unwrap();assert!(!engine::actor_passable(&r,a,11.,10.));assert!(!crate::travel::can_traverse(&r,a,11.,10.));
        let rng=r.rng;assert!(!engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap());assert_eq!(r.rng,rng);
        assert!(interaction_at(&r,11.,10.));assert!(engine::actor_passable(&r,&r.player,11.,10.));
    }
}
#[test] fn borge_occupancy_teleport_spawn_and_revival_block_without_rng() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles.fill("wall".into());
    for x in [10,11] {r.floor_mut().unwrap().tiles[10*w+x]="floor".into();}
    let rng=r.rng;crate::revision50_wave_teleport::teleport(&mut r,ENEMY).unwrap();assert_eq!(engine::actor(&r,ENEMY).unwrap().x,10.);assert_eq!(r.rng,rng);
    let mut rng=crate::rng::GenerationRng::new(123);let before=rng.state;let mut id=500.;
    assert!(crate::population::create_population_actor_revision(&mut id,r.floor_mut().unwrap(),"rat",10*w+11,false,&mut rng,52.).is_err());assert_eq!(id,500.);assert_eq!(rng.state,before);
    let mut a=engine::actor(&r,ENEMY).unwrap().clone();a.x=11.;assert!(crate::revision51_stairs::blocks_revival(&r,r.floor().unwrap(),&a));
}
#[test] fn borge_occupancy_loaded_hostiles_relocate_deterministically() {
    let mut r=room();engine::actor_mut_trusted(&mut r,ENEMY).unwrap().x=11.;let rng=r.rng;
    let raw=crate::storage::serialize(&r).unwrap();let a=crate::storage::deserialize(&raw).unwrap();let b=crate::storage::deserialize(&raw).unwrap();
    assert_eq!(a,b);assert_eq!(a.rng,rng);let actor=engine::actor(&a,ENEMY).unwrap();assert!(!reserved_cell(a.floor().unwrap(),actor.y as usize*a.floor().unwrap().width()+actor.x as usize));
    assert_eq!((a.player.x,a.player.y),(r.player.x,r.player.y));
}
#[test] fn borge_occupancy_legacy_and_allied_altar_unchanged() {
    let mut r=room();r.simulation_revision=51.0.into();assert!(engine::actor_passable(&r,engine::actor(&r,ENEMY).unwrap(),11.,10.));
    r.simulation_revision=52.0.into();engine::actor_mut_trusted(&mut r,ENEMY).unwrap().state="ally".into();assert!(engine::actor_passable(&r,engine::actor(&r,ENEMY).unwrap(),11.,10.));
    assert!(!engine::actor_passable(&r,&r.player,11.,7.));
}
