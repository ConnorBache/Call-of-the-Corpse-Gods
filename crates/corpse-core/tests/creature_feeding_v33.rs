//! Creature-only normal-turn hunger; safe, non-depleting wall/water feeding.
use corpse_core::{Actor, Command, Field, Run, creature_feeding as food, engine, storage};
use std::sync::Arc;
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.tick=0.;r.phase=Field::Missing;r.floors.retain(|f|f.depth==1.);r.actors.retain(|k,_|k=="1");r.history.clear();r.future=None;
    r.player.x=65.;r.player.y=20.;r.player.energy=100.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.conditions.insert("invisible".into(),10000.);r.inventory.clear();r.actors_at_mut(1.).clear();
    let f=r.floor_mut().unwrap();let n=f.tiles.len();let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.environment=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;
    f.ichor=Field::Missing;f.slime=Field::Missing;f.mud=Field::Missing;f.razorleaves=Field::Missing;
    f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.currents=Field::Missing;
    f.water_blood=Field::Missing;f.kelpie_blood_sources=Field::Missing;f.bloody_walls=Field::Missing;
    f.seen=vec![true;n];f.last_tick=0.;r
}
fn npc(r:&mut Run,x:f64,y:f64)->f64 {
    let mut a:Actor=serde_json::from_str::<Run>(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().actors_at(1.)[0].clone();
    a.id=r.next_id;r.next_id+=1.;a.name="Feeding test creature".into();a.creature_kind=Field::Missing;
    a.x=x;a.y=y;a.hp=100.;a.max_hp=100.;a.energy=100.;a.stats.max_hp=100.0.into();a.stats.max_energy=100.;
    a.abilities.clear();a.active.clear();a.conditions.clear();a.state="wandering".into();a.dead=false.into();a.last_seen=Field::Missing;a.monster_state=Field::Missing;
    a.conditions.insert("bestiaryTraitsInitialized".into(),1.);a.conditions.insert("paleTraitsInitialized".into(),1.);
    let id=a.id;r.actors_at_mut(1.).push(a);id
}
fn cell(r:&Run,x:usize,y:usize)->usize {y*r.floor().unwrap().width()+x}
fn wall(r:&mut Run,x:usize,y:usize)->usize {
    let i=cell(r,x,y);let f=r.floor_mut().unwrap();let n=f.tiles.len();f.tiles[i]="wall".into();
    f.bloody_walls.get_or_insert_with(||vec![false;n])[i]=true;i
}
fn hungry(r:&mut Run,id:f64) {engine::actor_mut(r,id).unwrap().conditions.insert(food::AGE.into(),99.);}
fn step(r:&mut Run,id:f64,may_move:bool)->bool {
    r.tick+=1.;r.resolving_tick=true.into();food::begin_turn(r);
    let result=food::take_turn(r,id,may_move,false).unwrap();food::end_turn(r);r.resolving_tick=Field::Missing;result
}
fn value(r:&Run,id:f64,key:&str)->f64 {engine::condition(engine::actor(r,id).unwrap(),key)}
#[test]
fn exactly_one_hundred_normal_turns_then_ten_stationary_turns_reset_without_buffs() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);
    let a=engine::actor(&r,id).unwrap().clone();let rng=r.rng;
    for turn in 1..100 {assert!(!step(&mut r,id,false));assert_eq!(value(&r,id,food::AGE),turn as f64);}
    assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),None);
    for count in 1..=9 {assert!(step(&mut r,id,false));assert_eq!(value(&r,id,"creatureFeedingTurns"),count as f64);assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Eating"));}
    assert!(step(&mut r,id,false));assert_eq!(value(&r,id,food::AGE),0.);assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),None);
    let b=engine::actor(&r,id).unwrap();assert_eq!((b.x,b.y,b.hp,b.energy,b.stats.clone()),(a.x,a.y,a.hp,a.energy,a.stats));assert_eq!(r.rng,rng);
    for _ in 0..99 {assert!(!step(&mut r,id,false));}assert!(step(&mut r,id,false));
    assert!(!r.player.conditions.contains_key(food::AGE));
}
#[test]
fn reachable_wall_uses_adjacent_safe_cell_and_never_walks_through_wall() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);let i=wall(&mut r,14,10);hungry(&mut r,id);
    for _ in 0..3 {assert!(step(&mut r,id,true));}
    assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),(13.,10.));
    assert!(step(&mut r,id,true));assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Eating"));
    assert_eq!(r.floor().unwrap().tiles[i],"wall");assert!(!engine::passable(&r,14.,10.));
    assert!(!engine::move_actor(&mut r,id,1.,0.,true).unwrap());
}
#[test]
fn thirst_uses_real_blood_only_and_drinks_from_land_without_drowning_or_depleting() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);hungry(&mut r,id);
    let i=cell(&r,11,10);let n=r.floor().unwrap().tiles.len();r.floor_mut().unwrap().tiles[i]="water".into();
    r.floor_mut().unwrap().blood=vec![true;n].into();assert!(!food::drinkable(r.floor().unwrap(),i));
    assert!(!step(&mut r,id,false));assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Hungering"));
    r.floor_mut().unwrap().water_blood=vec![0.;n].into();r.floor_mut().unwrap().water_blood.as_mut().unwrap()[i]=3.;
    for _ in 0..9 {assert!(step(&mut r,id,true));assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Drinking"));}
    assert!(step(&mut r,id,true));assert_eq!(value(&r,id,food::AGE),0.);assert_eq!(r.floor().unwrap().water_blood.as_ref().unwrap()[i],3.);
    assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),(10.,10.));
    r.floor_mut().unwrap().ichor=vec![1.;n].into();assert!(!food::drinkable(r.floor().unwrap(),i));
}
#[test]
fn damage_threat_movement_source_loss_and_paralysis_restart_full_meal() {
    for interruption in ["damage","hunting","movement","source","paralyzed"] {
        let mut r=room(33.);let id=npc(&mut r,10.,10.);let i=wall(&mut r,11,10);hungry(&mut r,id);
        for _ in 0..5 {assert!(step(&mut r,id,false));}
        match interruption {
            "damage"=>{engine::damage_environment(&mut r,id,7.,None,"attack",true).unwrap();assert_eq!(engine::actor(&r,id).unwrap().hp,93.);},
            "hunting"=>engine::actor_mut(&mut r,id).unwrap().state="hunting".into(),
            "movement"=>engine::actor_mut(&mut r,id).unwrap().y=11.,
            "source"=>r.floor_mut().unwrap().bloody_walls.as_mut().unwrap()[i]=false,
            _=>{engine::actor_mut(&mut r,id).unwrap().conditions.insert("paralyzed".into(),1.);}
        }
        step(&mut r,id,false);
        let a=engine::actor_mut(&mut r,id).unwrap();a.state="wandering".into();a.y=10.;a.conditions.shift_remove("paralyzed");
        r.floor_mut().unwrap().bloody_walls.as_mut().unwrap()[i]=true;
        for _ in 0..8 {step(&mut r,id,false);assert_eq!(value(&r,id,food::AGE),100.,"{interruption}");}
        // Position change could start a new meal on that interruption turn, never retain old progress.
        assert!(value(&r,id,"creatureFeedingTurns")<=9.,"{interruption}");
    }
}
#[test]
fn slow_feeds_every_normal_turn_and_haste_never_advances_the_meal_clock() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);
    engine::actor_mut(&mut r,id).unwrap().abilities=vec!["enemy:beast:slow".into(),"enemy:beast:rooted".into()];
    for n in 1..=9 {
        engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(value(&r,id,"creatureFeedingTurns"),n as f64);
        let before=r.clone();assert!(food::take_turn(&mut r,id,false,true).unwrap());assert_eq!(r,before);
    }
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(value(&r,id,food::AGE),0.);
}
#[test]
fn rooted_aquatic_unreachable_and_rally_cases_cannot_teleport_or_override_orders() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,14,10);hungry(&mut r,id);
    engine::actor_mut(&mut r,id).unwrap().abilities.push("enemy:beast:rooted".into());
    assert!(!step(&mut r,id,true));assert_eq!(engine::actor(&r,id).unwrap().x,10.);
    engine::actor_mut(&mut r,id).unwrap().abilities=vec!["enemy:beast:aquatic".into()];
    let at=cell(&r,10,10);r.floor_mut().unwrap().tiles[at]="water".into();assert!(!step(&mut r,id,true));
    assert_eq!(engine::actor(&r,id).unwrap().x,10.);
    engine::actor_mut(&mut r,id).unwrap().abilities.clear();r.floor_mut().unwrap().tiles[at]="floor".into();
    for y in 1..28 {let at=cell(&r,12,y);r.floor_mut().unwrap().tiles[at]="wall".into();}
    assert!(!step(&mut r,id,true));
    wall(&mut r,11,10);let a=engine::actor_mut(&mut r,id).unwrap();a.state="ally".into();a.perfect=true.into();a.order="p".to_string().into();
    assert!(!step(&mut r,id,true));assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Hungering"));
}
#[test]
fn serialization_snapshot_both_temporal_routes_and_legacy_gate_keep_ecology_consistent() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);step(&mut r,id,false);
    let raw=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&raw).unwrap();assert_eq!(r,loaded);
    let snapshot=corpse_core::temporal::Snapshot::capture(&r);let mut advanced=r.clone();step(&mut advanced,id,false);snapshot.restore(&mut advanced).unwrap();assert_eq!(advanced,r);
    let mut e=corpse_core::temporal::Engine::new(r.clone()).unwrap();
    e.dispatch(&Command::Wait).unwrap();let fed=value(&e.run,id,"creatureFeedingTurns");
    corpse_core::temporal::remember_future(&mut e.run,&e.history).unwrap();e.dispatch(&Command::Wait).unwrap();
    assert!(corpse_core::temporal::restore_future(&mut e.run,&mut e.history).unwrap());assert_eq!(value(&e.run,id,"creatureFeedingTurns"),fed);
    let mut e=corpse_core::temporal::Engine::new(r.clone()).unwrap();e.run.player.conditions.insert("revertCharges".into(),2.);e.dispatch(&Command::Wait).unwrap();assert!(corpse_core::temporal::revert(&mut e.run,&mut e.history).unwrap());assert_eq!(value(&e.run,id,"creatureFeedingTurns"),1.);
    let mut old=room(32.);let id=npc(&mut old,10.,10.);let before=old.clone();food::begin_turn(&mut old);assert!(!food::take_turn(&mut old,id,true,false).unwrap());food::end_turn(&mut old);assert_eq!(old,before);
    assert!(!storage::serialize(&old).unwrap().contains("bloodyWalls"));
}
#[test]
fn offscreen_exact_clock_completes_and_restarts_meals_without_player_hunger() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);
    let mut f=r.floor().unwrap().clone();f.depth=2.;f.last_tick=0.;f.dirty=true;
    let actors=r.actors_at_mut(1.).drain(..).collect();r.actors.insert("2".into(),Arc::new(actors));r.floors.push(Arc::new(f));
    for tick in 1..=10 {r.tick=tick as f64;corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();}
    assert_eq!(engine::condition(&r.actors_at(2.)[0],food::AGE),0.);assert_eq!((r.actors_at(2.)[0].x,r.actors_at(2.)[0].y),(10.,10.));
    assert_eq!(r.actors_at(2.)[0].id,id);r.tick=109.;corpse_core::offscreen::tick_inactive_floors(&mut r).unwrap();assert_eq!(engine::condition(&r.actors_at(2.)[0],food::AGE),99.);
    assert!(!r.player.conditions.contains_key(food::AGE));
}
#[test]
fn decoration_is_deterministic_and_observed_wall_memory_does_not_leak_hidden_changes() {
    let mut r=room(33.);for x in 5..30 {let i=cell(&r,x,5);r.floor_mut().unwrap().tiles[i]="wall".into();}
    let mut same=r.clone();food::decorate_floor(r.floor_mut().unwrap(),17);food::decorate_floor(same.floor_mut().unwrap(),17);assert_eq!(r,same);
    let f=r.floor().unwrap();let i=f.bloody_walls.as_ref().unwrap().iter().position(|v|*v).unwrap();
    assert_eq!(f.tiles[i],"wall");let hazard=corpse_core::hazard_memory::observed_hazards_at(f,i,0.);assert!(hazard.bloody_wall.value_or(false));
    let mut mask=vec![false;f.tiles.len()];mask[i]=true;corpse_core::hazard_memory::remember_observed_hazards(r.floor_mut().unwrap(),&mask,0.);
    r.floor_mut().unwrap().bloody_walls.as_mut().unwrap()[i]=false;mask[i]=false;
    corpse_core::hazard_memory::remember_observed_hazards(r.floor_mut().unwrap(),&mask,1.);
    assert!(r.floor().unwrap().remembered_hazards.as_ref().unwrap()[&i.to_string()].bloody_wall.value_or(false));
}
#[test]
fn actual_haste_dispatch_and_spawn_sleep_use_world_turns_not_commands() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);
    engine::actor_mut(&mut r,id).unwrap().state="sleeping".into();
    engine::actor_mut(&mut r,id).unwrap().abilities.push("enemy:beast:fast".into());
    r.player.conditions.insert("hasted".into(),100.);
    let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert_eq!(r.tick,0.);assert_eq!(value(&r,id,food::AGE),99.);
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(r.tick,1.);assert_eq!(value(&r,id,"creatureFeedingTurns"),1.);
    let before=(engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y);
    let out=engine::dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!out.took_turn);
    assert_eq!(r.tick,1.);assert_eq!(value(&r,id,"creatureFeedingTurns"),1.);
    assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),before);
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(value(&r,id,"creatureFeedingTurns"),2.);
}
#[test]
fn autonomous_ally_can_feed_but_following_and_combat_take_priority() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);
    r.player.x=9.;r.player.y=10.;engine::actor_mut(&mut r,id).unwrap().state="ally".into();
    assert!(step(&mut r,id,true));assert_eq!(food::description(&r,engine::actor(&r,id).unwrap()),Some("Eating"));
    r.player.x=15.;assert!(!step(&mut r,id,true));assert_eq!(value(&r,id,"creatureFeedingTurns"),0.);
    r.player.x=9.;let enemy=npc(&mut r,10.,11.);engine::actor_mut(&mut r,enemy).unwrap().state="hunting".into();
    assert!(!step(&mut r,id,true));assert_eq!(value(&r,id,"creatureFeedingTurns"),0.);
}
#[test]
fn hunting_detection_interrupts_before_idle_food_action() {
    let mut r=room(33.);let id=npc(&mut r,10.,10.);wall(&mut r,11,10);hungry(&mut r,id);step(&mut r,id,false);
    r.player.conditions.clear();r.player.x=9.;r.player.y=10.;
    let a=engine::actor_mut(&mut r,id).unwrap();a.state="hunting".into();a.last_seen=corpse_core::ActorLastSeen{x:9.,y:10.}.into();a.conditions.insert("targetId".into(),0.);
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert_eq!(value(&r,id,"creatureFeedingTurns"),0.);assert_eq!(engine::actor(&r,id).unwrap().state,"hunting");
    assert_eq!(value(&r,id,food::AGE),100.);
}
#[test]
fn actual_generation_keeps_wall_geometry_and_seeds_only_real_river_food_in_r33() {
    use corpse_core::generation::world::generate_floor_full_revision;
    let f=generate_floor_full_revision(3,123456,None,33.).unwrap();
    let marked=f.bloody_walls.as_ref().expect("generated bloody wall overlay");
    assert!(marked.iter().any(|v|*v));
    for (i,yes) in marked.iter().enumerate() {if *yes {assert_eq!(f.tiles[i],"wall");}}
    for &source in f.blood_river_sources.as_ref().into_iter().flatten() {
        let at=source as usize;
        assert!(food::drinkable(&f,at));
        assert!(f.generation.as_ref().unwrap().landmarks.as_ref().unwrap().iter().any(|l|l.kind=="river"&&l.cells.contains(&source)));
    }
    let legacy=generate_floor_full_revision(3,123456,None,32.).unwrap();
    assert!(legacy.bloody_walls.is_missing());assert!(legacy.blood_river_sources.is_missing());
}

#[test]
fn revision35_hungry_route_crosses_portal_once_and_respects_remote_occupant() {
    use corpse_core::portals_generation::PortalPair;
    let mut r=room(35.);let id=npc(&mut r,10.,10.);
    let (a,b)=(cell(&r,11,10),cell(&r,40,10));
    r.floor_mut().unwrap().portals=vec![PortalPair{a,b,color_index:0}].into();
    wall(&mut r,42,10);hungry(&mut r,id);
    assert!(step(&mut r,id,true));
    assert_eq!((engine::actor(&r,id).unwrap().x,engine::actor(&r,id).unwrap().y),(40.,10.));
    let mut blocked=room(35.);let id=npc(&mut blocked,10.,10.);npc(&mut blocked,40.,10.);
    blocked.floor_mut().unwrap().portals=vec![PortalPair{a,b,color_index:0}].into();
    wall(&mut blocked,42,10);hungry(&mut blocked,id);
    step(&mut blocked,id,true);
    let actor=engine::actor(&blocked,id).unwrap();
    assert_ne!((actor.x,actor.y),(40.,10.));
    assert_ne!((actor.x,actor.y),(11.,10.));
}
