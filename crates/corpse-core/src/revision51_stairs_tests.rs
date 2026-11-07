use super::*;
use crate::{Field, GodbeastEvent};
const ENEMY: f64 = 101.;
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.actors = Default::default(); r.next_id = 200.;
    r.player.x = 14.; r.player.y = 10.; r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    let f = r.floor_mut().unwrap(); let w = f.width(); f.tiles.fill("wall".into());
    for y in 8..=12 { for x in 8..=16 { f.tiles[y*w+x] = "floor".into(); } }
    r.actors_at_mut(1.).push(Actor { id: ENEMY, name: "Test enemy".into(), x: 10., y: 10.,
        hp: 100., max_hp: 100., energy: 100., stats: crate::Stats {max_energy:100.,..Default::default()},
        state: "hunting".into(), ..Default::default() });
    r
}
fn set_tile(r: &mut Run, x: usize, y: usize, value: &str) {
    let f=r.floor_mut().unwrap(); let w=f.width(); f.tiles[y*w+x]=value.into();
}
#[test]
fn stairs_block_hostile_neutral_flying_phasing_and_forced_movement_without_rng() {
    for stair in ["up", "down"] { for mode in ["ordinary", "flight", "phase", "forced", "neutral"] {
        let mut r=room(51.);set_tile(&mut r,11,10,stair);
        let a=engine::actor_mut_trusted(&mut r,ENEMY).unwrap();
        match mode {
            "flight" => a.abilities.push("enemy:beast:flight".into()),
            "phase" => a.active.push("etill-phase-through".into()),
            "forced" => {a.conditions.insert("forcedMove".into(),1.);},
            "neutral" => a.state="neutral".into(), _ => {}
        }
        let a=engine::actor(&r,ENEMY).unwrap();
        assert!(!engine::actor_passable(&r,a,11.,10.));
        assert!(!crate::travel::can_traverse(&r,a,11.,10.));
        let rng=r.rng;
        assert!(!engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap(),"{mode} {stair}");
        assert_eq!(engine::actor(&r,ENEMY).unwrap().x,10.);assert_eq!(r.rng,rng);
    }}
}
#[test]
fn legacy_and_friendly_stair_movement_remain_legal() {
    for revision in [29.,50.,51.] { for state in ["hunting","ally"] {
        let mut r=room(revision);set_tile(&mut r,11,10,"up");
        engine::actor_mut_trusted(&mut r,ENEMY).unwrap().state=state.into();
        assert_eq!(engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap(),revision<51.||state=="ally");
    }}
    let mut r=room(51.);set_tile(&mut r,13,10,"up");
    assert!(engine::actor_passable(&r,&r.player,13.,10.));
    assert!(crate::travel::can_traverse(&r,&r.player,13.,10.));
}
#[test]
fn pathfinding_routes_around_stairs_instead_of_selecting_a_rejected_step() {
    let mut r=room(51.);set_tile(&mut r,11,10,"down");
    let step=engine::creature_path_step(&mut r,ENEMY,Point{x:14,y:10}).unwrap();
    assert_ne!(step,Point{x:1,y:0});
    assert!(engine::move_actor(&mut r,ENEMY,step.x as f64,step.y as f64,true).unwrap());
}
#[test]
fn teleport_cannot_choose_the_only_other_square_when_it_is_a_stair() {
    for revision in [50.,51.] {
        let mut r=room(revision);r.floor_mut().unwrap().tiles.fill("wall".into());
        set_tile(&mut r,10,10,"floor");set_tile(&mut r,11,10,"up");
        let rng=r.rng;crate::revision50_wave_teleport::teleport(&mut r,ENEMY).unwrap();
        assert_eq!(engine::actor(&r,ENEMY).unwrap().x,if revision<51. {11.} else {10.});
        if revision>=51. {assert_eq!(r.rng,rng);}
    }
}
#[test]
fn population_rejects_stairs_before_ids_or_rng_and_legacy_is_unchanged() {
    let mut r=room(51.);set_tile(&mut r,11,10,"up");let w=r.floor().unwrap().width();
    let mut rng=crate::rng::GenerationRng::new(1234);let original=rng.state;let mut id=100.;
    assert!(crate::population::create_population_actor_revision(&mut id,r.floor_mut().unwrap(),"rat",10*w+11,false,&mut rng,51.).is_err());
    assert_eq!(id,100.);assert_eq!(rng.state,original);
    assert!(crate::population::create_population_actor_revision(&mut id,r.floor_mut().unwrap(),"rat",10*w+11,false,&mut rng,50.).is_ok());
}
#[test]
fn witch_summon_with_only_stairs_free_defers_without_payment_or_randomness() {
    for revision in [50.,51.] {
        let mut r=room(revision);r.floor_mut().unwrap().tiles.fill("wall".into());
        set_tile(&mut r,10,10,"floor");set_tile(&mut r,11,10,"up");set_tile(&mut r,12,10,"floor");r.player.x=12.;
        engine::actor_mut_trusted(&mut r,ENEMY).unwrap().abilities=vec!["enemy:beast:witch".into()];
        let (rng,id,player)=(r.rng,r.next_id,r.player.id);
        assert_eq!(crate::witch::summon(&mut r,ENEMY,player).unwrap(),revision<51.);
        if revision>=51. {assert_eq!((r.rng,r.next_id),(rng,id));assert_eq!(engine::actor(&r,ENEMY).unwrap().energy,100.);}
    }
}
#[test]
fn birther_with_only_stairs_free_defers_without_payment_or_randomness() {
    for revision in [50.,51.] {
        let mut r=room(revision);r.floor_mut().unwrap().tiles.fill("wall".into());
        set_tile(&mut r,10,10,"floor");set_tile(&mut r,11,10,"down");set_tile(&mut r,12,10,"floor");r.player.x=12.;
        engine::actor_mut_trusted(&mut r,ENEMY).unwrap().abilities=vec!["enemy:pale:birth".into()];
        let (rng,id,player)=(r.rng,r.next_id,r.player.id);
        assert_eq!(crate::ai::enemy_action(&mut r,ENEMY,player).unwrap(),revision<51.);
        if revision>=51. {assert_eq!((r.rng,r.next_id),(rng,id));assert_eq!(engine::actor(&r,ENEMY).unwrap().energy,100.);}
    }
}
fn mercuria(revision:f64)->Run {
    let mut r=room(revision);r.actors_at_mut(1.).clear();r.floor_mut().unwrap().tiles.fill("wall".into());
    set_tile(&mut r,10,10,"floor");set_tile(&mut r,11,10,"up");set_tile(&mut r,12,10,"floor");
    set_tile(&mut r,10,11,"down");set_tile(&mut r,10,12,"floor");
    r.floor_mut().unwrap().godbeast_event=GodbeastEvent {kind:"Mercuria".into(),x:10.,y:10.,activated:true.into(),..Default::default()}.into();r
}
#[test]
fn ichor_stops_at_both_stairs_and_cannot_cross_them_legacy_spread_is_retained() {
    for revision in [50.,51.] {
        let mut r=mercuria(revision);let rng=r.rng;
        for _ in 0..4 {crate::ichor::tick_ichor_revision(r.floor_mut().unwrap(),revision);}
        let f=r.floor().unwrap();let w=f.width();let ichor=f.ichor.as_ref().unwrap();
        for at in [10*w+11,10*w+12,11*w+10,12*w+10] {assert_eq!(ichor[at]>0.,revision<51.);}
        assert_eq!(f.tiles[10*w+11],"up");assert_eq!(f.tiles[11*w+10],"down");assert_eq!(r.rng,rng);
    }
}
#[test]
fn opened_frontier_and_fish_spawn_do_not_reintroduce_ichor_or_fish_on_stairs() {
    let mut r=mercuria(51.);crate::ichor::tick_ichor_revision(r.floor_mut().unwrap(),51.);
    let w=r.floor().unwrap().width();crate::ichor::enqueue_opening_revision(r.floor_mut().unwrap(),10*w+11,51.);
    assert_eq!(r.floor().unwrap().ichor.as_ref().unwrap()[10*w+11],0.);
    // A malicious/stale ichor layer cannot admit a fish on a stair either.
    r.player.x=10.;r.player.y=10.;
    let f=r.floor_mut().unwrap();f.ichor.as_mut().unwrap()[10*w+11]=100.;
    f.godbeast_event.as_mut().unwrap().flood_age=8.0.into();let (rng,id)=(r.rng,r.next_id);
    crate::ichor::tick_fish(&mut r).unwrap();assert!(r.actors_at(1.).is_empty());assert_eq!((r.rng,r.next_id),(rng,id));
}
#[test]
fn flood_frontier_save_and_temporal_snapshots_restore_identical_spread() {
    let mut r=mercuria(51.);crate::ichor::tick_ichor_revision(r.floor_mut().unwrap(),51.);
    let mut saved:Run=serde_json::from_str(&crate::storage::serialize(&r).unwrap()).unwrap();
    let snapshot=crate::temporal::Snapshot::capture(&r);
    let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    let mut anchored=anchor.native_state().unwrap().clone();
    r.floor_mut().unwrap().ichor=Field::Missing;snapshot.restore(&mut r).unwrap();
    for state in [&mut r,&mut saved,&mut anchored] {crate::ichor::tick_ichor_revision(state.floor_mut().unwrap(),51.);}
    assert_eq!(r.floor().unwrap().ichor,saved.floor().unwrap().ichor);
    assert_eq!(r.floor().unwrap().godbeast_event,anchored.floor().unwrap().godbeast_event);
    assert_eq!(r.rng,saved.rng);
}

#[test]
fn temporary_command_cannot_move_onto_stairs_before_or_after_expiry() {
    let mut r=room(51.);set_tile(&mut r,11,10,"up");
    let a=engine::actor_mut_trusted(&mut r,ENEMY).unwrap();a.state="ally".into();a.conditions.insert("temporaryAlly".into(),1.);
    assert!(!engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap());
    let a=engine::actor_mut_trusted(&mut r,ENEMY).unwrap();crate::abilities::clear_status_conditions(a);
    assert_eq!(a.state,"wandering");
    assert!(!engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap());
    assert_eq!((engine::actor(&r,ENEMY).unwrap().x,engine::actor(&r,ENEMY).unwrap().y),(10.,10.));
}

#[test]
fn temporarily_covered_stairs_cannot_be_occupied_before_restoration() {
    use crate::revision50_terrain::{Cell,Kind,Modification};
    let mut r=room(51.);set_tile(&mut r,11,10,"wall");
    engine::actor_mut_trusted(&mut r,ENEMY).unwrap().active.push("etill-phase-through".into());
    let f=r.floor_mut().unwrap();f.revision50_terrain=vec![Modification{kind:Kind::Wall,
        cells:vec![Cell{x:11,y:10}],previous_tiles:vec!["up".into()],hp:Some(100.)}].into();
    assert!(!engine::actor_passable(&r,engine::actor(&r,ENEMY).unwrap(),11.,10.));
    assert!(!engine::move_actor(&mut r,ENEMY,1.,0.,true).unwrap());
    r.simulation_revision=50.0.into();
    assert!(engine::actor_passable(&r,engine::actor(&r,ENEMY).unwrap(),11.,10.));
}
#[test]
fn forced_fall_chooses_ground_instead_of_destination_stairs() {
    for revision in [50.,51.] {
        let mut r=room(revision);set_tile(&mut r,10,10,"chasm");
        let index=r.floors.iter().position(|f|f.depth==2.).unwrap();
        let f=crate::mutation_journal::make_mut(&mut r.floors[index]);let w=f.width();
        f.tiles.fill("wall".into());f.tiles[10*w+10]="up".into();f.tiles[10*w+11]="floor".into();
        crate::travel::fall_actors_one_level(&mut r,1.,&[ENEMY],None).unwrap();
        let a=r.actors_at(2.).iter().find(|a|a.id==ENEMY).unwrap();
        assert_eq!((a.x,a.y),(if revision>=51.{11.}else{10.},10.));
    }
}
