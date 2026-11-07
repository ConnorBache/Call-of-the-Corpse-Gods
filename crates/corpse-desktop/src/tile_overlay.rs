//! Fixed overlay footprints for tile painting. No retained world/Cell Scene, tree,
//! framebuffer copy or rectangle bounding-box coalescer is constructed here.
use crate::{render::{Painter,View,damage::Rect},pack::PackPaintState};
#[derive(Clone,PartialEq)]
pub struct Hover{pub report:crate::inspection_ui::Report,pub tile:(i32,i32,i32,i32),pub bounds:Rect}
#[derive(Clone,PartialEq)]
pub struct Effect{pub x:i32,pub y:i32,pub color:[u8;3],pub light:u64,pub glyph:Option<String>,pub bounds:Rect}
#[derive(Clone,PartialEq)]
pub struct Shutter{pub bounds:Rect,pub offset:usize}
#[derive(Default)]
pub struct Frame{pub pack:Option<PackPaintState>,pub hover:Option<Hover>,pub effects:Vec<Effect>,pub shutter:Option<Shutter>,pub full_modal:bool,pub panels:Vec<Rect>,pub redraw_panels:bool,pub choice:usize,pub inspector:Option<(Option<u64>,usize)>,pub save_error:Option<String>}
#[derive(Default)]
pub struct Tracker{previous:Option<Frame>}
fn logical(p:&Painter,r:crate::modal_ui::Rect)->Option<Rect>{p.logical_damage_rect(r.0,r.1,r.2,r.3)}
fn banner(p:&Painter)->Option<Rect>{p.logical_damage_rect(8,8,1264,76)}
impl Tracker{
 pub fn invalidate(&mut self){self.previous=None;}
 /// O(pack hover text + emitted effect descriptors); returned regions retain
 /// their separate geometry. The tile grid/span union performs no box collapse.
 pub fn changes(&self,p:&Painter,next:&Frame)->Vec<Rect>{
  let Some(old)=&self.previous else{return vec![Rect::new(0,0,p.width,p.height)];};
  let mut out=Vec::new();
  if old.full_modal!=next.full_modal||((old.full_modal||next.full_modal)&&(next.redraw_panels||old.choice!=next.choice)){out.push(Rect::new(0,0,p.width,p.height));}
  if old.panels!=next.panels||next.redraw_panels||old.inspector!=next.inspector{out.extend(old.panels.iter().chain(&next.panels).copied());}
  match(&old.pack,&next.pack){(Some(a),Some(b))=>out.extend(b.changes(a).into_iter().filter_map(|r|logical(p,r))),
   (a,b)=>{out.extend(a.iter().chain(b.iter()).filter_map(|v|logical(p,v.bounds)));}}
  if old.hover!=next.hover{out.extend(old.hover.iter().chain(next.hover.iter()).map(|v|v.bounds));}
  if old.effects!=next.effects{out.extend(old.effects.iter().chain(&next.effects).map(|v|v.bounds));}
  if old.shutter!=next.shutter{out.extend(old.shutter.iter().chain(next.shutter.iter()).map(|v|v.bounds));}
  if old.save_error!=next.save_error{out.extend(banner(p));}out
 }
 pub fn commit(&mut self,next:Frame){self.previous=Some(next);}
}
impl Frame{
 /// Complete translucent text footprints, expanded before resetting the backdrop.
 /// Pack is conservatively one panel for underlying map changes; pointer-only
 /// changes still originate from precise old/new row/button/tooltip regions.
 pub fn dependencies(&self,p:&Painter)->Vec<Rect>{
  let mut out=Vec::new();if let Some(s)=&self.pack{out.extend(logical(p,s.bounds));}
  out.extend(self.hover.iter().map(|s|s.bounds));out.extend(self.effects.iter().map(|s|s.bounds));
  out.extend(self.shutter.iter().map(|s|s.bounds));if self.save_error.is_some(){out.extend(banner(p));}out
 }
}
/// Exact emitted effect tiles, using a fixed-coordinate table instead of a tree.
/// O(N+C+E), where N map cells, C observed layers, E emitted effect tiles.
pub fn effects(p:&Painter,v:&View,cues:&[crate::effects::Cue],now:f64,reduced:bool)->Vec<Effect>{
 if cues.is_empty(){return Vec::new();}
 let n=v.width*v.height;let mut known=vec![false;n];let mut actors=vec![None;n];
 for c in &v.cells{if c.x<v.width&&c.y<v.height{let at=c.y*v.width+c.x;known[at]|=c.current;if c.actor_id.is_some(){actors[at]=Some(c);}}}
 let mut tiles:Vec<Option<crate::effects::Tile>>=(0..n).map(|_|None).collect();
 for cue in cues.iter().filter(|c|c.event.depth==v.depth){for t in crate::effects::tiles(cue,now,reduced){
  if t.x<0||t.y<0||t.x>=v.width as i32||t.y>=v.height as i32{continue;}let at=t.y as usize*v.width+t.x as usize;
  if known[at]&&tiles[at].as_ref().is_none_or(|old|t.light>=old.light){tiles[at]=Some(t);}
 }}
 let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
 tiles.into_iter().enumerate().filter_map(|(i,t)|{let t=t?;let pad=cw.max(ch).max(4).saturating_mul(t.glyph.as_ref().map_or(1,|g|g.chars().count().max(1)as i32));
 p.logical_damage_rect(ox+t.x*cw-pad,oy+t.y*ch-pad,cw+2*pad,ch+2*pad).map(|mut bounds|{if let Some(c)=actors[i]{if let Some(r)=p.cell_damage_rect(c,v.width,v.height){bounds=bounds.union(r);}}Effect{x:t.x,y:t.y,color:t.color,light:t.light.to_bits(),glyph:t.glyph,bounds}})
 }).collect()
}
pub fn hover(p:&Painter,report:&crate::inspection_ui::Report,tile:(i32,i32,i32,i32))->Option<Hover>{
 if !crate::inspection_ui::hover_available(report){return None;}let r=crate::modal_ui::hover_bounds(&p.profile,tile,i32::MAX/4);
 p.logical_damage_rect(r.0,8,r.2,crate::render::HEIGHT as i32-16).map(|bounds|Hover{report:report.clone(),tile,bounds})
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn unchanged_empty_overlay_has_no_marks(){let p=Painter::new();let mut t=Tracker::default();t.commit(Frame::default());assert!(t.changes(&p,&Frame::default()).is_empty());}
 #[test]fn shutter_and_banner_close_marks_old_background(){let p=Painter::new();let mut t=Tracker::default();let r=Rect::new(8,20,300,180);t.commit(Frame{shutter:Some(Shutter{bounds:r,offset:0}),save_error:Some("failure".into()),..Default::default()});let d=t.changes(&p,&Frame::default());assert!(d.contains(&r));assert!(d.contains(&banner(&p).unwrap()));}
}
/// Bounded panel keys are presentation state, not a cloned world Scene.
#[derive(Clone,Copy,PartialEq,Eq)]
pub struct PanelKeys{toolbar:Option<usize>,bestiary:bool,enemy:Option<u64>,ability:Option<usize>,chronicle:bool,enemy_offset:usize}
impl PanelKeys{
 pub fn read(p:&Painter)->Self{Self{toolbar:p.toolbar_focus,bestiary:p.bestiary_unread,enemy:p.enemy_hover,ability:p.ability_hover,chronicle:p.chronicle_hover,enemy_offset:p.enemy_list_offset}}
 pub fn mark_changes(self,old:Option<Self>,p:&Painter,v:&View,plan:&mut crate::render::tile_damage::TilePlanner){
  let Some(old)=old else{plan.mark_native_rect(Rect::new(0,0,p.width,p.height));return;};let left=p.hud_left();let h=crate::render::chronicle_height(&p.profile);
  let mut mark=|x,y,w,hh|{if let Some(r)=p.logical_damage_rect(x,y,w,hh){plan.mark_native_rect(r);}};
  if self.toolbar!=old.toolbar||self.bestiary!=old.bestiary{let y=p.toolbar_y()-24;mark(0,y,p.logical_width(),p.logical_height()-y);}
  if self.enemy!=old.enemy||self.enemy_offset!=old.enemy_offset{let(_,_,ox,_)=p.map_layout(v.width,v.height);mark(0,h,ox,p.logical_height()-h);}
  if self.ability!=old.ability{mark(left,0,p.logical_width()-left,p.logical_height());}
  if self.chronicle!=old.chronicle{mark(0,0,left,h+20);}
  if self.enemy!=old.enemy {
   for id in [old.enemy,self.enemy] {if let Some(b)=crate::perception_ui::selected(v,id) {for r in crate::perception_ui::bounds(p,v,b){plan.mark_native_rect(r);}}}
   for cell in &v.cells {if cell.actor_id.is_some_and(|id|[old.enemy,self.enemy].contains(&Some(id.to_bits()))) {
    if let Some(rect)=p.cell_damage_rect(cell,v.width,v.height){plan.mark_native_rect(rect);}
   }}
  }
 }
}
/// Mark old/new in-place path/selection inputs before/after mutation. O(path+C),
/// no Run query, observation, pathfinding or copied cells.
pub fn mark_presentation(p:&Painter,v:&View,plan:&mut crate::render::tile_damage::TilePlanner){
 let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
 for &(x,y) in v.path.iter().chain(&v.target_path).chain(&v.target_area).chain(v.target_impact.iter()).chain(&v.possible_exits).chain(v.hover.iter()){
  if let Some(r)=p.logical_damage_rect(ox+x as i32*cw-cw,oy+y as i32*ch-ch,3*cw,3*ch){plan.mark_native_rect(r);}
 }
 if let Some(id)=v.selected_actor{for c in &v.cells{if c.actor_id==Some(id){if let Some(r)=p.cell_damage_rect(c,v.width,v.height){plan.mark_native_rect(r);}}}}
}
/// Install exact row union, expanding whole translucent text footprints BEFORE
/// resetting their backdrop. Effects are marked together when any damage exists;
/// fixed panel dependency closure has a bounded number of entries, not E² effect scans.
pub fn finish_dependencies(p:&mut Painter,plan:&mut crate::render::tile_damage::TilePlanner,state:&Frame){
 plan.finish_damage(p);if p.tile_mask.is_empty(){return;}
 for e in &state.effects{plan.mark_native_rect(e.bounds);}
 plan.finish_damage(p);
 let mut deps=state.panels.clone();if state.full_modal{deps.push(Rect::new(0,0,p.width,p.height));}if let Some(v)=&state.pack{deps.extend(logical(p,v.bounds));}deps.extend(state.hover.iter().map(|v|v.bounds));deps.extend(state.shutter.iter().map(|v|v.bounds));if state.save_error.is_some(){deps.extend(banner(p));}
 let mut added=vec![false;deps.len()];
 loop{let mut any=false;for(i,r)in deps.iter().copied().enumerate(){if !added[i]&&p.tile_mask.intersects(r){plan.mark_native_rect(r);added[i]=true;any=true;}}
  if !any{break;}plan.finish_damage(p);
 }
}
/// Borrowed native pixels with full pitch: no prefix copy. Bounds checked before SDL.
pub fn upload_source(p:&Painter,r:Rect)->Result<(&[u8],usize),String>{
 if r.w==0||r.h==0||r.right()>p.width||r.bottom()>p.height{return Err("invalid tile upload bounds".into());}
 let pitch=p.width.checked_mul(4).ok_or("tile upload pitch overflow")?;
 let start=r.y.checked_mul(pitch).and_then(|v|v.checked_add(r.x.checked_mul(4)?)).ok_or("tile upload offset overflow")?;
 let len=(r.h-1).checked_mul(pitch).and_then(|v|v.checked_add(r.w.checked_mul(4)?)).ok_or("tile upload length overflow")?;
 let bytes=p.pixels.get(start..start.checked_add(len).ok_or("tile upload end overflow")?).ok_or("tile upload missing pixels")?;Ok((bytes,pitch))
}
#[cfg(test)]mod pixel_tests{
 use super::*;
 fn pixels(a:&Painter,b:&Painter,label:&str){if let Some(i)=a.pixels.iter().zip(&b.pixels).position(|(x,y)|x!=y){panic!("{label}: first differing byte {i}");}assert_eq!(a.pixels.len(),b.pixels.len());}
 #[test]fn tile_pack_hover_close_and_in_place_path_match_full(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let mut view=crate::observe::observe(&run);
  let mut p=Painter::new();let mut full=Painter::new();let mut plan=crate::render::tile_damage::TilePlanner::default();let mut track=Tracker::default();
  let mut pack=crate::pack::PackView::observe(&run);pack.configure(&p,crate::modal_ui::Frame::from_profile(&p.profile).map.tuple());
  plan.replace_view(None,&view,&p);
  for step in 0..7{
   if step==1||step==5{mark_presentation(&p,&view,&mut plan);view.path=if step==1{view.cells.iter().take(3).map(|c|(c.x,c.y)).collect()}else{Vec::new()};view.hover=view.path.last().copied();mark_presentation(&p,&view,&mut plan);}
   let open=(2..=4).contains(&step);if open{if let Some((x,y))=pack.retained_test_hover_point(0){pack.hover(x+step*5,y);}}
   let state=Frame{pack:open.then(||pack.paint_state(&p)),..Default::default()};for r in track.changes(&p,&state){plan.mark_native_rect(r);}finish_dependencies(&mut p,&mut plan,&state);plan.paint_base(&mut p,&view,"",0.);
   if !p.tile_mask.is_empty(){if let Some(s)=&state.pack{pack.paint_prepared(&mut p,s);}}
   p.tile_clip_enabled=false;full.game_at(&view,"",false,0.);if open{pack.paint(&mut full);}pixels(&p,&full,&format!("pack step {step}"));track.commit(state);plan.clear_frame();
  }
 }
 #[test]fn r50_item_hover_tile_open_change_close_matches_full_paint(){
  let mut run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  run.simulation_revision=50.0.into();run.inventory.clear();run.actors=Default::default();run.ground.clear();
  run.player.abilities.clear();run.player.active.clear();run.player.conditions.clear();run.player.x=10.;run.player.y=10.;
  let f=run.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.doors=corpse_core::Field::Missing;f.portals=corpse_core::Field::Missing;f.environment=corpse_core::Field::Missing;
  let item=corpse_core::Item{id:900.,kind:"potion".into(),name:"Unidentified potion".into(),effect:"freezing".to_string().into(),identified:false.into(),..Default::default()};
  run.ground_at_mut(run.depth).push(corpse_core::GroundItem{x:11.,y:10.,item,..Default::default()});
  let before=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);let report=crate::inspection_ui::observe_tile(&run,11,10);
  assert!(report.creature.is_none());assert!(crate::inspection_ui::hover_available(&report));
  let mut p=Painter::new();let mut full=Painter::new();let mut plan=crate::render::tile_damage::TilePlanner::default();let mut track=Tracker::default();
  plan.replace_view(None,&view,&p);let(cw,ch,ox,oy)=p.map_layout(view.width,view.height);let tile=(ox+11*cw,oy+10*ch,ox+12*cw,oy+11*ch);
  assert!(crate::overlay_damage::hover(&p,&report,tile).is_some(),"retained admission matches tile admission");
  for step in 0..7 {
   let open=matches!(step,1|2|3|5);let mut shown=report.clone();if step==3 {shown.item_hover=vec!["Potion of Energy\nRestores 50 energy, up to maximum energy.".into()];}
   let state=Frame{hover:if open{hover(&p,&shown,tile)}else{None},..Default::default()};
   for rect in track.changes(&p,&state){plan.mark_native_rect(rect);}finish_dependencies(&mut p,&mut plan,&state);plan.paint_base(&mut p,&view,"",0.);
   if !p.tile_mask.is_empty(){if let Some(h)=&state.hover{crate::inspection_ui::paint_hover_at(&h.report,&mut p,h.tile);}}
   p.tile_clip_enabled=false;full.game_at(&view,"",false,0.);if open{crate::inspection_ui::paint_hover_at(&shown,&mut full,tile);}
   pixels(&p,&full,&format!("item hover step {step}"));assert_eq!(p.clip,None);track.commit(state);plan.clear_frame();
  }
  assert_eq!(serde_json::to_vec(&run).unwrap(),before);
 }
 #[test]fn tile_shutter_and_banner_retract_restore_exact_backdrop(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let view=crate::observe::observe(&run);let mut p=Painter::new();let mut full=Painter::new();let mut plan=crate::render::tile_damage::TilePlanner::default();let mut track=Tracker::default();plan.replace_view(None,&view,&p);
  let mut shutter=crate::chronicle_overlay::Shutter::default();let lines:Vec<_>=(0..60).map(|i|format!("Chronicle line {i}: unchanged world behind moving panel.")).collect();
  for(step,now)in [0.,80.,160.,260.,340.,450.,600.].into_iter().enumerate(){if step==1{shutter.open(now,false);}if step==4{shutter.close(now,false);}let show=step>0&&step<6;let error=(step==2||step==3).then(||"Bounded test error".to_string());
   let state=Frame{shutter:show.then(||crate::chronicle_overlay::bounds(&shutter,now,false)).flatten().and_then(|(x,y,w,h)|p.logical_damage_rect(x,y,w,h)).map(|bounds|Shutter{bounds,offset:shutter.offset}),save_error:error,..Default::default()};for r in track.changes(&p,&state){plan.mark_native_rect(r);}finish_dependencies(&mut p,&mut plan,&state);plan.paint_base(&mut p,&view,"",0.);
   if !p.tile_mask.is_empty(){if show{crate::chronicle_overlay::paint(&mut p,&lines,&shutter,now,false);}if let Some(e)=&state.save_error{crate::paint_save_failure(&mut p,e);}}
   p.tile_clip_enabled=false;full.game_at(&view,"",false,0.);if show{crate::chronicle_overlay::paint(&mut full,&lines,&shutter,now,false);}if let Some(e)=&state.save_error{crate::paint_save_failure(&mut full,e);}pixels(&p,&full,&format!("shutter step {step}"));track.commit(state);plan.clear_frame();
  }
 }
 #[test]fn native_upload_is_borrowed_and_checks_last_row(){let mut p=Painter::new();p.resize(31,19);let r=Rect::new(30,18,1,1);let(b,pitch)=upload_source(&p,r).unwrap();assert_eq!(pitch,31*4);assert_eq!(b.as_ptr(),p.pixels[(18*31+30)*4..].as_ptr());assert_eq!(b.len(),4);assert!(upload_source(&p,Rect::new(31,18,1,1)).is_err());}
}
/// Bound driver-call fragmentation without enlarging CPU paint damage. The 128
/// call ceiling is a prototype policy, not a measured universal GPU optimum.
pub struct UploadPlan{pub full:bool,pub rectangles:Vec<Rect>,pub bytes:usize,pub calls:usize,pub requested_calls:usize,pub requested_bytes:usize}
pub fn upload_plan(p:&Painter,force_full:bool)->UploadPlan{
 let mut rectangles=if force_full{Vec::new()}else{p.tile_mask.upload_rectangles()};
 let requested_calls=if force_full{1}else{rectangles.len()};let requested_bytes=if force_full{p.width*p.height*4}else{rectangles.iter().map(|r|r.w*r.h*4).sum()};
 let full=force_full||rectangles.len()>128;
 if full{rectangles.clear();UploadPlan{full,rectangles,bytes:p.width*p.height*4,calls:1,requested_calls,requested_bytes}}
 else{let bytes=rectangles.iter().map(|r|r.w*r.h*4).sum();let calls=rectangles.len();UploadPlan{full,rectangles,bytes,calls,requested_calls,requested_bytes}}
}
#[cfg(test)]mod upload_policy_tests{
 use super::*;
 #[test]fn fragmented_upload_fallback_does_not_expand_paint_mask(){let mut p=Painter::new();p.tile_mask.reset(p.width,p.height);for y in (0..260).step_by(2){p.tile_mask.add(Rect::new(0,y,1,1));}p.tile_mask.finish();let before=p.tile_mask.area();let upload=upload_plan(&p,false);assert!(upload.full);assert_eq!(upload.calls,1);assert_eq!(upload.bytes,p.width*p.height*4);assert_eq!(p.tile_mask.area(),before);assert_eq!(before,130);}
}
#[cfg(test)]mod modal_footprint_tests{
 use super::*;
 #[test]fn stable_bounded_modal_does_not_mark_unrelated_frame(){let p=Painter::new();let r=Rect::new(337,125,636,579);let mut t=Tracker::default();t.commit(Frame{panels:vec![r],..Default::default()});let stable=Frame{panels:vec![r],..Default::default()};assert!(t.changes(&p,&stable).is_empty());let changed=Frame{panels:vec![r],redraw_panels:true,..Default::default()};assert!(t.changes(&p,&changed).iter().all(|a|*a==r));let closed=t.changes(&p,&Frame::default());assert_eq!(closed,vec![r]);}
}
#[cfg(test)]mod sdl_upload_tests{
 use super::*;
 #[test]
 #[ignore="lead-controlled standalone SDL software readback; not GPU/Windows timing"]
 fn tile_upload_plan_sparse_last_row_and_expose_readback(){
  let _sdl=sdl2::init().unwrap();let mut p=Painter::new();p.resize(31,19);
  for(y,row)in p.pixels.chunks_mut(31*4).enumerate(){for(x,c)in row.chunks_mut(4).enumerate(){c.copy_from_slice(&[(x*7)as u8,(y*11)as u8,77,255]);}}
  let surface=sdl2::surface::Surface::new(31,19,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();let mut canvas=surface.into_canvas().unwrap();let creator=canvas.texture_creator();let mut texture=creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32,31,19).unwrap();
  texture.update(None,&p.pixels,p.width*4).unwrap();
  for regions in [vec![Rect::new(30,18,1,1)],vec![Rect::new(0,2,3,4),Rect::new(20,2,4,4)],Vec::new()]{
   p.tile_mask.reset(p.width,p.height);for r in regions{p.tile_mask.add(r);for y in r.y..r.bottom(){for x in r.x..r.right(){let at=(y*p.width+x)*4;p.pixels[at..at+4].copy_from_slice(&[180,22,99,255]);}}}p.tile_mask.finish();
   let plan=upload_plan(&p,false);if p.tile_mask.is_empty(){assert_eq!(plan.calls,0);assert_eq!(plan.bytes,0);}
   if plan.full{texture.update(None,&p.pixels,p.width*4).unwrap();}else{for r in plan.rectangles{let(bytes,pitch)=upload_source(&p,r).unwrap();texture.update(Some(sdl2::rect::Rect::new(r.x as i32,r.y as i32,r.w as u32,r.h as u32)),bytes,pitch).unwrap();}}
   // The empty case is expose: existing texture is presented without paint/upload.
   canvas.clear();canvas.copy(&texture,None,None).unwrap();assert_eq!(canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap(),p.pixels);
  }
 }
}
#[cfg(test)]mod inspector_selection_tests{
 use super::*;
 #[test]fn two_bounded_inspector_selections_repaint_even_without_world_damage(){
  let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let mut v=crate::observe::observe(&r);
  v.creatures=(1..=2).map(|id|crate::render::Creature{perception_range:None,identity_outline:None,hallucination:None,shared_resources:false,invulnerable:false,id:id as f64,name:format!("Test creature {id}"),glyph:"b".into(),color:[220,180,30],state:"hunting".into(),distance:4.,hp:20.,max_hp:20.,energy:10.,max_energy:10.,statuses:Vec::new(),meters:Vec::new()}).collect();
  let mut p=Painter::new();let mut full=Painter::new();let mut plan=crate::render::tile_damage::TilePlanner::default();let mut track=Tracker::default();plan.replace_view(None,&v,&p);
  for id in [1.,2.,2.]{let panel=p.logical_damage_rect(333,121,644,587).unwrap();let state=Frame{panels:vec![panel],inspector:Some((Some(f64::to_bits(id)),0)),..Default::default()};let marks=track.changes(&p,&state);for rect in marks{plan.mark_native_rect(rect);}finish_dependencies(&mut p,&mut plan,&state);plan.paint_base(&mut p,&v,"",0.);if !p.tile_mask.is_empty(){p.creatures(&v,Some(id),0);}p.tile_clip_enabled=false;
   full.game_at(&v,"",false,0.);full.creatures(&v,Some(id),0);if let Some(i)=p.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b){panic!("inspector{id}: differing byte{i}");}track.commit(state);plan.clear_frame();}
 }
}

#[cfg(test)]mod initial_panel_index_tests{
 use super::*;
 #[test]fn first_panel_paint_keeps_borrowed_view_index_valid_for_next_command(){
  let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=crate::observe::observe(&r);assert!(!v.cells.is_empty());
  let mut p=Painter::new();let mut plan=crate::render::tile_damage::TilePlanner::default();plan.eligibility.enabled=true;
  plan.replace_view(None,&v,&p);let keys=PanelKeys::read(&p);keys.mark_changes(None,&p,&v,&mut plan);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
  let before=plan.eligibility.compared_layers;plan.replace_view(Some(&v),&v,&p);keys.mark_changes(Some(keys),&p,&v,&mut plan);plan.finish_damage(&mut p);
  assert!(plan.eligibility.compared_layers>before,"next command must compare existing indexed layers");assert!(p.tile_mask.is_empty(),"unchanged second frame should not be forced full by panel initialization");
 }
}
