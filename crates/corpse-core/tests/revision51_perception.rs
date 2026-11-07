use corpse_core::{Actor,Floor,Run,vision,revision51_perception as rules,engine,ai};
use std::sync::Arc;
fn room(revision:f64,light:f64)->Run {
 let mut r=Run{depth:1.,simulation_revision:revision.into(),status:"playing".into(),..Default::default()};
 r.player=Actor{id:1.,x:2.,y:2.,hp:100.,max_hp:100.,state:"ally".into(),..Default::default()};
 r.player.stats.perception=0.;r.player.stats.max_energy=100.;
 r.floors.push(Arc::new(Floor{depth:1.,tiles:vec!["floor".into();1681],lights:vec![light;1681],seen:vec![false;1681],..Default::default()}));r
}
fn enemy(x:f64)->Actor {Actor{id:2.,x,y:2.,hp:100.,max_hp:100.,state:"wandering".into(),..Default::default()}}
#[test]
fn player_uses_own_perception_for_both_tiles_and_creatures() {
 let mut r=room(51.,0.);let mut a=enemy(12.);
 assert!(vision::visible(&r,22,2));assert!(!vision::visible(&r,23,2));
 assert!(vision::directly_visible_actor(&r,&a));a.x=13.;assert!(!vision::directly_visible_actor(&r,&a));
 r.player.stats.perception=100.;a.x=22.;assert!(vision::directly_visible_actor(&r,&a));a.x=23.;assert!(!vision::directly_visible_actor(&r,&a));
 r.player.stats.perception=5.;assert!(vision::visible(&r,23,2));
}
#[test]
fn lit_tiles_and_creatures_require_line_of_sight_not_distance() {
 let mut r=room(51.,1.);let a=enemy(40.);
 assert!(vision::visible(&r,40,2));assert!(vision::directly_visible_actor(&r,&a));
 r.floor_mut().unwrap().tiles[2*41+20]="wall".into();
 assert!(vision::visible(&r,20,2));assert!(!vision::visible(&r,40,2));assert!(!vision::directly_visible_actor(&r,&a));
}
#[test]
fn actor_stealth_hides_actor_not_the_tile_and_invisibility_is_preserved() {
 let r=room(51.,0.);let mut a=enemy(8.);a.conditions.insert("stealth".into(),5.);
 assert!(vision::visible(&r,8,2));assert!(!vision::directly_visible_actor(&r,&a));
 a.x=7.;assert!(vision::directly_visible_actor(&r,&a));
 a.conditions.insert("invisible".into(),10.);assert!(!vision::directly_visible_actor(&r,&a));
}
#[test]
fn fractional_stealth_is_added_without_legacy_rounding() {
 let mut r=room(51.,0.);r.player.stats.perception=5.;let mut a=enemy(12.);a.conditions.insert("stealth".into(),0.5);
 assert!(vision::directly_visible_actor(&r,&a));
 assert!(vision::VisibilityProjection::new(&r).directly_visible_actor(&a));
 a.conditions.insert("submergedStealth".into(),0.25);
 assert!(!vision::directly_visible_actor(&r,&a));
 a.x=11.;assert!(vision::directly_visible_actor(&r,&a));
}
#[test]
fn awareness_is_fixed_after_shared_plausible_sight() {
 let mut r=room(51.,0.);let mut a=enemy(12.);
 assert_eq!(vision::detection_chance(&r,&a,&r.player),25.);
 a.state="sleeping".into();assert_eq!(vision::detection_chance(&r,&a,&r.player),10.);
 a.x=13.;r.floor_mut().unwrap().lights.fill(1.);assert_eq!(vision::detection_chance(&r,&a,&r.player),0.);
 a.state="wandering".into();assert_eq!(vision::detection_chance(&r,&a,&r.player),25.);
 a.state="hunting".into();assert_eq!(vision::detection_chance(&r,&a,&r.player),100.);
 r.player.conditions.insert("invisible".into(),10.);assert_eq!(vision::detection_chance(&r,&a,&r.player),0.);
 a.conditions.insert("telepathic".into(),10.);assert_eq!(vision::detection_chance(&r,&a,&r.player),100.);
 a.state="wandering".into();assert_eq!(vision::detection_chance(&r,&a,&r.player),25.);
}
#[test]
fn negative_lighting_and_stealth_are_shared_with_ally_target_acquisition() {
 let mut r=room(51.,-50.);let a=enemy(8.);r.actors_at_mut(1.).push(a);
 assert!(ai::nearest_ally_enemy(&r,&r.player).is_none());
 r.actors_at_mut(1.)[0].x=7.;assert!(ai::nearest_ally_enemy(&r,&r.player).is_some());
}
#[test]
fn hunting_does_not_expire_into_recovery_sleep_en_route() {
 let r=room(51.,0.);let mut a=enemy(20.);a.state="hunting".into();a.stats.max_energy=100.;a.energy=0.;
 a.conditions.insert("monsterUnseenTurns".into(),20.);a.last_seen=corpse_core::ActorLastSeen{x:2.,y:2.}.into();
 assert!(!ai::advance_rest(&r,&mut a,false));assert_eq!(a.state,"hunting");assert!(a.last_seen.as_ref().is_some());
}
#[test]
fn legacy_saved_rules_do_not_change() {
 let mut r=room(50.,0.);let a=enemy(19.);
 assert!(vision::directly_visible_actor(&r,&a));assert!(vision::visible(&r,40,2));
 r.player.stats.perception=0.;assert!(vision::directly_visible_actor(&r,&a));
 let old=corpse_core::startup::new_run_shell_revision("0-1-2:vision","Avatar",false,50.).unwrap();
 let new=corpse_core::startup::new_run_shell_revision("0-1-2:vision","Avatar",false,51.).unwrap();
 assert_eq!(old.player.stats.perception,300.);assert_eq!(new.player.stats.perception,0.);
}
#[test]
fn projected_scalar_visibility_and_memory_match_new_rules() {
 let mut r=room(51.,-15.);
 for i in 0..12 {let mut a=enemy((3+i)as f64);a.id=(2+i)as f64;a.conditions.insert("stealth".into(),(i%3)as f64);r.actors_at_mut(1.).push(a);}
 let p=vision::VisibilityProjection::new(&r);
 for a in r.actors_at(1.) {assert_eq!(p.directly_visible_actor(a),vision::directly_visible_actor(&r,a));}
 assert_eq!(vision::terrain_visibility(&r),(0..1681).map(|i|vision::visible(&r,i%41,i/41)).collect::<Vec<_>>());
 engine::remember(&mut r);assert!(r.floor().unwrap().seen[2*41+19]);assert!(!r.floor().unwrap().seen[2*41+20]);
 let before=r.clone();let _=vision::perception_tile_mask(&r,&r.player);assert_eq!(r,before);
}
#[test]
fn water_has_only_the_authored_submerged_stealth_not_legacy_perception_multiplier() {
 let mut r=room(51.,0.);let a=enemy(13.);r.floor_mut().unwrap().tiles.fill("water".into());
 assert!(!vision::directly_visible_actor(&r,&a));
}
#[test]
fn same_plane_and_portal_optical_distance_are_shared() {
 let mut r=room(51.,0.);let mut a=enemy(39.);a.state="hunting".into();
 assert!(!vision::plausibly_visible_actor(&r,&a,&r.player));
 r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:2*41+3,b:2*41+38,color_index:0}].into();
 assert!(vision::plausibly_visible_actor(&r,&a,&r.player));assert_eq!(vision::detection_chance(&r,&a,&r.player),100.);
 corpse_core::dreamwalking::ensure_layer(&mut r);a.conditions.insert("phantasmiiPlane".into(),1.);assert!(!vision::plausibly_visible_actor(&r,&a,&r.player));
}
#[test]
fn pure_sleep_awareness_cap_is_inclusive_and_bright_light_does_not_bypass_it() {
 assert_eq!(rules::awareness_chance(true,false,true,false,10.),10.);
 assert_eq!(rules::awareness_chance(true,false,true,false,10.001),0.);
 assert_eq!(rules::awareness_chance(false,false,false,true,1.),0.);
 assert_eq!(rules::awareness_chance(true,true,false,false,100.),100.);
}
