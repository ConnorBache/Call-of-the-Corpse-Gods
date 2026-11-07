use corpse_core::{Actor,Command,Field,Run,Stats,engine,projectiles};
use corpse_core::revision50_amulet_effects::{self as amulets,Effect};
fn room()->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=51.0.into();r.player.x=10.;r.player.y=10.;r.player.hp=100.;r.player.max_hp=100.;r.player.base_max_hp=100.0.into();r.player.energy=100.;
 r.player.stats=Stats{perception:300.,max_energy:100.,..Default::default()};r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
 r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
 r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
 r.inventory.clear();r.actors.clear();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;r.phase="normal".to_string().into();
 let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.remembered_tiles=vec!["floor".into();f.tiles.len()].into();
 f.doors=Field::Missing;f.environment=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;f.godbeast_event=Field::Missing;r
}
fn enemy(r:&mut Run,id:f64,x:f64){let a=Actor{id,x,y:10.,hp:500.,max_hp:500.,state:"wandering".into(),..Default::default()};r.actors_at_mut(r.depth).push(a);}
fn put(r:&mut Run,kind:Effect){r.inventory.push(amulets::make_amulet(kind,900.).unwrap());}
fn cast()->Command{Command::UseAt{id:900.,x:18.,y:10.}}
#[test]fn blocked_blink_is_atomic_and_never_shortens(){
 for kind in ["creature","wall","door","Borge","Epidite"] {
  let mut r=room();put(&mut r,Effect::Blinking);let w=r.floor().unwrap().width();
  match kind {
   "creature"=>enemy(&mut r,20.,12.),
   "wall"=>r.floor_mut().unwrap().tiles[10*w+12]="wall".into(),
   "door"=>r.floor_mut().unwrap().doors=serde_json::from_value(serde_json::json!({(10*w+12).to_string():{"open":false}})).unwrap(),
   kind=>r.floor_mut().unwrap().godbeast_event=corpse_core::GodbeastEvent{kind:kind.into(),x:14.,y:10.,..Default::default()}.into(),
  }
  let before=r.clone();assert!(amulets::admission(&r,&cast()).unwrap().is_err(),"{kind}");
  assert!(amulets::command(&mut r,&cast()).is_err());assert_eq!(r,before,"{kind}");
 }
}
#[test]fn observed_delivery_matches_effects_and_preserves_lightning_piercing(){
 for kind in [Effect::Lightning,Effect::Firebolt,Effect::Poison,Effect::Acid] {
  let mut r=room();put(&mut r,kind);enemy(&mut r,20.,14.);enemy(&mut r,21.,20.);
  let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+25]="wall".into();
  let preview=amulets::aim_preview(&r,&r.inventory[0],11.,10.);let before=r.clone();
  assert_eq!(amulets::aim_preview(&r,&r.inventory[0],11.,10.),preview);assert_eq!(r,before);
  amulets::command(&mut r,&Command::UseAt{id:900.,x:11.,y:10.}).unwrap();
  if kind==Effect::Lightning {assert_eq!(preview.delivery.last(),Some(&(24,10)));assert_eq!(preview.impact,Some((25,10)));assert_eq!(engine::actor(&r,21.).unwrap().hp,450.);}
  else {assert_eq!(preview.delivery.last(),Some(&(14,10)));assert_eq!(preview.impact,Some((14,10)));assert_eq!(engine::actor(&r,21.).unwrap().hp,500.);}
 }
}
#[test]fn fixed_fixtures_block_physical_rays_only_from_revision51(){
 for kind in ["Borge","Epidite"] {
  let mut r=room();r.floor_mut().unwrap().godbeast_event=corpse_core::GodbeastEvent{kind:kind.into(),x:15.,y:10.,..Default::default()}.into();
  assert!(projectiles::trace(&r,20.,10.).x<15.);let p=amulets::aim_preview(&r,&amulets::make_amulet(Effect::Lightning,900.).unwrap(),11.,10.);
  assert!(!p.delivery.contains(&(15,10)));assert!(p.impact.is_some());
  r.simulation_revision=50.0.into();assert_eq!(projectiles::trace(&r,20.,10.).x,20.);
 }
}
#[test]fn hidden_occupancy_and_remembered_terrain_do_not_leak(){
 let mut r=room();r.player.conditions.clear();r.player.stats.perception=-1000.;let f=r.floor_mut().unwrap();f.lights.fill(-100.);f.light_sources=Field::Missing;let w=f.width();
 let item=amulets::make_amulet(Effect::Lightning,900.).unwrap();let before=amulets::aim_preview(&r,&item,30.,10.);
 assert!(!corpse_core::vision::visible(&r,20,10));assert!(!before.delivery.contains(&(20,10)));
 r.floor_mut().unwrap().tiles[10*w+20]="wall".into();enemy(&mut r,20.,19.);assert_eq!(amulets::aim_preview(&r,&item,30.,10.),before);
 let mut r=room();r.player.conditions.clear();put(&mut r,Effect::Blinking);let preview=amulets::aim_preview(&r,&r.inventory[0],18.,10.);
 enemy(&mut r,20.,12.);engine::actor_mut(&mut r,20.).unwrap().conditions.insert("invisible".into(),10.);
 assert!(!corpse_core::vision::visible_actor(&r,engine::actor(&r,20.).unwrap()));assert_eq!(amulets::aim_preview(&r,&r.inventory[0],18.,10.),preview);
 let before=r.clone();assert!(amulets::command(&mut r,&cast()).is_err());assert_eq!(r,before);
}
#[test]fn acid_physical_portal_distance_and_blink_endpoint_range(){
 let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
 put(&mut r,Effect::Acid);let preview=amulets::aim_preview(&r,&r.inventory[0],11.,10.);
 assert!(preview.delivery.contains(&(30,10)));assert_eq!(preview.delivery.last(),Some(&(36,10)));
 enemy(&mut r,20.,37.);amulets::command(&mut r,&Command::UseAt{id:900.,x:11.,y:10.}).unwrap();assert_eq!(engine::actor(&r,20.).unwrap().hp,500.);
 r.inventory.clear();put(&mut r,Effect::Blinking);assert!(amulets::admission(&r,&Command::UseAt{id:900.,x:30.,y:10.}).unwrap().is_err());
}
#[test]fn iceblast_is_area_only_and_beckoning_exposes_pull_route(){
 let mut r=room();let p=amulets::aim_preview(&r,&amulets::make_amulet(Effect::Iceblast,900.).unwrap(),15.,10.);
 assert!(p.delivery.is_empty());assert_eq!(p.area.len(),9);assert_eq!(p.impact,Some((15,10)));
 enemy(&mut r,20.,15.);let item=corpse_core::Item{id:900.,kind:"amulet".into(),effect:"beckoning".to_string().into(),..Default::default()};
 let before=r.clone();let p=amulets::aim_preview(&r,&item,15.,10.);assert_eq!(p.delivery,vec![(15,10),(14,10),(13,10),(12,10),(11,10)]);assert_eq!(r,before);
}

#[test]fn beckoning_preview_respects_known_movement_restrictions(){
 let mut r=room();enemy(&mut r,20.,15.);let w=r.floor().unwrap().width();
 let item=corpse_core::Item{id:900.,kind:"amulet".into(),effect:"beckoning".to_string().into(),..Default::default()};
 // All eight candidate routes cross x=14 near the target.
 r.floor_mut().unwrap().grates=vec![(9*w+14)as f64,(10*w+14)as f64,(11*w+14)as f64].into();
 let before=r.clone();assert!(amulets::aim_preview(&r,&item,15.,10.).delivery.is_empty());assert_eq!(r,before);
 r.floor_mut().unwrap().grates=Field::Missing;
 engine::actor_mut(&mut r,20.).unwrap().abilities.push("enemy:beast:rooted".into());
 assert!(amulets::aim_preview(&r,&item,15.,10.).delivery.is_empty());
}
