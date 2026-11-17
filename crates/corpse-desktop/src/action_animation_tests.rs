//! Native raster and command-driven R52 action receipts; no screenshot mockups.
use super::*;
use corpse_core::Run;
use crate::render::View;
use corpse_core::events::{AnimationCue,Delivery,EventPoint,PresentationEvent,Stage};
fn event(key:&str,style:&str,stage:Stage,action:u32,cells:&[(f64,f64)])->PresentationEvent{
 let cells:Vec<_>=cells.iter().map(|&(x,y)|EventPoint{x,y}).collect();
 PresentationEvent{kind:"animation".into(),from:None,to:cells.last().cloned().unwrap_or(EventPoint{x:10.,y:10.}),glyph:None,color:None,label:None,depth:1.,
 animation:Some(AnimationCue{delivery:if stage==Stage::Travel&&(style.starts_with("bolt")||matches!(style,"ember"|"dart"|"glob")){if key=="shared"||key=="E61"{Delivery::Projectile}else{Delivery::Magic}}else{Delivery::Local},proposal:key.into(),style:style.into(),stage,action,cells,surface:None,item_id:None})}
}
fn capture(p:&Painter,name:&str){if let Ok(dir)=std::env::var("CORPSE_ACTION_REVIEW"){std::fs::create_dir_all(&dir).unwrap();let f=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();let mut enc=png::Encoder::new(f,p.width as u32,p.height as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();}}
#[test]fn action_animation_exact_cells_order_and_instant(){
 let route=[(2.,3.),(3.,3.),(15.,11.)];let mut fx=effects::Effects::default();
 fx.add(vec![event("A01","bolt",Stage::Health,0,&route[2..]),event("A01","bolt",Stage::Travel,0,&route),event("E70","collapse",Stage::Death,0,&route[2..])],1.,1.,0.,false);
 assert_eq!(fx.cues[0].event.animation.as_ref().unwrap().stage,Stage::Travel);
 assert!(fx.cues[0].started<fx.cues[1].started&&fx.cues[1].started<fx.cues[2].started);
 for time in [0.,30.,80.,130.]{for t in effects::tiles(&fx.cues[0],time,false){assert!(route.contains(&(t.x as f64,t.y as f64)),"invented route cell");}}
 let a=effects::tiles(&fx.cues[0],20.,true);let b=effects::tiles(&fx.cues[0],100.,true);assert_eq!(a.len(),b.len());assert!(a.iter().zip(&b).all(|(a,b)|a.mark==b.mark&&a.light==b.light));
 fx.configure_motion(true,true);fx.add(vec![event("A01","bolt",Stage::Travel,0,&route),event("E70","collapse",Stage::Death,0,&route[2..])],2.,1.,400.,false);
 assert!(fx.cues.iter().all(|c|c.started==400.));assert_eq!(fx.cues[0].event.animation.as_ref().unwrap().cells.len(),1);
}
#[test]fn action_animation_families_use_uniform_tile_light_not_vector_art(){
 let styles=["slash","puncture","droplet","venom-two","venom-four","bracket","root","speed","eye","shield","cracked-shield","bolt","slab","miss","rejected","gear","ember","stitch","dip","dissolve","diamond","mask","split-ring","crossed-ring","wave","sound"];
 let mut p=Painter::new();let mut expected=Painter::new();
 expected.clear([22,23,29]);expected.blend_rect(20,20,60,60,[220,190,120],58);
 for style in styles {p.clear([22,23,29]);action_marks::paint(&mut p,&action_marks::Mark{style:style.into(),progress:600,direction:Some((1,1))},20,20,60,60,[220,190,120],0.9);assert_eq!(p.pixels,expected.pixels,"decorative geometry for {style}");}
 capture(&p,"light-only-feedback-tile");
}

fn room()->Run{
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.next_id=1000.;r.player.x=10.;r.player.y=10.;r.player.hp=100.;r.player.max_hp=100.;r.player.energy=200.;r.player.abilities=vec!["isheilah-spellslinger".into()];r.player.active.clear();r.player.conditions.clear();r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);r.inventory.clear();r.actors.clear();r.ground.clear();for d in r.floors.iter().map(|f|f.depth).collect::<Vec<_>>(){r.actors_at_mut(d);r.ground_at_mut(d);}r.offers.clear();let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.doors=Field::Missing;f.environment=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.godbeast_event=Field::Missing;
 let mut a=corpse_core::population::create_actor_revision("rat",123.,14.,10.,false,52.).unwrap();a.hp=100.;a.max_hp=100.;r.actors_at_mut(r.depth).push(a);r
}
#[test]fn action_animation_actual_amulet_native_captures(){
 use corpse_core::revision50_amulet_effects::{self as a,Effect};
 for(kind,key,name)in [(Effect::Lightning,"A01","lightning"),(Effect::Firebolt,"A02","firebolt"),(Effect::Poison,"A03","poison"),(Effect::Iceblast,"A04","iceblast"),(Effect::Acid,"A05","acid"),(Effect::Blinking,"A06","blink")]{
  let mut r=room();r.inventory.push(a::make_amulet(kind,900.).unwrap());let target=if kind==Effect::Blinking{(12.,12.)}else{(14.,10.)};
  let out=corpse_core::engine::dispatch(&mut r,&Command::UseAt{id:900.,x:target.0,y:target.1}).unwrap();
  assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal==key)),"actual command missing {key}");
  let saved=serde_json::to_vec(&r).unwrap();let view=observe::observe(&r);let mut fx=effects::Effects::default();fx.add(out.events,r.tick,r.depth,0.,false);
  let mut p=Painter::new();for(time,label)in [(50.,"travel"),(150.,"special"),(220.,"health"),(290.,"end")]{p.game_at(&view,"",false,0.);p.effects(&view,&fx.cues,time,false);capture(&p,&format!("{name}-{label}"));}
  p.game_at(&view,"",false,0.);p.effects(&view,&fx.cues,190.,true);capture(&p,&format!("{name}-reduced"));
  let mut pack=pack::PackView::observe(&r);pack.configure(&p,p.frame().map.tuple());p.game_at(&view,"",false,0.);pack.paint(&mut p);pack.paint_action_marks(&mut p,&fx.cues,180.);effects::paint_inventory_hud(&mut p,&view,&fx.cues,180.,true);capture(&p,&format!("{name}-pack-reserve"));
  assert_eq!(serde_json::to_vec(&r).unwrap(),saved);
 }
}

#[test]fn action_animation_busy_room_latency_and_interruption_are_bounded(){
 let mut fx=effects::Effects::default();let events=(0..200).flat_map(|action|[Stage::Travel,Stage::Impact,Stage::Special,Stage::Health,Stage::Death].map(move|stage|event("E70","collapse",stage,action,&[(2.,3.)]))).collect();
 fx.add(events,1.,1.,1000.,false);assert!(fx.animate(1451.));
 for pair in fx.cues.chunks(5).collect::<Vec<_>>().windows(2) { let end=pair[0].iter().map(|c|c.started+effects::duration(&c.event)).fold(0f64,f64::max);assert!(pair[1][0].started>=end); }
 fx.add(vec![event("P23","speed",Stage::Travel,0,&[(3.,3.)])],2.,1.,1100.,false);assert_eq!(fx.cues.len(),1);assert_eq!(fx.cues[0].started,1100.);
 fx.add(vec![],2.,1.,1101.,true);assert!(fx.cues.is_empty());
}

#[test]fn action_animation_actual_potion_remainder_and_consumed_row_captures(){
 let mut r=room();r.player.abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into()];r.actors.clear();r.inventory.push(corpse_core::Item{id:900.,kind:"potion".into(),name:"Potion of Healing".into(),effect:"healing".to_string().into(),identified:true.into(),..Default::default()});
 for label in ["potion-retained-half","potion-consumed-half"]{
  let out=corpse_core::engine::dispatch(&mut r,&Command::Use{id:900.}).unwrap();assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P66")));
  let mut fx=effects::Effects::default();fx.add(out.events,r.tick,r.depth,0.,false);let view=observe::observe(&r);let mut p=Painter::new();p.game_at(&view,"",false,0.);let mut pack=pack::PackView::observe(&r);pack.configure(&p,p.frame().map.tuple());pack.paint(&mut p);pack.paint_action_marks(&mut p,&fx.cues,180.);effects::paint_inventory_hud(&mut p,&view,&fx.cues,180.,true);capture(&p,label);
 }
}

#[test]fn action_animation_inventory_anchors_and_exact_god_color(){
 let r=room();let view=observe::observe(&r);
 for (width,height)in [(1280,800),(1600,900),(960,600)] {
  let mut p=Painter::new();p.resize(width,height);p.set_responsive_canvas(true);
  let mut cues=Vec::new();for (key,style,id)in [("A01","reserve-drain",Some(900.)),("A01","energy-drain",None),("P65","spark-god-11",Some(900.))] {
   let mut e=event(key,style,Stage::Special,0,&[]);let a=e.animation.as_mut().unwrap();a.surface=Some("inventory".into());a.item_id=id;cues.push(e);
  }
  let mut fx=effects::Effects::default();fx.add(cues,1.,1.,0.,false);
  assert!(effects::inventory_marks(&fx.cues,80.).iter().any(|(_,m,c,_)|m.style=="spark-god-11"&&*c==ability_ui::god_color(11)));
  let energy=effects::inventory_hud_anchor(&p,&action_marks::Mark{style:"energy-drain".into(),progress:500,direction:None});assert_eq!(energy.0,p.hud_left()+12);assert!(energy.1<=118&&energy.1+energy.3>=126);
  let pack_anchor=effects::inventory_hud_anchor(&p,&action_marks::Mark{style:"pack-capacity".into(),progress:500,direction:None});assert_eq!(p.toolbar_action(pack_anchor.0+12,pack_anchor.1+12),Some(render::TOOLBAR_VISIBLE_ACTIONS[0]));
  p.game_at(&view,"",false,0.);effects::paint_inventory_hud(&mut p,&view,&fx.cues,80.,false);capture(&p,&format!("hud-closed-pack-{width}x{height}"));
  let mut pack=pack::PackView::observe(&r);pack.configure(&p,p.frame().map.tuple());pack.paint(&mut p);pack.paint_action_marks(&mut p,&fx.cues,80.);effects::paint_inventory_hud(&mut p,&view,&fx.cues,80.,true);capture(&p,&format!("hud-open-pack-{width}x{height}"));
 }
}
#[test]fn action_animation_actual_control_amulet_native_captures(){
 for(effect,key)in [("beckoning","A07"),("discord","A08"),("obstruction","A09"),("roots","A10"),("teleportation","A11"),("floods","A12")]{
  let mut r=room();let mut item=corpse_core::revision50_amulet_effects::make_amulet(corpse_core::revision50_amulet_effects::Effect::Lightning,900.).unwrap();item.effect=effect.to_string().into();item.name=corpse_core::revision50_control_amulets::display_name(&item).unwrap().into();r.inventory.push(item);
  let (x,y)=if effect=="obstruction"{(13.,10.)}else{(14.,10.)};let out=corpse_core::engine::dispatch(&mut r,&Command::UseAt{id:900.,x,y}).unwrap();assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal==key)),"{effect}");let view=observe::observe(&r);let mut fx=effects::Effects::default();fx.add(out.events,r.tick,r.depth,0.,false);let mut p=Painter::new();
  for(time,label)in [(50.,"travel"),(150.,"special"),(220.,"health"),(290.,"end")]{p.game_at(&view,"",false,0.);p.effects(&view,&fx.cues,time,false);capture(&p,&format!("{effect}-{label}"));}
 }
}
#[test]fn action_animation_arrival_and_departure_light_exact_endpoints(){
 for style in ["unfold","unfold-moon","mask-unfold","bracket-unfold","fold","fold-moon","mask-iris"] {
  let mut e=event("P13",style,Stage::Travel,0,&[(2.,3.),(15.,11.)]);e.glyph=Some("@".into());
  let cue=effects::Cue{event:e,started:0.,held:false,static_marks:false};
  for age in [0.,30.,80.,130.] {let tiles=effects::tiles(&cue,age,false);assert!(tiles.iter().all(|t|t.glyph.is_none()&&t.mark.is_none()));assert!(tiles.iter().all(|t|[(2,3),(15,11)].contains(&(t.x,t.y))));}
 }
}

#[test]fn action_animation_duel_boundary_stays_at_original_anchor(){
 let mut r=room();r.player.abilities.push("pelian-fated-duel".into());r.player.active.push("pelian-fated-duel".into());for(k,v)in[("r50DuelX",11.),("r50DuelY",10.),("r50DuelDepth",r.depth)]{r.player.conditions.insert(k.into(),v);}
 let before=observe::observe(&r);let edges=|v:&View|v.cells.iter().filter(|c|c.duel_edges.iter().any(|&e|e)).map(|c|((c.x,c.y),c.duel_edges)).collect::<Vec<_>>();assert!(!edges(&before).is_empty());r.actors_at_mut(r.depth)[0].x=17.;let after=observe::observe(&r);assert_eq!(edges(&before),edges(&after));let mut p=Painter::new();p.game_at(&after,"",false,2000.);capture(&p,"duel-persistent-original-boundary");r.player.active.clear();assert!(edges(&observe::observe(&r)).is_empty());
}
#[test]fn action_animation_plane_crossfade_is_observed_only_and_interruptible(){
 let r=room();let mut before=observe::observe(&r);let mut after=observe::observe(&r);before.cells.retain(|c|c.x<13);after.cells.retain(|c|c.x>=13);let mut p=Painter::new();let mut hashes=Vec::new();
 for age in [0.,125.,249.,250.] {p.game_at(&after,"",false,0.);p.plane_crossfade(&before,age);hashes.push(crc32fast::hash(&p.pixels));capture(&p,&format!("plane-crossfade-{}",age as i32));}assert_ne!(hashes[0],hashes[1]);assert_eq!(hashes[2],hashes[3]);
 let mut fx=effects::Effects::default();fx.plane_changed(before,100.);assert!(fx.animate(200.));fx.add(vec![],1.,1.,210.,false);assert!(fx.plane_transition.is_none());fx.configure_motion(true,false);fx.plane_changed(observe::observe(&r),300.);assert!(fx.plane_transition.is_none());fx.configure_motion(false,true);fx.plane_changed(observe::observe(&r),300.);assert!(fx.plane_transition.is_none());
}
#[test]fn action_animation_real_dreamwalk_and_wake_crossfade_captures(){
 let mut r=room();r.actors_at_mut(r.depth).clear();r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,depth:r.depth,..Default::default()}.into();let mut engine=temporal::Engine::new(r).unwrap();let mut fx=effects::Effects::default();let mut p=Painter::new();
 for (id,label)in [(corpse_core::dreamwalking::DREAMWALK_ID,"dream-enter"),(corpse_core::dreamwalking::DREAMWALK_ID,"dream-wake")] {
  dispatch(&mut engine,&mut fx,100.,&Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();assert!(fx.plane_transition.is_some(),"{label}");let view=observe::observe(&engine.run);let transition=fx.plane_transition.as_ref().unwrap();
  for age in [0.,125.,250.] {p.game_at(&view,"",false,0.);p.plane_crossfade(&transition.before,age);capture(&p,&format!("{label}-{}",age as i32));}
 }
}
#[test]fn action_animation_symbolic_links_light_endpoints_without_drawing_a_path(){
 for style in ["droplet-return","arc-redirect","arc-swap-forward","stitch-thought","notch-target","fracture-trait:enemy:beast:venom","wave-advance","speed-push"] {
  let e=event("P68",style,Stage::Special,0,&[(2.,3.),(15.,11.)]);
  let cue=effects::Cue{event:e,started:0.,held:false,static_marks:false};
  let tiles=effects::tiles(&cue,30.,false);assert_eq!(tiles.len(),2);assert!(tiles.iter().all(|t|t.glyph.is_none()&&t.mark.is_none()));assert_eq!((tiles[0].x,tiles[0].y),(2,3));assert_eq!((tiles[1].x,tiles[1].y),(15,11));
 }
}

#[test]fn action_animation_reaction_and_commandment_use_local_light(){
 let mut r=room();r.player.abilities=vec!["isheilah-quick-hand".into()];r.player.active=r.player.abilities.clone();r.player.reactions=0.0.into();let view=observe::observe(&r);let mut p=Painter::new();p.game_at(&view,"",false,0.);let a=p.reaction_action_anchor(&view,"reaction-spent");assert!(a.1>=ability_ui::top(&p.profile));let mut e=event("P16","reaction-spent",Stage::Special,0,&[]);e.animation.as_mut().unwrap().surface=Some("inventory".into());let mut fx=effects::Effects::default();fx.add(vec![e],1.,1.,0.,false);effects::paint_inventory_hud(&mut p,&view,&fx.cues,180.,false);capture(&p,"reaction-spent-actual-cost-row");
 let mut hashes=std::collections::HashSet::new();for style in ["wave-pressure","speed-commandment","stitch-grapple","reaction-spent","fracture-status:poisoned","fracture-status:slowed"] {p.clear([0,0,0]);action_marks::paint(&mut p,&action_marks::Mark{style:style.into(),progress:500,direction:Some((1,0))},30,30,60,60,[255,255,255],1.);hashes.insert(crc32fast::hash(&p.pixels));}assert_eq!(hashes.len(),1,"resource feedback must use one uniform tile-light primitive");
}

#[test]fn action_animation_tiled_crossfade_and_inventory_match_full_every_frame(){
 let r=room();let before=observe::observe(&r);let mut after=observe::observe(&r);after.cells.retain(|c|c.x>=12);
 let mut tiled=Painter::new();let mut full=Painter::new();let mut plan=render::tile_damage::TilePlanner::default();let mut track=tile_overlay::Tracker::default();plan.replace_view(None,&after,&tiled);
 let mut fx=effects::Effects::default();let mut e=event("A01","energy-drain",Stage::Special,0,&[]);e.animation.as_mut().unwrap().surface=Some("inventory".into());fx.add(vec![e],1.,1.,0.,false);fx.plane_changed(before,0.);
 for age in [0.,125.,180.,249.,250.,300.] {
  fx.prune(age);let mut panels=Vec::new();if fx.plane_transition.is_some(){let b=tiled.frame().map;panels.extend(tiled.logical_damage_rect(b.0,b.1,b.2,b.3));}if !effects::inventory_marks(&fx.cues,age).is_empty(){panels.extend(tiled.logical_damage_rect(tiled.hud_left(),40,tiled.logical_width()-tiled.hud_left(),662));}
  let state=tile_overlay::Frame{panels,redraw_panels:fx.redraw_panels(age),..Default::default()};for r in track.changes(&tiled,&state){plan.mark_native_rect(r);}tile_overlay::finish_dependencies(&mut tiled,&mut plan,&state);plan.paint_base(&mut tiled,&after,"",0.);if !tiled.tile_mask.is_empty(){if let Some(t)=fx.plane_transition.as_ref(){tiled.plane_crossfade(&t.before,age);}effects::paint_inventory_hud(&mut tiled,&after,&fx.cues,age,false);}tiled.tile_clip_enabled=false;
  full.game_at(&after,"",false,0.);if let Some(t)=fx.plane_transition.as_ref(){full.plane_crossfade(&t.before,age);}effects::paint_inventory_hud(&mut full,&after,&fx.cues,age,false);assert_eq!(crc32fast::hash(&tiled.pixels),crc32fast::hash(&full.pixels),"tile/full phase {age}");capture(&tiled,&format!("tile-crossfade-{}",age as i32));track.commit(state);plan.clear_frame();
 }
}

#[test]fn action_animation_inventory_normal_fades_reduced_and_instant_stay_static(){
 for (reduced,instant)in [(false,false),(true,false),(false,true)] {let mut fx=effects::Effects::default();fx.configure_motion(reduced,instant);let mut e=event("P66","split-potion",Stage::Special,0,&[]);e.animation.as_mut().unwrap().surface=Some("inventory".into());fx.add(vec![e],1.,1.,0.,false);let start=fx.cues[0].started;let a=effects::inventory_marks(&fx.cues,start+20.);let b=effects::inventory_marks(&fx.cues,start+90.);assert_eq!(a[0].1==b[0].1,reduced||instant);assert_eq!(a[0].3==b[0].3,reduced||instant);}
 let mut p=Painter::new();let mut hashes=Vec::new();for(style,t)in [("split-potion",100),("split-potion",900),("fade-potion",500),("reserve-rise",100),("reserve-rise",900)] {p.clear([0,0,0]);action_marks::paint(&mut p,&action_marks::Mark{style:style.into(),progress:t,direction:None},30,30,60,60,[255,255,255],1.);hashes.push(crc32fast::hash(&p.pixels));}assert!(hashes.iter().all(|h|*h==hashes[0]),"progress must not deform an inventory symbol");
}
#[test]fn action_animation_legacy_plane_dispatch_has_no_new_crossfade(){let mut r=room();r.simulation_revision=51.0.into();r.actors_at_mut(r.depth).clear();r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,depth:r.depth,..Default::default()}.into();let mut engine=temporal::Engine::new(r).unwrap();let mut fx=effects::Effects::default();dispatch(&mut engine,&mut fx,100.,&Command::Ability{id:corpse_core::dreamwalking::DREAMWALK_ID.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();assert!(corpse_core::dreamwalking::active(&engine.run));assert!(fx.plane_transition.is_none());}

#[test]fn action_animation_actual_ascend_departure_column_capture(){
 let mut r=room();r.depth=2.;r.actors_at_mut(1.).clear();r.floor_mut().unwrap().tiles.fill("floor".into());r.floor_mut().unwrap().lights.fill(100.);let (x,y)=(r.player.x as usize,r.player.y as usize);let f=std::sync::Arc::make_mut(&mut r.floors[0]);let w=f.width();f.tiles[y*w+x]="chasm".into();r.player.conditions.clear();r.player.abilities=vec!["anavani-ascend".into()];r.player.ability_reserves=Field::Missing;corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,"anavani-ascend",50.,50.).unwrap();let mut engine=temporal::Engine::new(r).unwrap();let mut fx=effects::Effects::default();dispatch(&mut engine,&mut fx,0.,&Command::Ability{id:"anavani-ascend".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();assert_eq!(engine.run.depth,1.);let t=fx.plane_transition.as_ref().expect("real Ascend transition");assert!(t.ascend);let view=observe::observe(&engine.run);let mut p=Painter::new();for age in [0.,125.,250.]{p.game_at(&view,"",false,0.);p.scene_crossfade(&t.before,age,true);capture(&p,&format!("ascend-column-{}",age as i32));}
}

#[test]fn action_animation_actual_construction_and_removal_captures(){
 for mode in [1,2] {let mut r=room();r.actors_at_mut(r.depth).clear();r.player.abilities=vec![corpse_core::revision50_terrain::COGWORKS.into()];r.player.ability_reserves=Field::Missing;corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,corpse_core::revision50_terrain::COGWORKS,20.,20.).unwrap();
  for (selected,label,expected)in [(mode,if mode==1{"pit-build"}else{"wall-build"},if mode==1{"pit-sink"}else{"slab"}),(3,if mode==1{"pit-remove"}else{"wall-remove"},if mode==1{"pit-rise"}else{"slab-sink"})] {assert!(corpse_core::revision50_terrain::select_mode(&mut r,selected as f64));let out=corpse_core::engine::dispatch(&mut r,&Command::Ability{id:corpse_core::revision50_terrain::COGWORKS.into(),target:Field::Missing,x:13.0.into(),y:10.0.into()}).unwrap();assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P61"&&a.style==expected)));let view=observe::observe(&r);let mut fx=effects::Effects::default();fx.add(out.events,r.tick,r.depth,0.,false);let mut p=Painter::new();for age in [145.,235.]{p.game_at(&view,"",false,0.);p.effects(&view,&fx.cues,age,false);capture(&p,&format!("{label}-{}",age as i32));}}
 }
}

#[test]fn brogue_projectile_moves_one_glyph_on_exact_route_without_a_trail(){
 let route=[(2.,3.),(3.,3.),(15.,11.)];
 let mut e=event("shared","bolt",Stage::Travel,0,&route);e.glyph=Some("↑".into());
 let cue=effects::Cue{event:e,started:100.,held:false,static_marks:false};
 for (time,point) in [(100.,(2,3)),(140.,(3,3)),(180.,(15,11))] {
  let tiles=effects::tiles(&cue,time,false);assert_eq!(tiles.len(),1);let t=&tiles[0];assert_eq!((t.x,t.y),point);assert_eq!(t.glyph.as_deref(),Some("↑"));assert!(t.mark.is_none());
 }
 assert!(effects::tiles(&cue,99.,false).is_empty());assert!(effects::tiles(&cue,220.,false).is_empty());
 for time in [100.,150.,200.] {let tiles=effects::tiles(&cue,time,true);assert_eq!(tiles.len(),1);assert_eq!((tiles[0].x,tiles[0].y),(15,11));assert!(tiles[0].glyph.is_none());}
}
#[test]fn brogue_area_and_proc_feedback_are_exact_cell_lights(){
 for (key,style,stage) in [("A12","wave",Stage::Travel),("E03","fan-orange",Stage::Travel),("P01","bracket",Stage::Special),("E70","collapse",Stage::Death),("P65","gear",Stage::Special)] {
  let mut e=event(key,style,stage,0,&[(2.,3.),(3.,3.),(15.,11.)]);e.glyph=Some("@".into());let cue=effects::Cue{event:e,started:0.,held:false,static_marks:false};
  for time in [0.,30.,80.] {for t in effects::tiles(&cue,time,false){assert!(t.glyph.is_none()&&t.mark.is_none());assert!([(2,3),(3,3),(15,11)].contains(&(t.x,t.y)));}}
 }
}
#[test]fn brogue_plain_melee_stays_quiet_but_ranged_impact_and_ability_proc_remain(){
 let mut fx=effects::Effects::default();let melee=||vec![event("shared","slash",Stage::Impact,0,&[(2.,3.)]),event("shared","puncture",Stage::Health,0,&[(2.,3.)])];
 fx.add(melee(),1.,1.,0.,false);assert!(fx.cues.is_empty());
 let mut proc=melee();proc.push(event("P01","bracket",Stage::Special,0,&[(2.,3.)]));fx.add(proc,2.,1.,0.,false);assert_eq!(fx.cues.len(),1);assert_eq!(fx.cues[0].event.animation.as_ref().unwrap().proposal,"P01");
 let mut ranged=melee();ranged.push(event("shared","bolt",Stage::Travel,0,&[(1.,3.),(2.,3.)]));fx.add(ranged,3.,1.,0.,false);assert_eq!(fx.cues.len(),3);assert_eq!(fx.cues[1].started,80.,"impact must not precede projectile arrival");
}
#[test]fn brogue_actual_ordinary_move_and_melee_have_no_cosmetic_flourish(){
 let mut r=room();r.player.abilities.clear();r.actors_at_mut(r.depth).clear();
 let out=corpse_core::engine::dispatch(&mut r,&Command::Move{dx:0.,dy:1.}).unwrap();let mut fx=effects::Effects::default();fx.add(out.events,r.tick,r.depth,0.,false);assert!(fx.cues.is_empty());
 let mut enemy=corpse_core::population::create_actor_revision("rat",123.,r.player.x+1.,r.player.y,false,52.).unwrap();enemy.hp=1000.;enemy.max_hp=1000.;enemy.abilities.clear();enemy.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(enemy);
 let out=corpse_core::engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();fx.add(out.events,r.tick,r.depth,0.,false);assert!(fx.cues.is_empty(),"ordinary melee must not gain hit/miss/health ornaments: {:?}",fx.cues.iter().map(|c|&c.event).collect::<Vec<_>>());
}
#[test]fn brogue_transition_never_repaints_outgoing_terrain_or_identity(){
 let r=room();let before=observe::observe(&r);let mut p=Painter::new();p.clear([9,10,11]);let pristine=p.pixels.clone();let cell=before.cells.iter().find(|c|c.actor_id==Some(before.player_id)).unwrap();let(cw,ch,ox,oy)=p.map_layout(before.width,before.height);let bounds=p.logical_damage_rect(ox+cell.x as i32*cw,oy+cell.y as i32*ch,cw,ch).unwrap();
 p.scene_crossfade(&before,100.,true);let mut changed=0;
 for(i,(a,b))in pristine.chunks_exact(4).zip(p.pixels.chunks_exact(4)).enumerate(){if a!=b{changed+=1;assert!(bounds.contains(i%p.width,i/p.width),"transition escaped the player tile");}}
 assert!(changed>0);
}

#[test]fn brogue_tile_renderer_matches_full_for_travel_proc_impact_and_expiry(){
 let r=room();let view=observe::observe(&r);
 for reduced in [false,true] {
  let mut fx=effects::Effects::default();fx.configure_motion(reduced,false);
  fx.add(vec![event("A01","bolt",Stage::Travel,0,&[(10.,10.),(11.,10.),(14.,10.)]),event("A01","spark",Stage::Impact,0,&[(14.,10.)]),event("P01","bracket",Stage::Special,0,&[(10.,10.)])],r.tick,r.depth,0.,false);
  let(mut tiled,mut full)=(Painter::new(),Painter::new());let mut plan=render::tile_damage::TilePlanner::default();let mut track=tile_overlay::Tracker::default();plan.replace_view(None,&view,&tiled);
  for now in [0.,30.,70.,110.,140.,180.,220.,280.,350.] {
   let state=tile_overlay::Frame{effects:tile_overlay::effects(&tiled,&view,&fx.cues,now,reduced),..Default::default()};
   for r in track.changes(&tiled,&state){plan.mark_native_rect(r);}tile_overlay::finish_dependencies(&mut tiled,&mut plan,&state);plan.paint_base(&mut tiled,&view,"",0.);if !tiled.tile_mask.is_empty(){tiled.effects(&view,&fx.cues,now,reduced);}tiled.tile_clip_enabled=false;
   full.game_at(&view,"",false,0.);full.effects(&view,&fx.cues,now,reduced);assert!(tiled.pixels==full.pixels,"tile/full mismatch reduced={reduced} t={now}");
   capture(&full,&format!("brogue-projectile-{}-{now}",if reduced{"reduced"}else{"normal"}));track.commit(state);plan.clear_frame();
  }
 }
}

#[test]fn brogue_ranged_impact_does_not_require_a_disclosed_travel_route(){
 let mut fx=effects::Effects::default();fx.add(vec![event("shared-ranged","slash",Stage::Impact,0,&[(10.,10.)]),event("shared-ranged","puncture",Stage::Health,0,&[(10.,10.)])],1.,1.,0.,false);assert_eq!(fx.cues.len(),2);
 for cue in &fx.cues{let tiles=effects::tiles(cue,cue.started+20.,false);assert_eq!(tiles.len(),1);assert!(tiles[0].glyph.is_none()&&tiles[0].mark.is_none());}
}

#[test]fn brogue_magic_advances_light_with_tapered_tail_without_a_glyph(){
 for key in ["A01","A02","A03","A05","E04","E08","E38","E47","E49"] {
  let e=event(key,"bolt",Stage::Travel,0,&[(2.,3.),(3.,3.),(15.,11.),(16.,11.),(17.,11.),(18.,11.)]);
  let cue=effects::Cue{event:e,started:0.,held:false,static_marks:false};
  let first=effects::tiles(&cue,0.,false);assert_eq!(first.len(),1);assert_eq!((first[0].x,first[0].y),(2,3));
  let second=effects::tiles(&cue,32.,false);assert_eq!(second.len(),2);assert!(second[1].light>second[0].light);
  for t in [0.,32.,64.,96.,128.,192.,240.] {let tiles=effects::tiles(&cue,t,false);assert!(tiles.iter().all(|p|p.glyph.is_none()&&p.mark.is_none()));assert!(tiles.len()<=4);}
  assert!(effects::tiles(&cue,128.,false).iter().all(|p|(p.x,p.y)!=(2,3)),"tail must retract");
  assert!(effects::tiles(&cue,320.,false).is_empty());
 }
}
#[test]fn brogue_long_routes_keep_per_cell_time_and_impacts_wait_for_arrival(){
 for (key,step) in [("shared",40.),("A02",32.)] {
  let route:Vec<_>=(0..30).map(|x|(x as f64,3.)).collect();let mut fx=effects::Effects::default();
  fx.add(vec![event(key,"bolt",Stage::Travel,0,&route),event(key,"spark",Stage::Impact,0,&route[29..])],1.,1.,0.,false);
  assert_eq!(fx.cues[1].started,effects::duration(&fx.cues[0].event));
  for i in [0,1,5,20,29] {let tiles=effects::tiles(&fx.cues[0],i as f64*step,false);assert_eq!(tiles.last().map(|p|p.x),Some(i));}
  assert!(fx.animate(500.));assert!(!fx.animate(fx.cues[1].started+effects::duration(&fx.cues[1].event)));
 }
}
#[test]fn brogue_dispatch_time_never_consumes_unpainted_travel(){
 let mut fx=effects::Effects::default();fx.add(vec![event("A02","ember",Stage::Travel,0,&[(1.,3.),(2.,3.),(3.,3.)])],1.,1.,10.,false);
 fx.defer_until_frame(10.);fx.begin_frame(5000.);fx.prune(5000.);assert_eq!(fx.cues.len(),1);assert_eq!(fx.cues[0].started,5000.);
 assert_eq!(effects::tiles(&fx.cues[0],5000.,false)[0].x,1);fx.begin_frame(5100.);assert_eq!(fx.cues[0].started,5000.);
 fx.clear();fx.begin_frame(6000.);assert!(fx.cues.is_empty());
}

#[test]fn brogue_actual_arrows_and_thrown_bottles_keep_visible_glyph_flight(){
 for bottle in [false,true] {
  let mut r=room();r.player.abilities.clear();r.actors_at_mut(r.depth).clear();
  let command=if bottle {
   r.inventory.push(corpse_core::Item{id:900.,kind:"potion".into(),name:"Potion of Healing".into(),glyph:"!".into(),effect:"healing".to_string().into(),identified:true.into(),..Default::default()});
   Command::Throw{id:900.,x:20.,y:10.}
  } else {
   r.inventory.push(corpse_core::Item{id:900.,name:"Bow".into(),kind:"weapon".into(),damage:15.0.into(),ranged_weapon:"bow".to_string().into(),hands:2.0.into(),equipped:"hand1".to_string().into(),..Default::default()});
   r.inventory.push(corpse_core::Item{id:901.,name:"Arrow".into(),kind:"ammo".into(),effect:"arrows".to_string().into(),quantity:5.0.into(),..Default::default()});
   Command::Shoot{id:900.,x:20.,y:10.}
  };
  let mut engine=temporal::Engine::new(r).unwrap();let mut fx=effects::Effects::default();dispatch(&mut engine,&mut fx,0.,&command).unwrap();fx.begin_frame(0.);
  let cue=fx.cues.iter().find(|c|c.event.animation.as_ref().is_some_and(|a|a.delivery==Delivery::Projectile)).expect("physical flight receipt");
  assert_eq!(cue.event.glyph.as_deref(),Some(if bottle{"!"}else{"→"}));let a=cue.event.animation.as_ref().unwrap();assert!(a.cells.len()>5);
  let view=observe::observe(&engine.run);let mut painter=Painter::new();
  for i in [0,2,4,6] {let time=cue.started+i as f64*effects::PROJECTILE_CELL_MS;let tiles=effects::tiles(cue,time,false);assert_eq!(tiles.len(),1);assert_eq!(tiles[0].glyph,cue.event.glyph);
   fx.firebolt_terrain.prepare(&view,&fx.cues,time,false,&mut painter.firebolt_terrain);painter.game_at(&view,"",false,0.);painter.effects(&view,&fx.cues,time,false);capture(&painter,&format!("physical-{}-{i}",if bottle{"bottle"}else{"arrow"}));
  }
 }
}

#[test]
fn magic_brightness_preserves_routes_fades_and_reduced_motion() {
 let route=[(10.,10.),(11.,10.),(12.,10.)];
 for (key,style,strength) in [("A01","bolt",0.90),("A02","ember",0.62),("A03","dart",0.62),("A05","glob",0.62),("shared","bolt",0.32)] {
  let cue=effects::Cue{event:event(key,style,Stage::Travel,0,&route),started:0.,held:false,static_marks:false};
  for age in [0.,16.,32.,64.] {
   for t in effects::tiles(&cue,age,false) {
    assert_eq!(t.field_strength,strength);
    assert!(route.contains(&(t.x as f64,t.y as f64)));
    assert!(t.light>=0. && t.light<=0.95);
    assert_eq!(t.glyph.is_some(),key=="shared");
    assert!(t.mark.is_none());
   }
  }
  let tiles=effects::tiles(&cue,20.,true);assert_eq!(tiles.len(),1);
  assert_eq!((tiles[0].x,tiles[0].y),(12,10));
  assert_eq!(tiles[0].light,0.55);assert_eq!(tiles[0].field_strength,strength);
  assert!(tiles[0].glyph.is_none());
  assert!(effects::tiles(&cue,1000.,false).is_empty());
 }
}

#[test]
fn sequencing_descent_travel_flash_then_ground_changes() {
 let mut r=room(); r.player.abilities.clear(); r.actors_at_mut(r.depth).clear();
 r.inventory.push(corpse_core::Item{id:900.,kind:"potion".into(),name:"Potion of Descent".into(),glyph:"!".into(),effect:"descent".to_string().into(),identified:true.into(),..Default::default()});
 let mut engine=temporal::Engine::new(r).unwrap(); let mut fx=effects::Effects::default();
 let time=std::time::Instant::now();
 dispatch(&mut engine,&mut fx,0.,&Command::Throw{id:900.,x:20.,y:10.}).unwrap();
 eprintln!("sequenced descent dispatch {:?}",time.elapsed());
 fx.begin_frame(0.);
 let travel=fx.cues.iter().find(|c|c.event.animation.as_ref().is_some_and(|a|a.stage==Stage::Travel&&a.delivery==Delivery::Projectile)).unwrap();
 let action=travel.event.animation.as_ref().unwrap().action;
 let end=fx.cues.iter().filter(|c|c.event.animation.as_ref().is_some_and(|a|a.action==action)).map(|c|c.started+effects::duration(&c.event)).fold(0f64,f64::max);
 let flash_time=fx.cues.iter().find(|c|c.event.animation.as_ref().is_some_and(|a|a.action==action&&a.stage!=Stage::Travel)).map_or(end-1.,|c|c.started+18.);
 let tile=|v:&View|v.terrain_snapshot.as_ref().unwrap().cells[10*v.width+20].tile.clone();
 let final_view=observe::observe(&engine.run);
 assert_ne!(tile(&final_view).as_deref(),Some("floor"));
 for (now,label) in [(0.,"before"),(travel.started+80.,"travel"),(flash_time,"flash")] {
  let v=fx.sequence.view(now).expect("hold before impact ends"); assert_eq!(tile(v).as_deref(),Some("floor"));
  let mut p=Painter::new();p.game_at(v,"",false,0.);p.effects(v,&fx.cues,now,false);capture(&p,&format!("sequence-descent-{label}"));
 }
 let v=fx.sequence.view(end).unwrap_or(&final_view);assert_ne!(tile(v).as_deref(),Some("floor"));
 let mut p=Painter::new();p.game_at(v,"",false,0.);capture(&p,"sequence-descent-after");
}
#[test]
fn sequencing_actions_do_not_overlap() {
 let mut fx=effects::Effects::default();
 fx.add(vec![event("A01","bolt",Stage::Travel,1,&[(1.,1.),(2.,1.)]),event("A01","stitch",Stage::Health,1,&[(2.,1.)]),event("E18","bolt",Stage::Travel,2,&[(2.,1.),(1.,1.)])],1.,1.,0.,false);
 let first_end=fx.cues.iter().filter(|c|c.event.animation.as_ref().unwrap().action==1).map(|c|c.started+effects::duration(&c.event)).fold(0f64,f64::max);
 let next=fx.cues.iter().find(|c|c.event.animation.as_ref().unwrap().action==2).unwrap();assert!(next.started>=first_end);
}
#[test]
fn sequencing_intermediate_health_and_visibility_are_not_future_state() {
 let mut r=room(); r.player.hp=100.; let before=observe::observe(&r);
 r.player.hp=80.; let middle=observe::observe(&r);
 r.player.hp=50.; let after=observe::observe(&r);
 let mut fx=effects::Effects::default();
 fx.add(vec![event("P01","stitch",Stage::Special,1,&[(10.,10.)]),event("E18","bolt",Stage::Special,2,&[(10.,10.)])],1.,1.,0.,false);
 let end1=fx.cues[0].started+effects::duration(&fx.cues[0].event);
 let end2=fx.cues[1].started+effects::duration(&fx.cues[1].event);
 fx.sequence.capture(before,vec![corpse_core::events::PresentationFrame{action:1,observation:std::rc::Rc::new(middle)},corpse_core::events::PresentationFrame{action:2,observation:std::rc::Rc::new(after)}],&fx.cues,appearance_profile::Profile::defaults());
 assert_eq!(fx.sequence.view(0.).unwrap().hp,100.);
 assert_eq!(fx.sequence.view(end1).unwrap().hp,80.);
 assert_eq!(fx.sequence.view(end2-1.).unwrap().hp,80.);
 assert!(fx.sequence.view(end2).is_none());
 fx.defer_until_frame(0.);assert!(fx.presentation_pending(10000.));
 fx.begin_frame(10000.);assert_eq!(fx.sequence.view(10000.).unwrap().hp,100.);
 assert_eq!(fx.sequence.view(10000.+end1).unwrap().hp,80.);
 fx.clear();assert!(!fx.presentation_pending(0.));
}
#[test]
fn sequencing_observer_does_not_change_simulation_and_reports_cost() {
 let mut r=room();r.player.abilities.clear();
 for i in 0..20 {let mut a=corpse_core::population::create_actor_revision("rat",200.+i as f64,20.+(i%10) as f64,12.+(i/10) as f64,false,52.).unwrap();a.hp=100.;r.actors_at_mut(r.depth).push(a);}
 let mut expected=r.clone();let t=std::time::Instant::now();corpse_core::engine::dispatch(&mut expected,&Command::Wait).unwrap();let plain=t.elapsed();
 let mut engine=temporal::Engine::new(r).unwrap();let mut fx=effects::Effects::default();let t=std::time::Instant::now();dispatch(&mut engine,&mut fx,0.,&Command::Wait).unwrap();
 eprintln!("dense room: core {:?}; observed {:?}",plain,t.elapsed());
 assert_eq!(engine.run.player.hp,expected.player.hp);assert_eq!(engine.run.rng,expected.rng);assert_eq!(engine.run.tick,expected.tick);
 assert_eq!(serde_json::to_value(&engine.run.actors).unwrap(),serde_json::to_value(&expected.actors).unwrap());
}
