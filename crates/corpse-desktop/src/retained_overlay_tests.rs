// Fixed-time retained compositor oracle. No SDL/window or game dispatch.
#[cfg(test)]mod retained_overlay_tests{
 use super::*;
 #[test]fn shutter_animation_scroll_close_and_error_match_full_pixels(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let v=observe::observe(&run);
  let mut p=Painter::new();let mut full=Painter::new();let mut base=render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();let mut shutter=chronicle_overlay::Shutter::default();let lines:Vec<_>=(0..80).map(|i|format!("Remembered event {i}: the passage remains." )).collect();
  shutter.open(0.,false);
  for (i,now) in [0.,55.,110.,220.,221.,270.,330.,440.,500.].into_iter().enumerate(){
   if i==4{shutter.scroll(5,lines.len());}if i==5{shutter.close(now,false);}
   let visible=i<8;
   let state=overlay_damage::Frame{shutter:visible.then(||chronicle_overlay::bounds(&shutter,now,false)).flatten().and_then(|(x,y,w,h)|p.logical_damage_rect(x,y,w,h)).map(|bounds|overlay_damage::Shutter{bounds,offset:shutter.offset}),save_error:(i==3||i==4).then(||"A precise failed save.".to_string()),..Default::default()};
   let mut d=base.update(&mut p,&v,"",0.,i==0,false).unwrap();d.extend(&tracker.changes(&p,&state));base.restore(&mut p,&d).unwrap();
   for r in d.rects(){p.damage_clip=Some(*r);if state.shutter_intersects(*r){chronicle_overlay::paint(&mut p,&lines,&shutter,now,false);}if state.banner_intersects(&p,*r){paint_save_failure(&mut p,state.save_error.as_ref().unwrap());}}
   p.damage_clip=None;full.game_at(&v,"",false,0.);if visible{chronicle_overlay::paint(&mut full,&lines,&shutter,now,false);}if let Some(e)=&state.save_error{paint_save_failure(&mut full,e);}assert_eq!(p.pixels,full.pixels,"frame {i}");tracker.commit(state);
  }
 }
}
#[cfg(test)]mod retained_effect_tests{
 use super::*;
 #[test]fn effect_actor_long_superscript_removal_is_exact_at_native_scales(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  for(size_w,size_h)in [(1280,800),(1920,1200)]{
   let mut v=observe::observe(&run);
   // Explicit detached visual fixture: the seed-room observation need not
   // contain an admitted visible actor. Exercise the actual actor redraw branch.
   let c=v.cells.first_mut().expect("fixture has observed cells");
   c.current=true;c.direct=true;c.actor_id=Some(987654.);c.glyph="b".into();c.fg=[230,180,40];
   c.superscript="123456789012345678901234".into();let(x,y)=(c.x,c.y);
   let cue=effects::Cue{event:corpse_core::events::PresentationEvent{kind:"status".into(),from:None,to:corpse_core::events::EventPoint{x:x as f64,y:y as f64},glyph:Some("!".into()),color:Some("#88ffaa".into()),label:None,depth:v.depth, animation: None, },started:0.,held:false,static_marks:false};
   let mut p=Painter::new();p.resize(size_w,size_h);let mut full=Painter::new();full.resize(size_w,size_h);let mut base=render::RetainedGame::default();let mut tracker=overlay_damage::Tracker::default();
   for(i,now)in [0.,20.,120.,300.].into_iter().enumerate(){let cues=if i==0||i==3{vec![]}else{vec![cue.clone()]};let state=overlay_damage::Frame{effects:overlay_damage::effects(&p,&v,&cues,now,false),..Default::default()};let mut d=base.update(&mut p,&v,"",0.,i==0,false).unwrap();d.extend(&tracker.changes(&p,&state));state.expand_dependencies(&p,&mut d);base.restore(&mut p,&d).unwrap();for r in d.rects(){p.damage_clip=Some(*r);if state.effects_intersect(*r){p.effects(&v,&cues,now,false);}}p.damage_clip=None;full.game_at(&v,"",false,0.);full.effects(&v,&cues,now,false);assert_eq!(p.pixels,full.pixels,"scale {size_w} frame {i}");tracker.commit(state);}
  }
 }
}
#[cfg(test)]mod retained_sdl_tests{
 use super::*;
 /// Software SDL correctness only, not accelerated-GPU or Windows latency evidence.
 #[test]
 #[ignore="lead-controlled SDL software texture/readback oracle; run alone"]
 fn native_subrect_upload_matches_full_texture_without_touching_other_pixels(){
  let _sdl=sdl2::init().unwrap();
  let surface=sdl2::surface::Surface::new(31,19,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();
  let mut canvas=surface.into_canvas().unwrap();let creator=canvas.texture_creator();
  let mut texture=creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32,31,19).unwrap();
  let mut pixels=vec![0u8;31*19*4];for(y,row)in pixels.chunks_mut(31*4).enumerate(){for(x,c)in row.chunks_mut(4).enumerate(){c.copy_from_slice(&[(x*7)as u8,(y*11)as u8,77,255]);}}
  texture.update(None,&pixels,31*4).unwrap();canvas.clear();canvas.copy(&texture,None,None).unwrap();assert_eq!(canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap(),pixels);
  for rect in [render::damage::Rect::new(30,18,1,1),render::damage::Rect::new(3,4,7,9),render::damage::Rect::new(0,0,31,1)]{
   for y in rect.y..rect.bottom(){for x in rect.x..rect.right(){let at=(y*31+x)*4;pixels[at..at+4].copy_from_slice(&[220,31,129,255]);}}
   let(bytes,pitch)=render::damage::upload_source(&pixels,31,19,rect).unwrap();texture.update(Some(sdl2::rect::Rect::new(rect.x as i32,rect.y as i32,rect.w as u32,rect.h as u32)),bytes,pitch).unwrap();canvas.clear();canvas.copy(&texture,None,None).unwrap();assert_eq!(canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap(),pixels);
  }
  // Device-reset recovery: discard texture storage, keep exact CPU pixels, then
  // recreate and upload them. No Painter or observation operation is invoked.
  let retained=pixels.clone();
  texture=creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32,31,19).unwrap();
  let d=render::damage::Damage::full(31,19);for r in d.rects(){let(bytes,pitch)=render::damage::upload_source(&pixels,31,19,*r).unwrap();texture.update(Some(sdl2::rect::Rect::new(r.x as i32,r.y as i32,r.w as u32,r.h as u32)),bytes,pitch).unwrap();}
  canvas.clear();canvas.copy(&texture,None,None).unwrap();assert_eq!(canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap(),retained);assert_eq!(pixels,retained);

 }
}
