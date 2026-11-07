use super::*;
use crate::{Run,engine,temporal::Snapshot};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.x=12.;r.player.y=14.;
    r.player.abilities=vec![COGWORKS.into()];r.player.active.clear();r.player.conditions.clear();
    r.player.energy=0.;r.player.ability_reserves=indexmap::IndexMap::from([
        (COGWORKS.into(),crate::revision50_ability_energy::authored_reserve(20.,20.).unwrap())]).into();
    r.actors_at_mut(r.depth).clear();let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());
    f.doors=Default::default();f.environment=Default::default();f.razorleaves=Default::default();
    select_mode(&mut r,2.);r
}
fn place(r:&mut Run) {assert!(activate_cogworks(r,20.,14.).unwrap());}
fn enemy(r:&mut Run)->f64 {
    let id=100.;r.actors_at_mut(r.depth).push(Actor{id,x:19.,y:14.,hp:100.,max_hp:100.,state:"hunting".into(),
        stats:crate::Stats{offense:10.,..Default::default()},..Default::default()});id
}
fn hp(r:&Run)->f64 {r.floor().unwrap().revision50_terrain.as_ref().unwrap()[0].hp.unwrap()}
#[test] fn shared_wall_placement_payment_collision_damage_and_full_restore() {
    let mut r=room();let w=r.floor().unwrap().width();
    for (n,tile) in ["floor","grass","shallow-water","tree","wall"].into_iter().enumerate() {r.floor_mut().unwrap().tiles[(12+n)*w+20]=tile.into();}
    let prior=r.floor().unwrap().tiles.clone();place(&mut r);
    assert_eq!(r.player.energy,0.);assert_eq!(r.player.ability_reserves.as_ref().unwrap()[COGWORKS].energy,0.);
    assert_eq!(hp(&r),200.);assert!(validate_run(&r).is_ok());
    for y in 12..=16 {assert!(!engine::actor_passable(&r,&r.player,20.,y as f64));}
    assert!(damage_wall(r.floor_mut().unwrap(),Cell{x:20,y:12},70.).unwrap());assert_eq!(hp(&r),130.);
    assert!(damage_wall(r.floor_mut().unwrap(),Cell{x:20,y:16},60.).unwrap());assert_eq!(hp(&r),70.);
    assert!(damage_wall(r.floor_mut().unwrap(),Cell{x:20,y:14},70.).unwrap());
    assert_eq!(r.floor().unwrap().tiles,prior);assert!(r.floor().unwrap().revision50_terrain.as_ref().unwrap().is_empty());
    assert!(validate_run(&r).is_ok());
}
#[test] fn hostile_bump_strikes_with_one_damage_draw_and_no_movement() {
    let mut r=room();place(&mut r);let id=enemy(&mut r);
    let mut expected=r.clone();let variance=0.75+crate::rng::random(&mut expected)*0.5;
    let damage=crate::combat::ordinary_strike_damage(20.,variance,0.,false,false);
    assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());assert_eq!(hp(&r),200.-damage);assert_eq!(r.rng,expected.rng);
    let a=engine::actor(&r,id).unwrap();assert_eq!((a.x,a.y),(19.,14.));
    engine::actor_mut_trusted(&mut r,id).unwrap().conditions.insert("weaponDamage".into(),10000.);
    assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());
    assert!(r.floor().unwrap().revision50_terrain.as_ref().unwrap().is_empty());
    assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),(19.,14.));
    assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());assert_eq!(engine::actor(&r,id).unwrap().x,20.);
}
#[test] fn haste_forced_player_friendly_neutral_and_clay_cannot_break_wall() {
    let mut r=room();place(&mut r);let id=enemy(&mut r);
    for case in ["haste","forced","ally","neutral"] {
        let mut test=r.clone();let a=engine::actor_mut_trusted(&mut test,id).unwrap();
        if case=="forced" {a.conditions.insert("forcedMove".into(),1.);}
        if case=="ally"||case=="neutral" {a.state=case.into();}
        let before=test.rng;assert!(!engine::move_actor(&mut test,id,1.,0.,case=="haste").unwrap(),"{case}");
        assert_eq!(hp(&test),200.);assert_eq!(test.rng,before);
    }
    r.player.x=19.;let pid=r.player.id;assert!(!engine::move_actor(&mut r,pid,1.,0.,false).unwrap());assert_eq!(hp(&r),200.);
    let mut test=r.clone();let a=engine::actor_mut_trusted(&mut test,id).unwrap();a.state="ally".into();
    a.conditions.insert(crate::revision50_control_amulets::DISCORD.into(),6.);
    assert!(engine::move_actor(&mut test,id,1.,0.,false).unwrap());assert!(hp(&test)<200.);
    let mut mods=Vec::new();let mut cost=20.;let cell=Cell{x:18,y:14};
    commit(50.,r.floor_mut().unwrap(),&mut cost,&mut mods,&Placement{kind:Kind::ClaySoldier,cells:vec![cell]}).unwrap();
    r.floor_mut().unwrap().revision50_terrain.as_mut().unwrap().extend(mods);
    let before=r.rng;assert!(!engine::move_actor(&mut r,id,-1.,0.,false).unwrap());assert_eq!(r.rng,before);
}
#[test] fn breach_path_uses_only_breakable_walls_when_normal_path_absent() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles.fill("wall".into());
    for x in 10..=22 {r.floor_mut().unwrap().tiles[14*w+x]="floor".into();}
    place(&mut r);let id=enemy(&mut r);engine::actor_mut_trusted(&mut r,id).unwrap().x=21.;
    let step=engine::creature_path_step(&mut r,id,crate::movement::Point{x:12,y:14});
    assert_eq!(step,Some(crate::movement::Point{x:-1,y:0}));
    assert!(engine::move_actor(&mut r,id,-1.,0.,false).unwrap());assert!(hp(&r)<200.);
    r.floor_mut().unwrap().revision50_terrain=Default::default();
    assert_eq!(engine::creature_path_step(&mut r,id,crate::movement::Point{x:12,y:14}),None);
}
#[test] fn occupied_and_underfunded_walls_are_atomic() {
    let mut r=room();let id=enemy(&mut r);engine::actor_mut_trusted(&mut r,id).unwrap().x=20.;
    let before=r.clone();assert!(!activate_cogworks(&mut r,20.,14.).unwrap());assert_eq!(r,before);
    r.actors_at_mut(r.depth).clear();r.player.ability_reserves.as_mut().unwrap().get_mut(COGWORKS).unwrap().energy=19.;
    let before=r.clone();assert!(!activate_cogworks(&mut r,20.,14.).unwrap());assert_eq!(r,before);
}
#[test] fn wall_save_snapshot_and_free_remove_preserve_shared_health() {
    let mut r=room();place(&mut r);damage_wall(r.floor_mut().unwrap(),Cell{x:20,y:14},37.).unwrap();
    let mut loaded:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();assert_eq!(hp(&loaded),163.);
    let snapshot=Snapshot::from_value(Snapshot::capture(&loaded).value().unwrap()).unwrap();
    damage_wall(loaded.floor_mut().unwrap(),Cell{x:20,y:12},200.).unwrap();snapshot.restore(&mut loaded).unwrap();
    assert_eq!(hp(&loaded),163.);assert!(validate_run(&loaded).is_ok());select_mode(&mut loaded,3.);
    assert!(activate_cogworks(&mut loaded,20.,12.).unwrap());assert_eq!(loaded.player.energy,0.);
    assert!(loaded.floor().unwrap().revision50_terrain.as_ref().unwrap().is_empty());
}
#[test] fn malformed_wall_records_are_rejected_without_mutation() {
    let mut r=room();place(&mut r);
    for variant in 0..7 {
        let mut bad=r.clone();let f=bad.floor_mut().unwrap();let m=&mut f.revision50_terrain.as_mut().unwrap()[0];
        match variant {0=>m.hp=Some(0.),1=>m.hp=Some(201.),2=>m.hp=None,3=>{m.cells.pop();},4=>m.previous_tiles[0]="invented".into(),5=>m.cells[1]=m.cells[0],_=>m.cells[0]=Cell{x:99,y:99}}
        assert!(validate_run(&bad).is_err());let before=bad.clone();
        assert!(damage_wall(bad.floor_mut().unwrap(),Cell{x:20,y:14},1.).is_err());assert_eq!(bad,before);
    }
    r.simulation_revision=46.0.into();assert!(validate_run(&r).is_err());
}
#[test] fn physical_and_dream_walls_have_independent_health() {
    let mut r=room();crate::dreamwalking::ensure_layer(&mut r);place(&mut r);let id=enemy(&mut r);
    engine::actor_mut_trusted(&mut r,id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
    assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());assert_eq!(hp(&r),200.);
    engine::actor_mut_trusted(&mut r,id).unwrap().x=19.;
    crate::dreamwalking::with_plane(&mut r,true,|r| {
        let mut cost=20.;let mut mods=Vec::new();let plan=cogworks_preview(50.,2,Cell{x:12,y:14},Cell{x:20,y:14},r.floor().unwrap().width(),r.floor().unwrap().height(),|_|false).unwrap();
        commit(50.,r.floor_mut().unwrap(),&mut cost,&mut mods,&plan).unwrap();r.floor_mut().unwrap().revision50_terrain=mods.into();
    });
    assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());assert_eq!(hp(&r),200.);
    crate::dreamwalking::with_plane(&mut r,true,|r|assert!(hp(r)<200.));assert!(validate_run(&r).is_ok());
}
#[test] fn sight_cache_observes_wall_creation_and_removal_in_same_tick() {
    let mut r=room();let target=Actor{x:22.,y:14.,..Default::default()};
    assert!(crate::vision::line_of_sight(&r,&r.player,&target));let tick=r.tick;place(&mut r);
    assert!(!crate::vision::line_of_sight(&r,&r.player,&target));
    damage_wall(r.floor_mut().unwrap(),Cell{x:20,y:14},200.).unwrap();
    assert!(crate::vision::line_of_sight(&r,&r.player,&target));assert_eq!(r.tick,tick);
}
#[test] fn temporary_terrain_cannot_expire_under_persistent_wall() {
    let mut r=room();let w=r.floor().unwrap().width();let at=14*w+20;
    r.floor_mut().unwrap().revision50_temporary_terrain=vec![crate::revision50_control_amulets::TemporaryTerrain{
        cell:at,previous:"floor".into(),tile:"shallow-water".into(),expires_at:10.}].into();
    r.floor_mut().unwrap().tiles[at]="shallow-water".into();let before=r.clone();
    assert!(!activate_cogworks(&mut r,20.,14.).unwrap());assert_eq!(r,before);
    r.floor_mut().unwrap().revision50_temporary_terrain=Default::default();let id=enemy(&mut r);
    let a=engine::actor_mut_trusted(&mut r,id).unwrap();a.conditions.insert("raisedWallIndex".into(),at as f64);
    a.conditions.insert("raisedWallExpires".into(),10.);a.conditions.insert("raisedWallPlane".into(),0.);
    r.floor_mut().unwrap().tiles[at]="wall".into();let before=r.clone();
    assert!(!activate_cogworks(&mut r,20.,14.).unwrap());assert_eq!(r,before);
}
#[test] fn offscreen_normal_turn_can_breach_without_duplicate_catchup() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles.fill("wall".into());
    for x in 10..=22 {r.floor_mut().unwrap().tiles[14*w+x]="floor".into();}place(&mut r);
    let mut f=r.floor().unwrap().clone();f.depth=2.;f.last_tick=0.;f.dirty=true;
    r.floors[1]=std::sync::Arc::new(f);r.tick=1.;r.next_id=1000.;
    r.actors_at_mut(2.).clear();r.actors_at_mut(2.).push(Actor{id:100.,x:21.,y:14.,hp:100.,max_hp:100.,energy:100.,state:"hunting".into(),
        last_seen:crate::ActorLastSeen{x:12.,y:14.}.into(),stats:crate::Stats{offense:10.,max_energy:100.,..Default::default()},..Default::default()});
    crate::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let hp=r.floors[1].revision50_terrain.as_ref().unwrap()[0].hp.unwrap();assert!(hp<200.);assert_eq!(self::hp(&r),200.);
    crate::offscreen::catch_up_floor(&mut r,2.).unwrap();assert_eq!(r.floors[1].revision50_terrain.as_ref().unwrap()[0].hp,Some(hp));
}

#[test] fn null_terrain_metadata_is_rejected_on_each_plane() {
    let mut r=room();r.floor_mut().unwrap().revision50_terrain=crate::Field::Null;
    assert!(validate_run(&r).is_err());r.floor_mut().unwrap().revision50_terrain=Default::default();
    crate::dreamwalking::ensure_layer(&mut r);
    crate::dreamwalking::with_plane(&mut r,true,|r|r.floor_mut().unwrap().revision50_terrain=crate::Field::Null);
    assert!(validate_run(&r).is_err());
}
