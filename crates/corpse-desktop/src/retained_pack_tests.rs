#[cfg(test)]mod retained_pack_tests {
 use super::*;
 use crate::{overlay_damage,render::damage::Damage};
 fn draw(p:&mut Painter,base:&mut crate::render::RetainedGame,tracker:&mut overlay_damage::Tracker,v:&crate::render::View,pack:Option<&PackView>,changed:bool)->Damage{
  let state=overlay_damage::Frame{pack:pack.map(|q|q.paint_state(p)),..Default::default()};
  let mut d=base.update(p,v,"",0.,changed,false).unwrap();d.extend(&tracker.changes(p,&state));state.expand_dependencies(p,&mut d);base.restore(p,&d).unwrap();
  for r in d.rects(){p.damage_clip=Some(*r);if state.pack_intersects(p,*r){pack.unwrap().paint_prepared(p,state.pack.as_ref().unwrap());}}
  p.damage_clip=None;tracker.commit(state);d
 }
 fn oracle(p:&mut Painter,v:&crate::render::View,pack:Option<&PackView>){p.damage_clip=None;p.game_at(v,"",false,0.);if let Some(q)=pack{q.paint(p);}}
 /// Fixed-clock whole-buffer oracle across first/open/hover/selection/scroll/close.
 #[test]fn retained_pack_hover_and_close_match_full_pixels(){
  let run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=crate::observe::observe(&run);
  let mut p=Painter::new();let mut full=Painter::new();let mut base=crate::render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();
  let mut pack=PackView::observe(&run);pack.configure(&p,modal_ui::Frame::from_profile(&p.profile).map.tuple());
  draw(&mut p,&mut base,&mut tracker,&v,None,true);oracle(&mut full,&v,None);assert_eq!(p.pixels,full.pixels);
  draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels);
  let rows=pack.row_positions();assert!(!rows.is_empty());let point=(pack.layout.list.0+10,rows[0].1+pack.layout.row/2);
  pack.hover(point.0,point.1);let d=draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false);assert!(!d.is_empty());assert!(d.area()<p.width*p.height);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels);
  assert!(!pack.hover(point.0,point.1));assert!(draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false).is_empty());
  pack.hover(point.0+7,point.1+2);draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels);
  let _=pack.click(point.0,point.1);draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels);
  pack.scroll(-2,point.0,point.1);draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),false);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels);
  draw(&mut p,&mut base,&mut tracker,&v,None,false);oracle(&mut full,&v,None);assert_eq!(p.pixels,full.pixels);
 }
 /// Changing scale invalidates both framebuffer and resolved overlay geometry.
 #[test]fn retained_pack_resize_matches_full_pixels(){
  let run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=crate::observe::observe(&run);let mut p=Painter::new();let mut full=Painter::new();let mut base=crate::render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();let mut pack=PackView::observe(&run);
  for size in [(1280,800),(1920,1200),(1600,900),(1280,800)]{p.resize(size.0,size.1);full.resize(size.0,size.1);base.invalidate();tracker.invalidate();pack.configure(&p,modal_ui::Frame::from_profile(&p.profile).map.tuple());let rows=pack.row_positions();pack.hover(pack.layout.list.0+10,rows[0].1+10);draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),true);oracle(&mut full,&v,Some(&pack));assert_eq!(p.pixels,full.pixels,"size {size:?}");}
 }
 #[test]fn explicit_profile_invalidation_preserves_full_oracle_without_resize(){
  let run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=crate::observe::observe(&run);let mut p=Painter::new();let mut full=Painter::new();let mut base=crate::render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();let mut pack=PackView::observe(&run);pack.configure(&p,modal_ui::Frame::from_profile(&p.profile).map.tuple());draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),true);
  let size=(p.width,p.height);let changed=p.profile.set("ui.colors.panel",serde_json::json!("#182b35")).unwrap().set("typography.primary.fontSize",serde_json::json!(20)).unwrap();p.profile=changed.clone();full.profile=changed;base.invalidate();tracker.invalidate();pack.configure(&p,modal_ui::Frame::from_profile(&p.profile).map.tuple());draw(&mut p,&mut base,&mut tracker,&v,Some(&pack),true);oracle(&mut full,&v,Some(&pack));assert_eq!((p.width,p.height),size);assert_eq!(p.pixels,full.pixels);
 }

 #[test]fn pack_under_retracting_shutter_restores_tooltip_and_text_exactly(){
  let run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=crate::observe::observe(&run);let mut p=Painter::new();let mut full=Painter::new();let mut base=crate::render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();let mut pack=PackView::observe(&run);pack.configure(&p,modal_ui::Frame::from_profile(&p.profile).map.tuple());let row=pack.row_positions()[0].1;pack.hover(pack.layout.list.0+20,row+10);let lines=vec!["A remembered Chronicle event.".to_string();70];let mut shutter=crate::chronicle_overlay::Shutter::default();
  for(i,now)in [0.,100.,220.,300.,410.,520.,600.].into_iter().enumerate(){if i==1{shutter.open(now,false);}if i==3{shutter.close(now,false);}let active=i>0&&i<6;
   let state=overlay_damage::Frame{pack:Some(pack.paint_state(&p)),shutter:if active{crate::chronicle_overlay::bounds(&shutter,now,false).and_then(|(x,y,w,h)|p.logical_damage_rect(x,y,w,h)).map(|bounds|overlay_damage::Shutter{bounds,offset:0})}else{None},..Default::default()};
   let mut d=base.update(&mut p,&v,"",0.,i==0,false).unwrap();d.extend(&tracker.changes(&p,&state));state.expand_dependencies(&p,&mut d);base.restore(&mut p,&d).unwrap();for r in d.rects(){p.damage_clip=Some(*r);if state.pack_intersects(&p,*r){pack.paint_prepared(&mut p,state.pack.as_ref().unwrap());}if state.shutter_intersects(*r){crate::chronicle_overlay::paint(&mut p,&lines,&shutter,now,false);}}p.damage_clip=None;full.game_at(&v,"",false,0.);pack.paint(&mut full);if active{crate::chronicle_overlay::paint(&mut full,&lines,&shutter,now,false);}assert_eq!(p.pixels,full.pixels,"overlap frame {i}");tracker.commit(state);
  }
 }

}
#[cfg(test)]impl PackView{
 /// Test-only exact hit point; uses production row layout rather than guessed coordinates.
 pub(crate)fn retained_test_hover_point(&self,index:usize)->Option<(i32,i32)>{let(_,y)=*self.row_positions().get(index)?;Some((self.layout.list.0+12,y-self.list_scroll+self.layout.row/2))}
}
