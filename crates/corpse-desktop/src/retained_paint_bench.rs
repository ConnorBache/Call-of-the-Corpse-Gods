// Lead-controlled same-binary full/retained software composition and SDL upload benchmark.
#[cfg(test)]mod retained_paint_bench{
 use super::*;
 use render::damage::{Damage,RasterStats};
 use std::time::Instant;
 #[derive(serde::Serialize)]struct Sample{prepare_ns:u128,base_update_including_discovery_ns:u128,overlay_damage_planning_ns:u128,base_restore_copy_ns:u128,overlay_paint_ns:u128,damage_pixels:usize,upload_bytes:usize,upload_calls:usize,pixel_visits:u64,pixel_writes:u64,cells_considered:u64,cells_painted:u64}
 #[derive(Default)]struct Retained{base:render::RetainedGame,overlays:overlay_damage::Tracker}
 struct Input<'a>{view:&'a render::View,pack:Option<&'a pack::PackView>,shutter:Option<(&'a chronicle_overlay::Shutter,&'a[String])>,now:f64,view_changed:bool,force_full:bool}
 fn state(p:&Painter,input:&Input<'_>)->overlay_damage::Frame{
  overlay_damage::Frame{pack:input.pack.map(|v|v.paint_state(p)),shutter:input.shutter.and_then(|(s,_)|chronicle_overlay::bounds(s,input.now,false).and_then(|(x,y,w,h)|p.logical_damage_rect(x,y,w,h)).map(|bounds|overlay_damage::Shutter{bounds,offset:s.offset})),..Default::default()}
 }
 // Calls the SAME RetainedGame, overlay Tracker, dependency expansion and prepared
 // pack/shutter painters as main. No separate pixel-change discovery algorithm.
 fn compose(p:&mut Painter,retained:&mut Retained,input:&Input<'_>,enabled:bool,counters:bool)->(Damage,Sample){
  p.raster_stats=RasterStats{enabled:counters,..Default::default()};
  let t=Instant::now();let next=state(p,input);let prepare_ns=t.elapsed().as_nanos();
  let t=Instant::now();let mut damage=if enabled{retained.base.update(p,input.view,"",input.now,input.view_changed,input.force_full).unwrap()}else{p.damage_clip=None;p.game_at(input.view,"",false,input.now);Damage::full(p.width,p.height)};let base_ns=t.elapsed().as_nanos();
  let t=Instant::now();if enabled{damage.extend(&retained.overlays.changes(p,&next));next.expand_dependencies(p,&mut damage);}let planning_ns=t.elapsed().as_nanos();
  let t=Instant::now();if enabled{retained.base.restore(p,&damage).unwrap();}let restore_ns=t.elapsed().as_nanos();
  let t=Instant::now();for r in damage.rects(){p.damage_clip=enabled.then_some(*r);if next.pack_intersects(p,*r){input.pack.unwrap().paint_prepared(p,next.pack.as_ref().unwrap());}if next.shutter_intersects(*r){let(s,lines)=input.shutter.unwrap();chronicle_overlay::paint(p,lines,s,input.now,false);}}p.damage_clip=None;let overlay_ns=t.elapsed().as_nanos();
  retained.overlays.commit(next);
  let s=p.raster_stats;
  let record=Sample{prepare_ns,base_update_including_discovery_ns:base_ns,overlay_damage_planning_ns:planning_ns,base_restore_copy_ns:restore_ns,overlay_paint_ns:overlay_ns,damage_pixels:damage.area(),upload_bytes:damage.upload_bytes(),upload_calls:damage.rects().len(),pixel_visits:s.pixel_visits,pixel_writes:s.pixel_writes,cells_considered:s.cells_considered,cells_painted:s.cells_painted};
  (damage,record)
 }
 fn surface(width:usize,height:usize)->sdl2::render::Canvas<sdl2::surface::Surface<'static>>{sdl2::surface::Surface::new(width as u32,height as u32,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap().into_canvas().unwrap()}
 fn upload(canvas:&mut sdl2::render::Canvas<sdl2::surface::Surface<'static>>,texture:&mut sdl2::render::Texture<'_>,p:&Painter,d:&Damage)->(u128,u128){
  let t=Instant::now();for r in d.rects(){let(bytes,pitch)=render::damage::upload_source(&p.pixels,p.width,p.height,*r).unwrap();texture.update(Some(sdl2::rect::Rect::new(r.x as i32,r.y as i32,r.w as u32,r.h as u32)),bytes,pitch).unwrap();}let upload_ns=t.elapsed().as_nanos();
  let t=Instant::now();if !d.is_empty(){canvas.clear();canvas.copy(texture,None,None).unwrap();canvas.present();}let present_ns=t.elapsed().as_nanos();(upload_ns,present_ns)
 }
 fn fixture()->corpse_core::Run{serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()}
 /// No gameplay timings: canonical fixture observation and view descriptor changes
 /// occur OUTSIDE samples. Includes retained planning even when damage is empty.
 #[test]
 #[ignore="lead-controlled software painting/upload benchmark, single thread"]
 fn retained_software_paint_and_upload_matrix(){
  let repeats:usize=std::env::var("RETAINED_REPEATS").unwrap_or_else(|_|"20".into()).parse().unwrap();assert!((1..=100).contains(&repeats));
  let out=std::path::PathBuf::from(std::env::var_os("RETAINED_OUTPUT").expect("explicit new JSON report path"));assert!(out.is_absolute()&&!out.exists());
  let counters=std::env::var("RETAINED_COUNTERS").as_deref()==Ok("1");let attribution=std::env::var("RETAINED_ATTRIBUTION").as_deref()==Ok("1");
  #[cfg(not(feature="runtime-timing"))]assert!(!attribution,"attribution needs runtime-timing feature");
  let _sdl=sdl2::init().unwrap();let mut rows=Vec::new();
  let cases=std::env::var("RETAINED_CASES").unwrap_or_else(|_|"stable_game,stable_pack_hover,changed_pack_hover,actor_move,sparse_fire,dense_fire,shutter,full_change".into());
  for case in cases.split(','){
   let raw=fixture();let mut view=observe::observe(&raw);let canonical=serde_json::to_vec(&raw).unwrap();
   // Explicit detached rendering fixtures, not claims about generated visible actors/fire.
   // The seed room may contain no current actor cells, so never filter to a no-op case.
   if case=="actor_move"{let mut actor=view.cells[view.cells.len()/2].clone();actor.current=true;actor.direct=true;actor.actor_id=Some(987654.);actor.glyph="b".into();actor.fg=[230,180,40];actor.x=actor.x.min(view.width.saturating_sub(2));view.cells.push(actor);}
   let mut animated_cells=0usize;let eligible_fire_cells=view.cells.iter().filter(|c|c.actor_id.is_none()).count();
   if case=="sparse_fire"||case=="dense_fire"{let count=if case=="sparse_fire"{3}else{usize::MAX};for c in view.cells.iter_mut().filter(|c|c.actor_id.is_none()).take(count){c.current=true;c.direct=true;c.fire=true;c.animations.clear();c.animations.push(render::Animation::Fire{phase:(c.x+c.y)as f64/7.,speed:1.,colors:[[227,45,19],[255,117,30],[255,215,84],[255,145,32]]});animated_cells+=1;}assert!(animated_cells>0,"fire fixture must exercise animation");if case=="sparse_fire"{assert_eq!(animated_cells,3);}else{assert_eq!(animated_cells,eligible_fire_cells);}}
   let mut full=Painter::new();let mut partial=Painter::new();let mut full_state=Retained::default();let mut partial_state=Retained::default();let mut pack=pack::PackView::observe(&raw);pack.configure(&partial,modal_ui::Frame::from_profile(&partial.profile).map.tuple());let point=pack.retained_test_hover_point(0).expect("fixture item");pack.hover(point.0,point.1);
   let mut shutter=chronicle_overlay::Shutter::default();let lines:Vec<_>=(0..80).map(|n|format!("Chronicle row {n}: a remembered event.")).collect();let mut canvas=surface(full.width,full.height);let creator=canvas.texture_creator();let mut full_texture=creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32,full.width as u32,full.height as u32).unwrap();let mut partial_texture=creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32,partial.width as u32,partial.height as u32).unwrap();
   let actor_origin=view.cells.iter().find(|c|c.actor_id==Some(987654.)&&c.current).map(|c|(c.actor_id,c.x,c.y));
   // Warm initial pixels/text masks and both texture contents, outside samples.
   let initial=Input{view:&view,pack:case.contains("pack").then_some(&pack),shutter:None,now:0.,view_changed:true,force_full:true};for(p,s,t)in[(&mut full,&mut full_state,&mut full_texture),(&mut partial,&mut partial_state,&mut partial_texture)]{let(d,_)=compose(p,s,&initial,true,false);upload(&mut canvas,t,p,&d);}assert_eq!(full.pixels,partial.pixels);
   for repeat in 1..=repeats{
    let mut now=0.;let changed=case=="actor_move";
    if case=="changed_pack_hover"{pack.hover(point.0+(repeat%7)as i32*9,point.1+(repeat%3)as i32);}
    if case=="actor_move"{let(id,x,y)=actor_origin.unwrap();let c=view.cells.iter_mut().find(|c|c.actor_id==id&&c.actor_id.is_some()).unwrap();c.x=x+repeat%2;c.y=y;}
    if case=="sparse_fire"||case=="dense_fire"{now=repeat as f64*83.;}
    if case=="shutter"{now=repeat as f64*55.;if repeat%8==1{shutter.open(now,false);}if repeat%8==5{shutter.close(now,false);}}
    let input=Input{view:&view,pack:case.contains("pack").then_some(&pack),shutter:(case=="shutter").then_some((&shutter,lines.as_slice())),now,view_changed:changed,force_full:case=="full_change"};
    let order=if repeat%2==1{[false,true]}else{[true,false]};
    for enabled in order{
     let(p,s,texture)=if enabled{(&mut partial,&mut partial_state,&mut partial_texture)}else{(&mut full,&mut full_state,&mut full_texture)};
     #[cfg(feature="runtime-timing")]{corpse_core::timing::set_enabled(false);corpse_core::timing::reset();corpse_core::timing::set_enabled(attribution);}
     let root=Instant::now();let(d,sample)=compose(p,s,&input,enabled,counters);let(upload_ns,present_ns)=upload(&mut canvas,texture,p,&d);let total_ns=root.elapsed().as_nanos();
     #[cfg(feature="runtime-timing")]corpse_core::timing::set_enabled(false);
     let mut data=serde_json::to_value(sample).unwrap();
     #[cfg(feature="runtime-timing")]{data["attribution_spans"]=serde_json::to_value(corpse_core::timing::take_report()).unwrap();}
     data["animated_cells"]=animated_cells.into();data["case"]=case.into();data["repeat"]=repeat.into();data["retained"]=enabled.into();data["total_software_pipeline_ns"]=u64::try_from(total_ns).unwrap().into();data["texture_upload_ns"]=u64::try_from(upload_ns).unwrap().into();data["software_canvas_copy_present_ns"]=u64::try_from(present_ns).unwrap().into();data["width"]=p.width.into();data["height"]=p.height.into();rows.push(data);
    }
    // Whole-frame correctness OUTSIDE both timed roots, not change discovery.
    assert_eq!(partial.pixels,full.pixels,"case {case} repeat {repeat}");
    canvas.clear();canvas.copy(&partial_texture,None,None).unwrap();assert_eq!(canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap(),partial.pixels,"texture {case} repeat {repeat}");
   }
   assert_eq!(serde_json::to_vec(&raw).unwrap(),canonical);
  }
  std::fs::write(&out,serde_json::to_vec_pretty(&serde_json::json!({"schema":1,"repeats":repeats,"scope":"Same-binary full versus retained paint+SDL SOFTWARE texture upload/copy. No GPU, Windows, input event, gameplay, observation or save timing. Setup/descriptor mutation/pixel and texture readback oracles outside interval. Actor movement and fire are controlled detached visual descriptors, not a gameplay simulation benchmark. Alternating paired order. Native1280x800. Full-change case explicitly forces full invalidation; separate resize oracle verifies dimensions.","raster_counters_enabled":counters,"attribution_enabled":attribution,"counts":"When enabled, raster operation counts are enabled in both arms; not unique changed pixels. Damage may include unchanged regions. Empty retained frames still include planning and descriptor costs.","rows":rows})).unwrap()).unwrap();println!("RETAINED_PAINT_REPORT {}",out.display());
 }
}
