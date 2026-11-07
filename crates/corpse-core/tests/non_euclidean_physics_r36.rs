//! Controlled r36 physical topology regression fixtures, not natural generation samples.
use corpse_core::{Command,Field,Run,engine,population,portals,projectiles,movement::{self,Point},half_here,dreamwalking as dream,
non_euclidean_generation::{NonEuclideanGroup as Group,NonEuclideanPocket as Pocket}};
use std::sync::Arc;
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 36.0.into();
    r.inventory.clear(); r.offers.clear(); r.history.clear(); r.replay.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing; r.log.clear();
    r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.x = 8.; r.player.y = 8.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.;
    r.player.stats.accuracy = 10000.; r.player.stats.offense = 20.; r.next_id = 1000.; r.rng = 1.;
    let mut f = r.floors[0].as_ref().clone();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.godbeast = false; f.doors = Field::Missing;
    f.light_sources = Field::Missing; f.blood = Field::Missing; f.roots = Field::Missing;
    f.fungal_roots = Field::Missing; f.razorleaves = Field::Missing; f.grates = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    let (w, h) = (f.width(), f.height());
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into(); f.lights.fill(1.);
    r.floors.clear(); r.actors.clear(); r.ground.clear();
    for depth in 1..=20 {
        let mut floor = f.clone(); floor.depth = depth as f64; r.floors.push(Arc::new(floor));
        r.actors.insert(depth.to_string(), Arc::new(vec![])); r.ground.insert(depth.to_string(), Arc::new(vec![]));
    }
    r.depth = 20.;
    let at=|x|10*w+x;
    r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![at(10),at(11),at(12)],groups:vec![Group{sources:vec![at(10),at(11)],destination:at(12)}]}].into();
    r.player.x=13.;r.player.y=10.;

    r
}
fn rat(r: &mut Run, x: f64, y: f64, ally: bool) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = population::create_actor_revision("rat", id, x, y, false, 36.).unwrap();
    a.state = if ally { "ally" } else { "sleeping" }.into();
    a.hp = 100.; a.max_hp = 100.; a.stats.dodge = -1000.; a.stats.defense = 0.; a.stats.dr = 0.;
    r.actors_at_mut(20.).push(a); id
}

fn arrow()->corpse_core::Item{corpse_core::Item{id:900.,kind:"ammo".into(),name:"Arrow".into(),glyph:"→".into(),effect:"arrows".to_string().into(),quantity:1.0.into(),..Default::default()}}
#[test]fn preview_stops_at_random_entry_and_committed_trace_draws_once(){
 for seed in [1u32,8192] {let mut r=room();r.rng=seed as f64;let before=r.clone();let p=projectiles::trace(&r,12.,10.);assert_eq!((p.x,p.y),(12.,10.));assert_eq!(r,before);
 let mut expected=seed;let draw=corpse_core::rng::next_u32(&mut expected)as f64/4294967296.;let p=projectiles::trace_committed(&mut r,12.,10.);assert_eq!((p.x,p.y),(if draw<0.5{10.}else{11.},10.));assert_eq!(r.rng,expected as f64);assert_eq!(p.distance,Some(1.));}
}
#[test]fn collision_or_blocked_exit_stops_before_fork_without_rng(){
 let mut r=room();let id=rat(&mut r,10.,10.,false);let before=r.rng;let p=projectiles::trace_committed(&mut r,12.,10.);assert_eq!(p.actor_id,Some(id));assert_eq!(r.rng,before);
 r.actors_at_mut(20.).clear();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]="wall".into();let p=projectiles::trace_committed(&mut r,12.,10.);assert_eq!((p.x,p.y),(12.,10.));assert_eq!(r.rng,before);
}
#[test]fn actual_throw_transfers_one_item_and_invalid_potion_does_not_draw(){
 let mut r=room();r.inventory.push(arrow());let mut expected=corpse_core::rng::to_u32(r.rng);corpse_core::rng::next_u32(&mut expected);
 assert_eq!(projectiles::command(&mut r,&Command::Throw{id:900.,x:12.,y:10.}).unwrap(),Some(true));assert!(r.inventory.is_empty());let items=&r.ground["20"];assert_eq!(items.len(),1);assert_eq!((items[0].x,items[0].y),(10.,10.));assert_eq!(r.rng,expected as f64);
 let mut bad=room();let mut i=arrow();i.kind="potion".into();i.effect="not-supported".to_string().into();bad.inventory.push(i);let before=bad.clone();assert!(projectiles::command(&mut bad,&Command::Throw{id:900.,x:12.,y:10.}).is_err());assert_eq!(bad,before);
}
#[test]fn thrown_potion_hits_shared_occupant_once_without_sampling_return(){
 let mut r=room();let id=rat(&mut r,10.,10.,false);engine::actor_mut(&mut r,id).unwrap().hp=20.;let mut i=arrow();i.kind="potion".into();i.effect="healing".to_string().into();r.inventory.push(i);let before=r.rng;
 assert_eq!(projectiles::command(&mut r,&Command::Throw{id:900.,x:12.,y:10.}).unwrap(),Some(true));assert_eq!(engine::actor(&r,id).unwrap().hp,70.);assert!(r.inventory.is_empty());assert_eq!(r.rng,before);
}
#[test]fn adjacent_sibling_step_is_not_a_zero_cost_transfer(){
 let mut r=room();r.player.x=11.;let path=portals::directed_ray(r.floor().unwrap(),Point{x:11,y:10},Point{x:10,y:10},|_|true);
 assert_eq!(path,vec![Point{x:11,y:10},Point{x:10,y:10},Point{x:12,y:10}]);assert_eq!(portals::ray_distance(r.floor().unwrap(),&path),1.);
 let ray=portals::directed_ray_trace(r.floor().unwrap(),Point{x:11,y:10},Point{x:10,y:10},|_|true,|options|Some(options[0]),|_|false);assert_eq!(ray.distance,1.);
}
#[test]fn npc_path_admits_both_outcomes_and_commits_only_actual_movement(){
 for seed in [1u32,8192]{let mut r=room();r.player.x=6.;let id=rat(&mut r,13.,10.,false);r.rng=seed as f64;
 let w=r.floor().unwrap().width();let f=r.floor_mut().unwrap();for tile in &mut f.tiles{*tile="wall".into()}for x in 6..=13{f.tiles[10*w+x]="floor".into()}
 let before=r.rng;let step=engine::creature_path_step(&mut r,id,Point{x:8,y:10}).unwrap();assert_eq!(step,Point{x:-1,y:0});assert_eq!(r.rng,before);
 assert!(engine::move_actor(&mut r,id,-1.,0.,true).unwrap());let a=engine::actor(&r,id).unwrap();assert!([10.,11.].contains(&a.x));assert_eq!(a.y,10.);
 for _ in 0..64 {if engine::actor(&r,id).unwrap().x==8.{break}let next=engine::creature_path_step(&mut r,id,Point{x:8,y:10}).expect("all outcomes retain a route");assert!(engine::move_actor(&mut r,id,next.x as f64,next.y as f64,true).unwrap());}
 assert_eq!(engine::actor(&r,id).unwrap().x,8.,"stochastic pursuit eventually reaches its target after replanning");}
}
#[test]fn stochastic_path_rejects_an_unwalkable_sibling(){
 let r=room();let f=r.floor().unwrap();let w=f.width();let mut allowed=vec![true;f.tiles.len()];allowed[10*w+11]=false;
 let field=movement::distance_field_portals(&f.tiles,w,Point{x:8,y:10},false,false,Some(&allowed),None,&portals::links(f));
 assert!(field.spatial_exits.as_ref().unwrap()[10*w+12].is_empty());
}
#[test]fn half_here_keeps_native_pocket_plane_after_dreamwalk(){
 let mut r=room();r.player.abilities.push("etill-half-here".into());r.player.active.push("etill-half-here".into());let id=half_here::create(&mut r,10.,10.).unwrap();
 r.dreamwalking=dream::State{unlocked:true,..Default::default()}.into();dream::ability(&mut r,dream::DREAMWALK_ID).unwrap();let a=engine::actor(&r,id).unwrap();assert!(!dream::dream_actor(&r,a));assert!(portals::same_position(&r,a,12.,10.));engine::actor_mut(&mut r,id).unwrap().conditions.insert("forcedMove".into(),1.);
 assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());let a=engine::actor(&r,id).unwrap();assert_eq!((a.x,a.y),(12.,10.));assert!(!dream::dream_actor(&r,a));assert_eq!((r.player.x,r.player.y),(13.,10.));
}
#[test]fn actual_npc_ranged_launch_resolves_fork_not_preview_stop(){
 let mut r=room();r.player.x=7.;r.player.stats.dodge=-1000.;let id=rat(&mut r,13.,10.,false);let a=engine::actor_mut(&mut r,id).unwrap();a.state="hunting".into();a.stats.accuracy=10000.;a.conditions.insert("lucky".into(),1.);
 let preview=projectiles::trace_from_actor(&r,engine::actor(&r,id).unwrap(),7.,10.);assert_eq!((preview.x,preview.y),(12.,10.));let before=r.rng;
 assert!(engine::attack_ranged(&mut r,id,0.,20.,6.,1.,false).unwrap());assert!(r.player.hp<100.);assert_ne!(r.rng,before);
}
#[test]fn linked_source_origins_are_deterministic_but_reverse_origin_is_not_previewed(){
 let mut r=room();r.player.x=10.;let before=r.rng;let p=projectiles::trace_committed(&mut r,14.,10.);assert_eq!((p.x,p.y),(14.,10.));assert_eq!(p.distance,Some(2.));assert_eq!(r.rng,before);
 // One-to-one counterpart launch uses the same non-projecting physical rule.
 let w=r.floor().unwrap().width();r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![10*w+10,10*w+11],groups:vec![Group{sources:vec![10*w+10],destination:10*w+11}]}].into();
 let p=projectiles::trace_committed(&mut r,14.,10.);assert_eq!((p.x,p.y),(14.,10.));assert_eq!(p.distance,Some(3.));assert_eq!(r.rng,before);
 for seed in [1u32,8192] {let mut r=room();r.player.x=11.;r.rng=seed as f64;let w=r.floor().unwrap().width();r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![10*w+10,10*w+11,11*w+10],groups:vec![Group{sources:vec![10*w+10,11*w+10],destination:10*w+11}]}].into();let f=r.floor().unwrap();let plan=portals::ray_plan(f,Point{x:11,y:10},Point{x:8,y:10},|_|true);assert_eq!(plan.origin_fork.len(),2);
 let preview=projectiles::trace(&r,8.,10.);assert_eq!((preview.x,preview.y),(11.,10.));assert_eq!(r.rng,seed as f64);
 let mut expected=seed;let draw=corpse_core::rng::next_u32(&mut expected)as f64/4294967296.;let p=projectiles::trace_committed(&mut r,8.,10.);assert_eq!(p.distance,Some(2.));assert_eq!(r.rng,expected as f64);
 // The return draw translates the selected direction; it does not force the intended target.
 assert_eq!(p.x,8.);assert_eq!(p.y,if draw<0.5{10.}else{11.});
 }
}
#[test]fn local_origin_shot_does_not_draw_and_blocked_return_origin_cannot_reroll(){
 let mut r=room();r.player.x=12.;let before=r.rng;let p=projectiles::trace_committed(&mut r,14.,10.);assert_eq!((p.x,p.y),(14.,10.));assert_eq!(r.rng,before);
 let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]="wall".into();let plan=portals::ray_plan(r.floor().unwrap(),Point{x:12,y:10},Point{x:8,y:10},|p|r.floor().unwrap().tiles[p.y as usize*w+p.x as usize]!="wall");assert!(plan.origin_fork.is_empty());let _=projectiles::trace_committed(&mut r,8.,10.);assert_eq!(r.rng,before);
}
#[test]fn legacy_throw_keeps_second_raster_trajectory_check(){
 let mut r=room();r.simulation_revision=35.0.into();r.floor_mut().unwrap().non_euclidean=Field::Missing;r.player.x=10.;r.player.y=10.;
 let intended=rat(&mut r,11.,13.,false);let blocker=rat(&mut r,11.,12.,false);r.inventory.push(arrow());r.player.conditions.insert("lucky".into(),1.);
 let initial=projectiles::trace(&r,11.,14.);assert_eq!(initial.actor_id,Some(intended));let retrace=projectiles::trace(&r,11.,13.);assert_eq!(retrace.actor_id,Some(blocker));
 assert_eq!(projectiles::command(&mut r,&Command::Throw{id:900.,x:11.,y:14.}).unwrap(),Some(true));assert_eq!(engine::actor(&r,intended).unwrap().hp,100.);assert_eq!(engine::actor(&r,blocker).unwrap().hp,100.);
}
fn origin_fork_caster(kind:&str,seed:u32)->(Run,f64){
 let mut r=room();r.player.x=8.;r.player.stats.dodge=-1000.;let w=r.floor().unwrap().width();
 r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![10*w+10,10*w+11,11*w+10],groups:vec![Group{sources:vec![10*w+10,11*w+10],destination:10*w+11}]}].into();
 let id=rat(&mut r,11.,10.,false);let a=engine::actor_mut(&mut r,id).unwrap();a.state="hunting".into();a.abilities=vec![format!("enemy:beast:{kind}")];a.energy=100.;a.stats.accuracy=10000.;a.conditions.insert("lucky".into(),1.);r.rng=seed as f64;(r,id)
}
#[test]fn real_horn_ai_admits_possible_shot_but_commits_fair_miss_and_recovery(){
 for seed in [1u32,8192]{let(mut r,id)=origin_fork_caster("horn-bolt",seed);let before=r.clone();let range=projectiles::possible_shot_range(&r,engine::actor(&r,id).unwrap(),&r.player,10.);assert_eq!(range,Some(2.));assert_eq!(r,before);
 assert!(corpse_core::monster_horn::special_action(&mut r,id,0.).unwrap());let a=engine::actor(&r,id).unwrap();assert_eq!(a.energy,92.);assert_eq!(a.conditions.get("rangedRecovery"),Some(&1.));
 if seed==1{assert!(r.player.hp<100.)}else{assert_eq!(r.player.hp,100.);let mut expected=seed;corpse_core::rng::next_u32(&mut expected);assert_eq!(r.rng,expected as f64);}
 }
}
#[test]fn real_witch_fork_miss_advances_phase_spends_and_sets_same_cooldown(){
 for seed in [1u32,8192]{let(mut r,id)=origin_fork_caster("witch",seed);let tick=r.tick;assert!(corpse_core::monster_spells::special_action(&mut r,id,0.).unwrap());let a=engine::actor(&r,id).unwrap();assert_eq!(a.energy,92.);assert_eq!(a.conditions.get("rangedRecovery"),Some(&1.));assert_eq!(a.conditions.get("monsterSpellReadyAt"),Some(&(tick+3.)));assert_eq!(a.conditions.get("monsterSpellPhase"),Some(&1.));
 assert_eq!(r.player.conditions.contains_key("slowed"),seed==1);let mut expected=seed;corpse_core::rng::next_u32(&mut expected);assert_eq!(r.rng,expected as f64);
 }
}
#[test]fn possible_shot_search_never_supplies_remote_optical_knowledge(){
 let(mut r,id)=origin_fork_caster("horn-bolt",1);let w=r.floor().unwrap().width();for y in 1..r.floor().unwrap().height()-1{r.floor_mut().unwrap().tiles[y*w+9]="wall".into();}
 assert!(projectiles::possible_shot_range(&r,engine::actor(&r,id).unwrap(),&r.player,10.).is_none());let before=r.clone();assert!(!corpse_core::monster_horn::special_action(&mut r,id,0.).unwrap());assert_eq!(r,before);
}
#[test]fn legacy_charged_moonbeam_out_of_range_still_discharges_without_attack(){
 let mut r=room();r.simulation_revision=35.0.into();r.floor_mut().unwrap().non_euclidean=Field::Missing;r.player.x=30.;
 let id=rat(&mut r,10.,10.,false);let a=engine::actor_mut(&mut r,id).unwrap();a.state="hunting".into();a.abilities=vec!["enemy:beast:moon-laser".into()];a.conditions.insert("laserCharge".into(),1.);
 assert!(corpse_core::monster_moon::special_action(&mut r,id,0.,false).unwrap());assert!(!engine::actor(&r,id).unwrap().conditions.contains_key("laserCharge"));assert_eq!(r.player.hp,100.);
}
#[test]fn witch_over_range_random_branch_is_a_paid_cooled_down_miss(){
 let mut r=room();r.player.x=1.;r.player.y=10.;let w=r.floor().unwrap().width();
 r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![10*w+5,10*w+6,11*w+6],groups:vec![Group{sources:vec![10*w+5,11*w+6],destination:10*w+6}]}].into();
 let caster=rat(&mut r,9.,10.,false);let a=engine::actor_mut(&mut r,caster).unwrap();a.state="hunting".into();a.abilities=vec!["enemy:beast:witch".into()];a.energy=100.;
 let other=rat(&mut r,1.,11.,true);r.rng=8192.;let tick=r.tick;
 assert_eq!(projectiles::possible_shot_range(&r,engine::actor(&r,caster).unwrap(),&r.player,7.),Some(7.));
 assert!(corpse_core::monster_spells::special_action(&mut r,caster,0.).unwrap());
 assert!(!r.player.conditions.contains_key("slowed"));assert!(!engine::actor(&r,other).unwrap().conditions.contains_key("slowed"));
 let a=engine::actor(&r,caster).unwrap();assert_eq!(a.energy,92.);assert_eq!(a.conditions.get("monsterSpellPhase"),Some(&1.));assert_eq!(a.conditions.get("monsterSpellReadyAt"),Some(&(tick+3.)));assert_eq!(a.conditions.get("rangedRecovery"),Some(&1.));let mut expected=8192;corpse_core::rng::next_u32(&mut expected);assert_eq!(r.rng,expected as f64);
}
